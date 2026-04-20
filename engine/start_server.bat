@echo off
echo Starting Rabuka Engine Game Server...
cd /d "%~dp0"

echo Starting Rust server with static file serving...
start "Rust Server" cmd /k cargo run -- web-server

echo Waiting for server to start...
timeout /t 5 /nobreak >nul

echo Opening game in browser...
start http://127.0.0.1:8080

echo Server started and browser opened.
pause
