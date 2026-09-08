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

REM Always run an incremental build: cargo rechecks freshness itself in
REM seconds when nothing changed. (The old timestamp-string GTR comparison
REM was locale-dependent and silently ran stale binaries.)
echo Checking server binary...
cargo build --release --features server
if errorlevel 1 (
    echo Build failed, not starting server.
    exit /b 1
)

REM Rebuild the WASM bundle if the toolchain is present. The backend serves
REM web_ui/public/wasm at /wasm (plus the / catch-all at /public/wasm), and
REM the Web Worker + test.html load through the wasm-bindgen glue there.
REM Best-effort: a missing wasm target / wasm-bindgen must never block the
REM server itself.
call :build_wasm

:run_server
echo Starting server from engine directory...
echo Press Ctrl+C to stop the server.
echo.

REM Run server in foreground (most reliable). Forward all CLI args so flags
REM like --ngrok actually reach the binary (it reads NGROK_AUTHTOKEN + argv).
"C:\rust_targets\release\rabuka_engine.exe" web-server %*
exit /b %errorlevel%

:build_wasm
rustup target list --installed 2>nul | findstr /c:"wasm32-unknown-unknown" >nul 2>&1
if errorlevel 1 (
    echo [wasm] wasm32-unknown-unknown target not installed, skipping WASM rebuild.
    echo [wasm] Install it with: rustup target add wasm32-unknown-unknown
    exit /b 0
)
where wasm-bindgen >nul 2>&1
if errorlevel 1 (
    echo [wasm] wasm-bindgen CLI not found, skipping WASM rebuild.
    echo [wasm] Install it with: cargo install wasm-bindgen-cli --version 0.2.128 --locked
    echo [wasm] (version must match platforms/wasm/Cargo.lock)
    exit /b 0
)
echo [wasm] Rebuilding WASM bundle...
pushd "%~dp0platforms\wasm"
cargo build --release --target wasm32-unknown-unknown
if errorlevel 1 (
    echo [wasm] WASM cargo build failed, keeping checked-in bundle.
    popd
    exit /b 0
)
wasm-bindgen --target web --out-dir "%~dp0web_ui\public\wasm" "%~dp0platforms\wasm\target\wasm32-unknown-unknown\release\rabuka_wasm.wasm"
if errorlevel 1 (
    echo [wasm] wasm-bindgen failed, keeping checked-in bundle.
    popd
    exit /b 0
)
popd
echo [wasm] WASM bundle refreshed in web_ui\public\wasm.
exit /b 0
