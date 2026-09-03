#!/data/data/com.termux/files/usr/bin/bash
# Rabuka Reloaded - Android Startup Script
# Runs the web server with cloudflared tunnel for free internet access

set -e

REPO_DIR="$HOME/rabuka_reloaded"
ENGINE_DIR="$REPO_DIR/engine"
BINARY="$ENGINE_DIR/target/release/rabuka_engine"
PORT=8080

echo "=========================================="
echo "  Rabuka Reloaded - Android Server"
echo "=========================================="
echo ""

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "ERROR: Server binary not found at $BINARY"
    echo "Run setup_termux.sh first to build the project"
    exit 1
fi

# Check if web_ui exists
if [ ! -d "$REPO_DIR/web_ui" ]; then
    echo "ERROR: web_ui directory not found at $REPO_DIR/web_ui"
    exit 1
fi

# Check if cards exist
if [ ! -d "$REPO_DIR/cards" ]; then
    echo "ERROR: cards directory not found at $REPO_DIR/cards"
    exit 1
fi

# Kill any existing cloudflared processes
pkill -f "cloudflared tunnel" 2>/dev/null || true

# Get local IP for LAN access
LOCAL_IP=$(ip route get 1.1.1.1 2>/dev/null | awk '{print $7}' | head -1)
if [ -z "$LOCAL_IP" ]; then
    LOCAL_IP="127.0.0.1"
fi

echo "Starting Rabuka Engine on port $PORT..."
echo "Local access:  http://127.0.0.1:$PORT"
echo "LAN access:    http://$LOCAL_IP:$PORT"
echo ""

# Start cloudflared in background for internet tunneling
echo "Starting cloudflared tunnel..."
cloudflared tunnel --url "http://localhost:$PORT" --no-autoupdate > cloudflared.log 2>&1 &
CLOUDFLARED_PID=$!

# Wait for cloudflared to start and get the URL
echo "Waiting for cloudflared tunnel URL..."
TUNNEL_URL=""
for i in {1..30}; do
    if grep -q "trycloudflare.com" cloudflared.log 2>/dev/null; then
        TUNNEL_URL=$(grep -o 'https://[^[:space:]]*trycloudflare.com' cloudflared.log | head -1)
        break
    fi
    sleep 1
done

if [ -n "$TUNNEL_URL" ]; then
    echo ""
    echo "=========================================="
    echo "  Internet Access Ready!"
    echo "=========================================="
    echo "Share this URL with your friends:"
    echo "  $TUNNEL_URL"
    echo ""
    echo "QR Code for easy sharing:"
    echo "$TUNNEL_URL" | qrencode -t utf8 2>/dev/null || echo "(install qrencode for QR codes: pkg install qrencode)"
    echo ""
else
    echo ""
    echo "WARNING: Cloudflared tunnel URL not detected yet."
    echo "Check cloudflared.log for details."
    echo "You can still play via LAN at: http://$LOCAL_IP:$PORT"
    echo ""
fi

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "Shutting down..."
    kill $CLOUDFLARED_PID 2>/dev/null || true
    kill $SERVER_PID 2>/dev/null || true
    exit 0
}
trap cleanup INT TERM

# Start the Rust server
cd "$ENGINE_DIR"
RUST_LOG=warn RABUKA_RULE_LOG=1 "$BINARY" web-server &
SERVER_PID=$!

echo "Server started (PID: $SERVER_PID)"
echo "Cloudflared PID: $CLOUDFLARED_PID"
echo ""
echo "Press Ctrl+C to stop the server"
echo ""

# Wait for server process
wait $SERVER_PID