@echo off
setlocal

echo === Rabuka PSP Build Script ===
echo.

echo [1/3] Baking card data (PSP)...
cd /d "%~dp0..\..\tools\bake"
cargo run --release -- psp
cd /d "%~dp0..\.."
echo [1/3] Done.
echo.

echo [2/3] Building PSP binary (requires nightly + cargo-psp)...
cd /d "%~dp0"
set CARGO_PROFILE_RELEASE_LTO=true
set CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
rustup override set nightly 2>nul
cargo psp --release --features psp
if errorlevel 1 (
    rustup override set stable 2>nul
    echo [FAIL] PSP build failed.
    echo Make sure you have: rustup toolchain install nightly ^&^& cargo install cargo-psp
    pause
    exit /b 1
)
rustup override set stable 2>nul
cd /d "%~dp0..\.."
echo [2/3] Done.
echo.

echo === Build Complete ===
if exist "%~dp0output\RABUKA.PBP" (
    echo Output: output\RABUKA.PBP
) else (
    echo [ERROR] EBOOT.PBP not found!
)
echo.
pause