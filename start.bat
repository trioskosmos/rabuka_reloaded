@echo off
echo Killing existing processes...
taskkill /F /IM node.exe >nul 2>&1
taskkill /F /IM rabuka_engine.exe >nul 2>&1

echo Starting Rust Backend...
cd engine
start /b cargo run --bin rabuka_engine web-server
cd ..

echo Waiting for Rust backend to be ready...
:wait_loop
powershell -Command "try { (Invoke-WebRequest -Uri http://127.0.0.1:8080/api/game-state -UseBasicParsing -TimeoutSec 1).StatusCode } catch { exit 1 }" >nul 2>&1
if %errorlevel% neq 0 (
    echo Backend not ready yet, waiting...
    timeout /t 2 /nobreak
    goto wait_loop
)
echo Rust backend is ready!

echo Building Rabuka Web UI...
cd web_ui
call npm run build

echo Copying i18n files...
xcopy "js\i18n" "dist\js\i18n\" /E /I /Y >nul

echo Starting Rabuka Web Server...
node server.js
pause
