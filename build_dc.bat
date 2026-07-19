@echo off
setlocal enabledelayedexpansion

echo === Rabuka Dreamcast Build Script ===
echo.
echo NOTE: Requires WSL2 Ubuntu with the dreamcast-rs toolchain at
echo /opt/toolchains/dc/rust/ (built once via install-toolchain.sh
echo and install-rust.sh from dreamcast.rs).
echo.
echo The output ELF is SH-4 architecture, ready for emulator testing.
echo.
echo [1/3] Verifying baked card data...
if not exist "%~dp0ports\psp\baked\decks.json" (
    echo [RUN] Baking cards first...
    cd /d "%~dp0ports\psp\tools\bake_cards"
    cargo run --release
    if %ERRORLEVEL% neq 0 (
        echo [FAIL] Bake failed.
        pause
        exit /b 1
    )
    cd /d "%~dp0"
) else (
    echo [OK] Card data found at ports\psp\baked\
)
echo [1/3] Done.
echo.

echo [2/3] Building Dreamcast binary in WSL...
wsl -d Ubuntu -u root bash -c "source /root/.cargo/env && . /opt/toolchains/dc/rust/misc/environ.sh 2>/dev/null && export CARGO_TARGET_DIR=/tmp/dc_build && cd /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/ports/dc && kos-cargo build --release"
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
echo For emulator testing: convert to CDI via:
echo   1. sh-elf-objcopy -R .stack -O binary input.elf 1ST_READ.BIN
echo   2. scramble 1ST_READ.BIN 1ST_READ.BIN
echo   3. Package with IP.BIN into CDI using genisoimage + cdi4dc
echo.
echo Emulator: flycast (https://flycast.cemu.net)
echo.
pause
