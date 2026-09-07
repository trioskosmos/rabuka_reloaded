@echo off
setlocal enabledelayedexpansion

REM Try to add Windows firewall rule for LAN access (may prompt UAC)
echo Adding firewall rule for port 8080...
powershell -NoProfile -Command "Start-Process netsh -Verb RunAs -ArgumentList 'advfirewall firewall add rule name=RabukaGameServer dir=in protocol=tcp localport=8080 action=allow' -WindowStyle Hidden" >nul 2>&1

echo Starting Rust Backend on http://127.0.0.1:8080...
echo [i18n] missing Japanese choice prompts are logged at WARN level -- watch the server output.

REM Enable structured verdict items in the in-game rule log
set RABUKA_RULE_LOG=1

REM Surface i18n gaps (missing Japanese choice prompts) loudly in the server log.
REM The engine logs these at WARN on boot (i18n_self_check) and at runtime.
set RUST_LOG=warn

REM If --ngrok specified, save its auth token argument
if "%1"=="--ngrok" set NGROK_AUTHTOKEN=%2

REM Use workspace target directory (C:\rust_targets) for faster builds
set CARGO_TARGET_DIR=C:\rust_targets

cd /d "%~dp0engine"

REM Build if binary doesn't exist or is older than source
if not exist "C:\rust_targets\release\rabuka_engine.exe" (
    echo Building server binary...
    cargo build --release --features server
) else (
    REM Check if any .rs file is newer than binary
    for /r %%f in (*.rs) do (
        if "%%f" NEQ "" (
            for %%b in ("C:\rust_targets\release\rabuka_engine.exe") do (
                if "%%~tf" GTR "%%~tb" (
                    echo Source changed, rebuilding...
                    cargo build --release --features server
                    goto :run_server
                )
            )
        )
    )
)

:run_server
echo Starting server from engine directory...
echo Press Ctrl+C to stop the server.
echo.

REM Run server in foreground (most reliable)
"C:\rust_targets\release\rabuka_engine.exe" web-server