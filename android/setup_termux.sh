#!/data/data/com.termux/files/usr/bin/bash
# Rabuka Reloaded - Termux Setup Script
# Run this in Termux on your Android device to install dependencies and build the server

set -e

echo "=========================================="
echo "  Rabuka Reloaded - Termux Setup"
echo "=========================================="
echo ""

# Update packages
echo "[1/6] Updating Termux packages..."
pkg update -y && pkg upgrade -y

# Install dependencies
echo "[2/6] Installing build dependencies..."
pkg install -y \
    rust \
    git \
    clang \
    make \
    cmake \
    pkg-config \
    openssl-tool \
    libandroid-support \
    termux-api \
    cloudflared

# Verify installations
echo "[3/6] Verifying installations..."
rustc --version
cargo --version
cloudflared --version

# Clone or update the repository
REPO_DIR="$HOME/rabuka_reloaded"
if [ -d "$REPO_DIR" ]; then
    echo "[4/6] Repository exists, pulling latest changes..."
    cd "$REPO_DIR"
    git pull
else
    echo "[4/6] Cloning repository..."
    git clone https://github.com/yourusername/rabuka_reloaded.git "$REPO_DIR"
    cd "$REPO_DIR"
fi

# Build the engine with server feature
echo "[5/6] Building Rabuka Engine (this may take 10-30 minutes)..."
cd "$REPO_DIR/engine"
cargo build --release --features server

# Verify build
echo "[6/6] Verifying build..."
if [ -f "target/release/rabuka_engine" ]; then
    echo ""
    echo "=========================================="
    echo "  Build Successful!"
    echo "=========================================="
    echo ""
    echo "Binary location: $REPO_DIR/engine/target/release/rabuka_engine"
    echo ""
    echo "To start the server, run:"
    echo "  cd $REPO_DIR/engine"
    echo "  ./start_android.sh"
    echo ""
else
    echo "ERROR: Build failed - binary not found"
    exit 1
fi