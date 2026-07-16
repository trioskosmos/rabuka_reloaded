@echo off
setlocal enabledelayedexpansion

echo === Rabuka PSP Build Script ===
echo.

echo [1/4] Baking card data...
cargo run --bin bake_cards --manifest-path engine_psp/Cargo.toml 2>&1
if %ERRORLEVEL% neq 0 (
    echo [FAIL] Bake failed.
    pause
    exit /b 1
)
echo [1/4] Done.

echo [2/4] Building PSP binary...
cargo psp build --manifest-path engine_psp/Cargo.toml --release 2>&1
if %ERRORLEVEL% neq 0 (
    echo [FAIL] PSP build failed.
    pause
    exit /b 1
)
echo [2/4] Done.

echo [3/4] Copying output...
if not exist "output_psp" mkdir output_psp
copy "engine_psp\target\mipsel-sony-psp\release\EBOOT.PBP" "output_psp\RABUKA.PBP" >nul
echo [3/4] Done.

echo.
echo === Build Complete ===
echo Output: output_psp\RABUKA.PBP
echo.
echo To test:
echo   ppsspp output_psp\RABUKA.PBP
echo.

pause
