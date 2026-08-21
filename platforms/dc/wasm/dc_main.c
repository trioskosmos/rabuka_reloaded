/* Rabuka Reloaded  ESega Dreamcast port (wasm2c transpiled engine). PLAYABLE.
 *
 * rust -> wasm32 -> wasm2c -> sh-elf-gcc. This file is the console shell:
 * a BIOS-font text grid + maple controller, implementing the four host
 * imports the engine's PlatformUi calls, then handing control to
 * rabuka_wasm_game_run() (mode select -> deck select -> match).
 *
 * Display: double-buffered 640x480 RGB565 (DM_MULTIBUFFER), BIOS font at
 * 24px line height (BFONT_HEIGHT). The engine sends UTF-8; the BIOS font
 * consumes Shift-JIS, so text is converted via a generated table
 * (sjis_table.c) and bfont_set_encoding(BFONT_CODE_SJIS)  EJapanese card
 * names render with the console's full-width glyphs (24px wide; ASCII is
 * the 12px thin font). Wrapping is by pixel width, not char count.
 */

#include <kos.h>
#include <string.h>
#include "rabuka_wasm.h"
#include "sjis_table.h"

#define SCREEN_W 640
#define SCREEN_H 480
#define FONT_H 24
#define THIN_W 12
#define WIDE_W 24
#define ROWS (SCREEN_H / FONT_H)      /* 20 */
#define MAX_ROW_BYTES 56              /* 53 thin glyphs max, + slack */
#define Y0 0

/* the single wasm instance, so imports can reach linear memory */
struct w2c_host { int unused; };  /* opaque to the engine; contents are ours */
w2c_0x24rabuka__wasm0x2Ewasm g_rabuka_inst;
struct w2c_host g_host;

/* each row is a Shift-JIS byte string (thin = 1 byte, wide = 2 bytes) */
static unsigned char lines[ROWS][MAX_ROW_BYTES];
static unsigned char row_len[ROWS];
static int cur_line = 0;
static int dirty = 0;
static int disp_fb = 0; /* informational: last flipped framebuffer */
static int back_idx = 1;

/* ---- text grid ---- */

static void paint(u16 *dst) {
    for (int i = 0; i < SCREEN_W * SCREEN_H; i++)
        dst[i] = 0x0000;
    for (int i = 0; i < ROWS; i++) {
        if (row_len[i] == 0)
            continue;
        bfont_draw_str(dst + i * FONT_H * SCREEN_W, SCREEN_W, true,
                       (const char *)lines[i]);
    }
}

/* Paint KOS's current drawing surface (vram_s) and flip. This is the
 * official multibuffer pattern (examples/dreamcast/video/multibuffer):
 * vid_flip(-1) scans out what we just painted and rotates vram_s to the
 * next hidden buffer, so every displayed frame is freshly painted and no
 * write ever hits the scaned-out surface. */
static void frame(void) {
    if (!dirty)
        return;
    dirty = 0;
    paint(vram_s);
    vid_flip(-1);
}

static void scroll_up(void) {
    for (int i = 1; i < ROWS; i++) {
        memcpy(lines[i - 1], lines[i], MAX_ROW_BYTES);
        row_len[i - 1] = row_len[i];
    }
    row_len[ROWS - 1] = 0;
}

/* cur_line = next row to write; scroll when it runs past the bottom */
static void ensure_line(void) {
    if (cur_line >= ROWS) {
        scroll_up();
        cur_line = ROWS - 1;
    }
}

static void blank_line(void) {
    ensure_line();
    row_len[cur_line] = 0;
    cur_line++;
}

static void row_append(int row, const unsigned char *bytes, int nbytes) {
    memcpy(lines[row] + row_len[row], bytes, nbytes);
    row_len[row] += nbytes;
}

/* Append a converted glyph run to the grid, wrapping on pixel width.
 * Prefers breaking at an ASCII space; wide (JP) glyphs may break anywhere. */
static void push_glyphs(const unsigned char *g, const unsigned char *w,
                        int nglyphs) {
    int x = 0;
    int start = 0;
    while (start < nglyphs) {
        ensure_line();
        /* fit as many glyphs as possible into the row */
        int i = start;
        int last_space_end = -1;
        while (i < nglyphs) {
            if (x + w[i] > SCREEN_W)
                break;
            x += w[i];
            i++;
            if (i < nglyphs && g[(i - 1) * 2] == ' ')
                last_space_end = i; /* remember space break candidates */
        }
        if (i == start) /* single glyph wider than screen: force it */
            i++;
        int end = (i < nglyphs && last_space_end > start) ? last_space_end : i;
        /* emit glyphs [start, end) */
        for (int k = start; k < end; k++) {
            int nb = (w[k] == WIDE_W) ? 2 : 1;
            if (row_len[cur_line] + nb > MAX_ROW_BYTES)
                break;
            row_append(cur_line, g + k * 2, nb);
        }
        cur_line++;
        /* skip one space at the break if we broke on a space */
        if (end < nglyphs && end == last_space_end && g[end * 2] == ' ')
            end++;
        start = end;
        x = 0;
    }
}

/* Convert UTF-8 -> Shift-JIS glyphs and push into the grid. */
static void put_line(const char *s, int len) {
    if (len == 0) {
        blank_line(); /* empty string = blank separator line */
        return;
    }
    static unsigned char g[1024];   /* sjis bytes, 2 per wide glyph */
    static unsigned char w[512];    /* per-glyph pixel width */
    int ng = 0;
    int i = 0;
    while (i < len && ng < 500) {
        unsigned char c = (unsigned char)s[i];
        if (c < 0x80) {
            g[ng * 2] = c;
            w[ng] = THIN_W;
            ng++;
            i++;
        } else {
            /* decode UTF-8 (2-4 bytes) */
            uint32_t cp;
            int adv;
            if ((c & 0xE0) == 0xC0 && i + 1 < len) {
                cp = ((c & 0x1F) << 6) | (s[i + 1] & 0x3F);
                adv = 2;
            } else if ((c & 0xF0) == 0xE0 && i + 2 < len) {
                cp = ((c & 0x0F) << 12) | ((s[i + 1] & 0x3F) << 6) |
                     (s[i + 2] & 0x3F);
                adv = 3;
            } else if ((c & 0xF8) == 0xF0 && i + 3 < len) {
                cp = ((c & 0x07) << 18) | ((s[i + 1] & 0x3F) << 12) |
                     ((s[i + 2] & 0x3F) << 6) | (s[i + 3] & 0x3F);
                adv = 4;
            } else {
                cp = '?';
                adv = 1;
            }
            uint16_t sj = sjis_from_unicode(cp);
            if (sj == 0) {
                g[ng * 2] = '?';
                w[ng] = THIN_W;
            } else {
                g[ng * 2] = (sj >> 8) & 0xFF;
                g[ng * 2 + 1] = sj & 0xFF;
                w[ng] = WIDE_W;
            }
            ng++;
            i += adv;
        }
    }
    push_glyphs(g, w, ng);
}

/* ---- host imports (called from inside the wasm engine) ---- */

void w2c_host_host_clear_screen(struct w2c_host *h) {
    (void)h;
    for (int i = 0; i < ROWS; i++)
        row_len[i] = 0;
    cur_line = 0;
    dirty = 1;
}

void w2c_host_host_println(struct w2c_host *h, u32 ptr, u32 len) {
    (void)h;
    static char buf[512];
    if (len > sizeof(buf) - 1)
        len = sizeof(buf) - 1;
    memcpy(buf, g_rabuka_inst.w2c_memory.data + ptr, len);
    buf[len] = '\0';
    /* split on embedded newlines; each piece is one logical line */
    char *start = buf;
    for (char *p = buf;; p++) {
        if (*p == '\n') {
            *p = '\0';
            put_line(start, p - start);
            start = p + 1;
            if (*start == '\0') {
                blank_line();
                break;
            }
        } else if (*p == '\0') {
            put_line(start, p - start);
            break;
        }
    }
    dirty = 1;
}

u32 w2c_host_host_poll_buttons(struct w2c_host *h) {
    (void)h;
    u32 mask = 0;
    maple_device_t *dev = maple_enum_type(0, MAPLE_FUNC_CONTROLLER);
    if (dev) {
        cont_state_t *st = (cont_state_t *)maple_dev_status(dev);
        if (st) {
            if (st->buttons & CONT_A)         mask |= 1;
            if (st->buttons & CONT_B)         mask |= 2;
            if (st->buttons & CONT_DPAD_UP)   mask |= 4;
            if (st->buttons & CONT_DPAD_DOWN) mask |= 8;
            if (st->buttons & CONT_START)     mask |= 16;
        }
    }
    return mask;
}

void w2c_host_host_wait_vblank(struct w2c_host *h) {
    (void)h;
    frame();
    thd_sleep(8);
}

/* ---- boot ---- */

int main(void) {
    vid_set_mode(DM_640x480 | DM_MULTIBUFFER, PM_RGB565);
    bfont_set_encoding(BFONT_CODE_SJIS);

    for (int i = 0; i < ROWS; i++)
        row_len[i] = 0;
    cur_line = 0;
    put_line("RABUKA RELOADED - DREAMCAST", 27);
    put_line("rust -> wasm -> wasm2c -> sh-elf-gcc", 36);
    blank_line();
    put_line("loading engine + card data...", 28);
    dirty = 1;
    frame();

    wasm_rt_init();
    memset(&g_rabuka_inst, 0, sizeof(g_rabuka_inst));
    wasm2c_0x24rabuka__wasm0x2Ewasm_instantiate(&g_rabuka_inst, &g_host);

    u32 r = w2c_0x24rabuka__wasm0x2Ewasm_rabuka_wasm_game_run(&g_rabuka_inst, 0x5EEDu);

    const char *res;
    switch (r) {
    case 0: res = "FIRST ATTACKER WINS THE MATCH!"; break;
    case 1: res = "SECOND ATTACKER WINS THE MATCH!"; break;
    case 2: res = "DRAW."; break;
    default: res = "MATCH ENDED."; break;
    }
    blank_line();
    put_line(res, strlen(res));
    put_line("Press START to exit.", 20);
    dirty = 1;
    frame();

    for (;;) {
        maple_device_t *dev = maple_enum_type(0, MAPLE_FUNC_CONTROLLER);
        if (dev) {
            cont_state_t *st = (cont_state_t *)maple_dev_status(dev);
            if (st && (st->buttons & CONT_START))
                arch_exit();
        }
        thd_sleep(10);
    }
    return 0;
}
