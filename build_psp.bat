@echo off
setlocal enabledelayedexpansion

echo === Rabuka PSP Build Script ===
echo.
echo NOTE: Japanese card names only render on JP-region PSPs (jpn0.pgf
echo in flash0:/font/). US/EU PSPs show "?" for CJK glyphs. To fix
echo universally, embed a CJK bitmap font in engine_psp/src/display.rs
echo or bundle jpn0.pgf and load via sceFontOpenUserMemory.
echo.

echo [1/3] Baking card data...
cd /d "%~dp0engine_psp\tools\bake_cards"
cargo run --release
cd /d "%~dp0"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] Bake failed.
    pause
    exit /b 1
)
echo [1/3] Done.
echo.

echo [2/3] Building PSP binary...
cd /d "%~dp0engine_psp"
cargo psp --release --features psp
cd /d "%~dp0"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] PSP build failed.
    pause
    exit /b 1
)
echo [2/3] Done.
echo.

echo [3/3] Copying output...
if not exist "output_psp" mkdir output_psp
copy "C:\rust_targets\mipsel-sony-psp\release\EBOOT.PBP" "output_psp\RABUKA.PBP" >nul 2>&1
if not exist "output_psp\RABUKA.PBP" (
    for /d %%d in (C:\rust_targets D:\rust_targets) do (
        if exist "%%d\mipsel-sony-psp\release\EBOOT.PBP" (
            copy "%%d\mipsel-sony-psp\release\EBOOT.PBP" "output_psp\RABUKA.PBP" >nul
        )
    )
)
echo [3/3] Done.
echo.

echo === Build Complete ===
if exist "output_psp\RABUKA.PBP" (
    echo Output: output_psp\RABUKA.PBP
    echo.
    echo To test:  "C:\Program Files\PPSSPP\PPSSPPWindows64.exe" output_psp\RABUKA.PBP
) else (
    echo [ERROR] EBOOT.PBP not found!
)
echo.
pause
