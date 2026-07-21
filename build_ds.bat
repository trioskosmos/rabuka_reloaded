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

echo [1/2] Building DS binary...
cd /d "%~dp0engine_ds"
cargo +nightly build --release -Zbuild-std=core,alloc -Zjson-target-spec --target armv5te-nintendo-ds.json
cd /d "%~dp0"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] DS build failed.
    pause
    exit /b 1
)
echo [1/2] Done.
echo.

echo [2/2] Creating NDS ROM...
set TARGET_DIR=C:\rust_targets\armv5te-nintendo-ds\release
set ARM7_ELF=%DEVKITPRO%\calico\bin\ds7_maine.elf
if not exist "%~dp0output_ds" mkdir "%~dp0output_ds"
if exist "%DEVKITPRO%\tools\bin\ndstool.exe" (
    "%DEVKITPRO%\tools\bin\ndstool.exe" -c "%~dp0output_ds\rabuka.nds" -9 "%TARGET_DIR%\rabuka_ds" -7 "%ARM7_ELF%" -b "%DEVKITPRO%\calico\share\nds-icon.bmp" "Rabuka Reloaded;built with devkitARM;devkitpro.org"
) else (
    echo [WARN] ndstool not found. ELF output at: %TARGET_DIR%\rabuka_ds
)
echo [2/2] Done.
echo.

echo === Build Complete ===
if exist "%~dp0output_ds\rabuka.nds" (
    echo Output: output_ds\rabuka.nds
) else (
    echo ELF output: %TARGET_DIR%\rabuka_ds
)
echo.
pause
