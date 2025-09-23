use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct MockNode {
    id: Uuid,
    address: IpAddr,
    port: u16,
    status: NodeStatus,
    storage: HashMap<String, Vec<u8>>,
    network_partition: bool,
    replication_factor: usize,
}

#[derive(Debug, Clone)]
enum NodeStatus {
    Active,
    Failed,
}

impl MockNode {
    fn new(id: Uuid, address: IpAddr, port: u16) -> Self {
        Self {
            id,
            address,
            port,
            status: NodeStatus::Active,
            storage: HashMap::new(),
            network_partition: false,
            replication_factor: 3,
        }
    }

    fn replicate_data(&mut self, key: String, data: Vec<u8>, target_nodes: &mut [MockNode]) -> bool {
        let mut successful_replicas = 1;
        self.storage.insert(key.clone(), data.clone());

        for node in target_nodes.iter_mut() {
            if node.id != self.id && !node.network_partition && matches!(node.status, NodeStatus::Active) {
                node.storage.insert(key.clone(), data.clone());
                successful_replicas += 1;
            }
        }

        successful_replicas >= self.replication_factor
    }

    fn simulate_network_partition(&mut self, partitioned: bool) {
        self.network_partition = partitioned;
        if partitioned {
            self.status = NodeStatus::Failed;
        } else {
            self.status = NodeStatus::Active;
        }
    }
}

#[tokio::test]
async fn test_basic_replication() {
    let mut nodes = vec![
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8001),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8002),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8003),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8004),
    ];

    let key = "test_object".to_string();
    let data = b"test_data".to_vec();
    
    let success = nodes[0].replicate_data(key.clone(), data.clone(), &mut nodes[1..]);
    assert!(success, "Data replication should succeed with sufficient nodes");

    let replicated_count = nodes
        .iter()
        .filter(|node| node.storage.contains_key(&key))
        .count();
    
    assert!(replicated_count >= 3, "Data should be replicated to at least 3 nodes");
}

#[tokio::test]
async fn test_partition_tolerance() {
    let mut nodes = vec![
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8001),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8002),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8003),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8004),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8005),
    ];

    let key = "partition_test".to_string();
    let data = b"partition_data".to_vec();
    
    let success_before_partition = nodes[0].replicate_data(key.clone(), data.clone(), &mut nodes[1..]);
    assert!(success_before_partition, "Replication should work before partition");

    nodes[3].simulate_network_partition(true);
    nodes[4].simulate_network_partition(true);

    let key2 = "partition_test2".to_string();
    let data2 = b"partition_data2".to_vec();
    
    let success_during_partition = nodes[0].replicate_data(key2.clone(), data2.clone(), &mut nodes[1..]);
    assert!(success_during_partition, "Replication should still work with majority available");

    nodes[3].simulate_network_partition(false);
    nodes[4].simulate_network_partition(false);

    let active_nodes_after_recovery = nodes.iter()
        .filter(|node| matches!(node.status, NodeStatus::Active))
        .count();
    
    assert_eq!(active_nodes_after_recovery, 5, "All nodes should be active after partition recovery");
}

#[tokio::test]
async fn test_data_consistency() {
    let mut nodes = vec![
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8001),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8002),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8003),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8004),
    ];

    let key = "consistency_test".to_string();
    let data = b"consistent_data".to_vec();
    
    let replication_success = nodes[0].replicate_data(key.clone(), data.clone(), &mut nodes[1..]);
    assert!(replication_success, "Initial replication should succeed");

    let mut consistent_replicas = 0;
    for node in &nodes {
        if let Some(stored_data) = node.storage.get(&key) {
            if stored_data == &data {
                consistent_replicas += 1;
            }
        }
    }

    assert!(consistent_replicas >= 3, "At least 3 nodes should have consistent data");

    let updated_data = b"updated_consistent_data".to_vec();
    let update_success = nodes[0].replicate_data(key.clone(), updated_data.clone(), &mut nodes[1..]);
    assert!(update_success, "Data update should succeed");

    let mut updated_replicas = 0;
    for node in &nodes {
        if let Some(stored_data) = node.storage.get(&key) {
            if stored_data == &updated_data {
                updated_replicas += 1;
            }
        }
    }

    assert!(updated_replicas >= 3, "Updated data should be consistent across replicas");
}

#[tokio::test]
async fn test_quorum_requirements() {
    let mut nodes = vec![
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8001),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8002),
        MockNode::new(Uuid::new_v4(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8003),
    ];
    
    nodes[0].replication_factor = 2;

    let key = "quorum_test".to_string();
    let data = b"quorum_data".to_vec();
    
    let success_with_quorum = nodes[0].replicate_data(key.clone(), data.clone(), &mut nodes[1..]);
    assert!(success_with_quorum, "Replication should succeed with quorum");

    nodes[1].simulate_network_partition(true);
    nodes[2].simulate_network_partition(true);

    let key2 = "quorum_test2".to_string();
    let data2 = b"quorum_data2".to_vec();
    
    let success_without_quorum = nodes[0].replicate_data(key2.clone(), data2.clone(), &mut nodes[1..]);
    assert!(!success_without_quorum, "Replication should fail without quorum");
}