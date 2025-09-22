use serde::{Deserialize, Serialize};
use std::net::IpAddr;

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
    pub fn new(node_ip: IpAddr, port: u16, peers: Vec<IpAddr>) -> Self {
        Self {
            node_ip,
            port,
            peers,
            storage_path: "/var/lib/o3storage".to_string(),
            max_storage_size: 50 * 1024 * 1024 * 1024 * 1024, // 50TB
            replication_factor: 3,
            consensus_timeout_ms: 5000,
            heartbeat_interval_ms: 1000,
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.node_ip, self.port)
    }
}

impl From<Config> for network::Config {
    fn from(config: Config) -> Self {
        network::Config {
            node_ip: config.node_ip,
            port: config.port,
            peers: config.peers,
            max_storage_size: config.max_storage_size,
            heartbeat_interval_ms: config.heartbeat_interval_ms,
            consensus_timeout_ms: config.consensus_timeout_ms,
        }
    }
}

impl From<Config> for consensus::Config {
    fn from(config: Config) -> Self {
        consensus::Config {
            node_ip: config.node_ip,
            port: config.port,
            peers: config.peers,
            storage_path: config.storage_path,
            max_storage_size: config.max_storage_size,
            replication_factor: config.replication_factor,
            consensus_timeout_ms: config.consensus_timeout_ms,
            heartbeat_interval_ms: config.heartbeat_interval_ms,
        }
    }
}

impl From<Config> for api::Config {
    fn from(config: Config) -> Self {
        api::Config {
            node_ip: config.node_ip,
            port: config.port,
            peers: config.peers,
            storage_path: config.storage_path,
            max_storage_size: config.max_storage_size,
            replication_factor: config.replication_factor,
            consensus_timeout_ms: config.consensus_timeout_ms,
            heartbeat_interval_ms: config.heartbeat_interval_ms,
        }
    }
}