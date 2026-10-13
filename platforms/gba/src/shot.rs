//! Diagnostics: dump what the GBA is actually presenting, into SRAM.
//!
//! Why: mGBA serves registers and IWRAM/EWRAM over its GDB stub but refuses
//! VRAM (`0x06000000`) and OAM (`0x07000000`) with `E06`, and has no CLI
//! screenshot. Judging "do the images show" by inference is how a completely
//! broken decoder shipped as fine for hours, so this captures the real thing.
//!
//! Format: two full-resolution 240x160 coverage bitmaps (BG and
//! OBJ), one bit per pixel = 4,800 bytes each, plus the BG palette
//! bank. Two slots (an early menu and the frame after it) fit the
//! 32 KiB SRAM window: 2*4800 (BG) + 0x220 (palette) + 2*4800
//! (OBJ) ~= 19.3 KiB.
//!   bit 0   backdrop (every layer transparent at that pixel)
//!   bit 1   the BG layer is opaque there (text/chrome/cards)
//!   OBJ slot set  a sprite covered this pixel (extent + layout)
//!
//! Full resolution is the whole point. An earlier version sampled
//! every second pixel to fit 8bpp, which turned 1px font strokes
//! into speckle and made a working screen look like heavy glitch --
//! the diagnostic was the artefact. The BG layer is where text and
//! chrome live and where corruption reads most clearly, so it is
//! captured exactly; sprites are marked rather than colour-coded,
//! which still shows their extent and layout.

const SRAM: usize = 0x0E00_0000;
const REG_RAM_CTRL: usize = 0x0400_0004;
const VRAM: usize = 0x0600_0000;
const OBJ_VRAM: usize = 0x0601_0000;
const OAM: usize = 0x0700_0000;

const OUT_W: usize = 240;
const OUT_H: usize = 160;
/// One bit per pixel (coverage only): 240*160 = 38,400 bits =
/// 4,800 bytes. `BGBITS`/`sprite_bmp` are exactly this size, so the
/// copy loop below stays in bounds.
const FRAME_BYTES: usize = OUT_W * OUT_H / 8;

const HDR: usize = 0x0000;
const FRAME_BASE: usize = 0x0020;
/// After BOTH BG slots, so slot 1 does not collide with the palettes
/// (the header records two frames).
const PAL_BASE: usize = FRAME_BASE + 2 * FRAME_BYTES;
const OBJ_BASE: usize = PAL_BASE + 0x220;

#[inline]
fn rd16(addr: usize) -> u16 {
    unsafe { core::ptr::read_volatile(addr as *const u16) }
}

#[inline]
fn wr8(addr: usize, v: u8) {
    unsafe { core::ptr::write_volatile(addr as *mut u8, v) }
}

#[inline]
fn vram8(a: usize) -> u8 {
    if (VRAM..VRAM + 0x18000).contains(&a) {
        unsafe { core::ptr::read_volatile(a as *const u8) }
    } else {
        0
    }
}

#[inline]
fn oam8(i: usize) -> u8 {
    if i < 1024 {
        unsafe { core::ptr::read_volatile((OAM + i) as *const u8) }
    } else {
        0
    }
}

const BG_SIZE: [(usize, usize); 4] = [(32, 32), (64, 32), (32, 64), (64, 64)];
const OBJ_SIZE: [(usize, usize); 16] = [
    (8, 8), (16, 8), (32, 8), (64, 8),
    (8, 16), (16, 16), (32, 16), (64, 16),
    (8, 32), (16, 32), (32, 32), (64, 32),
    (8, 64), (16, 64), (32, 64), (64, 64),
];

#[derive(Clone, Copy)]
struct Bg {
    on: bool,
    prio: u8,
    charbase: usize,
    eight: bool,
    screenbase: usize,
    mw: usize,
    mh: usize,
    ho: usize,
    vo: usize,
}

fn read_bg(n: usize, dispcnt: u16) -> Bg {
    let cnt = rd16(0x0400_0008 + n * 2);
    let (mw, mh) = BG_SIZE[((cnt >> 14) & 3) as usize];
    Bg {
        on: ((dispcnt >> (8 + n)) & 1) == 1,
        prio: (cnt & 3) as u8,
        charbase: (((cnt >> 2) & 3) as usize) << 14,
        eight: (cnt >> 7) & 1 == 1,
        screenbase: (((cnt >> 8) & 0x1F) as usize) << 11,
        mw,
        mh,
        ho: rd16(0x0400_0010 + n * 4) as usize,
        vo: rd16(0x0400_0012 + n * 4) as usize,
    }
}

/// Colour index this BG shows at a screen pixel, 0 = transparent.
fn bg_index(bg: &Bg, x: usize, y: usize) -> u8 {
    if !bg.on {
        return 0;
    }
    let px = (x + bg.ho) % (bg.mw * 8);
    let py = (y + bg.vo) % (bg.mh * 8);
    let map_off = bg.screenbase + ((py / 8) * bg.mw + (px / 8)) * 2;
    if map_off + 1 >= 0x18000 {
        return 0;
    }
    let entry = vram8(VRAM + map_off) as u16 | ((vram8(VRAM + map_off + 1) as u16) << 8);
    let tile = (entry & 0x3FF) as usize;
    let mut ix = px % 8;
    let mut iy = py % 8;
    if (entry >> 10) & 1 == 1 {
        ix = 7 - ix;
    }
    if (entry >> 11) & 1 == 1 {
        iy = 7 - iy;
    }
    if bg.eight {
        vram8(VRAM + bg.charbase + tile * 64 + iy * 8 + ix)
    } else {
        let b = vram8(VRAM + bg.charbase + tile * 32 + iy * 4 + (ix >> 1));
        if ix & 1 == 1 {
            b >> 4
        } else {
            b & 0xF
        }
    }
}

/// Bit-per-pixel BG index, 240*160 = 38,400 bits = 4,800 bytes.
static mut BGBITS: [u8; 4800] = [0; 4800];
/// OBJ coverage twin of [`BGBITS`], kept as a static so the host
/// can read it back over the GDB stub: EWRAM is served live,
/// SRAM is not. A stack local would die with `maybe_capture`'s
/// frame and leave the sprite extent unobservable.
static mut SPRITEBITS: [u8; 4800] = [0; 4800];
static mut STATE: u32 = 0;

fn sram_enable() {
    unsafe {
        core::ptr::write_volatile(REG_RAM_CTRL as *mut u16, (1 << 8) | 2);
    }
}

/// Capture one frame. Must run after `GraphicsFrame::commit`, or OAM and the
/// BG tilemaps still hold the previous frame.
///
/// `skip` drops the first N presents, because the opening frames are the blank
/// first screen. This is expensive (one volatile tile lookup per pixel, plus
/// the OBJ blits) so it captures a bounded number of frames and then disables
/// itself; leaving it on makes the game look hung, which masks the bug.
pub fn maybe_capture(skip: u32) {
    let shot = unsafe {
        let s = &mut *core::ptr::addr_of_mut!(STATE);
        // Two frames: an early menu and whatever came after it.
        if *s >= skip + 2 {
            return;
        }
        let n = *s;
        *s = s.wrapping_add(1);
        if n < skip {
            return;
        }
        n
    };
    sram_enable();

    let dispcnt = rd16(0x0400_0000);
    let obj_on = (dispcnt >> 12) & 1 == 1;

    let mut bgs = [
        read_bg(0, dispcnt),
        read_bg(1, dispcnt),
        read_bg(2, dispcnt),
        read_bg(3, dispcnt),
    ];
    bgs.sort_by(|a, b| a.prio.cmp(&b.prio));

    let bits: &mut [u8; 4800] = unsafe { &mut *core::ptr::addr_of_mut!(BGBITS) };
    for i in bits.iter_mut() {
        *i = 0;
    }

    // BG pass: frontmost opaque layer wins, index stored exactly.
    for y in 0..OUT_H {
        for x in 0..OUT_W {
            let mut idx = 0u8;
            for bg in bgs.iter() {
                let ci = bg_index(bg, x, y);
                if ci != 0 {
                    idx = ci;
                    break;
                }
            }
            if idx != 0 {
                let b = y * OUT_W + x;
                bits[b >> 3] |= 1 << (b & 7);
            }
        }
    }

    // OBJ pass: mark coverage. A sprite is visible where the P0 layer is
    // transparent; this port puts card art on P1.
    let sprite_bmp: &mut [u8; 4800] =
        unsafe { &mut *core::ptr::addr_of_mut!(SPRITEBITS) };
    for i in sprite_bmp.iter_mut() {
        *i = 0;
    }
    if obj_on {
        let front = bgs.iter().find(|b| b.prio == 0).copied();
        for i in (0..128usize).rev() {
            let o = i * 8;
            let a0 = oam8(o) as u16 | ((oam8(o + 1) as u16) << 8);
            let a1 = oam8(o + 2) as u16 | ((oam8(o + 3) as u16) << 8);
            let a2 = oam8(o + 4) as u16 | ((oam8(o + 5) as u16) << 8);
            if a0 & (1 << 15) == 0 || a0 & (1 << 9) != 0 {
                continue;
            }
            let eight = (a0 >> 15) & 1 == 1;
            if !eight && a0 & (1 << 14) != 0 {
                continue;
            }
            if (a2 >> 10) & 3 != 1 {
                continue;
            }
            let mut oy = (a0 & 0xFF) as i32;
            if oy >= 160 {
                oy -= 256;
            }
            let mut ox = (a1 & 0x1FF) as i32;
            if ox >= 240 {
                ox -= 512;
            }
            let (sw, sh) = OBJ_SIZE[((a1 >> 12) & 0xF) as usize];
            let tile = (a2 & 0x3FF) as usize;
            let hflip = (a1 >> 11) & 1 == 1;
            let vflip = (a1 >> 9) & 1 == 1;
            let tiles_w = sw / 8;

            let x_lo = ox.max(0) as usize;
            let y_lo = oy.max(0) as usize;
            let x_hi = (ox + sw as i32).min(240).max(0) as usize;
            let y_hi = (oy + sh as i32).min(160).max(0) as usize;
            for y in y_lo..y_hi {
                for x in x_lo..x_hi {
                    if let Some(front) = front {
                        if bg_index(&front, x, y) != 0 {
                            continue;
                        }
                    }
                    let dx = x as i32 - ox;
                    let dy = y as i32 - oy;
                    let mut sx = dx as usize;
                    let mut sy = dy as usize;
                    if hflip {
                        sx = sw - 1 - sx;
                    }
                    if vflip {
                        sy = sh - 1 - sy;
                    }
                    let slot = (sy / 8) * tiles_w + (sx / 8);
                    let ci = if eight {
                        vram8(OBJ_VRAM + tile * 64 + slot * 64 + (sy % 8) * 8 + (sx % 8))
                    } else {
                        let b = vram8(OBJ_VRAM + tile * 32 + slot * 32 + (sy % 8) * 4 + (sx % 8) / 2);
                        if sx % 8 & 1 == 1 {
                            b >> 4
                        } else {
                            b & 0xF
                        }
                    };
                    if ci == 0 {
                        continue;
                    }
                    let p = y * OUT_W + x;
                    sprite_bmp[p >> 3] |= 1 << (p & 7);
                }
            }
        }
    }

    let slot = (shot - skip) as usize;
    let base = SRAM + FRAME_BASE + slot * FRAME_BYTES;
    for i in 0..FRAME_BYTES {
        wr8(base + i, bits[i]);
    }
    let sbase = SRAM + OBJ_BASE + slot * FRAME_BYTES;
    for i in 0..FRAME_BYTES {
        wr8(sbase + i, sprite_bmp[i]);
    }

    // Palettes so the host can colour the BG indices exactly.
    for i in 0..0x100 {
        let v = rd16(0x0500_0000 + i * 2);
        wr8(SRAM + PAL_BASE + i * 2, (v & 0xFF) as u8);
        wr8(SRAM + PAL_BASE + i * 2 + 1, (v >> 8) as u8);
    }

    wr8(SRAM + HDR, b'S');
    wr8(SRAM + HDR + 1, b'F');
    wr8(SRAM + HDR + 2, OUT_W as u8);
    wr8(SRAM + HDR + 3, OUT_H as u8);
    wr8(SRAM + HDR + 4, 2);
    wr8(SRAM + HDR + 5, slot as u8);
    for (i, b) in shot.to_le_bytes().iter().enumerate() {
        wr8(SRAM + HDR + 8 + i, *b);
    }
}