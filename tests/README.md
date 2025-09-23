# O3Storage Testing Framework

Comprehensive test suite for O3StorageOS distributed storage system with 3-node cluster simulation.

## 🧪 **Test Categories**

### **1. Unit Tests**
- Individual component functionality
- Cryptographic operations
- Storage format validation
- Network protocol handling

### **2. Integration Tests** 
- Multi-component interactions
- API endpoint testing
- Consensus protocol validation
- Storage replication

### **3. Distributed System Tests**
- 3-node cluster operations
- Network partition tolerance
- Leader election
- Data consistency
- Failover scenarios

### **4. Performance Tests**
- Throughput benchmarks
- Latency measurements
- Resource utilization
- Scalability testing

### **5. Security Tests**
- Cryptographic validation
- Memory safety verification
- Attack surface analysis
- Penetration testing

## 🏗️ **Test Infrastructure**

### **Cluster Simulation Setup**
```
Node 1 (Leader)     Node 2 (Follower)    Node 3 (Follower)
┌─────────────┐     ┌─────────────────┐   ┌─────────────────┐
│ 127.0.0.1   │     │ 127.0.0.2       │   │ 127.0.0.3       │
│ Port: 8081  │◄────┤ Port: 8082      │   │ Port: 8083      │
│ Storage: /1 │     │ Storage: /2     │   │ Storage: /3     │
└─────────────┘     └─────────────────┘   └─────────────────┘
```

## 🚀 **Quick Start**

```bash
# Build test binaries
cargo build --release --bin o3storage-test

# Run complete test suite
./scripts/run-tests.sh

# Run specific test category
./scripts/run-tests.sh --category integration

# Start 3-node cluster for manual testing
./scripts/start-cluster.sh
```

## 📊 **Test Results Format**

Tests output structured results in JSON format:
```json
{
  "test_suite": "distributed_consensus",
  "timestamp": "2024-01-15T10:30:00Z",
  "results": {
    "passed": 45,
    "failed": 2,
    "skipped": 3,
    "total_time": "125.3s"
  },
  "details": [...]
}
```

## 🔍 **Test Categories Detail**

### **Storage Engine Tests**
- Object CRUD operations
- Versioning and metadata
- Integrity validation
- Space management

### **Network Stack Tests** 
- TCP connection handling
- S3 protocol compliance
- TLS encryption
- Error handling

### **Consensus Tests**
- Leader election
- Log replication
- Network partitions
- Recovery scenarios

### **Security Tests**
- Cryptographic functions
- Memory safety
- Attack resistance
- Audit compliance