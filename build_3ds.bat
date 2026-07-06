@echo off
setlocal enabledelayedexpansion

echo === Rabuka 3DS Build Script ===
echo.

:: Step 1 - Ensure devkitPro is installed
if not exist "C:\devkitPro\devkitARM\bin\arm-none-eabi-gcc.exe" (
    echo [FAIL] devkitPro not found at C:\devkitPro
    echo Install devkitPro with devkitARM + libctru from:
    echo   https://github.com/devkitPro/installer/releases
    pause
    exit /b 1
)
echo [1/5] devkitPro found

:: Step 2 - Set environment
set DEVKITPRO=C:\devkitPro
set DEVKITARM=%DEVKITPRO%\devkitARM
set "PATH=%DEVKITARM%\bin;%DEVKITPRO%\tools\bin;%PATH%"

:: Step 3 - Ensure nightly Rust + rust-src + cargo-3ds
echo [2/5] Checking Rust nightly...
rustup toolchain list 2>nul | findstr nightly >nul
if %errorlevel% neq 0 ( rustup toolchain install nightly )
rustup component add rust-src --toolchain nightly-x86_64-pc-windows-msvc 2>nul
echo [2/5] Rust nightly + rust-src ready

echo [3/5] Checking cargo-3ds...
cargo 3ds --version >nul 2>&1
if %errorlevel% neq 0 ( cargo install cargo-3ds )
echo [3/5] cargo-3ds ready

:: Step 4 - Bundle assets into romfs directory
echo [4/5] Bundling assets into romfs...
if not exist "%~dp0engine_3ds\romfs" mkdir "%~dp0engine_3ds\romfs"
if not exist "%~dp0engine_3ds\romfs\decks" mkdir "%~dp0engine_3ds\romfs\decks"
copy /Y "%~dp0cards\cards.json" "%~dp0engine_3ds\romfs\cards.json" >nul
copy /Y "%~dp0web_ui\decks\*.txt" "%~dp0engine_3ds\romfs\decks\" >nul

:: Pre-bake abilities_map.json for 3DS (avoids serde_json::Value parsing at runtime)
echo [4/5] Pre-baking abilities map...
cd /d "%~dp0"
cargo run --bin gen_abilities_map --manifest-path engine/Cargo.toml --release
if %errorlevel% neq 0 (
    echo   [WARN] gen_abilities_map failed - abilities will not be loaded on 3DS
)
echo [4/5] Assets bundled

:: Step 5 - Clean previous build artifacts to avoid stale cache issues
echo [5/5] Building 3DS binary (first build takes ~10 min)...
if exist "%~dp0engine_3ds\target" rmdir /s /q "%~dp0engine_3ds\target"
cd /d "%~dp0engine_3ds"
cargo +nightly 3ds build --bin rabuka_3ds --release --features 3ds
if %errorlevel% neq 0 (
    echo Build FAILED.
    pause
    exit /b 1
)
echo [5/5] Build succeeded

:: Copy output
if not exist "%~dp0output_3ds" mkdir "%~dp0output_3ds"
copy /Y "C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.3dsx" "%~dp0output_3ds\rabuka_3ds.3dsx" >nul

echo.
echo === Build Complete ===
echo File: output_3ds\rabuka_3ds.3dsx (cards + decks bundled inside)
echo.
echo Just load this .3dsx in Azahar - no extra files needed.
pause
