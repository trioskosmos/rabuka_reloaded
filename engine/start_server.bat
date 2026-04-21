@echo off
echo Starting Rabuka Engine Game Server...
cd /d "%~dp0"

echo Killing any existing server process...
taskkill /F /IM rabuka_engine.exe 2>nul

echo Starting Rust server with static file serving...
cargo run --bin rabuka_engine -- web-server

echo Server stopped.
pause
