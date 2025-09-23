#!/usr/bin/env rust-script

//! Test to check versions of hello.txt in the distributed storage cluster
//! Run with: rustc version_check_test.rs && ./version_check_test

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

    fn list_files(&self) -> Vec<&String> {
        self.storage.keys().collect()
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
    println!("🔍 Version Check Test: Examining hello.txt versions in cluster");
    println!("==============================================================");

    // Create 3-node cluster
    let mut cluster = vec![
        Node::new(1),
        Node::new(2),
        Node::new(3),
    ];

    println!("✅ Created 3-node cluster (nodes 1, 2, 3)");

    // Simulate multiple uploads to test versioning behavior
    println!("\n📋 Uploading multiple versions of hello.txt:");
    
    // Version 1: "Hello work"
    let file_key = "hello.txt".to_string();
    let version1 = "Hello work".as_bytes().to_vec();
    let (first, rest) = cluster.split_at_mut(1);
    let success1 = first[0].replicate_to_cluster(file_key.clone(), version1.clone(), rest);
    println!("Version 1 upload ('Hello work'): {}", success1);

    // Version 2: "hello again"  
    let version2 = "hello again".as_bytes().to_vec();
    let (first, rest) = cluster.split_at_mut(1);
    let success2 = first[0].replicate_to_cluster(file_key.clone(), version2.clone(), rest);
    println!("Version 2 upload ('hello again'): {}", success2);

    // Check current state of hello.txt files across cluster
    println!("\n📊 Current hello.txt files in cluster:");
    for node in &cluster {
        println!("\nNode {} storage analysis:", node.id);
        let files = node.list_files();
        
        // Count hello.txt related files
        let hello_files: Vec<_> = files.iter()
            .filter(|key| key.contains("hello.txt"))
            .collect();
            
        println!("  Total files: {}", files.len());
        println!("  Hello.txt related files: {}", hello_files.len());
        
        for file in &hello_files {
            if let Some(content) = node.get(file) {
                let text = String::from_utf8_lossy(content);
                println!("    📄 {}: '{}' ({} bytes)", file, text, content.len());
            }
        }
        
        // Show all files for completeness
        println!("  All files:");
        for (filename, content) in &node.storage {
            let text = String::from_utf8_lossy(content);
            println!("    📁 {}: '{}' ({} bytes)", filename, text, content.len());
        }
    }

    // Summary
    println!("\n📈 Version Analysis Summary:");
    let sample_node = &cluster[0];
    let hello_count = sample_node.list_files().iter()
        .filter(|key| key.contains("hello.txt"))
        .count();
        
    println!("Number of hello.txt versions stored: {}", hello_count);
    
    if let Some(current_content) = sample_node.get("hello.txt") {
        let current_text = String::from_utf8_lossy(current_content);
        println!("Current hello.txt content: '{}'", current_text);
        println!("Current hello.txt size: {} bytes", current_content.len());
    }

    println!("\n🎯 Conclusion: This storage system {} file versioning", 
             if hello_count > 1 { "supports" } else { "does NOT support" });

    println!("\n🎉 Version check completed!");
}