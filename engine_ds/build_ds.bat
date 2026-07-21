@echo off
setlocal enabledelayedexpansion

set DEVKITPRO=C:\devkitPro
set DEVKITARM=%DEVKITPRO%\devkitARM

set NIGHTLY=+nightly
set TARGET=armv5te-nintendo-ds
set TARGET_JSON=armv5te-nintendo-ds.json

set RUST_TARGETS=C:\rust_targets

echo Building Rabuka DS...
cargo %NIGHTLY% build --release -Zbuild-std=core,alloc -Zjson-target-spec --target %TARGET_JSON%
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

echo Creating NDS ROM...
"%DEVKITPRO%\tools\bin\ndstool.exe" -c output_ds\rabuka_ds.nds -9 "%RUST_TARGETS%\%TARGET%\release\rabuka_ds" -7 "%DEVKITPRO%\calico\bin\ds7_maine.elf"
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

echo Built: output_ds\rabuka_ds.nds
dir output_ds\rabuka_ds.nds
