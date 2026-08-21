#!/bin/bash
set -e
export PATH="/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
unset LD_LIBRARY_PATH CARGO_TARGET_DIR RUSTFLAGS
cd /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/genesis
export CARGO_PROFILE_RELEASE_OPT_LEVEL=2
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
touch src/bin/rabuka_genesis.rs
cargo +nightly build --release 2>&1 | tail -4
m68k-linux-gnu-size target/m68k-unknown-none-elf/release/rabuka_genesis
mkdir -p output
m68k-linux-gnu-objcopy -O binary target/m68k-unknown-none-elf/release/rabuka_genesis output/rabuka_genesis.bin
ls -la output/
