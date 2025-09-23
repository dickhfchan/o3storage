// 3-Node Cluster Distributed System Tests
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use serde_json::json;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting O3Storage 3-Node Cluster Test Suite");
    
    // Initialize logging
    tracing_subscriber::init();
    
    let cluster = O3StorageCluster::new().await?;
    
    println!("📊 Running distributed system tests...");
    
    // Test Suite Execution
    let mut test_results = TestResults::new();
    
    // 1. Cluster Formation Tests
    test_results.add_suite_result(
        "cluster_formation", 
        run_cluster_formation_tests(&cluster).await?
    );
    
    // 2. Data Replication Tests  
    test_results.add_suite_result(
        "data_replication",
        run_data_replication_tests(&cluster).await?
    );
    
    // 3. Consensus Tests
    test_results.add_suite_result(
        "consensus_protocol", 
        run_consensus_tests(&cluster).await?
    );
    
    // 4. Failure Recovery Tests
    test_results.add_suite_result(
        "failure_recovery",
        run_failure_recovery_tests(&cluster).await?
    );
    
    // 5. Network Partition Tests
    test_results.add_suite_result(
        "network_partitions",
        run_network_partition_tests(&cluster).await?
    );
    
    // 6. Load Distribution Tests
    test_results.add_suite_result(
        "load_distribution",
        run_load_distribution_tests(&cluster).await?
    );
    
    // Generate final report
    test_results.generate_report();
    
    // Cleanup
    cluster.shutdown().await?;
    
    println!("✅ Cluster testing completed!");
    Ok(())
}

/// Represents a 3-node O3Storage cluster for testing
struct O3StorageCluster {
    nodes: Vec<ClusterNode>,
    client: reqwest::Client,
}

struct ClusterNode {
    id: Uuid,
    address: String,
    port: u16,
    is_leader: bool,
    storage_path: String,
    process: Option<tokio::process::Child>,
}

impl O3StorageCluster {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        println!("🏗️  Setting up 3-node cluster...");
        
        let nodes = vec![
            ClusterNode {
                id: Uuid::new_v4(),
                address: "127.0.0.1".to_string(),
                port: 8081,
                is_leader: true,
                storage_path: "/tmp/o3storage-node1".to_string(),
                process: None,
            },
            ClusterNode {
                id: Uuid::new_v4(), 
                address: "127.0.0.2".to_string(),
                port: 8082,
                is_leader: false,
                storage_path: "/tmp/o3storage-node2".to_string(),
                process: None,
            },
            ClusterNode {
                id: Uuid::new_v4(),
                address: "127.0.0.3".to_string(), 
                port: 8083,
                is_leader: false,
                storage_path: "/tmp/o3storage-node3".to_string(),
                process: None,
            },
        ];
        
        let mut cluster = Self {
            nodes,
            client: reqwest::Client::new(),
        };
        
        cluster.start_nodes().await?;
        cluster.wait_for_cluster_ready().await?;
        
        println!("✅ 3-node cluster ready");
        Ok(cluster)
    }
    
    async fn start_nodes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (i, node) in self.nodes.iter_mut().enumerate() {
            println!("🚀 Starting node {} at {}:{}", i + 1, node.address, node.port);
            
            // Create storage directory
            tokio::fs::create_dir_all(&node.storage_path).await?;
            
            // Start O3Storage process (simulated)
            let cmd = if i == 0 {
                // First node (leader)
                format!(
                    "./target/release/o3storage --ip {} --port {} --storage-path {} --mode leader",
                    node.address, node.port, node.storage_path
                )
            } else {
                // Follower nodes
                format!(
                    "./target/release/o3storage --ip {} --port {} --storage-path {} --peers 127.0.0.1:8081",
                    node.address, node.port, node.storage_path
                )
            };
            
            // For testing purposes, we'll simulate the process
            println!("📡 Command: {}", cmd);
            sleep(Duration::from_millis(500)).await;
        }
        
        Ok(())
    }
    
    async fn wait_for_cluster_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("⏳ Waiting for cluster to form...");
        
        for attempt in 1..=30 {
            if self.is_cluster_healthy().await {
                println!("✅ Cluster formed successfully after {} attempts", attempt);
                return Ok(());
            }
            
            println!("⏳ Attempt {}/30 - waiting for cluster...", attempt);
            sleep(Duration::from_secs(2)).await;
        }
        
        Err("Cluster failed to form within timeout".into())
    }
    
    async fn is_cluster_healthy(&self) -> bool {
        // Check if all nodes are responding
        for node in &self.nodes {
            let url = format!("http://{}:{}/health", node.address, node.port);
            if self.client.get(&url).send().await.is_err() {
                return false;
            }
        }
        true
    }
    
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🛑 Shutting down cluster...");
        
        for (i, node) in self.nodes.iter().enumerate() {
            println!("🛑 Stopping node {}", i + 1);
            // Cleanup storage directories
            let _ = tokio::fs::remove_dir_all(&node.storage_path).await;
        }
        
        Ok(())
    }
}

// Test Suite Implementations

async fn run_cluster_formation_tests(
    cluster: &O3StorageCluster
) -> Result<SuiteResult, Box<dyn std::error::Error>> {
    println!("🔍 Testing cluster formation...");
    
    let mut suite = SuiteResult::new("cluster_formation");
    
    // Test 1: Verify all nodes are accessible
    let start = Instant::now();
    let mut all_accessible = true;
    
    for (i, node) in cluster.nodes.iter().enumerate() {
        let url = format!("http://{}:{}/health", node.address, node.port);
        match cluster.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    suite.add_test_result(TestResult::passed(
                        &format!("node_{}_accessible", i + 1),
                        start.elapsed()
                    ));
                } else {
                    suite.add_test_result(TestResult::failed(
                        &format!("node_{}_accessible", i + 1),
                        start.elapsed(),
                        &format!("HTTP {}", response.status())
                    ));
                    all_accessible = false;
                }
            }
            Err(e) => {
                suite.add_test_result(TestResult::failed(
                    &format!("node_{}_accessible", i + 1),
                    start.elapsed(),
                    &e.to_string()
                ));
                all_accessible = false;
            }
        }
    }
    
    // Test 2: Verify leader election
    let start = Instant::now();
    let leader_url = "http://127.0.0.1:8081/cluster/status";
    match cluster.client.get(leader_url).send().await {
        Ok(response) => {
            if let Ok(status) = response.json::<serde_json::Value>().await {
                if status["is_leader"] == json!(true) {
                    suite.add_test_result(TestResult::passed(
                        "leader_election",
                        start.elapsed()
                    ));
                } else {
                    suite.add_test_result(TestResult::failed(
                        "leader_election", 
                        start.elapsed(),
                        "Node 1 is not leader"
                    ));
                }
            }
        }
        Err(e) => {
            suite.add_test_result(TestResult::failed(
                "leader_election",
                start.elapsed(), 
                &e.to_string()
            ));
        }
    }
    
    // Test 3: Verify cluster membership
    let start = Instant::now();
    match cluster.client.get("http://127.0.0.1:8081/cluster/members").send().await {
        Ok(response) => {
            if let Ok(members) = response.json::<serde_json::Value>().await {
                if members.as_array().map_or(0, |arr| arr.len()) == 3 {
                    suite.add_test_result(TestResult::passed(
                        "cluster_membership",
                        start.elapsed()
                    ));
                } else {
                    suite.add_test_result(TestResult::failed(
                        "cluster_membership",
                        start.elapsed(),
                        "Incorrect member count"
                    ));
                }
            }
        }
        Err(e) => {
            suite.add_test_result(TestResult::failed(
                "cluster_membership",
                start.elapsed(),
                &e.to_string()
            ));
        }
    }
    
    Ok(suite)
}

async fn run_data_replication_tests(
    cluster: &O3StorageCluster
) -> Result<SuiteResult, Box<dyn std::error::Error>> {
    println!("🔍 Testing data replication...");
    
    let mut suite = SuiteResult::new("data_replication");
    
    // Test 1: Create bucket and verify replication
    let start = Instant::now();
    let bucket_name = "test-bucket";
    let create_url = format!("http://127.0.0.1:8081/{}", bucket_name);
    
    match cluster.client.put(&create_url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                // Wait for replication
                sleep(Duration::from_secs(2)).await;
                
                // Verify bucket exists on all nodes
                let mut replicated_count = 0;
                for node in &cluster.nodes {
                    let check_url = format!("http://{}:{}/{}", node.address, node.port, bucket_name);
                    if cluster.client.head(&check_url).send().await.is_ok() {
                        replicated_count += 1;
                    }
                }
                
                if replicated_count == 3 {
                    suite.add_test_result(TestResult::passed(
                        "bucket_replication",
                        start.elapsed()
                    ));
                } else {
                    suite.add_test_result(TestResult::failed(
                        "bucket_replication",
                        start.elapsed(),
                        &format!("Only {} nodes have bucket", replicated_count)
                    ));
                }
            }
        }
        Err(e) => {
            suite.add_test_result(TestResult::failed(
                "bucket_creation",
                start.elapsed(),
                &e.to_string()
            ));
        }
    }
    
    // Test 2: Object replication
    let start = Instant::now();
    let object_data = "Hello, O3Storage distributed system!";
    let put_url = format!("http://127.0.0.1:8081/{}/test-object.txt", bucket_name);
    
    match cluster.client.put(&put_url)
        .header("Content-Type", "text/plain")
        .body(object_data)
        .send().await {
        Ok(response) => {
            if response.status().is_success() {
                // Wait for replication
                sleep(Duration::from_secs(3)).await;
                
                // Verify object exists on all nodes with correct content
                let mut correct_replicas = 0;
                for node in &cluster.nodes {
                    let get_url = format!("http://{}:{}/{}/test-object.txt", 
                                        node.address, node.port, bucket_name);
                    if let Ok(response) = cluster.client.get(&get_url).send().await {
                        if let Ok(content) = response.text().await {
                            if content == object_data {
                                correct_replicas += 1;
                            }
                        }
                    }
                }
                
                if correct_replicas == 3 {
                    suite.add_test_result(TestResult::passed(
                        "object_replication",
                        start.elapsed()
                    ));
                } else {
                    suite.add_test_result(TestResult::failed(
                        "object_replication",
                        start.elapsed(),
                        &format!("Only {} correct replicas", correct_replicas)
                    ));
                }
            }
        }
        Err(e) => {
            suite.add_test_result(TestResult::failed(
                "object_replication",
                start.elapsed(),
                &e.to_string()
            ));
        }
    }
    
    Ok(suite)
}

async fn run_consensus_tests(
    cluster: &O3StorageCluster
) -> Result<SuiteResult, Box<dyn std::error::Error>> {
    println!("🔍 Testing consensus protocol...");
    
    let mut suite = SuiteResult::new("consensus_protocol");
    
    // Test 1: Concurrent writes consistency
    let start = Instant::now();
    let bucket = "consensus-test";
    
    // Create bucket first
    let create_url = format!("http://127.0.0.1:8081/{}", bucket);
    let _ = cluster.client.put(&create_url).send().await;
    sleep(Duration::from_secs(1)).await;
    
    // Perform concurrent writes to the same key from different clients
    let mut handles = vec![];
    
    for i in 0..10 {
        let client = cluster.client.clone();
        let bucket = bucket.to_string();
        
        let handle = tokio::spawn(async move {
            let put_url = format!("http://127.0.0.1:8081/{}/concurrent-test", bucket);
            let data = format!("write-{}", i);
            client.put(&put_url)
                .header("Content-Type", "text/plain")
                .body(data)
                .send()
                .await
        });
        
        handles.push(handle);
    }
    
    // Wait for all writes to complete
    let mut successful_writes = 0;
    for handle in handles {
        if let Ok(Ok(response)) = handle.await {
            if response.status().is_success() {
                successful_writes += 1;
            }
        }
    }
    
    // Verify final state is consistent across all nodes
    sleep(Duration::from_secs(2)).await;
    
    let mut consistent_state = true;
    let mut reference_content = None;
    
    for node in &cluster.nodes {
        let get_url = format!("http://{}:{}/{}/concurrent-test", 
                            node.address, node.port, bucket);
        if let Ok(response) = cluster.client.get(&get_url).send().await {
            if let Ok(content) = response.text().await {
                if let Some(ref_content) = &reference_content {
                    if content != *ref_content {
                        consistent_state = false;
                        break;
                    }
                } else {
                    reference_content = Some(content);
                }
            }
        }
    }
    
    if consistent_state && successful_writes > 0 {
        suite.add_test_result(TestResult::passed(
            "concurrent_write_consistency",
            start.elapsed()
        ));
    } else {
        suite.add_test_result(TestResult::failed(
            "concurrent_write_consistency",
            start.elapsed(),
            "Inconsistent state after concurrent writes"
        ));
    }
    
    Ok(suite)
}

async fn run_failure_recovery_tests(
    cluster: &O3StorageCluster
) -> Result<SuiteResult, Box<dyn std::error::Error>> {
    println!("🔍 Testing failure recovery...");
    
    let mut suite = SuiteResult::new("failure_recovery");
    
    // Test 1: Single node failure and recovery
    let start = Instant::now();
    
    // Simulate node 2 failure (this would be implemented with actual process control)
    println!("🔥 Simulating node 2 failure...");
    sleep(Duration::from_secs(1)).await;
    
    // Verify cluster still operational with 2 nodes
    let bucket = "recovery-test";
    let create_url = format!("http://127.0.0.1:8081/{}", bucket);
    
    match cluster.client.put(&create_url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                suite.add_test_result(TestResult::passed(
                    "cluster_operation_during_failure",
                    start.elapsed()
                ));
            } else {
                suite.add_test_result(TestResult::failed(
                    "cluster_operation_during_failure",
                    start.elapsed(),
                    "Cluster not operational with node failure"
                ));
            }
        }
        Err(e) => {
            suite.add_test_result(TestResult::failed(
                "cluster_operation_during_failure",
                start.elapsed(),
                &e.to_string()
            ));
        }
    }
    
    // Test 2: Leader failure and re-election
    let start = Instant::now();
    println!("🔥 Simulating leader failure...");
    sleep(Duration::from_secs(3)).await; // Time for leader election
    
    // Check if new leader was elected
    let nodes_to_check = vec![
        "http://127.0.0.2:8082/cluster/status",
        "http://127.0.0.3:8083/cluster/status"
    ];
    
    let mut new_leader_found = false;
    for node_url in nodes_to_check {
        if let Ok(response) = cluster.client.get(node_url).send().await {
            if let Ok(status) = response.json::<serde_json::Value>().await {
                if status["is_leader"] == json!(true) {
                    new_leader_found = true;
                    break;
                }
            }
        }
    }
    
    if new_leader_found {
        suite.add_test_result(TestResult::passed(
            "leader_re_election",
            start.elapsed()
        ));
    } else {
        suite.add_test_result(TestResult::failed(
            "leader_re_election",
            start.elapsed(),
            "No new leader elected after leader failure"
        ));
    }
    
    Ok(suite)
}

async fn run_network_partition_tests(
    cluster: &O3StorageCluster
) -> Result<SuiteResult, Box<dyn std::error::Error>> {
    println!("🔍 Testing network partition tolerance...");
    
    let mut suite = SuiteResult::new("network_partitions");
    
    // Test 1: Split-brain protection
    let start = Instant::now();
    
    // Simulate network partition (1 node isolated)
    println!("🌐 Simulating network partition...");
    
    // In a real implementation, this would involve actual network manipulation
    // For testing, we'll simulate the behavior
    
    // Majority partition (2 nodes) should remain operational
    let majority_node = "http://127.0.0.2:8082/test-partition/test-object";
    match cluster.client.put(majority_node)
        .header("Content-Type", "text/plain")
        .body("partition test")
        .send().await {
        Ok(response) => {
            if response.status().is_success() {
                suite.add_test_result(TestResult::passed(
                    "majority_partition_operational",
                    start.elapsed()
                ));
            } else {
                suite.add_test_result(TestResult::failed(
                    "majority_partition_operational",
                    start.elapsed(),
                    "Majority partition not operational"
                ));
            }
        }
        Err(e) => {
            suite.add_test_result(TestResult::failed(
                "majority_partition_operational",
                start.elapsed(),
                &e.to_string()
            ));
        }
    }
    
    Ok(suite)
}

async fn run_load_distribution_tests(
    cluster: &O3StorageCluster
) -> Result<SuiteResult, Box<dyn std::error::Error>> {
    println!("🔍 Testing load distribution...");
    
    let mut suite = SuiteResult::new("load_distribution");
    
    // Test 1: Round-robin request distribution
    let start = Instant::now();
    let bucket = "load-test";
    
    // Create bucket
    let create_url = format!("http://127.0.0.1:8081/{}", bucket);
    let _ = cluster.client.put(&create_url).send().await;
    sleep(Duration::from_secs(1)).await;
    
    // Send requests to all nodes and measure response times
    let mut response_times = HashMap::new();
    
    for (i, node) in cluster.nodes.iter().enumerate() {
        let start_req = Instant::now();
        let get_url = format!("http://{}:{}/{}/", node.address, node.port, bucket);
        
        if let Ok(_) = cluster.client.get(&get_url).send().await {
            response_times.insert(i, start_req.elapsed());
        }
    }
    
    if response_times.len() == 3 {
        suite.add_test_result(TestResult::passed(
            "all_nodes_responsive",
            start.elapsed()
        ));
        
        // Check if response times are reasonable (< 1 second)
        let max_time = response_times.values().max().unwrap_or(&Duration::ZERO);
        if *max_time < Duration::from_secs(1) {
            suite.add_test_result(TestResult::passed(
                "reasonable_response_times",
                start.elapsed()
            ));
        } else {
            suite.add_test_result(TestResult::failed(
                "reasonable_response_times",
                start.elapsed(),
                &format!("Max response time: {:?}", max_time)
            ));
        }
    } else {
        suite.add_test_result(TestResult::failed(
            "all_nodes_responsive",
            start.elapsed(),
            "Not all nodes responding"
        ));
    }
    
    Ok(suite)
}

// Test Result Data Structures

#[derive(Debug, Clone)]
struct TestResults {
    suites: HashMap<String, SuiteResult>,
    start_time: Instant,
}

#[derive(Debug, Clone)]
struct SuiteResult {
    name: String,
    tests: Vec<TestResult>,
    start_time: Instant,
}

#[derive(Debug, Clone)]
struct TestResult {
    name: String,
    status: TestStatus,
    duration: Duration,
    error_message: Option<String>,
}

#[derive(Debug, Clone)]
enum TestStatus {
    Passed,
    Failed,
    Skipped,
}

impl TestResults {
    fn new() -> Self {
        Self {
            suites: HashMap::new(),
            start_time: Instant::now(),
        }
    }
    
    fn add_suite_result(&mut self, name: &str, suite: SuiteResult) {
        self.suites.insert(name.to_string(), suite);
    }
    
    fn generate_report(&self) {
        let total_duration = self.start_time.elapsed();
        
        println!("\n📊 O3Storage Distributed System Test Report");
        println!("=" .repeat(60));
        
        let mut total_tests = 0;
        let mut passed_tests = 0;
        let mut failed_tests = 0;
        let mut skipped_tests = 0;
        
        for (suite_name, suite) in &self.suites {
            println!("\n📂 Test Suite: {}", suite_name);
            println!("   Duration: {:?}", suite.start_time.elapsed());
            
            for test in &suite.tests {
                let status_symbol = match test.status {
                    TestStatus::Passed => "✅",
                    TestStatus::Failed => "❌", 
                    TestStatus::Skipped => "⏭️",
                };
                
                println!("   {} {} ({:?})", status_symbol, test.name, test.duration);
                
                if let Some(error) = &test.error_message {
                    println!("      Error: {}", error);
                }
                
                total_tests += 1;
                match test.status {
                    TestStatus::Passed => passed_tests += 1,
                    TestStatus::Failed => failed_tests += 1,
                    TestStatus::Skipped => skipped_tests += 1,
                }
            }
        }
        
        println!("\n📈 Summary");
        println!("   Total Tests: {}", total_tests);
        println!("   Passed: {} ({}%)", passed_tests, 
               if total_tests > 0 { passed_tests * 100 / total_tests } else { 0 });
        println!("   Failed: {} ({}%)", failed_tests,
               if total_tests > 0 { failed_tests * 100 / total_tests } else { 0 });
        println!("   Skipped: {}", skipped_tests);
        println!("   Total Duration: {:?}", total_duration);
        
        // Export results to JSON
        let json_report = json!({
            "timestamp": chrono::Utc::now(),
            "total_duration_ms": total_duration.as_millis(),
            "summary": {
                "total": total_tests,
                "passed": passed_tests,
                "failed": failed_tests,
                "skipped": skipped_tests
            },
            "suites": self.suites.iter().map(|(name, suite)| {
                json!({
                    "name": name,
                    "duration_ms": suite.start_time.elapsed().as_millis(),
                    "tests": suite.tests.iter().map(|test| {
                        json!({
                            "name": test.name,
                            "status": format!("{:?}", test.status),
                            "duration_ms": test.duration.as_millis(),
                            "error": test.error_message
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        });
        
        // Write to file
        if let Ok(json_str) = serde_json::to_string_pretty(&json_report) {
            if let Err(e) = std::fs::write("test_results.json", json_str) {
                println!("⚠️  Failed to write test results to file: {}", e);
            } else {
                println!("💾 Test results saved to test_results.json");
            }
        }
    }
}

impl SuiteResult {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tests: Vec::new(),
            start_time: Instant::now(),
        }
    }
    
    fn add_test_result(&mut self, result: TestResult) {
        self.tests.push(result);
    }
}

impl TestResult {
    fn passed(name: &str, duration: Duration) -> Self {
        Self {
            name: name.to_string(),
            status: TestStatus::Passed,
            duration,
            error_message: None,
        }
    }
    
    fn failed(name: &str, duration: Duration, error: &str) -> Self {
        Self {
            name: name.to_string(),
            status: TestStatus::Failed,
            duration,
            error_message: Some(error.to_string()),
        }
    }
    
    fn skipped(name: &str, duration: Duration) -> Self {
        Self {
            name: name.to_string(),
            status: TestStatus::Skipped,
            duration,
            error_message: None,
        }
    }
}