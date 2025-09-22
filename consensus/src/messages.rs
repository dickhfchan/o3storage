use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{NodeId, NodeInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessage {
    Heartbeat(HeartbeatMessage),
    VoteRequest(VoteRequest),
    VoteResponse(VoteResponse),
    AppendEntries(AppendEntriesRequest),
    AppendEntriesResponse(AppendEntriesResponse),
    ReplicationRequest(ReplicationRequest),
    ReplicationResponse(ReplicationResponse),
    JoinRequest(JoinRequest),
    JoinResponse(JoinResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    pub from: NodeId,
    pub term: u64,
    pub timestamp: DateTime<Utc>,
    pub is_leader: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    pub candidate_id: NodeId,
    pub term: u64,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResponse {
    pub from: NodeId,
    pub term: u64,
    pub vote_granted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    pub leader_id: NodeId,
    pub term: u64,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    pub from: NodeId,
    pub term: u64,
    pub success: bool,
    pub match_index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub index: u64,
    pub term: u64,
    pub timestamp: DateTime<Utc>,
    pub entry_type: LogEntryType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEntryType {
    ObjectReplication {
        object_id: String,
        operation: ReplicationOperation,
        metadata: ObjectReplicationMetadata,
    },
    ClusterConfig {
        nodes: Vec<NodeInfo>,
    },
    NoOp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationOperation {
    Store,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectReplicationMetadata {
    pub bucket: String,
    pub key: String,
    pub version_id: Uuid,
    pub size: u64,
    pub checksum: String,
    pub target_nodes: Vec<NodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationRequest {
    pub request_id: Uuid,
    pub object_id: String,
    pub operation: ReplicationOperation,
    pub metadata: ObjectReplicationMetadata,
    pub data: Option<bytes::Bytes>,
    pub from: NodeId,
    pub target_replicas: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationResponse {
    pub request_id: Uuid,
    pub from: NodeId,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    pub node_info: NodeInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinResponse {
    pub accepted: bool,
    pub current_leader: Option<NodeId>,
    pub cluster_view: Vec<NodeInfo>,
    pub reason: Option<String>,
}