@echo off
REM Build every diagnostic ROM for the Rabuka GBA port into output\.
REM
REM One ROM per question. There are no in-game hotkeys: each variant's
REM behaviour is fixed at compile time by a cargo feature, so the ROM you
REM boot is the ROM whose behaviour you get. Build the baseline (the
REM control) alongside them and compare.
REM
REM Skip the card-art bake: the baked art is verified (1905 distinct blobs
REM per kind, valid LZ77 headers), and re-baking on every build would churn
REM that verified output for nothing. Run tools\bake_card_art.py by hand if
REM the source art actually changes.
setlocal enabledelayedexpansion
REM devkitPro msys2 ships a Unix `link` that shadows MSVC's link.exe and
REM breaks cargo's host build scripts.
set "PATH=%PATH:C:\devkitPro\msys2\usr\bin;=%"
set "PATH=%PATH:C:\devkitPro\msys2\mingw64\bin;=%"
set "PATH=%PATH:C:\devkitPro\msys2\mingw32\bin;=%"
set "PATH=%PATH:C:/devkitPro/msys2/usr/bin;=%"

set "OUT=%~dp0output"
if not exist "%OUT%" mkdir "%OUT%"

REM agb-gbafix fails on OneDrive paths (file-mapping error 1224): stage in
REM local temp, fix there, copy back.
set "STAGE=%TEMP%\rabuka_gba_diag"
if not exist "%STAGE%" mkdir "%STAGE%"

set "ELF=C:\rust_targets\thumbv4t-none-eabi\release\rabuka_gba"
set "BUILT=0"
set "FAILED="

call :build_rom baseline ""
call :build_rom noart    "diag-noart"
call :build_rom trace    "diag-trace"
call :build_rom cardid   "diag-cardid"
call :build_rom spriteraw "diag-spriteraw"
call :build_rom bgspeck  "diag-bgspeck"

echo.
if defined FAILED (
  echo FAILED: !FAILED!
  exit /b 1
)
echo Built %BUILT% ROM(s) into %OUT%
exit /b 0

:build_rom
REM %1 = short name, %2 = feature flag (empty for baseline)
set "NAME=%~1"
set "FEAT=%~2"
echo.
echo === !NAME! ^(feature: "!FEAT!"^)
if "!FEAT!"=="" (
  cargo +nightly build --release -Z build-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
) else (
  cargo +nightly build --release --no-default-features --features "!FEAT!" -Z build-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
)
if errorlevel 1 (
  set "FAILED=!FAILED! !NAME!"
  exit /b 0
)
if not exist "%ELF%" (
  echo   ELF missing: %ELF%
  set "FAILED=!FAILED! !NAME!"
  exit /b 0
)
copy /Y "%ELF%" "%STAGE%\rom" >nul
agb-gbafix "%STAGE%\rom" -o "%STAGE%\rom.gba"
if errorlevel 1 (
  set "FAILED=!FAILED! !NAME!"
  exit /b 0
)
copy /Y "%STAGE%\rom.gba" "%OUT%\rabuka_gba_!NAME!.gba" >nul
set /a BUILT+=1
for %%F in ("%OUT%\rabuka_gba_!NAME!.gba") do echo   %%~nxF  %%~zF bytes
exit /b 0