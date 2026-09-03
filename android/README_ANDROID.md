# Rabuka Reloaded - Android Guide

Run the Rabuka Reloaded multiplayer card game server on your Android phone for **free** - no web hosting required!

## How It Works

1. **Termux** - Linux environment on Android (no root needed)
2. **Native Rust compilation** - Build the server directly on your phone
3. **cloudflared tunnel** - Free, secure internet tunneling (built into the startup script)
4. **Share the URL** - Friends join via browser, no app install needed

## Requirements

- Android 7.0+ (API 24+)
- ~2 GB free storage (for Rust toolchain + build)
- 4 GB+ RAM recommended (8 GB for faster builds)
- Internet connection (for initial setup and cloudflared)

## Quick Start

### Option 1: Automated Setup (Recommended)

1. **Install Termux** from F-Droid (recommended) or GitHub releases
   - F-Droid: https://f-droid.org/packages/com.termux/
   - Don't use Google Play version (outdated)

2. **Open Termux** and run:
   ```bash
   curl -fsSL https://raw.githubusercontent.com/yourusername/rabuka_reloaded/main/android/setup_termux.sh | bash
   ```

3. **Wait for build** (10-30 minutes depending on device)

4. **Start the server**:
   ```bash
   cd ~/rabuka_reloaded/engine
   ./start_android.sh
   ```

5. **Share the cloudflared URL** printed at startup with friends!

### Option 2: Manual Setup

```bash
# In Termux:
pkg update && pkg upgrade -y
pkg install -y rust git clang make cmake pkg-config openssl-tool libandroid-support termux-api cloudflared qrencode

git clone https://github.com/yourusername/rabuka_reloaded.git ~/rabuka_reloaded
cd ~/rabuka_reloaded/engine
cargo build --release --features server

# Start server:
./start_android.sh
```

## Playing Multiplayer

### You (Host)
1. Run `./start_android.sh`
2. Share the **cloudflared URL** (e.g., `https://abc123.trycloudflare.com`) with friends
3. Or share LAN IP for local WiFi play

### Friends (Players)
1. Open the URL in any browser (Chrome, Firefox, Safari)
2. Enter a username
3. Create or join a room
4. Play!

## Features

- **PvP Multiplayer** - Real-time via Server-Sent Events (SSE)
- **AI Practice** - Play against built-in ISMCTS bot
- **Deck Builder** - Full deck editor with validation
- **Card Browser** - Search all 2,500+ cards
- **Tutorial** - Interactive rules guide
- **QR Codes** - Scan to join rooms instantly

## Troubleshooting

### Build fails with "out of memory"
```bash
# Add swap (requires root or Termux 0.118+)
pkg install tsu
tsu
swapon /data/swapfile 2>/dev/null || (fallocate -l 2G /data/swapfile && mkswap /data/swapfile && swapon /data/swapfile)
```

### cloudflared tunnel not showing
- Check `cloudflared.log` in the engine directory
- Ensure internet connection works
- Try restarting the script

### Friends can't connect
- Verify they're using the **https://** cloudflared URL (not localhost)
- Check firewall isn't blocking (Termux doesn't have firewall by default)
- Try LAN IP if on same WiFi

### Server crashes
```bash
# Run with more logging:
RUST_LOG=debug ./start_android.sh
```

## Performance Tips

- **Close other apps** while building/running
- **Plug in charger** - compilation uses CPU heavily
- **Use release build** (default) - much faster than debug
- **Keep screen on** - Termux may pause when screen off (use `termux-wake-lock`)

## File Locations

| File | Location |
|------|----------|
| Server binary | `~/rabuka_reloaded/engine/target/release/rabuka_engine` |
| Game data | `~/rabuka_reloaded/cards/`, `~/rabuka_reloaded/web_ui/` |
| Logs | `~/rabuka_reloaded/engine/cloudflared.log` |
| Recordings | `~/rabuka_reloaded/engine/recording_<ROOM_ID>.bin` |

## Updating

```bash
cd ~/rabuka_reloaded
git pull
cd engine
cargo build --release --features server
```

## Uninstall

```bash
rm -rf ~/rabuka_reloaded
pkg uninstall rust clang make cmake pkg-config openssl-tool cloudflared qrencode
```

## Why This Works for Free

- **No VPS needed** - Your phone IS the server
- **cloudflared** - Cloudflare's free tunneling service (no account needed for trycloudflare.com)
- **SSE** - Lightweight real-time updates, works on any browser
- **Termux** - Full Linux userspace, compiles native ARM64 code

## Technical Details

- **Architecture**: aarch64-linux-android (native ARM64)
- **Server**: Actix-web 4 (Rust async web framework)
- **Tunnel**: cloudflared (Cloudflare Tunnel client)
- **Protocol**: HTTP + Server-Sent Events for real-time
- **Data**: Local JSON card database (2,526 cards, 936 abilities)

## License

Same as main project - see LICENSE in repository root.

---

**Enjoy playing Rabuka Reloaded with friends anywhere!** 🎮