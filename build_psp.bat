@echo off
setlocal enabledelayedexpansion

echo === Rabuka PSP Build Script ===
echo.

echo [1/3] Baking card data (PSP)...
cd /d "%~dp0tools\bake"
cargo run --release -- psp
cd /d "%~dp0"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] Bake failed.
    pause
    exit /b 1
)
echo [1/3] Done.
echo.

echo [2/3] Building PSP binary...
cd /d "%~dp0platforms\psp"
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
) else (
    echo [ERROR] EBOOT.PBP not found!
)
echo.
pause
