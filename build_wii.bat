@echo off
setlocal enabledelayedexpansion

echo === Rabuka Wii Build Script ===
echo.

echo [1/5] Checking devkitPPC...
if not exist "C:\devkitPro\devkitPPC\bin\powerpc-eabi-gcc.exe" (
    echo [FAIL] devkitPPC not found at C:\devkitPro
    echo Install devkitPro with devkitPPC from:
    echo   https://github.com/devkitPro/installer/releases
    pause
    exit /b 1
)
set DEVKITPRO=C:\devkitPro
set DEVKITPPC=%DEVKITPRO%\devkitPPC
set "PATH=%DEVKITPPC%\bin;%DEVKITPRO%\tools\bin;%PATH%"
echo [OK] devkitPPC found

echo [2/5] Verifying baked card data...
if not exist "%~dp0platforms\psp\baked\decks.json" (
    echo [RUN] Baking cards first...
    cd /d "%~dp0tools\bake"
    cargo run --release
    if %ERRORLEVEL% neq 0 (
        echo [FAIL] Bake failed.
        pause
        exit /b 1
    )
    cd /d "%~dp0"
) else (
    echo [OK] Card data found at platforms\psp\baked\
)

echo [3/5] Building Rust static library (Wii PowerPC)...
cd /d "%~dp0platforms\wii"
cargo +nightly build -Z build-std=core,alloc,panic_abort -Z json-target-spec --target powerpc-unknown-eabi.json --release
if %ERRORLEVEL% neq 0 (
    echo [FAIL] Rust build failed.
    pause
    exit /b 1
)
cd /d "%~dp0"
echo [3/5] Static library built

echo [4/5] Linking with devkitPPC + libogc...
set ELF=%~dp0output_wii\rabuka_wii.elf
set DOL=%~dp0output_wii\rabuka_wii.dol
set LIBC=%DEVKITPRO%\libogc\lib\wii
set RUST_LIB=%rust_targets%\powerpc-unknown-eabi\release
if not exist "%RUST_LIB%\librabuka_wii.a" (
    rem Try cargo's default target dir
    set RUST_LIB=%~dp0platforms\wii\target\powerpc-unknown-eabi\release
)
if not exist "%RUST_LIB%\librabuka_wii.a" (
    set RUST_LIB=C:\rust_targets\powerpc-unknown-eabi\release
)
rem Ensure output directory exists
if not exist "%~dp0output_wii" mkdir "%~dp0output_wii"

powerpc-eabi-gcc -mrvl -meabi -mhard-float -I"%DEVKITPRO%\libogc\include" -o "%ELF%" "%~dp0platforms\wii\entry.c" -L"%LIBC%" -L"%RUST_LIB%" -lrabuka_wii -logc -lc -lm
if %ERRORLEVEL% neq 0 (
    echo [FAIL] Linking failed.
    pause
    exit /b 1
)
echo [4/5] ELF linked: output_wii\rabuka_wii.elf

echo [5/5] Generating DOL...
elf2dol "%ELF%" "%DOL%"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] DOL generation failed.
    pause
    exit /b 1
)
echo [5/5] DOL generated: output_wii\rabuka_wii.dol

echo.
echo === Build Complete ===
echo ELF: output_wii\rabuka_wii.elf
echo DOL: output_wii\rabuka_wii.dol
echo.
echo To test in Dolphin:
echo   Open Dolphin -^> File -^> Load DOL... -^> select output_wii\rabuka_wii.dol
echo.
pause
