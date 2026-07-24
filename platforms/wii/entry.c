#include <gccore.h>
#include <ogc/gx.h>
#include <stdlib.h>

extern void rabuka_main(void);

GXRModeObj *rmode;
void *xfb[2];
int fb_idx;
u16 fbWidth;
u16 efbHeight;
u16 xfbHeight;
u8 aa;
u8 field_rendering;
u8 viTVMode;

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
