/* Atari Jaguar text console: a 1-bpp bitmap at $42000 rendered by
 * the object processor (light8x8 font), Jagpad input at
 * $F14000/$F14002. Display/input logic ported from the proven
 * wasm2c shell (platforms/jaguar/wasm/jag_main.c).
 *
 * ASCII only — the 8x8 font has no Japanese glyphs, so Japanese
 * card names print as gaps (same as the wasm2c build).
 */
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdarg.h>
#include "console.h"

typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;

/* ---- display (1-bpp bitmap object processed onto the screen) ---- */
#define TEXT_SCREEN 0x00042000u
#define BYTES_PER_ROW 40          /* 320 px / 8 */
#define MAX_Y 240
#define CHARS_PER_ROW 40
#define CHAR_ROWS 30

#define FONT_BASE 8               /* .fnt header is 8 bytes; 8 bytes/glyph */
extern const unsigned char font_light8x8[];

static int cur_col = 0;
static int cur_row = 0;

static volatile unsigned char * const text_screen =
    (volatile unsigned char *)TEXT_SCREEN;

static void disp_clear(void) {
    for (int i = 0; i < BYTES_PER_ROW * MAX_Y; i++)
        text_screen[i] = 0;
    cur_col = 0;
    cur_row = 0;
}

static void disp_newline(void) {
    cur_col = 0;
    cur_row++;
    if (cur_row >= CHAR_ROWS)
        cur_row = 0;
}

static void disp_put_char(unsigned char c) {
    if (cur_col >= CHARS_PER_ROW)
        disp_newline();
    unsigned int glyph_off = FONT_BASE + (unsigned int)c * 8;
    for (int r = 0; r < 8; r++) {
        unsigned int addr = (unsigned int)(cur_row * 8 + r) * BYTES_PER_ROW + cur_col;
        text_screen[addr] = font_light8x8[glyph_off + r];
    }
    cur_col++;
}

static void disp_print(const unsigned char *s, unsigned int len) {
    for (unsigned int i = 0; i < len; i++) {
        unsigned char b = s[i];
        if (b == '\n')
            disp_newline();
        else if (b >= 0x20 && b <= 0x7E)
            disp_put_char(b);
        /* non-ASCII (Japanese card names): skipped, no glyphs in this font */
    }
}

/* ---- input (Jagpad) ---- */
#define JAGPAD (*(volatile u16 *)0x00F14000u)
#define JOYDIR (*(volatile u16 *)0x00F14002u)

/* engine button mask: A=1, B=2, Up=4, Down=8, Start=16 */
static u32 pad_state(void) {
    u16 buttons = JAGPAD;
    u16 dir = JOYDIR;
    u32 mask = 0;
    if (buttons & 0x0001) mask |= 1;   /* A */
    if (buttons & 0x0002) mask |= 2;   /* B */
    if (buttons & 0x0008) mask |= 16;  /* Pause -> Start */
    if (buttons & 0x8000) mask |= 16;  /* Option -> Start */
    if (dir & 0x0001) mask |= 4;       /* Up */
    if (dir & 0x0002) mask |= 8;       /* Down */
    return mask;
}

/* ---- frame pacing (single buffered; spin ~1 frame like the POC) ---- */
static void frame_wait(void) {
    volatile unsigned n = 0;
    while (n < 200000u)
        n++;
}

/* ---- console API (console.h) ---- */

void console_init(void) {
    disp_clear();
}
void console_clear(void) {
    disp_clear();
}
void console_putc(char c) {
    disp_print((const unsigned char *)&c, 1);
}
void console_puts(const char *s) {
    disp_print((const unsigned char *)s, (unsigned int)strlen(s));
}
void console_printf(const char *fmt, ...) {
    char b[600];
    va_list ap; va_start(ap, fmt);
    vsnprintf(b, sizeof b, fmt, ap);
    va_end(ap);
    disp_print((const unsigned char *)b, (unsigned int)strlen(b));
}

void __console_out(int c) {
    unsigned char b = (unsigned char)c;
    disp_print(&b, 1);
}

unsigned int console_poll_buttons(void) {
    return pad_state();
}

void console_frame_wait(void) {
    frame_wait();
}
