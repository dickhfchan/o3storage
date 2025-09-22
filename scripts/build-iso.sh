#!/bin/bash
# O3Storage ISO Builder
# Builds a minimal ISO image for ARM64 devices with O3Storage pre-installed

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build"
ISO_DIR="$BUILD_DIR/iso"

echo "Building O3Storage ISO for ARM64..."

# Create build directories
mkdir -p "$BUILD_DIR"
mkdir -p "$ISO_DIR"

# Build the O3Storage binary
echo "Building O3Storage binary..."
cd "$PROJECT_ROOT"
cargo build --release --target aarch64-unknown-linux-gnu

# Create minimal filesystem structure
echo "Creating minimal filesystem..."
mkdir -p "$ISO_DIR"/{boot,usr/bin,etc,var/lib,lib,sbin,bin}

# Copy the O3Storage binary
cp "$PROJECT_ROOT/target/aarch64-unknown-linux-gnu/release/o3storage" "$ISO_DIR/usr/bin/"

# Create init script
cat > "$ISO_DIR/sbin/init" << 'EOF'
#!/bin/bash
# O3Storage System Init

# Mount essential filesystems
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev

# Set hostname
echo "o3storage" > /proc/sys/kernel/hostname

# Create storage directory
mkdir -p /var/lib/o3storage

# Start O3Storage daemon
echo "Starting O3Storage..."
/usr/bin/o3storage

# Keep system running
while true; do
    sleep 60
done
EOF

chmod +x "$ISO_DIR/sbin/init"

# Create system configuration
cat > "$ISO_DIR/etc/fstab" << 'EOF'
proc /proc proc defaults 0 0
sysfs /sys sysfs defaults 0 0
devtmpfs /dev devtmpfs defaults 0 0
tmpfs /tmp tmpfs defaults 0 0
EOF

# Create network configuration template
cat > "$ISO_DIR/etc/network.conf" << 'EOF'
# O3Storage Network Configuration
# Edit this file to configure network settings

# Primary network interface
INTERFACE=eth0

# IP Configuration (static or dhcp)
IP_CONFIG=dhcp

# Static IP settings (used when IP_CONFIG=static)
STATIC_IP=192.168.1.100
NETMASK=255.255.255.0
GATEWAY=192.168.1.1
DNS=8.8.8.8

# O3Storage cluster peers (comma-separated IP addresses)
CLUSTER_PEERS=
EOF

# Create service management script
cat > "$ISO_DIR/usr/bin/o3-service" << 'EOF'
#!/bin/bash
# O3Storage Service Management

case "$1" in
    start)
        echo "Starting O3Storage..."
        /usr/bin/o3storage --config /etc/o3storage.conf &
        echo $! > /var/run/o3storage.pid
        ;;
    stop)
        echo "Stopping O3Storage..."
        if [ -f /var/run/o3storage.pid ]; then
            kill $(cat /var/run/o3storage.pid)
            rm /var/run/o3storage.pid
        fi
        ;;
    restart)
        $0 stop
        sleep 2
        $0 start
        ;;
    status)
        if [ -f /var/run/o3storage.pid ] && kill -0 $(cat /var/run/o3storage.pid) 2>/dev/null; then
            echo "O3Storage is running (PID: $(cat /var/run/o3storage.pid))"
        else
            echo "O3Storage is not running"
        fi
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status}"
        exit 1
        ;;
esac
EOF

chmod +x "$ISO_DIR/usr/bin/o3-service"

# Create setup wizard
cat > "$ISO_DIR/usr/bin/o3-setup" << 'EOF'
#!/bin/bash
# O3Storage Initial Setup Wizard

clear
echo "=================================="
echo "   O3Storage Setup Wizard"
echo "=================================="
echo

# Check hardware requirements
echo "Checking hardware requirements..."

# Check architecture
if [ "$(uname -m)" != "aarch64" ]; then
    echo "ERROR: This system requires ARM64 (aarch64) architecture"
    exit 1
fi

# Check memory
MEM_KB=$(grep MemTotal /proc/meminfo | awk '{print $2}')
MEM_GB=$((MEM_KB / 1024 / 1024))
if [ $MEM_GB -lt 8 ]; then
    echo "WARNING: Minimum 8GB RAM recommended, found ${MEM_GB}GB"
fi

echo "Hardware check passed."
echo

# Network configuration
echo "Network Configuration:"
echo "======================"

# Get available interfaces
INTERFACES=$(ls /sys/class/net/ | grep -v lo)
echo "Available network interfaces: $INTERFACES"

read -p "Enter network interface to use (default: eth0): " INTERFACE
INTERFACE=${INTERFACE:-eth0}

read -p "Use DHCP? (y/n, default: y): " USE_DHCP
USE_DHCP=${USE_DHCP:-y}

if [[ "$USE_DHCP" == "n" || "$USE_DHCP" == "N" ]]; then
    read -p "Enter static IP address: " STATIC_IP
    read -p "Enter netmask (default: 255.255.255.0): " NETMASK
    NETMASK=${NETMASK:-255.255.255.0}
    read -p "Enter gateway: " GATEWAY
    read -p "Enter DNS server (default: 8.8.8.8): " DNS
    DNS=${DNS:-8.8.8.8}
fi

# O3Storage configuration
echo
echo "O3Storage Configuration:"
echo "========================"

read -p "Enter storage path (default: /var/lib/o3storage): " STORAGE_PATH
STORAGE_PATH=${STORAGE_PATH:-/var/lib/o3storage}

read -p "Enter maximum storage size in TB (default: 50): " MAX_STORAGE_TB
MAX_STORAGE_TB=${MAX_STORAGE_TB:-50}

read -p "Enter cluster peer IP addresses (comma-separated, optional): " CLUSTER_PEERS

read -p "Enter API port (default: 8080): " API_PORT
API_PORT=${API_PORT:-8080}

# Create configuration file
cat > /etc/o3storage.conf << EOL
# O3Storage Configuration
storage_path = "$STORAGE_PATH"
max_storage_size = "$((MAX_STORAGE_TB * 1024 * 1024 * 1024 * 1024))"
api_port = $API_PORT
cluster_peers = "$CLUSTER_PEERS"
EOL

# Configure network
if [[ "$USE_DHCP" == "y" || "$USE_DHCP" == "Y" ]]; then
    dhclient $INTERFACE
else
    ip addr add $STATIC_IP/$NETMASK dev $INTERFACE
    ip route add default via $GATEWAY
    echo "nameserver $DNS" > /etc/resolv.conf
fi

# Create storage directory
mkdir -p "$STORAGE_PATH"

echo
echo "Setup complete!"
echo "==============="
echo "Configuration saved to /etc/o3storage.conf"
echo "Storage directory: $STORAGE_PATH"
echo "API will be available on port: $API_PORT"
echo
echo "To start O3Storage: o3-service start"
echo "To check status: o3-service status"
echo
EOF

chmod +x "$ISO_DIR/usr/bin/o3-setup"

# Create README
cat > "$ISO_DIR/README.txt" << 'EOF'
O3Storage - Distributed Immutable Object Storage
================================================

This is a minimal O3Storage system image for ARM64 devices.

Initial Setup:
1. Boot from this image
2. Run: o3-setup
3. Follow the setup wizard
4. Start the service: o3-service start

Key Features:
- Immutable object storage with versioning
- S3-compatible API
- Distributed replication (minimum 3 nodes)
- Automatic consensus and cluster management
- Zero-downtime operation with degraded read capability
- Hardware-optimized for ARM64 Cortex-A76

Directory Structure:
- /usr/bin/o3storage     - Main O3Storage binary
- /usr/bin/o3-setup      - Setup wizard
- /usr/bin/o3-service    - Service management
- /etc/o3storage.conf    - Configuration file
- /var/lib/o3storage     - Default storage directory

For more information, visit: https://github.com/your-org/o3storage
EOF

echo "ISO filesystem created at: $ISO_DIR"
echo "To create bootable ISO, use additional tools like genisoimage or xorriso"
echo "Example: genisoimage -r -J -o o3storage.iso $ISO_DIR"

echo "Build complete!"
echo "ISO directory: $ISO_DIR"