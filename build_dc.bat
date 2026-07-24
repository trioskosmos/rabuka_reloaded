@echo off
setlocal enabledelayedexpansion

echo === Rabuka Dreamcast Build Script ===
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

echo [2/3] Building Dreamcast staticlib + linking in WSL...
wsl -d Ubuntu -u root bash -c "source /root/.cargo/env && source /opt/toolchains/dc/rust/misc/environ.sh 2>/dev/null && export CARGO_TARGET_DIR=/tmp/dc_build && export OUTPUT_DIR=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/output_dc && mkdir -p $OUTPUT_DIR && cd /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/dc && kos-cargo build --release 2>&1 && echo '=== COMPILATION DONE ===' && sh-elf-gcc -c entry.c -o /tmp/dc_build/entry.o -I${KOS_BASE}/include -I${KOS_BASE}/kernel/arch/dreamcast/include && echo '=== ENTRY COMPILED ===' && sh-elf-gcc /tmp/dc_build/entry.o /tmp/dc_build/sh-elf/release/librabuka_dc.a -Wl,--gc-sections -T${KOS_BASE}/utils/ldscripts/shlelf.xc -nodefaultlibs -L${KOS_BASE}/lib/dreamcast -L${KOS_BASE}/addons/lib/dreamcast -L${KOS_PORTS}/lib -Wl,--start-group -lkallisti -lm -lc -lgcc -Wl,--end-group -o $OUTPUT_DIR/rabuka_dc.elf && echo '=== LINKING DONE ===' && ${KOS_OBJCOPY} -R .stack -O binary $OUTPUT_DIR/rabuka_dc.elf $OUTPUT_DIR/disc/1ST_READ.BIN && echo '=== BINARY CREATED ==='"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] Dreamcast build failed.
    pause
    exit /b 1
)
echo [2/3] Done.
echo.

echo [3/3] Copying output...
if exist "%~dp0output_dc\disc\1ST_READ.BIN" (
    echo 1ST_READ.BIN created successfully.
    echo Output: output_dc\rabuka_dc.elf
) else (
    echo [WARN] 1ST_READ.BIN not found. Build may have failed.
)
echo [3/3] Done.
echo.

echo === Build Complete ===
echo File: output_dc\rabuka_dc.elf
echo.
pause
