use thiserror::Error;

#[derive(Error, Debug)]
pub enum O3StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Consensus error: {0}")]
    Consensus(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Hardware requirements not met: {0}")]
    HardwareError(String),

    #[error("Insufficient replicas: {0}")]
    InsufficientReplicas(usize),

    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("System error: {0}")]
    System(String),
}

pub type Result<T> = std::result::Result<T, O3StorageError>;