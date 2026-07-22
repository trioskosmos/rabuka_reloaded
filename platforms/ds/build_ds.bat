@echo off
setlocal enabledelayedexpansion

set DEVKITPRO=C:\devkitPro
set DEVKITARM=%DEVKITPRO%\devkitARM

set NIGHTLY=+nightly
set TARGET=armv5te-nintendo-ds
set TARGET_JSON=armv5te-nintendo-ds.json

set RUST_TARGETS=C:\rust_targets

echo Building Rabuka DS...

:: Step 1: Bake the card database
echo.
echo [1/4] Baking card database...
pushd ..\..\tools\bake
call cargo run --release -- ds
if %ERRORLEVEL% neq 0 popd & exit /b %ERRORLEVEL%
popd

:: Step 2: Build DS binary
echo.
echo [2/4] Building DS binary...
cargo %NIGHTLY% build --release -Zbuild-std=core,alloc -Zjson-target-spec --target %TARGET_JSON%
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

:: Step 3: Create NDS ROM
echo.
echo [3/4] Creating NDS ROM...
"%DEVKITPRO%\tools\bin\ndstool.exe" -c output_ds\rabuka_ds.nds -9 "%RUST_TARGETS%\%TARGET%\release\rabuka_ds" -7 "%DEVKITPRO%\calico\bin\ds7_maine.elf"
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

:: Step 4: Copy to root output_ds\ for convenience
echo.
echo [4/4] Copying to output_ds\rabuka.nds...
copy /y output_ds\rabuka_ds.nds ..\..\output_ds\rabuka.nds >nul

echo.
echo === Build Complete ===
echo Output: output_ds\rabuka.nds
dir ..\..\output_ds\rabuka.nds
