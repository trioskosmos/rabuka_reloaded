@echo off
setlocal enabledelayedexpansion
set DEVKITPRO=C:\devkitPro
set DEVKITARM=%DEVKITPRO%\devkitARM
set "PATH=%DEVKITARM%\bin;%DEVKITPRO%\tools\bin;%PATH%"
cd /d "%~dp0engine_3ds"
echo [build] cargo +nightly 3ds build --bin rabuka_3ds --release --features 3ds
cargo +nightly 3ds build --bin rabuka_3ds --release --features 3ds
if %errorlevel% neq 0 (
    echo BUILD FAILED
    pause
    exit /b 1
)
echo BUILD OK
pause
