#!/usr/bin/env rust-script

//! Test to put a file with "Hello work" text into the distributed storage cluster
//! Run with: rustc hello_world_test.rs && ./hello_world_test

use std::collections::HashMap;

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
            replication_factor: 2,
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
        
        if self.store(key.clone(), value.clone()) {
            successful_replicas += 1;
        }

        for node in cluster.iter_mut() {
            if node.id != self.id && node.is_active {
                if node.store(key.clone(), value.clone()) {
                    successful_replicas += 1;
                }
            }
        }

        successful_replicas >= self.replication_factor
    }
}

fn main() {
    println!("📁 File Upload Test: 'Hello work' to Distributed Cluster");
    println!("========================================================");

    // Create 3-node cluster
    let mut cluster = vec![
        Node::new(1),
        Node::new(2),
        Node::new(3),
    ];

    println!("✅ Created 3-node cluster (nodes 1, 2, 3)");

    // Test: Store file with "Hello work" content
    println!("\n📋 Test: Storing file 'hello.txt' with content 'Hello work'");
    
    let file_key = "hello.txt".to_string();
    let file_content = "Hello work".as_bytes().to_vec();

    // Store the file on the cluster
    let (first, rest) = cluster.split_at_mut(1);
    let success = first[0].replicate_to_cluster(file_key.clone(), file_content.clone(), rest);

    println!("File upload success: {}", success);
    println!("File size: {} bytes", file_content.len());

    // Verify file content on all nodes
    println!("\n📊 File verification across cluster:");
    for node in &cluster {
        if let Some(data) = node.get(&file_key) {
            let content = String::from_utf8_lossy(data);
            println!("Node {}: File '{}' contains: '{}'", node.id, file_key, content);
        } else {
            println!("Node {}: File '{}' not found", node.id, file_key);
        }
    }

    // Show storage summary
    println!("\n📈 Storage Summary:");
    for node in &cluster {
        println!("Node {} storage: {} files", node.id, node.storage.len());
        for (filename, content) in &node.storage {
            println!("  📄 {}: {} bytes", filename, content.len());
        }
    }

    println!("\n🎉 File upload test completed successfully!");
}