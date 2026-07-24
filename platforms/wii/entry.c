#include <gccore.h>
#include <ogc/gx.h>
#include <stdlib.h>

extern void rabuka_main(void);

GXRModeObj *rmode;
void *xfb[2] = {NULL, NULL};
int fb_idx;
u16 fbWidth;
u16 efbHeight;
u16 xfbHeight;
u8 aa;
u8 field_rendering;
u8 viTVMode;
void *vfilter;
void *sample_pattern;

int main(int argc, char **argv) {
    VIDEO_Init();
    PAD_Init();

    rmode = VIDEO_GetPreferredMode(NULL);
    fbWidth = rmode->fbWidth;
    efbHeight = rmode->efbHeight;
    xfbHeight = rmode->xfbHeight;
    aa = rmode->aa;
    field_rendering = rmode->field_rendering;
    viTVMode = rmode->viTVMode;
    vfilter = rmode->vfilter;
    sample_pattern = rmode->sample_pattern;

    xfb[0] = MEM_K0_TO_K1(SYS_AllocateFramebuffer(rmode));
    xfb[1] = MEM_K0_TO_K1(SYS_AllocateFramebuffer(rmode));
    fb_idx = 0;

    VIDEO_Configure(rmode);
    VIDEO_SetNextFramebuffer(xfb[fb_idx]);
    VIDEO_SetBlack(false);
    VIDEO_Flush();
    VIDEO_WaitVSync();
    if (rmode->viTVMode & VI_NON_INTERLACE)
        VIDEO_WaitVSync();

    rabuka_main();

    return 0;
}

// C wrappers for GX inline functions
void rust_GX_Begin(u8 prim, u8 vtxfmt, u16 n) { GX_Begin(prim, vtxfmt, n); }
void rust_GX_End(void) { GX_End(); }
void rust_GX_Position3f32(f32 x, f32 y, f32 z) { GX_Position3f32(x, y, z); }
void rust_GX_Color1u32(u32 clr) { GX_Color1u32(clr); }
void rust_GX_TexCoord2f32(f32 s, f32 t) { GX_TexCoord2f32(s, t); }
void rust_DCFlushRange(void *addr, u32 len) { DCFlushRange(addr, len); }
