@echo off
setlocal enabledelayedexpansion

echo === Rabuka Dreamcast Build Script ===
echo.
echo NOTE: Requires WSL2 Ubuntu with the dreamcast-rs toolchain.
echo.

echo [1/3] Verifying baked card data...
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
echo [1/3] Done.
echo.

echo [2/3] Building Dreamcast binary in WSL...
wsl -d Ubuntu -u root bash -c "source /root/.cargo/env && . /opt/toolchains/dc/rust/misc/environ.sh 2>/dev/null && export CARGO_TARGET_DIR=/tmp/dc_build && cd /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/dc && kos-cargo build --release"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] Dreamcast build failed.
    pause
    exit /b 1
)
echo [2/3] Done.
echo.

echo [3/3] Copying output...
wsl -d Ubuntu -u root bash -c "find /tmp/dc_build -name 'rabuka_dc.elf' -exec cp {} /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/output_dc/rabuka_dc.elf \;"
if exist "%~dp0output_dc\rabuka_dc.elf" (
    echo Output: output_dc\rabuka_dc.elf
) else (
    echo [WARN] ELF not found.
)
echo [3/3] Done.
echo.

echo === Build Complete ===
echo File: output_dc\rabuka_dc.elf
echo.
pause
