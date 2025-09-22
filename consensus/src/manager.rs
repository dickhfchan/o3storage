use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{Result, ConsensusError, NodeId, NodeInfo, NodeStatus, ClusterView, Config, ClusterState};
use crate::messages::*;
use crate::raft::RaftNode;

pub struct ConsensusManager {
    node_id: NodeId,
    raft_node: Arc<RaftNode>,
    cluster_state: Arc<RwLock<ClusterState>>,
    message_sender: mpsc::UnboundedSender<ConsensusMessage>,
    message_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<ConsensusMessage>>>>,
    config: Config,
}


impl ConsensusManager {
    pub async fn new(
        config: Config,
        cluster_state: Arc<RwLock<ClusterState>>,
    ) -> Result<Self> {
        let node_id = NodeId::new();
        let (tx, rx) = mpsc::unbounded_channel();
        
        let raft_node = Arc::new(RaftNode::new(node_id.clone(), config.clone()).await?);
        
        Ok(Self {
            node_id,
            raft_node,
            cluster_state,
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(Some(rx))),
            config,
        })
    }

    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting consensus manager for node {:?}", self.node_id);

        let mut receiver = {
            let mut guard = self.message_receiver.write().await;
            guard.take().ok_or_else(|| {
                ConsensusError::InvalidMessage("Message receiver already taken".to_string())
            })?
        };

        let heartbeat_task = {
            let node_id = self.node_id.clone();
            let config = self.config.clone();
            let sender = self.message_sender.clone();
            tokio::spawn(async move {
                Self::heartbeat_loop(node_id, config, sender).await
            })
        };

        let raft_task = {
            let raft = self.raft_node.clone();
            tokio::spawn(async move {
                raft.run().await
            })
        };

        let message_processing_task = {
            let raft = self.raft_node.clone();
            let cluster_state = self.cluster_state.clone();
            tokio::spawn(async move {
                Self::process_messages(receiver, raft, cluster_state).await
            })
        };

        tokio::select! {
            result = heartbeat_task => {
                tracing::error!("Heartbeat task failed: {:?}", result);
                Err(ConsensusError::Network("Heartbeat task failed".to_string()))
            }
            result = raft_task => {
                tracing::error!("Raft task failed: {:?}", result);
                Err(ConsensusError::Network("Raft task failed".to_string()))
            }
            result = message_processing_task => {
                tracing::error!("Message processing task failed: {:?}", result);
                Err(ConsensusError::Network("Message processing task failed".to_string()))
            }
        }
    }

    pub async fn request_replication(
        &self,
        object_id: String,
        operation: ReplicationOperation,
        metadata: ObjectReplicationMetadata,
        data: Option<bytes::Bytes>,
    ) -> Result<()> {
        let cluster_state = self.cluster_state.read().await;
        
        if cluster_state.total_replicas < self.config.replication_factor {
            return Err(ConsensusError::InsufficientReplicas {
                current: cluster_state.total_replicas,
                required: self.config.replication_factor,
            });
        }

        let request = ReplicationRequest {
            request_id: Uuid::new_v4(),
            object_id,
            operation,
            metadata,
            data,
            from: self.node_id.clone(),
            target_replicas: self.config.replication_factor,
        };

        self.raft_node.submit_replication_request(request).await
    }

    pub async fn get_cluster_view(&self) -> ClusterView {
        self.raft_node.get_cluster_view().await
    }

    pub async fn is_leader(&self) -> bool {
        self.raft_node.is_leader().await
    }

    pub async fn get_leader(&self) -> Option<NodeId> {
        self.raft_node.get_current_leader().await
    }

    async fn heartbeat_loop(
        node_id: NodeId,
        config: crate::Config,
        sender: mpsc::UnboundedSender<ConsensusMessage>,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_millis(config.heartbeat_interval_ms)
        );

        loop {
            interval.tick().await;
            
            let heartbeat = ConsensusMessage::Heartbeat(HeartbeatMessage {
                from: node_id.clone(),
                term: 0, // Will be filled by Raft
                timestamp: Utc::now(),
                is_leader: false, // Will be updated by Raft
            });

            if sender.send(heartbeat).is_err() {
                tracing::error!("Failed to send heartbeat message");
                break;
            }
        }

        Ok(())
    }

    async fn process_messages(
        mut receiver: mpsc::UnboundedReceiver<ConsensusMessage>,
        raft: Arc<RaftNode>,
        cluster_state: Arc<RwLock<crate::ClusterState>>,
    ) -> Result<()> {
        while let Some(message) = receiver.recv().await {
            if let Err(e) = Self::handle_message(message, &raft, &cluster_state).await {
                tracing::error!("Failed to handle consensus message: {}", e);
            }
        }
        Ok(())
    }

    async fn handle_message(
        message: ConsensusMessage,
        raft: &Arc<RaftNode>,
        cluster_state: &Arc<RwLock<crate::ClusterState>>,
    ) -> Result<()> {
        match message {
            ConsensusMessage::Heartbeat(heartbeat) => {
                raft.handle_heartbeat(heartbeat.clone()).await?;
                Self::update_cluster_state_from_heartbeat(&heartbeat, cluster_state).await;
            }
            ConsensusMessage::VoteRequest(vote_req) => {
                raft.handle_vote_request(vote_req).await?;
            }
            ConsensusMessage::VoteResponse(vote_resp) => {
                raft.handle_vote_response(vote_resp).await?;
            }
            ConsensusMessage::AppendEntries(append_req) => {
                raft.handle_append_entries(append_req).await?;
            }
            ConsensusMessage::AppendEntriesResponse(append_resp) => {
                raft.handle_append_entries_response(append_resp).await?;
            }
            ConsensusMessage::ReplicationRequest(repl_req) => {
                raft.handle_replication_request(repl_req).await?;
            }
            ConsensusMessage::ReplicationResponse(repl_resp) => {
                raft.handle_replication_response(repl_resp).await?;
            }
            ConsensusMessage::JoinRequest(join_req) => {
                raft.handle_join_request(join_req).await?;
            }
            ConsensusMessage::JoinResponse(join_resp) => {
                raft.handle_join_response(join_resp).await?;
            }
        }
        Ok(())
    }

    async fn update_cluster_state_from_heartbeat(
        heartbeat: &HeartbeatMessage,
        cluster_state: &Arc<RwLock<crate::ClusterState>>,
    ) {
        let mut state = cluster_state.write().await;
        
        let now = Utc::now();
        
        if let Some(node) = state.active_nodes.iter_mut().find(|n| n.id == heartbeat.from) {
            node.last_seen = now;
            node.status = crate::NodeStatus::Active;
        } else {
            let node_info = crate::NodeInfo {
                id: heartbeat.from,
                address: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), // TODO: Get actual address
                port: 8080, // TODO: Get actual port
                last_seen: now,
                status: crate::NodeStatus::Active,
            };
            state.active_nodes.push(node_info);
        }
    }
}