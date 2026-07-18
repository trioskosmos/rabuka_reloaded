@echo off
echo Opening all Dreamcast toolchain download links in browser...
echo.

echo === Source tarballs ===
start "" "https://ftpmirror.gnu.org/gnu/binutils/binutils-2.44.tar.xz"
start "" "https://sourceware.org/pub/newlib/newlib-4.5.0.20241231.tar.gz"
start "" "https://ftpmirror.gnu.org/gnu/gdb/gdb-16.2.tar.xz"
start "" "https://ftpmirror.gnu.org/gnu/gmp/gmp-6.2.1.tar.xz"
start "" "https://ftpmirror.gnu.org/gnu/mpfr/mpfr-4.1.0.tar.xz"
start "" "https://ftpmirror.gnu.org/gnu/mpc/mpc-1.2.1.tar.gz"
start "" "https://libisl.sourceforge.io/isl-0.24.tar.bz2"

echo === Git repos (ZIP downloads) ===
start "" "https://github.com/dreamcast-rs/gcc/archive/refs/heads/master.zip"
start "" "https://github.com/dreamcast-rs/rustc_codegen_gcc/archive/refs/heads/2025-08-14.zip"
start "" "https://github.com/dreamcast-rs/libc/archive/refs/heads/libc-0.2-kos.zip"
start "" "https://github.com/dreamcast-rs/rust/archive/refs/heads/kos-2025-08-14.zip"
start "" "https://github.com/dreamcast-rs/rust-for-dreamcast/archive/refs/heads/master.zip"
start "" "https://github.com/dreamcast-rs/KallistiOS/archive/refs/heads/master.zip"

echo.
echo Done! All download tabs should be opening now.
echo.
echo Place everything in: %%USERPROFILE%%\Downloads\dc-chain-sources\
echo Then let me know and I'll process them.
pause
