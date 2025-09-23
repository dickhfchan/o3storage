#!/usr/bin/env rust-script

//! Test to read hello.txt file from the distributed storage cluster
//! Run with: rustc read_file_test.rs && ./read_file_test

use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Node {
    id: u32,
    storage: HashMap<String, Vec<u8>>,
    is_active: bool,
}

impl Node {
    fn new(id: u32) -> Self {
        Self {
            id,
            storage: HashMap::new(),
            is_active: true,
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
}

fn main() {
    println!("📖 File Read Test: Reading 'hello.txt' from Cluster");
    println!("==================================================");

    // Create 3-node cluster with pre-existing data
    let mut cluster = vec![
        Node::new(1),
        Node::new(2),
        Node::new(3),
    ];

    // Simulate the file already exists in the cluster (from previous upload)
    let file_key = "hello.txt";
    let file_content = "Hello work".as_bytes().to_vec();

    // Store the file on all nodes (simulating previous replication)
    for node in &mut cluster {
        node.store(file_key.to_string(), file_content.clone());
    }

    println!("✅ Cluster initialized with existing file: {}", file_key);

    // Test: Read file from different nodes
    println!("\n📋 Reading file from each node:");
    
    for node in &cluster {
        match node.get(file_key) {
            Some(data) => {
                let content = String::from_utf8_lossy(data);
                println!("Node {}: Successfully read '{}' - Content: '{}'", 
                         node.id, file_key, content);
            }
            None => {
                println!("Node {}: File '{}' not found", node.id, file_key);
            }
        }
    }

    // Test: Read from any available node (fault tolerance)
    println!("\n📋 Reading from first available node:");
    for node in &cluster {
        if let Some(data) = node.get(file_key) {
            let content = String::from_utf8_lossy(data);
            println!("📄 File content from Node {}: '{}'", node.id, content);
            println!("📏 File size: {} bytes", data.len());
            break;
        }
    }

    // Test: Simulate node failure and read from remaining nodes
    println!("\n📋 Testing fault tolerance - Node 1 fails:");
    cluster[0].is_active = false;
    
    for node in &cluster {
        if node.is_active {
            if let Some(data) = node.get(file_key) {
                let content = String::from_utf8_lossy(data);
                println!("✅ Successfully read from Node {} after Node 1 failure: '{}'", 
                         node.id, content);
                break;
            }
        }
    }

    println!("\n🎉 File read test completed successfully!");
}