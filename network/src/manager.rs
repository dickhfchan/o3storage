use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{Result, NetworkError, NetworkNode, Config};
use crate::discovery::PeerDiscovery;
use crate::communication::MessageHandler;

pub struct NetworkManager {
    node_id: Uuid,
    peer_discovery: Arc<RwLock<PeerDiscovery>>,
    message_handler: Arc<MessageHandler>,
    config: Config,
}

impl NetworkManager {
    pub async fn new(config: Config) -> Result<Self> {
        let node_id = Uuid::new_v4();
        
        let peer_discovery = Arc::new(RwLock::new(PeerDiscovery::new(config.clone())));
        let message_handler = Arc::new(MessageHandler::new(node_id, config.clone()));

        Ok(Self {
            node_id,
            peer_discovery,
            message_handler,
            config,
        })
    }

    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting network manager for node {}", self.node_id);

        let discovery_task = {
            let discovery = self.peer_discovery.clone();
            tokio::spawn(async move {
                let mut discovery_guard = discovery.write().await;
                discovery_guard.start_discovery().await
            })
        };

        let message_task = {
            let handler = self.message_handler.clone();
            tokio::spawn(async move {
                handler.start().await
            })
        };

        let peer_management_task = {
            let discovery = self.peer_discovery.clone();
            let handler = self.message_handler.clone();
            let config = self.config.clone();
            tokio::spawn(async move {
                Self::manage_peer_connections(discovery, handler, config).await
            })
        };

        tokio::select! {
            result = discovery_task => {
                tracing::error!("Discovery task failed: {:?}", result);
                Err(NetworkError::Discovery("Discovery task failed".to_string()))
            }
            result = message_task => {
                tracing::error!("Message handler task failed: {:?}", result);
                Err(NetworkError::Protocol("Message handler failed".to_string()))
            }
            result = peer_management_task => {
                tracing::error!("Peer management task failed: {:?}", result);
                Err(NetworkError::ConnectionFailed("Peer management failed".to_string()))
            }
        }
    }

    pub async fn get_connected_peers(&self) -> Vec<Uuid> {
        self.message_handler.get_connected_nodes().await
    }

    pub async fn get_discovered_peers(&self) -> Vec<NetworkNode> {
        let discovery = self.peer_discovery.read().await;
        discovery.get_discovered_peers()
    }

    pub async fn send_message_to_peer(&self, peer_id: Uuid, message: crate::NetworkMessage) -> Result<()> {
        self.message_handler.send_message(peer_id, message).await
    }

    pub async fn broadcast_message(&self, message: crate::NetworkMessage) -> Result<()> {
        self.message_handler.broadcast_message(message).await
    }

    pub async fn add_peer(&self, node: NetworkNode) -> Result<()> {
        // Add to discovery list
        {
            let mut discovery = self.peer_discovery.write().await;
            discovery.add_discovered_peer(node.clone());
        }

        // Establish connection
        self.message_handler.add_connection(node).await
    }

    pub async fn remove_peer(&self, peer_id: Uuid) -> Result<()> {
        // Remove from discovery list
        {
            let mut discovery = self.peer_discovery.write().await;
            discovery.remove_peer(&peer_id);
        }

        // Remove connection
        self.message_handler.remove_connection(peer_id).await
    }

    pub async fn update_local_status(&self, available_space: u64) {
        let mut discovery = self.peer_discovery.write().await;
        discovery.update_local_capabilities(available_space);
    }

    async fn manage_peer_connections(
        discovery: Arc<RwLock<PeerDiscovery>>,
        handler: Arc<MessageHandler>,
        config: Config,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(
            std::time::Duration::from_millis(config.heartbeat_interval_ms * 2)
        );

        loop {
            interval.tick().await;

            // Get discovered peers
            let discovered_peers = {
                let discovery_guard = discovery.read().await;
                discovery_guard.get_discovered_peers()
            };

            // Get currently connected peers
            let connected_peers = handler.get_connected_nodes().await;

            // Add connections for newly discovered peers
            for peer in discovered_peers {
                if !connected_peers.contains(&peer.id) {
                    tracing::info!("Establishing connection to discovered peer {}", peer.id);
                    if let Err(e) = handler.add_connection(peer.clone()).await {
                        tracing::error!("Failed to connect to peer {}: {}", peer.id, e);
                    }
                }
            }

            // TODO: Remove stale connections
            // TODO: Send periodic ping messages
            // TODO: Update peer status based on connectivity
        }
    }
}