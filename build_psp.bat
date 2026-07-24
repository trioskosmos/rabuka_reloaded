@echo off
setlocal enabledelayedexpansion

echo === Rabuka PSP Build Script ===
echo.

echo [1/4] Generating CJK bitmap font...
cd /d "%~dp0"
"%LOCALAPPDATA%\Microsoft\WindowsApps\python.exe" tools\gen_cjk_font.py
if %ERRORLEVEL% neq 0 (
    echo [WARN] CJK font gen failed (missing Windows GDI/font?). Falling back to existing cjk_font.rs
)
echo [1/4] Done.
echo.

echo [2/4] Baking card data (PSP)...
cd /d "%~dp0tools\bake"
cargo run --release -- psp
cd /d "%~dp0"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] Bake failed.
    pause
    exit /b 1
)
echo [2/4] Done.
echo.

echo [3/4] Building PSP binary...
cd /d "%~dp0platforms\psp"
cargo psp --release --features psp
cd /d "%~dp0"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] PSP build failed.
    pause
    exit /b 1
)
echo [4/4] Done.
echo.

echo === Build Complete ===
if exist "output_psp\RABUKA.PBP" (
    echo Output: output_psp\RABUKA.PBP
    echo.
    echo === CJK Font Notes ===
    echo The PSP embeds a 16x16 CJK bitmap font (src/cjk_font.rs, 234 glyphs, ~8.6KB).
    echo On JP-region PSPs, the system jpn0.pgf font is used for higher-quality rendering.
    echo On US/EU PSPs CJK falls back to the embedded bitmap font.
    echo To regenerate the font for new card data, run:
    echo   %%LOCALAPPDATA%%\Microsoft\WindowsApps\python.exe tools\gen_cjk_font.py
) else (
    echo [ERROR] EBOOT.PBP not found!
)
echo.
pause
