# O3StorageOS Cluster Management Guide

## Overview
This guide covers advanced cluster management procedures for O3StorageOS, including scaling, maintenance, recovery, and administration of multi-node deployments.

## Table of Contents
1. [Cluster Architecture](#cluster-architecture)
2. [Cluster Initialization](#cluster-initialization)
3. [Node Management](#node-management)
4. [Scaling Operations](#scaling-operations)
5. [Maintenance Procedures](#maintenance-procedures)
6. [Disaster Recovery](#disaster-recovery)
7. [Security Management](#security-management)
8. [Advanced Operations](#advanced-operations)

## Cluster Architecture

### Consensus and Leadership
O3StorageOS uses Raft consensus algorithm with the following roles:

- **Leader**: Handles all write operations and coordinates replication
- **Follower**: Replicates data from leader and can serve read requests
- **Candidate**: Temporary state during leader election

### Quorum Requirements
- **Minimum cluster size**: 3 nodes (recommended)
- **Quorum size**: (N/2) + 1 nodes must be available for writes
- **Fault tolerance**: Can lose (N-1)/2 nodes and maintain availability

Example configurations:
- 3 nodes: Can lose 1 node
- 5 nodes: Can lose 2 nodes  
- 7 nodes: Can lose 3 nodes

## Cluster Initialization

### Bootstrap New Cluster

**Step 1: Prepare Infrastructure**
```bash
# Ensure all nodes can communicate
for node in 192.168.1.101 192.168.1.102 192.168.1.103; do
    ping -c 3 $node
    telnet $node 8080
done

# Synchronize time across nodes
sudo ntpdate -s time.nist.gov
```

**Step 2: Start Bootstrap Node**
```bash
# On first node (192.168.1.101)
./o3storage --ip 192.168.1.101 --port 8080 --bootstrap

# Verify leadership
curl -s http://192.168.1.101:8080/health | jq '.cluster'
```

**Step 3: Join Additional Nodes**
```bash
# On second node (192.168.1.102)
./o3storage --ip 192.168.1.102 --port 8080 --peers 192.168.1.101

# On third node (192.168.1.103)  
./o3storage --ip 192.168.1.103 --port 8080 --peers 192.168.1.101
```

**Step 4: Verify Cluster Formation**
```bash
# Check cluster status on each node
for node in 192.168.1.101 192.168.1.102 192.168.1.103; do
    echo "Node $node:"
    curl -s http://$node:8080/health | jq '.cluster.active_nodes'
done
```

### Cluster Configuration Management

**Create Cluster Config File** (`cluster.json`):
```json
{
  "cluster_id": "o3storage-prod",
  "nodes": [
    {
      "id": "node-1",
      "ip": "192.168.1.101",
      "port": 8080,
      "role": "leader"
    },
    {
      "id": "node-2", 
      "ip": "192.168.1.102",
      "port": 8080,
      "role": "follower"
    },
    {
      "id": "node-3",
      "ip": "192.168.1.103", 
      "port": 8080,
      "role": "follower"
    }
  ],
  "replication_factor": 3,
  "consistency_level": "strong"
}
```

## Node Management

### Add New Node to Existing Cluster

**Step 1: Prepare New Node**
```bash
# On new node (192.168.1.104)
# Install O3Storage following ARM_DEPLOYMENT_GUIDE.md

# Ensure network connectivity to cluster
for node in 192.168.1.101 192.168.1.102 192.168.1.103; do
    telnet $node 8080
done
```

**Step 2: Join Cluster**
```bash
# Start new node pointing to existing cluster
./o3storage --ip 192.168.1.104 --port 8080 --peers 192.168.1.101,192.168.1.102,192.168.1.103
```

**Step 3: Verify Addition**
```bash
# Check that all nodes see the new member
for node in 192.168.1.101 192.168.1.102 192.168.1.103 192.168.1.104; do
    echo "Node $node sees:"
    curl -s http://$node:8080/health | jq '.cluster.active_nodes'
done
```

### Remove Node from Cluster

**Graceful Removal**
```bash
# Stop the node service
ssh 192.168.1.104 "sudo systemctl stop o3storage"

# Verify cluster adapts (remaining nodes form quorum)
curl -s http://192.168.1.101:8080/health | jq '.cluster'
```

**Forced Removal** (if node is unresponsive)
```bash
# From cluster leader, remove dead node
curl -X POST http://192.168.1.101:8080/admin/remove-node \
  -H "Content-Type: application/json" \
  -d '{"node_id": "192.168.1.104:8080"}'
```

### Replace Failed Node

**Step 1: Identify Failed Node**
```bash
# Check cluster health
curl -s http://192.168.1.101:8080/health | jq '.cluster'

# Identify which node is missing
for node in 192.168.1.101 192.168.1.102 192.168.1.103; do
    echo -n "Node $node: "
    curl -f -s http://$node:8080/health >/dev/null && echo "OK" || echo "FAILED"
done
```

**Step 2: Deploy Replacement**
```bash
# Deploy new node with same configuration
# On replacement node (192.168.1.105)
./o3storage --ip 192.168.1.105 --port 8080 --peers 192.168.1.101,192.168.1.102

# Update DNS/load balancer to point to new node
```

**Step 3: Verify Recovery**
```bash
# Check cluster stability
watch 'curl -s http://192.168.1.101:8080/health | jq ".cluster"'
```

## Scaling Operations

### Scale Up (Add Nodes)

**Horizontal Scaling Strategy**
```bash
# Add nodes in pairs for optimal distribution
# Add nodes 4 and 5
./o3storage --ip 192.168.1.104 --port 8080 --peers 192.168.1.101,192.168.1.102,192.168.1.103
./o3storage --ip 192.168.1.105 --port 8080 --peers 192.168.1.101,192.168.1.102,192.168.1.103,192.168.1.104

# Wait for data rebalancing
./monitor-rebalance.sh
```

**Monitor Rebalancing**
```bash
#!/bin/bash
# monitor-rebalance.sh

while true; do
    echo "$(date): Checking rebalance status..."
    
    for node in 192.168.1.101 192.168.1.102 192.168.1.103 192.168.1.104 192.168.1.105; do
        objects=$(curl -s http://$node:8080/health | jq '.storage.total_objects')
        echo "Node $node: $objects objects"
    done
    
    echo "---"
    sleep 30
done
```

### Scale Down (Remove Nodes)

**Graceful Scale Down**
```bash
# Remove nodes one at a time, starting with highest IP
# Stop node 5 first
ssh 192.168.1.105 "sudo systemctl stop o3storage"

# Wait for data rebalancing to complete
./monitor-rebalance.sh

# Then remove node 4
ssh 192.168.1.104 "sudo systemctl stop o3storage"
```

**Verify Scaling**
```bash
# Check final cluster state
curl -s http://192.168.1.101:8080/health | jq
```

## Maintenance Procedures

### Rolling Updates

**Step 1: Update Strategy**
```bash
# Update nodes in reverse order (followers first, leader last)
UPDATE_ORDER=(192.168.1.103 192.168.1.102 192.168.1.101)
NEW_BINARY="o3storage-v2.0.0-arm64"
```

**Step 2: Update Each Node**
```bash
#!/bin/bash
# rolling-update.sh

for node in "${UPDATE_ORDER[@]}"; do
    echo "Updating node $node..."
    
    # Stop service
    ssh $node "sudo systemctl stop o3storage"
    
    # Backup current binary  
    ssh $node "sudo cp /opt/o3storage/o3storage /opt/o3storage/o3storage.backup"
    
    # Deploy new binary
    scp $NEW_BINARY $node:/tmp/o3storage-new
    ssh $node "sudo mv /tmp/o3storage-new /opt/o3storage/o3storage"
    ssh $node "sudo chown o3storage:o3storage /opt/o3storage/o3storage"
    ssh $node "sudo chmod +x /opt/o3storage/o3storage"
    
    # Start service
    ssh $node "sudo systemctl start o3storage"
    
    # Wait for node to rejoin cluster
    echo "Waiting for $node to rejoin cluster..."
    while ! curl -f -s http://$node:8080/health >/dev/null; do
        sleep 5
    done
    
    # Verify cluster health
    active_nodes=$(curl -s http://$node:8080/health | jq '.cluster.active_nodes')
    echo "Node $node rejoined. Cluster has $active_nodes active nodes."
    
    # Wait before updating next node
    echo "Waiting 60 seconds before next update..."
    sleep 60
done

echo "Rolling update completed!"
```

### Cluster Maintenance Mode

**Enable Maintenance Mode**
```bash
# Disable writes across cluster
curl -X POST http://192.168.1.101:8080/admin/maintenance \
  -d '{"enabled": true, "reason": "scheduled maintenance"}'

# Verify maintenance mode
curl -s http://192.168.1.101:8080/health | jq '.cluster.write_enabled'
```

**Perform Maintenance Tasks**
```bash
# Example: Clean up old log files
for node in 192.168.1.101 192.168.1.102 192.168.1.103; do
    ssh $node "sudo find /var/log/o3storage -name '*.log.*' -mtime +7 -delete"
done

# Example: Compact storage
for node in 192.168.1.101 192.168.1.102 192.168.1.103; do
    ssh $node "sudo systemctl stop o3storage"
    ssh $node "sudo /opt/o3storage/compact-storage.sh"
    ssh $node "sudo systemctl start o3storage"
    sleep 30
done
```

**Disable Maintenance Mode**
```bash
curl -X POST http://192.168.1.101:8080/admin/maintenance \
  -d '{"enabled": false}'
```

### Health Monitoring During Maintenance

**Automated Health Check**
```bash
#!/bin/bash
# health-monitor.sh

NODES=(192.168.1.101 192.168.1.102 192.168.1.103)
ALERT_EMAIL="admin@company.com"

while true; do
    unhealthy_nodes=0
    
    for node in "${NODES[@]}"; do
        if ! curl -f -s http://$node:8080/health >/dev/null; then
            echo "$(date): Node $node is unhealthy!"
            unhealthy_nodes=$((unhealthy_nodes + 1))
        fi
    done
    
    # Alert if quorum is at risk
    if [ $unhealthy_nodes -gt 1 ]; then
        echo "$(date): CRITICAL - Multiple nodes unhealthy!" | \
          mail -s "O3Storage Cluster Alert" $ALERT_EMAIL
    fi
    
    sleep 60
done
```

## Disaster Recovery

### Backup Procedures

**Full Cluster Backup**
```bash
#!/bin/bash
# cluster-backup.sh

BACKUP_DIR="/backup/o3storage/$(date +%Y%m%d_%H%M%S)"
NODES=(192.168.1.101 192.168.1.102 192.168.1.103)

mkdir -p "$BACKUP_DIR"

echo "Starting cluster backup..."

# Stop all nodes for consistent backup
for node in "${NODES[@]}"; do
    echo "Stopping node $node..."
    ssh $node "sudo systemctl stop o3storage"
done

# Backup each node's data
for i in "${!NODES[@]}"; do
    node=${NODES[$i]}
    echo "Backing up node $node..."
    
    ssh $node "sudo tar -czf /tmp/node-data.tar.gz /opt/o3storage/data"
    scp $node:/tmp/node-data.tar.gz "$BACKUP_DIR/node-$((i+1))-data.tar.gz"
    ssh $node "sudo rm /tmp/node-data.tar.gz"
done

# Backup cluster configuration
cp cluster.json "$BACKUP_DIR/"

# Restart all nodes
for node in "${NODES[@]}"; do
    echo "Starting node $node..."
    ssh $node "sudo systemctl start o3storage"
done

echo "Cluster backup completed: $BACKUP_DIR"
```

### Disaster Recovery Scenarios

**Scenario 1: Single Node Failure**
```bash
# Replace failed node following "Replace Failed Node" procedure above
# Data is automatically replicated from healthy nodes
```

**Scenario 2: Majority Node Failure (Quorum Lost)**
```bash
# Stop all remaining nodes
for node in 192.168.1.101 192.168.1.102; do
    ssh $node "sudo systemctl stop o3storage"
done

# Restore from backup on all nodes
BACKUP_DATE="20241201_143000"
for i in 1 2 3; do
    node_ip="192.168.1.10$i"
    echo "Restoring node $i ($node_ip)..."
    
    # Restore data
    scp "/backup/o3storage/$BACKUP_DATE/node-$i-data.tar.gz" $node_ip:/tmp/
    ssh $node_ip "sudo rm -rf /opt/o3storage/data/*"
    ssh $node_ip "sudo tar -xzf /tmp/node-$i-data.tar.gz -C /"
    ssh $node_ip "sudo chown -R o3storage:o3storage /opt/o3storage/data"
done

# Start cluster with bootstrap mode
ssh 192.168.1.101 "./o3storage --ip 192.168.1.101 --port 8080 --bootstrap"
sleep 30

# Start other nodes
ssh 192.168.1.102 "./o3storage --ip 192.168.1.102 --port 8080 --peers 192.168.1.101"
ssh 192.168.1.103 "./o3storage --ip 192.168.1.103 --port 8080 --peers 192.168.1.101"
```

**Scenario 3: Complete Cluster Failure**
```bash
# Full cluster restoration from backup
./restore-cluster-from-backup.sh /backup/o3storage/20241201_143000
```

### Point-in-Time Recovery

**Create Point-in-Time Snapshot**
```bash
#!/bin/bash
# snapshot-cluster.sh

SNAPSHOT_NAME="snapshot-$(date +%Y%m%d_%H%M%S)"
NODES=(192.168.1.101 192.168.1.102 192.168.1.103)

# Create consistent snapshot across all nodes
for node in "${NODES[@]}"; do
    curl -X POST http://$node:8080/admin/snapshot \
      -d "{\"name\": \"$SNAPSHOT_NAME\"}"
done

echo "Snapshot created: $SNAPSHOT_NAME"
```

**Restore from Snapshot**
```bash
SNAPSHOT_NAME="snapshot-20241201_143000"

for node in "${NODES[@]}"; do
    curl -X POST http://$node:8080/admin/restore \
      -d "{\"snapshot\": \"$SNAPSHOT_NAME\"}"
done
```

## Security Management

### Cluster Authentication

**Generate Cluster Certificates**
```bash
# Create CA certificate
openssl genrsa -out ca-key.pem 4096
openssl req -new -x509 -days 365 -key ca-key.pem -out ca-cert.pem

# Generate node certificates
for i in 1 2 3; do
    openssl genrsa -out node-$i-key.pem 4096
    openssl req -new -key node-$i-key.pem -out node-$i.csr \
      -subj "/CN=192.168.1.10$i"
    openssl x509 -req -days 365 -in node-$i.csr -CA ca-cert.pem \
      -CAkey ca-key.pem -out node-$i-cert.pem -CAcreateserial
done
```

**Deploy Certificates**
```bash
for i in 1 2 3; do
    node_ip="192.168.1.10$i"
    scp ca-cert.pem node-$i-cert.pem node-$i-key.pem $node_ip:/opt/o3storage/certs/
    ssh $node_ip "sudo chown -R o3storage:o3storage /opt/o3storage/certs"
    ssh $node_ip "sudo chmod 600 /opt/o3storage/certs/*.pem"
done
```

### Access Control

**API Key Management**
```bash
# Generate API keys for different access levels
API_KEYS=(
    "admin:$(openssl rand -hex 32)"
    "read-write:$(openssl rand -hex 32)"  
    "read-only:$(openssl rand -hex 32)"
)

# Store keys securely
for key in "${API_KEYS[@]}"; do
    echo "$key" >> /opt/o3storage/api-keys.txt
done

sudo chmod 600 /opt/o3storage/api-keys.txt
```

### Network Security

**Configure Cluster Firewall**
```bash
#!/bin/bash
# cluster-firewall.sh

CLUSTER_IPS="192.168.1.101,192.168.1.102,192.168.1.103"
ALLOWED_CLIENTS="10.0.0.0/8,172.16.0.0/12"

for node in 192.168.1.101 192.168.1.102 192.168.1.103; do
    ssh $node "
        # Reset UFW
        sudo ufw --force reset
        
        # Allow cluster communication
        for ip in \$(echo $CLUSTER_IPS | tr ',' ' '); do
            sudo ufw allow from \$ip to any port 8080
            sudo ufw allow from \$ip to any port 8090
        done
        
        # Allow client access
        for subnet in \$(echo $ALLOWED_CLIENTS | tr ',' ' '); do
            sudo ufw allow from \$subnet to any port 8080
        done
        
        # Enable firewall
        sudo ufw --force enable
    "
done
```

## Advanced Operations

### Data Rebalancing

**Trigger Manual Rebalancing**
```bash
# Initiate rebalancing after cluster topology changes
curl -X POST http://192.168.1.101:8080/admin/rebalance \
  -d '{"strategy": "even_distribution"}'

# Monitor rebalancing progress
watch 'curl -s http://192.168.1.101:8080/admin/rebalance-status | jq'
```

### Cluster Splitting and Merging

**Split Cluster** (for maintenance or geographical distribution)
```bash
# Create two separate clusters from one
# Cluster A: nodes 1-2
# Cluster B: nodes 3

# Stop all nodes
for node in 192.168.1.101 192.168.1.102 192.168.1.103; do
    ssh $node "sudo systemctl stop o3storage"
done

# Start Cluster A
ssh 192.168.1.101 "./o3storage --ip 192.168.1.101 --port 8080 --bootstrap"
ssh 192.168.1.102 "./o3storage --ip 192.168.1.102 --port 8080 --peers 192.168.1.101"

# Start Cluster B  
ssh 192.168.1.103 "./o3storage --ip 192.168.1.103 --port 8080 --bootstrap"
```

### Performance Monitoring

**Real-time Cluster Metrics**
```bash
#!/bin/bash
# cluster-metrics.sh

while true; do
    clear
    echo "O3Storage Cluster Metrics - $(date)"
    echo "================================="
    
    for node in 192.168.1.101 192.168.1.102 192.168.1.103; do
        echo "Node $node:"
        health=$(curl -s http://$node:8080/health 2>/dev/null)
        
        if [ $? -eq 0 ]; then
            objects=$(echo "$health" | jq -r '.storage.total_objects')
            used_space=$(echo "$health" | jq -r '.storage.used_space_bytes')
            active_nodes=$(echo "$health" | jq -r '.cluster.active_nodes')
            
            echo "  Objects: $objects"
            echo "  Used Space: $(numfmt --to=iec-i --suffix=B $used_space)"
            echo "  Cluster Size: $active_nodes nodes"
        else
            echo "  Status: UNREACHABLE"
        fi
        echo ""
    done
    
    sleep 10
done
```

This cluster management guide provides comprehensive procedures for maintaining and operating O3StorageOS clusters in production environments. Regular monitoring, proper backup procedures, and following these operational guidelines will ensure high availability and reliability of your distributed storage system.