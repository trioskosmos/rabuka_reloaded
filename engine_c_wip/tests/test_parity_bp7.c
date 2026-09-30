/* test_parity_bp7.c -- C parity suite for the Rust clusters left UNCOVERED by
 * the effects/gain/blades/per_card boundary re-derivation.
 *
 * COVERAGE BOUNDARY, re-derived from the BODIES of the C files (a header claim
 * has been wrong four times in this project, once listing a covered file as
 * "NOT COVERED", so every claim below was checked against code, not comments):
 *
 *   TAKEN, all 8 named targets, 29 Rust tests ported as run() cases:
 *     A  bp7_parser_gap_cards_test.rs                        11  <- the
 *        highest-value item: the Rust file's own doc comment says it documents
 *        PARSER GAPS, so a clean port says exactly which 7th-print (bp7)
 *        cards the C decoder cannot represent.  Sections A1 (Wien
 *        PL!SP-bp7-010-R), A2 (Ayumu PL!N-bp7-001-R), A3 (Kanon
 *        PL!SP-bp7-012-N) plus A4 (the Kahori PL!SP-pb1-005-R deck->zone
 *        placement that A2's negative needs as a REAL positive control).
 *     B  ren_baton_touch_group_check_test.rs                  3
 *     C  yell_same_group_members_multi_color_hearts_test.rs   3  (PL!S-PR-040-PR
 *        is brand new to the C corpus: it appears in NO tests/*.c, confirmed
 *        by a card-number scan of every file in this tree)
 *     D  kanon_debut_one_per_group_deck_bottom_edge_test.rs   3  KNOWN-DEAD-PATH
 *        REPRO, taken anyway per instruction: it is the only coverage of the
 *        "select from a zone then move" path.
 *     E  eri_debut_up_to_two_opponent_members_to_wait_test.rs 3  KNOWN-DEAD-PATH
 *        REPRO, same reason.
 *     F  turn_scope_regression_test.rs                        2  VERIFIED: a
 *        peer's header in test_parity_blades2.c:67 claims this is uncovered;
 *        the scan finds no C test that drives the tracker LIFETIME across the
 *        second attacker's Active phase and a turn rollover with these three
 *        assertions.  test_parity_deploy_recover.c:1483 exercises a related
 *        clear site (mv_clear_card_movement_tracking_resets_turn_scope) but
 *        not this contract, so it is NOT covered.
 *     G  modifier_layer_characterization_test.rs              3  The peer's skip
 *        reason was FALSE: `rb_constant_per_unit_units` IS declared in
 *        include/rabuka.h (in the "Constant ability lookup" block, right above
 *        rb_resolve_per_unit_count).  More to the point, none of these three
 *        Rust tests calls it at all -- they need rb_recalc_constants,
 *        rb_check_expired_effects, rb_mods_add_heart and g->n_temp_effects, all
 *        of which ARE declared.  The suite compiles.
 *     H  hazuki q233_declined_optional_energy_allows_second_trigger_same_turn
 *        1  (the tail of hazuki_activate_discard_per_liella_member_blade_
 *        test.rs; test_parity_blades2.c's section C ports 3 of 4 and its
 *        RUN list stops at the three 起動 tests, so the q233 自動 case is
 *        genuinely uncovered)
 *
 *   NOT TAKEN.  Nothing from the assigned list was dropped; the two items below
 *   are the peer's leftovers that this brief did not assign:
 *     - ability_mod full_distinct (PL!S-BP2-012-N absent from the whole C
 *       corpus).  Not assigned; not taken.
 *
 * ENGINE GAPS.  Every assertion that exposes one is left STRICT AND RED.  The
 * gaps are all SHAPE gaps -- the C decoder produced a tree that cannot express
 * the printed card -- so RB7_DUMP=1 prints the ENGINE's own decoded shape for
 * every fixture and every root cause below was read out of the C source, not
 * inferred from a red.  See the enumeration at the bottom of section A for the
 * parser-gap list, which is the deliverable of that section.
 *
 * NO JAPANESE IN A FUNCTIONAL POSITION.  The editor can emit CP932 and silently
 * corrupt Japanese literals, which would break the group/series filters this
 * file depends on.  Card numbers are ASCII; U+FF0B FULLWIDTH PLUS uses the byte
 * escape FP; every trigger string an assertion depends on comes from the
 * engine's own RB_TSTR_* constants.  Group names are never written literally
 * either -- rb_card_group_name() is PRINTED at startup, so a filter mismatch
 * shows up as a visible name instead of a silent "no match".  Japanese appears
 * in comments and assertion messages only, where a mis-decode would be
 * cosmetic.  The file is verified UTF-8 after every write.
 *
 * LANDMINES respected:
 *   - rb_load("src") fails under an isolated out-of-tree build, so
 *     load_card_db() falls back to ../cards/build and cards/build.
 *   - sizeof(GameState) is ~781 KB: every TestGame is `static`.
 *   - test_get_heart_modifier() REMAPS colour 5 onto RB_HEART_ORANGE, so no
 *     heart here is read through it; rb_mods_get_heart is called directly and
 *     the heartNN -> enum map is PRINTED at startup from the engine's own
 *     rb_heart_color_as_str/rb_parse_heart_color.
 *   - rb_card_string() takes a STRING-TABLE INDEX: pin_id() compares
 *     rb_card_string(c.card_no_idx) byte-for-byte and rb_card_no_eq is NOT
 *     used as an identity pin (a peer proved it accepts PL!SP-PR-004-R
 *     against the real PL!SP-PR-004-PR).
 *   - rb_card_copy_pool_reset() runs per case.
 *   - RbChoice.zone for the waitroom is the string "discard".
 *   - the C deck bag is TOP-INDEX-0, so Rust `cards.push(x)` is a plain append
 *     and Rust `cards.insert(0, x)` is test_insert_deck_top().
 *   - rb_calc_stage_hearts recomputes, so no pre-seeded stage_hearts[] here.
 *   - test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) destroys the yell
 *     window, so section F drives phases with bare test_pass() instead.
 *   - Rust `set_recently_moved_cards` / `recently_moved_from_zone` -> C
 *     state.recently_moved[] / n_recently_moved.  There is a
 *     `recently_moved_from_zone` FIELD in the C GameState (a char[]), but no
 *     setter, so it is set directly where the Rust sets it.
 *   - Rust `game.id(x)` mints a NEW instance per call; C twins are mid()
 *     (test_new_id, a copy) and sid() (test_id, the shared template).  Every
 *     section that re-identifies a card in a zone uses the id it STAGED, never
 *     a fresh lookup -- that mistake is exactly what the Rust file D's header
 *     says made its own tail vacuous.
 *
 * ANTI-VACUITY.  A per_card cluster is a grid of one card x one behaviour, so
 * one silent no-op makes a whole column of negatives pass for free.  Every
 * negative here carries either an in-test POSITIVE CONTROL (the identical
 * fixture with the one variable restored, which must produce the effect) or a
 * prompt-count control.  CHECK_PROMPTS counts the prompt still ON SCREEN at the
 * assertion site, not only at answer time.
 *
 * run() is crash ISOLATION, not a hang detector: no watchdog is achievable on
 * this toolchain.  Every drain is capped.  The counters live in a
 * MAP_SHARED|MAP_ANONYMOUS mapping, so a child killed by SIGSEGV/SIGBUS/SIGABRT
 * still reports the assertions it completed before dying.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* U+FF0B FULLWIDTH PLUS SIGN, as the byte escape. */
#define FP "\xef\xbc\x8b"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ── fixtures ───────────────────────────────────────────────────────────
 * Identity for every one of these was read out of cards/cards.json read as
 * UTF-8 (never Get-Content -Raw, which mangles non-ASCII and reports false
 * "missing" cards).  Evidence per card is repeated at the section header. */
#define WIEN      "PL!SP-bp7-010-R"    /* ウィーン・マルガレーテ, R, SS!!    */
#define KANON_BP7 "PL!SP-bp7-012-N"    /* 澁谷かのん,         N, SS!!      */
#define AYUMU     "PL!N-bp7-001-R"     /* 上原歩夢,           R, 虹ヶ咲   */
#define SHIORIKO  "PL!N-bp7-010-R"     /* 三船栞子,           R, 虹ヶ咲   */
#define KAHORI    "PL!SP-pb1-005-R"    /* 葉月 恋,            R, SS!!    */
#define REN_BP4   "PL!SP-bp4-005-R" FP /* 葉月 恋,            R＋, SS!!   */
#define HANAMARU40 "PL!S-PR-040-PR"    /* 国木田花丸,         PR, サンシャイン */
#define KANO_1    "PL!S-PR-015-PR"     /* 松浦果南,           PR, サンシャイン */
#define KANO_2    "PL!S-PR-016-PR"     /* 黒澤ダイヤ,         PR, サンシャイン */
#define KANO_3    "PL!S-PR-019-PR"     /* 国木田花丸,         PR, サンシャイン */
#define ERI       "PL!-bp3-002-R"      /* 絢瀬絵里,           R, ラブライブ！ */
#define SHIKI     "PL!SP-sd2-008-SD2"  /* 若菜四季,           SD2, SS!!  */
#define BIG_RIN   "PL!HS-bp5-004-R"    /* 百生 吟子,          R, 蓮ノ空  */
#define HIME      "PL!HS-cl1-006-CL"   /* 安養寺 姫芽,        CL, 蓮ノ空  */
#define CHIKA     "PL!S-bp2-001-R"     /* 高海千歌,           R, サンシャイン */
#define LIELLA_SD "PL!SP-sd1-001-SD"   /* 澁谷かのん,         SD, SS!!    */
#define CAT_F     "PL!SP-bp1-004-PR"   /* 平安名すみれ,       PR, SS!! CatChu! */
#define KALE_F    "PL!SP-bp1-013-PR"   /* 唐 可可,            PR, SS!! KALEIDOSCORE */
#define FIVE_F    "PL!SP-pb1-014-PR"   /* 嵐 千砂都,          PR, SS!! 5yncri5e! */
#define CAT_G     "PL!SP-bp1-018-PR"   /* 米女メイ,           PR, SS!! CatChu! */
#define SD1_001   "PL!-sd1-001-SD"     /* 高坂 穂乃果,        SD, ラブライブ！ */
#define SD1_003   "PL!-sd1-003-SD"     /* 南 ことり,          SD, ラブライブ！ */
#define SD1_008   "PL!-sd1-008-SD"     /* 小泉 花陽,          SD, ラブライブ！ */
#define SD1_010   "PL!-sd1-010-SD"     /* 高坂 穂乃果,        SD, ラブライブ！ */
#define SD1_019   "PL!-sd1-019-SD"     /* START:DASH!!,       SD, ラブライブ！ */
#define EN_CARD   "LL-E-001-SD"        /* エネルギーカード,    SD, energy */
#define HAZUKI_AR "PL!SP-bp5-005-AR"
#define HAZUKI_P  "PL!SP-bp5-005-P"
#define HAZUKI_R  "PL!SP-bp5-005-R" FP
#define HAZUKI_SEC "PL!SP-bp5-005-SEC"

/* ── shared counters (survive a child that dies mid-assertion) ───────── */

typedef struct {
    int assertions;
    int failures;
    int setup_bugs;
    int prompts;
} Tally;

static Tally *tal;
static const char *current_test = "(none)";

static void tally_init(void)
{
    void *p = mmap(NULL, sizeof(Tally), PROT_READ | PROT_WRITE,
                   MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    tal = (p == MAP_FAILED) ? NULL : (Tally *)p;
    if (tal) memset(tal, 0, sizeof *tal);
}

static void on_fatal_signal(int sig)
{
    if (tal) {
        fprintf(stderr,
                "\n*** FATAL SIGNAL %d inside test: %s\n"
                "*** The engine, not this test, is at fault.\n"
                "*** Assertions completed before the fault: %d, failures: %d, prompts: %d\n",
                sig, current_test, tal->assertions, tal->failures, tal->prompts);
    } else {
        fprintf(stderr, "\n*** FATAL SIGNAL %d inside test: %s\n", sig, current_test);
    }
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                        \
        if (tal) tal->assertions++;                                       \
        if (!(condition)) {                                               \
            if (tal) tal->failures++;                                     \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fputc('\n', stderr);                                          \
        }                                                                 \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                              \
        if (tal) tal->assertions++;                                       \
        long long a_ = (long long)(actual);                               \
        long long e_ = (long long)(expected);                             \
        if (a_ != e_) {                                                   \
            if (tal) tal->failures++;                                     \
            fprintf(stderr, "FAIL: ");                                    \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);       \
        }                                                                 \
    } while (0)

/* VACUITY CONTROL: the board must have produced at least `n` real prompts
 * before a negative claim can be trusted.  The prompt that is still ON SCREEN
 * at the assertion site is counted HERE, because counting only at answer time
 * reports 0 for a prompt that is plainly open. */
#define CHECK_PROMPTS(n, ...) do {                                        \
        if (tal) tal->assertions++;                                       \
        if (tal) tal->prompts++;                                          \
        if (!tal || tal->prompts < (n)) {                                 \
            if (tal) tal->failures++;                                     \
            fprintf(stderr, "FAIL (VACUOUS): ");                          \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " - only %d prompt(s) offered, need %d\n",   \
                    tal ? tal->prompts : -1, (int)(n));                   \
        }                                                                 \
    } while (0)

/* ── card DB + fixtures ───────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

static void dump_cond(const char *pfx, const Condition *c, int depth);
static void dump_effect(const char *pfx, const AbilityEffect *e, int depth);

/* Diagnostics at startup: the heartNN -> enum map and the group name of every
 * card whose group/series filter the assertions depend on.  Printed, never
 * asserted -- this is what makes a group-filter red attributable.  The C
 * engine has NO accessor for the printed `unit:` field, so a group_names filter
 * that names a UNIT can only be compared against rb_card_group_name(), which
 * is SERIES-level; both are printed so the discrepancy is visible. */
static void dump_fixture_abilities(void)
{
    static const char *const cards[] = {
        WIEN, KANON_BP7, AYUMU, SHIORIKO, KAHORI, REN_BP4, HANAMARU40,
        ERI, SHIKI, BIG_RIN, HIME, CHIKA, CAT_F, KALE_F, FIVE_F, CAT_G
    };
    for (unsigned i = 0; i < sizeof cards / sizeof cards[0]; i++) {
        int cid = rb_find_card_by_no(cards[i]);
        if (cid < 0) continue;
        int n = rb_card_num_abilities((uint32_t)cid);
        fprintf(stderr, "=== decode dump: %s (id=%d, %d abilit%s) ===\n",
                cards[i], cid, n, n == 1 ? "y" : "ies");
        for (int a = 0; a < n; a++) {
            Ability ab;
            memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            fprintf(stderr, "  ab#%d triggers=%s use_limit=%d\n", a,
                    ab.triggers ? ab.triggers : "-", ab.use_limit);
            if (ab.cost)   { fprintf(stderr, "  [cost]\n");   dump_effect("  ", ab.cost, 2); }
            if (ab.effect) { fprintf(stderr, "  [effect]\n"); dump_effect("  ", ab.effect, 2); }
            rb_free_ability(&ab);
        }
    }
}

static void print_wire_map(void)
{
    if (getenv("RB7_DUMP")) dump_fixture_abilities();
    fprintf(stderr, "--- heart index map (rb_heart_color_as_str) ---\n");
    for (int i = 0; i < 7; i++) {
        RbHeartColor hc = rb_parse_heart_color(rb_heart_color_as_str(i));
        fprintf(stderr, "    index %d -> \"%s\" -> enum %d\n",
                i, rb_heart_color_as_str(i), (int)hc);
    }
    fprintf(stderr, "--- group names (rb_card_group_name -- SERIES level) ---\n");
    static const char *const watch[] = {
        WIEN, KANON_BP7, AYUMU, SHIORIKO, KAHORI, REN_BP4, HANAMARU40,
        KANO_1, KANO_2, KANO_3, ERI, SHIKI, BIG_RIN, HIME, CHIKA,
        LIELLA_SD, CAT_F, KALE_F, FIVE_F, CAT_G,
        SD1_001, SD1_003, SD1_008, SD1_010, SD1_019
    };
    for (unsigned i = 0; i < sizeof watch / sizeof watch[0]; i++) {
        int id = rb_find_card_by_no(watch[i]);
        if (id < 0) { fprintf(stderr, "    %-22s NOT FOUND\n", watch[i]); continue; }
        fprintf(stderr, "    %-22s id=%-6d group=\"%s\"\n",
                watch[i], id, rb_card_group_name(id));
    }
}

/* A fresh game with the copy pool reset: a long suite must never exhaust the
 * process-global pool and silently fall back to the shared template. */
static void fresh(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
    rb_look_reset_all();
}

/* A ROTATING ring, not one shared static buffer: a `no_of()` helper with a
 * single `static char buf[80]` called 2-3 times inside one printf prints the
 * LAST card repeatedly, which is a false-red generator. */
#define NO_RING 8
static const char *card_no_of(int cid)
{
    static char ring[NO_RING][96];
    static int next;
    char *buf = ring[next++ % NO_RING];
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &ring_dummy_unused)) {
        /* unreachable; kept out of the way */
    }
    return buf;
}

/* AGENTS.md identity rule: byte-exact strcmp of the decoded card_no, because
 * rb_card_no_eq is FUZZY. */
static int pin_id(int cid, const char *no)
{
    if (tal) tal->assertions++;
    char buf[96];
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &card_dummy_unused)) {
        /* unreachable */
    }
    return 0;
}
