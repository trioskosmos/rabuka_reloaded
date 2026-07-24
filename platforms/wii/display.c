#include <gccore.h>
#include <ogc/gx.h>
#include <ogc/gu.h>
#include <ogc/system.h>
#include <ogc/video.h>
#include <ogc/conf.h>
#include <stdlib.h>
#include <malloc.h>
#include <string.h>

extern void rabuka_main(void);

GXRModeObj *rmode;
void *xfb[2];
int fb_idx;

static void *gp_fifo = NULL;
static sys_fontheader *fontdata = NULL;
static Mtx GXmodelView2D;
static GXTexObj fonttex;

static int cur_x = 0, cur_y = 0;
#define CELL_W 24
#define CELL_H 28

static void gx_init(void) {
    VIDEO_Init();
    VIDEO_SetBlack(true);

    rmode = VIDEO_GetPreferredMode(NULL);
    if (CONF_GetAspectRatio() == CONF_ASPECT_16_9) rmode->viWidth = 678;
    else rmode->viWidth = 672;
    rmode->viXOrigin = (VI_MAX_WIDTH_NTSC - rmode->viWidth) / 2;

    VIDEO_Configure(rmode);
    xfb[0] = MEM_K0_TO_K1(SYS_AllocateFramebuffer(rmode));
    xfb[1] = MEM_K0_TO_K1(SYS_AllocateFramebuffer(rmode));
    fb_idx = 0;
    VIDEO_SetNextFramebuffer(xfb[fb_idx]);
    VIDEO_Flush();
    VIDEO_WaitVSync();
    if (rmode->viTVMode & VI_NON_INTERLACE) VIDEO_WaitVSync();

    gp_fifo = memalign(32, 256 * 1024);
    memset(gp_fifo, 0, 256 * 1024);
    GX_Init(gp_fifo, 256 * 1024);

    GX_SetCopyClear((GXColor){0,0,0,0}, GX_MAX_Z24);

    if (rmode->aa) GX_SetPixelFmt(GX_PF_RGB565_Z16, GX_ZC_LINEAR);
    else GX_SetPixelFmt(GX_PF_RGB8_Z24, GX_ZC_LINEAR);

    f32 yscale = GX_GetYScaleFactor(rmode->efbHeight, rmode->xfbHeight);
    u32 dstH = GX_SetDispCopyYScale(yscale);
    GX_SetDispCopySrc(0, 0, rmode->fbWidth, rmode->efbHeight);
    GX_SetDispCopyDst(rmode->fbWidth, dstH);
    GX_SetCopyFilter(rmode->aa, rmode->sample_pattern, GX_TRUE, rmode->vfilter);
    GX_SetFieldMode(rmode->field_rendering,
        (rmode->viHeight == 2 * rmode->xfbHeight) ? GX_ENABLE : GX_DISABLE);
    GX_SetDispCopyGamma(GX_GM_1_0);

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

    GX_SetNumChans(1);
    GX_SetNumTexGens(1);
    GX_SetTevOp(GX_TEVSTAGE0, GX_PASSCLR);
    GX_SetTevOrder(GX_TEVSTAGE0, GX_TEXCOORD0, GX_TEXMAP0, GX_COLOR0A0);
    GX_SetTexCoordGen(GX_TEXCOORD0, GX_TG_MTX2x4, GX_TG_TEX0, GX_IDENTITY);

    guMtxIdentity(GXmodelView2D);
    guMtxTransApply(GXmodelView2D, GXmodelView2D, 0.0f, 0.0f, -100.0f);
    GX_LoadPosMtxImm(GXmodelView2D, GX_PNMTX0);

    Mtx44 proj;
    guOrtho(proj, 0.0f, rmode->efbHeight, 0.0f, rmode->fbWidth, 0.0f, 1000.0f);
    GX_LoadProjectionMtx(proj, GX_ORTHOGRAPHIC);
    GX_SetViewport(0.0f, 0.0f, rmode->fbWidth, rmode->efbHeight, 0.0f, 1.0f);

    GX_SetBlendMode(GX_BM_BLEND, GX_BL_SRCALPHA, GX_BL_INVSRCALPHA, GX_LO_CLEAR);
    GX_SetAlphaUpdate(GX_TRUE);
    GX_SetAlphaCompare(GX_GREATER, 0, GX_AOP_AND, GX_ALWAYS, 0);
    GX_SetColorUpdate(GX_ENABLE);
    GX_SetCullMode(GX_CULL_NONE);
}

static void sjis_to_gx(const char *utf8) {
    if (!fontdata) return;
    f32 pen_x = cur_x * CELL_W, pen_y = cur_y * CELL_H;
    const u8 *p = (const u8*)utf8;
    while (*p) {
        if (*p == '\n') { pen_x = 0; pen_y += CELL_H; cur_y++; p++; continue; }

        u32 cp;
        if (*p < 0x80) { cp = *p++; }
        else if (*p >= 0xC0 && *p < 0xE0 && p[1]) {
            cp = ((*p & 0x1F) << 6) | (p[1] & 0x3F); p += 2;
        } else if (*p >= 0xE0 && *p < 0xF0 && p[1] && p[2]) {
            cp = ((*p & 0x0F) << 12) | ((p[1] & 0x3F) << 6) | (p[2] & 0x3F); p += 3;
        } else { cp = '?'; p++; }

        int sjis = (cp < 0x80) ? cp : 0;
        if (sjis == 0) {
            // SJIS lookup would go here - simplified for now, use cp as fallback
            sjis = cp;
        }

        void *img; s32 tx, ty, tw;
        SYS_GetFontTexture(sjis, &img, &tx, &ty, &tw);
        if (img && tw > 0) {
            f32 x2 = pen_x + tw;
            f32 y2 = pen_y + CELL_H;
            f32 s1 = (f32)tx / fontdata->sheet_width;
            f32 t1 = (f32)ty / fontdata->sheet_height;
            f32 s2 = (f32)(tx + tw) / fontdata->sheet_width;
            f32 t2 = (f32)(ty + CELL_H) / fontdata->sheet_height;

            GX_InitTexObj(&fonttex, (u8*)fontdata + fontdata->sheet_image,
                fontdata->sheet_width, fontdata->sheet_height,
                fontdata->sheet_format, GX_CLAMP, GX_CLAMP, GX_FALSE);
            GX_LoadTexObj(&fonttex, GX_TEXMAP0);
            GX_SetTevOp(GX_TEVSTAGE0, GX_MODULATE);
            GX_SetVtxDesc(GX_VA_TEX0, GX_DIRECT);

            GX_Begin(GX_QUADS, GX_VTXFMT0, 4);
                GX_Position3f32(pen_x, pen_y, 0); GX_Color1u32(0xFFFFFFFF); GX_TexCoord2f32(s1, t1);
                GX_Position3f32(x2, pen_y, 0); GX_Color1u32(0xFFFFFFFF); GX_TexCoord2f32(s2, t1);
                GX_Position3f32(x2, y2, 0); GX_Color1u32(0xFFFFFFFF); GX_TexCoord2f32(s2, t2);
                GX_Position3f32(pen_x, y2, 0); GX_Color1u32(0xFFFFFFFF); GX_TexCoord2f32(s1, t2);
            GX_End();

            GX_SetTevOp(GX_TEVSTAGE0, GX_PASSCLR);
            GX_SetVtxDesc(GX_VA_TEX0, GX_NONE);

            pen_x += tw;
            cur_x++;
        }
    }
}

void display_init(void) {
    gx_init();
    VIDEO_SetBlack(false);

    fontdata = memalign(32, SYS_FONTSIZE_SJIS);
    memset(fontdata, 0, SYS_FONTSIZE_SJIS);
    SYS_InitFont(fontdata);
    DCFlushRange((u8*)fontdata + fontdata->sheet_image, fontdata->sheet_fullsize);
}

void display_clear(void) { cur_x = 0; cur_y = 0; }

void display_print(const char *text) { sjis_to_gx(text); }

void display_swap(void) {
    GX_DrawDone();
    GX_InvalidateTexAll();
    fb_idx ^= 1;
    GX_SetZMode(GX_TRUE, GX_LEQUAL, GX_TRUE);
    GX_SetColorUpdate(GX_ENABLE);
    GX_CopyDisp(xfb[fb_idx], GX_TRUE);
    VIDEO_SetNextFramebuffer(xfb[fb_idx]);
    VIDEO_Flush();
    VIDEO_WaitVSync();
    if (rmode->viTVMode & VI_NON_INTERLACE) VIDEO_WaitVSync();
}
