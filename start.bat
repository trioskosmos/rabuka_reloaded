@echo off
setlocal enabledelayedexpansion

REM Add the Windows firewall rule for LAN access once; skip the UAC prompt
REM when the rule already exists (previously prompted on EVERY launch).
netsh advfirewall firewall show rule name=RabukaGameServer >nul 2>&1
if errorlevel 1 (
    echo Adding firewall rule for port 8080...
    powershell -NoProfile -Command "Start-Process netsh -Verb RunAs -ArgumentList 'advfirewall firewall add rule name=RabukaGameServer dir=in protocol=tcp localport=8080 action=allow' -WindowStyle Hidden" >nul 2>&1
)

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
REM --bin rabuka_engine: a bare `cargo build` builds ALL ~17 [[bin]] targets
REM (bot_arena, sim_bench, play_game, calibrate, ...) -- minutes of codegen
REM and linking for binaries the game never runs.
echo Checking server binary...
cargo build --release --features server --bin rabuka_engine
if errorlevel 1 (
    echo Build failed, not starting server.
    pause
    exit /b 1
)

REM The WASM browser bundle is NOT rebuilt on every launch: platforms/wasm
REM builds the whole engine with lto=true + codegen-units=1, so ANY engine
REM edit used to cost minutes here even though the desktop game never loads
REM the bundle. Pass --wasm to refresh web_ui\public\wasm when the browser
REM client (Web Worker + test.html) needs it.
set BUILD_WASM=0
set FORWARD_ARGS=
:argloop
if "%~1"=="" goto argdone
if /i "%~1"=="--wasm" (
    set BUILD_WASM=1
) else (
    set FORWARD_ARGS=!FORWARD_ARGS! %1
)
shift
goto argloop
:argdone

if "%BUILD_WASM%"=="1" call :build_wasm

:run_server
echo Starting server from engine directory...
echo Press Ctrl+C to stop the server.
echo.

REM Open the game UI in the default browser once the server is up. Detached
REM and delayed so it neither blocks the foreground server nor dies with it.
start "" /b powershell -NoProfile -WindowStyle Hidden -Command "Start-Sleep -Seconds 3; Start-Process 'http://127.0.0.1:8080'"

REM Run server in foreground (most reliable). Forward all CLI args so flags
REM like --ngrok actually reach the binary (it reads NGROK_AUTHTOKEN + argv).
"C:\rust_targets\release\rabuka_engine.exe" web-server %FORWARD_ARGS%
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
wasm-bindgen --target web --out-dir "%~dp0web_ui\public\wasm" "%CARGO_TARGET_DIR%\wasm32-unknown-unknown\release\rabuka_wasm.wasm"
if errorlevel 1 (
    echo [wasm] wasm-bindgen failed, keeping checked-in bundle.
    popd
    exit /b 0
)
popd
echo [wasm] WASM bundle refreshed in web_ui\public\wasm.
exit /b 0
