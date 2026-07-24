@echo off
REM Rabuka Wii build script
REM Requires: devkitPro toolchain with devkitPPC, nightly Rust, rust-src component
REM
REM Install:
REM   1. devkitPro: https://devkitpro.org/wiki/Getting_Started
REM      Select devkitPPC + libogc from installer
REM   2. Rust nightly + rust-src:
REM      rustup toolchain install nightly
REM      rustup component add rust-src --toolchain nightly
REM
REM Build:
REM   cargo +nightly build -Z build-std=std,panic_abort --target powerpc-unknown-eabi.json --release
REM
REM Post-process:
REM   powerpc-eabi-objcopy -O binary target/powerpc-unknown-eabi/release/rabuka_wii rabuka_wii.dol
REM   (or use elf2dol from devkitPro)

set TARGET=powerpc-unknown-eabi
set PROFILE=release

echo Building rabuka_wii for Wii...
cargo +nightly build -Z build-std=std,panic_abort --target %TARGET% --%PROFILE%

if errorlevel 1 (
    echo Build failed!
    exit /b 1
)

echo Generating DOL...
set ELF=target\%TARGET%\%PROFILE%\rabuka_wii
set DOL=..\..\output_wii\rabuka_wii.dol

if not exist ..\..\output_wii mkdir ..\..\output_wii

powerpc-eabi-objcopy -O binary %ELF% %DOL%
if errorlevel 1 (
    echo objcopy failed, trying elf2dol...
    elf2dol %ELF% %DOL%
)

echo Done: %DOL%
