@echo off
setlocal enabledelayedexpansion
REM ============================================================
REM  Rabuka Reloaded - Dreamcast build (wasm2c pipeline)
REM  rust -> wasm32 -> wasm2c C -> sh-elf-gcc (KallistiOS) -> .cdi
REM
REM  One-time WSL setup: see platforms\dc\wasm\SETUP_WSL.md
REM  (prebuilt toolchain in /root/sh-elf + /root/kos, wabt 1.0.41 in
REM  /root/wabt-1.0.41, mkdcdisc in /root/mkdcdisc, /root/dcbuild
REM  working dir with wasm-rt runtime + stub/sys/mman.h).
REM
REM  This script always syncs the shell + build scripts from the repo
REM  into /root/dcbuild and runs the FULL build (recompiles the engine).
REM  Full build takes ~5 min; do not "optimize" it into a relink-only
REM  path -- a stale rabuka_wasm.o once shipped an AI player with an
REM  empty deck even though the fix was already committed.
REM ============================================================

echo === [1/5] Building engine wasm (Windows cargo) ===
cd /d "%~dp0..\wasm"
cargo build --target wasm32-unknown-unknown --release
if %ERRORLEVEL% neq 0 (
    echo [FAIL] cargo build failed.
    pause
    exit /b 1
)

echo === [2/5] Syncing sources + transpiling wasm to C (WSL) ===
wsl -d Ubuntu -- bash -lc "set -e; D=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/dc/wasm; mkdir -p /root/dcbuild/stub/sys && touch /root/dcbuild/stub/sys/mman.h; cp $D/dc_main.c $D/build_dc_wasm.sh $D/relink_dc.sh /root/dcbuild/ && cp $D/runtime/wasm-rt* /root/dcbuild/ && cp $D/runtime/sjis_table.* /root/dcbuild/ 2>/dev/null; cd /root/dcbuild && /root/wabt-1.0.41/bin/wasm2c /mnt/c/rust_targets/wasm32-unknown-unknown/release/rabuka_wasm.wasm -o rabuka_wasm.c"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] wasm2c failed.
    pause
    exit /b 1
)

echo === [3/5] Compiling for SH-4 + linking KOS ELF (WSL, ~5 min) ===
wsl -d Ubuntu -- bash -lc "cd /root/dcbuild && bash build_dc_wasm.sh > build.log 2>&1 || { tail -20 build.log; exit 1; } && grep -q 'ALL DONE' build.log"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] SH-4 build failed - see /root/dcbuild/build.log
    pause
    exit /b 1
)

echo === [4/5] Packaging bootable disc (WSL) ===
wsl -d Ubuntu -- bash -lc "set -e; /root/sh-elf/bin/sh-elf-strip /root/dcbuild/rabuka_dc.elf -o /root/dcbuild/rabuka_stripped.elf; cd /root/mkdcdisc/build && ./mkdcdisc -e /root/dcbuild/rabuka_stripped.elf -n RABUKA -o /root/dcbuild/rabuka.cdi"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] mkdcdisc failed.
    pause
    exit /b 1
)

echo === [5/5] Deploying ===
if not exist "%~dp0output" mkdir "%~dp0output"
copy /Y "\\wsl$\Ubuntu\root\dcbuild\rabuka_dc.elf" "%~dp0output\rabuka_dc.elf" >nul
copy /Y "\\wsl$\Ubuntu\root\dcbuild\rabuka_stripped.elf" "%~dp0output\rabuka_dc_stripped.elf" >nul
wsl -d Ubuntu -- bash -lc "cp /root/dcbuild/rabuka.cdi /mnt/c/Emulators/Flycast/games/rabuka.cdi && cp /root/dcbuild/rabuka.cdi /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/dc/output/rabuka.cdi && ls -lh /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/dc/output/rabuka.cdi /mnt/c/Emulators/Flycast/games/rabuka.cdi | awk '{print $9, $5}'"
if exist "C:\Emulators\Flycast\games\rabuka.cdi" if exist "%~dp0output\rabuka.cdi" (
    echo.
    echo ================================================
    echo  SUCCESS:
    echo    %~dp0output\rabuka.cdi
    echo    C:\Emulators\Flycast\games\rabuka.cdi
    echo  Open either in Flycast and play.
    echo ================================================
) else (
    echo [WARN] CDI not found in one of the deploy targets.
    if not exist "C:\Emulators\Flycast\games\rabuka.cdi" echo   missing: C:\Emulators\Flycast\games\rabuka.cdi
    if not exist "%~dp0output\rabuka.cdi" echo   missing: %~dp0output\rabuka.cdi
)
pause
