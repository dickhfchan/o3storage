use crate::config::Config;
use crate::error::{O3StorageError, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

pub struct Node {
    config: Config,
    cluster_state: Arc<RwLock<ClusterState>>,
    storage_engine: Arc<storage::StorageEngine>,
    consensus_manager: Arc<consensus::ConsensusManager>,
    api_server: Arc<api::Server>,
    network_manager: Arc<network::NetworkManager>,
}

#[derive(Debug, Clone)]
pub struct ClusterState {
    pub active_nodes: Vec<NodeInfo>,
    pub total_replicas: usize,
    pub is_write_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: uuid::Uuid,
    pub address: std::net::IpAddr,
    pub port: u16,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub status: NodeStatus,
}

#[derive(Debug, Clone)]
pub enum NodeStatus {
    Active,
    Inactive,
    Failed,
}

impl Node {
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing O3Storage node at {}", config.bind_address());

        let storage_engine = Arc::new(
            storage::StorageEngine::new(&config.storage_path, config.max_storage_size).await?
        );

        let cluster_state = Arc::new(RwLock::new(ClusterState {
            active_nodes: Vec::new(),
            total_replicas: 0,
            is_write_enabled: false,
        }));

        let consensus_manager = Arc::new(
            consensus::ConsensusManager::new(config.clone().into(), cluster_state.clone()).await?
        );

        let network_manager = Arc::new(
            network::NetworkManager::new(config.clone().into()).await?
        );

        let api_server = Arc::new(
            api::Server::new(
                config.clone().into(),
                storage_engine.clone(),
                consensus_manager.clone(),
                cluster_state.clone(),
            ).await?
        );

        Ok(Self {
            config,
            cluster_state,
            storage_engine,
            consensus_manager,
            api_server,
            network_manager,
        })
    }

    pub async fn start(self) -> Result<()> {
        info!("Starting O3Storage node services");

        let storage_task = {
            let storage = self.storage_engine.clone();
            tokio::spawn(async move {
                storage.start().await
            })
        };

        let consensus_task = {
            let consensus = self.consensus_manager.clone();
            tokio::spawn(async move {
                consensus.start().await
            })
        };

        let network_task = {
            let network = self.network_manager.clone();
            tokio::spawn(async move {
                network.start().await
            })
        };

        let api_task = {
            let api = self.api_server.clone();
            tokio::spawn(async move {
                api.start().await
            })
        };

        let cluster_monitor_task = {
            let cluster_state = self.cluster_state.clone();
            let config = self.config.clone();
            tokio::spawn(async move {
                Self::monitor_cluster_health(cluster_state, config).await
            })
        };

        tokio::select! {
            result = storage_task => {
                error!("Storage engine stopped: {:?}", result);
                Err(O3StorageError::Storage("Storage engine failed".to_string()))
            }
            result = consensus_task => {
                error!("Consensus manager stopped: {:?}", result);
                Err(O3StorageError::Consensus("Consensus manager failed".to_string()))
            }
            result = network_task => {
                error!("Network manager stopped: {:?}", result);
                Err(O3StorageError::Network("Network manager failed".to_string()))
            }
            result = api_task => {
                error!("API server stopped: {:?}", result);
                Err(O3StorageError::Network("API server failed".to_string()))
            }
            result = cluster_monitor_task => {
                error!("Cluster monitor stopped: {:?}", result);
                Err(O3StorageError::Consensus("Cluster monitor failed".to_string()))
            }
        }
    }

    async fn monitor_cluster_health(
        cluster_state: Arc<RwLock<ClusterState>>,
        config: Config,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_millis(config.heartbeat_interval_ms)
        );

        loop {
            interval.tick().await;

            let mut state = cluster_state.write().await;
            
            let active_count = state.active_nodes
                .iter()
                .filter(|node| matches!(node.status, NodeStatus::Active))
                .count();

            state.total_replicas = active_count;
            
            let was_write_enabled = state.is_write_enabled;
            state.is_write_enabled = active_count >= config.replication_factor;

            if was_write_enabled && !state.is_write_enabled {
                warn!("Write operations disabled: insufficient replicas ({} < {})", 
                      active_count, config.replication_factor);
            } else if !was_write_enabled && state.is_write_enabled {
                info!("Write operations enabled: sufficient replicas ({})", active_count);
            }

            let now = chrono::Utc::now();
            state.active_nodes.retain_mut(|node| {
                let elapsed = now.signed_duration_since(node.last_heartbeat);
                if elapsed.num_milliseconds() > (config.heartbeat_interval_ms * 3) as i64 {
                    if matches!(node.status, NodeStatus::Active) {
                        warn!("Node {}:{} marked as failed due to missed heartbeats", 
                              node.address, node.port);
                        node.status = NodeStatus::Failed;
                    }
                    false
                } else {
                    true
                }
            });
        }
    }
}