mod manager;
mod discovery;
mod communication;

pub use manager::NetworkManager;
pub use discovery::{PeerDiscovery, DiscoveryMethod};
pub use communication::MessageHandler;

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkNode {
    pub id: Uuid,
    pub address: IpAddr,
    pub port: u16,
    pub last_seen: DateTime<Utc>,
    pub status: NodeStatus,
    pub capabilities: NodeCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeStatus {
    Active,
    Inactive,
    Joining,
    Leaving,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub storage_capacity: u64,
    pub available_space: u64,
    pub supported_protocols: Vec<String>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    Ping(PingMessage),
    Pong(PongMessage),
    Discovery(DiscoveryMessage),
    DiscoveryResponse(DiscoveryResponseMessage),
    Consensus(consensus::ConsensusMessage),
    Storage(StorageMessage),
    Cluster(ClusterMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    pub from: Uuid,
    pub timestamp: DateTime<Utc>,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongMessage {
    pub from: Uuid,
    pub to: Uuid,
    pub timestamp: DateTime<Utc>,
    pub sequence: u64,
    pub round_trip_time: Option<std::time::Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryMessage {
    pub from: NetworkNode,
    pub cluster_id: Option<String>,
    pub seeking_cluster: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResponseMessage {
    pub from: NetworkNode,
    pub cluster_nodes: Vec<NetworkNode>,
    pub cluster_id: String,
    pub is_leader: bool,
    pub leader_node: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageMessage {
    ObjectRequest {
        request_id: Uuid,
        object_id: String,
        operation: StorageOperation,
    },
    ObjectResponse {
        request_id: Uuid,
        success: bool,
        data: Option<bytes::Bytes>,
        error: Option<String>,
    },
    ReplicationSync {
        objects: Vec<ObjectSyncInfo>,
        from_node: Uuid,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageOperation {
    Get,
    Put(bytes::Bytes),
    Delete,
    Verify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSyncInfo {
    pub object_id: String,
    pub checksum: String,
    pub version_id: Uuid,
    pub size: u64,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    Join(JoinRequest),
    JoinResponse(JoinResponse),
    Leave(LeaveRequest),
    NodeUpdate(NodeUpdateMessage),
    ClusterStatus(ClusterStatusMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    pub node: NetworkNode,
    pub cluster_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinResponse {
    pub accepted: bool,
    pub cluster_id: String,
    pub cluster_nodes: Vec<NetworkNode>,
    pub leader: Option<Uuid>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRequest {
    pub node_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeUpdateMessage {
    pub node: NetworkNode,
    pub update_type: NodeUpdateType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeUpdateType {
    StatusChange,
    CapabilityUpdate,
    AddressChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatusMessage {
    pub cluster_id: String,
    pub active_nodes: Vec<NetworkNode>,
    pub leader: Option<Uuid>,
    pub total_capacity: u64,
    pub available_capacity: u64,
    pub replication_status: ReplicationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationStatus {
    pub target_replicas: usize,
    pub min_replicas_met: bool,
    pub objects_under_replicated: u64,
    pub objects_over_replicated: u64,
}

pub type Result<T> = std::result::Result<T, NetworkError>;

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Discovery failed: {0}")]
    Discovery(String),
    
    #[error("Authentication failed: {0}")]
    Authentication(String),
}

// Dummy config for now
#[derive(Debug, Clone)]
pub struct Config {
    pub node_ip: IpAddr,
    pub port: u16,
    pub peers: Vec<IpAddr>,
    pub max_storage_size: u64,
    pub heartbeat_interval_ms: u64,
    pub consensus_timeout_ms: u64,
}