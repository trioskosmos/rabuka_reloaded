@echo off
setlocal enabledelayedexpansion

echo === Rabuka Nintendo DS Build Script ===
echo.

if "%DEVKITPRO%"=="" (
    echo [ERROR] DEVKITPRO is not set. Install devkitPro and set environment variables.
    pause
    exit /b 1
)
if "%DEVKITARM%"=="" (
    echo [ERROR] DEVKITARM is not set.
    pause
    exit /b 1
)

echo [1/3] Baking card data...
cd /d "%~dp0ports\psp\tools\bake_cards"
cargo run --release
cd /d "%~dp0"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] Bake failed.
    pause
    exit /b 1
)
echo [1/3] Done.
echo.

echo [2/3] Building DS binary...
cd /d "%~dp0ports\ds"
cargo build --release --target armv5te-nintendo-ds
cd /d "%~dp0"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] DS build failed.
    pause
    exit /b 1
)
echo [2/3] Done.
echo.

echo [3/3] Creating NDS ROM...
set TARGET_DIR=%~dp0ports\ds\target\armv5te-nintendo-ds\release
if exist "%DEVKITPRO%\tools\bin\ndstool.exe" (
    "%DEVKITPRO%\tools\bin\ndstool.exe" -c "%~dp0output_ds\rabuka.nds" -9 "%TARGET_DIR%\rabuka_ds.elf"
) else (
    echo [WARN] ndstool not found. ELF output at: %TARGET_DIR%\rabuka_ds.elf
)
echo [3/3] Done.
echo.

echo === Build Complete ===
if exist "%~dp0output_ds\rabuka.nds" (
    echo Output: output_ds\rabuka.nds
) else (
    echo ELF output: %TARGET_DIR%\rabuka_ds.elf
)
echo.
pause
