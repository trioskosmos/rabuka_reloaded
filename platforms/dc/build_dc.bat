@echo off
setlocal enabledelayedexpansion
REM ============================================================
REM  Rabuka Reloaded - Dreamcast build (wasm2c pipeline)
REM  rust -> wasm32 -> wasm2c C -> sh-elf-gcc (KallistiOS) -> .cdi
REM
REM  Requires (one-time, in WSL Ubuntu):
REM    /root/sh-elf        prebuilt sh-elf toolchain
REM                        (drpaneas/dreamcast-toolchain-builds, gcc15.1.0-kos2.2.1)
REM    /root/kos           KallistiOS 2.2.1 (prebuilt libs)
REM    /root/wabt-1.0.41   wabt linux binaries (wasm2c)
REM    /root/mkdcdisc      built from github.com/Mark65537/mkdcdisc
REM    /root/dcbuild       working dir (created on first run; needs
REM                        wasm-rt runtime files + stub/sys/mman.h,
REM                        see platforms/dc/wasm/SETUP_WSL.md)
REM    apt install genisoimage
REM ============================================================

echo === [1/5] Building engine wasm (Windows cargo) ===
cd /d "%~dp0..\wasm"
cargo build --target wasm32-unknown-unknown --release
if %ERRORLEVEL% neq 0 (
    echo [FAIL] cargo build failed.
    pause
    exit /b 1
)

echo === [2/5] Transpiling wasm to C (WSL) ===
wsl -d Ubuntu -- bash -lc "cd /root/dcbuild && cp -f /mnt/c/rust_targets/wasm32-unknown-unknown/release/rabuka_wasm.wasm . && /root/wabt-1.0.41/bin/wasm2c rabuka_wasm.wasm -o rabuka_wasm.c"
if %ERRORLEVEL% neq 0 (
    echo [FAIL] wasm2c failed.
    pause
    exit /b 1
)

echo === [3/5] Compiling for SH-4 + linking KOS ELF (WSL, ~4 min) ===
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
wsl -d Ubuntu -- bash -lc "cp /root/dcbuild/rabuka.cdi /mnt/c/Emulators/Flycast/games/rabuka.cdi"
if exist "C:\Emulators\Flycast\games\rabuka.cdi" (
    echo.
    echo ================================================
    echo  SUCCESS: C:\Emulators\Flycast\games\rabuka.cdi
    echo  Open it in Flycast and play.
    echo ================================================
) else (
    echo [WARN] CDI not deployed to Flycast games folder.
)
pause
