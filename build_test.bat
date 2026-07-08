@echo off
setlocal enabledelayedexpansion
echo === Building ===
set DEVKITPRO=C:\devkitPro
set DEVKITARM=%DEVKITPRO%\devkitARM
set "PATH=%DEVKITARM%\bin;%DEVKITPRO%\tools\bin;%PATH%"
cd /d "C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_3ds"
where arm-none-eabi-gcc
if %errorlevel% neq 0 ( echo ERROR: gcc not found & exit /b 1 )
cargo +nightly 3ds build --bin rabuka_3ds --release --features 3ds
if %errorlevel% neq 0 ( echo BUILD FAILED & exit /b 1 )
echo BUILD OK
