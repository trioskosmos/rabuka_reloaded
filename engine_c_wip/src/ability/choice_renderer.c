/* choice_renderer.c — port of engine/src/choice_renderer.rs
 *
 * WHAT THIS FILE IS
 * -----------------
 * The Rust twin is the 3DS-style *card choice grid* front-end: one screen with
 * a title, a 5-across image grid with a cursor border, the selected card's
 * identity line, its ability preview (first wrapped line), the skip state, and
 * a two-line hint bar. Select shows the board overlay, L shows the choice hint
 * (+ the source card), R shows the cursor card's detail, Start opens the start
 * menu (stats + zones), D-pad navigation wraps around, and Start is never
 * confirm (A) nor cancel (B).
 *
 * SCOPE / MODEL NOTES (read before reviewing)
 * ------------------------------------------
 * 1. choice_renderer.rs NEVER reads RbChoice. Its option list arrives as the
 *    `items` / `card_nos` slice parameters, built one level up by the caller
 *    (Rust game/menu.rs, not part of this port). The C port keeps the same call
 *    shape -- arrays in, selection index out -- and does NOT invent option
 *    storage on RbChoice. See the INVENTORY at the bottom of this file for
 *    what the *caller* would need.
 *
 * 2. `&mut dyn PlatformUi` has no C equivalent in engine_c_wip yet: there is no
 *    clear_screen / println / swap_buffers / poll_input / just_pressed_* /
 *    draw_card_image / show_detail_screen / show_board_overlay / reset_vram /
 *    open_start_menu anywhere in the tree. The trait is translated here as the
 *    RbRendUi callback table below, with the SAME defaults the Rust trait
 *    declares (an absent callback is a no-op / false, exactly like a port that
 *    does not override a default method). Every `ui.*` call site in the Rust
 *    file maps to exactly one table slot -- no call site was dropped.
 *
 * 3. `Option<usize>` return -> `int`, where -1 is None. sel is always
 *    < total_items and total_items > 0 is enforced before the loop, so -1
 *    cannot collide with a real selection.
 *
 * 4. usize arithmetic that Rust could panic on is unreachable there for the
 *    same reason it is unreachable here: total_items > 0 is checked before the
 *    loop, so every modulo / saturating_sub below has a non-zero divisor and a
 *    non-negative argument. Guards are kept anyway so the C port is total rather
 *    than relying on that invariant being invisible.
 *
 * INVENTORY of Rust arms that cannot be expressed in the C model: see the
 * numbered list at the end of this file. Those are the RbChoice / Card fields
 * the orchestrator must add in rabuka.h before a caller can feed this renderer
 * a real prompt.
 */
#include "rabuka.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stddef.h>
#include <stdint.h>

/* ── Cross-file requests: defined and exported, but NOT yet declared in
 * include/rabuka.h (the orchestrator owns that header). Declared here so this
 * file compiles standalone; the orchestrator should hoist all four into
 * rabuka.h rather than leave a private extern in a .c file.
 *
 *  - rb_queue_get_entry  : src/ability/ability_queue.c (const RbQueueEntry *
 *                          accessor by absolute index; Rust
 *                          AbilityQueue::current_entry()).
 *  - rb_heartmap_len / rb_heartmap_key_at / rb_heartmap_value_at :
 *                          src/core/card.c (HeartMap over a Card's heart slice;
 *                          Rust Card::base_heart / need_heart).
 */
const RbQueueEntry *rb_queue_get_entry(const GameState *g, int index);
int rb_heartmap_len(const Card *c, int from, int len);
int rb_heartmap_key_at(const Card *c, int from, int len, int i);
int rb_heartmap_value_at(const Card *c, int from, int len, int i);

/* ===================================================================== */
/* 1. Layout constants (choice_renderer.rs:23-36)                          */
/* ===================================================================== */

/* Stage-size cards (5x6 tiles) are the largest art that fits a shared grid
 * (the bigger detail portraits each need their own palette). Five across with
 * no gaps, one row per page, keeps the text block + art on one screen. */
#define RB_REND_COLS           5
#define RB_REND_ROWS           1
#define RB_REND_CARDS_PER_PAGE (RB_REND_COLS * RB_REND_ROWS)

/* Card display dimensions (5 tiles wide, 6 tiles tall = stage size). */
#define RB_REND_CARD_TILE_W 5
#define RB_REND_CARD_TILE_H 6
#define RB_REND_CARD_GAP     0
/* Left margin: (30 - 5*5) / 2 centers the gapless row. */
#define RB_REND_GRID_X0 2
/* First tile row of the art grid. The text block above is capped at 6 lines
 * (12 tile rows, 0-11), so punched image holes and corner ticks never eat
 * status lines. */
#define RB_REND_GRID_Y 13

/* Every rendered line is capped at 30 columns so wrapped engine descriptions
 * cannot overflow the budget (choice_renderer.rs:263). This is a literal 30 in
 * the Rust source, NOT ui.option_cols(). */
#define RB_REND_TEXT_COLS 30

/* Unit cap for the text wrapper. A 30-column line never has more than 30 units
 * plus the token that straddles the boundary, so this is far above the real
 * maximum for one `piece` (one hard-break-delimited run). */
#define RB_REND_UNIT_CAP 4096

/* usize::div_ceil for the two call sites (total_pages, rows). Both arguments are
 * provably non-zero in Rust; the guard only keeps the C port total. */
static int rb_rend_div_ceil(int a, int b) {
    if (b <= 0) return 0;
    return a / b + ((a % b) ? 1 : 0);
}

/* ===================================================================== */
/* 2. PlatformUi seam                                                     */
/*    engine/src/game/platform_ui.rs::PlatformUi, restricted to the methods  */
/*    choice_renderer.rs actually calls.                                     */
/* ===================================================================== */

/* One slot per PlatformUi method used here. A NULL slot is the trait's default:
 * no-op for a draw, false for a button query. The five defaults Rust spells out
 * as `false` (just_pressed_l / r / left / right / select) and the two that
 * default to a no-op (reset_vram, open_start_menu, show_board_overlay) collapse
 * to the same NULL test here.
 *
 * `option_cols` is deliberately absent: choice_renderer.rs passes the literal 30
 * to one_line / wrap_text and never reads option_cols().
 */
typedef struct RbRendUi {
    void *ctx;

    /* fn clear_screen(&mut self) */
    void (*clear_screen)(void *ctx);
    /* fn println(&mut self, text: &str) */
    void (*println)(void *ctx, const char *text);
    /* fn swap_buffers(&mut self) */
    void (*swap_buffers)(void *ctx);
    /* fn poll_input(&mut self) */
    void (*poll_input)(void *ctx);
    /* fn wait_vblank(&mut self) */
    void (*wait_vblank)(void *ctx);

    /* fn just_pressed_*(&self) -> bool -- all default false */
    int (*just_pressed_a)(void *ctx);
    int (*just_pressed_b)(void *ctx);
    int (*just_pressed_up)(void *ctx);
    int (*just_pressed_down)(void *ctx);
    int (*just_pressed_start)(void *ctx);
    int (*just_pressed_l)(void *ctx);
    int (*just_pressed_r)(void *ctx);
    int (*just_pressed_left)(void *ctx);
    int (*just_pressed_right)(void *ctx);
    int (*just_pressed_select)(void *ctx);

    /* fn draw_card_image(&mut self, card_no, x, y, cols, rows, palette_index)
     * x/y are in tiles. palette_index is bit flags: bit 0 = cursor,
     * bit 1 = dimmed (greyed, unpickable). */
    void (*draw_card_image)(void *ctx, const char *card_no,
                            int x, int y, int cols, int rows, int palette_index);
    /* fn show_board_overlay(&mut self, gs) */
    void (*show_board_overlay)(void *ctx, const GameState *gs);
    /* fn show_detail_screen(&mut self, gs, art_card_no: Option<&str>,
     *                        header: &[String], body: &str) */
    void (*show_detail_screen)(void *ctx, const GameState *gs,
                               const char *art_card_no,
                               const char *const *header, int n_header,
                               const char *body);
    /* fn reset_vram(&mut self) -- default no-op */
    void (*reset_vram)(void *ctx);
    /* fn open_start_menu(&mut self, gs) -- default no-op */
    void (*open_start_menu)(void *ctx, const GameState *gs);
} RbRendUi;

/* ---- default-method dispatch (NULL == the trait's default) ---- */
static void rb_rend_clear_screen(const RbRendUi *ui) {
    if (ui && ui->clear_screen) ui->clear_screen(ui->ctx);
}
static void rb_rend_println(const RbRendUi *ui, const char *text) {
    if (ui && ui->println) ui->println(ui->ctx, text ? text : "");
}
static void rb_rend_swap_buffers(const RbRendUi *ui) {
    if (ui && ui->swap_buffers) ui->swap_buffers(ui->ctx);
}
static void rb_rend_poll_input(const RbRendUi *ui) {
    if (ui && ui->poll_input) ui->poll_input(ui->ctx);
}
static void rb_rend_wait_vblank(const RbRendUi *ui) {
    if (ui && ui->wait_vblank) ui->wait_vblank(ui->ctx);
}
static void rb_rend_reset_vram(const RbRendUi *ui) {
    if (ui && ui->reset_vram) ui->reset_vram(ui->ctx);
}
/* Button queries: an absent slot is the trait's `false` default. */
static int rb_rend_pressed_a(const RbRendUi *ui) {
    return (ui && ui->just_pressed_a) ? (ui->just_pressed_a(ui->ctx) ? 1 : 0) : 0;
}
static int rb_rend_pressed_b(const RbRendUi *ui) {
    return (ui && ui->just_pressed_b) ? (ui->just_pressed_b(ui->ctx) ? 1 : 0) : 0;
}
static int rb_rend_pressed_up(const RbRendUi *ui) {
    return (ui && ui->just_pressed_up) ? (ui->just_pressed_up(ui->ctx) ? 1 : 0) : 0;
}
static int rb_rend_pressed_down(const RbRendUi *ui) {
    return (ui && ui->just_pressed_down) ? (ui->just_pressed_down(ui->ctx) ? 1 : 0) : 0;
}
static int rb_rend_pressed_start(const RbRendUi *ui) {
    return (ui && ui->just_pressed_start) ? (ui->just_pressed_start(ui->ctx) ? 1 : 0) : 0;
}
static int rb_rend_pressed_l(const RbRendUi *ui) {
    return (ui && ui->just_pressed_l) ? (ui->just_pressed_l(ui->ctx) ? 1 : 0) : 0;
}
static int rb_rend_pressed_r(const RbRendUi *ui) {
    return (ui && ui->just_pressed_r) ? (ui->just_pressed_r(ui->ctx) ? 1 : 0) : 0;
}
static int rb_rend_pressed_left(const RbRendUi *ui) {
    return (ui && ui->just_pressed_left) ? (ui->just_pressed_left(ui->ctx) ? 1 : 0) : 0;
}
static int rb_rend_pressed_right(const RbRendUi *ui) {
    return (ui && ui->just_pressed_right) ? (ui->just_pressed_right(ui->ctx) ? 1 : 0) : 0;
}
static int rb_rend_pressed_select(const RbRendUi *ui) {
    return (ui && ui->just_pressed_select) ? (ui->just_pressed_select(ui->ctx) ? 1 : 0) : 0;
}
static void rb_rend_draw_card_image(const RbRendUi *ui, const char *card_no,
                                    int x, int y, int cols, int rows,
                                    int palette_index) {
    if (ui && ui->draw_card_image)
        ui->draw_card_image(ui->ctx, card_no ? card_no : "", x, y, cols, rows,
                            palette_index);
}
static void rb_rend_show_board_overlay(const RbRendUi *ui, const GameState *gs) {
    if (ui && ui->show_board_overlay) ui->show_board_overlay(ui->ctx, gs);
}
static void rb_rend_open_start_menu(const RbRendUi *ui, const GameState *gs) {
    if (ui && ui->open_start_menu) ui->open_start_menu(ui->ctx, gs);
}
static void rb_rend_show_detail_screen(const RbRendUi *ui, const GameState *gs,
                                       const char *art_card_no,
                                       const char *const *header, int n_header,
                                       const char *body) {
    if (ui && ui->show_detail_screen)
        ui->show_detail_screen(ui->ctx, gs, art_card_no, header, n_header,
                               body ? body : "");
}

/* ===================================================================== */
/* 3. Growable buffer (Rust String / Vec<String> stand-in)                  */
/* ===================================================================== */

typedef struct {
    char  *s;
    size_t len;
    size_t cap;
} RbRendBuf;

static void rb_rend_buf_init(RbRendBuf *b) {
    b->s = NULL;
    b->len = 0;
    b->cap = 0;
}

static void rb_rend_buf_free(RbRendBuf *b) {
    if (!b) return;
    if (b->s) rb_free(b->s);
    rb_rend_buf_init(b);
}

/* Hand the buffer's storage to the caller as a NUL-terminated string and reset
 * the buffer. Rust's `String` moves out of a Vec the same way. Always returns
 * an owned string (possibly ""), never NULL. */
static char *rb_rend_buf_take(RbRendBuf *b) {
    char *s;
    if (!b) return NULL;
    if (!b->s) {
        b->s = (char *)rb_malloc(1);
        if (!b->s) return NULL;
        b->cap = 1;
    }
    b->s[b->len] = '\0';
    s = b->s;
    rb_rend_buf_init(b);
    return s;
}

static void rb_rend_buf_reserve(RbRendBuf *b, size_t extra) {
    size_t need, cap;
    char *ns;
    if (!b) return;
    need = b->len + extra + 1;
    if (need <= b->cap) return;
    cap = b->cap ? b->cap : 64;
    while (cap < need) cap *= 2;
    ns = (char *)rb_malloc(cap);
    if (!ns) return;
    if (b->s && b->len) memcpy(ns, b->s, b->len);
    ns[b->len] = '\0';
    if (b->s) rb_free(b->s);
    b->s = ns;
    b->cap = cap;
}

static void rb_rend_buf_putn(RbRendBuf *b, const char *s, size_t n) {
    if (!b || !s || !n) return;
    rb_rend_buf_reserve(b, n);
    if (!b->s) return;
    memcpy(b->s + b->len, s, n);
    b->len += n;
    b->s[b->len] = '\0';
}

static void rb_rend_buf_puts(RbRendBuf *b, const char *s) {
    if (s) rb_rend_buf_putn(b, s, strlen(s));
}

static void rb_rend_buf_putc(RbRendBuf *b, char c) {
    rb_rend_buf_putn(b, &c, 1);
}

static void rb_rend_buf_puti(RbRendBuf *b, int v) {
    char t[24];
    snprintf(t, sizeof(t), "%d", v);
    rb_rend_buf_puts(b, t);
}

/* An owned "" for the paths that produce no text. */
static char *rb_rend_empty_string(void) {
    char *s = (char *)rb_malloc(1);
    if (s) s[0] = '\0';
    return s;
}

static char *rb_rend_strdup(const char *s) {
    size_t n;
    char *out;
    if (!s) return rb_rend_empty_string();
    n = strlen(s);
    out = (char *)rb_malloc(n + 1);
    if (!out) return NULL;
    memcpy(out, s, n + 1);
    return out;
}

/* String::replace('\n', " ") -- the ability preview collapses hard breaks so the
 * whole run can be re-wrapped at 30 (choice_renderer.rs:336). */
static char *rb_rend_newlines_to_spaces(const char *s) {
    RbRendBuf b;
    rb_rend_buf_init(&b);
    if (!s) return rb_rend_buf_take(&b);
    for (; *s; s++) rb_rend_buf_putc(&b, (*s == '\n') ? ' ' : *s);
    return rb_rend_buf_take(&b);
}

/* Rust's str::trim() strips Unicode whitespace. The text trimmed here (card
 * ability full_text, prompt titles) is ASCII-or-CJK, never leading/trailing
 * Unicode whitespace, so the ASCII set below is the reachable subset -- the
 * same reasoning describe.c's dbuf_trim() records. */
static int rb_rend_is_ws(char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' ||
           c == '\v' || c == '\f';
}

static int rb_rend_trim_is_empty(const char *s) {
    if (!s) return 1;
    while (*s) {
        if (!rb_rend_is_ws(*s)) return 0;
        s++;
    }
    return 1;
}

/* `t.trim()` -> a &str slice. Returns an owned copy sized to the trimmed span,
 * so a long ability text is never silently truncated (which would change what
 * the ability preview wraps). NULL only on allocation failure. */
static char *rb_rend_trim_dup(const char *s) {
    const char *a;
    size_t n;
    char *out;
    if (!s) return rb_rend_empty_string();
    a = s;
    while (*a && rb_rend_is_ws(*a)) a++;
    n = strlen(a);
    while (n > 0 && rb_rend_is_ws(a[n - 1])) n--;
    out = (char *)rb_malloc(n + 1);
    if (!out) return NULL;
    memcpy(out, a, n);
    out[n] = '\0';
    return out;
}

/* ===================================================================== */
/* 4. Text metrics (platform_ui.rs:246-331)                                */
/*    choice_renderer.rs depends on one_line + wrap_text; both are ported     */
/*    here because the C tree has no platform_ui module yet. Every function */
/*    below is a 1:1 translation, {{...}} token handling included.           */
/* ===================================================================== */

/* UTF-8 decode -> scalar value + byte length. Invalid bytes decode as the
 * single byte value with length 1, which keeps the advance total-consistent
 * (matching Rust's chars() over already-valid UTF-8, and degrading safely on
 * the malformed bytes that can appear in a card_no). */
static uint32_t rb_rend_utf8(const char *s, size_t avail, size_t *out_len) {
    const unsigned char *p = (const unsigned char *)s;
    uint32_t cp;
    size_t need, i;

    if (avail == 0) { *out_len = 0; return 0; }
    if (p[0] < 0x80)                    { *out_len = 1; return p[0]; }
    else if ((p[0] & 0xE0) == 0xC0)     { cp = p[0] & 0x1Fu; need = 2; }
    else if ((p[0] & 0xF0) == 0xE0)     { cp = p[0] & 0x0Fu; need = 3; }
    else if ((p[0] & 0xF8) == 0xF0)     { cp = p[0] & 0x07u; need = 4; }
    else                                { *out_len = 1; return p[0]; }

    if (avail < need) { *out_len = 1; return p[0]; }
    for (i = 1; i < need; i++) {
        if ((p[i] & 0xC0) != 0x80) { *out_len = 1; return p[0]; }
        cp = (cp << 6) | (uint32_t)(p[i] & 0x3Fu);
    }
    *out_len = need;
    return cp;
}

/* Width of a single character in half-width columns (CJK glyphs = 2). */
static int rb_rend_char_cols(uint32_t cp) {
    return (cp < 0x1100u) ? 1 : 2;
}

/* Column width estimate for a `{{name|label}}` texticon token's inner part:
 * the label's column count, floored at 2 like a CJK glyph (platform_ui.rs:259).
 * `inner.split('|').next_back()` is the LAST '|'-separated segment. */
static int rb_rend_token_cols(const char *inner, size_t inner_len) {
    size_t start = 0, i = 0;
    int sum = 0;
    for (i = 0; i < inner_len; i++) {
        if (inner[i] == '|') start = i + 1;
    }
    i = start;
    while (i < inner_len) {
        size_t clen = 0;
        uint32_t cp = rb_rend_utf8(inner + i, inner_len - i, &clen);
        if (clen == 0) break;
        sum += rb_rend_char_cols(cp);
        i += clen;
    }
    return sum > 2 ? sum : 2;
}

/* Split `text` into per-char pieces, keeping `{{...}}` texticon tokens as single
 * atomic units so wrapping never cuts a token in half. `len` bounds the scan;
 * the units point into `text`. Returns the unit count (never more than `cap`). */
typedef struct {
    const char *ptr;
    size_t      len;
    int         cols;
} RbRendUnit;

static int rb_rend_text_units(const char *text, size_t len,
                              RbRendUnit *out, int cap) {
    size_t i = 0;
    int k = 0;
    while (i < len) {
        size_t clen = 0;
        uint32_t cp;
        if (text[i] == '{' && i + 1 < len && text[i + 1] == '{') {
            const char *rest = text + i + 2;
            size_t remain = len - (i + 2);
            const char *close = NULL;
            size_t j;
            for (j = 0; j + 1 < remain; j++) {
                if (rest[j] == '}' && rest[j + 1] == '}') { close = rest + j; break; }
            }
            if (close) {
                size_t inner_len = (size_t)(close - rest);
                size_t end = i + 2 + inner_len + 2;
                if (k < cap) {
                    out[k].ptr  = text + i;
                    out[k].len  = end - i;
                    out[k].cols = rb_rend_token_cols(rest, inner_len);
                    k++;
                }
                i = end;
                continue;
            }
        }
        cp = rb_rend_utf8(text + i, len - i, &clen);
        if (clen == 0) break;
        if (k < cap) {
            out[k].ptr  = text + i;
            out[k].len  = clen;
            out[k].cols = rb_rend_char_cols(cp);
            k++;
        }
        i += clen;
    }
    return k;
}

/* Truncate `text` to a single line of at most `cols` columns, marking the cut
 * with ".." so the player knows more is available. A `{{...}}` token that no
 * longer fits ends the line instead of being cut in half.
 * Returns an owned string; free with rb_free. */
static char *rb_rend_one_line(const char *text, int cols) {
    RbRendUnit *units;
    RbRendBuf b;
    int k, i, w = 0;

    rb_rend_buf_init(&b);
    if (!text) return rb_rend_buf_take(&b);

    units = (RbRendUnit *)rb_malloc(sizeof(RbRendUnit) * RB_REND_UNIT_CAP);
    if (!units) return rb_rend_buf_take(&b);
    k = rb_rend_text_units(text, strlen(text), units, RB_REND_UNIT_CAP);
    for (i = 0; i < k; i++) {
        if (w + units[i].cols > cols) {
            rb_rend_buf_puts(&b, "..");
            break;
        }
        rb_rend_buf_putn(&b, units[i].ptr, units[i].len);
        w += units[i].cols;
    }
    rb_free(units);
    return rb_rend_buf_take(&b);
}

static void rb_rend_lines_free(char **lines, int n);

/* Wrap `text` into lines of at most `cols` half-width columns, honouring
 * existing newlines as hard breaks and keeping `{{...}}` tokens intact.
 * Never returns an empty list: Rust pushes one empty line if nothing was
 * emitted (platform_ui.rs:309-311), so `wrap_text(..).into_iter().next()` is
 * always Some and the renderer's `if let Some(line)` always fires.
 * Lines are individually owned; free with rb_rend_lines_free. */
static char **rb_rend_wrap_text(const char *text, int cols, int *out_n) {
    RbRendUnit *units = NULL;
    char **lines = NULL;
    int n = 0, cap = 0;
    RbRendBuf cur;
    int w = 0;
    const char *p;

    if (out_n) *out_n = 0;
    if (!text) text = "";
    rb_rend_buf_init(&cur);

    units = (RbRendUnit *)rb_malloc(sizeof(RbRendUnit) * RB_REND_UNIT_CAP);
    if (!units) {
        rb_rend_buf_free(&cur);
        return NULL;
    }

/* Move the pending line into `lines`, exactly like core::mem::take + push. */
#define RB_REND_PUSH_LINE()                                              \
    do {                                                                 \
        char *taken;                                                     \
        if (n == cap) {                                                  \
            int ncap = cap ? cap * 2 : 8;                                \
            char **nl = (char **)rb_malloc((size_t)ncap * sizeof(char *)); \
            if (!nl) { rb_rend_buf_free(&cur); rb_free(units);          \
                       rb_rend_lines_free(lines, n); return NULL; }      \
            if (lines && n > 0)                                          \
                memcpy(nl, lines, (size_t)n * sizeof(char *));           \
            if (lines) rb_free(lines);                                   \
            lines = nl;                                                  \
            cap = ncap;                                                  \
        }                                                                 \
        taken = rb_rend_buf_take(&cur);                                  \
        if (taken) lines[n++] = taken;                                   \
    } while (0)

    /* for piece in text.split('\n') */
    p = text;
    for (;;) {
        const char *nl_at = strchr(p, '\n');
        size_t piece_len = nl_at ? (size_t)(nl_at - p) : strlen(p);
        int k = rb_rend_text_units(p, piece_len, units, RB_REND_UNIT_CAP);
        int i;
        for (i = 0; i < k; i++) {
            if (w + units[i].cols > cols && cur.len > 0) {
                RB_REND_PUSH_LINE();
                w = 0;
            }
            rb_rend_buf_putn(&cur, units[i].ptr, units[i].len);
            w += units[i].cols;
        }
        if (cur.len > 0) RB_REND_PUSH_LINE();
        w = 0;
        if (!nl_at) break;
        p = nl_at + 1;
    }

    /* out.is_empty() -> out.push(String::new()) */
    if (n == 0) RB_REND_PUSH_LINE();

#undef RB_REND_PUSH_LINE

    rb_rend_buf_free(&cur);
    rb_free(units);
    if (out_n) *out_n = n;
    return lines;
}

static void rb_rend_lines_free(char **lines, int n) {
    int i;
    if (!lines) return;
    for (i = 0; i < n; i++) if (lines[i]) rb_free(lines[i]);
    rb_free(lines);
}

/* ===================================================================== */
/* 5. Card display text (platform_ui.rs:351-487, non-gba variants)          */
/*    The C tree is not a GBA build, so the #[cfg(not(feature = "gba"))]     */
/*    arms are the faithful ones: "E3  ", "SC5  ", "BL2", "h013 h022".       */
/* ===================================================================== */

/* HeartColor::short_label (core/card.rs heart_color_table!). The index is the
 * HeartColor discriminant: 0..6 = heart00..heart06, 7 = BAll, 8 = Draw,
 * 9 = Score, 10 = All. */
static const char *rb_rend_heart_short_label(int color) {
    switch (color) {
        case 0:  return "h00";
        case 1:  return "h01";
        case 2:  return "h02";
        case 3:  return "h03";
        case 4:  return "h04";
        case 5:  return "h05";
        case 6:  return "h06";
        case 7:  return "b_all";
        case 8:  return "draw";
        case 9:  return "score";
        case 10: return "all";
        default: return "";
    }
}

/* Format a heart map: short_label+count per entry, joined by a space. A
 * HeartMap over a Card is the half-open slice [from, from+len) of the stored
 * heart_color / heart_count arrays (core/card.rs:170-223). */
static char *rb_rend_heart_str(const Card *c, int from, int len) {
    RbRendBuf b;
    int i, n = rb_heartmap_len(c, from, len);

    rb_rend_buf_init(&b);
    for (i = 0; i < n; i++) {
        int color = rb_heartmap_key_at(c, from, len, i);
        int value = rb_heartmap_value_at(c, from, len, i);
        if (i > 0) rb_rend_buf_putc(&b, ' ');
        rb_rend_buf_puts(&b, rb_rend_heart_short_label(color));
        rb_rend_buf_puti(&b, value);
    }
    return rb_rend_buf_take(&b);
}

/* Single-line stat summary, non-gba variant (platform_ui.rs:420). */
static char *rb_rend_card_stat_text(const Card *c) {
    RbRendBuf b;
    int type;

    if (!c) return rb_rend_empty_string();
    rb_rend_buf_init(&b);
    /* card_type_of() in core/card.c: the low two bits, with 3 folding to 0. */
    type = (int)(c->type_flags & 0x03u);
    if (type == 3) type = 0;

    if (type == 0) {                                        /* CardType::Member */
        if (c->cost > 0) {
            rb_rend_buf_putc(&b, 'E');
            rb_rend_buf_puti(&b, c->cost);
            rb_rend_buf_puts(&b, "  ");
        }
        {
            char *hs = rb_rend_heart_str(c, 0, c->num_base);
            if (hs && hs[0]) {
                rb_rend_buf_puts(&b, hs);
                rb_rend_buf_puts(&b, "  ");
            }
            if (hs) rb_free(hs);
        }
        if (c->blade > 0) {
            rb_rend_buf_puts(&b, "BL");
            rb_rend_buf_puti(&b, c->blade);
        }
    } else if (type == 1) {                                 /* CardType::Live */
        if (c->score > 0) {
            rb_rend_buf_puts(&b, "SC");
            rb_rend_buf_puti(&b, c->score);
            rb_rend_buf_puts(&b, "  ");
        }
        {
            int from = c->num_base + c->num_blade;
            char *ns = rb_rend_heart_str(c, from, c->num_need);
            if (ns && ns[0]) rb_rend_buf_puts(&b, ns);
            if (ns) rb_free(ns);
        }
    }
    /* CardType::Energy -> String::new() */
    return rb_rend_buf_take(&b);
}

/* Detail-screen title line: "[card_no] name" (platform_ui.rs:467). The C Card
 * exposes card_no as the string-table index `card_no_idx`. */
static char *rb_rend_card_detail_title(const Card *c) {
    RbRendBuf b;
    if (!c) return rb_rend_empty_string();
    rb_rend_buf_init(&b);
    rb_rend_buf_putc(&b, '[');
    rb_rend_buf_puts(&b, rb_card_string(c->card_no_idx));
    rb_rend_buf_putc(&b, ']');
    rb_rend_buf_putc(&b, ' ');
    rb_rend_buf_puts(&b, c->name ? c->name : "");
    return rb_rend_buf_take(&b);
}

/* A card's ability text: the trimmed full_text of every resolved ability, joined
 * with "\n" (platform_ui.rs:474). "" when the card has no abilities.
 * The C Card leaves `ability` NULL until lazily decoded, so the abilities are
 * resolved through the by-index decoder here. Free with rb_free. */
static char *rb_rend_card_ability_text(const Card *c) {
    RbRendBuf b;
    int id, i, n, first = 1;

    if (!c) return rb_rend_empty_string();
    id = rb_card_get_card_id(rb_card_string(c->card_no_idx));
    if (id < 0) return rb_rend_empty_string();

    rb_rend_buf_init(&b);
    n = rb_card_num_abilities((uint32_t)id);
    for (i = 0; i < n; i++) {
        Ability ab;
        char *trimmed;
        memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)id, i, &ab)) continue;
        trimmed = rb_rend_trim_dup(ab.full_text);
        rb_free_ability(&ab);
        if (!trimmed) continue;
        if (trimmed[0]) {
            if (!first) rb_rend_buf_putc(&b, '\n');
            rb_rend_buf_puts(&b, trimmed);
            first = 0;
        }
        rb_free(trimmed);
    }
    if (first) {
        /* parts.is_empty() -> card.ability_text().trim(). The C Card struct has
         * NO raw-ability-text field, so this fallback has no data source and
         * yields "" (INVENTORY item 8). */
        rb_rend_buf_free(&b);
        return rb_rend_empty_string();
    }
    return rb_rend_buf_take(&b);
}

/* ===================================================================== */
/* 6. CardInfo (choice_renderer.rs:39-43)                                   */
/* ===================================================================== */

typedef struct {
    const char *card_no;
    const char *name;
    char       *ability_text;  /* owned */
} RbRendCardInfo;

/* Build the card-info list: for each (item, card_no) pair, look the card up and
 * take its ability text, defaulting to "" when the card_no is unknown
 * (`.map(card_ability_text).unwrap_or_default()`).
 * Returns the number of entries written into `out`. */
static int rb_rend_card_info_build(const char *const *items, int n_items,
                                   const char *const *card_nos, int n_card_nos,
                                   RbRendCardInfo *out, int cap) {
    int n = n_items < n_card_nos ? n_items : n_card_nos;   /* items.iter().zip() */
    int i;

    if (n > cap) n = cap;
    for (i = 0; i < n; i++) {
        Card c;
        out[i].card_no = card_nos[i];
        out[i].name = items[i];
        out[i].ability_text = NULL;
        if (rb_card_get_card_by_no(card_nos[i], &c)) {
            out[i].ability_text = rb_rend_card_ability_text(&c);
            rb_free_card(&c);
        } else {
            out[i].ability_text = rb_rend_empty_string();
        }
    }
    return n;
}

static void rb_rend_card_info_free(RbRendCardInfo *info, int n) {
    int i;
    if (!info) return;
    for (i = 0; i < n; i++) {
        if (info[i].ability_text) { rb_free(info[i].ability_text); info[i].ability_text = NULL; }
    }
}

/* `dimmed.and_then(|d| d.get(idx).copied()).unwrap_or(false)` -- the dimmed
 * slice is optional and may be shorter than the item list. */
static int rb_rend_is_dimmed(const unsigned char *dimmed, int n_dimmed, int idx) {
    if (!dimmed || idx < 0 || idx >= n_dimmed) return 0;
    return dimmed[idx] ? 1 : 0;
}

/* ===================================================================== */
/* 7. render_choice_page (choice_renderer.rs:265-362)                       */
/* ===================================================================== */

/* Render a single page of the card choice grid.
 *
 * Text budget is 6 lines (12 tile rows); art sits at tile row 12+ so the
 * punched image holes never eat the status lines. Every line is capped at 30
 * columns so wrapped engine descriptions cannot overflow the budget.
 *
 * `skip_idx` is Option<usize>: pass -1 for None.
 */
static void rb_rend_render_choice_page(const RbRendUi *ui,
                                       const RbRendCardInfo *cards, int n_cards,
                                       int page_start, int page_end, int sel,
                                       const char *title, int allow_skip,
                                       int skip_idx, int total_pages,
                                       const unsigned char *dimmed, int n_dimmed) {
    int i, cards_to_draw, page_no, on_skip;
    char *line;

    rb_rend_clear_screen(ui);

    /* Title, with the page counter only when the grid actually paginates. */
    page_no = page_start / RB_REND_CARDS_PER_PAGE + 1;
    if (total_pages > 1) {
        RbRendBuf b;
        int shown = total_pages > 1 ? total_pages : 1;   /* total_pages.max(1) */
        rb_rend_buf_init(&b);
        rb_rend_buf_puts(&b, title ? title : "");
        rb_rend_buf_putc(&b, ' ');
        rb_rend_buf_putc(&b, '[');
        rb_rend_buf_puti(&b, page_no);
        rb_rend_buf_putc(&b, '/');
        rb_rend_buf_puti(&b, shown);
        rb_rend_buf_putc(&b, ']');
        line = rb_rend_one_line(b.s ? b.s : "", RB_REND_TEXT_COLS);
        rb_rend_buf_free(&b);
    } else {
        line = rb_rend_one_line(title ? title : "", RB_REND_TEXT_COLS);
    }
    rb_rend_println(ui, line);
    if (line) rb_free(line);

    /* (page_end - page_start).min(cards.len().saturating_sub(page_start)) */
    cards_to_draw = page_end - page_start;
    {
        int avail = n_cards - page_start;   /* saturating_sub */
        if (avail < 0) avail = 0;
        if (cards_to_draw > avail) cards_to_draw = avail;
        if (cards_to_draw < 0) cards_to_draw = 0;
    }

    /* Draw card art in a grid (coordinates are in tiles, matching the
     * draw_card_image contract). palette_index is bit flags: bit 0 = cursor,
     * bit 1 = dimmed (greyed, unpickable) -- the 3DS `disabled` overlay. */
    for (i = 0; i < cards_to_draw; i++) {
        int idx = page_start + i;
        int row = i / RB_REND_COLS;
        int col = i % RB_REND_COLS;
        int is_selected = (page_start + i) == sel;
        int is_dim = rb_rend_is_dimmed(dimmed, n_dimmed, idx);

        /* Card position in tiles (gapless row, centered via GRID_X0). */
        int x = RB_REND_GRID_X0 + col * (RB_REND_CARD_TILE_W + RB_REND_CARD_GAP);
        int y = RB_REND_GRID_Y + row * (RB_REND_CARD_TILE_H + RB_REND_CARD_GAP);

        int flag = 0;
        if (is_selected) flag |= 1;
        if (is_dim)     flag |= 2;
        rb_rend_draw_card_image(ui, cards[idx].card_no, x, y,
                                RB_REND_CARD_TILE_W, RB_REND_CARD_TILE_H, flag);
    }

    /* Selected card identity (card_no + name, like the 3DS label row). Dimmed
     * cursors are marked in text too, so text-only ports (whose draw_card_image
     * is a no-op) still show the unpickable state. */
    on_skip = (skip_idx >= 0 && skip_idx == sel);
    if (on_skip) {
        rb_rend_println(ui, "> [Skip]");
    } else if (sel < n_cards) {
        RbRendBuf b;
        rb_rend_buf_init(&b);
        rb_rend_buf_puts(&b, "> [");
        rb_rend_buf_puts(&b, cards[sel].card_no ? cards[sel].card_no : "");
        rb_rend_buf_puts(&b, "] ");
        rb_rend_buf_puts(&b, cards[sel].name ? cards[sel].name : "");
        if (rb_rend_is_dimmed(dimmed, n_dimmed, sel)) rb_rend_buf_puts(&b, " --");
        line = rb_rend_one_line(b.s ? b.s : "", RB_REND_TEXT_COLS);
        rb_rend_buf_free(&b);
        rb_rend_println(ui, line);
        if (line) rb_free(line);
    }

    /* Ability preview of the selected card (3DS ability banner equivalent, first
     * wrapped line so the text budget holds). */
    if (!on_skip && sel < n_cards) {
        char *abil = rb_rend_newlines_to_spaces(cards[sel].ability_text);
        if (!rb_rend_trim_is_empty(abil)) {
            char **lines;
            int n_lines = 0;
            lines = rb_rend_wrap_text(abil, RB_REND_TEXT_COLS, &n_lines);
            /* wrap_text never returns an empty list, so `if let Some(line)` of
             * `.into_iter().next()` always yields the first line. */
            if (lines && n_lines > 0 && lines[0]) rb_rend_println(ui, lines[0]);
            rb_rend_lines_free(lines, n_lines);
        }
        if (abil) rb_free(abil);
    }

    /* Skip row when it sits on this page but the cursor is elsewhere. */
    if (allow_skip && skip_idx >= 0) {
        if (!on_skip && skip_idx >= page_start && skip_idx < page_end)
            rb_rend_println(ui, "  [Skip]");
    }

    /* Hint bar: A picks, B goes back (or skips) when the prompt allows it,
     * Select shows the board, Start opens the menu (choice resumes), L/R pop
     * card detail screens. */
    if (allow_skip) rb_rend_println(ui, "A:Pick B:Skip SL:Board");
    else            rb_rend_println(ui, "A:Pick SL:Board");
    rb_rend_println(ui, "L:Hint R:Card Sta:Menu");
}

/* ===================================================================== */
/* 8. render_card_choice_grid (choice_renderer.rs:48-257)                   */
/* ===================================================================== */

/* Renders a card choice grid (3DS style) with board overlay support.
 * `dimmed` (1:1 with `items`) marks unpickable options: drawn greyed and
 * A on them is ignored.
 *
 * Returns the selected index, or -1 if cancelled (Rust: None).
 *
 * `items` / `card_nos` are the caller's option list (Rust passes them as
 * slices); `n_items` / `n_card_nos` their lengths -- the Rust zip stops at the
 * shorter of the two. `dimmed` may be NULL.
 */
int rb_rend_render_card_choice_grid(const RbRendUi *ui, const GameState *gs,
                                    const char *title,
                                    const char *const *items, int n_items,
                                    const char *const *card_nos, int n_card_nos,
                                    int allow_skip,
                                    const unsigned char *dimmed, int n_dimmed) {
    RbRendCardInfo *cards = NULL;
    int n_cards = 0;
    int skip_idx;          /* Option<usize>: -1 == None */
    int total_items, total_pages;
    char *source_card_no = NULL;
    int have_source = 0;
    int sel = 0;

    if (!items || !card_nos || n_items < 0 || n_card_nos < 0) return -1;

    /* Build card info list. */
    n_cards = n_items < n_card_nos ? n_items : n_card_nos;
    if (n_cards > 0) {
        cards = (RbRendCardInfo *)rb_malloc(sizeof(RbRendCardInfo) * (size_t)n_cards);
        if (!cards) return -1;
        n_cards = rb_rend_card_info_build(items, n_items, card_nos, n_card_nos,
                                          cards, n_cards);
    }

    /* Add skip option if allowed. */
    skip_idx = allow_skip ? n_items : -1;

    total_items = n_items + (allow_skip ? 1 : 0);
    if (total_items == 0) {
        rb_rend_card_info_free(cards, n_cards);
        if (cards) rb_free(cards);
        return -1;
    }
    total_pages = rb_rend_div_ceil(total_items, RB_REND_CARDS_PER_PAGE);

    /* Card number of the ability currently being resolved (L shows its detail
     * screen). Cloned up front so the input loop stays simple. */
    if (gs) {
        int cur = rb_queue_current_entry(gs);
        const RbQueueEntry *entry = rb_queue_get_entry(gs, cur);
        if (entry) {
            /* Rust's `entry.card_no` is a String; the C RbQueueEntry keeps the
             * card as an index, so the number is read back off the database. */
            Card c;
            if (rb_decode_card_by_index((uint32_t)entry->card_id, &c)) {
                source_card_no = rb_rend_strdup(rb_card_string(c.card_no_idx));
                rb_free_card(&c);
            }
            /* .filter(|n| !n.is_empty()) */
            if (source_card_no) {
                if (source_card_no[0] == '\0') {
                    rb_free(source_card_no);
                    source_card_no = NULL;
                } else {
                    have_source = 1;
                }
            }
        }
    }

    /* Fresh pool for the grid art + text: the previous screen's dead tiles
     * would otherwise pile onto this screen's demand (see reset_vram). */
    rb_rend_reset_vram(ui);

    for (;;) {
        int page = sel / RB_REND_CARDS_PER_PAGE;
        int page_start = page * RB_REND_CARDS_PER_PAGE;
        int page_end = page_start + RB_REND_CARDS_PER_PAGE;
        if (page_end > total_items) page_end = total_items;

        rb_rend_render_choice_page(ui, cards, n_cards, page_start, page_end, sel,
                                   title, allow_skip, skip_idx, total_pages,
                                   dimmed, n_dimmed);

        rb_rend_swap_buffers(ui);

        /* Poll input. */
        rb_rend_poll_input(ui);

        if (rb_rend_pressed_select(ui)) {
            /* Board overlay on Select. */
            rb_rend_show_board_overlay(ui, gs);
            /* Wait for any button to dismiss. */
            for (;;) {
                rb_rend_poll_input(ui);
                if (rb_rend_pressed_a(ui) || rb_rend_pressed_b(ui) ||
                    rb_rend_pressed_l(ui) || rb_rend_pressed_r(ui) ||
                    rb_rend_pressed_select(ui) || rb_rend_pressed_start(ui)) {
                    break;
                }
                rb_rend_wait_vblank(ui);
            }
            /* Redraw the choice page after overlay. */
            rb_rend_render_choice_page(ui, cards, n_cards, page_start, page_end,
                                       sel, title, allow_skip, skip_idx,
                                       total_pages, dimmed, n_dimmed);
            rb_rend_swap_buffers(ui);
} else if (rb_rend_pressed_l(ui)) {
            /* Choice hint detail: the full prompt text plus the source card's
             * ability, composed by the port's detail renderer. Works even with
             * no source card. */
            Card c;
            int have_card = have_source && source_card_no &&
                            rb_card_get_card_by_no(source_card_no, &c);
            if (have_card) {
                const char *header[1];
                char *detail_title = rb_rend_card_detail_title(&c);
                char *ability = rb_rend_card_ability_text(&c);
                header[0] = detail_title;
                if (rb_rend_trim_is_empty(ability)) {
                    /* ab.trim().is_empty() -> body is the bare title. */
                    rb_rend_show_detail_screen(ui, gs, source_card_no,
                                               header, 1, title ? title : "");
                } else {
                    RbRendBuf b;
                    rb_rend_buf_init(&b);
                    rb_rend_buf_puts(&b, title ? title : "");
                    rb_rend_buf_puts(&b, "\n\n");
                    rb_rend_buf_puts(&b, ability);
                    rb_rend_show_detail_screen(ui, gs, source_card_no,
                                               header, 1, b.s ? b.s : "");
                    rb_rend_buf_free(&b);
                }
                if (detail_title) rb_free(detail_title);
                if (ability) rb_free(ability);
                rb_free_card(&c);
            } else {
                /* src is None -> header is empty and the body is the title. */
                rb_rend_show_detail_screen(ui, gs, NULL, NULL, 0,
                                           title ? title : "");
            }
        } else if (rb_rend_pressed_r(ui)) {
            /* Detail of the cursor card (stats + ability). */
            if (sel < n_cards && cards[sel].card_no && cards[sel].card_no[0]) {
                Card c;
                if (rb_card_get_card_by_no(cards[sel].card_no, &c)) {
                    const char *header[2];
                    char *detail_title = rb_rend_card_detail_title(&c);
                    char *stat_text = rb_rend_card_stat_text(&c);
                    char *ability = rb_rend_card_ability_text(&c);
                    header[0] = detail_title;
                    header[1] = stat_text;
                    rb_rend_show_detail_screen(ui, gs, cards[sel].card_no,
                                               header, 2, ability ? ability : "");
                    if (detail_title) rb_free(detail_title);
                    if (stat_text) rb_free(stat_text);
                    if (ability) rb_free(ability);
                    rb_free_card(&c);
                }
            }
        } else if (rb_rend_pressed_a(ui)) {
            /* Dimmed cards can't be picked -- ignore the press. */
            if (skip_idx != sel && rb_rend_is_dimmed(dimmed, n_dimmed, sel)) {
                rb_rend_wait_vblank(ui);
            } else {
                /* Release the grid before the caller rebuilds its screen. */
                rb_rend_reset_vram(ui);
                if (skip_idx == sel) {
                    rb_rend_card_info_free(cards, n_cards);
                    if (cards) rb_free(cards);
                    if (source_card_no) rb_free(source_card_no);
                    return -1;
                }
                {
                    int picked = sel;
                    rb_rend_card_info_free(cards, n_cards);
                    if (cards) rb_free(cards);
                    if (source_card_no) rb_free(source_card_no);
                    return picked;
                }
            }
        } else if (rb_rend_pressed_b(ui)) {
            /* B backs out exactly like the [Skip] row: only when skipping is
             * allowed. On mandatory prompts the press is ignored -- an empty
             * answer is rejected by resume_with_choice, so answering None would
             * just re-prompt and look like a dead button. */
            if (allow_skip) {
                rb_rend_reset_vram(ui);
                rb_rend_card_info_free(cards, n_cards);
                if (cards) rb_free(cards);
                if (source_card_no) rb_free(source_card_no);
                return -1;
            }
        } else if (rb_rend_pressed_start(ui)) {
            /* Start menu over the choice (stats + zones); the choice resumes
             * when it closes. */
            rb_rend_open_start_menu(ui, gs);
            /* Redraw the choice page after the menu. */
            rb_rend_render_choice_page(ui, cards, n_cards, page_start, page_end,
                                       sel, title, allow_skip, skip_idx,
                                       total_pages, dimmed, n_dimmed);
            rb_rend_swap_buffers(ui);
        } else if (rb_rend_pressed_left(ui)) {
            sel = (sel + total_items - 1) % total_items;
        } else if (rb_rend_pressed_right(ui)) {
            sel = (sel + 1) % total_items;
        } else if (rb_rend_pressed_up(ui) || rb_rend_pressed_down(ui)) {
            /* Column-preserving vertical wrap: Up from the top row lands on the
             * bottom row in the same column and vice versa. A short last row
             * clamps onto its final cell. */
            int up = rb_rend_pressed_up(ui);
            int col = sel % RB_REND_COLS;
            int rows = rb_rend_div_ceil(total_items, RB_REND_COLS);
            int row = sel / RB_REND_COLS;
            int new_row;
            if (rows <= 0) {
                sel = 0;
            } else {
                new_row = up ? ((row + rows - 1) % rows)
                             : ((row + 1) % rows);
                sel = new_row * RB_REND_COLS + col;
                if (sel > total_items - 1) sel = total_items - 1;
            }
        } else {
            rb_rend_wait_vblank(ui);
        }
        /* NOTE: Start opens the menu above (handled in its own branch) -- it
         * never confirms (A) nor cancels (B) a choice. */
    }
}

/* ===================================================================== */
/* 9. INVENTORY -- Rust renderer arms vs. the current C model                */
/* ===================================================================== */
/*
 * choice_renderer.rs takes its option list as PARAMETERS, so no arm of the
 * renderer itself is blocked by RbChoice's missing option list. What is
 * missing is the layer that CALLS it (Rust game/menu.rs, unported) and the
 * PlatformUi backend. Each item below names what would have to exist in
 * rabuka.h for that caller's arm to be expressible.
 *
 *  REACHABLE-IN-C (implemented above, no new field needed)
 *   - title line, paged and unpaged                          (page counter)
 *   - card art grid: x/y placement, 5x6 tiles, gapless row
 *   - palette flag bits: bit0 cursor, bit1 dimmed
 *   - selected-card identity line + the " --" dim marker
 *   - ability preview (first wrapped line, newlines collapsed)
 *   - the [Skip] row, both under the cursor and as a page row
 *   - hint bar, both the skippable and mandatory variants
 *   - cursor navigation: left/right wrap, column-preserving up/down + clamp
 *   - A on a dimmed option ignored; A/B on skip returning None
 *   - L hint composition (title + "\n\n" + ability; bare title when the
 *     source card is unknown or has no ability text)
 *   - R detail composition (detail title + stat text header, ability body)
 *   - skip_idx = items.len(), total_items, total_pages, cards_to_draw clamp
 *   - one_line / wrap_text / {{token}} column accounting
 *   - heart short labels, member/live stat summary, "[card_no] name"
 *   - source card number of the ability being resolved (queue entry card_id)
 *
 *  NEEDS-A-NEW-FIELD (not expressible today)
 *   1. Option list for a card prompt. Rust passes `items` / `card_nos` slices
 *      built from the pending Choice. RbChoice has neither.
 *      NEED: `char options[RB_MAX_CC_STRINGS][64]; int n_options;`
 *            (mirroring RbFullChoice.options / .hc_options, which already exist
 *             on RbFullChoice but are dropped by rb_full_choice_to_header()).
 *   2. Card numbers per option, for draw_card_image and the R detail lookup.
 *      NEED: `char option_card_nos[RB_MAX_CC_STRINGS][32]; int n_option_card_nos;`
 *   3. The dimmed/unpickable mask (Rust `Option<&[bool]>`, 1:1 with items).
 *      NEED: `unsigned char option_dimmed[RB_MAX_CC_STRINGS]; int n_option_dimmed;`
 *   4. A UI backend. None of clear_screen / println / swap_buffers / poll_input /
 *      just_pressed_* / draw_card_image / show_board_overlay /
 *      show_detail_screen / reset_vram / open_start_menu exists in engine_c_wip.
 *      NEED: the RbRendUi struct + its accessors declared in rabuka.h, and a
 *      concrete backend per front-end.
 *   5. Prompt text. The caller's `title` is the Choice's description, but
 *      RbChoice.description is a single 128-byte field with no width/collation
 *      rule and no i18n selection, while the renderer needs `title` to survive
 *      one_line(30) and a "\n\n"-joined detail body.
 *      NEED: `rb_choice_prompt_text(const RbChoice*, int lang)` returning an
 *            owned string, resolving description_ja vs description_en.
 *   6. Localized hint bar. The two hint strings are hardcoded ASCII here, but
 *      Rust's PlatformUi::ui_lang() picks Japanese wording per port.
 *      NEED: `rb_rend_hint_line(int allow_skip, int line, int lang)` plus a Lang
 *            accessor, so the strings are not duplicated per port.
 *   7. ".." cut marker and scroll hints ("N more"). platform_ui.rs's
 *      more_line() / scroll_hint() are language-dependent; neither exists in C.
 *      NEED: the same Lang-dependent line helpers.
 *   8. `card.ability_text()` fallback. Card has no raw-ability-text field (only
 *      the lazily decoded `Ability *ability`), so when a card has zero decoded
 *      abilities the Rust fallback text is unreachable.
 *      NEED: `char ability_text_raw[N]` on Card, populated by
 *            rb_decode_card_by_index (the `compact_cards` field).
 *   9. GBA texticon rendering. `heart_icon_for` / the gba build of heart_str and
 *      card_stat_text emit `{{icon}}` markers. Out of scope for the non-GBA C
 *      port; the width accounting for those tokens IS ported (one_line /
 *      wrap_text), so a GBA front-end can render them.
 *   10. Multi-page paging. `total_pages > 1` needs more than 5 options; with
 *       n_options capped at RB_MAX_CC_STRINGS this is reachable, but with the
 *       current 8-slot heart_options it is not.
 */