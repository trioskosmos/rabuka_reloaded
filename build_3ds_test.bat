@echo off
setlocal enabledelayedexpansion

echo === Rabuka 3DS TEST Build (minimal romfs) ===
echo.

if not exist "C:\devkitPro\devkitARM\bin\arm-none-eabi-gcc.exe" (
    echo [FAIL] devkitPro not found at C:\devkitPro
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

echo [4.5/7] Building minimal romfs (data only, no images/font/audio)...
set "ROMFS=%~dp0platforms\3ds\romfs"
set "ROMFS_FULL=%~dp0platforms\3ds\romfs_full"

REM Save full romfs aside
if exist "%ROMFS_FULL%" rmdir /s /q "%ROMFS_FULL%"
rename "%ROMFS%" "romfs_full"

REM Create minimal romfs with only data files
mkdir "%ROMFS%"
mkdir "%ROMFS%\decks"
copy /Y "%ROMFS_FULL%\cards.bin" "%ROMFS%\" >nul
copy /Y "%ROMFS_FULL%\abilities.json" "%ROMFS%\" >nul
xcopy /E /Y /Q "%ROMFS_FULL%\decks\*" "%ROMFS%\decks\" >nul
xcopy /E /Y /Q "%ROMFS_FULL%\locales\*" "%ROMFS%\locales\" >nul
if exist "%ROMFS_FULL%\cards_manifest.json" copy /Y "%ROMFS_FULL%\cards_manifest.json" "%ROMFS%\" >nul
echo [4.5/7] Minimal romfs ready (cards.bin + abilities.json + decks + locales)

echo [5/7] Building 3DS binary...
if exist "%~dp0platforms\3ds\target" rmdir /s /q "%~dp0platforms\3ds\target"
if exist "C:\rust_targets\armv6k-nintendo-3ds" rmdir /s /q "C:\rust_targets\armv6k-nintendo-3ds"
cd /d "%~dp0platforms\3ds"
set RUSTFLAGS=
set CARGO_PROFILE_RELEASE_LTO=false
set CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
cargo +nightly 3ds build --bin rabuka_3ds --release --features 3ds
if %errorlevel% neq 0 (
    echo Build FAILED.
    REM Restore full romfs
    rmdir /s /q "%ROMFS%"
    rename "%ROMFS_FULL%" "romfs"
    cd /d "%~dp0"
    pause
    exit /b 1
)
echo [5/7] Build succeeded

REM Restore full romfs
rmdir /s /q "%ROMFS%"
rename "%ROMFS_FULL%" "romfs"
echo [5.5/7] Full romfs restored

if not exist "%~dp0output_3ds" mkdir "%~dp0output_3ds"
copy /Y "C:\rust_targets\armv6k-nintendo-3ds\release\rabuka_3ds.3dsx" "%~dp0output_3ds\rabuka_3ds.3dsx" >nul

for %%F in ("%~dp0output_3ds\rabuka_3ds.3dsx") do set /a "SIZE_MB=%%~zF / 1048576"
echo.
echo === Build Complete ===
echo   3DSX: output_3ds\rabuka_3ds.3dsx (~!SIZE_MB! MB)
echo   No card images, no font, no audio. Cards render as text.
echo.
pause
