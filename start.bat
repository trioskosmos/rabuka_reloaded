@echo off
echo Building Rabuka Web UI...
cd /d "%~dp0web_ui"
call npm run build
if %errorlevel% neq 0 (
    echo Frontend build failed!
    pause
    exit /b 1
)

echo Starting Rust Backend on http://127.0.0.1:8080...
cd /d "%~dp0engine"
start /b cargo run --release --bin rabuka_engine web-server

echo Waiting for Rust backend to be ready...
:wait_loop
powershell -Command "try { (Invoke-WebRequest -Uri http://127.0.0.1:8080/api/game-state -UseBasicParsing -TimeoutSec 1).StatusCode } catch { exit 1 }" >nul 2>&1
if %errorlevel% neq 0 (
    timeout /t 2 /nobreak >nul
    goto wait_loop
)
echo Rust backend is ready!
echo Game UI: http://127.0.0.1:8080
pause
