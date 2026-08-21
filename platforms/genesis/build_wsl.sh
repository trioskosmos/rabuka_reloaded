#!/bin/bash
set -e
export PATH="/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
unset LD_LIBRARY_PATH CARGO_TARGET_DIR RUSTFLAGS
cd /mnt/c/Users/trios/OneDrive/Documents/rabuka_reloaded/platforms/genesis
cargo +nightly build --release 2>&1
echo "BUILD_EXIT=$?"
