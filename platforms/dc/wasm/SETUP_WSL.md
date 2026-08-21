# Dreamcast wasm2c port — WSL one-time setup

The build (`build_dc.bat`) drives everything from Windows, but the WSL side
needs these persistent pieces under `/root` (the old rustc_codegen_gcc
toolchain from Jul 2026 is gone and NOT needed anymore).

## Pipeline

```
engine (Rust, no_std)  --cargo-->  rabuka_wasm.wasm      (platforms/wasm)
rabuka_wasm.wasm       --wasm2c--> rabuka_wasm.c (~20MB)  (/root/wabt-1.0.41)
rabuka_wasm.c + dc_main.c --kos-cc--> rabuka_dc.elf      (/root/sh-elf + /root/kos)
rabuka_dc.elf          --mkdcdisc--> rabuka.cdi           (/root/mkdcdisc)
```

## One-time steps (WSL Ubuntu, as root)

1. **sh-elf toolchain + prebuilt KallistiOS 2.2.1**
   Download `dreamcast-toolchain-gcc15.1.0-kos2.2.1-linux-x86_64.tar.gz` from
   <https://github.com/drpaneas/dreamcast-toolchain-builds/releases>
   (471MB) and extract in `/root` — it produces `/root/sh-elf`, `/root/kos`,
   `/root/kos-ports` (KOS libs are prebuilt; no `make` needed).

2. **wabt 1.0.41** (wasm2c — do NOT use Ubuntu's apt wabt 1.0.36; the
   generated C must match the runtime headers)
   ```
   cd /root && wget https://github.com/WebAssembly/wabt/releases/download/1.0.41/wabt-1.0.41-linux-x64.tar.gz
   tar xzf wabt-1.0.41-linux-x64.tar.gz
   ```

3. **mkdcdisc** (ELF → bootable .cdi)
   ```
   apt install meson ninja-build libisofs-dev libconfuse-dev pkg-config genisoimage
   git clone https://github.com/Mark65537/mkdcdisc.git /root/mkdcdisc
   cd /root/mkdcdisc && meson setup build && ninja -C build
   ```

4. **/root/dcbuild working dir**
   ```
   mkdir /root/dcbuild
   cp platforms/dc/wasm/runtime/wasm-rt* /root/dcbuild/   # patched 1.0.41 runtime
   mkdir -p /root/dcbuild/stub/sys && touch /root/dcbuild/stub/sys/mman.h
   ```
   The `stub/sys/mman.h` hides the POSIX header wabt includes unconditionally
   (unused when `WASM_RT_USE_MMAP=0`). `wasm-rt.h` in this folder carries one
   local patch: `WASM_RT_LONGJMP_UNCHECKED` uses plain `longjmp` when the
   signal handler is disabled (upstream calls `siglongjmp` unconditionally,
   which newlib/KOS lacks).

## Files

| File | Purpose |
|------|---------|
| `dc_main.c` | DC shell: 40x28 BIOS-font text grid, maple controller, the four `w2c_host_*` imports |
| `build_dc_wasm.sh` | full build (runtime + engine + link), ~4-5 min |
| `relink_dc.sh` | fast rebuild (shell change only), ~5 s |
| `runtime/` | patched wasm-rt 1.0.41 runtime + mman stub |

## Notes

- Engine wasm crate: `platforms/wasm` (feature `wasm` in engine/Cargo.toml).
  Exports `rabuka_wasm_game_run(seed)` (playable) plus the headless smoke
  exports; imports `host_clear_screen/host_println/host_poll_buttons/
  host_wait_vblank` from module `host`.
- Flicker fix: text prints only mark the grid dirty; the framebuffer is
  redrawn once per `wait_vblank`.
- Flycast v2.7 (win64) boots the .cdi with HLE BIOS — no BIOS file needed.
- RAM budget: ~4.3MB code+data at 0x8c010000, 93-page (6.1MB) wasm linear
  memory malloc'd at runtime — fits 16MB.
