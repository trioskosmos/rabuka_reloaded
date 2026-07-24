@echo off
setlocal enabledelayedexpansion
set DEVKITPRO=C:\devKitPro
set DEVKITPPC=%DEVKITPRO%\devkitPPC
set PATH=%DEVKITPPC%\bin;%DEVKITPRO%\tools\bin;%PATH%

echo [1/5] Rust staticlib...
cd /d "%~dp0platforms\wii"
cargo +nightly build -Z build-std=core,alloc,panic_abort -Z json-target-spec --target powerpc-unknown-eabi.json --release
if %ERRORLEVEL% neq 0 ( echo FAIL; pause; exit /b 1 )

echo [2/5] Linking...
set RUSTLIB=C:\rust_targets\powerpc-unknown-eabi\release
if not exist "%RUSTLIB%\librabuka_wii.a" set RUSTLIB=%~dp0platforms\wii\target\powerpc-unknown-eabi\release
if not exist "%~dp0output_wii" mkdir "%~dp0output_wii"
powerpc-eabi-gcc -mrvl -meabi -mhard-float -I"%DEVKITPRO%\libogc\include" -o "%~dp0output_wii\rabuka_wii.elf" entry.c display.c -L"%RUSTLIB%" -lrabuka_wii -L"%DEVKITPRO%\libogc\lib\wii" -logc -lc -lm
if %ERRORLEVEL% neq 0 ( echo FAIL; pause; exit /b 1 )

echo [3/5] DOL...
elf2dol "%~dp0output_wii\rabuka_wii.elf" "%~dp0output_wii\rabuka_wii.dol"
echo DONE %~dp0output_wii\rabuka_wii.dol
pause
