mod manager;
mod raft;
mod messages;

pub use manager::ConsensusManager;
pub use raft::RaftNode;
pub use messages::{ConsensusMessage, ReplicationRequest, ReplicationResponse, ReplicationOperation, ObjectReplicationMetadata};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::net::IpAddr;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterView {
    pub nodes: Vec<NodeInfo>,
    pub leader: Option<NodeId>,
    pub term: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: NodeId,
    pub address: IpAddr,
    pub port: u16,
    pub status: NodeStatus,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeStatus {
    Active,
    Inactive,
    Failed,
    Joining,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub node_ip: IpAddr,
    pub port: u16,
    pub peers: Vec<IpAddr>,
    pub storage_path: String,
    pub max_storage_size: u64,
    pub replication_factor: usize,
    pub consensus_timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ClusterState {
    pub active_nodes: Vec<NodeInfo>,
    pub total_replicas: usize,
    pub is_write_enabled: bool,
}

pub type Result<T> = std::result::Result<T, ConsensusError>;

#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Not leader: current leader is {0:?}")]
    NotLeader(Option<NodeId>),
    
    #[error("Insufficient replicas: {current} < {required}")]
    InsufficientReplicas { current: usize, required: usize },
    
    #[error("Election timeout")]
    ElectionTimeout,
    
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}