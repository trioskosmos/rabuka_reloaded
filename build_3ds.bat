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
echo [1/6] devkitPro found

:: Step 2 - Set environment
set DEVKITPRO=C:\devkitPro
set DEVKITARM=%DEVKITPRO%\devkitARM
set "PATH=%DEVKITARM%\bin;%DEVKITPRO%\tools\bin;%PATH%"

:: Step 3 - Ensure nightly Rust + rust-src + cargo-3ds
echo [2/6] Checking Rust nightly...
rustup toolchain list 2>nul | findstr nightly >nul
if %errorlevel% neq 0 ( rustup toolchain install nightly )
rustup component add rust-src --toolchain nightly-x86_64-pc-windows-msvc 2>nul
echo [2/6] Rust nightly + rust-src ready

echo [3/6] Checking cargo-3ds...
cargo 3ds --version >nul 2>&1
if %errorlevel% neq 0 ( cargo install cargo-3ds )
echo [3/6] cargo-3ds ready

:: Step 4 - Pre-bake abilities into cards_baked.json (runs on fast desktop CPU)
echo [4/6] Pre-baking abilities...
if not exist "%~dp0engine_3ds\romfs" mkdir "%~dp0engine_3ds\romfs"
if not exist "%~dp0engine_3ds\romfs\decks" mkdir "%~dp0engine_3ds\romfs\decks"
cd /d "%~dp0engine_3ds"
set RUSTFLAGS=-C link-arg=/STACK:8388608
cargo run --bin bake --release -- "%~dp0engine_3ds/romfs"
if %errorlevel% neq 0 (
    echo [FAIL] bake failed.
    pause
    exit /b 1
)
echo [4/6] cards.json ready

:: Copy deck files
copy /Y "%~dp0web_ui\decks\*.txt" "%~dp0engine_3ds\romfs\decks\" >nul

echo [4/6] cards.json with baked abilities ready

:: Step 4.5 - Card image conversion (skip if t3x atlases already exist)
dir /b "%~dp0engine_3ds\romfs\cards\*.t3x" >nul 2>nul
if errorlevel 1 (
    echo [4.5/6] Converting card images...
    if exist "%~dp0web_ui\img\cards_webp\*.webp" (
        cd /d "%~dp0engine_3ds"
        where python3 >nul 2>&1
        if !errorlevel! equ 0 (
            python scripts/convert_cards.py
        ) else (
            echo [WARN] python3 not found - skipping card image conversion
            echo        Cards will appear as text-only
        )
        cd /d "%~dp0engine_3ds"
    ) else (
        echo [WARN] No card images found at web_ui\img\cards_webp
    )
) else (
    echo [4.5/6] Card images already converted (t3x atlases found)
)

:: Step 5 - Build 3DS binary
echo [5/6] Building 3DS binary (first build takes ~10 min)...
if exist "%~dp0engine_3ds\target" rmdir /s /q "%~dp0engine_3ds\target"
cd /d "%~dp0engine_3ds"
set RUSTFLAGS=
cargo +nightly 3ds build --bin rabuka_3ds --release --features 3ds
if %errorlevel% neq 0 (
    echo Build FAILED.
    pause
    exit /b 1
)
echo [5/6] Build succeeded

:: Copy output
if not exist "%~dp0output_3ds" mkdir "%~dp0output_3ds"
copy /Y "C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.3dsx" "%~dp0output_3ds\rabuka_3ds.3dsx" >nul

echo.
echo === Build Complete ===
echo File: output_3ds\rabuka_3ds.3dsx (abilities baked into cards.json)
echo.
echo Just load this .3dsx in Azahar - no extra files needed.
pause
