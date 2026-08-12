// Copyright (c) 2026 Matheus C. França
// SPDX-License-Identifier: Apache-2.0
//! SNES convenience library: safe wrappers around PPU, VRAM, CGRAM,
//! joypad, NMI and DMA – all `unsafe` confined to tiny I/O helpers.

use core::ptr::write_volatile;

use crate::hardware;

// ---------------------------------------------------------------------------
// VBlank / Joypad – mutable statics (names must match crt0 NMI handler)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static mut vblank_flag: u16 = 0;
#[unsafe(no_mangle)]
pub static mut pad_keys: [u16; 2] = [0; 2];
#[unsafe(no_mangle)]
pub static mut pad_keysold: [u16; 2] = [0; 2];
#[unsafe(no_mangle)]
pub static mut pad_keysdown: [u16; 2] = [0; 2];

// Joypad button masks
pub const KEY_B: u16 = 0x8000;
pub const KEY_Y: u16 = 0x4000;
pub const KEY_SELECT: u16 = 0x2000;
pub const KEY_START: u16 = 0x1000;
pub const KEY_UP: u16 = 0x0800;
pub const KEY_DOWN: u16 = 0x0400;
pub const KEY_LEFT: u16 = 0x0200;
pub const KEY_RIGHT: u16 = 0x0100;
pub const KEY_A: u16 = 0x0080;
pub const KEY_X: u16 = 0x0040;
pub const KEY_L: u16 = 0x0020;
pub const KEY_R: u16 = 0x0010;

/// Returns `true` if **all** of `buttons` are currently held on pad (0 or 1).
pub fn held(pad: usize, buttons: u16) -> bool {
    unsafe { pad_keys[pad] & buttons == buttons }
}

/// Returns `true` if **all** of `buttons` were just pressed this frame.
pub fn pressed(pad: usize, buttons: u16) -> bool {
    unsafe { pad_keysdown[pad] & buttons == buttons }
}

// ---------------------------------------------------------------------------
// Display control
// ---------------------------------------------------------------------------

/// Disable display (force blank) and turn off NMI / IRQ.
pub fn ppu_off() {
    hardware::write_inidisp(0x80);
    hardware::write_nmitimen(0x00);
}

/// Enable display at full brightness.
pub fn ppu_on() {
    hardware::write_inidisp(0x0f);
}

/// Zero all PPU write registers $2100–$212F and reset BG scroll.
/// **Must be called during force-blank before the first `ppu_on()`.**
pub fn ppu_init() {
    for addr in 0x2100..=0x212F {
        unsafe { write_volatile(addr as *mut u8, 0) }
    }
    bg_scroll_zero();
}

/// Wait for the next VBlank (via NMI).
pub fn wait_vblank() {
    hardware::write_nmitimen(0x81);
    unsafe { vblank_flag = 0 }
    unsafe { core::arch::asm!("wai", options(nomem, nostack)) }
}

// ---------------------------------------------------------------------------
// VRAM access
// ---------------------------------------------------------------------------

/// Set VRAM word address for subsequent `vram_write` operations.
pub fn vram_set_addr(addr: u16) {
    hardware::write_vmaddl(addr as u8);
    hardware::write_vmaddh((addr >> 8) as u8);
}

/// Write one VRAM word (low byte then high byte).
pub fn vram_write(lo: u8, hi: u8) {
    hardware::write_vmdatal(lo);
    hardware::write_vmdatah(hi);
}

// ---------------------------------------------------------------------------
// DMA helpers (only safe during force-blank)
// ---------------------------------------------------------------------------

/// GP‑DMA: copy `size` bytes from `src` to VRAM at word address `vram_addr`.
/// **Requires force-blank active.**
pub fn dma_copy_vram(src: *const u8, vram_addr: u16, size: u16) {
    hardware::write_vmain(0x80);
    vram_set_addr(vram_addr);

    let ptr = src as u16;
    unsafe {
        write_volatile(hardware::dmap(0), 0x01);
        write_volatile(hardware::bbad(0), 0x18);
        write_volatile(hardware::a1tl(0), ptr as u8);
        write_volatile(hardware::a1th(0), (ptr >> 8) as u8);
        write_volatile(hardware::dasl(0), size as u8);
        write_volatile(hardware::dash(0), (size >> 8) as u8);
        write_volatile(hardware::MDMAEN, 0x01);
    }
}

/// GP‑DMA: copy `size` bytes from `src` to CGRAM at byte address `cgram_addr`.
/// **Requires force-blank active.**
pub fn dma_copy_cgram(src: *const u8, cgram_addr: u8, size: u16) {
    hardware::write_cgadd(cgram_addr);

    let ptr = src as u16;
    unsafe {
        write_volatile(hardware::dmap(0), 0x00);
        write_volatile(hardware::bbad(0), 0x22);
        write_volatile(hardware::a1tl(0), ptr as u8);
        write_volatile(hardware::a1th(0), (ptr >> 8) as u8);
        write_volatile(hardware::dasl(0), size as u8);
        write_volatile(hardware::dash(0), (size >> 8) as u8);
        write_volatile(hardware::MDMAEN, 0x01);
    }
}

// ---------------------------------------------------------------------------
// CGRAM
// ---------------------------------------------------------------------------

/// Write a 15‑bit BGR colour to CGRAM at palette byte index.
pub fn cgram_set(index: u8, colour: u16) {
    hardware::write_cgadd(index);
    hardware::write_cgdata(colour as u8);
    hardware::write_cgdata((colour >> 8) as u8);
}

// ---------------------------------------------------------------------------
// Scroll reset
// ---------------------------------------------------------------------------

/// Zero all BG scroll registers (each register must be written twice).
pub fn bg_scroll_zero() {
    for _ in 0..2 {
        hardware::write_bg1hofs(0);
        hardware::write_bg1vofs(0);
        hardware::write_bg2hofs(0);
        hardware::write_bg2vofs(0);
        hardware::write_bg3hofs(0);
        hardware::write_bg3vofs(0);
        hardware::write_bg4hofs(0);
        hardware::write_bg4vofs(0);
    }
}
