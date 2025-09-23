#!/usr/bin/env rust-script

//! Versioned Distributed Storage System
//! Supports file versioning with timestamps and version numbers
//! Run with: rustc versioned_storage.rs && ./versioned_storage

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct FileVersion {
    version_number: u32,
    content: Vec<u8>,
    timestamp: u64,
    size: usize,
}

impl FileVersion {
    fn new(version_number: u32, content: Vec<u8>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let size = content.len();
        
        Self {
            version_number,
            content,
            timestamp,
            size,
        }
    }
}

#[derive(Debug, Clone)]
struct VersionedFile {
    filename: String,
    versions: Vec<FileVersion>,
    latest_version: u32,
}

impl VersionedFile {
    fn new(filename: String) -> Self {
        Self {
            filename,
            versions: Vec::new(),
            latest_version: 0,
        }
    }

    fn add_version(&mut self, content: Vec<u8>) -> u32 {
        self.latest_version += 1;
        let version = FileVersion::new(self.latest_version, content);
        self.versions.push(version);
        self.latest_version
    }

    fn get_version(&self, version_number: u32) -> Option<&FileVersion> {
        self.versions.iter()
            .find(|v| v.version_number == version_number)
    }

    fn get_latest(&self) -> Option<&FileVersion> {
        self.versions.last()
    }

    fn list_versions(&self) -> Vec<u32> {
        self.versions.iter()
            .map(|v| v.version_number)
            .collect()
    }

    fn version_count(&self) -> usize {
        self.versions.len()
    }
}

#[derive(Debug, Clone)]
struct VersionedNode {
    id: u32,
    storage: HashMap<String, VersionedFile>,
    is_active: bool,
    replication_factor: usize,
}

impl VersionedNode {
    fn new(id: u32) -> Self {
        Self {
            id,
            storage: HashMap::new(),
            is_active: true,
            replication_factor: 2,
        }
    }

    fn store_version(&mut self, filename: String, content: Vec<u8>) -> Option<u32> {
        if !self.is_active {
            return None;
        }

        let version_number = if let Some(file) = self.storage.get_mut(&filename) {
            file.add_version(content)
        } else {
            let mut new_file = VersionedFile::new(filename.clone());
            let version = new_file.add_version(content);
            self.storage.insert(filename, new_file);
            version
        };

        Some(version_number)
    }

    fn get_version(&self, filename: &str, version_number: u32) -> Option<&Vec<u8>> {
        if !self.is_active {
            return None;
        }

        self.storage.get(filename)?
            .get_version(version_number)
            .map(|v| &v.content)
    }

    fn get_latest(&self, filename: &str) -> Option<&Vec<u8>> {
        if !self.is_active {
            return None;
        }

        self.storage.get(filename)?
            .get_latest()
            .map(|v| &v.content)
    }

    fn list_file_versions(&self, filename: &str) -> Vec<u32> {
        if !self.is_active {
            return Vec::new();
        }

        self.storage.get(filename)
            .map(|f| f.list_versions())
            .unwrap_or_default()
    }

    fn list_files(&self) -> Vec<String> {
        if !self.is_active {
            return Vec::new();
        }

        self.storage.keys().cloned().collect()
    }

    fn get_file_info(&self, filename: &str) -> Option<(u32, usize)> {
        if !self.is_active {
            return None;
        }

        self.storage.get(filename).map(|f| (f.latest_version, f.version_count()))
    }

    fn replicate_to_cluster(&mut self, filename: String, content: Vec<u8>, cluster: &mut [VersionedNode]) -> Option<u32> {
        if !self.is_active {
            return None;
        }

        let mut successful_replicas = 0;
        let mut version_number = None;

        if let Some(version) = self.store_version(filename.clone(), content.clone()) {
            successful_replicas += 1;
            version_number = Some(version);
        }

        for node in cluster.iter_mut() {
            if node.id != self.id && node.is_active {
                if node.store_version(filename.clone(), content.clone()).is_some() {
                    successful_replicas += 1;
                }
            }
        }

        if successful_replicas >= self.replication_factor {
            version_number
        } else {
            None
        }
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
    println!("🗂️  Versioned Distributed Storage System Test");
    println!("============================================");

    // Create 3-node cluster
    let mut cluster = vec![
        VersionedNode::new(1),
        VersionedNode::new(2),
        VersionedNode::new(3),
    ];

    println!("✅ Created 3-node versioned cluster (nodes 1, 2, 3)");

    // Test 1: Upload multiple versions of hello.txt
    println!("\n📋 Test 1: Uploading multiple versions of hello.txt");
    
    let filename = "hello.txt".to_string();
    
    // Version 1
    let content1 = "Hello work".as_bytes().to_vec();
    let (first, rest) = cluster.split_at_mut(1);
    if let Some(v1) = first[0].replicate_to_cluster(filename.clone(), content1, rest) {
        println!("✅ Version {} uploaded: 'Hello work'", v1);
    }

    // Version 2
    let content2 = "hello again".as_bytes().to_vec();
    let (first, rest) = cluster.split_at_mut(1);
    if let Some(v2) = first[0].replicate_to_cluster(filename.clone(), content2, rest) {
        println!("✅ Version {} uploaded: 'hello again'", v2);
    }

    // Version 3
    let content3 = "hello version 3".as_bytes().to_vec();
    let (first, rest) = cluster.split_at_mut(1);
    if let Some(v3) = first[0].replicate_to_cluster(filename.clone(), content3, rest) {
        println!("✅ Version {} uploaded: 'hello version 3'", v3);
    }

    // Test 2: List all versions
    println!("\n📋 Test 2: Listing all versions of hello.txt");
    for node in &cluster {
        let versions = node.list_file_versions(&filename);
        println!("Node {}: {} versions found: {:?}", node.id, versions.len(), versions);
    }

    // Test 3: Retrieve specific versions
    println!("\n📋 Test 3: Retrieving specific versions");
    let test_node = &cluster[0];
    
    for version in [1, 2, 3] {
        if let Some(content) = test_node.get_version(&filename, version) {
            let text = String::from_utf8_lossy(content);
            println!("Version {}: '{}'", version, text);
        }
    }

    // Test 4: Get latest version
    println!("\n📋 Test 4: Getting latest version");
    if let Some(latest_content) = test_node.get_latest(&filename) {
        let text = String::from_utf8_lossy(latest_content);
        println!("Latest version: '{}'", text);
    }

    // Test 5: File information
    println!("\n📋 Test 5: File information");
    for node in &cluster {
        if let Some((latest_ver, total_versions)) = node.get_file_info(&filename) {
            println!("Node {}: Latest version {}, Total versions: {}", 
                     node.id, latest_ver, total_versions);
        }
    }

    // Test 6: Fault tolerance with versioning
    println!("\n📋 Test 6: Testing fault tolerance");
    cluster[1].simulate_failure();
    
    // Add version 4 with one node down
    let content4 = "hello version 4 (with node failure)".as_bytes().to_vec();
    let (first, rest) = cluster.split_at_mut(1);
    if let Some(v4) = first[0].replicate_to_cluster(filename.clone(), content4, rest) {
        println!("✅ Version {} uploaded during node failure", v4);
    }

    // Check versions on remaining active nodes
    for node in &cluster {
        if node.is_active {
            let versions = node.list_file_versions(&filename);
            println!("Active Node {}: {} versions: {:?}", node.id, versions.len(), versions);
        }
    }

    println!("\n🎉 Versioned storage system test completed!");
}