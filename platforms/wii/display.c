#include <gccore.h>
#include <ogc/gx.h>
#include <ogc/gu.h>
#include <ogc/system.h>
#include <ogc/video.h>
#include <stdlib.h>
#include <malloc.h>
#include <string.h>
#include "sjis_map.h"

extern GXRModeObj *rmode;
extern void *xfb[2];
extern int fb_idx;
extern u16 fbWidth;
extern u16 efbHeight;
extern u16 xfbHeight;
extern u8 aa;
extern u8 field_rendering;
extern u8 viTVMode;

static sys_fontheader *fontdata = NULL;
static int cur_x = 0, cur_y = 0;
static const int CELL_W = 24, CELL_H = 28;
static const int CURSOR_X = 2, CURSOR_Y = 2;

static u16 utf32_to_sjis(u32 cp) {
    if (cp < 0x80) return (u16)cp;
    int lo = 0, hi = SJIS_MAP_ENTRIES - 1;
    while (lo <= hi) {
        int mid = (lo + hi) / 2;
        const uint8_t *e = sjis_map_data + mid * 6;
        u32 key = (u32)e[0] | ((u32)e[1] << 8) | ((u32)e[2] << 16) | ((u32)e[3] << 24);
        if (key == cp) return ((u16)e[4] << 8) | e[5];
        if (cp < key) hi = mid - 1;
        else lo = mid + 1;
    }
    return 0;
}

static u32 utf8_next(const u8 **p) {
    u8 b = *(*p)++;
    if (b < 0x80) return b;
    if (b >= 0xC0 && b < 0xE0) return ((b & 0x1F) << 6) | (*(*p)++ & 0x3F);
    if (b >= 0xE0 && b < 0xF0) {
        u32 cp = (b & 0x0F) << 12;
        cp |= (*(*p)++ & 0x3F) << 6;
        cp |= *(*p)++ & 0x3F;
        return cp;
    }
    return '?';
}

void display_init(void) {
    void *fifo = memalign(32, 256 * 1024);
    memset(fifo, 0, 256 * 1024);
    GX_Init(fifo, 256 * 1024);

    GX_SetCopyClear((GXColor){0,0,0,0}, GX_MAX_Z24);
    GX_SetPixelFmt(GX_PF_RGB8_Z24, GX_ZC_LINEAR);

    f32 yscale = GX_GetYScaleFactor(efbHeight, xfbHeight);
    u32 dstH = GX_SetDispCopyYScale(yscale);
    GX_SetDispCopySrc(0, 0, fbWidth, efbHeight);
    GX_SetDispCopyDst(fbWidth, dstH);
    GX_SetCopyFilter(aa, NULL, GX_TRUE, NULL);
    GX_SetFieldMode(field_rendering, (viTVMode & VI_NON_INTERLACE) ? GX_DISABLE : GX_ENABLE);
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

    Mtx mv;
    guMtxIdentity(mv);
    guMtxTransApply(mv, mv, 0, 0, -100);
    GX_LoadPosMtxImm(mv, GX_PNMTX0);

    Mtx44 proj;
    guOrtho(proj, 0, 479, 0, 639, 0, 1000);
    GX_LoadProjectionMtx(proj, GX_ORTHOGRAPHIC);
    GX_SetViewport(0, 0, fbWidth, efbHeight, 0, 1);

    GX_SetBlendMode(GX_BM_BLEND, GX_BL_SRCALPHA, GX_BL_INVSRCALPHA, GX_LO_CLEAR);
    GX_SetAlphaUpdate(GX_TRUE);
    GX_SetAlphaCompare(GX_GREATER, 0, GX_AOP_AND, GX_ALWAYS, 0);
    GX_SetColorUpdate(GX_ENABLE);
    GX_SetCullMode(GX_CULL_NONE);

    VIDEO_WaitVSync();

    fontdata = memalign(32, SYS_FONTSIZE_SJIS);
    memset(fontdata, 0, SYS_FONTSIZE_SJIS);
    SYS_InitFont(fontdata);
    DCFlushRange((u8*)fontdata + fontdata->sheet_image, fontdata->sheet_fullsize);
}

void display_clear(void) {
    cur_x = CURSOR_X;
    cur_y = CURSOR_Y;
}

void display_print(const char *str) {
    const u8 *p = (const u8*)str;
    while (*p) {
        if (*p == '\n') { cur_x = CURSOR_X; cur_y++; p++; continue; }
        u32 cp = utf8_next(&p);
        u16 sjis = utf32_to_sjis(cp);
        if (sjis == 0) sjis = '?';

        void *img;
        s32 tx, ty, tw;
        SYS_GetFontTexture((s32)sjis, &img, &tx, &ty, &tw);
        if (img && tw > 0) {
            GXTexObj to;
            GX_InitTexObj(&to, (u8*)fontdata + fontdata->sheet_image,
                          fontdata->sheet_width, fontdata->sheet_height,
                          fontdata->sheet_format, GX_CLAMP, GX_CLAMP, GX_FALSE);
            GX_LoadTexObj(&to, GX_TEXMAP0);

            GX_SetTevOp(GX_TEVSTAGE0, GX_MODULATE);
            GX_SetVtxDesc(GX_VA_TEX0, GX_DIRECT);

            f32 x = cur_x * CELL_W, y = cur_y * CELL_H;
            f32 s1 = (f32)tx / fontdata->sheet_width;
            f32 t1 = (f32)ty / fontdata->sheet_height;
            f32 s2 = (f32)(tx + tw) / fontdata->sheet_width;
            f32 t2 = (f32)(ty + CELL_H) / fontdata->sheet_height;

            GX_Begin(GX_QUADS, GX_VTXFMT0, 4);
            GX_Position3f32(x, y, 0); GX_Color1u32(0xFFFFFFFF); GX_TexCoord2f32(s1, t1);
            GX_Position3f32(x+tw, y, 0); GX_Color1u32(0xFFFFFFFF); GX_TexCoord2f32(s2, t1);
            GX_Position3f32(x+tw, y+CELL_H, 0); GX_Color1u32(0xFFFFFFFF); GX_TexCoord2f32(s2, t2);
            GX_Position3f32(x, y+CELL_H, 0); GX_Color1u32(0xFFFFFFFF); GX_TexCoord2f32(s1, t2);
            GX_End();

            cur_x++;
        }
    }
}

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
    if (viTVMode & VI_NON_INTERLACE) VIDEO_WaitVSync();
}
