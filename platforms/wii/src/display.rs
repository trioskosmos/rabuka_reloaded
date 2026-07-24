use alloc::vec::Vec;
use core::ffi::c_void;

// GX constants — ALL VERIFIED against libogc ogc/gx.h
const GX_TRUE: u8 = 1;
const GX_FALSE: u8 = 0;
const GX_ENABLE: u8 = 1;
const GX_DIRECT: u8 = 1;
const GX_NONE: u8 = 0;
const GX_VA_POS: u8 = 9;
const GX_VA_CLR0: u8 = 11;
const GX_VA_TEX0: u8 = 13;
const GX_POS_XYZ: u8 = 1;
const GX_CLR_RGBA: u8 = 1;
const GX_TEX_ST: u8 = 1;
const GX_F32: u8 = 4;
const GX_RGBA8: u8 = 5;
const GX_VTXFMT0: u8 = 0;
const GX_QUADS: u8 = 0x80;
const GX_TEXMAP0: u8 = 0;
const GX_CLAMP: u8 = 0;
const GX_TF_I4: u8 = 0;
const GX_TEVSTAGE0: u8 = 0;
const GX_TEXCOORD0: u8 = 0;
const GX_PNMTX0: u8 = 0;
const GX_ORTHOGRAPHIC: u32 = 1;
const GX_PF_RGB8_Z24: u8 = 0;
const GX_ZC_LINEAR: u8 = 0;
const GX_LEQUAL: u8 = 3;
const GX_GM_1_0: u8 = 0;
const GX_CULL_NONE: u8 = 0;
const GX_MODULATE: u8 = 0; // index for GX_SetTevOp
const GX_PASSCLR: u8 = 4; // index for GX_SetTevOp
const GX_MAX_Z24: u32 = 0x00FFFFFF;
const GX_BM_BLEND: u8 = 1;
const GX_BL_SRCALPHA: u8 = 4;
const GX_BL_INVSRCALPHA: u8 = 5;
const GX_LO_CLEAR: u8 = 0;
const GX_TG_MTX2x4: u8 = 1;
const GX_TG_TEX0: u8 = 4;
const GX_IDENTITY: u8 = 60;
const GX_GREATER: u8 = 4;
const GX_AOP_AND: u8 = 0;
const GX_ALWAYS: u8 = 7;

extern "C" {
    // Exported from entry.c
    static xfb: [*mut c_void; 2];
    static mut fb_idx: i32;
    static fbWidth: u16;
    static efbHeight: u16;
    static xfbHeight: u16;
    static aa: u8;
    static field_rendering: u8;
    static viTVMode: u8;

    // libogc functions
    fn GX_Init(base: *mut c_void, size: u32);
    fn GX_SetCopyClear(clear_color: *mut c_void, clear_z: u32);
    fn GX_SetPixelFmt(pix_fmt: u8, z_fmt: u8);
    fn GX_GetYScaleFactor(efb_h: u32, xfb_h: u32) -> f32;
    fn GX_SetDispCopyYScale(yscale: f32) -> u32;
    fn GX_SetDispCopySrc(x: u32, y: u32, w: u32, h: u32);
    fn GX_SetDispCopyDst(w: u32, h: u32);
    fn GX_SetCopyFilter(aa: u8, sample_pattern: *mut c_void, vf: u8, vfilter: *mut c_void);
    fn GX_SetFieldMode(rd: u8, mode: u8);
    fn GX_SetDispCopyGamma(gamma: u8);
    fn GX_ClearVtxDesc();
    fn GX_InvVtxCache();
    fn GX_InvalidateTexAll();
    fn GX_SetVtxDesc(attr: u8, enabled: u8);
    fn GX_SetVtxAttrFmt(fmt: u8, attr: u8, cnt: u8, typ: u8, shift: u8);
    fn GX_SetZMode(compare_enable: u8, compare_func: u8, update_enable: u8);
    fn GX_SetNumChans(num: u8);
    fn GX_SetNumTexGens(num: u8);
    fn GX_SetTevOp(tev: u8, mode: u8);
    fn GX_SetTevOrder(tev: u8, coord: u8, map: u8, color: u8);
    fn GX_SetTexCoordGen(coord: u8, typ: u8, src: u8, mtx: u8);
    fn GX_SetBlendMode(mode: u8, src: u8, dst: u8, logic: u8);
    fn GX_SetAlphaUpdate(enable: u8);
    fn GX_SetAlphaCompare(comp0: u8, ref0: u8, op: u8, comp1: u8, ref1: u8);
    fn GX_SetColorUpdate(enable: u8);
    fn GX_SetCullMode(mode: u8);
    fn GX_LoadPosMtxImm(mtx: *mut f32, id: u8);
    fn GX_LoadProjectionMtx(mtx: *mut f32, proj_type: u32);
    fn GX_SetViewport(x: f32, y: f32, w: f32, h: f32, near: f32, far: f32);
    fn GX_CopyDisp(dest: *mut c_void, clear: u8);
    fn GX_DrawDone();
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
    fn GX_Flush();
    fn c_guMtxIdentity(mtx: *mut f32);
    fn c_guMtxTransApply(mtx: *mut f32, dst: *mut f32, x: f32, y: f32, z: f32);
    fn guOrtho(mtx: *mut f32, t: f32, b: f32, l: f32, r: f32, n: f32, f: f32);
    fn SYS_InitFont(font_data: *mut c_void) -> u32;
    fn SYS_GetFontEncoding() -> u32;
    fn SYS_GetFontTexture(c: i32, img: *mut *mut c_void, x: *mut i32, y: *mut i32, w: *mut i32);
    fn VIDEO_WaitVSync();
    fn malloc(size: usize) -> *mut c_void;

    // Rust wrapper functions from entry.c
    fn rust_GX_Begin(prim: u8, vtxfmt: u8, n: u16);
    fn rust_GX_Position3f32(x: f32, y: f32, z: f32);
    fn rust_GX_Color1u32(clr: u32);
    fn rust_GX_TexCoord2f32(s: f32, t: f32);
    fn rust_DCFlushRange(addr: *mut c_void, len: u32);
}

#[repr(C)]
struct GXTexObj {
    _data: [u8; 0x20],
}

#[repr(C)]
struct SysFontHeader {
    font_type: u16,
    first_char: u16,
    last_char: u16,
    inval_char: u16,
    asc: u16,
    desc: u16,
    width: u16,
    leading: u16,
    cell_width: u16,
    cell_height: u16,
    sheet_size: u32,
    sheet_format: u16,
    sheet_column: u16,
    sheet_row: u16,
    sheet_width: u16,
    sheet_height: u16,
    width_table: u32,
    sheet_image: u32,
    sheet_fullsize: u32,
}

const CELL_W: f32 = 24.0;
const CELL_H: f32 = 28.0;
const CURSOR_X: i32 = 2;
const CURSOR_Y: i32 = 2;
const FONT_SIZE: f32 = 28.0;
const SYS_FONTSIZE_SJIS: usize = 1183488;

pub struct Display {
    cursor_x: i32,
    cursor_y: i32,
    font_data: *mut c_void,
    font_sheet_img: *mut c_void,
    sheet_w: f32,
    sheet_h: f32,
    cell_w: f32,
    cell_h: f32,
}

impl Display {
    pub fn new() -> Self {
        unsafe {
            // GX FIFO
            let fifo = malloc(256 * 1024);
            core::ptr::write_bytes(fifo, 0, 256 * 1024);
            GX_Init(fifo, 256 * 1024);

            let w = fbWidth as u32;
            let h = efbHeight as u32;

            GX_SetCopyClear(b"0000\0" as *const _ as *mut c_void, GX_MAX_Z24);
            GX_SetPixelFmt(GX_PF_RGB8_Z24, GX_ZC_LINEAR);

            // Using hardcoded 640x480 for NTSC
            let yscale = GX_GetYScaleFactor(h, h);
            let xfb_h = GX_SetDispCopyYScale(yscale);
            GX_SetDispCopySrc(0, 0, w, h);
            GX_SetDispCopyDst(w, xfb_h as u32);
            GX_SetCopyFilter(aa, core::ptr::null_mut(), GX_TRUE, core::ptr::null_mut());
            GX_SetFieldMode(
                field_rendering,
                if viTVMode & 1 != 0 {
                    GX_ENABLE
                } else {
                    GX_FALSE
                },
            );
            GX_SetDispCopyGamma(GX_GM_1_0);

            // Vertex descriptor
            GX_ClearVtxDesc();
            GX_InvVtxCache();
            GX_InvalidateTexAll();
            GX_SetVtxDesc(GX_VA_TEX0, GX_NONE);
            GX_SetVtxDesc(GX_VA_POS, GX_DIRECT);
            GX_SetVtxDesc(GX_VA_CLR0, GX_DIRECT);
            GX_SetVtxAttrFmt(GX_VTXFMT0, GX_VA_POS, GX_POS_XYZ, GX_F32, 0);
            GX_SetVtxAttrFmt(GX_VTXFMT0, GX_VA_TEX0, GX_TEX_ST, GX_F32, 0);
            GX_SetVtxAttrFmt(GX_VTXFMT0, GX_VA_CLR0, GX_CLR_RGBA, GX_RGBA8, 0);
            GX_SetZMode(GX_FALSE, GX_LEQUAL, GX_TRUE);

            // TEV
            GX_SetNumChans(1);
            GX_SetNumTexGens(1);
            GX_SetTevOp(GX_TEVSTAGE0, GX_PASSCLR);
            GX_SetTevOrder(GX_TEVSTAGE0, GX_TEXCOORD0, GX_TEXMAP0, GX_NONE);
            GX_SetTexCoordGen(GX_TEXCOORD0, GX_TG_MTX2x4, GX_TG_TEX0, GX_IDENTITY);

            // Ortho projection
            let mut model: [f32; 12] = [0.0; 12];
            c_guMtxIdentity(model.as_mut_ptr());
            c_guMtxTransApply(model.as_mut_ptr(), model.as_mut_ptr(), 0.0, 0.0, -100.0);
            GX_LoadPosMtxImm(model.as_mut_ptr(), GX_PNMTX0);

            let mut proj: [f32; 16] = [0.0; 16];
            guOrtho(proj.as_mut_ptr(), 0.0, 480.0, 0.0, 640.0, 0.0, 1000.0);
            GX_LoadProjectionMtx(proj.as_mut_ptr(), GX_ORTHOGRAPHIC);
            GX_SetViewport(0.0, 0.0, 640.0, 480.0, 0.0, 1.0);

            // Blending
            GX_SetBlendMode(GX_BM_BLEND, GX_BL_SRCALPHA, GX_BL_INVSRCALPHA, GX_LO_CLEAR);
            GX_SetAlphaUpdate(GX_TRUE);
            GX_SetAlphaCompare(GX_GREATER, 0, GX_AOP_AND, GX_ALWAYS, 0);
            GX_SetColorUpdate(GX_ENABLE);
            GX_SetCullMode(GX_CULL_NONE);

            VIDEO_WaitVSync();

            // System font
            let font_data = malloc(SYS_FONTSIZE_SJIS);
            core::ptr::write_bytes(font_data, 0, SYS_FONTSIZE_SJIS);
            SYS_InitFont(font_data);

            let hdr = &*(font_data as *const SysFontHeader);
            let sheet_img = (font_data as usize + hdr.sheet_image as usize) as *mut c_void;
            rust_DCFlushRange(sheet_img, hdr.sheet_fullsize);

            Display {
                cursor_x: CURSOR_X,
                cursor_y: CURSOR_Y,
                font_data,
                font_sheet_img: sheet_img,
                sheet_w: hdr.sheet_width as f32,
                sheet_h: hdr.sheet_height as f32,
                cell_w: hdr.cell_width as f32,
                cell_h: hdr.cell_height as f32,
            }
        }
    }

    pub fn clear(&mut self) {
        self.cursor_x = CURSOR_X;
        self.cursor_y = CURSOR_Y;
    }

    pub fn print(&mut self, text: &str) {
        let sjis = utf8_to_sjis_bytes(text);
        self.render_sjis(&sjis);
    }

    pub fn println(&mut self, text: &str) {
        let sjis = utf8_to_sjis_bytes(text);
        self.render_sjis(&sjis);
        self.cursor_x = CURSOR_X;
        self.cursor_y += 1;
    }

    pub fn print_raw(&mut self, bytes: &[u8]) {
        self.render_sjis(bytes);
    }

    pub fn println_raw(&mut self, bytes: &[u8]) {
        self.render_sjis(bytes);
        self.cursor_x = CURSOR_X;
        self.cursor_y += 1;
    }

    pub fn swap_buffers(&mut self) {
        unsafe {
            GX_DrawDone();
            GX_InvalidateTexAll();
            let fb = if fb_idx == 0 { 0 } else { 1 };
            fb_idx = 1 - fb_idx;
            GX_SetZMode(GX_TRUE, GX_LEQUAL, GX_TRUE);
            GX_SetColorUpdate(GX_ENABLE);
            GX_CopyDisp(xfb[fb as usize], GX_TRUE);
            GX_DrawDone();
            VIDEO_WaitVSync();
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

    pub fn print_card_item_no_idx(&mut self, _prefix: &str, _name_sjis: &[u8]) {
        // Fallback: render as text (card_name_sjis not available here)
    }

    fn render_sjis(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b == 0 {
                break;
            }
            if b == b'\n' {
                self.cursor_x = CURSOR_X;
                self.cursor_y += 1;
                i += 1;
                continue;
            }
            let ch: i32;
            if b >= 0x80 && i + 1 < bytes.len() {
                ch = ((b as i32) << 8) | bytes[i + 1] as i32;
                i += 2;
            } else {
                ch = b as i32;
                i += 1;
            }
            self.draw_char(ch);
            self.cursor_x += 1;
        }
    }

    fn draw_char(&self, ch: i32) {
        unsafe {
            let mut img: *mut c_void = core::ptr::null_mut();
            let mut sx: i32 = 0;
            let mut sy: i32 = 0;
            let mut sw: i32 = 0;
            SYS_GetFontTexture(ch, &mut img, &mut sx, &mut sy, &mut sw);
            if img.is_null() {
                return;
            }
            if sw == 0 {
                sw = self.cell_w as i32;
            }

            let mut texobj = GXTexObj { _data: [0u8; 0x20] };
            GX_InitTexObj(
                &mut texobj,
                self.font_sheet_img,
                self.sheet_w as u16,
                self.sheet_h as u16,
                GX_TF_I4,
                GX_CLAMP,
                GX_CLAMP,
                GX_FALSE,
            );
            GX_LoadTexObj(&mut texobj, GX_TEXMAP0);

            GX_SetTevOp(GX_TEVSTAGE0, GX_MODULATE);
            GX_SetVtxDesc(GX_VA_TEX0, GX_DIRECT);

            let x = self.cursor_x as f32 * self.cell_w;
            let y = self.cursor_y as f32 * self.cell_h;
            let x2 = x + sw as f32 * FONT_SIZE / self.cell_h;
            let y2 = y + FONT_SIZE;
            let s1 = sx as f32 / self.sheet_w;
            let t1 = sy as f32 / self.sheet_h;
            let s2 = (sx + self.cell_w as i32) as f32 / self.sheet_w;
            let t2 = (sy + self.cell_h as i32) as f32 / self.sheet_h;

            rust_GX_Begin(GX_QUADS, GX_VTXFMT0, 4);
            // Vertex 0
            rust_GX_Position3f32(x, y, 0.0);
            rust_GX_Color1u32(0xFFFFFFFF);
            rust_GX_TexCoord2f32(s1, t1);
            // Vertex 1
            rust_GX_Position3f32(x2, y, 0.0);
            rust_GX_Color1u32(0xFFFFFFFF);
            rust_GX_TexCoord2f32(s2, t1);
            // Vertex 2
            rust_GX_Position3f32(x2, y2, 0.0);
            rust_GX_Color1u32(0xFFFFFFFF);
            rust_GX_TexCoord2f32(s2, t2);
            // Vertex 3
            rust_GX_Position3f32(x, y2, 0.0);
            rust_GX_Color1u32(0xFFFFFFFF);
            rust_GX_TexCoord2f32(s1, t2);
            GX_Flush();

            GX_SetTevOp(GX_TEVSTAGE0, GX_PASSCLR);
            GX_SetVtxDesc(GX_VA_TEX0, GX_NONE);
        }
    }
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
        } else {
            static MAP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sjis_map.bin"));
            let n = MAP.len() / 6;
            let mut found = false;
            if n > 0 {
                let mut lo = 0usize;
                let mut hi = n - 1;
                while lo <= hi {
                    let mid = (lo + hi) / 2;
                    let off = mid * 6;
                    let key = (MAP[off] as u32)
                        | (MAP[off + 1] as u32) << 8
                        | (MAP[off + 2] as u32) << 16
                        | (MAP[off + 3] as u32) << 24;
                    if key == cp {
                        let hi_b = MAP[off + 4];
                        let lo_b = MAP[off + 5];
                        if hi_b != 0 {
                            out.push(hi_b);
                        }
                        out.push(lo_b);
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
        }
    }
    out
}
