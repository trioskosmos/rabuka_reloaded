@echo off
REM Build the Rabuka PS1 port and copy the PS-X EXE to output_ps1\.
REM
REM Requirements:
REM   - Rust nightly-2025-05-23 (rustup) with rust-src component
REM   - cargo-psx  (cargo install --path research/ps1_rust/psx-sdk-rs/cargo-psx)
REM   - psx-sdk-rs  (fetched as a git dependency of platforms/ps1)
REM   - Python 3  (for tools\bake_deck_cards.py)
REM
REM Builds to C:\rust_targets\mipsel-sony-psx\release\rabuka_ps1.exe
setlocal
cd /d "%~dp0"

REM Bake per-deck card data into the engine (from cards.json + web_ui\decks).
REM Keeps engine\src\decks_cards_gen.rs in sync so load_two_decks() always has
REM the right cards for the selected decks.
python tools\bake_deck_cards.py
if errorlevel 1 (
    echo.
    echo FAILED: tools\bake_deck_cards.py
    exit /b 1
)

cd platforms\ps1

REM Fat LTO + size opt shrink the MIPS code enough to fit PS1's 2MB RAM
REM (without it, .text alone overflows the region).
set RUSTFLAGS=-Copt-level=z -Clto=fat -Cembed-bitcode=yes -Ccodegen-units=1

cargo psx build
if errorlevel 1 (
    echo.
    echo Build failed.
    exit /b 1
)

if not exist "%~dp0output_ps1" mkdir "%~dp0output_ps1"
copy /y "C:\rust_targets\mipsel-sony-psx\release\rabuka_ps1.exe" "%~dp0output_ps1\rabuka.ps-exe" >nul
echo.
echo Built output_ps1\rabuka.ps-exe
echo Run it in DuckStation:
echo   "C:\Users\trios\AppData\Local\Programs\DuckStation\duckstation-qt-x64-ReleaseLTCG.exe" "%~dp0output_ps1\rabuka.ps-exe"
endlocal
