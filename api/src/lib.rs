mod server;
mod handlers;
mod auth;
mod xml;
mod error;

pub use server::Server;
pub use error::{ApiError, ApiResult};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// Shared types
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

impl Config {
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.node_ip, self.port)
    }
}

#[derive(Debug, Clone)]
pub struct ClusterState {
    pub active_nodes: Vec<NodeInfo>,
    pub total_replicas: usize,
    pub is_write_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: Uuid,
    pub address: IpAddr,
    pub port: u16,
    pub last_heartbeat: DateTime<Utc>,
    pub status: NodeStatus,
}

#[derive(Debug, Clone)]
pub enum NodeStatus {
    Active,
    Inactive,
    Joining,
    Leaving,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Response<T> {
    pub data: T,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBucketsResponse {
    pub buckets: Vec<BucketInfo>,
    pub owner: Owner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    pub name: String,
    pub creation_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListObjectsV2Response {
    pub is_truncated: bool,
    pub contents: Vec<ObjectInfo>,
    pub name: String,
    pub prefix: Option<String>,
    pub delimiter: Option<String>,
    pub max_keys: u32,
    pub common_prefixes: Vec<CommonPrefix>,
    pub encoding_type: Option<String>,
    pub key_count: u32,
    pub continuation_token: Option<String>,
    pub next_continuation_token: Option<String>,
    pub start_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub key: String,
    pub last_modified: DateTime<Utc>,
    pub etag: String,
    pub size: u64,
    pub storage_class: String,
    pub owner: Option<Owner>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonPrefix {
    pub prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutObjectResponse {
    pub etag: String,
    pub version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetObjectResponse {
    pub data: bytes::Bytes,
    pub content_type: String,
    pub content_length: u64,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
    pub version_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteObjectResponse {
    pub version_id: Option<String>,
    pub delete_marker: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadObjectResponse {
    pub content_type: String,
    pub content_length: u64,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
    pub version_id: Option<String>,
    pub metadata: HashMap<String, String>,
}