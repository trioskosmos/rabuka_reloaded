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

echo [2/3] Building Dreamcast in WSL...
wsl -d Ubuntu -u root bash -c "source /root/.cargo/env && source /opt/toolchains/dc/rust/misc/environ.sh 2>/dev/null && export CARGO_TARGET_DIR=/opt/toolchains/dc/rust/build_target && export OUT=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/output_dc && mkdir -p $OUT && cd /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/dc && kos-cargo build --release 2>&1 && echo '=== STATICLIB DONE ===' && PATH=/opt/toolchains/dc/rust/sh-elf/bin:/opt/toolchains/dc/rust/kos/utils/build_wrappers:/opt/toolchains/dc/rust/bin:/usr/bin:/bin && sh-elf-gcc -c entry.c -o /opt/toolchains/dc/rust/build_target/entry.o -I/opt/toolchains/dc/rust/kos/include -I/opt/toolchains/dc/rust/kos/kernel/arch/dreamcast/include -ffunction-sections -fdata-sections && sh-elf-gcc /opt/toolchains/dc/rust/build_target/entry.o /opt/toolchains/dc/rust/build_target/sh-elf/release/librabuka_dc.a -Wl,--undefined=_rabuka_main -Wl,--gc-sections -T/opt/toolchains/dc/rust/kos/utils/ldscripts/shlelf.xc -nodefaultlibs -L/opt/toolchains/dc/rust/kos/lib/dreamcast -L/opt/toolchains/dc/rust/kos/addons/lib/dreamcast -L/opt/toolchains/dc/rust/kos-ports/lib -Wl,--start-group -lkallisti -lkallisti_arch -lm -lc -lgcc -Wl,--end-group -o $OUT/rabuka_dc.elf && echo '=== LINK DONE ===' && mkdir -p $OUT/disc && sh-elf-objcopy -R .stack -O binary $OUT/rabuka_dc.elf $OUT/disc/1ST_READ.BIN && echo '=== BINARY DONE ==='"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] Dreamcast build failed.
    pause
    exit /b 1
)
echo [2/3] Done.
echo.

echo [3/3] Verifying output...
if exist "%~dp0output_dc\disc\1ST_READ.BIN" (
    echo.
    echo ========================================
    echo  Dreamcast build successful!
    echo  ELF: output_dc\rabuka_dc.elf
    echo  1ST_READ.BIN: output_dc\disc\1ST_READ.BIN
    echo ========================================
    echo.
    echo To create a CD image, use:
    echo   wsl -d Ubuntu sh-elf-objcopy ... (or use a tool like cdirip)
) else (
    if exist "%~dp0output_dc\rabuka_dc.elf" (
        echo [OK] rabuka_dc.elf created but 1ST_READ.BIN may need manual step.
    ) else (
        echo [WARN] Output not found. Build may have failed.
    )
)
echo [3/3] Done.
echo.

pause
