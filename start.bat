@echo off
setlocal enabledelayedexpansion

REM Try to add Windows firewall rule for LAN access (may prompt UAC)
echo Adding firewall rule for port 8080...
powershell -NoProfile -Command "Start-Process netsh -Verb RunAs -ArgumentList 'advfirewall firewall add rule name=RabukaGameServer dir=in protocol=tcp localport=8080 action=allow' -WindowStyle Hidden" >nul 2>&1

echo Starting Rust Backend on http://127.0.0.1:8080...
echo [i18n] missing Japanese choice prompts are logged at WARN level -- watch the server output.
cd /d "%~dp0engine"

REM Enable structured verdict items in the in-game rule log
set RABUKA_RULE_LOG=1

REM Surface i18n gaps (missing Japanese choice prompts) loudly in the server log.
REM The engine logs these at WARN on boot (i18n_self_check) and at runtime.
set RUST_LOG=warn

REM If --ngrok specified, save its auth token argument
if "%1"=="--ngrok" set NGROK_AUTHTOKEN=%2

start /b C:/rust_targets/release/rabuka_engine.exe web-server

echo Waiting for Rust backend to be ready...
:wait_loop
powershell -Command "try { (Invoke-WebRequest -Uri http://127.0.0.1:8080/api/game-state -UseBasicParsing -TimeoutSec 1).StatusCode } catch { exit 1 }" >nul 2>&1
if %errorlevel% neq 0 (
    powershell -NoProfile -Command "Start-Sleep -Seconds 2" >nul 2>&1
    goto wait_loop
)
echo Rust backend is ready!
echo Game UI: http://127.0.0.1:8080
echo.
echo Share the cloudflared URL (printed above) to play over the internet.
pause
