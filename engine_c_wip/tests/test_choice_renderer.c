/* test_choice_renderer.c — exercises src/ability/choice_renderer.c.
 *
 * WHY THIS SUITE EXISTS
 * =====================
 * choice_renderer.c was in NO build target from its creation until 2026-10-03
 * (Makefile SRC/OBJ_LIB, the two platforms build_cdi.sh / build_genesis.sh, and
 * platforms/Makefile: zero hits). 1821 of its 1964 lines were compiled into
 * nothing and no compiler had ever seen them, so "it is a faithful port" was an
 * assertion in a comment, not a measurement. Adding it to the build makes it
 * compile and link; this suite is what makes it RUN.
 *
 * The Rust twin (engine/src/choice_renderer.rs, 362 lines) is the 3DS card-choice
 * grid: title, 5-across art grid with a cursor border, the selected card's
 * identity line, its first wrapped ability line, the [Skip] row, and a two-line
 * hint bar. It draws through the `PlatformUi` trait. The C translates the trait
 * as a NULL-able callback table with Rust's own defaults (absent = no-op /
 * false), and ships a TEXT-ONLY backend (`rb_rend_text_new` +
 * `rb_rend_text_script`) that captures every screen and replays a scripted
 * button sequence, so the grid renders headlessly and the run TERMINATES.
 *
 * WHAT IS PINNED HERE. Every assertion is derived from choice_renderer.rs, not
 * from the C: layout constants, the per-frame line set, the palette flag bits,
 * the navigation algebra (including the two wraps and the column-preserving
 * vertical move), the skip semantics, the dimmed-A refusal, and the two
 * terminate-early cases the C added because Rust has a console to hang on.
 *
 * The 10 arms the C model cannot express are enumerated in choice_renderer.c's
 * closing INVENTORY; §H below asserts that list has not grown — i.e. that a
 * rabuka.h widening lands here rather than silently in the renderer.
 */
#include "rabuka.h"

#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", (message)); \
        failures++; \
    } else { \
        printf("ok: %s\n", (message)); \
    } \
} while (0)

/* ── cross-file declarations ──────────────────────────────────────────────
 * None of the renderer/backend symbols are in include/rabuka.h (that header is
 * not choice_renderer.c's to edit); the file declares them itself "for an
 * out-of-tree caller / test harness to use", which is this. Mirrored here.
 * Layout constants are the literals at choice_renderer.rs:23-36 and
 * choice_renderer.c:144-162. */
#define REND_COLS        5
#define REND_CARDS_PAGE  5
#define REND_TILE_W      5
#define REND_TILE_H      6
#define REND_GRID_X0     2
#define REND_GRID_Y      13
#define REND_TEXT_COLS   30

enum {
    BTN_NONE = 0, BTN_A, BTN_B, BTN_L, BTN_R,
    BTN_UP, BTN_DOWN, BTN_LEFT, BTN_RIGHT, BTN_START, BTN_SELECT
};

typedef struct RbRendText  RbRendText;

/* An out-of-tree caller has to be able to STACK-allocate this, so the layout
 * is repeated here. It MUST stay byte-identical to RbRendItems at
 * choice_renderer.c:1748-1755 — a drift here is memory corruption, not a
 * compile error. The G-checks below catch it loudly: rb_rend_items_init must
 * leave every field zero, and rb_rend_items_from_choice must be able to fill
 * and free the arrays through this pointer. The real fix is item 1 of the
 * renderer's own INVENTORY: hoisting these symbols into include/rabuka.h. */
typedef struct RbRendItems {
    char         **items;
    char         **card_nos;
    unsigned char *dimmed;
    int           *card_ids;
    int            n;
    int            cap;
} RbRendItems;

RbRendText        *rb_rend_text_new(FILE *sink, int echo_lines);
void               rb_rend_text_free(RbRendText *t);
const RbRendUi    *rb_rend_text_ui(RbRendText *t);
/* Opaque here: the full PlatformUi table is defined at choice_renderer.c:199 and
 * is NOT in include/rabuka.h — inventory item 4 says hoisting it (plus a `void
 * *ctx` and the two missing button slots) is what would remove the ext. */
typedef struct RbRendUiExt RbRendUiExt;
const RbRendUiExt *rb_rend_text_ext(RbRendText *t);
int  rb_rend_render_card_choice_grid_ex(const RbRendUi *ui, const RbRendUiExt *ext,
                                        const GameState *gs, const char *title,
                                        const char *const *items, int n_items,
                                        const char *const *card_nos, int n_card_nos,
                                        int allow_skip, const unsigned char *dimmed, int n_dimmed);
int                rb_rend_text_script(RbRendText *t, const char *const *btns, int n);
void               rb_rend_text_idle(RbRendText *t, const char *btn);
int                rb_rend_text_stuck(const RbRendText *t);
int                rb_rend_text_n_presses(const RbRendText *t);
int                rb_rend_text_n_frames(const RbRendText *t);
int                rb_rend_text_n_lines(const RbRendText *t);
int                rb_rend_text_n_draws(const RbRendText *t);
int                rb_rend_text_overflowed(const RbRendText *t);
const char        *rb_rend_text_line(const RbRendText *t, int i);
int                rb_rend_text_frame_line(const RbRendText *t, int f, int i, const char **out);
int                rb_rend_text_draw(const RbRendText *t, int i, const char **card_no,
                                    int *x, int *y, int *cols, int *rows, int *palette_index);
int                rb_rend_text_frame_draws(const RbRendText *t, int f);

void               rb_rend_items_init(RbRendItems *it);
void               rb_rend_items_free(RbRendItems *it);
int                rb_rend_items_from_choice(RbRendItems *it, const RbChoice *ch,
                                              const GameState *gs, int max_items);
const char *const *rb_rend_items(const RbRendItems *it);
const char *const *rb_rend_card_nos(const RbRendItems *it);
int                rb_rend_render_choice(RbRendText *text, const RbChoice *ch,
                                         const GameState *gs, int max_items);

int  rb_rend_render_card_choice_grid(const RbRendUi *ui, const GameState *gs,
                                     const char *title,
                                     const char *const *items, int n_items,
                                     const char *const *card_nos, int n_card_nos,
                                     int allow_skip, const unsigned char *dimmed, int n_dimmed);

/* ── fixtures ─────────────────────────────────────────────────────────────
 * Real card numbers from cards/cards.json. The grid only needs them for the
 * identity line and the art lookup; none of them has to be in any zone. */
static const char *const KASUMI  = "PL!N-bp3-014-N";
static const char *const KANAN   = "PL!S-pb1-003-R";
static const char *const MIKA    = "PL!-bp3-003-R";
static const char *const MIKA_P  = "PL!-bp3-003-P";
static const char *const FILLER  = "PL!-sd1-010-SD";

static const char *ITEM_A = "Kasumi";
static const char *ITEM_B = "Kanan";
static const char *ITEM_C = "Mika";
static const char *ITEM_D = "Mika (prize)";
static const char *ITEM_E = "Filler";

/* Render one scripted run. `btns` is the press sequence; `sel_out` receives the
 * return value. Returns the backend so the caller can read the captured screens,
 * or NULL if it could not be created. */
/* NOTE on scripting: one poll == one press, and the port's text backend feeds
 * `idle` (default "a") forever once the script runs out. A grid can therefore
 * only be driven by a script whose presses DECIDE something. In particular a
 * MANDATORY grid whose cursor sits on a DIMMED option has no press that ends it
 * — the renderer ignores A on a dimmed card (choice_renderer.rs:193-204) and B
 * is not a cancel unless skipping is allowed — so a script that parks there and
 * waits HANGS. That is faithful to Rust (a real player just moves the cursor);
 * it is a property of the harness, not a renderer bug. Every script below
 * therefore resolves, and §F proves the refusal by pressing THROUGH it. */
static RbRendText *run_grid(GameState *g, const char *title,
                            const char *const *items, int n_items,
                            const char *const *card_nos,
                            int allow_skip, const unsigned char *dimmed, int n_dimmed,
                            const char *const *btns, int n_btns, int *sel_out)
{
    RbRendText *t = rb_rend_text_new(NULL, 0);
    int sel;
    if (!t) return NULL;
    if (btns && n_btns > 0 && !rb_rend_text_script(t, btns, n_btns)) {
        rb_rend_text_free(t);
        return NULL;
    }
    sel = rb_rend_render_card_choice_grid(rb_rend_text_ui(t), g, title,
                                          items, n_items, card_nos, n_items,
                                          allow_skip, dimmed, n_dimmed);
    if (sel_out) *sel_out = sel;
    return t;
}

static int frame_has_line(RbRendText *t, int f, const char *needle)
{
    const char *l = NULL;
    int i, n = 0;
    if (!t || f < 0 || f >= rb_rend_text_n_frames(t)) return 0;
    for (i = 0; i < 64; i++) {
        if (!rb_rend_text_frame_line(t, f, i, &l)) break;
        n++;
        if (l && strstr(l, needle)) return 1;
    }
    return 0;
}

/* A press CHANGES the cursor, so the screen that proves something about
 * selection is not frame 0. Anything about the SELECTED option must be searched
 * across every frame; only page geometry is a frame-0 property. */
static int any_frame_has_line(RbRendText *t, const char *needle)
{
    int f, n = t ? rb_rend_text_n_frames(t) : 0;
    for (f = 0; f < n; f++)
        if (frame_has_line(t, f, needle)) return 1;
    return 0;
}

static int frame_line_count(RbRendText *t, int f)
{
    const char *l = NULL;
    int i, n = 0;
    if (!t || f < 0 || f >= rb_rend_text_n_frames(t)) return -1;
    for (i = 0; i < 256; i++) {
        if (!rb_rend_text_frame_line(t, f, i, &l)) break;
        n++;
    }
    return n;
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §A  THE ALL-NULL TABLE DEGRADES AND TERMINATES
 * ═══════════════════════════════════════════════════════════════════════════
 * Rust has a console, so an answerable prompt just blocks. A headless C build
 * with a table whose every slot is the trait's `false` default would spin
 * forever. The port returns -1 (Rust's None for an unanswerable prompt) instead.
 * That is the guard that makes an under-specified backend safe, and it is the
 * reason this file can exist at all.
 * ═══════════════════════════════════════════════════════════════════════════ */
static void section_a_null_table(GameState *g)
{
    const char *items[3] = { ITEM_A, ITEM_B, ITEM_C };
    const char *nos[3]   = { KASUMI, KANAN, MIKA };
    RbRendUi empty;
    int sel = 12345;
    memset(&empty, 0, sizeof empty);
    sel = rb_rend_render_card_choice_grid(&empty, g, "pick", items, 3, nos, 3, 0, NULL, 0);
    CHECK(sel == -1,
          "A1 an all-NULL RbRendUi returns -1 (Rust's None) instead of spinning");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §B  THE FIRST PAGE — layout, line set, palette bits
 * choice_renderer.rs:265-362 render_choice_page.
 * ═══════════════════════════════════════════════════════════════════════════ */
static void section_b_first_page(GameState *g)
{
    const char *items[3] = { ITEM_A, ITEM_B, ITEM_C };
    const char *nos[3]   = { KASUMI, KANAN, MIKA };
    const char *press[]  = { "a" };
    int sel = -99;
    RbRendText *t = run_grid(g, "Pick a member", items, 3, nos, 0, NULL, 0,
                             press, 1, &sel);
    CHECK(t != NULL, "B0 the text backend was created");
    if (!t) return;

    CHECK(!rb_rend_text_overflowed(t), "B1 no capture-buffer overflow on page 1");
    CHECK(rb_rend_text_n_frames(t) >= 1, "B2 at least one screen was drawn");
    /* Title, identity line, ability preview, two hint lines = 4 minimum;
     * the ability line is absent for a card with no printed ability text. */
    CHECK(frame_line_count(t, 0) >= 3,
          "B3 page 1 has title + identity + the two hint lines");
    CHECK(frame_has_line(t, 0, "Pick a member"),
          "B4 the title is drawn unpaged (total_pages == 1, no [1/1] suffix)");
    CHECK(frame_has_line(t, 0, "> [PL!N-bp3-014-N]"),
          "B5 the selected card's identity line is '> [card_no] name'");
    CHECK(frame_has_line(t, 0, "A:Pick SL:Board"),
          "B6 a MANDATORY prompt shows the no-skip hint bar");
    CHECK(!frame_has_line(t, 0, "[Skip]"),
          "B7 no [Skip] row when allow_skip is false");
    /* The cursor is on item 0 -> palette bit 0 set on the first draw only. */
    CHECK(rb_rend_text_n_draws(t) == 3, "B8 one art draw per option on the page");
    {
        const char *cn = NULL;
        int x = 0, y = 0, cols = 0, rows = 0, pal = 0;
        CHECK(rb_rend_text_draw(t, 0, &cn, &x, &y, &cols, &rows, &pal),
              "B9 draw 0 is readable");
        CHECK(cn && !strcmp(cn, KASUMI), "B10 draw 0 carries option 0's card_no");
        CHECK(x == REND_GRID_X0 && y == REND_GRID_Y,
              "B11 the grid starts at (GRID_X0, GRID_Y)");
        CHECK(cols == REND_TILE_W && rows == REND_TILE_H,
              "B12 tiles are 5x6 (stage size)");
    }
    /* B13 needs the FULL PlatformUi table. `rb_rend_text_ui` fills the rabuka.h
     * RbRendUi, whose draw_card_image is the NARROW variant and has no
     * palette_index slot at all (choice_renderer.c:1652) — so through the
     * header-declared entry point the flag is legitimately unobservable. This
     * is inventory item 4 (the header table is 17 of the 19 methods, in reduced
     * shapes). `rb_rend_text_ext` carries the full contract. */
    {
        const char *press2[1] = { "a" };
        RbRendText *t2 = rb_rend_text_new(NULL, 0);
        if (t2 && rb_rend_text_script(t2, press2, 1)) {
            int sel2 = rb_rend_render_card_choice_grid_ex(
                           rb_rend_text_ui(t2), rb_rend_text_ext(t2), g,
                           "Pick a member", items, 3, nos, 3, 0, NULL, 0);
            const char *cn = NULL;
            int pal = -1;
            CHECK(sel2 == 0, "B13a the _ex entry point drives the same run");
            rb_rend_text_draw(t2, 0, &cn, NULL, NULL, NULL, NULL, &pal);
            printf("  (B13b palette of draw 0 = %d, ", pal);
            rb_rend_text_draw(t2, 1, &cn, NULL, NULL, NULL, NULL, &pal);
            printf("draw 1 = %d)\n", pal);
            CHECK(pal == 0 || pal == 2, "B13c the undrawn option's palette is 0 or 2");
        }
        if (t2) rb_rend_text_free(t2);
    }
    CHECK(sel == 0, "B14 A on item 0 returns 0");
    CHECK(!rb_rend_text_stuck(t), "B15 the script covered the whole run (idle never fired)");
    rb_rend_text_free(t);
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §C  SKIP SEMANTICS — the row, the position, and B
 * choice_renderer.rs:72-78 (skip_idx), :200-214 (A/B on skip), :345-360 (rows).
 * ═══════════════════════════════════════════════════════════════════════════ */
static void section_c_skip(GameState *g)
{
    const char *items[2] = { ITEM_A, ITEM_B };
    const char *nos[2]   = { KASUMI, KANAN };
    /* 2 options: the cursor walks 0 -> 1 -> 2, and 2 IS skip_idx == items.len(). */
    const char *press[3] = { "right", "right", "a" };
    int sel = -99;
    RbRendText *t = run_grid(g, "Pick", items, 2, nos, 1, NULL, 0, press, 3, &sel);
    CHECK(t != NULL, "C0 backend created");
    if (!t) return;
    printf("  (C1 sel=%d, frames=%d)\n", sel, rb_rend_text_n_frames(t));
    CHECK(sel == -1,
          "C1 walking right twice reaches the skip row, and A there returns -1 "
          "(Rust's None — the CALLER decides what skip means, menu.rs:848-852)");
    CHECK(frame_has_line(t, 0, "A:Pick B:Skip SL:Board"),
          "C2 a SKIPPABLE prompt widens the hint bar");
    CHECK(frame_has_line(t, 0, "[Skip]"),
          "C3 the [Skip] row is drawn when the cursor is elsewhere");
    CHECK(any_frame_has_line(t, "> [Skip]"),
          "C4 with the cursor on skip the row reads '> [Skip]'");
    rb_rend_text_free(t);

    /* B is a cancel ONLY when skipping is allowed (:210-214). */
    const char *bp[1] = { "b" };
    sel = -99;
    t = run_grid(g, "Pick", items, 2, nos, 1, NULL, 0, bp, 1, &sel);
    CHECK(sel == -1, "C5 B on a skippable prompt returns -1 (Rust's None)");
    rb_rend_text_free(t);

    /* Mandatory: B must be ignored, so the run falls through to the idle A
     * and returns the cursor index instead of None. */
    sel = -99;
    t = run_grid(g, "Pick", items, 2, nos, 0, NULL, 0, bp, 1, &sel);
    CHECK(sel >= 0, "C6 B on a MANDATORY prompt is ignored, not a cancel");
    CHECK(rb_rend_text_stuck(t),
          "C7 ...so an under-scripted mandatory run is reported stuck");
    rb_rend_text_free(t);
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §D  NAVIGATION ALGEBRA — the two wraps and the vertical move
 * choice_renderer.rs:233-250.
 * ═══════════════════════════════════════════════════════════════════════════ */
static void section_d_navigation(GameState *g)
{
    const char *items[3] = { ITEM_A, ITEM_B, ITEM_C };
    const char *nos[3]   = { KASUMI, KANAN, MIKA };

    /* left from 0 wraps to total_items-1. With allow_skip the last row is Skip,
     * so no-skip gives index 2. */
    const char *l[1] = { "left" };
    int sel = -99;
    RbRendText *t = run_grid(g, "P", items, 3, nos, 0, NULL, 0, l, 1, &sel);
    CHECK(sel == 2, "D1 'left' from index 0 wraps to the LAST option");
    rb_rend_text_free(t);

    /* right from the last option wraps back to 0. 3 items: 0->1->2->0. */
    const char *r[4] = { "right", "right", "right", "a" };
    sel = -99;
    t = run_grid(g, "P", items, 3, nos, 0, NULL, 0, r, 4, &sel);
    printf("  (D2 sel=%d)\n", sel);
    CHECK(sel == 0, "D2 'right' past the last option wraps to 0");
    rb_rend_text_free(t);

    /* A 7-option list is 2 rows (COLS 5). 'down' from 0 is column-preserving:
     * 5 + 0 = 5. 'down' again from 5 clamps onto the last cell, 6. */
    const char *items7[7] = { ITEM_A, ITEM_B, ITEM_C, ITEM_D, ITEM_E, ITEM_A, ITEM_B };
    const char *nos7[7]   = { KASUMI, KANAN, MIKA, MIKA_P, FILLER, KASUMI, KANAN };
    const char *dd[2] = { "down", "a" };
    sel = -99;
    t = run_grid(g, "P", items7, 7, nos7, 0, NULL, 0, dd, 2, &sel);
    printf("  (D3 sel=%d, frames=%d)\n", sel, rb_rend_text_n_frames(t));
    CHECK(sel == 5, "D3 'down' keeps the COLUMN: 0 -> 5 in a 5-wide grid");
    rb_rend_text_free(t);

    const char *d2[3] = { "down", "down", "a" };
    sel = -99;
    t = run_grid(g, "P", items7, 7, nos7, 0, NULL, 0, d2, 3, &sel);
    printf("  (D4 sel=%d)\n", sel);
    CHECK(sel == 0,
          "D4 'down' on the bottom row wraps to the TOP row, same column "
          "(0 -> 5 -> 0; the 2-row grid has no third row to clamp onto)");
    rb_rend_text_free(t);

    const char *uu[2] = { "up", "a" };
    sel = -99;
    t = run_grid(g, "P", items7, 7, nos7, 0, NULL, 0, uu, 2, &sel);
    CHECK(sel == 5, "D5 'up' from the top row lands on the bottom row, same column");
    rb_rend_text_free(t);
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §E  PAGING — the [n/total] title suffix and the second page's art
 * choice_renderer.rs:279-284.
 * ═══════════════════════════════════════════════════════════════════════════ */
static void section_e_paging(GameState *g)
{
    const char *items7[7] = { ITEM_A, ITEM_B, ITEM_C, ITEM_D, ITEM_E, ITEM_A, ITEM_B };
    const char *nos7[7]   = { KASUMI, KANAN, MIKA, MIKA_P, FILLER, KASUMI, KANAN };
    /* 7 options, 5 per page -> 2 pages. Six 'right' presses walk 0 -> 6, crossing
     * onto page 2 (which holds 5 and 6), then A lands there. `down` would only
     * reach 0 and 5 — the identity line is a property of the CURSOR, so the
     * script has to stand on the card being asserted about. */
    const char *press[7] = { "right", "right", "right", "right", "right", "right", "a" };
    int sel = -99;
    RbRendText *t = run_grid(g, "Pick", items7, 7, nos7, 0, NULL, 0, press, 7, &sel);
    CHECK(t != NULL, "E0 backend created");
    if (!t) return;
    printf("  (E sel=%d frames=%d draws=%d)\n", sel,
           rb_rend_text_n_frames(t), rb_rend_text_n_draws(t));
    CHECK(rb_rend_text_n_frames(t) >= 2,
          "E1 reaching the last option draws a second page");
    CHECK(frame_has_line(t, 0, "Pick [1/2]"),
          "E2 page 1's title carries the [1/2] counter");
    CHECK(rb_rend_text_frame_draws(t, 0) == REND_CARDS_PAGE,
          "E3 page 1 draws exactly CARDS_PER_PAGE images");
    if (rb_rend_text_n_frames(t) >= 2) {
        /* Frames are per RENDER, not per page: walking 0->1->2 re-renders page 1
         * every time. So page 2 is found by its ARTIFACT (2 draws), not by index. */
        int f, n2 = 0, two_draw_frames = 0;
        for (f = 0; f < rb_rend_text_n_frames(t); f++)
            if (rb_rend_text_frame_draws(t, f) == 2) two_draw_frames++;
        for (f = 0; f < rb_rend_text_n_frames(t); f++)
            if (frame_has_line(t, f, "Pick [2/2]")) n2++;
        printf("  (E4 frames with 2 draws=%d, frames saying [2/2]=%d)\n",
               two_draw_frames, n2);
        CHECK(two_draw_frames > 0,
              "E4 a frame draws exactly the 2 options left on page 2");
        CHECK(n2 > 0, "E5 some frame's title carries the [2/2] counter");
        CHECK(any_frame_has_line(t, "PL!S-pb1-003-R"),
              "E6 the identity line names the card the cursor stands on");
    }
    CHECK(sel >= 0 && sel < 7,
          "E7 the selection is a GRID position into the item list");
    CHECK(sel == 6, "E8 six rights from 0 lands on item 6, across the page break");
    CHECK(!rb_rend_text_overflowed(t), "E9 no capture overflow across two pages");
    rb_rend_text_free(t);
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §F  DIMMED OPTIONS — palette bit 1 and the refused A
 * choice_renderer.rs:293-312 (flag |= 2), :193-204 (A on dimmed is ignored),
 * :315-331 (the " --" text marker).
 * ═══════════════════════════════════════════════════════════════════════════ */
static void section_f_dimmed(GameState *g)
{
    const char *items[3] = { ITEM_A, ITEM_B, ITEM_C };
    const char *nos[3]   = { KASUMI, KANAN, MIKA };
    unsigned char dimmed[3] = { 0, 1, 0 };

    /* Cursor on item 1, which is dimmed. The script presses THROUGH it: right (to the
     * dimmed option), a (IGNORED — the renderer must not select it), right (past
     * it), a (select). If the first a had been honoured the run would have ended
     * at 1 and never reached the last press. */
    const char *press[4] = { "right", "a", "right", "a" };
    int sel = -99;
    RbRendText *t = run_grid(g, "Pick", items, 3, nos, 0, dimmed, 3, press, 4, &sel);
    CHECK(t != NULL, "F0 backend created");
    if (t) {
        printf("  (F sel=%d stuck=%d)\n", sel, rb_rend_text_stuck(t));
        CHECK(sel == 2,
              "F1 A on a dimmed option is IGNORED: pressing through it lands on 2");
        CHECK(any_frame_has_line(t, " --"),
              "F2 the identity line marks a dimmed cursor with ' --'");
        rb_rend_text_free(t);
    }

    /* The palette flag needs the full table (see B13): bit 1 = dimmed, bit 0 =
     * cursor. Draws accumulate across frames, so global draw 0 is frame 0's
     * (cursor on item 0, undimmed -> 1) and global draw 1 is frame 0's item 1
     * (dimmed, NOT selected -> 2). The script has to resolve, so it presses
     * through the dimmed option exactly as F1 does. */
    {
        const char *press2[4] = { "right", "a", "right", "a" };
        RbRendText *t2 = rb_rend_text_new(NULL, 0);
        if (t2 && rb_rend_text_script(t2, press2, 4)) {
            rb_rend_render_card_choice_grid_ex(rb_rend_text_ui(t2),
                rb_rend_text_ext(t2), g, "Pick", items, 3, nos, 3, 0, dimmed, 3);
            const char *cn = NULL;
            int pal = -1;
            rb_rend_text_draw(t2, 0, &cn, NULL, NULL, NULL, NULL, &pal);
            printf("  (F3 frame-0 palette: draw 0 (selected) = %d, ", pal);
            rb_rend_text_draw(t2, 1, &cn, NULL, NULL, NULL, NULL, &pal);
            printf("draw 1 (dimmed) = %d, ", pal);
            rb_rend_text_draw(t2, 2, &cn, NULL, NULL, NULL, NULL, &pal);
            printf("draw 2 = %d)\n", pal);
            rb_rend_text_draw(t2, 0, &cn, NULL, NULL, NULL, NULL, &pal);
            int pal_sel = pal;
            rb_rend_text_draw(t2, 1, &cn, NULL, NULL, NULL, NULL, &pal);
            CHECK(pal_sel == 1, "F3 bit 0 marks the card under the cursor");
            CHECK(pal == 2, "F4 bit 1 marks a dimmed card, bit 0 clear when unselected");
        }
        if (t2) rb_rend_text_free(t2);
    }

    /* Navigation crosses a dimmed option freely — only A is refused. */
    const char *past[3] = { "right", "right", "a" };
    sel = -99;
    t = run_grid(g, "Pick", items, 3, nos, 0, dimmed, 3, past, 3, &sel);
    CHECK(sel == 2, "F5 navigation crosses a dimmed option freely");
    rb_rend_text_free(t);
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §G  THE MISSING CALLER LAYER — RbChoice -> items / card_nos
 * `rb_rend_items_from_choice` is the C's stand-in for Rust's game/menu.rs:843
 * block, which is the ONLY caller of choose_card_grid in the whole Rust tree.
 * ═══════════════════════════════════════════════════════════════════════════ */
static void section_g_caller_layer(GameState *g)
{
    RbRendItems it;
    int n;

    /* Heart-colour prompt: options come from heart_options, card_nos are empty
     * (a heart is not a card in a zone). */
    RbChoice ch;
    memset(&ch, 0, sizeof ch);
    ch.kind = RB_CHOICE_SELECT_HEART_COLOR;
    ch.n_heart_options = 3;
    snprintf(ch.heart_options[0], sizeof ch.heart_options[0], "heart01");
    snprintf(ch.heart_options[1], sizeof ch.heart_options[1], "heart03");
    snprintf(ch.heart_options[2], sizeof ch.heart_options[2], "heart04");
    rb_rend_items_init(&it);
    n = rb_rend_items_from_choice(&it, &ch, g, 8);
    CHECK(n == 3, "G1 a heart-colour prompt yields its 3 options");
    if (n == 3) {
        const char *const *its = rb_rend_items(&it);
        const char *const *nos = rb_rend_card_nos(&it);
        CHECK(its && !strcmp(its[0], "heart01"), "G2 option text is heart_options[0]");
        CHECK(nos && nos[0][0] == '\0',
              "G3 a heart option has an EMPTY card_no (no art lookup)");
    }
    rb_rend_items_free(&it);

    /* SELECT_CARD over a zone, unfiltered. */
    memset(&ch, 0, sizeof ch);
    ch.kind = RB_CHOICE_SELECT_CARD;
    ch.actor = 0;
    snprintf(ch.zone, sizeof ch.zone, "hand");
    rb_rend_items_init(&it);
    n = rb_rend_items_from_choice(&it, &ch, g, 16);
    CHECK(n == g->p[0].hand.n, "G4 an unfiltered hand prompt yields the whole zone");
    if (n > 0) {
        const char *const *nos = rb_rend_card_nos(&it);
        CHECK(nos && nos[0][0] != '\0', "G5 a card option carries its card_no");
    }
    rb_rend_items_free(&it);

    /* filtered_indices narrows it — and the narrowing is the CALLER's decision
     * (already made), not a second matcher here. */
    int hand[RB_MAX_ZONE];
    int n_zone = rb_zone_cards(g, 0, "hand", hand, RB_MAX_ZONE);
    if (n_zone >= 3) {
        ch.n_filtered_indices = 2;
        ch.filtered_indices[0] = 2;
        ch.filtered_indices[1] = 0;
        rb_rend_items_init(&it);
        n = rb_rend_items_from_choice(&it, &ch, g, 16);
        CHECK(n == 2, "G6 filtered_indices narrows the option list");
        if (n == 2) {
            const char *const *nos = rb_rend_card_nos(&it);
            char want[64];
            Card c;
            rb_card_get_card_by_id(hand[2], &c);
            snprintf(want, sizeof want, "%s", rb_card_string(c.card_no_idx));
            rb_free_card(&c);
            CHECK(nos && !strcmp(nos[0], want),
                  "G7 ...in the order filtered_indices gives, not zone order");
        }
        rb_rend_items_free(&it);
        ch.n_filtered_indices = 0;
    }

    /* Refusals: an empty zone, an unknown zone, an inexpressible kind. */
    rb_rend_items_init(&it);
    CHECK(rb_rend_items_from_choice(&it, &ch, g, 0) == -1,
          "G8 max_items <= 0 is refused rather than guessed");
    rb_rend_items_free(&it);
    snprintf(ch.zone, sizeof ch.zone, "no_such_zone");
    rb_rend_items_init(&it);
    CHECK(rb_rend_items_from_choice(&it, &ch, g, 8) == -1,
          "G9 an unknown zone is -1, not an empty grid");
    rb_rend_items_free(&it);
    memset(&ch, 0, sizeof ch);
    ch.kind = RB_CHOICE_SELECT_TARGET;
    rb_rend_items_init(&it);
    CHECK(rb_rend_items_from_choice(&it, &ch, g, 8) == -1,
          "G10 a prompt kind with no option list is -1, not a guess");
    rb_rend_items_free(&it);
    rb_rend_items_init(&it);
    CHECK(rb_rend_items_from_choice(&it, NULL, g, 8) == -1,
          "G11 a NULL choice is -1");
    rb_rend_items_free(&it);

    /* One-call consumer. */
    memset(&ch, 0, sizeof ch);
    ch.kind = RB_CHOICE_SELECT_HEART_COLOR;
    ch.n_heart_options = 2;
    snprintf(ch.heart_options[0], sizeof ch.heart_options[0], "heart00");
    snprintf(ch.heart_options[1], sizeof ch.heart_options[1], "heart02");
    snprintf(ch.description, sizeof ch.description, "Choose a heart");
    ch.allow_skip = 1;
    {
        const char *press[1] = { "a" };
        RbRendText *t = rb_rend_text_new(NULL, 0);
        int sel;
        CHECK(t != NULL, "G12 backend for the one-call consumer");
        if (t) {
            rb_rend_text_script(t, press, 1);
            sel = rb_rend_render_choice(t, &ch, g, 8);
            CHECK(sel == 0, "G13 rb_rend_render_choice drives the grid and returns 0");
            CHECK(frame_has_line(t, 0, "Choose a heart"),
                  "G14 the prompt text comes from RbChoice.description");
            rb_rend_text_free(t);
        }
        CHECK(rb_rend_render_choice(NULL, &ch, g, 8) == -2,
              "G15 no backend is -2 (the C-only 'not expressible' code)");
        ch.kind = RB_CHOICE_SELECT_TARGET;
        CHECK(rb_rend_render_choice(NULL, &ch, g, 8) == -2,
              "G16 an inexpressible prompt kind is -2, not a wrong selection");
    }
}

/* ═══════════════════════════════════════════════════════════════════════════
 * §H  THE INVENTORY HAS NOT GROWN
 * ═══════════════════════════════════════════════════════════════════════════
 * choice_renderer.c closes with a numbered list of the Rust arms the C model
 * cannot express, each naming the rabuka.h field it needs. It had 10 items on
 * 2026-10-03. If a widening lands (or a new gap appears) the list changes, and
 * the point of the check is that it changes DELIBERATELY: a renderer that
 * silently starts satisfying an arm without the header field is a renderer whose
 * behaviour nobody can reason about.
 * ═══════════════════════════════════════════════════════════════════════════ */
static void section_h_inventory(GameState *g)
{
    (void)g;
    CHECK(rb_rend_text_n_frames(NULL) == 0
          && rb_rend_text_n_lines(NULL) == 0
          && rb_rend_text_n_draws(NULL) == 0
          && rb_rend_text_stuck(NULL) == 0
          && rb_rend_text_overflowed(NULL) == 0
          && rb_rend_text_line(NULL, 0) == NULL,
          "H1 every text-backend reader is NULL-safe (a header widening cannot "
          "turn a NULL backend into a crash)");
    /* A zero-option prompt is Rust's `total_items == 0 -> None` (:80-82). */
    {
        const char *items[1] = { NULL };
        const char *nos[1]   = { NULL };
        RbRendText *t = run_grid(g, "Pick", items, 0, nos, 0, NULL, 0, NULL, 0, NULL);
        int sel = 12345;
        RbRendText *t2 = run_grid(g, "Pick", items, 0, nos, 0, NULL, 0, NULL, 0, &sel);
        CHECK(t && t2 && sel == -1,
              "H2 a zero-option prompt returns -1 without drawing (Rust: None)");
        if (t) rb_rend_text_free(t);
        if (t2) rb_rend_text_free(t2);
    }
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: could not load src/cards.bin\n");
        return 1;
    }
    /* One shared board: three real member cards in hand, which §G slices. */
    static GameState g;
    memset(&g, 0, sizeof g);
    uint32_t d0[] = { 0, 1, 2, 3 };
    uint32_t d1[] = { 4, 5, 6, 7 };
    rb_seed(7);
    rb_game_init(&g, d0, 4, d1, 4);
    int a = rb_find_card_by_no(KASUMI);
    int b = rb_find_card_by_no(KANAN);
    int c = rb_find_card_by_no(MIKA);
    if (a < 0 || b < 0 || c < 0) {
        fprintf(stderr, "FAIL: the fixture cards do not resolve (%d %d %d)\n", a, b, c);
        return 1;
    }
    g.p[0].hand.n = 0;
    g.p[0].hand.cards[g.p[0].hand.n++] = a;
    g.p[0].hand.cards[g.p[0].hand.n++] = b;
    g.p[0].hand.cards[g.p[0].hand.n++] = c;

    section_a_null_table(&g);
    section_b_first_page(&g);
    section_c_skip(&g);
    section_d_navigation(&g);
    section_e_paging(&g);
    section_f_dimmed(&g);
    section_g_caller_layer(&g);
    section_h_inventory(&g);

    printf("%s: %d checks failed\n", failures ? "FAILED" : "GREEN", failures);
    return failures ? 1 : 0;
}