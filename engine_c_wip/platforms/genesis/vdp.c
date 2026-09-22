/* engine_c/platforms/genesis/vdp.c — minimal VDP text console (Mode 5, H40).
   Uploads genesis_font as 8x8 tiles and writes ASCII into Plane A. */
#include <stddef.h>
#include <stdint.h>
#include "vdp.h"
#include "vdp_font.h"

#define VDP_CTRL  (*(volatile uint32_t *)0xC00004)
#define VDP_DATA  (*(volatile uint16_t *)0xC00000)
#define SRAM_ADDR ((volatile uint16_t *)0x200000)

#define PLANE_A_BASE 0xC000u
#define PLANE_W      64          /* name-table stride (cells) in H40 */
#define FONT_COLOR   0x0F        /* lit-pixel color (palette 0, color 15) */

static void vdp_reg(uint16_t reg, uint16_t val) {
    VDP_CTRL = (uint32_t)0x80000000u | ((uint32_t)(reg & 0x1F) << 8) | (uint32_t)(val & 0xFF);
}

static void vdp_set_vram_addr(uint32_t a) {
    VDP_CTRL = 0x40000000u | ((a & 0x3FFFu) << 16) | ((a >> 14) & 0x3u);
}
static void vdp_set_cram_addr(uint32_t a) {
    VDP_CTRL = 0xC0000000u | ((a & 0x3FFFu) << 16) | ((a >> 14) & 0x3u);
}

void vdp_init(void) {
    /* Mode 5 text setup: H40, 224 lines, Plane A at 0xC000. */
    vdp_reg(0,  0x14);
    vdp_reg(1,  0x74);   /* display on + DMA + VINT */
    vdp_reg(2,  0x30);   /* Plane A = 0xC000 */
    vdp_reg(3,  0x3C);   /* Window = 0xF000 (unused) */
    vdp_reg(4,  0x07);   /* Plane B = 0xE000 */
    vdp_reg(5,  0x6C);   /* Sprite attr = 0xD800 */
    vdp_reg(6,  0x00);
    vdp_reg(7,  0x00);   /* background = palette 0 color 0 */
    vdp_reg(8,  0x00);
    vdp_reg(9,  0x00);
    vdp_reg(10, 0x00);
    vdp_reg(11, 0x00);
    vdp_reg(12, 0x81);   /* H40 + 224-line */
    vdp_reg(13, 0x3F);   /* H-scroll = 0xFC00 */
    vdp_reg(14, 0x00);
    vdp_reg(15, 0x02);   /* auto-increment = 2 */

    /* Upload font tiles (each code maps to its ASCII tile index). */
    for (int c = FONT_FIRST; c <= FONT_LAST; c++) {
        vdp_set_vram_addr((uint32_t)c * 32u);
        const unsigned char *g = genesis_font[c - FONT_FIRST];
        for (int i = 0; i < 32; i += 2) {
            VDP_DATA = (uint16_t)(((uint16_t)g[i] << 8) | g[i + 1]);
        }
    }

    /* Palette 0: color 0 = black background, 1..15 = light foreground. */
    vdp_set_cram_addr(0);
    VDP_DATA = 0x0000;            /* color 0 (background) */
    for (int i = 1; i < 16; i++) VDP_DATA = 0x0EEE;  /* light */

    vdp_clear(0);
}

void vdp_clear(uint8_t color_idx) {
    (void)color_idx;
    vdp_set_vram_addr(PLANE_A_BASE);
    for (int i = 0; i < PLANE_W * 28; i++) VDP_DATA = 0x0020; /* space tile */
}

void vdp_puts(int row, int col, const char *s) {
    if (row < 0 || row >= 28 || col < 0) return;
    uint32_t a = (uint32_t)PLANE_A_BASE + (uint32_t)(row * PLANE_W + col) * 2u;
    vdp_set_vram_addr(a);
    for (const char *p = s; *p; p++, col++) {
        if (col >= PLANE_W) break;
        int ch = (unsigned char)*p;
        if (ch >= 'a' && ch <= 'z') ch -= 32;          /* lowercase -> uppercase */
        if (ch < FONT_FIRST || ch > FONT_LAST) ch = '?';
        VDP_DATA = (uint16_t)ch;   /* tile index == ASCII code, palette 0 */
    }
}

void vdp_write_result_word(int addr, uint16_t val) {
    SRAM_ADDR[addr] = val;   /* battery-backed RAM is at 0x200000 */
}
