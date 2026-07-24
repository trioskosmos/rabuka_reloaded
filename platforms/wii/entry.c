#include <gccore.h>
#include <ogc/gx.h>
#include <stdlib.h>

extern void rabuka_main(void);

static void *xfb[2] = {NULL, NULL};
static int which_fb = 0;

int main(int argc, char **argv) {
    VIDEO_Init();
    PAD_Init();

    GXRModeObj *rmode = VIDEO_GetPreferredMode(NULL);
    xfb[0] = MEM_K0_TO_K1(SYS_AllocateFramebuffer(rmode));
    xfb[1] = MEM_K0_TO_K1(SYS_AllocateFramebuffer(rmode));
    VIDEO_Configure(rmode);
    VIDEO_SetNextFramebuffer(xfb[0]);
    VIDEO_SetBlack(false);
    VIDEO_Flush();
    VIDEO_WaitVSync();
    if (rmode->viTVMode & VI_NON_INTERLACE)
        VIDEO_WaitVSync();

    rabuka_main();
    return 0;
}

// C wrappers for GX inline functions so Rust can call them via FFI
void rust_GX_Begin(u8 prim, u8 vtxfmt, u16 n) { GX_Begin(prim, vtxfmt, n); }
void rust_GX_End(void) { GX_End(); }
void rust_GX_Position3f32(f32 x, f32 y, f32 z) { GX_Position3f32(x, y, z); }
void rust_GX_Color4u8(u8 r, u8 g, u8 b, u8 a) { GX_Color4u8(r, g, b, a); }
void rust_GX_TexCoord2f32(f32 s, f32 t) { GX_TexCoord2f32(s, t); }
void rust_GX_ClearVtxDesc(void) { GX_ClearVtxDesc(); }
void rust_GX_SetVtxDesc(u8 attr, u8 enabled) { GX_SetVtxDesc(attr, enabled); }
void rust_GX_SetVtxAttrFmt(u8 fmt, u8 attr, u8 cnt, u8 type, u8 shift) { GX_SetVtxAttrFmt(fmt, attr, cnt, type, shift); }
void rust_GX_InvalidateTexAll(void) { GX_InvalidateTexAll(); }
void rust_GX_SetNumChans(u8 num) { GX_SetNumChans(num); }
void rust_GX_SetNumTexGens(u8 num) { GX_SetNumTexGens(num); }
void rust_GX_SetTevOp(u8 tev, u16 mode) { GX_SetTevOp(tev, mode); }
void rust_GX_SetTevOrder(u8 tev, u8 coord, u8 map, u8 color) { GX_SetTevOrder(tev, coord, map, color); }
void rust_GX_SetTexCoordGen(u8 coord, u8 typ, u8 src, u8 mtx) { GX_SetTexCoordGen(coord, typ, src, mtx); }
void rust_DCFlushRange(void *addr, u32 len) { DCFlushRange(addr, len); }
