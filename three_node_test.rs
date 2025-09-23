#!/usr/bin/env rust-script

//! Three-node cluster test
//! Run with: rustc three_node_test.rs && ./three_node_test

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct Node {
    id: u32,
    storage: HashMap<String, Vec<u8>>,
    is_active: bool,
    replication_factor: usize,
}

impl Node {
    fn new(id: u32) -> Self {
        Self {
            id,
            storage: HashMap::new(),
            is_active: true,
            replication_factor: 2, // Majority of 3 nodes
        }
    }

    fn store(&mut self, key: String, value: Vec<u8>) -> bool {
        if !self.is_active {
            return false;
        }
        self.storage.insert(key, value);
        true
    }

    fn get(&self, key: &str) -> Option<&Vec<u8>> {
        if !self.is_active {
            return None;
        }
        self.storage.get(key)
    }

    fn replicate_to_cluster(&mut self, key: String, value: Vec<u8>, cluster: &mut [Node]) -> bool {
        if !self.is_active {
            return false;
        }

        let mut successful_replicas = 0;
        
        // Store on this node first
        if self.store(key.clone(), value.clone()) {
            successful_replicas += 1;
        }

        // Replicate to other active nodes
        for node in cluster.iter_mut() {
            if node.id != self.id && node.is_active {
                if node.store(key.clone(), value.clone()) {
                    successful_replicas += 1;
                }
            }
        }

        successful_replicas >= self.replication_factor
    }

    fn simulate_failure(&mut self) {
        self.is_active = false;
        println!("Node {} failed", self.id);
    }

    fn recover(&mut self) {
        self.is_active = true;
        println!("Node {} recovered", self.id);
    }
}

fn main() {
    println!("🔧 Three-Node Cluster Test");
    println!("==========================");

    // Create 3-node cluster
    let mut cluster = vec![
        Node::new(1),
        Node::new(2),
        Node::new(3),
    ];

    println!("✅ Created 3-node cluster (nodes 1, 2, 3)");

    // Test 1: Basic replication across all 3 nodes
    println!("\n📋 Test 1: Basic 3-node replication");
    let key1 = "test_key_1".to_string();
    let value1 = b"test_value_1".to_vec();

    let (first, rest) = cluster.split_at_mut(1);
    let success = first[0].replicate_to_cluster(key1.clone(), value1.clone(), rest);

    println!("Replication success: {}", success);

    // Check data on all nodes
    for (i, node) in cluster.iter().enumerate() {
        if let Some(data) = node.get(&key1) {
            println!("Node {}: has data '{}'", node.id, String::from_utf8_lossy(data));
        } else {
            println!("Node {}: no data", node.id);
        }
    }

    // Test 2: One node failure scenario
    println!("\n📋 Test 2: One node failure");
    cluster[1].simulate_failure(); // Fail node 2

    let key2 = "test_key_2".to_string();
    let value2 = b"test_value_2".to_vec();

    let (first, rest) = cluster.split_at_mut(1);
    let success_with_failure = first[0].replicate_to_cluster(key2.clone(), value2.clone(), rest);

    println!("Replication with 1 node down: {}", success_with_failure);

    // Check which nodes have the new data
    for node in &cluster {
        if let Some(data) = node.get(&key2) {
            println!("Node {}: has data '{}'", node.id, String::from_utf8_lossy(data));
        } else {
            println!("Node {}: no data", node.id);
        }
    }

    // Test 3: Two nodes failure (should fail)
    println!("\n📋 Test 3: Two nodes failure (expecting failure)");
    cluster[2].simulate_failure(); // Fail node 3 as well

    let key3 = "test_key_3".to_string();
    let value3 = b"test_value_3".to_vec();

    let (first, rest) = cluster.split_at_mut(1);
    let success_with_two_failures = first[0].replicate_to_cluster(key3.clone(), value3.clone(), rest);

    println!("Replication with 2 nodes down: {}", success_with_two_failures);

    // Test 4: Node recovery
    println!("\n📋 Test 4: Node recovery");
    cluster[1].recover(); // Recover node 2

    let key4 = "test_key_4".to_string();
    let value4 = b"test_value_4".to_vec();

    let (first, rest) = cluster.split_at_mut(1);
    let success_after_recovery = first[0].replicate_to_cluster(key4.clone(), value4.clone(), rest);

    println!("Replication after recovery: {}", success_after_recovery);

    // Check final cluster state
    println!("\n📊 Final cluster state:");
    for node in &cluster {
        println!("Node {} ({}): {} keys stored", 
                 node.id, 
                 if node.is_active { "active" } else { "failed" },
                 node.storage.len());
        for (key, value) in &node.storage {
            println!("  {}: {}", key, String::from_utf8_lossy(value));
        }
    }

    // Test 5: Quorum validation
    println!("\n📋 Test 5: Quorum validation");
    let active_nodes = cluster.iter().filter(|n| n.is_active).count();
    let has_quorum = active_nodes > cluster.len() / 2;
    println!("Active nodes: {}/{}", active_nodes, cluster.len());
    println!("Has quorum: {}", has_quorum);

    println!("\n🎉 Three-node cluster tests completed!");
}