@echo off
REM ============================================================
REM  Build Rabuka Jaguar cartridge (.j64) for BigPEmu
REM  - Builds the m68k staticlib
REM  - Links + wraps as .j64 (Univ.bin + rom + allff)
REM  - Copies to C:\Emulators\BigPEmu\rabuka.j64
REM  - Launches BigPEmu
REM
REM  This is the ONLY entry point. Everything runs under WSL.
REM ============================================================

REM --- WSL-native script paths (forward slashes; backslashes get eaten by bash) ---
set WSL_ROOT=/mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/research/jaguar/rabuka_boot

echo.
echo [Rabuka Jaguar] Step 1/3: building m68k staticlib...
echo.
wsl -d Ubuntu -- bash %WSL_ROOT%/build_m68k.sh
if errorlevel 1 (
  echo [Rabuka Jaguar] Staticlib build FAILED.
  pause
  exit /b 1
)

echo.
echo [Rabuka Jaguar] Step 2/3: linking + wrapping .j64 cartridge...
echo.
wsl -d Ubuntu -- bash %WSL_ROOT%/build.sh
if errorlevel 1 (
  echo [Rabuka Jaguar] Cartridge build FAILED.
  pause
  exit /b 1
)

echo.
echo [Rabuka Jaguar] Step 3/3: launching BigPEmu...
echo.
start "" "C:\Emulators\BigPEmu\BigPEmu.exe" "C:\Emulators\BigPEmu\rabuka.j64"

echo.
echo [Rabuka Jaguar] Done. Cartridge: C:\Emulators\BigPEmu\rabuka.j64
pause