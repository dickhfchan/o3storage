mod engine;
mod object;
mod metadata;
mod versioning;

pub use engine::StorageEngine;
pub use object::{Object, ObjectId, ObjectMetadata, ObjectReference};
pub use metadata::MetadataStore;
pub use versioning::{Version, VersionedObject};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_objects: u64,
    pub total_size_bytes: u64,
    pub used_space_bytes: u64,
    pub available_space_bytes: u64,
    pub replication_status: HashMap<ObjectId, ReplicationStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationStatus {
    pub replicas: Vec<Uuid>, // Node IDs where replicas exist
    pub target_replicas: usize,
    pub is_fully_replicated: bool,
}

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Object not found: {0}")]
    ObjectNotFound(String),
    
    #[error("Insufficient space: {0}")]
    InsufficientSpace(String),
    
    #[error("Corruption detected: {0}")]
    Corruption(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Invalid object: {0}")]
    InvalidObject(String),
}

impl From<bincode::Error> for StorageError {
    fn from(err: bincode::Error) -> Self {
        StorageError::Serialization(err.to_string())
    }
}