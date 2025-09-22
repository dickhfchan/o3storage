use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use std::net::SocketAddr;

use crate::{Result, NetworkError, NetworkNode, NetworkMessage, Config};

pub struct MessageHandler {
    node_id: Uuid,
    connections: Arc<RwLock<HashMap<Uuid, Connection>>>,
    message_sender: mpsc::UnboundedSender<(Uuid, NetworkMessage)>,
    message_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<(Uuid, NetworkMessage)>>>>,
    config: Config,
}

pub struct Connection {
    pub node_id: Uuid,
    pub address: SocketAddr,
    pub status: ConnectionStatus,
    pub last_activity: std::time::Instant,
    pub message_sender: mpsc::UnboundedSender<NetworkMessage>,
}

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Failed,
}

impl MessageHandler {
    pub fn new(node_id: Uuid, config: Config) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            node_id,
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(Some(rx))),
            config,
        }
    }

    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting message handler for node {}", self.node_id);

        let mut receiver = {
            let mut guard = self.message_receiver.write().await;
            guard.take().ok_or_else(|| {
                NetworkError::Protocol("Message receiver already taken".to_string())
            })?
        };

        let message_processing_task = {
            let connections = self.connections.clone();
            tokio::spawn(async move {
                Self::process_messages(receiver, connections).await
            })
        };

        let connection_maintenance_task = {
            let connections = self.connections.clone();
            let config = self.config.clone();
            tokio::spawn(async move {
                Self::maintain_connections(connections, config).await
            })
        };

        tokio::select! {
            result = message_processing_task => {
                tracing::error!("Message processing task failed: {:?}", result);
                Err(NetworkError::Protocol("Message processing failed".to_string()))
            }
            result = connection_maintenance_task => {
                tracing::error!("Connection maintenance task failed: {:?}", result);
                Err(NetworkError::ConnectionFailed("Connection maintenance failed".to_string()))
            }
        }
    }

    pub async fn send_message(&self, target_node: Uuid, message: NetworkMessage) -> Result<()> {
        let connections = self.connections.read().await;
        
        if let Some(connection) = connections.get(&target_node) {
            connection.message_sender.send(message)
                .map_err(|_| NetworkError::ConnectionFailed(
                    format!("Failed to send message to node {}", target_node)
                ))?;
            Ok(())
        } else {
            Err(NetworkError::ConnectionFailed(
                format!("No connection to node {}", target_node)
            ))
        }
    }

    pub async fn broadcast_message(&self, message: NetworkMessage) -> Result<()> {
        let connections = self.connections.read().await;
        let mut failed_nodes = Vec::new();

        for (node_id, connection) in connections.iter() {
            if let Err(_) = connection.message_sender.send(message.clone()) {
                failed_nodes.push(*node_id);
            }
        }

        if !failed_nodes.is_empty() {
            tracing::warn!("Failed to broadcast to {} nodes: {:?}", failed_nodes.len(), failed_nodes);
        }

        Ok(())
    }

    pub async fn add_connection(&self, node: NetworkNode) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        if connections.contains_key(&node.id) {
            tracing::debug!("Connection to node {} already exists", node.id);
            return Ok(());
        }

        let address = SocketAddr::new(node.address, node.port);
        let (tx, rx) = mpsc::unbounded_channel();

        let connection = Connection {
            node_id: node.id,
            address,
            status: ConnectionStatus::Connecting,
            last_activity: std::time::Instant::now(),
            message_sender: tx,
        };

        connections.insert(node.id, connection);
        drop(connections);

        // Spawn connection handler
        let node_id = node.id;
        let connections_ref = self.connections.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::handle_connection(node_id, address, rx, connections_ref).await {
                tracing::error!("Connection handler for node {} failed: {}", node_id, e);
            }
        });

        tracing::info!("Added connection to node {} at {}", node.id, address);
        Ok(())
    }

    pub async fn remove_connection(&self, node_id: Uuid) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(_connection) = connections.remove(&node_id) {
            tracing::info!("Removed connection to node {}", node_id);
        }

        Ok(())
    }

    pub async fn get_connected_nodes(&self) -> Vec<Uuid> {
        let connections = self.connections.read().await;
        connections.keys().copied().collect()
    }

    async fn process_messages(
        mut receiver: mpsc::UnboundedReceiver<(Uuid, NetworkMessage)>,
        connections: Arc<RwLock<HashMap<Uuid, Connection>>>,
    ) -> Result<()> {
        while let Some((from_node, message)) = receiver.recv().await {
            if let Err(e) = Self::handle_incoming_message(from_node, message, &connections).await {
                tracing::error!("Failed to handle message from node {}: {}", from_node, e);
            }
        }
        Ok(())
    }

    async fn handle_incoming_message(
        from_node: Uuid,
        message: NetworkMessage,
        connections: &Arc<RwLock<HashMap<Uuid, Connection>>>,
    ) -> Result<()> {
        // Update last activity for the connection
        {
            let mut connections_guard = connections.write().await;
            if let Some(connection) = connections_guard.get_mut(&from_node) {
                connection.last_activity = std::time::Instant::now();
            }
        }

        match message {
            NetworkMessage::Ping(ping) => {
                tracing::trace!("Received ping from node {}", from_node);
                // TODO: Send pong response
            }
            NetworkMessage::Pong(pong) => {
                tracing::trace!("Received pong from node {}", from_node);
                // TODO: Update RTT metrics
            }
            NetworkMessage::Discovery(discovery) => {
                tracing::debug!("Received discovery message from node {}", from_node);
                // TODO: Handle discovery message
            }
            NetworkMessage::DiscoveryResponse(response) => {
                tracing::debug!("Received discovery response from node {}", from_node);
                // TODO: Update peer list
            }
            NetworkMessage::Consensus(consensus_msg) => {
                tracing::trace!("Received consensus message from node {}", from_node);
                // TODO: Forward to consensus manager
            }
            NetworkMessage::Storage(storage_msg) => {
                tracing::trace!("Received storage message from node {}", from_node);
                // TODO: Forward to storage engine
            }
            NetworkMessage::Cluster(cluster_msg) => {
                tracing::debug!("Received cluster message from node {}", from_node);
                // TODO: Handle cluster management message
            }
        }

        Ok(())
    }

    async fn handle_connection(
        node_id: Uuid,
        address: SocketAddr,
        mut message_receiver: mpsc::UnboundedReceiver<NetworkMessage>,
        connections: Arc<RwLock<HashMap<Uuid, Connection>>>,
    ) -> Result<()> {
        tracing::debug!("Starting connection handler for node {} at {}", node_id, address);

        // Update connection status to connected
        {
            let mut connections_guard = connections.write().await;
            if let Some(connection) = connections_guard.get_mut(&node_id) {
                connection.status = ConnectionStatus::Connected;
            }
        }

        // TODO: Establish actual network connection (TCP/HTTP/etc.)
        // For now, just simulate message handling

        while let Some(message) = message_receiver.recv().await {
            if let Err(e) = Self::send_network_message(address, &message).await {
                tracing::error!("Failed to send message to {}: {}", address, e);
                
                // Mark connection as failed
                let mut connections_guard = connections.write().await;
                if let Some(connection) = connections_guard.get_mut(&node_id) {
                    connection.status = ConnectionStatus::Failed;
                }
                break;
            }
        }

        // Clean up connection
        {
            let mut connections_guard = connections.write().await;
            connections_guard.remove(&node_id);
        }

        tracing::info!("Connection handler for node {} terminated", node_id);
        Ok(())
    }

    async fn send_network_message(
        address: SocketAddr,
        message: &NetworkMessage,
    ) -> Result<()> {
        // TODO: Implement actual network message sending
        // This could be HTTP POST, TCP connection, UDP packet, etc.
        // For now, just simulate successful sending
        
        tracing::trace!("Sending message to {}: {:?}", address, message);
        
        // Simulate network delay
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        
        Ok(())
    }

    async fn maintain_connections(
        connections: Arc<RwLock<HashMap<Uuid, Connection>>>,
        config: Config,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(
            std::time::Duration::from_millis(config.heartbeat_interval_ms)
        );

        loop {
            interval.tick().await;

            let mut to_remove = Vec::new();
            let timeout_duration = std::time::Duration::from_millis(config.consensus_timeout_ms);

            {
                let connections_guard = connections.read().await;
                for (node_id, connection) in connections_guard.iter() {
                    if connection.last_activity.elapsed() > timeout_duration {
                        to_remove.push(*node_id);
                    }
                }
            }

            if !to_remove.is_empty() {
                let mut connections_guard = connections.write().await;
                for node_id in to_remove {
                    if let Some(_) = connections_guard.remove(&node_id) {
                        tracing::warn!("Removed inactive connection to node {}", node_id);
                    }
                }
            }
        }
    }
}