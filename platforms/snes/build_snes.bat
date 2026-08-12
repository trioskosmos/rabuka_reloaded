@echo off
REM Build the Rabuka SNES port and produce a .sfc ROM.
REM
REM STATUS: DRAFT — expects rust-mos + llvm-mos-sdk on WSL2/Ubuntu, not native
REM Windows. This .bat is a thin wrapper that shells into WSL; see build_snes.sh.
REM
REM Prereqs (see engine\PORTS.md "SNES - the path forward"):
REM   1. rust-mos installed in WSL via prebuilt release tarball +
REM      `rustup toolchain link mos <dir>` (or `mrkits/rust-mos` Docker image).
REM   2. llvm-mos-sdk `snes` platform (mos-snes-clang + crt0) + LoROM linker
REM      script (lorom.ld).
REM   3. targets\mos-snes-none.json committed in platforms\snes.
REM
setlocal
cd /d "%~dp0..\.."
python tools\bake_deck_cards.py
if errorlevel 1 (
    echo.
    echo FAILED: tools\bake_deck_cards.py
    exit /b 1
)

cd /d "%~dp0"
wsl bash build_snes.sh
if errorlevel 1 (
    echo.
    echo WSL SNES build failed (or WSL not configured). See engine\PORTS.md.
    exit /b 1
)

echo.
echo Built output\rabuka_snes.sfc
