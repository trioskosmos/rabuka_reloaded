#!/usr/bin/env bash
# Build the Rabuka SNES port and produce a .sfc ROM.
#
# STATUS: DRAFT — expects rust-mos + llvm-mos-sdk inside WSL2/Ubuntu.
#
# Prereqs (see engine/PORTS.md "SNES - the path forward"):
#   1. rust-mos toolchain, linked into rustup as `mos`:
#        rustup toolchain link mos <rust-mos-install-dir>
#      (or use the mrkits/rust-mos Docker image).
#   2. llvm-mos-sdk `snes` platform: provides mos-snes-clang + crt0.
#   3. targets/mos-snes-none.json committed under platforms/snes.
#
# The linker flags below are placeholders to be confirmed against the SDK.
set -euo pipefail

# Locate repo root from this script's path (platforms/snes/build_snes.sh)
cd "$(dirname "$0")"
cd ../..

# Bake per-deck card data into the engine (keeps load_two_decks() in sync).
python3 tools/bake_deck_cards.py

cd platforms/snes

# Build with the rust-mos toolchain against the custom SNES target spec.
# build.rs in this crate wires the LoROM linker script + crt0.
cargo +mos build --release --target targets/mos-snes-none.json

# The rust-mos / llvm-mos lld already emits a .sfc via the target's exe-suffix;
# copy it to output/. If objcopy to .sfc is needed, run it here.
mkdir -p output
cp "target/mos-snes-none/release/rabuka_snes" output/rabuka_snes.sfc

echo "Built output/rabuka_snes.sfc"
