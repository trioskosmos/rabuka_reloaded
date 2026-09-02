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
REM Fix link.exe shadowing: devkitPro msys2 provides a Unix 'link' (hardlink util)
REM that shadows MSVC's link.exe on PATH, breaking cargo's host build scripts.
set "PATH=%PATH:C:\devkitPro\msys2\usr\bin;=%"
set "PATH=%PATH:C:\devkitPro\msys2\mingw64\bin;=%"
set "PATH=%PATH:C:\devkitPro\msys2\mingw32\bin;=%"
set "PATH=%PATH:C:/devkitPro/msys2/usr/bin;=%"
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
REM Host link fix: if MSVC link.exe is missing (no Build Tools), use the GNU
REM nightly host which uses gcc/ld instead.
where link.exe >nul 2>&1
if %errorlevel% neq 0 (
  echo [INFO] MSVC link.exe not found, using GNU toolchain for host build
  cargo +nightly-x86_64-pc-windows-gnu build --release -Z build-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
) else (
  cargo +nightly build --release -Z build-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
)
if errorlevel 1 (
    echo.
    echo Build failed.
    exit /b 1
)

if not exist "%~dp0output" mkdir "%~dp0output"

REM agb-gbafix fails on OneDrive paths (file-mapping error 1224).
REM Workaround: copy ELF to local temp, fix there, copy back.
set "TMP_GBABUILD=%TEMP%\rabuka_gba_build"
if not exist "%TMP_GBABUILD%" mkdir "%TMP_GBABUILD%"
copy /Y "C:\rust_targets\thumbv4t-none-eabi\release\rabuka_gba" "%TMP_GBABUILD%\rabuka_gba" >nul
agb-gbafix "%TMP_GBABUILD%\rabuka_gba" -o "%TMP_GBABUILD%\rabuka_gba.gba"
if errorlevel 1 (
    echo.
    echo agb-gbafix failed.
    exit /b 1
)
copy /Y "%TMP_GBABUILD%\rabuka_gba.gba" "%~dp0output\rabuka_gba.gba" >nul
if errorlevel 1 (
    echo.
    echo Failed to copy final ROM.
    exit /b 1
)

echo.
echo Built output\rabuka_gba.gba
pause
