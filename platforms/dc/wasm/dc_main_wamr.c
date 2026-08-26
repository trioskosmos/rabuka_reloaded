/* Rabuka Reloaded  ESega Dreamcast port (WAMR classic-interpreter engine).
 *
 * rust -> wasm32 -> [WAMR classic interpreter on SH-4].
 * No wasm2c: the 2MB rabuka_wasm.wasm is embedded in the ELF as data,
 * loaded at boot, interpreted in place. Target-side code is just this
 * shell + the interpreter (~100-200KB total instead of ~4MB transpiled C).
 *
 * Display/input identical to the wasm2c shell: double-buffered 640x480
 * RGB565, BIOS Shift-JIS font via generated table (sjis_table.c), maple
 * controller. Implements the four host imports the engine calls, then
 * runs rabuka_wasm_game_run() (mode select -> deck select -> match).
 */

#include <kos.h>
#include <string.h>
#include "wasm_export.h"
#include "sjis_table.h"

#define SCREEN_W 640
#define SCREEN_H 480
#define FONT_H 24
#define THIN_W 12
#define WIDE_W 24
#define ROWS (SCREEN_H / FONT_H)      /* 20 */
#define MAX_ROW_BYTES 64              /* 53 thin (53B) or 26 wide (52B) + slack */
#ifndef BUILD_TAG
#define BUILD_TAG __DATE__ " " __TIME__
#endif

/* embedded rabuka_wasm.wasm (objcopy binary blob; must outlive the module) */
extern const uint8_t binary_rabuka_wasm_wasm_start[];
extern const uint8_t binary_rabuka_wasm_wasm_end[];

/* each row is a Shift-JIS byte string (thin = 1 byte, wide = 2 bytes). */
static unsigned char lines[ROWS][MAX_ROW_BYTES + 1];
static unsigned char row_len[ROWS];
static int cur_line = 0;
static int dirty = 0;
static int back_idx = 1;

/* ---- on-screen vitals (reserved bottom row) ----
 * The engine scrolls through rows 0..ROWS-2; row ROWS-1 always shows
 * fps / avg frame ms / heap high-water so performance and memory can be
 * judged live without external tooling. */
static wasm_module_inst_t g_inst = NULL;
static wasm_exec_env_t g_exec = NULL;
static wasm_function_inst_t hwm_fn = NULL;
static wasm_function_inst_t cur_fn = NULL;
static wasm_function_inst_t rc_bytes_fn = NULL;
static wasm_function_inst_t rc_n_fn = NULL;
static uint64_t last_frame_ms = 0;
static uint32_t frame_avg_ms = 0;
static uint32_t fps_x10 = 0;
static uint32_t frame_count = 0;
static uint64_t fps_window_start = 0;
static uint32_t heap_hwm_kb = 0;
static uint32_t heap_cur_kb = 0;
static uint32_t heap_rc_kb = 0;
static uint32_t heap_rc_n = 0;

static uint32_t call0(wasm_function_inst_t fn) {
    if (!fn || !g_inst || !g_exec)
        return 0;
    uint32_t args[1] = { 0 };
    if (!wasm_runtime_call_wasm(g_exec, fn, 0, args))
        return 0;
    return args[0];
}

static void poll_heap_stats(void) {
    heap_hwm_kb = call0(hwm_fn) / 1024;
    heap_cur_kb = call0(cur_fn) / 1024;
    heap_rc_kb = call0(rc_bytes_fn) / 1024;
    heap_rc_n = call0(rc_n_fn);
}

/* ---- text grid ---- */

static void paint(uint16_t *dst) {
    for (int i = 0; i < SCREEN_W * SCREEN_H; i++)
        dst[i] = 0x0000;
    for (int i = 0; i < ROWS - 1; i++) {
        if (row_len[i] == 0)
            continue;
        bfont_draw_str(dst + i * FONT_H * SCREEN_W, SCREEN_W, true,
                       (const char *)lines[i]);
    }
    char sb[56];
    snprintf(sb, sizeof(sb), "%u.%ufps %ums h:%uK c:%uK rc:%uK/%u", fps_x10 / 10,
             fps_x10 % 10, frame_avg_ms, heap_hwm_kb, heap_cur_kb, heap_rc_kb,
             heap_rc_n);
    bfont_draw_str(dst + (ROWS - 1) * FONT_H * SCREEN_W, SCREEN_W, true, sb);
}

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

static void ensure_line(void) {
    if (cur_line >= ROWS - 1) {
        scroll_up();
        cur_line = ROWS - 2;
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
    lines[row][row_len[row]] = '\0';
}

static void push_glyphs(const unsigned char *g, const unsigned char *w,
                        int nglyphs) {
    int x = 0;
    int start = 0;
    while (start < nglyphs) {
        ensure_line();
        int i = start;
        int last_space_end = -1;
        while (i < nglyphs) {
            if (x + w[i] > SCREEN_W)
                break;
            x += w[i];
            i++;
            if (i < nglyphs && g[(i - 1) * 2] == ' ')
                last_space_end = i;
        }
        if (i == start)
            i++;
        int end = (i < nglyphs && last_space_end > start) ? last_space_end : i;
        for (int k = start; k < end; k++) {
            int nb = (w[k] == WIDE_W) ? 2 : 1;
            if (row_len[cur_line] + nb > MAX_ROW_BYTES)
                break;
            row_append(cur_line, g + k * 2, nb);
        }
        cur_line++;
        if (end < nglyphs && end == last_space_end && g[end * 2] == ' ')
            end++;
        start = end;
        x = 0;
    }
}

static void put_line(const char *s, int len) {
    if (len == 0) {
        blank_line();
        return;
    }
    static unsigned char g[1024];
    static unsigned char w[512];
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

/* ---- host imports (called from inside the interpreted engine) ---- */

static void host_clear_screen(wasm_exec_env_t env) {
    (void)env;
    for (int i = 0; i < ROWS; i++)
        row_len[i] = 0;
    cur_line = 0;
    dirty = 1;
}

static void host_println(wasm_exec_env_t env, int32_t ptr, int32_t len) {
    static char buf[512];
    wasm_module_inst_t inst = wasm_runtime_get_module_inst(env);
    if (!inst || !wasm_runtime_validate_app_addr(inst, ptr, len)) {
        put_line("<println bad addr>", 17);
        dirty = 1;
        return;
    }
    const char *src = wasm_runtime_addr_app_to_native(inst, ptr);
    if (len > sizeof(buf) - 1)
        len = sizeof(buf) - 1;
    memcpy(buf, src, len);
    buf[len] = '\0';
    /* Tag the phase header with the build tag (proves which build runs). */
    if (strncmp(buf, "Turn ", 5) == 0) {
        size_t n = strlen(buf);
        if (n + 1 + strlen(BUILD_TAG) < sizeof(buf)) {
            buf[n] = ' ';
            strcpy(buf + n + 1, BUILD_TAG);
            len = n + 1 + strlen(BUILD_TAG);
        }
    }
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

static int32_t host_poll_buttons(wasm_exec_env_t env) {
    (void)env;
    uint32_t mask = 0;
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
    return (int32_t)mask;
}

static void host_wait_vblank(wasm_exec_env_t env) {
    (void)env;
    uint64_t now = timer_ms_gettime64();
    if (last_frame_ms != 0) {
        uint32_t dt = (uint32_t)(now - last_frame_ms);
        frame_avg_ms = frame_avg_ms ? (frame_avg_ms * 7 + dt) / 8 : dt;
    }
    last_frame_ms = now;
    frame_count++;
    if (now >= fps_window_start + 1000) {
        fps_x10 =
            (uint32_t)(frame_count * 10000 / (now - fps_window_start + 1));
        frame_count = 0;
        fps_window_start = now;
        poll_heap_stats();
        dirty = 1; /* repaint so the vitals row stays current */
    }
    frame();
    thd_sleep(8);
}

static NativeSymbol host_natives[] = {
    { "host_clear_screen", host_clear_screen, "()", NULL },
    { "host_println",      host_println,      "(ii)", NULL },
    { "host_poll_buttons", host_poll_buttons, "()i", NULL },
    { "host_wait_vblank",  host_wait_vblank,  "()", NULL },
};

/* ---- boot ---- */

int main(void) {
    vid_set_mode(DM_640x480 | DM_MULTIBUFFER, PM_RGB565);
    bfont_set_encoding(BFONT_CODE_SJIS);

    for (int i = 0; i < ROWS; i++)
        row_len[i] = 0;
    cur_line = 0;
    put_line("RABUKA RELOADED - DREAMCAST", 27);
    put_line("rust -> wasm -> WAMR interp (sh-4)", 34);
    blank_line();
    put_line("loading engine + card data...", 28);
    dirty = 1;
    frame();

    RuntimeInitArgs init;
    memset(&init, 0, sizeof(init));
    init.mem_alloc_type = Alloc_With_System_Allocator;
    if (!wasm_runtime_full_init(&init)) {
        put_line("FATAL: runtime init failed", 26);
        dirty = 1;
        frame();
        for (;;) ;
    }

    /* imports must be registered before wasm_runtime_load links them */
    if (!wasm_runtime_register_natives(
            "host", host_natives,
            sizeof(host_natives) / sizeof(host_natives[0]))) {
        put_line("FATAL: register natives failed", 30);
        dirty = 1;
        frame();
        for (;;) ;
    }

    size_t wasm_size =
        (size_t)(binary_rabuka_wasm_wasm_end - binary_rabuka_wasm_wasm_start);

    char err[160];
    wasm_module_t module = wasm_runtime_load((const uint8_t *)binary_rabuka_wasm_wasm_start,
                                             (uint32)wasm_size, err, sizeof(err));
    if (!module) {
        put_line("FATAL: wasm load failed:", 24);
        put_line(err, strnlen(err, sizeof(err)));
        dirty = 1;
        frame();
        for (;;) ;
    }

    wasm_module_inst_t inst =
        wasm_runtime_instantiate(module, 256 * 1024, 0, err, sizeof(err));
    if (!inst) {
        put_line("FATAL: instantiate failed:", 26);
        put_line(err, strnlen(err, sizeof(err)));
        dirty = 1;
        frame();
        for (;;) ;
    }

    wasm_exec_env_t env = wasm_runtime_create_exec_env(inst, 256 * 1024);
    wasm_function_inst_t run =
        wasm_runtime_lookup_function(inst, "rabuka_wasm_game_run");
    if (!env || !run) {
        put_line("FATAL: export lookup failed", 27);
        dirty = 1;
        frame();
        for (;;) ;
    }

    /* vitals plumbing for the status row */
    g_inst = inst;
    g_exec = env;
    hwm_fn = wasm_runtime_lookup_function(inst, "rabuka_wasm_heap_highwater");
    cur_fn = wasm_runtime_lookup_function(inst, "rabuka_wasm_heap_cursor");
    rc_bytes_fn =
        wasm_runtime_lookup_function(inst, "rabuka_wasm_heap_recyclable");
    rc_n_fn = wasm_runtime_lookup_function(inst, "rabuka_wasm_heap_entries");
    fps_window_start = timer_ms_gettime64();

    uint32 args[1] = { 0x5EEDu };
    if (!wasm_runtime_call_wasm(env, run, 1, args)) {
        const char *ex = wasm_runtime_get_exception(inst);
        put_line(ex ? ex : "FATAL: call failed", ex ? strnlen(ex, 120) : 18);
        dirty = 1;
        frame();
        for (;;) ;
    }

    const char *res;
    switch (args[0]) {
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
