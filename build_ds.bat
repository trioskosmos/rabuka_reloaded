@echo off
setlocal enabledelayedexpansion

echo === Rabuka Nintendo DS Build Script ===
echo.

if "%DEVKITPRO%"=="" (
    echo [ERROR] DEVKITPRO is not set.
    pause
    exit /b 1
)
if "%DEVKITARM%"=="" (
    echo [ERROR] DEVKITARM is not set.
    pause
    exit /b 1
)

set DEVKITPRO=C:\devkitPro
set DEVKITARM=%DEVKITPRO%\devkitARM
set TARGET_DIR=C:\rust_targets\armv5te-nintendo-ds\release

echo [1/3] Baking card data (generating PSP JSONs for DS)...
cd /d "%~dp0tools\bake"
call cargo run --release -- psp
cd /d "%~dp0"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] Bake failed.
    pause
    exit /b 1
)
echo [1/3] Done.
echo.

echo [2/3] Building DS binary...
cd /d "%~dp0platforms\ds"
set CARGO_PROFILE_RELEASE_OPT_LEVEL=z
set CARGO_PROFILE_RELEASE_LTO=true
set CARGO_PROFILE_RELEASE_STRIP=true
set CARGO_PROFILE_RELEASE_PANIC=abort
set CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
call cargo +nightly build --release -Zbuild-std=core,alloc -Zjson-target-spec --target armv5te-nintendo-ds.json
cd /d "%~dp0"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] DS build failed.
    pause
    exit /b 1
)
echo [2/3] Done.
echo.

echo [3/3] Creating NDS ROM...
set ARM7_ELF=%DEVKITPRO%\calico\bin\ds7_maine.elf
if not exist "%~dp0output_ds" mkdir "%~dp0output_ds"
if exist "%DEVKITPRO%\tools\bin\ndstool.exe" (
    "%DEVKITPRO%\tools\bin\ndstool.exe" -c "%~dp0output_ds\rabuka.nds" -9 "%TARGET_DIR%\rabuka_ds" -7 "%ARM7_ELF%"
) else (
    echo [WARN] ndstool not found at %DEVKITPRO%\tools\bin\ndstool.exe
    echo       (set DEVKITPRO=C:\devkitPro or adjust PATH)
)
echo [3/3] Done.
echo.

echo === Build Complete ===
if exist "%~dp0output_ds\rabuka.nds" (
    echo Output: output_ds\rabuka.nds
) else (
    echo ELF output: %TARGET_DIR%\rabuka_ds
)
echo.
pause
