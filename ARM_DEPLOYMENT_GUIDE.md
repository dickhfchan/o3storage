# O3StorageOS ARM Deployment Guide

## Overview
Complete guide for deploying O3StorageOS distributed storage system on ARM devices, specifically optimized for ARM64 Cortex-A76 architecture.

## System Requirements

### Hardware Requirements
- **Minimum Configuration:**
  - ARM64 CPU (Cortex-A76 recommended)
  - 4GB RAM (8GB+ recommended for production)
  - 32GB storage (NVMe SSD recommended)
  - Gigabit Ethernet
  
- **Recommended Production Configuration:**
  - ARM64 Cortex-A76 CPU (4+ cores)
  - 16GB RAM
  - 1TB+ NVMe SSD
  - 10Gb Ethernet
  - Hardware entropy source

### Supported ARM Devices
- **Single Board Computers:**
  - Raspberry Pi 4B/5 (8GB model)
  - NVIDIA Jetson Nano/Xavier
  - Khadas VIM4
  - Orange Pi 5
  - Rock Pi 4

- **ARM Servers:**
  - AWS Graviton instances
  - Oracle Ampere servers
  - Marvell ThunderX2/3
  - Qualcomm Centriq

## Pre-Installation Setup

### 1. Prepare Development Environment

```bash
# Install Rust toolchain on x86_64 build machine
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install nightly toolchain with ARM target
rustup toolchain install nightly
rustup override set nightly
rustup target add aarch64-unknown-linux-gnu
rustup component add rust-src llvm-tools-preview

# Install cross-compilation tools
sudo apt update
sudo apt install -y gcc-aarch64-linux-gnu qemu-user-static
```

### 2. Configure Cross-Compilation

Create `.cargo/config.toml`:
```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

## Build Process

### 1. Clone and Setup Repository

```bash
git clone https://github.com/dickhfchan/o3storage.git
cd o3storage
```

### 2. Build for ARM64

```bash
# Build the main application
cargo build --release --target aarch64-unknown-linux-gnu

# Build test executables
rustc --target aarch64-unknown-linux-gnu -O hello_world_test.rs -o hello_world_test_arm
rustc --target aarch64-unknown-linux-gnu -O standalone_test.rs -o standalone_test_arm
rustc --target aarch64-unknown-linux-gnu -O three_node_test.rs -o three_node_test_arm
```

### 3. Create Deployment Package

```bash
# Create deployment directory
mkdir -p deploy/arm64
cd deploy/arm64

# Copy ARM64 binaries
cp ../../target/aarch64-unknown-linux-gnu/release/o3storage ./
cp ../../*_test_arm ./

# Copy configuration files
cp ../../scripts/* ./
cp ../../README.md ./
```

## Installation on ARM Device

### 1. Transfer Files to ARM Device

```bash
# Using SCP
scp -r deploy/arm64/* user@arm-device:/opt/o3storage/

# Or using USB/SD card
sudo cp -r deploy/arm64/* /media/usb/o3storage/
```

### 2. Set Up System Dependencies

```bash
# On ARM device
sudo apt update
sudo apt install -y build-essential pkg-config

# Create system user
sudo useradd -r -s /bin/false o3storage
sudo mkdir -p /opt/o3storage/data
sudo chown -R o3storage:o3storage /opt/o3storage
```

### 3. Configure System Limits

Add to `/etc/security/limits.conf`:
```
o3storage soft nofile 65536
o3storage hard nofile 65536
o3storage soft memlock unlimited
o3storage hard memlock unlimited
```

### 4. Create Systemd Service

Create `/etc/systemd/system/o3storage.service`:
```ini
[Unit]
Description=O3Storage Distributed Storage Node
After=network.target
Wants=network.target

[Service]
Type=simple
User=o3storage
Group=o3storage
WorkingDirectory=/opt/o3storage
ExecStart=/opt/o3storage/o3storage --ip %i --port 8080
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=o3storage

# Security settings
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ReadWritePaths=/opt/o3storage/data
CapabilityBoundingSet=CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
```

## Single Node Deployment

### 1. Basic Configuration

```bash
# Navigate to installation directory
cd /opt/o3storage

# Run interactive setup
sudo -u o3storage ./o3storage

# Or specify parameters directly
sudo -u o3storage ./o3storage --ip 192.168.1.100 --port 8080
```

### 2. Enable Service

```bash
# Enable and start service
sudo systemctl enable o3storage
sudo systemctl start o3storage

# Check status
sudo systemctl status o3storage
sudo journalctl -u o3storage -f
```

### 3. Verify Installation

```bash
# Test the service
curl http://192.168.1.100:8080/health

# Run basic test
./hello_world_test_arm
```

## Multi-Node Cluster Deployment

### 1. Plan Network Architecture

Example 3-node cluster:
- Node 1: 192.168.1.101:8080 (Leader)
- Node 2: 192.168.1.102:8080 (Follower)  
- Node 3: 192.168.1.103:8080 (Follower)

### 2. Deploy First Node (Bootstrap)

```bash
# On Node 1 (192.168.1.101)
cd /opt/o3storage
sudo -u o3storage ./o3storage --ip 192.168.1.101 --port 8080

# Enable service
sudo systemctl enable o3storage@192.168.1.101
sudo systemctl start o3storage@192.168.1.101
```

### 3. Deploy Additional Nodes

```bash
# On Node 2 (192.168.1.102)
sudo -u o3storage ./o3storage --ip 192.168.1.102 --port 8080 --peers 192.168.1.101

# On Node 3 (192.168.1.103)  
sudo -u o3storage ./o3storage --ip 192.168.1.103 --port 8080 --peers 192.168.1.101,192.168.1.102
```

### 4. Verify Cluster

```bash
# Check cluster health on any node
curl http://192.168.1.101:8080/health
curl http://192.168.1.102:8080/health 
curl http://192.168.1.103:8080/health

# Run distributed test
./three_node_test_arm
```

## Performance Optimization

### 1. ARM-Specific Optimizations

```bash
# Enable performance governor
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Configure CPU affinity
sudo systemctl edit o3storage
```

Add to override:
```ini
[Service]
ExecStart=
ExecStart=taskset -c 0-3 /opt/o3storage/o3storage --ip %i --port 8080
```

### 2. Storage Optimization

```bash
# Mount data directory with optimizations
sudo mount -o noatime,nodiratime /dev/nvme0n1 /opt/o3storage/data

# Add to /etc/fstab
/dev/nvme0n1 /opt/o3storage/data ext4 noatime,nodiratime 0 2
```

### 3. Network Optimization

Add to `/etc/sysctl.conf`:
```
# Increase network buffers
net.core.rmem_max = 33554432
net.core.wmem_max = 33554432
net.ipv4.tcp_rmem = 4096 87380 33554432
net.ipv4.tcp_wmem = 4096 65536 33554432

# Enable TCP optimization
net.ipv4.tcp_congestion_control = bbr
net.core.default_qdisc = fq
```

Apply changes:
```bash
sudo sysctl -p
```

## Security Configuration

### 1. Firewall Setup

```bash
# Configure UFW
sudo ufw allow 8080/tcp comment 'O3Storage API'
sudo ufw allow from 192.168.1.0/24 to any port 8080 comment 'O3Storage Cluster'
sudo ufw --force enable
```

### 2. TLS Configuration (Optional)

```bash
# Generate self-signed certificate
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Configure TLS in service
sudo -u o3storage ./o3storage --ip 192.168.1.101 --port 8443 --tls-cert cert.pem --tls-key key.pem
```

## Monitoring and Maintenance

### 1. Log Management

```bash
# View logs
sudo journalctl -u o3storage -f

# Configure log rotation
sudo systemctl edit o3storage
```

Add:
```ini
[Service]
StandardOutput=append:/var/log/o3storage/o3storage.log
StandardError=append:/var/log/o3storage/o3storage.log
```

### 2. Health Monitoring

Create monitoring script `/opt/o3storage/monitor.sh`:
```bash
#!/bin/bash
NODE_IP="192.168.1.101"
HEALTH_URL="http://$NODE_IP:8080/health"

if curl -f -s $HEALTH_URL > /dev/null; then
    echo "$(date): Node healthy"
else
    echo "$(date): Node unhealthy - restarting service"
    systemctl restart o3storage@$NODE_IP
fi
```

Add to crontab:
```bash
# Monitor every 5 minutes
*/5 * * * * /opt/o3storage/monitor.sh >> /var/log/o3storage/monitor.log
```

## Troubleshooting

### Common Issues

1. **Permission Denied**
   ```bash
   # Fix ownership
   sudo chown -R o3storage:o3storage /opt/o3storage
   ```

2. **Port Already in Use**
   ```bash
   # Check what's using the port
   sudo netstat -tulpn | grep :8080
   sudo systemctl stop o3storage
   ```

3. **Memory Issues**
   ```bash
   # Increase swap if needed
   sudo fallocate -l 4G /swapfile
   sudo chmod 600 /swapfile
   sudo mkswap /swapfile
   sudo swapon /swapfile
   ```

4. **Network Connectivity**
   ```bash
   # Test cluster connectivity
   telnet 192.168.1.102 8080
   
   # Check firewall
   sudo ufw status
   ```

### Debug Mode

```bash
# Run with debug logging
RUST_LOG=debug sudo -u o3storage ./o3storage --ip 192.168.1.101 --port 8080
```

## Backup and Recovery

### 1. Data Backup

```bash
# Backup data directory
sudo tar -czf o3storage-backup-$(date +%Y%m%d).tar.gz /opt/o3storage/data

# Backup configuration
sudo cp /etc/systemd/system/o3storage.service /opt/o3storage/backup/
```

### 2. Recovery Procedure

```bash
# Stop service
sudo systemctl stop o3storage

# Restore data
sudo tar -xzf o3storage-backup-20241201.tar.gz -C /

# Fix permissions
sudo chown -R o3storage:o3storage /opt/o3storage

# Start service
sudo systemctl start o3storage
```

## Updates and Upgrades

### 1. Application Updates

```bash
# Stop service
sudo systemctl stop o3storage

# Backup current version
sudo cp /opt/o3storage/o3storage /opt/o3storage/o3storage.backup

# Deploy new version
sudo cp new-o3storage-arm64 /opt/o3storage/o3storage
sudo chown o3storage:o3storage /opt/o3storage/o3storage
sudo chmod +x /opt/o3storage/o3storage

# Start service
sudo systemctl start o3storage
```

### 2. Rolling Updates for Clusters

```bash
# Update one node at a time
# 1. Stop node 3
sudo systemctl stop o3storage@192.168.1.103

# 2. Update binary and restart
sudo systemctl start o3storage@192.168.1.103

# 3. Wait for cluster sync, then update node 2
# 4. Finally update node 1 (leader last)
```

This deployment guide provides comprehensive instructions for successfully deploying O3StorageOS on ARM devices in both single-node and cluster configurations.