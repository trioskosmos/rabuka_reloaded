/* Rabuka Reloaded — Atari Jaguar port (wasm2c transpiled engine). SHELL.
 *
 * rust -> wasm32 -> wasm2c -> m68k-unknown-linux-gnu-gcc, executed in place
 * from cartridge ROM at $802000. This file is the console shell: a 1-bpp
 * text grid at $42000 rendered by the object processor (light8x8 font),
 * Jagpad input at $F14000/$F14002, and the four host imports the engine's
 * PlatformUi calls. ASCII only — the 8x8 font has no Japanese glyphs.
 *
 * Display/input logic ported from the working rustc_codegen_gcc POC
 * (platforms/jaguar/src/{display,input}.rs).
 */

#include <stdint.h>
#include <stddef.h>
#include "rabuka_wasm.h"

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

/* ---- the single wasm instance so imports can reach linear memory ---- */
struct w2c_host { int unused; };
w2c_0x24rabuka__wasm0x2Ewasm g_rabuka_inst;
struct w2c_host g_host;

/* ---- host imports (called from inside the wasm engine) ---- */

void w2c_host_host_clear_screen(struct w2c_host *h) {
    (void)h;
    disp_clear();
}

void w2c_host_host_println(struct w2c_host *h, u32 ptr, u32 len) {
    (void)h;
    disp_print(g_rabuka_inst.w2c_memory.data + ptr, len);
}

u32 w2c_host_host_poll_buttons(struct w2c_host *h) {
    (void)h;
    return pad_state();
}

void w2c_host_host_wait_vblank(struct w2c_host *h) {
    (void)h;
    frame_wait();
}

/* ---- boot: called from boot_jw.S after init ---- */

void jag_main(void) {
    disp_clear();
    {
        static const unsigned char banner[] = "RABUKA RELOADED - JAGUAR";
        disp_print(banner, sizeof(banner) - 1);
        disp_newline();
        static const unsigned char line2[] = "rust>wasm>wasm2c>m68k-gcc";
        disp_print(line2, sizeof(line2) - 1);
        disp_newline();
    }

    wasm_rt_init();
    memset(&g_rabuka_inst, 0, sizeof(g_rabuka_inst));
    wasm2c_0x24rabuka__wasm0x2Ewasm_instantiate(&g_rabuka_inst, &g_host);

    u32 r = w2c_0x24rabuka__wasm0x2Ewasm_rabuka_wasm_game_run(&g_rabuka_inst, 0x5EEDu);

    static const unsigned char l0[] = "RESULT: FIRST ATTACKER WINS";
    static const unsigned char l1[] = "RESULT: SECOND ATTACKER WINS";
    static const unsigned char l2[] = "RESULT: DRAW";
    static const unsigned char lx[] = "MATCH ENDED";
    const unsigned char *res = lx;
    unsigned int rl = sizeof(lx) - 1;
    if (r == 0) { res = l0; rl = sizeof(l0) - 1; }
    else if (r == 1) { res = l1; rl = sizeof(l1) - 1; }
    else if (r == 2) { res = l2; rl = sizeof(l2) - 1; }
    disp_print(res, rl);

    for (;;)
        frame_wait();
}
