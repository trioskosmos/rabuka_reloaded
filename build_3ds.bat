@echo off
setlocal enabledelayedexpansion

echo === Rabuka 3DS Build Script ===
echo.

if not exist "C:\devkitPro\devkitARM\bin\arm-none-eabi-gcc.exe" (
    echo [FAIL] devkitPro not found at C:\devkitPro
    echo Install devkitPro with devkitARM + libctru from:
    echo   https://github.com/devkitPro/installer/releases
    pause
    exit /b 1
)
echo [1/7] devkitPro found

set DEVKITPRO=C:\devkitPro
set DEVKITARM=%DEVKITPRO%\devkitARM
set "PATH=%DEVKITARM%\bin;%DEVKITPRO%\tools\bin;%PATH%"

echo [2/7] Checking Rust nightly...
rustup toolchain list 2>nul | findstr nightly >nul
if %errorlevel% neq 0 ( rustup toolchain install nightly )
rustup component add rust-src --toolchain nightly-x86_64-pc-windows-msvc 2>nul
echo [2/7] Rust nightly + rust-src ready

echo [3/7] Checking cargo-3ds...
cargo 3ds --version >nul 2>&1
if %errorlevel% neq 0 ( cargo install cargo-3ds )
echo [3/7] cargo-3ds ready

echo [4/7] Pre-baking card data (unified bake tool)...
if not exist "%~dp0platforms\3ds\romfs" mkdir "%~dp0platforms\3ds\romfs"
if not exist "%~dp0platforms\3ds\romfs\decks" mkdir "%~dp0platforms\3ds\romfs\decks"
cd /d "%~dp0tools\bake"
cargo run --release -- 3ds "%~dp0platforms\3ds\romfs"
if %errorlevel% neq 0 (
    echo [FAIL] bake failed.
    pause
    exit /b 1
)
cd /d "%~dp0"
echo [4/7] cards.bin + abilities.json ready

copy /Y "%~dp0web_ui\decks\*.txt" "%~dp0platforms\3ds\romfs\decks\" >nul

echo [4/7] Card images...
dir /b "%~dp0platforms\3ds\romfs\cards\*.t3x" >nul 2>nul
if errorlevel 1 (
    echo [4/7] Converting card images...
    if exist "%~dp0web_ui\img\cards_webp\*.webp" (
        cd /d "%~dp0platforms\3ds"
        where python3 >nul 2>&1
        if !errorlevel! equ 0 (
            python scripts/convert_cards.py
        ) else (
            echo [WARN] python3 not found - skipping card image conversion
        )
    )
) else (
    echo [4/7] Card images already converted
)

echo [5/7] Building 3DS binary...
if exist "%~dp0platforms\3ds\target" rmdir /s /q "%~dp0platforms\3ds\target"
if exist "C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.3dsx" del "C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.3dsx"
cd /d "%~dp0platforms\3ds"
set RUSTFLAGS=
cargo +nightly 3ds build --bin rabuka_3ds --release --features 3ds
if %errorlevel% neq 0 (
    echo Build FAILED.
    pause
    exit /b 1
)
echo [5/7] Build succeeded

if not exist "%~dp0output_3ds" mkdir "%~dp0output_3ds"
copy /Y "C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.3dsx" "%~dp0output_3ds\rabuka_3ds.3dsx" >nul
echo [5/7] 3DSX: output_3ds\rabuka_3ds.3dsx

echo [6/7] Creating CIA (requires makerom from devkitPro)...
set "SRC_ELF=C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.elf"
set "SRC_SMDH=C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.smdh"
set "OUT_CIA=%~dp0output_3ds\rabuka_3ds.cia"

if exist "%DEVKITPRO%\tools\bin\makerom.exe" (
    if not exist "%SRC_SMDH%" (
        echo [6/7] SMDH not found at %SRC_SMDH% -- skipping CIA.
    ) else (
        makerom -f cia -o "%OUT_CIA%" -elf "%SRC_ELF%" -smdh "%SRC_SMDH%"
        if errorlevel 1 (
            echo [6/7] makerom failed -- skipping CIA.
        ) else (
            echo [6/7] CIA: output_3ds\rabuka_3ds.cia (install via FBI)
        )
    )
) else (
    echo [6/7] makerom not found -- skipping CIA.
    echo        3DSX file is ready for use via 3dslink or SD card.
)

echo.
echo === Build Complete ===
echo   3DSX: output_3ds\rabuka_3ds.3dsx (run via 3dslink / SD card)
echo   CIA:  see above (install via FBI on 3DS)
echo.
pause
