// Copyright (c) 2026 Matheus C. França
// SPDX-License-Identifier: Apache-2.0
//! SNES hardware register definitions – 8‑bit I/O, bank $00 ($2100–$437F).

use core::ptr::{read_volatile, write_volatile};

const fn reg(addr: u16) -> *mut u8 {
    addr as *mut u8
}

// PPU: Display
pub const INIDISP: *mut u8 = reg(0x2100); // bit7=force-blank, bits3-0=brightness
pub const OBSEL: *mut u8 = reg(0x2101);
pub const OAMADDL: *mut u8 = reg(0x2102);
pub const OAMADDH: *mut u8 = reg(0x2103);
pub const OAMDATA: *mut u8 = reg(0x2104);

// PPU: BG / Mode
pub const BGMODE: *mut u8 = reg(0x2105);
pub const MOSAIC: *mut u8 = reg(0x2106);
pub const BG1SC: *mut u8 = reg(0x2107);
pub const BG2SC: *mut u8 = reg(0x2108);
pub const BG3SC: *mut u8 = reg(0x2109);
pub const BG4SC: *mut u8 = reg(0x210a);
pub const BG12NBA: *mut u8 = reg(0x210b);
pub const BG34NBA: *mut u8 = reg(0x210c);

// PPU: Scroll (write-twice — low then high byte)
pub const BG1HOFS: *mut u8 = reg(0x210d);
pub const BG1VOFS: *mut u8 = reg(0x210e);
pub const BG2HOFS: *mut u8 = reg(0x210f);
pub const BG2VOFS: *mut u8 = reg(0x2110);
pub const BG3HOFS: *mut u8 = reg(0x2111);
pub const BG3VOFS: *mut u8 = reg(0x2112);
pub const BG4HOFS: *mut u8 = reg(0x2113);
pub const BG4VOFS: *mut u8 = reg(0x2114);

// PPU: VRAM
pub const VMAIN: *mut u8 = reg(0x2115);
pub const VMADDL: *mut u8 = reg(0x2116);
pub const VMADDH: *mut u8 = reg(0x2117);
pub const VMDATAL: *mut u8 = reg(0x2118);
pub const VMDATAH: *mut u8 = reg(0x2119);

// PPU: Mode 7
pub const M7SEL: *mut u8 = reg(0x211a);
pub const M7A: *mut u8 = reg(0x211b);
pub const M7B: *mut u8 = reg(0x211c);
pub const M7C: *mut u8 = reg(0x211d);
pub const M7D: *mut u8 = reg(0x211e);
pub const M7X: *mut u8 = reg(0x211f);
pub const M7Y: *mut u8 = reg(0x2120);

// PPU: CGRAM palette
pub const CGADD: *mut u8 = reg(0x2121);
pub const CGDATA: *mut u8 = reg(0x2122);

// PPU: Window masking
pub const W12SEL: *mut u8 = reg(0x2123);
pub const W34SEL: *mut u8 = reg(0x2124);
pub const WOBJSEL: *mut u8 = reg(0x2125);
pub const WH0: *mut u8 = reg(0x2126);
pub const WH1: *mut u8 = reg(0x2127);
pub const WH2: *mut u8 = reg(0x2128);
pub const WH3: *mut u8 = reg(0x2129);
pub const WBGLOG: *mut u8 = reg(0x212a);
pub const WOBJLOG: *mut u8 = reg(0x212b);

// PPU: Layer enable
pub const TM: *mut u8 = reg(0x212c);
pub const TS: *mut u8 = reg(0x212d);
pub const TMW: *mut u8 = reg(0x212e);
pub const TSW: *mut u8 = reg(0x212f);

// PPU: Color math
pub const CGWSEL: *mut u8 = reg(0x2130);
pub const CGADSUB: *mut u8 = reg(0x2131);
pub const COLDATA: *mut u8 = reg(0x2132);

// PPU: Readback
pub const MPYL: *mut u8 = reg(0x2134);
pub const MPYM: *mut u8 = reg(0x2135);
pub const MPYH: *mut u8 = reg(0x2136);
pub const RDNMI: *mut u8 = reg(0x4210);
pub const TIMEUP: *mut u8 = reg(0x4211);
pub const HVBJOY: *mut u8 = reg(0x4212);

// NMI / Timer / IRQ
pub const NMITIMEN: *mut u8 = reg(0x4200);
pub const HTIMEL: *mut u8 = reg(0x4207);
pub const HTIMEH: *mut u8 = reg(0x4208);
pub const VTIMEL: *mut u8 = reg(0x4209);
pub const VTIMEH: *mut u8 = reg(0x420a);

// Joypad
pub const JOYWR: *mut u8 = reg(0x4201);
pub const JOY1L: *mut u8 = reg(0x4218);
pub const JOY1H: *mut u8 = reg(0x4219);
pub const JOY2L: *mut u8 = reg(0x421a);
pub const JOY2H: *mut u8 = reg(0x421b);

// DMA
pub const MDMAEN: *mut u8 = reg(0x420b);
pub const HDMAEN: *mut u8 = reg(0x420c);

pub fn dmap(n: u8) -> *mut u8 {
    reg(0x4300 + n as u16 * 0x10)
}
pub fn bbad(n: u8) -> *mut u8 {
    reg(0x4301 + n as u16 * 0x10)
}
pub fn a1tl(n: u8) -> *mut u8 {
    reg(0x4302 + n as u16 * 0x10)
}
pub fn a1th(n: u8) -> *mut u8 {
    reg(0x4303 + n as u16 * 0x10)
}
pub fn a1b(n: u8) -> *mut u8 {
    reg(0x4304 + n as u16 * 0x10)
}
pub fn dasl(n: u8) -> *mut u8 {
    reg(0x4305 + n as u16 * 0x10)
}
pub fn dash(n: u8) -> *mut u8 {
    reg(0x4306 + n as u16 * 0x10)
}

pub fn write_inidisp(val: u8) {
    unsafe { write_volatile(INIDISP, val) }
}
pub fn read_inidisp() -> u8 {
    unsafe { read_volatile(INIDISP) }
}

pub fn write_obsel(val: u8) {
    unsafe { write_volatile(OBSEL, val) }
}
pub fn read_obsel() -> u8 {
    unsafe { read_volatile(OBSEL) }
}

pub fn write_oamaddl(val: u8) {
    unsafe { write_volatile(OAMADDL, val) }
}
pub fn read_oamaddl() -> u8 {
    unsafe { read_volatile(OAMADDL) }
}

pub fn write_oamaddh(val: u8) {
    unsafe { write_volatile(OAMADDH, val) }
}
pub fn read_oamaddh() -> u8 {
    unsafe { read_volatile(OAMADDH) }
}

pub fn write_oamdata(val: u8) {
    unsafe { write_volatile(OAMDATA, val) }
}
pub fn read_oamdata() -> u8 {
    unsafe { read_volatile(OAMDATA) }
}

pub fn write_bgmode(val: u8) {
    unsafe { write_volatile(BGMODE, val) }
}
pub fn read_bgmode() -> u8 {
    unsafe { read_volatile(BGMODE) }
}

pub fn write_mosaic(val: u8) {
    unsafe { write_volatile(MOSAIC, val) }
}
pub fn read_mosaic() -> u8 {
    unsafe { read_volatile(MOSAIC) }
}

pub fn write_bg1sc(val: u8) {
    unsafe { write_volatile(BG1SC, val) }
}
pub fn read_bg1sc() -> u8 {
    unsafe { read_volatile(BG1SC) }
}

pub fn write_bg2sc(val: u8) {
    unsafe { write_volatile(BG2SC, val) }
}
pub fn read_bg2sc() -> u8 {
    unsafe { read_volatile(BG2SC) }
}

pub fn write_bg3sc(val: u8) {
    unsafe { write_volatile(BG3SC, val) }
}
pub fn read_bg3sc() -> u8 {
    unsafe { read_volatile(BG3SC) }
}

pub fn write_bg4sc(val: u8) {
    unsafe { write_volatile(BG4SC, val) }
}
pub fn read_bg4sc() -> u8 {
    unsafe { read_volatile(BG4SC) }
}

pub fn write_bg12nba(val: u8) {
    unsafe { write_volatile(BG12NBA, val) }
}
pub fn read_bg12nba() -> u8 {
    unsafe { read_volatile(BG12NBA) }
}

pub fn write_bg34nba(val: u8) {
    unsafe { write_volatile(BG34NBA, val) }
}
pub fn read_bg34nba() -> u8 {
    unsafe { read_volatile(BG34NBA) }
}

pub fn write_bg1hofs(val: u8) {
    unsafe { write_volatile(BG1HOFS, val) }
}
pub fn read_bg1hofs() -> u8 {
    unsafe { read_volatile(BG1HOFS) }
}

pub fn write_bg1vofs(val: u8) {
    unsafe { write_volatile(BG1VOFS, val) }
}
pub fn read_bg1vofs() -> u8 {
    unsafe { read_volatile(BG1VOFS) }
}

pub fn write_bg2hofs(val: u8) {
    unsafe { write_volatile(BG2HOFS, val) }
}
pub fn read_bg2hofs() -> u8 {
    unsafe { read_volatile(BG2HOFS) }
}

pub fn write_bg2vofs(val: u8) {
    unsafe { write_volatile(BG2VOFS, val) }
}
pub fn read_bg2vofs() -> u8 {
    unsafe { read_volatile(BG2VOFS) }
}

pub fn write_bg3hofs(val: u8) {
    unsafe { write_volatile(BG3HOFS, val) }
}
pub fn read_bg3hofs() -> u8 {
    unsafe { read_volatile(BG3HOFS) }
}

pub fn write_bg3vofs(val: u8) {
    unsafe { write_volatile(BG3VOFS, val) }
}
pub fn read_bg3vofs() -> u8 {
    unsafe { read_volatile(BG3VOFS) }
}

pub fn write_bg4hofs(val: u8) {
    unsafe { write_volatile(BG4HOFS, val) }
}
pub fn read_bg4hofs() -> u8 {
    unsafe { read_volatile(BG4HOFS) }
}

pub fn write_bg4vofs(val: u8) {
    unsafe { write_volatile(BG4VOFS, val) }
}
pub fn read_bg4vofs() -> u8 {
    unsafe { read_volatile(BG4VOFS) }
}

pub fn write_vmain(val: u8) {
    unsafe { write_volatile(VMAIN, val) }
}
pub fn read_vmain() -> u8 {
    unsafe { read_volatile(VMAIN) }
}

pub fn write_vmaddl(val: u8) {
    unsafe { write_volatile(VMADDL, val) }
}
pub fn read_vmaddl() -> u8 {
    unsafe { read_volatile(VMADDL) }
}

pub fn write_vmaddh(val: u8) {
    unsafe { write_volatile(VMADDH, val) }
}
pub fn read_vmaddh() -> u8 {
    unsafe { read_volatile(VMADDH) }
}

pub fn write_vmdatal(val: u8) {
    unsafe { write_volatile(VMDATAL, val) }
}
pub fn read_vmdatal() -> u8 {
    unsafe { read_volatile(VMDATAL) }
}

pub fn write_vmdatah(val: u8) {
    unsafe { write_volatile(VMDATAH, val) }
}
pub fn read_vmdatah() -> u8 {
    unsafe { read_volatile(VMDATAH) }
}

pub fn write_m7sel(val: u8) {
    unsafe { write_volatile(M7SEL, val) }
}
pub fn read_m7sel() -> u8 {
    unsafe { read_volatile(M7SEL) }
}

pub fn write_m7a(val: u8) {
    unsafe { write_volatile(M7A, val) }
}
pub fn read_m7a() -> u8 {
    unsafe { read_volatile(M7A) }
}

pub fn write_m7b(val: u8) {
    unsafe { write_volatile(M7B, val) }
}
pub fn read_m7b() -> u8 {
    unsafe { read_volatile(M7B) }
}

pub fn write_m7c(val: u8) {
    unsafe { write_volatile(M7C, val) }
}
pub fn read_m7c() -> u8 {
    unsafe { read_volatile(M7C) }
}

pub fn write_m7d(val: u8) {
    unsafe { write_volatile(M7D, val) }
}
pub fn read_m7d() -> u8 {
    unsafe { read_volatile(M7D) }
}

pub fn write_m7x(val: u8) {
    unsafe { write_volatile(M7X, val) }
}
pub fn read_m7x() -> u8 {
    unsafe { read_volatile(M7X) }
}

pub fn write_m7y(val: u8) {
    unsafe { write_volatile(M7Y, val) }
}
pub fn read_m7y() -> u8 {
    unsafe { read_volatile(M7Y) }
}

pub fn write_cgadd(val: u8) {
    unsafe { write_volatile(CGADD, val) }
}
pub fn read_cgadd() -> u8 {
    unsafe { read_volatile(CGADD) }
}

pub fn write_cgdata(val: u8) {
    unsafe { write_volatile(CGDATA, val) }
}
pub fn read_cgdata() -> u8 {
    unsafe { read_volatile(CGDATA) }
}

pub fn write_w12sel(val: u8) {
    unsafe { write_volatile(W12SEL, val) }
}
pub fn read_w12sel() -> u8 {
    unsafe { read_volatile(W12SEL) }
}

pub fn write_w34sel(val: u8) {
    unsafe { write_volatile(W34SEL, val) }
}
pub fn read_w34sel() -> u8 {
    unsafe { read_volatile(W34SEL) }
}

pub fn write_wobjsel(val: u8) {
    unsafe { write_volatile(WOBJSEL, val) }
}
pub fn read_wobjsel() -> u8 {
    unsafe { read_volatile(WOBJSEL) }
}

pub fn write_wh0(val: u8) {
    unsafe { write_volatile(WH0, val) }
}
pub fn read_wh0() -> u8 {
    unsafe { read_volatile(WH0) }
}

pub fn write_wh1(val: u8) {
    unsafe { write_volatile(WH1, val) }
}
pub fn read_wh1() -> u8 {
    unsafe { read_volatile(WH1) }
}

pub fn write_wh2(val: u8) {
    unsafe { write_volatile(WH2, val) }
}
pub fn read_wh2() -> u8 {
    unsafe { read_volatile(WH2) }
}

pub fn write_wh3(val: u8) {
    unsafe { write_volatile(WH3, val) }
}
pub fn read_wh3() -> u8 {
    unsafe { read_volatile(WH3) }
}

pub fn write_wbglog(val: u8) {
    unsafe { write_volatile(WBGLOG, val) }
}
pub fn read_wbglog() -> u8 {
    unsafe { read_volatile(WBGLOG) }
}

pub fn write_wobjlog(val: u8) {
    unsafe { write_volatile(WOBJLOG, val) }
}
pub fn read_wobjlog() -> u8 {
    unsafe { read_volatile(WOBJLOG) }
}

pub fn write_tm(val: u8) {
    unsafe { write_volatile(TM, val) }
}
pub fn read_tm() -> u8 {
    unsafe { read_volatile(TM) }
}

pub fn write_ts(val: u8) {
    unsafe { write_volatile(TS, val) }
}
pub fn read_ts() -> u8 {
    unsafe { read_volatile(TS) }
}

pub fn write_tmw(val: u8) {
    unsafe { write_volatile(TMW, val) }
}
pub fn read_tmw() -> u8 {
    unsafe { read_volatile(TMW) }
}

pub fn write_tsw(val: u8) {
    unsafe { write_volatile(TSW, val) }
}
pub fn read_tsw() -> u8 {
    unsafe { read_volatile(TSW) }
}

pub fn write_cgwsel(val: u8) {
    unsafe { write_volatile(CGWSEL, val) }
}
pub fn read_cgwsel() -> u8 {
    unsafe { read_volatile(CGWSEL) }
}

pub fn write_cgadsub(val: u8) {
    unsafe { write_volatile(CGADSUB, val) }
}
pub fn read_cgadsub() -> u8 {
    unsafe { read_volatile(CGADSUB) }
}

pub fn write_coldata(val: u8) {
    unsafe { write_volatile(COLDATA, val) }
}
pub fn read_coldata() -> u8 {
    unsafe { read_volatile(COLDATA) }
}

pub fn write_mpyl(val: u8) {
    unsafe { write_volatile(MPYL, val) }
}
pub fn read_mpyl() -> u8 {
    unsafe { read_volatile(MPYL) }
}

pub fn write_mpym(val: u8) {
    unsafe { write_volatile(MPYM, val) }
}
pub fn read_mpym() -> u8 {
    unsafe { read_volatile(MPYM) }
}

pub fn write_mpyh(val: u8) {
    unsafe { write_volatile(MPYH, val) }
}
pub fn read_mpyh() -> u8 {
    unsafe { read_volatile(MPYH) }
}

pub fn write_rdnmi(val: u8) {
    unsafe { write_volatile(RDNMI, val) }
}
pub fn read_rdnmi() -> u8 {
    unsafe { read_volatile(RDNMI) }
}

pub fn write_timeup(val: u8) {
    unsafe { write_volatile(TIMEUP, val) }
}
pub fn read_timeup() -> u8 {
    unsafe { read_volatile(TIMEUP) }
}

pub fn write_hvbjoy(val: u8) {
    unsafe { write_volatile(HVBJOY, val) }
}
pub fn read_hvbjoy() -> u8 {
    unsafe { read_volatile(HVBJOY) }
}

pub fn write_nmitimen(val: u8) {
    unsafe { write_volatile(NMITIMEN, val) }
}
pub fn read_nmitimen() -> u8 {
    unsafe { read_volatile(NMITIMEN) }
}

pub fn write_htimel(val: u8) {
    unsafe { write_volatile(HTIMEL, val) }
}
pub fn read_htimel() -> u8 {
    unsafe { read_volatile(HTIMEL) }
}

pub fn write_htimeh(val: u8) {
    unsafe { write_volatile(HTIMEH, val) }
}
pub fn read_htimeh() -> u8 {
    unsafe { read_volatile(HTIMEH) }
}

pub fn write_vtimel(val: u8) {
    unsafe { write_volatile(VTIMEL, val) }
}
pub fn read_vtimel() -> u8 {
    unsafe { read_volatile(VTIMEL) }
}

pub fn write_vtimeh(val: u8) {
    unsafe { write_volatile(VTIMEH, val) }
}
pub fn read_vtimeh() -> u8 {
    unsafe { read_volatile(VTIMEH) }
}

pub fn write_joywr(val: u8) {
    unsafe { write_volatile(JOYWR, val) }
}
pub fn read_joywr() -> u8 {
    unsafe { read_volatile(JOYWR) }
}

pub fn write_joy1l(val: u8) {
    unsafe { write_volatile(JOY1L, val) }
}
pub fn read_joy1l() -> u8 {
    unsafe { read_volatile(JOY1L) }
}

pub fn write_joy1h(val: u8) {
    unsafe { write_volatile(JOY1H, val) }
}
pub fn read_joy1h() -> u8 {
    unsafe { read_volatile(JOY1H) }
}

pub fn write_joy2l(val: u8) {
    unsafe { write_volatile(JOY2L, val) }
}
pub fn read_joy2l() -> u8 {
    unsafe { read_volatile(JOY2L) }
}

pub fn write_joy2h(val: u8) {
    unsafe { write_volatile(JOY2H, val) }
}
pub fn read_joy2h() -> u8 {
    unsafe { read_volatile(JOY2H) }
}

pub fn write_mdmaen(val: u8) {
    unsafe { write_volatile(MDMAEN, val) }
}
pub fn read_mdmaen() -> u8 {
    unsafe { read_volatile(MDMAEN) }
}

pub fn write_hdmaen(val: u8) {
    unsafe { write_volatile(HDMAEN, val) }
}
pub fn read_hdmaen() -> u8 {
    unsafe { read_volatile(HDMAEN) }
}

// ── Color helper ────────────────────────────────────────────────────────
/// Build a 15‑bit BGR colour word (SNES format: `0bbbbbgggggrrrrr`).
pub const fn color(r: u8, g: u8, b: u8) -> u16 {
    (r as u16) | ((g as u16) << 5) | ((b as u16) << 10)
}
