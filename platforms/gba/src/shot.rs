//! Diagnostics: composite the presented frame into SRAM so a host tool can
//! see it.
//!
//! Why this exists: mGBA serves registers and IWRAM/EWRAM over its GDB stub
//! but refuses both VRAM (`0x06000000`) and OAM (`0x07000000`) with `E06`, and
//! it has no command-line screenshot. Judging "do the images show properly" by
//! inference is how a completely broken decoder kept shipping as fine, so this
//! dumps what the hardware is actually presenting.
//!
//! Encoding: the two layers here index into fixed palettes -- BG uses bank 15
//! only (16 colours), OBJ uses the 240-colour card master palette. A sample is
//! therefore exactly 8 bits: `0x00..=0x0F` is a BG index, `0x10 + i` is OBJ
//! index `i`. Slot 0 is backdrop: BG colour 0 is transparent and is never
//! stored, so it cannot collide, whereas OBJ index 239 would have collided
//! with an 0xFF sentinel. That is lossless, so the host rebuilds the true image
//! from the samples plus the two palette dumps.
//!
//! Half resolution (120x80) at 8bpp is 9,600 B per frame, which lets three
//! frames plus both palettes fit the 32 KiB SRAM window.
//!
//! Cost matters: this runs on the GBA's 16.78 MHz ARM7 and the emulator
//! appears to stall while it does. The BG pass is a single pixel-major sweep
//! with an early exit on the front layer, and the OBJ pass walks the 128 OAM
//! entries once and blits each sprite's rectangle -- not 9,600 per-pixel scans
//! of all 128 objects, which was ~1.2M iterations and never finished a frame.
//!
//! Priority: this port runs `ui_back` on P2, card art on P1 and `ui_front` on
//! P0, and sprites are always P1, so a sprite pixel is visible exactly where
//! the P0 layer is transparent. The blit checks that one layer and nothing
//! else, which is why no per-pixel priority map is needed.

const SRAM: usize = 0x0E00_0000;
const REG_RAM_CTRL: usize = 0x0400_0004;
const VRAM: usize = 0x0600_0000;
const OBJ_VRAM: usize = 0x0601_0000;
const OAM: usize = 0x0700_0000;

const OUT_W: usize = 120;
const OUT_H: usize = 80;
const FRAME_BYTES: usize = OUT_W * OUT_H;
const FRAMES: usize = 3;

const HDR: usize = 0x0000;
const FRAME_BASE: usize = 0x0020;
const PAL_BASE: usize = FRAME_BASE + FRAME_BYTES * FRAMES;

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

/// Per-BG state needed to resolve a pixel, read once per frame.
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

/// Colour index this BG shows at a screen pixel, or 0 for transparent.
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

/// Framebuffer of sample values. In .bss (EWRAM), not IWRAM.
static mut FB: [u8; FRAME_BYTES] = [0; FRAME_BYTES];
static mut STATE: u32 = 0;
static mut SLOT: u32 = 0;

fn sram_enable() {
    // REG_RAM_CTRL bit 8 enables the SRAM window; bits 0-1 select 32 KiB.
    unsafe {
        core::ptr::write_volatile(REG_RAM_CTRL as *mut u16, (1 << 8) | 2);
    }
}

/// Capture one frame. Must be called after `GraphicsFrame::commit`, or OAM and
/// the BG tilemaps still hold the previous frame.
///
/// This is expensive on a 16.78 MHz ARM7 -- roughly 40k tile lookups plus the
/// OBJ blits, all through volatile VRAM reads -- so it captures a fixed number
/// of frames and then switches itself off. Leaving it on every N frames makes
/// the game look like it has hung, which is a measurement artefact that
/// completely masks the bug being measured.
///
/// `skip` drops the first N presents before capturing, because the opening
/// frames are still the blank first screen; capturing those tells you nothing.
pub fn maybe_capture(skip: u32, _interval: u32) {
    let shot = unsafe {
        let s = &mut *core::ptr::addr_of_mut!(STATE);
        if *s >= FRAMES as u32 + skip {
            return;
        }
        let n = *s;
        *s = s.wrapping_add(1);
        if n < skip {
            return;
        }
        (n - skip) as usize
    };
    sram_enable();

    let dispcnt = rd16(0x0400_0000);
    let obj_on = (dispcnt >> 12) & 1 == 1;
    let obj_1d = (dispcnt >> 6) & 1 == 1;

    // Front-to-back by priority: highest priority layer first, early exit on
    // an opaque pixel.
    let mut bgs = [read_bg(0, dispcnt), read_bg(1, dispcnt), read_bg(2, dispcnt), read_bg(3, dispcnt)];
    bgs.sort_by(|a, b| a.prio.cmp(&b.prio));

    let fb: &mut [u8; FRAME_BYTES] = unsafe { &mut *core::ptr::addr_of_mut!(FB) };

    for oy in 0..OUT_H {
        let y = oy * 2;
        for ox in 0..OUT_W {
            let x = ox * 2;
            let mut s = 0x00u8;
            for bg in bgs.iter() {
                let ci = bg_index(bg, x, y);
                if ci != 0 {
                    s = ci & 0x0F;
                    break;
                }
            }
            fb[oy * OUT_W + ox] = s;
        }
    }

    // Sprites: OAM 0 is highest priority, so draw 127 -> 0 and let later
    // (higher priority) entries overwrite.
    if obj_on {
        // The P0 layer is the one that can hide sprites.
        let front = bgs.iter().find(|b| b.prio == 0).copied();
        for i in (0..128usize).rev() {
            let o = i * 8;
            let a0 = oam8(o) as u16 | ((oam8(o + 1) as u16) << 8);
            let a1 = oam8(o + 2) as u16 | ((oam8(o + 3) as u16) << 8);
            let a2 = oam8(o + 4) as u16 | ((oam8(o + 5) as u16) << 8);
            if a0 & (1 << 15) == 0 {
                continue; // disabled
            }
            if a0 & (1 << 9) != 0 {
                continue; // affine, unused here
            }
            let eight = (a0 >> 15) & 1 == 1;
            if !eight && a0 & (1 << 14) != 0 {
                continue;
            }
            let mut oy0 = (a0 & 0xFF) as i32;
            if oy0 >= 160 {
                oy0 -= 256;
            }
            let mut ox0 = (a1 & 0x1FF) as i32;
            if ox0 >= 240 {
                ox0 -= 512;
            }
            let (sw, sh) = OBJ_SIZE[((a1 >> 12) & 0xF) as usize];
            if (a2 >> 10) & 3 != 1 {
                continue; // this port only puts card art on P1
            }
            let tile = (a2 & 0x3FF) as usize;
            let hflip = (a1 >> 11) & 1 == 1;
            let vflip = (a1 >> 9) & 1 == 1;
            let tiles_w = sw / 8;

            // Clip to the screen, then to our half-resolution grid.
            let x_lo = ox0.max(0) as usize;
            let y_lo = oy0.max(0) as usize;
            let x_hi = (ox0 + sw as i32).min(240).max(0) as usize;
            let y_hi = (oy0 + sh as i32).min(160).max(0) as usize;

            for y in y_lo..y_hi {
                if y & 1 != 0 {
                    continue; // sample even rows only
                }
                let oy = y >> 1;
                for x in x_lo..x_hi {
                    if x & 1 != 0 {
                        continue; // sample even columns only
                    }
                    let ox = x >> 1;
                    // A sprite shows wherever the P0 layer is transparent.
                    if let Some(front) = front {
                        if bg_index(&front, x, y) != 0 {
                            continue;
                        }
                    }
                    let dx = x as i32 - ox0;
                    let dy = y as i32 - oy0;
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
                        // OAM tile index counts 32-byte units; 8bpp doubles it.
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
                    // Slot 0 is reserved for "backdrop": BG colour 0 is
                    // transparent and never written, so 0 cannot collide with
                    // a real BG sample. OBJ indices run 0..239, giving slots
                    // 16..255 -- using 255 as a sentinel would collide with
                    // OBJ index 239, which is a real palette entry.
                    fb[oy * OUT_W + ox] = 16u8.saturating_add(ci);
                }
            }
        }
        let _ = obj_1d;
    }

    // Push the ring slot out to SRAM.
    let slot = unsafe {
        let s = &mut *core::ptr::addr_of_mut!(SLOT);
        let cur = (*s % FRAMES as u32) as usize;
        *s = s.wrapping_add(1);
        cur
    };
    let base = SRAM + FRAME_BASE + slot * FRAME_BYTES;
    for i in 0..FRAME_BYTES {
        wr8(base + i, fb[i]);
    }

    // Palettes: BG bank 15 (16 colours) then the 256-entry OBJ palette.
    for i in 0..0x200 {
        let v = rd16(0x0500_0000 + i * 2);
        wr8(SRAM + PAL_BASE + i * 2, (v & 0xFF) as u8);
        wr8(SRAM + PAL_BASE + i * 2 + 1, (v >> 8) as u8);
    }

    // Header last, so a torn ring is detectable.
    wr8(SRAM + HDR, b'S');
    wr8(SRAM + HDR + 1, b'H');
    wr8(SRAM + HDR + 2, OUT_W as u8);
    wr8(SRAM + HDR + 3, OUT_H as u8);
    wr8(SRAM + HDR + 4, FRAMES as u8);
    wr8(SRAM + HDR + 5, slot as u8);
    for (i, b) in (shot as u32).to_le_bytes().iter().enumerate() {
        wr8(SRAM + HDR + 8 + i, *b);
    }
}