# O3StorageOS Operating Manual

## Table of Contents
1. [System Overview](#system-overview)
2. [API Reference](#api-reference)
3. [Basic Operations](#basic-operations)
4. [Advanced Usage](#advanced-usage)
5. [Monitoring and Maintenance](#monitoring-and-maintenance)
6. [Troubleshooting](#troubleshooting)
7. [Performance Tuning](#performance-tuning)

## System Overview

O3StorageOS provides a distributed, S3-compatible object storage system with the following key components:

- **Storage Engine**: Immutable object storage with versioning
- **Consensus Manager**: Raft-based cluster coordination
- **API Server**: S3-compatible HTTP interface
- **Network Layer**: Custom TCP/IP stack for cluster communication

### Architecture
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│     Node 1      │────│     Node 2      │────│     Node 3      │
│   (Leader)      │    │   (Follower)    │    │   (Follower)    │
├─────────────────┤    ├─────────────────┤    ├─────────────────┤
│   S3 API:8080   │    │   S3 API:8080   │    │   S3 API:8080   │
│   Consensus     │    │   Consensus     │    │   Consensus     │
│   Storage       │    │   Storage       │    │   Storage       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## API Reference

### S3-Compatible API Endpoints

#### Bucket Operations

**Create Bucket**
```http
PUT /{bucket}
Host: node-ip:8080
Authorization: AWS4-HMAC-SHA256 ...
```

**List Buckets**
```http
GET /
Host: node-ip:8080
Authorization: AWS4-HMAC-SHA256 ...
```

**Delete Bucket**
```http
DELETE /{bucket}
Host: node-ip:8080
Authorization: AWS4-HMAC-SHA256 ...
```

#### Object Operations

**Put Object**
```http
PUT /{bucket}/{key}
Host: node-ip:8080
Content-Type: application/octet-stream
Content-Length: {size}
Authorization: AWS4-HMAC-SHA256 ...
x-amz-meta-{name}: {value}

[object data]
```

**Get Object**
```http
GET /{bucket}/{key}?versionId={version-id}
Host: node-ip:8080
Authorization: AWS4-HMAC-SHA256 ...
```

**Head Object**
```http
HEAD /{bucket}/{key}
Host: node-ip:8080
Authorization: AWS4-HMAC-SHA256 ...
```

**Delete Object**
```http
DELETE /{bucket}/{key}?versionId={version-id}
Host: node-ip:8080
Authorization: AWS4-HMAC-SHA256 ...
```

**List Objects**
```http
GET /{bucket}?list-type=2&max-keys=1000&prefix={prefix}
Host: node-ip:8080
Authorization: AWS4-HMAC-SHA256 ...
```

#### System Operations

**Health Check**
```http
GET /health
Host: node-ip:8080
```

Response:
```json
{
  "status": "healthy",
  "cluster": {
    "active_nodes": 3,
    "total_replicas": 3,
    "write_enabled": true
  },
  "storage": {
    "total_objects": 1250,
    "used_space_bytes": 5368709120,
    "available_space_bytes": 994631680000
  }
}
```

## Basic Operations

### Starting the System

**Single Node**
```bash
# Interactive setup
./o3storage

# Command line
./o3storage --ip 192.168.1.100 --port 8080
```

**Multi-Node Cluster**
```bash
# First node (bootstrap)
./o3storage --ip 192.168.1.101 --port 8080

# Additional nodes
./o3storage --ip 192.168.1.102 --port 8080 --peers 192.168.1.101
./o3storage --ip 192.168.1.103 --port 8080 --peers 192.168.1.101,192.168.1.102
```

### Basic File Operations

**Upload File**
```bash
# Using curl
curl -X PUT "http://192.168.1.101:8080/mybucket/myfile.txt" \
  -H "Content-Type: text/plain" \
  -d "Hello, O3Storage!"

# Using AWS CLI
aws s3 cp myfile.txt s3://mybucket/ --endpoint-url http://192.168.1.101:8080
```

**Download File**
```bash
# Using curl
curl "http://192.168.1.101:8080/mybucket/myfile.txt"

# Using AWS CLI
aws s3 cp s3://mybucket/myfile.txt ./ --endpoint-url http://192.168.1.101:8080
```

**List Files**
```bash
# Using curl
curl "http://192.168.1.101:8080/mybucket/"

# Using AWS CLI
aws s3 ls s3://mybucket/ --endpoint-url http://192.168.1.101:8080
```

**Delete File**
```bash
# Using curl
curl -X DELETE "http://192.168.1.101:8080/mybucket/myfile.txt"

# Using AWS CLI
aws s3 rm s3://mybucket/myfile.txt --endpoint-url http://192.168.1.101:8080
```

## Advanced Usage

### Working with Object Versions

**Upload with Metadata**
```bash
curl -X PUT "http://192.168.1.101:8080/mybucket/document.pdf" \
  -H "Content-Type: application/pdf" \
  -H "x-amz-meta-author: John Doe" \
  -H "x-amz-meta-department: Engineering" \
  --data-binary @document.pdf
```

**Get Specific Version**
```bash
# List all versions first
curl "http://192.168.1.101:8080/mybucket/?versions"

# Get specific version
curl "http://192.168.1.101:8080/mybucket/document.pdf?versionId=550e8400-e29b-41d4-a716-446655440000"
```

**Delete Specific Version**
```bash
curl -X DELETE "http://192.168.1.101:8080/mybucket/document.pdf?versionId=550e8400-e29b-41d4-a716-446655440000"
```

### Bulk Operations

**Batch Upload Script**
```bash
#!/bin/bash
BUCKET="data-backup"
SOURCE_DIR="/path/to/data"
ENDPOINT="http://192.168.1.101:8080"

find "$SOURCE_DIR" -type f | while read file; do
    key=$(echo "$file" | sed "s|$SOURCE_DIR/||")
    echo "Uploading $key..."
    curl -X PUT "$ENDPOINT/$BUCKET/$key" \
      -H "Content-Type: application/octet-stream" \
      --data-binary "@$file"
done
```

**Parallel Upload with GNU Parallel**
```bash
find /path/to/data -type f | parallel -j 8 \
  'curl -X PUT "http://192.168.1.101:8080/backup/{/}" \
   --data-binary "@{}" \
   -H "Content-Type: application/octet-stream"'
```

### Using Python SDK

**Install AWS SDK**
```bash
pip install boto3
```

**Python Example**
```python
import boto3
from botocore.config import Config

# Configure client
config = Config(
    region_name='us-east-1',
    signature_version='s3v4',
    s3={
        'addressing_style': 'path'
    }
)

s3_client = boto3.client(
    's3',
    endpoint_url='http://192.168.1.101:8080',
    aws_access_key_id='o3storage',
    aws_secret_access_key='o3storage-secret',
    config=config
)

# Create bucket
s3_client.create_bucket(Bucket='my-bucket')

# Upload file
with open('data.json', 'rb') as f:
    s3_client.put_object(
        Bucket='my-bucket',
        Key='data/file.json',
        Body=f,
        Metadata={'author': 'system', 'type': 'json'}
    )

# Download file
response = s3_client.get_object(Bucket='my-bucket', Key='data/file.json')
data = response['Body'].read()

# List objects
response = s3_client.list_objects_v2(Bucket='my-bucket', Prefix='data/')
for obj in response.get('Contents', []):
    print(f"Key: {obj['Key']}, Size: {obj['Size']}")
```

### Using Node.js SDK

**Install AWS SDK**
```bash
npm install @aws-sdk/client-s3
```

**Node.js Example**
```javascript
const { S3Client, CreateBucketCommand, PutObjectCommand, GetObjectCommand } = require('@aws-sdk/client-s3');
const fs = require('fs');

// Configure client
const client = new S3Client({
  endpoint: 'http://192.168.1.101:8080',
  region: 'us-east-1',
  credentials: {
    accessKeyId: 'o3storage',
    secretAccessKey: 'o3storage-secret',
  },
  forcePathStyle: true,
});

async function example() {
  try {
    // Create bucket
    await client.send(new CreateBucketCommand({ Bucket: 'my-bucket' }));
    
    // Upload file
    const fileContent = fs.readFileSync('data.txt');
    await client.send(new PutObjectCommand({
      Bucket: 'my-bucket',
      Key: 'data.txt',
      Body: fileContent,
      ContentType: 'text/plain',
      Metadata: {
        'author': 'node-app',
        'timestamp': Date.now().toString()
      }
    }));
    
    // Download file
    const response = await client.send(new GetObjectCommand({
      Bucket: 'my-bucket',
      Key: 'data.txt'
    }));
    
    console.log('File downloaded successfully');
  } catch (error) {
    console.error('Error:', error);
  }
}

example();
```

## Monitoring and Maintenance

### Health Monitoring

**Basic Health Check**
```bash
# Check single node
curl -s http://192.168.1.101:8080/health | jq

# Check all nodes
for ip in 192.168.1.101 192.168.1.102 192.168.1.103; do
  echo "Node $ip:"
  curl -s http://$ip:8080/health | jq '.status'
done
```

**Cluster Status Script**
```bash
#!/bin/bash
NODES=("192.168.1.101" "192.168.1.102" "192.168.1.103")

echo "O3Storage Cluster Status"
echo "======================="

for node in "${NODES[@]}"; do
    echo -n "Node $node: "
    if curl -f -s http://$node:8080/health > /dev/null; then
        health=$(curl -s http://$node:8080/health | jq -r '.status')
        active_nodes=$(curl -s http://$node:8080/health | jq -r '.cluster.active_nodes')
        echo "✓ $health (sees $active_nodes nodes)"
    else
        echo "✗ unreachable"
    fi
done
```

### Log Analysis

**View System Logs**
```bash
# Service logs
sudo journalctl -u o3storage -f

# Filter errors only
sudo journalctl -u o3storage | grep ERROR

# Show logs from last hour
sudo journalctl -u o3storage --since "1 hour ago"
```

**Log Parsing Script**
```bash
#!/bin/bash
# Parse O3Storage logs for key metrics

LOG_FILE="/var/log/o3storage/o3storage.log"

echo "O3Storage Log Analysis"
echo "===================="

echo "Error Summary:"
grep -c ERROR "$LOG_FILE" | awk '{print "  Total errors: " $1}'
grep ERROR "$LOG_FILE" | tail -5 | while read line; do
    echo "  Latest: $line"
done

echo -e "\nStorage Operations:"
grep "PUT\|GET\|DELETE" "$LOG_FILE" | tail -10

echo -e "\nCluster Events:"
grep "consensus\|replication" "$LOG_FILE" | tail -5
```

### Performance Monitoring

**Resource Usage**
```bash
# CPU and memory
top -p $(pgrep o3storage)

# Disk usage
df -h /opt/o3storage/data

# Network activity
sudo netstat -i
sudo ss -tuln | grep :8080
```

**Storage Statistics**
```bash
# Get storage stats from API
curl -s http://192.168.1.101:8080/health | jq '.storage'

# File system analysis
sudo du -sh /opt/o3storage/data/*
find /opt/o3storage/data -name "*.obj" | wc -l
```

### Backup Operations

**Manual Backup**
```bash
#!/bin/bash
BACKUP_DIR="/backup/o3storage"
DATE=$(date +%Y%m%d_%H%M%S)
NODE_IP="192.168.1.101"

echo "Creating O3Storage backup: $DATE"

# Create backup directory
mkdir -p "$BACKUP_DIR/$DATE"

# Stop service (for consistent backup)
sudo systemctl stop o3storage

# Backup data
sudo tar -czf "$BACKUP_DIR/$DATE/data.tar.gz" /opt/o3storage/data
sudo cp /etc/systemd/system/o3storage.service "$BACKUP_DIR/$DATE/"

# Restart service
sudo systemctl start o3storage

echo "Backup completed: $BACKUP_DIR/$DATE"
```

**Automated Backup Script**
```bash
#!/bin/bash
# Add to crontab: 0 2 * * * /opt/o3storage/backup.sh

RETENTION_DAYS=30
BACKUP_ROOT="/backup/o3storage"

# Create today's backup
./manual-backup.sh

# Clean old backups
find "$BACKUP_ROOT" -type d -mtime +$RETENTION_DAYS -exec rm -rf {} \;

echo "Automated backup completed"
```

## Troubleshooting

### Common Issues and Solutions

**1. Node Cannot Join Cluster**
```bash
# Check network connectivity
telnet 192.168.1.101 8080

# Check firewall
sudo ufw status
sudo iptables -L

# Verify peer configuration
curl -s http://192.168.1.101:8080/health | jq '.cluster'

# Solution: Fix network/firewall, restart node
sudo systemctl restart o3storage
```

**2. Storage Full**
```bash
# Check disk space
df -h /opt/o3storage/data

# Find largest files
sudo du -sh /opt/o3storage/data/* | sort -rh | head -10

# Clean up old versions (if needed)
# Note: This should be done carefully
sudo find /opt/o3storage/data -name "*.old" -delete
```

**3. High Memory Usage**
```bash
# Check memory usage
free -h
ps aux | grep o3storage

# Restart service to clear memory
sudo systemctl restart o3storage

# Configure memory limits in systemd
sudo systemctl edit o3storage
```
Add:
```ini
[Service]
MemoryLimit=8G
```

**4. API Timeouts**
```bash
# Check service status
sudo systemctl status o3storage

# Check resource utilization
htop

# Increase timeout in client
curl -m 60 http://192.168.1.101:8080/health

# Check for consensus issues
sudo journalctl -u o3storage | grep consensus
```

### Debug Mode

**Enable Debug Logging**
```bash
# Set environment variable
export RUST_LOG=debug

# Restart with debug
sudo RUST_LOG=debug systemctl restart o3storage

# Or run directly
sudo -u o3storage RUST_LOG=debug ./o3storage --ip 192.168.1.101 --port 8080
```

### Performance Issues

**Network Latency**
```bash
# Test network between nodes
ping -c 5 192.168.1.102
iperf3 -c 192.168.1.102

# Check cluster consensus timing
curl -s http://192.168.1.101:8080/health | jq '.cluster'
```

**Disk I/O Issues**
```bash
# Test disk performance
sudo hdparm -t /dev/nvme0n1
sudo fio --name=test --rw=write --size=1G --filename=/opt/o3storage/data/test
```

## Performance Tuning

### System-Level Optimizations

**CPU Governor**
```bash
# Set performance mode
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
```

**Network Tuning**
```bash
# Add to /etc/sysctl.conf
net.core.rmem_max = 33554432
net.core.wmem_max = 33554432
net.ipv4.tcp_rmem = 4096 87380 33554432
net.ipv4.tcp_wmem = 4096 65536 33554432
net.ipv4.tcp_congestion_control = bbr

sudo sysctl -p
```

**Storage Optimization**
```bash
# Mount with performance options
sudo mount -o noatime,nodiratime,relatime /dev/nvme0n1 /opt/o3storage/data

# Set I/O scheduler
echo mq-deadline | sudo tee /sys/block/nvme0n1/queue/scheduler
```

### Application Tuning

**Service Configuration**
```ini
# In /etc/systemd/system/o3storage.service
[Service]
Environment=RUST_LOG=info
Environment=TOKIO_WORKER_THREADS=8
LimitNOFILE=65536
```

**Memory Configuration**
```bash
# Increase system limits
echo "o3storage soft memlock unlimited" >> /etc/security/limits.conf
echo "o3storage hard memlock unlimited" >> /etc/security/limits.conf
```

This operating manual provides comprehensive guidance for managing O3StorageOS in production environments. For additional support, refer to the system logs and health endpoints for real-time diagnostics.