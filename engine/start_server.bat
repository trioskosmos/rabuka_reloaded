@echo off
echo Starting Rabuka Engine Game Server...
cd /d "%~dp0"

echo Starting Rust server with static file serving...
cargo run -- web-server

echo Server stopped.
pause
