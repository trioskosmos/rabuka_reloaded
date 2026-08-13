@echo off
REM Build the Rabuka GBA port and produce a .gba ROM.
REM
REM Requirements:
REM   - Rust nightly (rustup), e.g. cargo +nightly
REM   - agb crate (pulled by platforms\gba\Cargo.toml)
REM   - agb-gbafix  (cargo install agb-gbafix)
REM
REM Builds to C:\rust_targets\thumbv4t-none-eabi\release\rabuka_gba
setlocal
cd /d "%~dp0..\.."

REM Bake per-deck card data into the engine (keeps load_two_decks() in sync).
py -3 tools\bake_deck_cards.py
if errorlevel 1 (
    echo.
    echo FAILED: tools\bake_deck_cards.py
    exit /b 1
)

REM Bake GBA card art (8bpp tiles + palette) from the 3DS PNG cache.
cd /d "%~dp0..\.."
py -3 tools\bake_card_art.py
if errorlevel 1 (
    echo.
    echo FAILED: tools\bake_card_art.py
    exit /b 1
)

cd /d "%~dp0"

REM build-std provides target-specific core/alloc. Must be invoked with the
REM explicit -Z flags; the config in .cargo\config.toml handles target + gba.ld.
cargo +nightly build --release -Z build-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
if errorlevel 1 (
    echo.
    echo Build failed.
    exit /b 1
)

if not exist "%~dp0output" mkdir "%~dp0output"
agb-gbafix "C:\rust_targets\thumbv4t-none-eabi\release\rabuka_gba" -o "%~dp0output\rabuka_gba.gba"
if errorlevel 1 (
    echo.
    echo agb-gbafix failed.
    exit /b 1
)

echo.
echo Built output\rabuka_gba.gba
