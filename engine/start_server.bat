@echo off
echo Starting Rabuka Engine Web Server...
cd /d "%~dp0"
cargo run -- web-server
pause
