#!/usr/bin/env rust-script

//! Standalone distributed operations test runner
//! Run with: rustc standalone_test.rs && ./standalone_test

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct MockNode {
    id: u32,
    storage: HashMap<String, Vec<u8>>,
    is_active: bool,
    replication_factor: usize,
}

impl MockNode {
    fn new(id: u32) -> Self {
        Self {
            id,
            storage: HashMap::new(),
            is_active: true,
            replication_factor: 3,
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

    fn replicate_to_cluster(&mut self, key: String, value: Vec<u8>, cluster: &mut [MockNode]) -> bool {
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
    }

    fn recover(&mut self) {
        self.is_active = true;
    }
}

struct TestSuite {
    name: String,
    tests: Vec<TestResult>,
}

#[derive(Debug)]
struct TestResult {
    name: String,
    passed: bool,
    duration: Duration,
    error: Option<String>,
}

impl TestSuite {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tests: Vec::new(),
        }
    }

    fn run_test<F>(&mut self, name: &str, test_fn: F) 
    where 
        F: FnOnce() -> Result<(), String>
    {
        let start = Instant::now();
        match test_fn() {
            Ok(()) => {
                self.tests.push(TestResult {
                    name: name.to_string(),
                    passed: true,
                    duration: start.elapsed(),
                    error: None,
                });
                println!("✅ {}: {}", name, format_duration(start.elapsed()));
            }
            Err(error) => {
                self.tests.push(TestResult {
                    name: name.to_string(),
                    passed: false,
                    duration: start.elapsed(),
                    error: Some(error.clone()),
                });
                println!("❌ {}: {} - {}", name, format_duration(start.elapsed()), error);
            }
        }
    }

    fn print_summary(&self) {
        let passed = self.tests.iter().filter(|t| t.passed).count();
        let total = self.tests.len();
        let failed = total - passed;

        println!("\n📊 {} Test Suite Summary:", self.name);
        println!("   Total: {}, Passed: {}, Failed: {}", total, passed, failed);
        
        if failed > 0 {
            println!("   Failed tests:");
            for test in &self.tests {
                if !test.passed {
                    println!("     - {}: {}", test.name, test.error.as_ref().unwrap_or(&"Unknown error".to_string()));
                }
            }
        }
    }
}

fn format_duration(duration: Duration) -> String {
    if duration.as_millis() > 0 {
        format!("{:.2}ms", duration.as_millis())
    } else {
        format!("{:.2}μs", duration.as_micros())
    }
}

fn main() {
    println!("🚀 O3Storage Distributed Operations Test Suite");
    println!("===============================================");

    // Test Suite 1: Basic Replication
    let mut basic_suite = TestSuite::new("Basic Replication");
    
    basic_suite.run_test("basic_storage", || {
        let mut node = MockNode::new(1);
        node.store("key1".to_string(), b"value1".to_vec());
        
        match node.get("key1") {
            Some(value) if value == &b"value1".to_vec() => Ok(()),
            _ => Err("Failed to store and retrieve basic data".to_string()),
        }
    });

    basic_suite.run_test("cluster_replication", || {
        let mut cluster = vec![
            MockNode::new(1),
            MockNode::new(2), 
            MockNode::new(3),
            MockNode::new(4),
        ];

        let key = "replicated_key".to_string();
        let value = b"replicated_value".to_vec();

        let (first, rest) = cluster.split_at_mut(1);
        let success = first[0].replicate_to_cluster(key.clone(), value.clone(), rest);
        
        if !success {
            return Err("Replication failed".to_string());
        }

        let replica_count = cluster.iter()
            .filter(|node| node.get(&key).is_some())
            .count();

        if replica_count >= 3 {
            Ok(())
        } else {
            Err(format!("Insufficient replicas: {} < 3", replica_count))
        }
    });

    basic_suite.print_summary();

    // Test Suite 2: Fault Tolerance
    let mut fault_suite = TestSuite::new("Fault Tolerance");

    fault_suite.run_test("node_failure_tolerance", || {
        let mut cluster = vec![
            MockNode::new(1),
            MockNode::new(2),
            MockNode::new(3),
            MockNode::new(4),
            MockNode::new(5),
        ];

        // Store data successfully
        let key = "fault_test".to_string();
        let value = b"fault_data".to_vec();
        
        let (first, rest) = cluster.split_at_mut(1);
        let initial_success = first[0].replicate_to_cluster(key.clone(), value.clone(), rest);
        if !initial_success {
            return Err("Initial replication failed".to_string());
        }

        // Simulate node failures
        cluster[2].simulate_failure();
        cluster[3].simulate_failure();

        // Try replication with failed nodes
        let key2 = "fault_test2".to_string();
        let value2 = b"fault_data2".to_vec();
        
        let (first, rest) = cluster.split_at_mut(1);
        let success_with_failures = first[0].replicate_to_cluster(key2.clone(), value2.clone(), rest);
        
        if success_with_failures {
            Ok(())
        } else {
            Err("Replication failed with node failures".to_string())
        }
    });

    fault_suite.run_test("node_recovery", || {
        let mut cluster = vec![
            MockNode::new(1),
            MockNode::new(2),
            MockNode::new(3),
        ];

        // Simulate failure
        cluster[1].simulate_failure();
        cluster[2].simulate_failure();

        // Recover nodes
        cluster[1].recover();
        cluster[2].recover();

        // Test that recovered nodes can participate
        let active_count = cluster.iter().filter(|node| node.is_active).count();
        
        if active_count == 3 {
            Ok(())
        } else {
            Err(format!("Not all nodes recovered: {} active", active_count))
        }
    });

    fault_suite.print_summary();

    // Test Suite 3: Consistency
    let mut consistency_suite = TestSuite::new("Data Consistency");

    consistency_suite.run_test("replica_consistency", || {
        let mut cluster = vec![
            MockNode::new(1),
            MockNode::new(2),
            MockNode::new(3),
            MockNode::new(4),
        ];

        let key = "consistency_test".to_string();
        let value = b"consistent_data".to_vec();

        let (first, rest) = cluster.split_at_mut(1);
        first[0].replicate_to_cluster(key.clone(), value.clone(), rest);

        // Check all replicas have the same data
        let reference_value = cluster[0].get(&key);
        
        for (i, node) in cluster.iter().enumerate() {
            if let Some(node_value) = node.get(&key) {
                if Some(node_value) != reference_value {
                    return Err(format!("Node {} has inconsistent data", i));
                }
            } else if reference_value.is_some() {
                return Err(format!("Node {} missing data", i));
            }
        }

        Ok(())
    });

    consistency_suite.run_test("overwrite_consistency", || {
        let mut cluster = vec![
            MockNode::new(1),
            MockNode::new(2),
            MockNode::new(3),
        ];

        let key = "overwrite_test".to_string();
        let value1 = b"first_value".to_vec();
        let value2 = b"second_value".to_vec();

        // Store initial value
        let (first, rest) = cluster.split_at_mut(1);
        first[0].replicate_to_cluster(key.clone(), value1.clone(), rest);

        // Overwrite with new value
        let (first, rest) = cluster.split_at_mut(1);
        first[0].replicate_to_cluster(key.clone(), value2.clone(), rest);

        // Verify all nodes have the updated value
        for (i, node) in cluster.iter().enumerate() {
            if let Some(node_value) = node.get(&key) {
                if node_value != &value2 {
                    return Err(format!("Node {} has stale data", i));
                }
            } else {
                return Err(format!("Node {} missing updated data", i));
            }
        }

        Ok(())
    });

    consistency_suite.print_summary();

    // Test Suite 4: Quorum Logic
    let mut quorum_suite = TestSuite::new("Quorum Logic");

    quorum_suite.run_test("quorum_calculation", || {
        fn has_quorum(active: usize, total: usize) -> bool {
            active > total / 2
        }

        let test_cases = vec![
            (3, 5, true),   // 3 out of 5 = majority
            (2, 3, true),   // 2 out of 3 = majority  
            (2, 5, false),  // 2 out of 5 = minority
            (1, 3, false),  // 1 out of 3 = minority
        ];

        for (active, total, expected) in test_cases {
            if has_quorum(active, total) != expected {
                return Err(format!("Quorum calculation failed for {}/{}", active, total));
            }
        }

        Ok(())
    });

    quorum_suite.run_test("minimum_replicas", || {
        let mut cluster = vec![
            MockNode::new(1),
            MockNode::new(2),
            MockNode::new(3),
        ];

        // Set replication factor to 2 for this test
        cluster[0].replication_factor = 2;

        let key = "quorum_test".to_string();
        let value = b"quorum_data".to_vec();

        let (first, rest) = cluster.split_at_mut(1);
        let success = first[0].replicate_to_cluster(key.clone(), value.clone(), rest);

        if !success {
            return Err("Failed to meet minimum replication requirements".to_string());
        }

        // Fail one node and try again
        cluster[1].simulate_failure();

        let key2 = "quorum_test2".to_string();
        let value2 = b"quorum_data2".to_vec();

        let (first, rest) = cluster.split_at_mut(1);
        let success2 = first[0].replicate_to_cluster(key2.clone(), value2.clone(), rest);

        if success2 {
            Ok(())
        } else {
            Err("Failed to replicate with reduced cluster".to_string())
        }
    });

    quorum_suite.print_summary();

    println!("\n🎉 All distributed operations tests completed!");
    println!("===============================================");
}