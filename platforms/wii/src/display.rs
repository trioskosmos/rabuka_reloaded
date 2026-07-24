use alloc::vec::Vec;
use core::ffi::c_void;

// GX constants — all verified against libogc gx.h
const GX_TRUE: u8 = 1;
const GX_FALSE: u8 = 0;
const GX_ORTHOGRAPHIC: u32 = 1;
const GX_TEXMAP0: u8 = 0;
const GX_CLAMP: u8 = 0;
const GX_TF_I4: u8 = 0x0;
const GX_QUADS: u8 = 0x80;
const GX_VTXFMT0: u8 = 0;
const GX_VA_POS: u8 = 9;
const GX_VA_CLR0: u8 = 11;
const GX_VA_TEX0: u8 = 13;
const GX_NONE: u8 = 0;
const GX_DIRECT: u8 = 1;
const GX_POS_XYZ: u8 = 1;
const GX_CLR_RGBA: u8 = 1;
const GX_TEX_ST: u8 = 1;
const GX_F32: u8 = 4;
const GX_U8: u8 = 0;
const GX_RGBA8: u8 = 5;
const GX_CULL_NONE: u8 = 0;
const GX_GM_1_0: u8 = 0;
const GX_TEVSTAGE0: u8 = 0;
const GX_TEXCOORD0: u8 = 0;
const GX_TEXGEN_TEXCOORD: u8 = 0;
const GX_TEXGEN_REGULAR: u8 = 0;
const GX_COLOR0A0: u8 = 0;
const GX_TF_RGB565: u8 = 0x4;
const GX_ZC_LINEAR: u8 = 0;
const GX_PASSCLR: u8 = 4;

extern "C" {
    fn GX_Init(base: *mut c_void, size: u32);
    fn GX_SetViewport(x: f32, y: f32, w: f32, h: f32, near: f32, far: f32);
    fn GX_SetPixelFmt(pix_fmt: u8, z_fmt: u8);
    fn GX_SetZMode(compare_enable: u8, compare_func: u8, update_enable: u8);
    fn GX_SetDispCopyDst(w: u16, h: u16);
    fn GX_SetDispCopyGamma(gamma: u8);
    fn GX_SetCullMode(mode: u8);
    fn GX_LoadProjectionMtx(mtx: *mut f32, proj_type: u32);
    fn GX_CopyDisp(dest: *mut c_void, clear: u8);
    fn GX_DrawDone();
    fn VIDEO_SetNextFramebuffer(fb: *mut c_void);
    fn VIDEO_Flush();
    fn rust_GX_Begin(prim: u8, vtxfmt: u8, n: u16);
    fn rust_GX_Position3f32(x: f32, y: f32, z: f32);
    fn rust_GX_Color4u8(r: u8, g: u8, b: u8, a: u8);
    fn rust_GX_TexCoord2f32(s: f32, t: f32);
    fn rust_GX_End();

    fn GX_LoadTexObj(obj: *mut GXTexObj, map: u8);
    fn GX_InitTexObj(
        obj: *mut GXTexObj,
        img: *mut c_void,
        w: u16,
        h: u16,
        fmt: u8,
        wrap_s: u8,
        wrap_t: u8,
        mipmap: u8,
    );
    fn GX_SetMisc(misc: u32, val: u32);
    fn GX_Flush();
    fn rust_GX_SetNumChans(num: u8);
    fn rust_GX_SetNumTexGens(num: u8);
    fn rust_GX_SetTevOp(tev: u8, mode: u16);
    fn rust_GX_SetTevOrder(tev: u8, coord: u8, map: u8, color: u8);
    fn rust_GX_SetTexCoordGen(coord: u8, typ: u8, src: u8, mtx: u8);
    fn rust_GX_ClearVtxDesc();
    fn rust_GX_SetVtxDesc(attr: u8, enabled: u8);
    fn rust_GX_SetVtxAttrFmt(fmt: u8, attr: u8, comp_cnt: u8, comp_type: u8, shift: u8);
    fn rust_GX_InvalidateTexAll();
    fn SYS_InitFont(font_data: *mut c_void) -> u32;
    fn SYS_GetFontEncoding() -> u32;
    fn SYS_GetFontTexture(c: i32, img: *mut *mut c_void, x: *mut i32, y: *mut i32, w: *mut i32);
    fn SYS_GetFontTexel(c: i32, img: *mut c_void, pos: i32, stride: i32, width: *mut i32);
    fn guOrtho(mtx: *mut f32, t: f32, b: f32, l: f32, r: f32, n: f32, f: f32);
    fn VIDEO_GetNextFramebuffer() -> *mut c_void;
    fn VIDEO_CopyFrameBuffer();
    fn VIDEO_WaitVSync();
}

#[repr(C)]
struct GXTexObj {
    _data: [u8; 0x20],
}

const CELL_W: u32 = 24;
const CELL_H: u32 = 28;
const CHARS_PER_ROW: u32 = 640 / CELL_W;
const ROWS: u32 = 480 / CELL_H;
const CURSOR_X: u32 = 2;
const CURSOR_Y: u32 = 2;
const FIFO_SIZE: usize = 512 * 1024;

pub struct Display {
    fb: *mut c_void,
    cursor_x: u32,
    cursor_y: u32,
    font_data: *mut c_void,
    font_tex: GXTexObj,
    sheet_w: f32,
    sheet_h: f32,
}

impl Display {
    pub fn new() -> Self {
        unsafe {
            // Allocate GX FIFO buffer (512KB to avoid overflow)
            let fifo = malloc(FIFO_SIZE);
            core::ptr::write_bytes(fifo, 0, FIFO_SIZE);
            GX_Init(fifo, FIFO_SIZE as u32);

            GX_SetPixelFmt(GX_TF_RGB565, GX_ZC_LINEAR);
            GX_SetCullMode(GX_CULL_NONE);
            GX_SetZMode(GX_FALSE, GX_FALSE, GX_FALSE);

            // Safer FIFO handling
            GX_SetMisc(1, 1); // GX_MT_XF_FLUSH = 1, GX_XF_FLUSH_SAFE = 1

            GX_SetViewport(0.0, 0.0, 640.0, 480.0, 0.0, 1.0);
            GX_SetDispCopyDst(640, 480);
            GX_SetDispCopyGamma(GX_GM_1_0);
            GX_SetCullMode(GX_CULL_NONE);

            let mut proj: [f32; 16] = [0.0; 16];
            guOrtho(proj.as_mut_ptr(), 0.0, 480.0, 0.0, 640.0, 0.0, 1000.0);
            GX_LoadProjectionMtx(proj.as_mut_ptr(), GX_ORTHOGRAPHIC);

            rust_GX_SetNumChans(1);
            rust_GX_SetNumTexGens(1);
            rust_GX_SetTevOp(GX_TEVSTAGE0, 0); // GX_MODULATE
            rust_GX_SetTevOrder(GX_TEVSTAGE0, GX_TEXCOORD0, GX_TEXMAP0, GX_PASSCLR);
            rust_GX_SetTexCoordGen(GX_TEXCOORD0, GX_TEXGEN_TEXCOORD, GX_TEXGEN_REGULAR, GX_NONE);

            rust_GX_ClearVtxDesc();
            rust_GX_SetVtxDesc(GX_VA_POS, GX_DIRECT);
            rust_GX_SetVtxDesc(GX_VA_CLR0, GX_DIRECT);
            rust_GX_SetVtxDesc(GX_VA_TEX0, GX_DIRECT);
            rust_GX_SetVtxAttrFmt(GX_VTXFMT0, GX_VA_POS, GX_POS_XYZ, GX_F32, 0);
            rust_GX_SetVtxAttrFmt(GX_VTXFMT0, GX_VA_CLR0, GX_CLR_RGBA, 5, 0); // GX_RGBA8=5 for GX_Color4u8
            rust_GX_SetVtxAttrFmt(GX_VTXFMT0, GX_VA_TEX0, GX_TEX_ST, GX_F32, 0);

            // Initial clear: black frame
            let first_fb = VIDEO_GetNextFramebuffer();
            GX_CopyDisp(first_fb, GX_TRUE);
            GX_DrawDone();
            VIDEO_SetNextFramebuffer(first_fb);
            VIDEO_Flush();
            VIDEO_WaitVSync();

            // Load system font (1.15 MB for SJIS)
            let font_data_size = 1183488;
            let font_data = malloc(font_data_size);
            core::ptr::write_bytes(font_data, 0, font_data_size);
            SYS_InitFont(font_data);

            let font_tex_ptr = &*(font_data as *const SysFontHeader);
            let sheet_w = font_tex_ptr.sheet_width as f32;
            let sheet_h = font_tex_ptr.sheet_height as f32;

            let mut font_tex = GXTexObj { _data: [0u8; 0x20] };
            let tex_img = (font_data as usize + font_tex_ptr.sheet_image as usize) as *mut c_void;
            rust_DCFlushRange(tex_img, font_tex_ptr.sheet_fullsize as u32);
            GX_InitTexObj(
                &mut font_tex,
                tex_img,
                font_tex_ptr.sheet_width,
                font_tex_ptr.sheet_height,
                GX_TF_I4,
                GX_CLAMP,
                GX_CLAMP,
                GX_FALSE,
            );
            rust_GX_InvalidateTexAll();

            Display {
                fb: VIDEO_GetNextFramebuffer(),
                cursor_x: CURSOR_X,
                cursor_y: CURSOR_Y,
                font_data,
                font_tex,
                sheet_w,
                sheet_h,
            }
        }
    }

    pub fn clear(&mut self) {
        self.cursor_x = CURSOR_X;
        self.cursor_y = CURSOR_Y;
    }

    pub fn print(&mut self, text: &str) {
        let sjis_bytes = utf8_to_sjis_bytes(text);
        self.render_sjis_bytes(&sjis_bytes);
    }

    pub fn println(&mut self, text: &str) {
        let sjis_bytes = utf8_to_sjis_bytes(text);
        self.render_sjis_bytes(&sjis_bytes);
        self.cursor_x = CURSOR_X;
        self.cursor_y += 1;
        if self.cursor_y >= ROWS {
            self.cursor_y = ROWS - 1;
        }
    }

    pub fn print_raw(&mut self, bytes: &[u8]) {
        self.render_sjis_bytes(bytes);
    }

    pub fn println_raw(&mut self, bytes: &[u8]) {
        self.render_sjis_bytes(bytes);
        self.cursor_x = CURSOR_X;
        self.cursor_y += 1;
        if self.cursor_y >= ROWS {
            self.cursor_y = ROWS - 1;
        }
    }

    pub fn print_card_item_no_idx(&mut self, prefix: &str, name_sjis: &[u8]) {
        self.render_str_sjis(prefix, name_sjis);
        self.cursor_x = CURSOR_X;
        self.cursor_y += 1;
    }

    pub fn swap_buffers(&mut self) {
        unsafe {
            rust_GX_SetNumChans(1);
            rust_GX_SetNumTexGens(1);
            rust_GX_SetTevOp(GX_TEVSTAGE0, 0); // GX_MODULATE
            rust_GX_SetTevOrder(GX_TEVSTAGE0, GX_TEXCOORD0, GX_TEXMAP0, GX_PASSCLR);
            GX_CopyDisp(self.fb, GX_TRUE);
            GX_DrawDone();
            VIDEO_SetNextFramebuffer(self.fb);
            VIDEO_Flush();
            VIDEO_WaitVSync();
            self.fb = VIDEO_GetNextFramebuffer();
        }
    }

    pub fn wait_vsync(&self) {
        unsafe { VIDEO_WaitVSync() }
    }

    pub fn draw_menu(&mut self, items: &[&str], selected: usize, title: &str) {
        self.clear();
        self.println(title);
        self.println("-------------------");
        for (i, item) in items.iter().enumerate() {
            let prefix = if i == selected { " >" } else { "  " };
            self.println(&alloc::format!("{prefix} {item}"));
        }
    }

    fn render_sjis_bytes(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b == b'\n' {
                self.cursor_x = CURSOR_X;
                self.cursor_y += 1;
                i += 1;
                unsafe {
                    GX_Flush();
                }
                continue;
            }
            if b == 0 {
                break;
            }
            let ch: u16;
            if b >= 0x80 {
                if i + 1 < bytes.len() {
                    ch = (b as u16) << 8 | bytes[i + 1] as u16;
                    i += 2;
                } else {
                    break;
                }
            } else {
                ch = b as u16;
                i += 1;
            }
            self.draw_char(ch);
            self.cursor_x += 1;
        }
    }

    fn render_str_sjis(&mut self, prefix: &str, sjis: &[u8]) {
        let mut i = 0;
        for &b in prefix.as_bytes() {
            self.draw_char(b as u16);
            self.cursor_x += 1;
        }
        let mut j = 0;
        while j < sjis.len() {
            let b = sjis[j];
            if b == 0 {
                break;
            }
            let ch: u16;
            if b >= 0x80 {
                if j + 1 < sjis.len() {
                    ch = (b as u16) << 8 | sjis[j + 1] as u16;
                    j += 2;
                } else {
                    break;
                }
            } else {
                ch = b as u16;
                j += 1;
            }
            self.draw_char(ch);
            self.cursor_x += 1;
        }
    }

    fn draw_char(&mut self, ch: u16) {
        unsafe {
            let mut img: *mut c_void = core::ptr::null_mut();
            let mut sx: i32 = 0;
            let mut sy: i32 = 0;
            let mut sw: i32 = 0;
            SYS_GetFontTexture(ch as i32, &mut img, &mut sx, &mut sy, &mut sw);
            if sw == 0 {
                sw = CELL_W as i32;
            }

            GX_LoadTexObj(&mut self.font_tex, GX_TEXMAP0);

            let x = self.cursor_x as f32 * CELL_W as f32;
            let y = self.cursor_y as f32 * CELL_H as f32;
            let w = sw as f32;
            let h = CELL_H as f32;
            let u0 = sx as f32 / self.sheet_w;
            let v0 = sy as f32 / self.sheet_h;
            let u1 = (sx + sw) as f32 / self.sheet_w;
            let v1 = (sy + CELL_H as i32) as f32 / self.sheet_h;

            rust_GX_Begin(GX_QUADS, GX_VTXFMT0, 4);

            rust_GX_Position3f32(x, y, 0.0);
            rust_GX_Color4u8(255, 255, 255, 255);
            rust_GX_TexCoord2f32(u0, v0);

            rust_GX_Position3f32(x + w, y, 0.0);
            rust_GX_Color4u8(255, 255, 255, 255);
            rust_GX_TexCoord2f32(u1, v0);

            rust_GX_Position3f32(x + w, y + h, 0.0);
            rust_GX_Color4u8(255, 255, 255, 255);
            rust_GX_TexCoord2f32(u1, v1);

            rust_GX_Position3f32(x, y + h, 0.0);
            rust_GX_Color4u8(255, 255, 255, 255);
            rust_GX_TexCoord2f32(u0, v1);

            rust_GX_End();
        }
    }
}

#[repr(C)]
struct SysFontHeader {
    sheet_format: u16,
    sheet_column: u16,
    sheet_row: u16,
    sheet_width: u16,
    sheet_height: u16,
    cell_width: u16,
    cell_height: u16,
    sheet_size: u32,
    sheet_fullsize: u32,
    sheet_image: u32,
    width_table: u32,
    inval_char: u32,
    first_char: u32,
    last_char: u32,
}

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn rust_DCFlushRange(addr: *mut c_void, len: u32);
}

fn utf8_to_sjis_bytes(utf8: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let bytes = utf8.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\n' {
            out.push(b'\n');
            i += 1;
            continue;
        }
        let cp = if b < 0x80 {
            i += 1;
            b as u32
        } else if b >= 0xC0 && b < 0xE0 && i + 1 < bytes.len() {
            let cp = ((b & 0x1F) as u32) << 6 | (bytes[i + 1] & 0x3F) as u32;
            i += 2;
            cp
        } else if b >= 0xE0 && b < 0xF0 && i + 2 < bytes.len() {
            let cp = ((b & 0x0F) as u32) << 12
                | ((bytes[i + 1] & 0x3F) as u32) << 6
                | (bytes[i + 2] & 0x3F) as u32;
            i += 3;
            cp
        } else {
            i += 1;
            continue;
        };

        if cp < 0x80 {
            out.push(cp as u8);
        } else if cp >= 0x3000 {
            static SJIS_MAP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sjis_map.bin"));
            let entry_count = SJIS_MAP.len() / 6;
            let mut found = false;
            if entry_count > 0 {
                let mut lo = 0usize;
                let mut hi = entry_count - 1;
                while lo <= hi {
                    let mid = (lo + hi) / 2;
                    let offset = mid * 6;
                    let key = (SJIS_MAP[offset] as u32)
                        | (SJIS_MAP[offset + 1] as u32) << 8
                        | (SJIS_MAP[offset + 2] as u32) << 16
                        | (SJIS_MAP[offset + 3] as u32) << 24;
                    if key == cp {
                        let sjis_hi = SJIS_MAP[offset + 4];
                        let sjis_lo = SJIS_MAP[offset + 5];
                        if sjis_hi != 0 {
                            out.push(sjis_hi);
                        }
                        out.push(sjis_lo);
                        found = true;
                        break;
                    } else if cp < key {
                        hi = mid.saturating_sub(1);
                    } else {
                        lo = mid + 1;
                    }
                }
            }
            if !found {
                out.push(b'?');
            }
        } else {
            // Characters in 0x80-0x2FFF that are ASCII-range but not ASCII
            out.push(b'?');
        }
    }
    out
}
