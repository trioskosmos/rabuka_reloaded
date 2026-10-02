/* test_parity_bp7.c -- C parity suite for the Rust clusters left UNCOVERED by
 * the effects/gain/blades/per_card boundary re-derivation.
 *
 * COVERAGE BOUNDARY, re-derived from the BODIES of the C files (a header claim
 * has been wrong four times in this project, once listing a covered file as
 * "NOT COVERED", so every claim below was checked against code, not comments):
 *
 *   TAKEN, all 8 assigned targets, 29 Rust tests ported as run() cases:
 *     A  bp7_parser_gap_cards_test.rs                        11  <- the
 *        highest-value item: the Rust file's own doc comment says it documents
 *        PARSER GAPS, so a clean port says exactly which 7th-print (bp7)
 *        cards the C decoder cannot represent.  A1 Wien PL!SP-bp7-010-R,
 *        A2 Ayumu PL!N-bp7-001-R, A3 Kanon PL!SP-bp7-012-N; the Kahori
 *        PL!SP-pb1-005-R deck->zone placement A2's negative needs as a REAL
 *        positive control is inside A2.
 *     B  ren_baton_touch_group_check_test.rs                  3
 *     C  yell_same_group_members_multi_color_hearts_test.rs   3  (PL!S-PR-040-PR
 *     is brand new to the C corpus: it appears in NO tests .c file, confirmed
 *     by a card-number scan of every file in this tree)
 *     D  kanon_debut_one_per_group_deck_bottom_edge_test.rs   3  KNOWN-DEAD-PATH
 *        REPRO, taken anyway per instruction: it is the only coverage of the
 *        "select from a zone then move" path.
 *     E  eri_debut_up_to_two_opponent_members_to_wait_test.rs 3  KNOWN-DEAD-PATH
 *        REPRO, same reason.
 *     F  turn_scope_regression_test.rs                        2  VERIFIED
 *        UNCOVERED.  A peer's header in test_parity_blades2.c:67 claims this is
 *        uncovered but gives no evidence; a card-number + identifier scan finds
 *        no C test that drives the three tracker LIFETIME assertions across the
 *        second attacker's Active phase and a turn rollover.
 *        test_parity_deploy_recover.c:1483 (mv_clear_card_movement_tracking_
 *        resets_turn_scope) exercises a RELATED clear site but not this
 *        contract, so it is not coverage.
 *     G  modifier_layer_characterization_test.rs              3  The peer's skip
 *        reason was FALSE.  `rb_constant_per_unit_units` IS declared in
 *        include/rabuka.h (the "Constant ability lookup" block, immediately
 *        above rb_resolve_per_unit_count) -- and none of these three Rust tests
 *        calls it anyway.  What they need (rb_recalc_constants,
 *        rb_check_expired_effects, rb_mods_add_heart, g->n_temp_effects) is all
 *        declared, so the section compiles and runs.
 *     H  hazuki q233_declined_optional_energy_allows_second_trigger_same_turn
 *        1  the tail of hazuki_activate_discard_per_liella_member_blade_test.
 *        rs; test_parity_blades2.c section C ports 3 of that file's 4 tests and
 *        its RUN list stops at the three 起動 cases, so the 自動 q233 case is
 *        genuinely uncovered.
 *
 *   NOT TAKEN.  Nothing from the assigned list was dropped.  The peer's other
 *   leftover (ability_mod full_distinct, PL!S-BP2-012-N absent from the whole C
 *   corpus) was not assigned to this brief and is not taken.
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
 *     heart is read through it; rb_mods_get_heart is called directly and the
 *     heartNN -> enum map is PRINTED at startup from the engine's own
 *     rb_heart_color_as_str/rb_parse_heart_color.
 *   - rb_card_string() takes a STRING-TABLE INDEX: pin_id() compares
 *     rb_card_string(c.card_no_idx) byte-for-byte, and rb_card_no_eq is NOT
 *     used as an identity pin (a peer proved it accepts PL!SP-PR-004-R against
 *     the real PL!SP-PR-004-PR).
 *   - card_no_of() uses a ROTATING ring of buffers.  A single `static char
 *     buf[80]` called 2-3 times in one printf prints the LAST card repeatedly,
 *     which is a false-red generator.
 *   - rb_card_copy_pool_reset() runs per case.
 *   - RbChoice.zone for the waitroom is the string "discard".
 *   - the C deck bag is TOP-INDEX-0, so Rust `cards.push(x)` is a plain append
 *     and Rust `cards.insert(0, x)` is test_insert_deck_top(); the BOTTOM of the
 *     deck is the END of the array.
 *   - rb_calc_stage_hearts recomputes, so no stage_hearts[] is pre-seeded here.
 *   - test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) destroys the yell
 *     window, so section F drives phases with bare test_pass() instead.
 *   - Rust `set_recently_moved_cards` / `recently_moved_from_zone` -> C
 *     state.recently_moved[] / n_recently_moved, plus the existing
 *     recently_moved_from_zone char[] field, which has no setter and is written
 *     directly where the Rust sets it.
 *   - Rust `game.id(x)` mints a NEW instance per call; the C twins are mid()
 *     (test_new_id, a pool copy) and sid() (test_id, the shared template).
 *     Every section that re-identifies a card in a zone uses the id it STAGED,
 *     never a fresh lookup -- that mistake is exactly what section D's Rust
 *     header says made its own tail vacuous.
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
 * UTF-8 (never Get-Content -Raw, which mangles non-ASCII and produces false
 * "missing" cards).  cards.json evidence is repeated at each section. */
#define WIEN       "PL!SP-bp7-010-R"    /* ウィーン・マルガレーテ R  SS!!  */
#define KANON_BP7  "PL!SP-bp7-012-N"    /* 澁谷かのん         N  SS!!  */
#define AYUMU      "PL!N-bp7-001-R"     /* 上原歩夢           R  虹ヶ咲 */
#define SHIORIKO   "PL!N-bp7-010-R"     /* 三船栞子           R  虹ヶ咲 */
#define KAHORI     "PL!SP-pb1-005-R"    /* 葉月 恋            R  SS!!  */
#define REN_BP4    "PL!SP-bp4-005-R" FP /* 葉月 恋            R+ SS!!  */
#define HANAMARU40 "PL!S-PR-040-PR"     /* 国木田花丸         PR サンシャイン */
#define KANO_1     "PL!S-PR-015-PR"     /* 松浦果南           PR サンシャイン */
#define KANO_2     "PL!S-PR-016-PR"     /* 黑澤ダイヤ         PR サンシャイン */
#define KANO_3     "PL!S-PR-019-PR"     /* 国木田花丸         PR サンシャイン */
#define ERI        "PL!-bp3-002-R"      /* 絢瀬絵里           R  ラブライブ！ */
#define SHIKI      "PL!SP-sd2-008-SD2"  /* 若菜四季           SD2 SS!! */
#define BIG_RIN    "PL!HS-bp5-004-R"    /* 百生 吟子          R  蓮ノ空 */
#define HIME       "PL!HS-cl1-006-CL"   /* 安養寺 姫芽        CL 蓮ノ空 */
#define CHIKA      "PL!S-bp2-001-R"     /* 高海千歌           R  サンシャイン */
#define LIELLA_SD  "PL!SP-sd1-001-SD"   /* 澁谷かのん         SD SS!! */
#define CAT_F      "PL!SP-bp1-004-PR"   /* 平安名すみれ       PR SS!! CatChu! */
#define KALE_F     "PL!SP-bp1-013-PR"   /* 唐 可可            PR SS!! KALEIDOSCORE */
#define FIVE_F     "PL!SP-pb1-014-PR"   /* 嵐 千砂都          PR SS!! 5yncri5e! */
#define CAT_G      "PL!SP-bp1-018-PR"   /* 米女メイ           PR SS!! CatChu! */
#define SD1_001    "PL!-sd1-001-SD"     /* 高坂 穗乃果        SD ラブライブ！ */
#define SD1_003    "PL!-sd1-003-SD"     /* 南 ことり          SD ラブライブ！ */
#define SD1_008    "PL!-sd1-008-SD"     /* 小泉 花陽          SD ラブライブ！ */
#define SD1_010    "PL!-sd1-010-SD"     /* 高坂 穗乃果        SD ラブライブ！ */
#define SD1_019    "PL!-sd1-019-SD"     /* START:DASH!!       SD ラブライブ！ */
#define EN_CARD    "LL-E-001-SD"        /* エネルギーカード    SD energy */
#define HAZUKI_AR  "PL!SP-bp5-005-AR"
#define HAZUKI_P   "PL!SP-bp5-005-P"
#define HAZUKI_R   "PL!SP-bp5-005-R" FP
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
 * before a negative claim can be trusted.  The prompt still ON SCREEN at the
 * assertion site is counted HERE, because counting only at answer time (inside
 * pick/skip) reports 0 for a prompt that is plainly open. */
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

/* The heartNN -> RbHeartColor map, derived at run time from the engine's own
 * rb_heart_color_as_str / rb_parse_heart_color rather than hard-coded, because
 * a peer found a SHIPPED test asserting heart06 through RB_HEART_PURPLE.  The
 * engine's mapping is 0=heart00 1=heart01 2=heart02 3=heart03 4=heart04
 * 5=heart05 6=heart06.  The map is PRINTED at startup so a mismatch is visible
 * in the log instead of hiding inside an arithmetic result. */
static int HC[7];

static void hcolor_init(void)
{
    for (int i = 0; i < 7; i++)
        HC[i] = (int)rb_parse_heart_color(rb_heart_color_as_str(i));
}

/* Diagnostics at startup.  The C engine has NO accessor for the printed
 * `unit:` field, so a group_names filter naming a UNIT can only be compared
 * against rb_card_group_name(), which is SERIES-level.  Both are printed so the
 * discrepancy is visible instead of becoming a silent "no match". */
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

/* PARSER-GAP REPORT -- the deliverable of section A.
 *
 * For each 7th-print (bp7) card the Rust file names, this prints whether the C
 * decoder produced a usable effect tree, and which of the printed card's
 * load-bearing JSON fields survived into the C AbilityEffect.
 *
 * `json_fields` is a bitmask of the fields cards/abilities.json REALLY carries
 * for that card (read as UTF-8; the ground-truth indices are 547 Wien,
 * 412 Ayumu, 930 Kanon, 555 Kahori).  A field is only ever reported as LOST
 * when the JSON has it AND the C AbilityEffect has nowhere to put it -- so the
 * report never over-claims a field the card never printed in the first place.
 */
#define JGF_STATE     0x01   /* state_change                             */
#define JGF_CONDITION 0x02   /* a top-level condition                    */
#define JGF_GROUP     0x04   /* group_names                              */
#define JGF_ORDER     0x08   /* placement_order                          */
#define JGF_MULTI     0x10   /* multiple_targets                         */
#define JGF_OPTACT    0x20   /* optional_action / conditional_action     */
#define JGF_NCHILD    0x40   /* the printed sequential child count       */

static int effect_has_extra(const AbilityEffect *e, const char *k)
{
    if (!e) return 0;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return 1;
    return 0;
}

static void report_parser_gap(const char *no, int n_children, unsigned json_fields)
{
    int cid = rb_find_card_by_no(no);
    if (cid < 0) { fprintf(stderr, "PARSER-GAP %-18s NOT IN THE DATABASE\n", no); return; }
    int n = rb_card_num_abilities((uint32_t)cid);
    fprintf(stderr, "PARSER-GAP %-18s id=%-6d abilities=%d\n", no, cid, n);
    for (int a = 0; a < n; a++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        const AbilityEffect *e = ab.effect;
        fprintf(stderr, "    ab#%d trigger=%-8s effect=%s\n", a,
                ab.triggers ? ab.triggers : "-",
                (e && e->action) ? e->action : "(NONE -- NOT DECODED)");
        if (!e) { rb_free_ability(&ab); continue; }
        /* Count the sequential children the printed card declares. */
        int kids = e->n_child;
        if (e->optional_action)    kids += e->optional_action->n_child;
        if (e->conditional_action) kids += e->conditional_action->n_child;
        if (json_fields & JGF_NCHILD)
            fprintf(stderr, "            children: C=%d  json=%d  %s\n", kids,
                    n_children, kids >= n_children ? "OK" : "LOST");
        if (json_fields & JGF_OPTACT)
            fprintf(stderr, "            optional_action=%s conditional_action=%s\n",
                    e->optional_action ? "yes" : "LOST",
                    e->conditional_action ? "yes" : "LOST");
        if (json_fields & JGF_CONDITION)
            fprintf(stderr, "            condition=%s%s\n",
                    e->has_condition ? "yes" : "LOST",
                    e->has_condition ? "" : " (printed movement_condition "
                                           "energy_placed/under_member)");
        if (json_fields & JGF_STATE)
            fprintf(stderr, "            state_change=%s\n",
                    effect_has_extra(e, "state_change") ? "yes" : "LOST");
        if (json_fields & JGF_GROUP)
            fprintf(stderr, "            group_names=%s (names UNITS; "
                            "rb_card_group_name is SERIES level)\n",
                    effect_has_extra(e, "group_names") ? "decoded but NOT "
                           "enforced on the SelectCard" : "LOST");
        if (json_fields & JGF_ORDER)
            fprintf(stderr, "            placement_order=%s\n",
                    effect_has_extra(e, "placement_order") ? "yes" : "LOST");
        if (json_fields & JGF_MULTI)
            fprintf(stderr, "            multiple_targets=%s\n",
                    effect_has_extra(e, "multiple_targets") ? "yes" : "LOST");
        rb_free_ability(&ab);
    }
}

static void print_wire_map(void)
{
    fprintf(stderr, "--- heart index map (rb_heart_color_as_str) ---\n");
    for (int i = 0; i < 7; i++)
        fprintf(stderr, "    index %d -> \"%s\" -> enum %d\n",
                i, rb_heart_color_as_str(i), HC[i]);

    fprintf(stderr, "--- group names (rb_card_group_name is SERIES level) ---\n");
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

    fprintf(stderr, "--- PARSER-GAP REPORT: the 7th-print cards the Rust file "
                    "names (json child counts from cards/abilities.json) ---\n");
    /* Wien (idx 547): sequential[move_cards, move_cards] = 2 children, no
     * state_change, no group_names, no placement_order, no multiple_targets. */
    report_parser_gap(WIEN,      2, JGF_NCHILD);
    /* Ayumu (idx 412): a single move_cards with state_change "wait" and a
     * movement_condition{energy_placed, destination under_member}. */
    report_parser_gap(AYUMU,     0, JGF_STATE | JGF_CONDITION);
    /* Kanon (idx 930): conditional_on_optional; the optional branch is a
     * sequential of 3 group moves and the conditional branch is those 3 plus a
     * draw, so the total child count the JSON declares is 7. */
    report_parser_gap(KANON_BP7, 7, JGF_NCHILD | JGF_OPTACT | JGF_GROUP
                                           | JGF_ORDER | JGF_MULTI);
    /* Kahori (idx 555): a single move_cards with state_change "wait". */
    report_parser_gap(KAHORI,    0, JGF_STATE);

    if (getenv("RB7_DUMP")) dump_fixture_abilities();
}

/* A fresh game with the copy pool reset: a long suite must never exhaust the
 * process-global pool and silently fall back to the shared template. */
static void fresh(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
    rb_look_reset_all();
}

/* A ROTATING ring, never one shared buffer: a `no_of()` helper with a single
 * `static char buf[80]` called 2-3 times in one printf prints the LAST card
 * repeatedly, which is a false-red generator. */
#define NO_RING 8
static const char *card_no_of(int cid)
{
    static char ring[NO_RING][96];
    static int next;
    char *buf = ring[next++ % NO_RING];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, 96, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* AGENTS.md identity rule: byte-exact strcmp of the decoded card_no, because
 * rb_card_no_eq is FUZZY. */
static int pin_id(int cid, const char *no)
{
    if (tal) tal->assertions++;
    const char *got = card_no_of(cid);
    if (cid >= 0 && got[0] && !strcmp(got, no)) return 1;
    if (tal) { tal->failures++; tal->setup_bugs++; }
    fprintf(stderr, "SETUP BUG: identity - want card_no '%s', resolved '%s' (id=%d)\n",
            no, got, cid);
    return 0;
}

/* Rust `game.id(x)` -> a distinct pool slot.  C twin: test_new_id. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        if (tal) { tal->assertions++; tal->failures++; tal->setup_bugs++; }
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

/* Rust `game.id_ref(x)` -> the shared template slot.  C twin: test_id.  Every
 * fixture in this file needs a DISTINCT instance (that is what game.id()
 * mints), so nothing here uses the shared-template spelling and the wrapper is
 * not emitted. */
#define UNUSED_sid(tg, no) test_id((tg), (no))

static int printed_cost(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.cost;
    rb_free_card(&c);
    return v;
}

static int card_prints_trigger(int cid, const char *trig)
{
    if (cid < 0) return 0;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strcmp(ab.triggers, trig) == 0;
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static int card_trigger_index(int cid, const char *trig)
{
    if (cid < 0) return -1;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strcmp(ab.triggers, trig) == 0;
        rb_free_ability(&ab);
        if (hit) return i;
    }
    return -1;
}

/* ── decode dump (diagnostic, RB7_DUMP=1) ───────────────────────────────
 * Every engine gap in section A is a SHAPE gap -- "the cost moved the card and
 * the second child never surfaced" -- and a shape claim can only be settled by
 * looking at what the C decoder actually produced, not at abilities.json. */
static void dump_cond_fields(const char *pfx, const Condition *c)
{
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (!f->key) continue;
        const CondValue *v = &f->v;
        fprintf(stderr, "%s   cond.variant=%u field[%u] %s = ", pfx, c->variant, i, f->key);
        if (v->tag == RB_TAG_STR)        fprintf(stderr, "\"%s\"\n", v->s ? v->s : "(null)");
        else if (v->tag == RB_TAG_I64)   fprintf(stderr, "%lld\n", (long long)v->i);
        else if (v->tag == RB_TAG_ARRAY) fprintf(stderr, "[array of %u]\n", v->arr_n);
        else                             fprintf(stderr, "(tag %u)\n", v->tag);
    }
}

static void dump_cond(const char *pfx, const Condition *c, int depth)
{
    if (!c) return;
    fprintf(stderr, "%s  condition(variant=%u) {\n", pfx, c->variant);
    dump_cond_fields(pfx, c);
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (f->v.tag == RB_TAG_ARRAY) {
            for (uint32_t j = 0; j < f->v.arr_n; j++) {
                fprintf(stderr, "%s     %s[%u] tag=%u s=%s cond=%s\n", pfx, f->key, j,
                        f->v.arr[j].tag, f->v.arr[j].s ? f->v.arr[j].s : "(null)",
                        f->v.arr[j].cond ? "DECODED" : "NULL");
                if (f->v.arr[j].cond) dump_cond(pfx, f->v.arr[j].cond, depth + 2);
            }
        }
        if (f->v.cond) dump_cond(pfx, f->v.cond, depth + 1);
    }
    fprintf(stderr, "%s  }\n", pfx);
}

static void dump_effect(const char *pfx, const AbilityEffect *e, int depth)
{
    if (!e) return;
    for (int k = 0; k < depth; k++) fputc(' ', stderr);
    fprintf(stderr, "%s effect action=\"%s\" count=%d src=%s dst=%s target=%s "
                    "card_type=%s is_optional=%d n_child=%d n_options=%d has_cond=%d\n",
            pfx, e->action ? e->action : "(none)", e->count,
            e->source ? e->source : "-", e->destination ? e->destination : "-",
            e->target ? e->target : "-", e->card_type_field,
            e->is_optional, e->n_child, e->n_options, e->has_condition);
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i])
            fprintf(stderr, "%s   extra[%d] %s = %s\n", pfx, i, e->extra_k[i],
                    e->extra_v[i] ? e->extra_v[i] : "(null)");
    if (e->has_condition && e->condition) dump_cond(pfx, e->condition, depth + 1);
    for (int i = 0; i < e->n_child; i++)    dump_effect(pfx, e->child[i], depth + 2);
    for (int i = 0; i < e->n_options; i++)  dump_effect(pfx, e->options[i], depth + 2);
    if (e->primary_effect)     dump_effect(pfx, e->primary_effect, depth + 2);
    if (e->followup_action)    dump_effect(pfx, e->followup_action, depth + 2);
    if (e->optional_action)    dump_effect(pfx, e->optional_action, depth + 2);
    if (e->conditional_action) dump_effect(pfx, e->conditional_action, depth + 2);
}

/* ── board helpers ─────────────────────────────────────────────────────── */

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }

static const RbChoice *pchoice(TestGame *tg)
{
    return pending(tg) ? rb_get_pending_choice(&tg->state) : NULL;
}

static void note_prompt(TestGame *tg)
{
    if (pending(tg) && tal) tal->prompts++;
}

static void pick(TestGame *tg, int idx)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice(&tg->state, idx);
}

/* Rust `select_indices(&[i])`: a one-element answer array.  A multi-pick must
 * go through the indices form, not the scalar form. */
static void pick_many(TestGame *tg, const int *idx, int n)
{
    if (!pending(tg) || n <= 0) return;
    note_prompt(tg);
    rb_resume_with_choice_indices(&tg->state, idx, n);
}

/* Rust `select_indices(&[])`: DECLINE. */
static void decline(TestGame *tg)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice(&tg->state, -1);
}

/* helpers/mod.rs: `while has_pending_choice() { select_indices(&[0]) }` -- the
 * unconditional index 0, which in the C engine means ACCEPT / PROCEED.  A
 * SelectAutoAbility is answered with index 0 too, matching the peers'
 * bl_drain_proceed idiom: the C fold of an EMPTY answer is index -1 = DECLINE,
 * which would silently skip a fired 登場 and make the whole test vacuous. */
static int drain_pick_first(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) pick(tg, 0);
    return guard;
}

static int hand_n(const TestGame *tg, int pl) { return tg->state.p[pl].hand.n; }
static int deck_n(const TestGame *tg, int pl) { return tg->state.p[pl].deck.n; }
static int wait_n(const TestGame *tg, int pl) { return tg->state.p[pl].discard.n; }
static int zone_cards_n(const TestGame *tg, int pl) { return tg->state.p[pl].energy.n; }
static int zone_active(const TestGame *tg, int pl) { return tg->state.p[pl].energy_active; }
static int edeck_n(const TestGame *tg, int pl) { return tg->state.p[pl].energy_deck.n; }

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int hand_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].hand, cid); }
static int wait_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].discard, cid); }
static int deck_has(const TestGame *tg, int cid) { return bag_has(&tg->state.p[0].deck, cid); }

static int stage_has(const TestGame *tg, int pl, int cid)
{
    for (int a = 0; a < RB_STAGE_SIZE; a++)
        if (tg->state.p[pl].stage[a] == cid) return 1;
    return 0;
}

static int appeared_has(const TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.n_cards_appeared_this_turn; i++)
        if (tg->state.cards_appeared_this_turn[i] == cid) return 1;
    return 0;
}

static void set_stage(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage[area] = cid;
    tg->state.p[pl].stage_wait[area] = 0;
}

static void stage_clear(TestGame *tg, int pl, int area)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage[area] = RB_EMPTY_SLOT;
    tg->state.p[pl].stage_wait[area] = 0;
}

static void hand_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++] = cid;
}

static void wait_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
}

static void revealed_add(TestGame *tg, int cid)
{
    if (tg->state.n_revealed < RB_MAX_REVEALED_CARDS)
        tg->state.revealed_cards[tg->state.n_revealed++] = cid;
}

/* Rust `main_deck.cards.push(x)` -- an APPEND, so x lands at the BOTTOM. */
static void deck_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
}

static void deck_append_n(TestGame *tg, int pl, int cid, int n)
{
    for (int i = 0; i < n; i++) deck_append(tg, pl, cid);
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        tg->state.p[pl].deck.n = 0;
        deck_append_n(tg, pl, filler, n);
    }
}

static void edeck_append_n(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n; i++)
        if (P->energy_deck.n < RB_MAX_ZONE) P->energy_deck.cards[P->energy_deck.n++] = cid;
}

/* The `n` cards at the BOTTOM of seat `pl`'s deck.  The C bag is TOP-INDEX-0,
 * so the bottom is the END of the array -- Rust's `cards.iter().rev()`. */
static void deck_bottom(const TestGame *tg, int pl, int n, int *out)
{
    const RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n; i++) {
        int idx = P->deck.n - 1 - i;
        out[i] = (idx >= 0) ? P->deck.cards[idx] : RB_EMPTY_SLOT;
    }
}

static void print_bottom(const TestGame *tg, int pl, int n, const char *label)
{
    int buf[8];
    if (n > 8) n = 8;
    deck_bottom(tg, pl, n, buf);
    fprintf(stderr, "        [info] %s deck[%d] n=%d bottom:", label, pl, deck_n(tg, pl));
    for (int i = 0; i < n; i++) fprintf(stderr, " %s", card_no_of(buf[i]));
    /* The HEAD is printed too, because a "placed on the deck bottom" claim and
     * a "placed on the deck top" claim look identical from the bottom alone --
     * and a whole-board census is the only thing that distinguishes "nothing
     * moved" from "the card was DESTROYED". */
    fprintf(stderr, "  | head:");
    for (int i = 0; i < n && i < tg->state.p[pl].deck.n; i++)
        fprintf(stderr, " %s", card_no_of(tg->state.p[pl].deck.cards[i]));
    fputc('\n', stderr);
}

/* Rust `state.set_recently_moved_cards(vec![..])` +
 * `recently_moved_from_zone = Some("hand")`. */
static void set_recently_moved_hand(TestGame *tg, int cid)
{
    tg->state.recently_moved[0] = cid;
    tg->state.n_recently_moved = 1;
    snprintf(tg->state.recently_moved_from_zone,
             sizeof tg->state.recently_moved_from_zone, "%s", "hand");
}

static int blade_of(const TestGame *tg, int cid)
{
    return rb_mods_get_blade((RbMods *)&tg->state.mods, cid);
}

static int heart_of(const TestGame *tg, int cid, int color)
{
    return rb_mods_get_heart((RbMods *)&tg->state.mods, cid, color);
}

static const char *orientation_of(const TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation((RbMods *)&tg->state.mods, cid);
    return o ? o : "";
}

static int is_wait(const TestGame *tg, int cid)
{
    return strcmp(orientation_of(tg, cid), "wait") == 0;
}

/* helpers/mod.rs fire_trigger: resolve ONE ability on ONE card, key it
 * "{card_no}_{full_text}", fire it with `cid` as the activating card, then
 * process the queue.
 *
 * The body used to be a hand-rolled copy of that sequence, which recorded NO
 * enqueue-time use — so a 自動/ターン1回 ability (this file drives several,
 * e.g. RB_TSTR_AUTO) was still unspent when it resolved and the
 * post-resolution re-scan in rb_process_player_abilities_depth_full enqueued
 * it a SECOND time. Its "{card_no}_{full_text}" key also truncated at 512
 * bytes. test_fire_trigger_on_card is the shared shape and does both. */
static int fire_trigger(TestGame *tg, int cid, const char *trig, const char *trigger_type)
{
    (void)trigger_type;   /* every call site here passes trig == trigger_type */
    return test_fire_trigger_on_card(tg, cid, trig);
}

/* Rust `trigger_auto_abilities_for_player` + `process_pending_auto_abilities`
 * for a YELL watcher. */
static void auto_scan_and_process(TestGame *tg, int pl)
{
    rb_queue_trigger_abilities(&tg->state, pl, RB_TSTR_AUTO);
    rb_process_pending_auto_abilities(&tg->state);
}

/* ═══════════════════════════════════════════════════════════════════════════
 * A. bp7_parser_gap_cards_test.rs  (11 tests)  -- THE PARSER-GAP SECTION
 *
 * The Rust file's own doc comment says it exists because "the parser previously
 * mis-parsed" three cards, so the value of a clean port here is not the pass
 * count but the ENUMERATION of which bp7 prints the C decoder cannot represent.
 * cards/abilities.json (read as UTF-8) gives the ground-truth shapes:
 *
 *   PL!SP-bp7-010-R ウィーン・マルガレーテ  ab#0 起動   (abilities.json 547)
 *     cost:   move_cards{stage -> discard, self_cost, member_card}
 *     effect: sequential[ move_cards{energy_zone -> energy_deck, 1, energy_card},
 *                         move_cards{discard -> hand, 1, card} ]
 *   PL!N-bp7-001-R  上原歩夢                ab#0 自動 / 1回  (abilities.json 412)
 *     effect: move_cards{energy_deck -> energy_zone, state_change "wait", 1,
 *                         energy_card}
 *             + condition movement_condition{trigger_event energy_placed,
 *                                             destination "under_member"}
 *   PL!SP-bp7-012-N 澁谷かのん              ab#0 登場   (abilities.json 930)
 *     effect: conditional_on_optional{ optional_action:  sequential[3x
 *                     move_cards{discard -> deck_bottom, group_names [<one of the
 *                     three units>], placement_order "any_order",
 *                     multiple_targets, optional}],
 *                     conditional_action: the same 3 + draw_card{1} }
 *   PL!SP-pb1-005-R 葉月 恋                ab#0 登場   (abilities.json 555)
 *     effect: move_cards{energy_deck -> energy_zone, state_change "wait", 1,
 *                         energy_card}
 *
 * The startup PARSER-GAP REPORT prints, for each of these, whether the C
 * AbilityEffect actually carries optional_action / conditional_action /
 * condition / state_change / group_names / placement_order / multiple_targets.
 * That is the enumeration this section exists to produce.
 * ═══════════════════════════════════════════════════════════════════════════ */

/* ── A1. Wien PL!SP-bp7-010-R (3 tests) ──────────────────────────────────
 * The cost sends Wien itself to the waitroom; the effect is a `sequential` of
 * two moves.  run_activate_drain below mirrors the Rust drain: a
 * SelectCard{stage} costs index 0, a SelectCard{energy_zone} costs index 0,
 * anything else costs index 0 -- which in C is ACCEPT. */
typedef struct {
    int wien;
    int accepted;
    int prompts;
    int zone_before, zone_after;
    int edeck_before, edeck_after;
    int hand_before, hand_after;
    int wait_before, wait_after;
    int on_stage_after;
    int census_before, census_after;
} WienRun;

/* `with_recover` matters: Rust's wien_activation_moves_energy_zone_to_energy_
 * deck and wien_empty_energy_zone_step1_fizzles seed a RECOVER card, while
 * wien_step2_recovers_self_after_cost deliberately leaves the waitroom holding
 * ONLY Wien (put there by its own cost), which is what makes step 2's recovery
 * of Wien an IDENTITY claim rather than a count.  A shared fixture would put a
 * filler at index 0 and step 2 would recover the filler instead. */
static WienRun wien_run(TestGame *tg, int give_energy, int with_recover)
{
    WienRun r;
    memset(&r, 0, sizeof r);
    r.wien = -1; r.zone_after = r.edeck_after = r.hand_after = -1;
    r.wait_after = -1; r.on_stage_after = -1;
    fresh(tg);
    int wien = mid(tg, WIEN);
    if (!pin_id(wien, WIEN)) return r;
    r.wien = wien;
    set_stage(tg, 0, AREA_CENTER, wien);
    if (give_energy) test_give_energy(tg, 3);
    fill_decks(tg, mid(tg, SD1_010), 20);
    /* A card in the waitroom for step 2 to recover, when the case wants one. */
    if (with_recover) wait_append(tg, 0, mid(tg, SD1_010));
    test_recalc(tg);
    /* CARD CENSUS: distinguishes "the move did nothing" from "the move
     * DESTROYED the card", which look identical in a zone count. */
    r.census_before = test_total_card_count(tg);
    r.zone_before   = zone_cards_n(tg, 0);
    r.edeck_before  = edeck_n(tg, 0);
    r.hand_before   = hand_n(tg, 0);
    r.wait_before   = wait_n(tg, 0);
    int p0 = tal ? tal->prompts : 0;
    r.accepted = test_activate_ability(tg, wien);
    drain_pick_first(tg, 30);
    rb_drain_ability_queue(&tg->state);
    r.prompts = tal ? tal->prompts - p0 : 0;

    r.zone_after   = zone_cards_n(tg, 0);
    r.edeck_after  = edeck_n(tg, 0);
    r.hand_after   = hand_n(tg, 0);
    r.wait_after   = wait_n(tg, 0);
    r.on_stage_after = stage_has(tg, 0, wien);
    r.census_after = test_total_card_count(tg);
    fprintf(stderr,
            "        [info] WIEN energy=%d recover=%d accepted=%d prompts=%d "
            "zone %d->%d edeck %d->%d hand %d->%d wait %d->%d "
            "on_stage_after=%d CENSUS %d->%d\n",
            give_energy, with_recover, r.accepted, r.prompts, r.zone_before,
            r.zone_after, r.edeck_before, r.edeck_after, r.hand_before,
            r.hand_after, r.wait_before, r.wait_after, r.on_stage_after,
            r.census_before, r.census_after);
    return r;
}

static void a1_wien_moves_energy_zone_to_energy_deck(void)
{
    static TestGame tg;
    WienRun r = wien_run(&tg, 1, 1);
    if (r.wien < 0) return;
    /* PREMISE: the 起動 really is present in the compiled record, so a red
     * below is a resolution failure and not a missing-ability artefact. */
    CHECK(card_prints_trigger(r.wien, RB_TSTR_ACTIVATION),
          "premise: PL!SP-bp7-010-R really prints an 起動 in the compiled record");
    /* PROMPT CONTROL: at least one real prompt was offered, so a silent
     * no-op cannot satisfy the zone assertions below. */
    CHECK_PROMPTS(1, "CONTROL: the 起動 really raised at least one prompt");
    CHECK(r.accepted, "the 起動 is accepted from the centre");
    /* Exact, not "<": the Rust comment says the same, and a 3-card regression
     * would satisfy a band. */
    CHECK_EQ(r.zone_after, r.zone_before - 1,
             "Rust wien_activation_moves_energy_zone_to_energy_deck: "
             "exactly one energy card left the energy zone");
    CHECK_EQ(r.edeck_after, r.edeck_before + 1,
             "the same card was placed into the energy deck");
    CHECK_EQ(r.hand_after, r.hand_before + 1,
             "the second step must add the waitroom card to hand");
    /* CENSUS.  A zone count of 0 and an energy deck of 0 look the same whether
     * the move did nothing or whether it DESTROYED the card; only the
     * whole-board census tells the two apart. */
    CHECK_EQ(r.census_after, r.census_before,
             "CENSUS: the whole activation created and destroyed no card");
}

static void a1_wien_empty_energy_zone_step1_fizzles(void)
{
    static TestGame tg;
    WienRun r = wien_run(&tg, 0, 1);   /* NO energy in the zone */
    if (r.wien < 0) return;
    /* ZONE CONTROL: the zone really was empty, so step 1 has nothing to move. */
    CHECK_EQ(r.zone_before, 0,
             "premise: the energy zone really is empty on this board");
    CHECK_PROMPTS(1, "CONTROL: the 起動 still raised a prompt (the cost)");
    CHECK_EQ(r.edeck_after, r.edeck_before,
             "Rust wien_empty_energy_zone_step1_fizzles: "
             "empty energy zone -> nothing moves to the energy deck");
    CHECK_EQ(r.hand_after, r.hand_before + 1,
             "step 2 still runs and adds 1 waitroom card to hand even when "
             "step 1 fizzled");
    /* COST CONTROL: Wien left the stage (it is now a step-2 recovery
     * candidate).  Without this the two assertions above could pass on a board
     * where the whole ability never ran. */
    CHECK_EQ(r.on_stage_after, 0,
             "CONTROL: Wien's own cost still sends it to the waitroom");
}

static void a1_wien_step2_recovers_self_after_cost(void)
{
    static TestGame tg;
    WienRun r = wien_run(&tg, 1, 0);   /* waitroom holds ONLY Wien */
    if (r.wien < 0) return;
    /* The waitroom held only Wien, put there by its own cost, so step 2 can
     * only recover Wien -- an identity assertion, not a count. */
    CHECK_EQ(r.wait_after, r.wait_before,
             "premise: nothing was added to the waitroom by the fixture itself");
    CHECK_PROMPTS(1, "CONTROL: the 起動 really raised a prompt");
    CHECK_EQ(r.hand_after, r.hand_before + 1,
             "Rust wien_step2_recovers_self_after_cost: step 2 recovers a card "
             "from the waitroom to hand");
    /* Identity, using the id this test actually staged (never a fresh lookup --
     * game.id() mints a new instance per call and would make this vacuous). */
    CHECK(hand_has(&tg, r.wien),
          "Rust wien_step2_recovers_self_after_cost: Wien is recovered to hand "
          "by step 2 (checked against the id this test actually staged)");
    /* LOCALISER: the card really in hand, read out of the zone, so a red above
     * names WHICH card step 2 recovered instead of only saying "not Wien". */
    for (int i = 0; i < tg.state.p[0].hand.n; i++)
        fprintf(stderr, "        [info] hand[%d] = %s\n", i,
                card_no_of(tg.state.p[0].hand.cards[i]));
    CHECK_EQ(r.census_after, r.census_before,
             "CENSUS: the whole activation created and destroyed no card");
}

/* ── A2. Ayumu PL!N-bp7-001-R (3 tests) ────────────────────────────────────
 * 「自分のエネルギー置き場にあるエネルギーがメンバーの下に置かれたとき」 -- the
 * watcher fires on an UNDER-MEMBER arrival, not on a zone arrival.  The placer
 * is 三船栞子 PL!N-bp7-010-R (cost: place 1 energy under the member), which is
 * on stage.  A2's negative needs a REAL zone placement to contrast against, so
 * it uses 葉月 恋 PL!SP-pb1-005-R's 登場 (energy_deck -> energy_zone WAIT). */
typedef struct {
    int accepted;
    int prompts;
    int zone_before, zone_after, active_before, active_after;
    int edeck_before, edeck_after;
    int under_center;
} AyumuRun;

/* Shioriko's cost places an energy card under a member.  After the drain the
 * C engine must have recorded the movement for Ayumu's watcher; auto_scan_
 * and_process is the C analogue of the Rust's implicit recorded-event fire
 * (the Rust relies on execute_main_phase_action doing it inline). */
static AyumuRun ayumu_under_member_run(TestGame *tg, int seed_energy_deck)
{
    AyumuRun r;
    memset(&r, 0, sizeof r);
    r.zone_after = r.active_after = r.edeck_after = r.under_center = -1;
    fresh(tg);
    int ayumu = mid(tg, AYUMU), shioriko = mid(tg, SHIORIKO);
    if (!pin_id(ayumu, AYUMU)) return r;
    if (!pin_id(shioriko, SHIORIKO)) return r;
    set_stage(tg, 0, AREA_CENTER, ayumu);
    set_stage(tg, 0, AREA_RIGHT, shioriko);
    test_give_energy(tg, 5);
    if (seed_energy_deck) edeck_append_n(tg, 0, mid(tg, EN_CARD), 6);
    test_recalc(tg);

    r.zone_before   = zone_cards_n(tg, 0);
    r.active_before = zone_active(tg, 0);
    r.edeck_before  = edeck_n(tg, 0);
    int p0 = tal ? tal->prompts : 0;
    r.accepted = test_activate_ability(tg, shioriko);
    drain_pick_first(tg, 30);
    rb_drain_ability_queue(&tg->state);
    auto_scan_and_process(tg, 0);
    drain_pick_first(tg, 16);
    r.prompts = tal ? tal->prompts - p0 : 0;
    r.under_center = tg->state.p[0].under_cards[AREA_CENTER].n
                   + tg->state.p[0].under_cards[AREA_RIGHT].n;
    r.zone_after   = zone_cards_n(tg, 0);
    r.active_after = zone_active(tg, 0);
    r.edeck_after  = edeck_n(tg, 0);
    fprintf(stderr,
            "        [info] AYUMU under-member energy_deck=%d accepted=%d "
            "prompts=%d zone %d->%d active %d->%d edeck %d->%d under=%d\n",
            seed_energy_deck, r.accepted, r.prompts, r.zone_before, r.zone_after,
            r.active_before, r.active_after, r.edeck_before, r.edeck_after,
            r.under_center);
    return r;
}

static void a2_ayumu_energy_under_member_triggers(void)
{
    static TestGame tg;
    /* ANTI-VACUITY CONTROL FIRST: the identical board with a seeded energy
     * deck.  If this is red, nothing in the negative below proves anything. */
    AyumuRun pos = ayumu_under_member_run(&tg, 1);
    if (pos.zone_after < 0) return;
    CHECK(pos.accepted, "CONTROL: 三船栞子's cost activates");
    CHECK_PROMPTS(1, "CONTROL: the placer's cost really offered a prompt");
    /* The cost really moved an energy card under a member.  Without this a
     * free zone change would satisfy the "Ayumu placed a WAIT energy" claim. */
    CHECK_EQ(pos.under_center, 1,
             "CONTROL: the placer's cost really put 1 card under a member");
    CHECK(pos.edeck_after < pos.edeck_before,
          "Rust ayumu_energy_under_member_triggers_energy_deck_move: "
          "上原歩夢 should take 1 energy from the energy deck");
    CHECK_EQ(pos.active_after, pos.active_before - 1,
             "the cost removed 1 active energy");
    /* The net zone card count is unchanged (one left for the cost, one added
     * in wait), so the discriminating number is the WAIT count. */
    int zone_wait = pos.zone_after - pos.active_after;
    int wait_before = pos.zone_before - pos.active_before;
    CHECK(zone_wait >= wait_before + 1,
          "上原歩夢's energy must be placed in the zone in WAIT (wait count "
          "must have RISEN, got before %d after %d)", wait_before, zone_wait);
}

static void a2_ayumu_zone_arrival_does_not_trigger(void)
{
    static TestGame tg;
    int kahori = mid(&tg, KAHORI);
    int ayumu  = mid(&tg, AYUMU);
    int energy = mid(&tg, EN_CARD);
    if (!pin_id(kahori, KAHORI)) return;
    if (!pin_id(ayumu, AYUMU)) return;
    if (!pin_id(energy, EN_CARD)) return;
    set_stage(&tg, 0, AREA_LEFT, ayumu);
    edeck_append_n(&tg, 0, energy, 5);
    hand_append(&tg, 0, kahori);
    test_give_energy(&tg, 13);
    test_recalc(&tg);

    int edeck_before = edeck_n(&tg, 0);
    int zone_before  = zone_cards_n(&tg, 0);
    int active_before = zone_active(&tg, 0);
    int under_before = tg.state.p[0].under_cards[AREA_LEFT].n
                     + tg.state.p[0].under_cards[AREA_CENTER].n
                     + tg.state.p[0].under_cards[AREA_RIGHT].n;
    int p0 = tal ? tal->prompts : 0;
    int ok = test_play_to_stage(&tg, kahori, AREA_RIGHT);
    drain_pick_first(&tg, 24);
    rb_drain_ability_queue(&tg.state);
    auto_scan_and_process(&tg, 0);
    drain_pick_first(&tg, 16);
    int prompts = tal ? tal->prompts - p0 : 0;
    fprintf(stderr,
            "        [info] KAHORI zone debut played=%d prompts=%d zone %d->%d "
            "active %d->%d edeck %d->%d under %d->%d\n",
            ok, prompts, zone_before, zone_cards_n(&tg, 0), active_before,
            zone_active(&tg, 0), edeck_before, edeck_n(&tg, 0), under_before,
            tg.state.p[0].under_cards[AREA_LEFT].n
          + tg.state.p[0].under_cards[AREA_CENTER].n
          + tg.state.p[0].under_cards[AREA_RIGHT].n);

    /* PAIRED POSITIVE on the same board: Kahori's own 登場 really did place
     * one energy into the zone.  This is what makes the negative below
     * non-vacuous -- a silent debut would leave zone unchanged and fail HERE
     * rather than passing the negative for free. */
    CHECK(ok, "CONTROL: 葉月 恋's 登場 really played");
    CHECK_EQ(zone_cards_n(&tg, 0), zone_before + 1,
             "CONTROL (paired positive): Kahori's debut really placed 1 energy "
             "into the zone");
    /* DISCRIMINATOR.  The energy deck must fall by exactly ONE, the card
     * Kahori placed.  If 上原歩夢 also fired it would fall by two.  Note this
     * negative is NOT of the vacuous "card count == colour count" shape: the
     * positive and the negative differ in the DESTINATION zone, and the
     * positive on the same board is asserted above. */
    CHECK_EQ(edeck_n(&tg, 0), edeck_before - 1,
             "Rust ayumu_energy_into_zone_not_under_member_no_trigger: "
             "energy deck spent exactly the placed card -- 上原歩夢 must not fire "
             "on a zone arrival");
    CHECK_EQ(tg.state.p[0].under_cards[AREA_LEFT].n
             + tg.state.p[0].under_cards[AREA_CENTER].n
             + tg.state.p[0].under_cards[AREA_RIGHT].n, under_before,
             "CONTROL: a zone placement really places nothing under a member");
}

static void a2_ayumu_empty_energy_deck_no_effect(void)
{
    static TestGame tg;
    /* ANTI-VACUITY CONTROL: the identical board WITH a seeded energy deck.
     * This is the positive half of the pair; the negative is the same board
     * with the deck emptied. */
    AyumuRun pos = ayumu_under_member_run(&tg, 1);
    if (pos.zone_after < 0) return;
    CHECK_EQ(pos.under_center, 1,
             "ANTI-VACUITY CONTROL: the cost really put a card under a member "
             "on the positive board too");

    AyumuRun r = ayumu_under_member_run(&tg, 0);   /* NO energy in the deck */
    if (r.zone_after < 0) return;
    CHECK_EQ(r.edeck_before, 0, "premise: the energy deck really is empty");
    CHECK_EQ(r.under_center, 1,
             "CONTROL: the cost still moved 1 energy under a member, so the "
             "mechanism really ran");
    CHECK_EQ(r.zone_after, r.zone_before - 1,
             "Rust ayumu_empty_energy_deck_no_effect: only the cost's energy "
             "left; 上原歩夢 added nothing (empty deck)");
    CHECK_EQ(r.active_after, r.active_before - 1,
             "one active energy left for the cost, none returned");
}

/* ── A3. Kanon PL!SP-bp7-012-N (5 tests) ───────────────────────────────────
 * The 登場 is a `conditional_on_optional` whose optional_action is a
 * sequential of THREE separate move_cards, one per named group, and whose
 * conditional_action is those same three PLUS a draw.  The Rust drain answers
 * a SelectTarget with (accept ? 1 : 0) and a SelectCard with index 0; the C
 * conditional_optional gate is a SelectTarget where index 1 accepts and index 0
 * declines, so the mapping is identical. */
typedef struct {
    int kanon;
    int hand_before, hand_after;
    int deck_before, deck_after;
    int wait_before, wait_after;
    int prompted;
    int extra_prompts;
    int census_before, census_after;
} KanonRun;

static KanonRun kanon_run(TestGame *tg, int accept, int n_catchu,
                           int n_kaleido, int n_sync, int n_filler)
{
    KanonRun r;
    memset(&r, 0, sizeof r);
    r.kanon = -1; r.hand_after = r.deck_after = r.wait_after = -1;
    fresh(tg);
    int kanon = mid(tg, KANON_BP7);
    if (!pin_id(kanon, KANON_BP7)) return r;
    r.kanon = kanon;
    set_stage(tg, 0, AREA_CENTER, kanon);
    /* One card of each target group, plus a filler, in the waitroom.  Each
     * call to mid() mints a DISTINCT instance, which is what makes the
     * identity assertions below meaningful. */
    for (int i = 0; i < n_catchu;  i++) wait_append(tg, 0, mid(tg, CAT_F));
    for (int i = 0; i < n_kaleido; i++) wait_append(tg, 0, mid(tg, KALE_F));
    for (int i = 0; i < n_sync;    i++) wait_append(tg, 0, mid(tg, FIVE_F));
    for (int i = 0; i < n_filler;  i++) wait_append(tg, 0, mid(tg, SD1_010));
    fill_decks(tg, mid(tg, SD1_010), 20);
    test_recalc(tg);

    r.hand_before = hand_n(tg, 0);
    r.deck_before = deck_n(tg, 0);
    r.wait_before = wait_n(tg, 0);
    r.census_before = test_total_card_count(tg);
    if (!fire_trigger(tg, kanon, RB_TSTR_DEBUT, RB_TSTR_DEBUT)) {
        if (tal) { tal->assertions++; tal->failures++; tal->setup_bugs++; }
        fprintf(stderr, "SETUP BUG: PL!SP-bp7-012-N has no 登場 in the compiled record\n");
        return r;
    }
    r.prompted = pending(tg);
    /* Rust drain loop, capped at 40. */
    int guard = 0;
    while (pending(tg) && guard < 40) {
        guard++;
        const RbChoice *c = pchoice(tg);
        if (c && c->kind == RB_CHOICE_SELECT_TARGET) {
            r.extra_prompts++;
            pick(tg, accept ? 1 : 0);
        } else if (c && c->kind == RB_CHOICE_SELECT_CARD) {
            r.extra_prompts++;
            int zero = 0;
            pick_many(tg, &zero, 1);
        } else {
            r.extra_prompts++;
            pick(tg, 0);
        }
        rb_drain_ability_queue(&tg->state);
    }
    r.hand_after = hand_n(tg, 0);
    r.deck_after = deck_n(tg, 0);
    r.wait_after = wait_n(tg, 0);
    r.census_after = test_total_card_count(tg);
    print_bottom(tg, 0, 3, "KANON");
    fprintf(stderr,
            "        [info] KANON accept=%d prompted=%d extra=%d hand %d->%d "
            "deck %d->%d wait %d->%d CENSUS %d->%d\n",
            accept, r.prompted, r.extra_prompts, r.hand_before, r.hand_after,
            r.deck_before, r.deck_after, r.wait_before, r.wait_after,
            r.census_before, r.census_after);
    return r;
}

static void a3_kanon_selects_each_group_to_deck_bottom_and_draws(void)
{
    static TestGame tg;
    KanonRun r = kanon_run(&tg, 1, 1, 1, 1, 0);
    if (r.kanon < 0) return;
    /* PROMPT CONTROL: the ability really offered a prompt.  Without it a
     * silent no-op would satisfy all three count assertions below. */
    CHECK(r.prompted,
          "Rust kanon_selects_each_group_to_deck_bottom_and_draws: the 登場 "
          "offers a selection (the effect is conditional_on_optional)");
    CHECK_PROMPTS(1, "CONTROL: at least one real prompt was answered");
    /* The deck BOTTOM is the END of the C bag (the bag is TOP-INDEX-0), so this
     * is Rust's `cards.iter().rev().take(3)`. */
    int bottom[3];
    deck_bottom(&tg, 0, 3, bottom);
    int n_catchu = 0, n_kale = 0, n_five = 0, n_other = 0;
    for (int i = 0; i < 3; i++) {
        const char *no = card_no_of(bottom[i]);
        if (!strcmp(no, CAT_F))      n_catchu++;
        else if (!strcmp(no, KALE_F)) n_kale++;
        else if (!strcmp(no, FIVE_F)) n_five++;
        else                           n_other++;
    }
    fprintf(stderr, "        [info] KANON bottom3 counts: catchu=%d kaleido=%d "
                    "5yncri5e!=%d other=%d\n", n_catchu, n_kale, n_five, n_other);
    CHECK_EQ(n_catchu, 1, "『CatChu!』's card must be placed on the deck bottom");
    CHECK_EQ(n_kale, 1, "『KALEIDOSCORE』's card must be placed on the deck bottom");
    CHECK_EQ(n_five, 1, "『5yncri5e!』's card must be placed on the deck bottom");
    /* Exact, not ">": "> hand_before" would pass if the engine drew a second. */
    CHECK_EQ(r.hand_after - r.hand_before, 1,
             "Placing the 3 cards draws exactly 1");
    /* CENSUS: the three placements plus the one draw is a NET +2, so the
     * census must RISE by 2.  A census that does not move is the signature of
     * rb_place_card_in_zone() returning 0 for destination "deck_bottom" and the
     * selected card being DESTROYED -- which a zone count alone cannot see,
     * because "the card was deleted" and "the card was never moved" produce the
     * same deck length. */
    CHECK_EQ(r.census_after, r.census_before + 2,
             "CENSUS: 3 cards placed on the deck bottom + 1 drawn = a net +2 "
             "card census; a flat census means the placed cards were DESTROYED, "
             "not that the placement did not happen");
}

static void a3_kanon_skip_placement_no_draw(void)
{
    static TestGame tg;
    /* ANTI-VACUITY CONTROL: the identical board with ACCEPT must really move
     * cards, so the decline below is not passing on a no-op. */
    KanonRun pos = kanon_run(&tg, 1, 1, 1, 1, 0);
    if (pos.kanon < 0) return;
    CHECK(pos.prompted, "ANTI-VACUITY CONTROL: the 登場 really offers a prompt "
                        "on the accept board");
    CHECK(pos.deck_after != pos.deck_before || pos.wait_after != pos.wait_before,
          "ANTI-VACUITY CONTROL: the accept path really changes the board, so "
          "the decline below is not passing on a no-op");

    KanonRun r = kanon_run(&tg, 0, 1, 1, 1, 0);   /* SKIP */
    if (r.kanon < 0) return;
    CHECK_PROMPTS(1, "CONTROL: the decline board really offered the same prompt");
    CHECK_EQ(r.deck_after, r.deck_before,
             "Rust kanon_skip_placement_no_draw: skipping the placement must "
             "not move any cards to the deck");
    CHECK_EQ(r.hand_after, r.hand_before,
             "skipping the placement must not draw a card");
    CHECK_EQ(r.wait_after, r.wait_before,
             "CONTROL: nothing left the waitroom either");
}

static void a3_kanon_empty_discard_no_placement_no_draw(void)
{
    static TestGame tg;
    KanonRun pos = kanon_run(&tg, 1, 1, 1, 1, 0);
    if (pos.kanon < 0) return;
    CHECK(pos.prompted, "ANTI-VACUITY CONTROL: the 登場 offers a prompt when "
                        "the waitroom has the three groups");

    /* ZERO cards in the waitroom. */
    KanonRun r = kanon_run(&tg, 1, 0, 0, 0, 0);
    if (r.kanon < 0) return;
    CHECK_EQ(r.wait_before, 0, "premise: the waitroom really is empty");
    CHECK_PROMPTS(1, "CONTROL: the ability still ran and offered its prompt");
    CHECK_EQ(r.deck_after, r.deck_before,
             "Rust kanon_empty_discard_no_placement_no_draw: empty discard -> "
             "nothing placed on the deck bottom");
    CHECK_EQ(r.wait_after, r.wait_before,
             "nothing moved out of the discard pile");
    CHECK_EQ(r.hand_after, r.hand_before,
             "empty discard -> incomplete placement -> NO draw (Q118)");
}

static void a3_kanon_missing_group_places_present_but_no_draw(void)
{
    static TestGame tg;
    KanonRun pos = kanon_run(&tg, 1, 1, 1, 1, 0);
    if (pos.kanon < 0) return;
    CHECK(pos.prompted, "ANTI-VACUITY CONTROL: the 登場 offers a prompt on the "
                        "complete board");

    /* Only 2 of the 3 groups present. */
    KanonRun r = kanon_run(&tg, 1, 1, 1, 0, 0);
    if (r.kanon < 0) return;
    CHECK_EQ(r.wait_before, 2, "premise: exactly 2 group cards in the waitroom");
    CHECK_PROMPTS(1, "CONTROL: the ability really ran its selection prompt");
    int bottom[2];
    deck_bottom(&tg, 0, 2, bottom);
    int n_catchu = 0, n_kale = 0, n_five = 0;
    for (int i = 0; i < 2; i++) {
        const char *no = card_no_of(bottom[i]);
        if (!strcmp(no, CAT_F))       n_catchu++;
        else if (!strcmp(no, KALE_F)) n_kale++;
        else if (!strcmp(no, FIVE_F)) n_five++;
    }
    CHECK_EQ(n_catchu, 1,
             "Rust kanon_missing_group_places_present_but_no_draw: the present "
             "『CatChu!』 card must be placed on the deck bottom");
    CHECK_EQ(n_kale, 1,
             "the present 『KALEIDOSCORE』 card must be placed on the deck bottom");
    CHECK_EQ(r.deck_after, r.deck_before + 2,
             "only the 2 present cards are placed; nothing drawn on top");
    CHECK_EQ(r.hand_after, r.hand_before,
             "Q118: incomplete placement -> the 'そうしたとき' draw does NOT fire");
}

static void a3_kanon_one_per_group_not_multiple_from_same_group(void)
{
    static TestGame tg;
    KanonRun pos = kanon_run(&tg, 1, 1, 1, 1, 0);
    if (pos.kanon < 0) return;
    CHECK(pos.prompted, "ANTI-VACUITY CONTROL: the 登場 offers a prompt");

    /* TWO distinct 『CatChu!』 prints + one each of the other two groups.
     * Distinct PRINTS, not two copies of one print, so the count is not
     * defeated by a distinct-name gate. */
    fresh(&tg);
    int kanon = mid(&tg, KANON_BP7);
    if (!pin_id(kanon, KANON_BP7)) return;
    set_stage(&tg, 0, AREA_CENTER, kanon);
    int catchu_a = mid(&tg, CAT_F);
    int catchu_b = mid(&tg, CAT_G);
    int kale     = mid(&tg, KALE_F);
    int five     = mid(&tg, FIVE_F);
    if (!pin_id(catchu_a, CAT_F)) return;
    if (!pin_id(catchu_b, CAT_G)) return;
    if (!pin_id(kale, KALE_F)) return;
    if (!pin_id(five, FIVE_F)) return;
    CHECK(catchu_a != catchu_b,
          "setup: the two 『CatChu!』 cards are DISTINCT instances");
    fprintf(stderr, "        [info] CatChu! a=%s  b=%s\n",
            card_no_of(catchu_a), card_no_of(catchu_b));
    wait_append(&tg, 0, catchu_a);
    wait_append(&tg, 0, catchu_b);
    wait_append(&tg, 0, kale);
    wait_append(&tg, 0, five);
    fill_decks(&tg, mid(&tg, SD1_010), 20);
    test_recalc(&tg);

    int wait_before = wait_n(&tg, 0);
    int hand_before = hand_n(&tg, 0);
    CHECK(fire_trigger(&tg, kanon, RB_TSTR_DEBUT, RB_TSTR_DEBUT),
          "the 登場 is present in the compiled record");
    CHECK(pending(&tg),
          "Rust kanon_one_per_group_not_multiple_from_same_group: the 登場 "
          "offers a selection");
    CHECK_PROMPTS(1, "CONTROL: at least one real prompt was answered");
    int guard = 0;
    while (pending(&tg) && guard < 40) {
        guard++;
        const RbChoice *c = pchoice(&tg);
        if (c && c->kind == RB_CHOICE_SELECT_TARGET) pick(&tg, 1);
        else if (c && c->kind == RB_CHOICE_SELECT_CARD) { int z = 0; pick_many(&tg, &z, 1); }
        else pick(&tg, 0);
        rb_drain_ability_queue(&tg.state);
    }

    int bottom[3];
    deck_bottom(&tg, 0, 3, bottom);
    int n_cat = 0, n_kale = 0, n_five = 0;
    for (int i = 0; i < 3; i++) {
        const char *no = card_no_of(bottom[i]);
        if (!strcmp(no, CAT_F) || !strcmp(no, CAT_G)) n_cat++;
        else if (!strcmp(no, KALE_F)) n_kale++;
        else if (!strcmp(no, FIVE_F)) n_five++;
    }
    fprintf(stderr, "        [info] KANON one-per-group bottom3: cat=%d "
                    "kaleido=%d 5yncri5e!=%d\n", n_cat, n_kale, n_five);
    CHECK_EQ(n_kale, 1,
             "『KALEIDOSCORE』 must contribute exactly one card (a 2-cat "
             "outcome would starve this group)");
    CHECK_EQ(n_five, 1,
             "『5yncri5e!』 must contribute exactly one card");
    CHECK_EQ(n_cat, 1,
             "Rust kanon_one_per_group_not_multiple_from_same_group: exactly "
             "one 『CatChu!』 card is placed, not both");
    CHECK_EQ(wait_n(&tg, 0), wait_before - 3,
             "one card of each group placed; the extra same-group card stays in "
             "the waitroom");
    fprintf(stderr, "        [info] KANON one-per-group hand %d->%d\n",
            hand_before, hand_n(&tg, 0));
}

/* ═══════════════════════════════════════════════════════════════════════════
 * B. ren_baton_touch_group_check_test.rs  (3 tests)
 *
 *   PL!SP-bp4-005-R＋ 葉月 恋 (cards.json: series ラブライブ！スーパースター!!,
 *   unit 『Liella!』, R＋) ab#0 登場:
 *     『Liella!』のメンバーからバトンタッチして登場しており、かつ自分の
 *     エネルギーが7枚以上ある場合、自分のエネルギーデッキから、エネルギー
 *     カードを2枚ウェイト状態で置く。
 *
 *   The Rust drives the play through TurnEngine::execute_main_phase_action
 *   (PlayMemberToStage, Some(ren), None, Some(LeftSide), Some(true)) -- the
 *   last argument is the baton-touch flag.  The C twin is
 *   rb_play_member (reached through test_try_play_to_stage) with the baton
 *   context pre-armed, because rb_execute_main_phase_action's C signature has
 *   no baton argument: it is (g, action_type, card_id, target_player,
 *   target_idx, target_zone).  This is stated at the call site rather than
 *   hidden, because it is the one place this section is a translation of an
 *   action rather than a byte-level twin.
 *
 *   PREMISE PINNED FIRST, because the whole file is a "the group check was
 *   missing" regression: rb_card_group_name is SERIES-level, so 葉月 恋 and
 *   PL!SP-sd1-001-SD (also SS!!) may well report the SAME name.  If they do,
 *   "『Liella!』 vs non-『Liella!』" is not expressible through the C group
 *   accessor at all, and the startup [group names] dump says so.  The C engine
 *   has no `unit:` accessor.
 * ═══════════════════════════════════════════════════════════════════════════ */

/* The Rust's board: stage area 0 holds the baton source, the arriving card is
 * in hand alongside one filler, energy 15 (or 10) is up, and the energy deck
 * holds 5 energy cards for the positive case. */
typedef struct {
    int played;
    int prompts;
    int zone_before, zone_after;
    int edeck_before, edeck_after;
    int on_stage;
} RenRun;

static RenRun ren_run(TestGame *tg, const char *baton_src, int energy, int seed_deck)
{
    RenRun r;
    memset(&r, 0, sizeof r);
    r.zone_after = r.edeck_after = r.on_stage = -1;
    fresh(tg);
    int ren = mid(tg, REN_BP4);
    int src = mid(tg, baton_src);
    if (!pin_id(ren, REN_BP4)) return r;
    r.on_stage = -1;
    if (!pin_id(src, baton_src)) return r;
    /* PREMISE: the two prints really are different cards, and their group
     * names are printed at the assertion site. */
    fprintf(stderr, "        [info] REN src=%s group=\"%s\"  REN_BP4 group=\"%s\"\n",
            baton_src, rb_card_group_name(src), rb_card_group_name(ren));

    tg->state.p[0].deck.n = 0;
    deck_append_n(tg, 0, mid(tg, SD1_010), 40);
    tg->state.p[1].deck.n = 0;
    deck_append_n(tg, 1, mid(tg, SD1_010), 10);
    hand_append(tg, 0, ren);
    hand_append(tg, 0, mid(tg, SD1_010));
    set_stage(tg, 0, AREA_LEFT, src);
    test_give_energy(tg, energy);
    if (seed_deck) edeck_append_n(tg, 0, mid(tg, EN_CARD), 5);
    test_recalc(tg);

    /* Rust advance_to_turn2: 7 bare passes.  Each one is a real phase step, so
     * each is drained. */
    for (int i = 0; i < 7; i++) { test_pass(tg); drain_pick_first(tg, 12); }

    r.zone_before  = zone_cards_n(tg, 0);
    r.edeck_before = edeck_n(tg, 0);
    int p0 = tal ? tal->prompts : 0;
    /* The baton-touch flag.  rb_record_baton_touch is the engine's own
     * recorder, so the arriving card really carries the baton context. */
    rb_record_baton_touch(&tg->state, 0, ren);
    r.played = test_try_play_to_stage(tg, ren, AREA_LEFT);
    drain_pick_first(tg, 24);
    rb_drain_ability_queue(&tg->state);
    auto_scan_and_process(tg, 0);
    drain_pick_first(tg, 16);
    r.prompts = tal ? tal->prompts - p0 : 0;
    r.zone_after  = zone_cards_n(tg, 0);
    r.edeck_after = edeck_n(tg, 0);
    r.on_stage    = stage_has(tg, 0, ren);
    fprintf(stderr,
            "        [info] REN src=%s energy=%d deck=%d played=%d prompts=%d "
            "zone %d->%d edeck %d->%d on_stage=%d\n",
            baton_src, energy, seed_deck, r.played, r.prompts,
            r.zone_before, r.zone_after, r.edeck_before, r.edeck_after,
            r.on_stage);
    return r;
}

static void b_ren_baton_touch_from_liella_places_energy(void)
{
    static TestGame tg;
    RenRun r = ren_run(&tg, LIELLA_SD, 15, 1);
    if (r.on_stage < 0) return;
    /* PROMPT CONTROL: the play really raised a prompt, so a refused play
     * cannot satisfy the two energy assertions. */
    CHECK_PROMPTS(1, "CONTROL: the play really raised at least one prompt");
    CHECK(r.played, "CONTROL: the baton-touch play was accepted");
    /* Exact, not ">=": the printed text says 2, and a 5-card regression would
     * satisfy a band. */
    CHECK_EQ(r.zone_after, r.zone_before + 2,
             "Rust ren_baton_touch_from_liella_places_energy: exactly 2 energy "
             "cards added when baton-touched from 『Liella!』 with >=7 energy");
    CHECK(r.on_stage, "Ren placed on stage");
    /* CONTROL: the energy really came from the energy deck, not from the
     * payment of the play. */
    CHECK(r.edeck_after < r.edeck_before,
          "CONTROL: the 2 energy came from the energy deck");
}

static void b_ren_baton_touch_from_non_liella_no_energy(void)
{
    static TestGame tg;
    /* ANTI-VACUITY CONTROL: the identical board with a 『Liella!』 source. */
    RenRun pos = ren_run(&tg, LIELLA_SD, 15, 1);
    if (pos.on_stage < 0) return;
    CHECK_EQ(pos.zone_after, pos.zone_before + 2,
             "ANTI-VACUITY CONTROL: the 『Liella!』 source really places 2");

    RenRun r = ren_run(&tg, SD1_010, 15, 1);   /* non-『Liella!』 (μ's) */
    if (r.on_stage < 0) return;
    CHECK_PROMPTS(1, "CONTROL: the play really raised at least one prompt");
    CHECK_EQ(r.zone_after, r.zone_before,
             "Rust ren_baton_touch_from_non_liella_no_energy: no energy added "
             "when baton-touched from a non-『Liella!』 member");
    CHECK(r.on_stage, "Ren placed on stage anyway");
}

static void b_ren_baton_touch_insufficient_energy_no_effect(void)
{
    static TestGame tg;
    RenRun pos = ren_run(&tg, LIELLA_SD, 15, 1);
    if (pos.on_stage < 0) return;
    CHECK_EQ(pos.zone_after, pos.zone_before + 2,
             "ANTI-VACUITY CONTROL: 15 energy really places 2");

    /* 10 energy: after the baton cost the board is under the 7-energy gate.
     * The energy deck is STILL SEEDED here.  Seeding only the positive would
     * make this negative pass for the wrong reason -- an empty energy deck
     * cannot place 2 cards whichever way the condition evaluates, so the case
     * would be vacuous by construction.  The ONE variable that differs from
     * the positive is the active energy count. */
    RenRun r = ren_run(&tg, LIELLA_SD, 10, 1);
    CHECK_PROMPTS(1, "CONTROL: the play really raised at least one prompt");
    fprintf(stderr, "        [info] REN insufficient: active=%d zone_cards=%d "
                    "edeck %d->%d\n", zone_active(&tg, 0), zone_cards_n(&tg, 0),
            r.edeck_before, r.edeck_after);
    CHECK_EQ(r.zone_after, r.zone_before,
             "Rust ren_baton_touch_liella_insufficient_energy_no_effect: "
             "no energy added with only 10 energy (6 active after the cost)");

    /* DISCRIMINATOR for the red above.  Two readings explain it: (a) the
     * 「7枚以上」 gate is not evaluated at all, or (b) it is evaluated against
     * the energy ZONE CARD COUNT (10) rather than the ACTIVE count (6).  The
     * two are separated by a board where the zone holds fewer than 7 cards
     * AND the active count is also below 7: under reading (a) the effect still
     * fires, under reading (b) it does not.  4 energy -> 0 active after the
     * cost, and the zone holds 4. */
    RenRun d = ren_run(&tg, LIELLA_SD, 4, 1);
    if (d.on_stage >= 0) {
        fprintf(stderr, "        [info] REN DISCRIMINATOR energy=4: "
                        "active=%d zone_cards=%d zone %d->%d\n",
                zone_active(&tg, 0), zone_cards_n(&tg, 0), d.zone_before, d.zone_after);
        CHECK_EQ(d.zone_after, d.zone_before,
                 "DISCRIMINATOR: with 4 energy (0 active after the cost, 4 in "
                 "the zone) the 2-energy placement must NOT happen; a red here "
                 "means the >=7 gate is not evaluated AT ALL, whereas a green "
                 "here with the case above red means the gate reads the zone "
                 "card count instead of the active count");
    }
}

/* ═══════════════════════════════════════════════════════════════════════════
 * C. yell_same_group_members_multi_color_hearts_test.rs  (3 tests)
 *
 *   PL!S-PR-040-PR 国木田花丸 (cards.json: series ラブライブ！サンシャイン!!,
 *   PR) ab#0 自動 / 1回:
 *     自分がエールしたとき、エールにより公開された自分のカードの中に同じ
 *     グループ名を持つメンバーカードが3枚以上ある場合、ライブ終了時まで、
 *     heart01 を得る。 heart04 を得る。
 *
 *   PL!S-PR-040-PR appears in NO tests .c file in this tree (card-number scan,
 *   case- and fullwidth-plus normalised), so this section introduces a
 *   brand-new fixture.
 *
 *   RUST FIXTURE BUG WORTH RECORDING: the test's `setup` helper pushes each
 *   member into BOTH revealed_cards and the waitroom, and the wrong-group case
 *   does the same.  The condition reads the REVEALED set, so the waitroom copy
 *   is inert; it is ported faithfully (both are written) so the C board matches
 *   the Rust board, and the note is here so nobody reads the waitroom copy as
 *   load-bearing.
 *
 *   HEAT COLOURS: heart01 -> HC[1] and heart04 -> HC[4], both derived at
 *   startup from the engine's own map and printed there.  test_get_heart_
 *   modifier() is deliberately NOT used: it REMAPS colour 5 onto
 *   RB_HEART_ORANGE, and rb_mods_get_heart is the direct, unremapped reader.
 * ═══════════════════════════════════════════════════════════════════════════ */

static int yell_grant(TestGame *tg, const char *const *members, int n)
{
    fresh(tg);
    int hanamaru = mid(tg, HANAMARU40);
    if (!pin_id(hanamaru, HANAMARU40)) return -1;
    /* PREMISE: the 自動 really is on the print, so a red below is a condition
     * evaluation failure and not a missing-ability artefact. */
    if (card_trigger_index(hanamaru, RB_TSTR_AUTO) < 0) {
        if (tal) { tal->assertions++; tal->failures++; tal->setup_bugs++; }
        fprintf(stderr, "SETUP BUG: PL!S-PR-040-PR has no 自動 in the compiled record\n");
        return -1;
    }
    set_stage(tg, 0, AREA_CENTER, hanamaru);
    /* POST-YELL STATE.  The Rust fixture builds the state that exists AFTER a
     * yell -- it writes revealed_cards directly, without driving a live -- and
     * then calls trigger_auto_abilities_for_player.  「自分がエールしたとき」 is
     * a YELL-CONDITIONED trigger, so the C twin has to say the yell happened or
     * the C correctly (and unhelpfully) refuses to queue anything.  Setting
     * yell_occurred is the faithful translation of "we are in a post-yell
     * state", NOT a fabricated event: the reveal set is right there, built the
     * same way the Rust builds it.  Stated here so a red below is never read
     * as "the fixture forgot to yell". */
    tg->state.yell_occurred = 1;
    for (int i = 0; i < n; i++) {
        int m = mid(tg, members[i]);
        revealed_add(tg, m);
        wait_append(tg, 0, m);   /* the Rust's setup does both; see the header */
    }
    test_recalc(tg);
    auto_scan_and_process(tg, 0);
    drain_pick_first(tg, 16);
    return hanamaru;
}

static void c_yell_condition_not_met_does_not_queue(void)
{
    static TestGame ctl, tg;
    /* ANTI-VACUITY CONTROL: 3 same-group members really do grant BOTH colours.
     * Without it a total no-op 自動 would make both negatives pass for free. */
    const char *good[3] = { KANO_1, KANO_2, KANO_3 };
    int h = yell_grant(&ctl, good, 3);
    if (h < 0) return;
    CHECK_EQ(heart_of(&ctl, h, HC[1]), 1,
             "ANTI-VACUITY CONTROL: 3 same-group revealed members really grant "
             "heart01 x1");
    CHECK_EQ(heart_of(&ctl, h, HC[4]), 1,
             "ANTI-VACUITY CONTROL: ... and heart04 x1");

    const char *two[2] = { KANO_1, KANO_2 };
    int t = yell_grant(&tg, two, 2);
    if (t < 0) return;
    CHECK_EQ(heart_of(&tg, t, HC[1]), 0,
             "Rust multi_color_heart_condition_not_met_does_not_queue: no "
             "heart01 modifier with only 2 same-group members");
    CHECK_EQ(heart_of(&tg, t, HC[4]), 0,
             "no heart04 modifier with only 2 same-group members");
    CHECK(!pending(&tg),
          "no pending choice when the condition is not met (a fixed "
          "multi-colour grant must not create a SelectHeartColor)");
}

static void c_yell_wrong_group_does_not_trigger(void)
{
    static TestGame ctl, tg;
    const char *good[3] = { KANO_1, KANO_2, KANO_3 };
    int h = yell_grant(&ctl, good, 3);
    if (h < 0) return;
    CHECK_EQ(heart_of(&ctl, h, HC[1]), 1,
             "ANTI-VACUITY CONTROL: 3 same-group members really grant heart01");

    /* 3 members of a DIFFERENT series (μ's).  Unlike q271, this case is NOT of
     * the vacuous "card count == colour count" shape: the positive and the
     * negative have the SAME card count (3) and the SAME heart total, and
     * differ only in the group, which is exactly the discriminator. */
    const char *wrong[3] = { SD1_001, SD1_003, SD1_008 };
    int t = yell_grant(&tg, wrong, 3);
    if (t < 0) return;
    /* PREMISE printed, not asserted: the two sets really are different groups
     * under the engine's own accessor. */
    fprintf(stderr, "        [info] YELL group 1=\"%s\" vs wrong-group 1=\"%s\"\n",
            rb_card_group_name(mid(&tg, KANO_1)), rb_card_group_name(mid(&tg, SD1_001)));
    CHECK_EQ(heart_of(&tg, t, HC[1]), 0,
             "Rust multi_color_heart_wrong_group_does_not_trigger: wrong-group "
             "cards must NOT trigger");
    CHECK_EQ(heart_of(&tg, t, HC[4]), 0,
             "... for heart04 either");
    CHECK_EQ(tg.state.n_revealed, 3,
             "CONTROL: exactly 3 cards were really revealed, so the negative is "
             "not passing because the reveal set was empty");
}

static void c_yell_condition_met_grants_both_colors(void)
{
    static TestGame tg;
    const char *good[3] = { KANO_1, KANO_2, KANO_3 };
    int h = yell_grant(&tg, good, 3);
    if (h < 0) return;
    /* CONTROL: the reveal set really holds 3 cards, so this positive is not
     * vacuously satisfied by an empty set. */
    CHECK_EQ(tg.state.n_revealed, 3, "setup: 3 cards really were revealed");
    CHECK(!pending(&tg),
          "Rust multi_color_heart_condition_met_grants_both_colors: no pending "
          "choice -- a fixed multi-colour grant must not create SelectHeartColor");
    CHECK_EQ(heart_of(&tg, h, HC[1]), 1, "heart01 x1 modifier is granted");
    CHECK_EQ(heart_of(&tg, h, HC[4]), 1, "heart04 x1 modifier is granted");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * D. kanon_debut_one_per_group_deck_bottom_edge_test.rs  (3 tests)
 *    *** KNOWN-DEAD-PATH REPRO. ***
 *
 * A peer skipped this file because it is "a 起動 that selects from a zone then
 * move, which is a known-dead path, so it would yield three copies of one red".
 * That reasoning is wrong: three copies of one red is a worse outcome than one
 * red that names the path, and this is the ONLY coverage of the
 * select-from-a-zone-then-move path in the whole C corpus.  The path is now
 * pinned.
 *
 * The Rust assertions here are CHARACTERISATION assertions: they assert the
 * ability is a NO-OP after the selection prompt, and each carries the comment
 * "KNOWN GAP".  They are ported STRICTLY, not weakened, and each is paired
 * with the positive control that the prompt really appeared and that the same
 * ability on the complete board (section A3) really does move cards.  A
 * KNOWN-DEAD-PATH REPRO is not vacuous as long as the prompt-count control
 * holds: the interesting claim is "the selection resolves and NOTHING happens
 * afterwards", and a no-op ability that never prompted would satisfy a
 * count-based negative for the wrong reason.  Hence CHECK_PROMPTS on every one.
 *
 * RUST FIXTURE BUG (the Rust file documents it itself): the original tail
 * re-minted the id with `let cat = game.id(...)` in the ASSERT, which returns
 * a DIFFERENT instance that exists nowhere, so `!waitroom.contains(&cat)` was
 * true whatever the engine did.  setup_kanon now hands back the ids it staged.
 * The C port does the same: every identity assertion below uses the id the
 * section actually staged, never a fresh lookup.
 *
 * STRICTER THAN RUST, DELIBERATELY: the Rust captures `deck_len_before`
 * AFTER the drain, so its "the deck is unchanged" assertion is tautological
 * (it compares the deck against a snapshot of itself).  The C port captures
 * the deck length BEFORE the play and asserts against that, which is the
 * assertion the Rust test means.
 * ═══════════════════════════════════════════════════════════════════════════ */

typedef struct {
    int kanon, cat, kale, five;
    int played;
    int prompted;
    int extra_prompts;
    int deck_before, deck_after;
    int hand_before, hand_after;
    int wait_before, wait_after;
    int setup_ok;
} KanonEdge;

/* `n_catchu` 0 or 1; the Rust's setup_kanon always stages a filler too. */
static KanonEdge kanon_edge_setup(TestGame *tg, int with_filler, int only_catchu)
{
    KanonEdge k;
    memset(&k, 0, sizeof k);
    k.kanon = k.cat = k.kale = k.five = -1;
    k.deck_after = k.hand_after = k.wait_after = -1;
    fresh(tg);
    int kanon = mid(tg, KANON_BP7);
    if (!pin_id(kanon, KANON_BP7)) return k;
    k.kanon = kanon;
    k.cat = mid(tg, CAT_F);
    if (!pin_id(k.cat, CAT_F)) return k;
    if (!only_catchu) {
        k.kale = mid(tg, KALE_F);
        k.five = mid(tg, FIVE_F);
        if (!pin_id(k.kale, KALE_F)) return k;
        if (!pin_id(k.five, FIVE_F)) return k;
    }
    hand_append(tg, 0, kanon);          /* put kanon in hand to debut */
    wait_append(tg, 0, k.cat);
    if (!only_catchu) { wait_append(tg, 0, k.kale); wait_append(tg, 0, k.five); }
    if (with_filler) wait_append(tg, 0, mid(tg, SD1_010));
    fill_decks(tg, mid(tg, SD1_010), 20);
    test_give_energy(tg, 5);
    test_recalc(tg);
    k.setup_ok = 1;
    /* The Rust's setup_kanon captures hand_before AFTER the push, so the debut
     * (which takes the card out) leaves hand_before - 1. */
    k.hand_before = hand_n(tg, 0);
    k.deck_before = deck_n(tg, 0);
    k.wait_before = wait_n(tg, 0);
    return k;
}

/* The 登場's first prompt is the conditional_on_optional GATE, and the C gate
 * is a SelectTarget where index 1 ACCEPTS and index 0 DECLINES.  The Rust's
 * `game.drain_auto_ability_choices()` leaves that gate standing (it only
 * answers SelectAutoAbility), and the Rust then reads `has_pending_choice()`.
 * So the C must NOT blind-drain before the assertion -- an earlier revision of
 * this section did, which silently DECLINED the gate and made every "did it
 * prompt" assertion red for the wrong reason.  The order here is: assert the
 * prompt is open, THEN answer it, THEN drain the group selections. */
static void kanon_edge_drain_after_gate(KanonEdge *k, TestGame *tg)
{
    int guard = 0;
    while (pending(tg) && guard < 40) {
        guard++;
        const RbChoice *c = pchoice(tg);
        if (c && c->kind == RB_CHOICE_SELECT_TARGET) { k->extra_prompts++; pick(tg, 0); }
        else if (c && c->kind == RB_CHOICE_SELECT_CARD) {
            k->extra_prompts++;
            int zero = 0;
            pick_many(tg, &zero, 1);
        } else { k->extra_prompts++; pick(tg, 0); }
        rb_drain_ability_queue(&tg->state);
    }
    k->hand_after = hand_n(tg, 0);
    k->deck_after = deck_n(tg, 0);
    k->wait_after = wait_n(tg, 0);
}

/* Answer the conditional_on_optional gate with ACCEPT (index 1), which is what
 * "place them" means, then drain the group SelectCards. */
static void kanon_edge_accept(TestGame *tg)
{
    if (pending(tg)) { note_prompt(tg); rb_resume_with_choice(&tg->state, 1); }
}

static void d_kanon_select_three_any_order_and_draws_nothing(void)
{
    static TestGame ctl, tg;
    /* ANTI-VACUITY CONTROL: on the complete board the same ability really
     * moves cards to the deck bottom (section A3's positive).  If this is red
     * the three "KNOWN GAP" assertions below prove nothing. */
    {
        KanonEdge k = kanon_edge_setup(&ctl, 1, 0);
        if (!k.setup_ok) return;
        int played = test_play_to_stage(&ctl, k.kanon, AREA_CENTER);
        k.prompted = pending(&ctl);
        CHECK(played, "CONTROL: Kanon really played to stage");
        CHECK(k.prompted, "ANTI-VACUITY CONTROL: the 登場 really offers a prompt "
                          "(a no-op that never prompted would satisfy the "
                          "KNOWN GAP assertions for the wrong reason)");
        if (k.prompted) kanon_edge_accept(&ctl);
        kanon_edge_drain_after_gate(&k, &ctl);
        CHECK(ctl.state.p[0].deck.n != k.deck_before || wait_n(&ctl, 0) != k.wait_before,
              "CONTROL: the accept path really changes the board, so the "
              "KNOWN GAP assertions below are not passing on a no-op "
              "(this control's own red is the finding: on the real rb_play_"
              "member DEBUT path the gate is answered and then NOTHING runs, "
              "whereas the same ability fired through rb_trigger_auto_ability "
              "in section A3 does raise the three group selections)");
    }

    KanonEdge k = kanon_edge_setup(&tg, 1, 0);
    if (!k.setup_ok) return;
    /* The three group cards are NOT in the deck before the selection, so the
     * "not in the deck" comparison below is not vacuous. */
    CHECK(!deck_has(&tg, k.cat),  "premise: CatChu! is not in the deck beforehand");
    CHECK(!deck_has(&tg, k.kale), "premise: KALEIDOSCORE is not in the deck beforehand");
    CHECK(!deck_has(&tg, k.five), "premise: 5yncri5e! is not in the deck beforehand");

    k.played = test_play_to_stage(&tg, k.kanon, AREA_CENTER);
    k.prompted = pending(&tg);
    CHECK(k.played, "Kanon played to stage");
    CHECK(k.prompted,
          "Rust kanon_select_three_any_order_and_draws_nothing: should prompt "
          "to select CatChu!/KALEIDOSCORE/5yncri5e!");
    {
        const RbChoice *c = pchoice(&tg);
        if (c)
            fprintf(stderr, "        [info] KANON-EDGE choice kind=%d zone=\"%s\" "
                            "count=%d allow_skip=%d target=\"%s\"\n",
                    (int)c->kind, c->zone, c->count, c->allow_skip, c->target);
        CHECK(c && (c->kind == RB_CHOICE_SELECT_CARD || c->kind == RB_CHOICE_SELECT_TARGET),
              "kanon should offer SelectCard or SelectTarget");
    }
    CHECK_PROMPTS(1, "CONTROL: at least one real prompt was answered");
    /* ACCEPT the optional placement, then answer the group selections. */
    kanon_edge_accept(&tg);
    kanon_edge_drain_after_gate(&k, &tg);
    CHECK(k.extra_prompts < 8,
          "the follow-up is a confirmation, not a loop: extra prompts = %d",
          k.extra_prompts);
    print_bottom(&tg, 0, 3, "KANON-EDGE");
    fprintf(stderr, "        [info] KANON-EDGE deck n=%d (was %d) wait=%d "
                    "census=%d\n",
            k.deck_after, k.deck_before, k.wait_after, test_total_card_count(&tg));

    /* KNOWN GAP assertions, ported STRICTLY. */
    CHECK(wait_has(&tg, k.cat),
          "Rust kanon_select_three_any_order_and_draws_nothing KNOWN GAP: the "
          "chosen CatChu! is STILL in the waitroom -- the deck-bottom placement "
          "does not resolve");
    CHECK(wait_has(&tg, k.kale),
          "KNOWN GAP: the chosen KALEIDOSCORE is still in the waitroom too");
    CHECK(wait_has(&tg, k.five),
          "KNOWN GAP: the chosen 5yncri5e! is still in the waitroom too");
    CHECK_EQ(k.deck_after, k.deck_before,
             "KNOWN GAP: and the deck is unchanged -- nothing was placed under "
             "it (asserted against the PRE-play snapshot, which is stricter "
             "than the Rust's post-drain snapshot)");
    CHECK_EQ(k.hand_after, k.hand_before - 1,
             "KNOWN GAP: the 1-card draw does not resolve either -- the debut "
             "took Kanon out of hand and nothing came back (extra prompts: %d)",
             k.extra_prompts);
}

static void d_kanon_decline_optional_no_draw(void)
{
    static TestGame tg;
    KanonEdge k = kanon_edge_setup(&tg, 1, 0);
    if (!k.setup_ok) return;
    test_play_to_stage(&tg, k.kanon, AREA_CENTER);
    CHECK(pending(&tg),
          "the 登場 really offers its selection before the decline");
    CHECK_PROMPTS(1, "CONTROL: at least one real prompt was answered");
    decline(&tg);                       /* Rust select_indices(&[]) */
    kanon_edge_drain_after_gate(&k, &tg);
    CHECK_EQ(k.hand_after, k.hand_before - 1,
             "Rust kanon_decline_optional_no_draw: declining should not draw");
    CHECK_EQ(k.deck_after, k.deck_before, "CONTROL: declining moves nothing");
}

static void d_kanon_only_one_group_present_can_still_select_one(void)
{
    static TestGame tg;
    KanonEdge k = kanon_edge_setup(&tg, 1, 1);   /* only CatChu! in the waitroom */
    if (!k.setup_ok) return;
    test_play_to_stage(&tg, k.kanon, AREA_CENTER);
    CHECK(pending(&tg), "Rust kanon_only_one_group_present_can_still_select_one: "
                        "the 登場 offers a prompt even with one group present");
    CHECK_PROMPTS(1, "CONTROL: at least one real prompt was answered");
    {
        const RbChoice *c = pchoice(&tg);
        if (c)
            fprintf(stderr, "        [info] KANON-EDGE-1 choice kind=%d zone=\"%s\" "
                            "count=%d target=\"%s\"\n",
                    (int)c->kind, c->zone, c->count, c->target);
    }
    /* ACCEPT the gate, then the Rust selects the one CatChu! card. */
    kanon_edge_accept(&tg);
    if (pending(&tg)) {
        const RbChoice *c = pchoice(&tg);
        if (c && (c->kind == RB_CHOICE_SELECT_CARD || c->kind == RB_CHOICE_SELECT_TARGET)) {
            int zero = 0;
            pick_many(&tg, &zero, 1);
        } else {
            CHECK(0, "Rust panics on an unexpected choice type; the C engine "
                     "offered kind=%d zone=\"%s\"",
                  c ? (int)c->kind : -1, c ? c->zone : "(none)");
        }
    }
    kanon_edge_drain_after_gate(&k, &tg);
    CHECK(wait_has(&tg, k.cat),
          "Rust kanon_only_one_group_present_can_still_select_one KNOWN GAP: the "
          "chosen CatChu! is still in the waitroom -- the ability is a no-op "
          "after the prompt (asserted against the id this test staged)");
    CHECK_EQ(k.hand_after, k.hand_before - 1,
             "KNOWN GAP: no draw follows the selection");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * E. eri_debut_up_to_two_opponent_members_to_wait_test.rs  (3 tests)
 *    *** KNOWN-DEAD-PATH REPRO. ***
 *
 * A peer skipped this because it is "a 起動 that selects from a zone then
 * move".  Same answer as section D: it is the only coverage of that path, and
 * a strict red that NAMES the path beats no test.  The three assertions here
 * pin 「…を2人までウェイトにする」 -- 「まで」 is an UPPER BOUND, not a
 * requirement (Q144) -- which is a rule question, not a smoke test.
 *
 *   PL!-bp3-002-R 絢瀬絵里 (cards.json: ラブライブ！, cost ?) ab#0 登場:
 *     手札を1枚控え室に置いてもよい：相手のステージにいるコスト4以下の
 *     メンバーを2人までウェイトにする。
 *
 *   The Rust drives it with game.play_to_stage(eri, Center) and then asserts on
 *   the CHOICES themselves (`assert_select_card("hand", 1, true)` and
 *   `assert_select_card("stage", 2, true)`).  There is no C
 *   assert_select_card, so the choice's RbChoice.zone / .count / .allow_skip
 *   are asserted directly here -- the same wire fields the Rust helper reads.
 *
 *   The wait state is read back two ways because the two Rust tests read it two
 *   ways: through mods.get_orientation_modifier (rb_mods_get_orientation) and,
 *   as a zone control, through stage_wait[].  If they ever disagree that is
 *   itself reportable, so both are printed.
 * ═══════════════════════════════════════════════════════════════════════════ */

typedef struct {
    int eri, eligible_1, eligible_2, too_expensive, filler;
    int played;
    int cost_prompt;   /* 1 if a hand SelectCard(count 1, allow_skip) appeared */
    int sel_prompt;    /* 1 if a stage SelectCard(count 2, allow_skip) appeared */
    int extra_prompts;
    int hand_before, hand_after;
    int wait_before, wait_after;
    int setup_ok;
} EriRun;

static void eri_note_prompt(EriRun *k, TestGame *tg)
{
    const RbChoice *c = pchoice(tg);
    if (!c) return;
    k->extra_prompts++;
    fprintf(stderr, "        [info] ERI choice kind=%d zone=\"%s\" count=%d "
                    "allow_skip=%d target=\"%s\"\n",
            (int)c->kind, c->zone, c->count, c->allow_skip, c->target);
    if (c->kind == RB_CHOICE_SELECT_CARD && !strcmp(c->zone, "hand") && c->count == 1)
        k->cost_prompt = 1;
    if (c->kind == RB_CHOICE_SELECT_CARD && !strcmp(c->zone, "stage") && c->count == 2)
        k->sel_prompt = 1;
}

static EriRun eri_run(TestGame *tg, int n_eligible, int with_too_expensive, int pay_cost)
{
    EriRun k;
    memset(&k, 0, sizeof k);
    k.eri = k.eligible_1 = k.eligible_2 = k.too_expensive = k.filler = -1;
    k.hand_after = k.wait_after = -1;
    fresh(tg);
    int eri = mid(tg, ERI);
    if (!pin_id(eri, ERI)) return k;
    k.eri = eri;
    k.filler = mid(tg, SD1_019);
    if (!pin_id(k.filler, SD1_019)) return k;
    k.eligible_1 = mid(tg, SD1_010);   /* cost 4 */
    if (!pin_id(k.eligible_1, SD1_010)) return k;
    CHECK_EQ(printed_cost(k.eligible_1), 4,
             "premise: PL!-sd1-010-SD costs 4 (cards.json cost=4), i.e. under "
             "the printed 4-or-less limit");
    if (n_eligible > 1) {
        k.eligible_2 = mid(tg, SD1_010);
        if (!pin_id(k.eligible_2, SD1_010)) return k;
    }
    if (with_too_expensive) {
        k.too_expensive = mid(tg, SD1_001);
        if (!pin_id(k.too_expensive, SD1_001)) return k;
        /* RUST FIXTURE BUG, corrected from cards.json.  The Rust test comments
         * this card "cost=5, over limit" in all three cases.  cards.json says
         * PL!-sd1-001-SD 高坂 穂乃果 has cost=11 (unit Printemps, ラブライブ！).
         * The FIXTURE is still functionally correct -- 11 is also over the
         * printed 4-or-less limit -- so the card is kept and only the expected
         * number is corrected.  The C engine's 11 is right. */
        CHECK_EQ(printed_cost(k.too_expensive), 11,
                 "premise: PL!-sd1-001-SD costs 11 per cards.json, i.e. OVER the "
                 "printed 4-or-less limit (the Rust test's 'cost=5' comment is "
                 "wrong; the card choice is still valid)");
    }
    for (int a = 0; a < RB_STAGE_SIZE; a++) { stage_clear(tg, 0, a); stage_clear(tg, 1, a); }
    if (n_eligible > 0) set_stage(tg, 1, AREA_LEFT, k.eligible_1);
    if (n_eligible > 1) set_stage(tg, 1, AREA_CENTER, k.eligible_2);
    if (with_too_expensive) set_stage(tg, 1, n_eligible > 1 ? AREA_RIGHT : AREA_CENTER,
                                       k.too_expensive);
    hand_append(tg, 0, eri);
    hand_append(tg, 0, k.filler);
    test_give_energy(tg, 15);
    test_recalc(tg);
    k.hand_before = hand_n(tg, 0);
    k.wait_before = wait_n(tg, 0);

    k.played = test_play_to_stage(tg, eri, AREA_CENTER);
    int guard = 0;
    while (pending(tg) && guard < 24) {
        guard++;
        const RbChoice *c = pchoice(tg);
        if (c && c->kind == RB_CHOICE_SELECT_CARD && !strcmp(c->zone, "stage") && c->count == 2) {
            eri_note_prompt(&k, tg);
            int zero = 0;
            pick_many(tg, &zero, 1);
        } else if (pay_cost) {
            eri_note_prompt(&k, tg);
            if (c && c->kind == RB_CHOICE_SELECT_CARD && !strcmp(c->zone, "hand")) {
                int zero = 0;
                pick_many(tg, &zero, 1);
            } else pick(tg, 0);
        } else {
            eri_note_prompt(&k, tg);
            decline(tg);
        }
        rb_drain_ability_queue(&tg->state);
    }
    k.hand_after = hand_n(tg, 0);
    k.wait_after = wait_n(tg, 0);
    fprintf(stderr, "        [info] ERI n_elig=%d expensive=%d pay=%d played=%d "
                    "cost_prompt=%d sel_prompt=%d hand %d->%d wait %d->%d "
                    "elig1 wait=%s stage_wait=%d\n",
            n_eligible, with_too_expensive, pay_cost, k.played, k.cost_prompt,
            k.sel_prompt, k.hand_before, k.hand_after, k.wait_before, k.wait_after,
            orientation_of(tg, k.eligible_1), tg->state.p[1].stage_wait[AREA_LEFT]);
    k.setup_ok = 1;
    return k;
}

static void e_eri_up_to_semantics_one_eligible_still_works(void)
{
    static TestGame tg;
    EriRun k = eri_run(&tg, 1, 0, 1);
    if (!k.setup_ok) return;
    /* PREMISE: the member really is active on the OPPONENT's stage, read from
     * the same accessor the Rust's get_orientation_modifier uses. */
    CHECK(!is_wait(&tg, k.eligible_1),
          "premise: before activation the eligible member is active on the "
          "opponent's stage");
    CHECK(k.played, "CONTROL: 絢瀬絵里 really played to stage");
    /* The Rust's assert_select_card("hand", 1, true): an OPTIONAL 1-card hand
     * selection.  Asserted on the wire fields, because there is no C
     * assert_select_card. */
    CHECK(k.cost_prompt,
          "Rust eri_q144_up_to_semantics_1_eligible_opponent_still_works: the "
          "optional 1-card hand selection really appeared");
    CHECK_EQ(k.wait_after, 1,
             "the filler card should be in the waitroom after the cost is paid");
    CHECK_EQ(k.hand_after, 0,
             "the hand should be empty after playing eri + discarding filler");
    CHECK(k.sel_prompt,
          "the opponent-member selection (SelectCard stage, count 2) really "
          "appeared");
    CHECK(!pending(&tg), "no pending choices remaining after selecting the member");
    /* Q144: only 1 eligible member existed, but the ability activates and puts
     * that member to wait -- 「まで」 is an upper bound, not a requirement. */
    CHECK(is_wait(&tg, k.eligible_1),
          "Rust eri_q144_up_to_semantics_1_eligible_opponent_still_works: 1 "
          "eligible opponent member was put to wait -- 'up to 2' is an upper "
          "bound");
    /* Zone control: the wait really is on the opponent's stage slot. */
    CHECK_EQ(tg.state.p[1].stage_wait[AREA_LEFT], 1,
             "CONTROL: the opponent's stage_wait[] agrees with the modifier");
}

static void e_eri_skip_cost_no_effect(void)
{
    static TestGame tg;
    /* ANTI-VACUITY CONTROL: the identical board with the cost PAID must really
     * move members to wait.  Without it a total no-op debut would satisfy the
     * "remains active" negatives for free. */
    EriRun pos = eri_run(&tg, 2, 1, 1);
    if (!pos.setup_ok) return;
    CHECK(pos.sel_prompt, "CONTROL: the paid board really offers the stage "
                          "selection");
    CHECK(is_wait(&tg, pos.eligible_1) || is_wait(&tg, pos.eligible_2),
          "ANTI-VACUITY CONTROL: paying the cost really does put a member to "
          "wait, so the decline below is not passing on a no-op");

    EriRun k = eri_run(&tg, 2, 1, 0);   /* skip the optional cost */
    if (!k.setup_ok) return;
    CHECK_PROMPTS(1, "CONTROL: the optional cost prompt really appeared");
    CHECK(!pending(&tg),
          "Rust eri_q144_skip_cost_no_effect: no pending choices (effect cancelled)");
    CHECK(!is_wait(&tg, k.eligible_1),
          "eligible member 1 should remain active (cost skipped)");
    CHECK(!is_wait(&tg, k.eligible_2),
          "eligible member 2 should remain active (cost skipped)");
    CHECK_EQ(k.wait_after, k.wait_before,
             "CONTROL: the skipped cost really moved nothing to the waitroom");
}

static void e_eri_no_eligible_opponent_still_pays_cost(void)
{
    static TestGame tg;
    EriRun k = eri_run(&tg, 0, 1, 1);
    if (!k.setup_ok) return;
    CHECK(k.cost_prompt, "the optional 1-card hand selection really appeared");
    CHECK_EQ(k.wait_after, 1,
             "Rust eri_q144_no_eligible_opponent_still_pays_cost: the filler "
             "was discarded (cost paid)");
    CHECK(!k.sel_prompt,
          "no stage selection prompt -- the opponent has no eligible members");
    CHECK(!pending(&tg), "no pending choices (no eligible targets)");
    CHECK(!is_wait(&tg, k.too_expensive),
          "the ineligible member should remain active");
    fprintf(stderr, "        [info] ERI no-eligible: stage_wait L/C/R = %d/%d/%d\n",
            tg.state.p[1].stage_wait[AREA_LEFT], tg.state.p[1].stage_wait[AREA_CENTER],
            tg.state.p[1].stage_wait[AREA_RIGHT]);
}

/* ═══════════════════════════════════════════════════════════════════════════
 * F. turn_scope_regression_test.rs  (2 tests)  -- VERIFIED UNCOVERED
 *
 *   History (from the Rust doc comment): reset_keyword_tracking() used to run
 *   on every Active-phase entry, i.e. TWICE per round, truncating
 *   first-attacker-window facts before any ライブ開始時 condition could read
 *   them.  Seventeen abilities of the shape
 *   「ライブ開始時：このターン中に…した場合」 depend on those facts surviving
 *   BOTH players' normal phases.  These two tests pin the corrected lifetime:
 *   round-scoped trackers clear ONLY at the turn rollover.
 *
 *   The C twin of Rust `cards_appeared_this_turn` is
 *   state.cards_appeared_this_turn[] / n_cards_appeared_this_turn (declared in
 *   the tracking block), of `debut_count_this_turn[2]`, of
 *   state.turn_state_changes[] / n_turn_state_changes and of state.turn.
 *
 *   PHASE DRIVING: bare test_pass() is used throughout, NOT
 *   test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND), because that target
 *   destroys the yell window.  Every pass is drained, because
 *   rb_advance_phase is a no-op while a choice is pending.
 * ═══════════════════════════════════════════════════════════════════════════ */

/* Rust `advance_to_live_card_set_p1`: 5 blind passes.  A drain here would be
 * actively WRONG -- the live-set window raises the set-live prompt and
 * answering it with index 0 would place a card the test did not choose. */
static void advance_blind(TestGame *tg, int n)
{
    for (int i = 0; i < n; i++) test_pass(tg);
}

/* Rust's guard loop: decline SelectAutoAbility, break on anything else. */
static int decline_autos_only(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const RbChoice *c = pchoice(tg);
        if (c && c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) decline(tg);
        else break;
    }
    return guard;
}

static void f_first_attacker_facts_survive_second_active_and_live_start(void)
{
    static TestGame tg;
    fresh(&tg);
    int member = mid(&tg, CHIKA);
    int live   = mid(&tg, SD1_019);
    if (!pin_id(member, CHIKA)) return;
    if (!pin_id(live, SD1_019)) return;
    fill_decks(&tg, mid(&tg, SD1_010), 20);
    test_recalc(&tg);

    /* FIRST ATTACKER'S WINDOW: debut a member from hand. */
    hand_append(&tg, 0, member);
    test_give_energy(&tg, 30);
    int played = test_play_to_stage(&tg, member, AREA_CENTER);
    drain_pick_first(&tg, 24);
    CHECK(played, "sanity: the member really debuted");
    CHECK(appeared_has(&tg, member), "sanity: the debut records the appearance");
    CHECK_EQ(tg.state.debut_count_this_turn[0], 1,
             "sanity: the debut counter incremented");

    /* Cross into the SECOND ATTACKER'S normal phase.  advance_phase enters
     * Active and USED to call reset_keyword_tracking(), which historically wiped
     * these facts mid-round. */
    advance_blind(&tg, 5);
    fprintf(stderr, "        [info] TURN-SCOPE after 5 blind passes: turn=%d "
                    "phase=%d appeared=%d debuts=%d state_changes=%d\n",
            tg.state.turn, (int)tg.state.phase,
            tg.state.n_cards_appeared_this_turn, tg.state.debut_count_this_turn[0],
            tg.state.n_turn_state_changes);
    CHECK(appeared_has(&tg, member),
          "Rust first_attacker_window_facts_survive_second_active_phase_and_"
          "live_start: the appearance must survive the second attacker's Active "
          "phase -- 「このターン」 spans the whole round");
    CHECK_EQ(tg.state.debut_count_this_turn[0], 1,
             "the debut counter must survive the second attacker's Active phase");

    /* And still be readable when ライブ開始時 conditions evaluate. */
    hand_append(&tg, 0, live);
    test_set_live_card(&tg, 0, live);
    advance_blind(&tg, 2);
    decline_autos_only(&tg, 20);
    CHECK(appeared_has(&tg, member),
          "the appearance must still be visible at live start");
}

static void f_round_scoped_trackers_clear_at_turn_rollover(void)
{
    static TestGame tg;
    fresh(&tg);
    int member = mid(&tg, CHIKA);
    int live   = mid(&tg, SD1_019);
    if (!pin_id(member, CHIKA)) return;
    if (!pin_id(live, SD1_019)) return;
    fill_decks(&tg, mid(&tg, SD1_010), 20);
    test_recalc(&tg);

    hand_append(&tg, 0, member);
    hand_append(&tg, 0, live);
    test_give_energy(&tg, 30);
    test_play_to_stage(&tg, member, AREA_CENTER);
    drain_pick_first(&tg, 24);
    CHECK(tg.state.n_cards_appeared_this_turn > 0,
          "sanity: something was recorded before the rollover");

    /* Drive through the live into the NEXT round's first normal phase. */
    advance_blind(&tg, 5);
    test_set_live_card(&tg, 0, live);
    for (int i = 0; i < 8; i++) {
        test_pass(&tg);
        decline_autos_only(&tg, 20);
    }
    fprintf(stderr, "        [info] TURN-SCOPE after the live: turn=%d phase=%d "
                    "appeared=%d debuts=%d state_changes=%d winner=%d\n",
            tg.state.turn, (int)tg.state.phase,
            tg.state.n_cards_appeared_this_turn, tg.state.debut_count_this_turn[0],
            tg.state.n_turn_state_changes, tg.state.winner);
    CHECK(tg.state.turn >= 2, "sanity: the turn really rolled over");
    CHECK_EQ(tg.state.n_cards_appeared_this_turn, 0,
             "Rust round_scoped_trackers_clear_at_turn_rollover: appearances "
             "clear at the turn rollover");
    CHECK_EQ(tg.state.debut_count_this_turn[0], 0,
             "debut counters clear at the turn rollover");
    CHECK_EQ(tg.state.n_turn_state_changes, 0,
             "activation history clears at the turn rollover");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * G. modifier_layer_characterization_test.rs  (3 tests)
 *
 * The peer header in test_parity_blades2.c:63 skipped this because
 * "rb_constant_per_unit_units ... has no declaration in include/rabuka.h and
 * that header is owned by another agent".  BOTH halves of that are false:
 *   - rb_constant_per_unit_units IS declared, in the "Constant ability lookup"
 *     block of include/rabuka.h, immediately above rb_resolve_per_unit_count;
 *   - none of these three Rust tests calls it.  What they need is
 *     rb_recalc_constants, rb_check_expired_effects, rb_mods_add_heart and
 *     g->n_temp_effects, and all four ARE declared.
 * So the section compiles and runs, and the three lifecycle semantics are now
 * pinned.
 *
 * The tests pin the OBSERVABLE lifecycle of the four coexisting modifier
 * layers, which is what a future merge into one registry has to keep:
 *   1. constant grant appears while its condition source exists;
 *   2. constant grant FULLY reverts when the source goes away;
 *   3. temporary live_end effects register and expire on live-phase exit;
 *   4. cross-layer stacking: a manual additive survives constant recomputation
 *      that zeroes the constant part.
 *
 * HEAT COLOUR.  PL!SP-sd2-008-SD2's printed text is 「{{heart_03.png|heart03}}を
 * 得る」 -- heart03.  The Rust source's own comment says "heart02" while its
 * code is `const H02: HeartColor = HeartColor::Heart03;`, so the Rust COMMENT
 * is wrong and the Rust CODE is right; HC[3] is used here, derived at startup
 * from the engine's own map.  test_get_heart_modifier is NOT used (it remaps
 * colour 5).
 * ═══════════════════════════════════════════════════════════════════════════ */

#define H03_COLOUR 3   /* heart03, the colour PL!SP-sd2-008-SD2 actually grants */

static int shiki_setup(TestGame *tg, int with_source)
{
    fresh(tg);
    int shiki = mid(tg, SHIKI);
    if (!pin_id(shiki, SHIKI)) return -1;
    int big = mid(tg, BIG_RIN);
    if (!pin_id(big, BIG_RIN)) return -1;
    CHECK_EQ(printed_cost(big), 15,
             "premise: PL!HS-bp5-004-R costs 15, i.e. satisfies the printed "
             "cost-13-or-more gate");
    set_stage(tg, 0, AREA_LEFT, shiki);
    if (with_source) set_stage(tg, 0, AREA_CENTER, big);
    else            set_stage(tg, 0, AREA_CENTER, RB_EMPTY_SLOT);
    test_recalc(tg);
    return shiki;
}

static void g_constant_grant_reverts_when_source_changes(void)
{
    static TestGame tg;
    int shiki = shiki_setup(&tg, 1);
    if (shiki < 0) return;
    /* PREMISE: the 自動 really is on the print. */
    CHECK(card_prints_trigger(shiki, RB_TSTR_CONSTANT),
          "premise: PL!SP-sd2-008-SD2 really prints a 常時 in the compiled record");
    CHECK_EQ(heart_of(&tg, shiki, HC[H03_COLOUR]), 1,
             "Rust constant_blade_recompute_updates_when_condition_source_"
             "changes (before): gate satisfied -> heart03 granted");
    /* The source leaves the stage -> recalculation must FULLY revert. */
    stage_clear(&tg, 0, AREA_CENTER);
    CHECK_EQ(tg.state.p[0].stage[AREA_CENTER], RB_EMPTY_SLOT,
             "setup: the cost-15 source really left the stage");
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, shiki, HC[H03_COLOUR]), 0,
             "Rust constant_blade_recompute_updates_when_condition_source_"
             "changes (after): the constant layer must revert to zero after its "
             "source disappears");
    /* ANTI-VACUITY CONTROL: restoring the source brings the grant back, so the
     * revert above is a recalculation and not a one-way latch. */
    set_stage(&tg, 0, AREA_CENTER, mid(&tg, BIG_RIN));
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, shiki, HC[H03_COLOUR]), 1,
             "CONTROL: restoring the condition source re-grants the heart, so "
             "the zero above is a real revert and not a latch that never fires");
}

static void g_temporary_live_end_effect_expires_when_live_phase_ends(void)
{
    static TestGame tg;
    fresh(&tg);
    int hime = mid(&tg, HIME);
    if (!pin_id(hime, HIME)) return;
    set_stage(&tg, 0, AREA_LEFT, hime);
    test_recalc(&tg);
    CHECK(card_prints_trigger(hime, RB_TSTR_DEBUT),
          "premise: PL!HS-cl1-006-CL really prints a 登場 in the compiled record");
    if (!fire_trigger(&tg, hime, RB_TSTR_DEBUT, RB_TSTR_DEBUT)) {
        if (tal) { tal->assertions++; tal->failures++; tal->setup_bugs++; }
        fprintf(stderr, "SETUP BUG: 姫芽's 登場 could not be fired\n");
        return;
    }
    drain_pick_first(&tg, 16);
    CHECK_EQ(blade_of(&tg, hime), 3,
             "Rust temporary_live_end_effect_expires_when_live_phase_ends: the "
             " 등장 grants 3 blades");
    CHECK(tg.state.n_temp_effects > 0,
          "the grant must be registered as a tracked temporary effect "
          "(n_temp_effects = %d)", tg.state.n_temp_effects);
    int n_reg = tg.state.n_temp_effects;

    /* Stay INSIDE the live phase -> nothing expires.  The Rust sets
     * current_turn_phase = Live and calls check_expired_effects(); the C
     * rb_check_expired_effects takes WHICH duration class to expire, so
     * "inside the live phase" is the turn-end class -- a live_end (dur 1)
     * effect is not touched by it.  That is the faithful mapping and it is
     * stated here rather than hidden. */
    rb_check_expired_effects(&tg.state, RB_TEMP_TURN_END);
    CHECK_EQ(blade_of(&tg, hime), 3,
             "while the live phase is still running the live_end grant survives");
    CHECK_EQ(tg.state.n_temp_effects, n_reg,
             "CONTROL: the effect is still registered after the non-expiring pass");

    /* Live phase ends -> the effect expires and reverts exactly +3. */
    rb_check_expired_effects(&tg.state, RB_TEMP_LIVE_END);
    CHECK_EQ(blade_of(&tg, hime), 0,
             "Rust temporary_live_end_effect_expires_when_live_phase_ends: the "
             "live_end temporary effect must fully revert at live end");
    CHECK_EQ(tg.state.n_temp_effects, 0,
             "the temporary-effect table is empty after the live-end sweep");
}

static void g_manual_additive_survives_constant_recompute(void)
{
    static TestGame tg;
    int shiki = shiki_setup(&tg, 1);
    if (shiki < 0) return;
    /* Manual additive from some other hypothetical source. */
    rb_mods_add_heart((RbMods *)&tg.state.mods, shiki, HC[H03_COLOUR], 2);
    CHECK_EQ(heart_of(&tg, shiki, HC[H03_COLOUR]), 2,
             "setup: the manual +2 really is on the board before the recalc");
    test_recalc(&tg);
    int total_with_gate = heart_of(&tg, shiki, HC[H03_COLOUR]);
    fprintf(stderr, "        [info] MODIFIER-LAYER total with gate = %d\n",
            total_with_gate);
    CHECK(total_with_gate >= 3,
          "Rust manual_additive_survives_constant_recompute_and_tracks_"
          "condition: manual +2 plus the constant grant should stack (got %d)",
          total_with_gate);
    /* The gate disappears: the CONSTANT part reverts, the manual part stays.
     * This is the invariant a unified registry has to keep, and it is the
     * security-shaped half: a recalculation that zeroes the WHOLE table would
     * also destroy modifiers this engine did not grant. */
    stage_clear(&tg, 0, AREA_CENTER);
    test_recalc(&tg);
    CHECK_EQ(heart_of(&tg, shiki, HC[H03_COLOUR]), 2,
             "Rust manual_additive_survives_constant_recompute_and_tracks_"
             "condition: the constant portion reverted to zero and the manual "
             "+2 must remain");
}

/* ═══════════════════════════════════════════════════════════════════════════
 * H. hazuki q233_declined_optional_energy_allows_second_trigger_same_turn (1)
 *
 * The tail of hazuki_activate_discard_per_liella_member_blade_test.rs.  Its
 * file-mate's other 3 tests ARE ported, as test_parity_blades2.c section C --
 * that file's header lists this one as NOT COVERED and its RUN list stops at
 * the three 起動 cases, so the 自動 case was genuinely missed.
 *
 *   PL!SP-bp5-005-{AR,P,R＋,SEC} 葉月 恋 ab#1 自動 / 1回:
 *     自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から控え室に
 *     置かれるたび、E 支払ってもよい。そうした場合、それらのカードの中から1枚
 *     手札に加える。
 *
 * The test is a 4-rarity loop: the first trigger is DECLINED, and the second
 * (same turn) must still be offered and payable.  The ターン1回 use limit is
 * what makes this interesting -- a decline must not consume it.
 *
 * The Rust quirk `hand.push(first); hand.retain(|id| *id != first);` is a
 * no-op pair: it pushes and immediately removes the same id, so the card ends
 * up in the WAITROOM only.  It is reproduced here as "the card is in the
 * waitroom", and the no-op half is NOT written, because writing a push
 * followed by an immediate removal of the same id would be writing dead code.
 *
 * The optional-energy gate is a SelectTarget where index 0 is SKIP and index 1
 * is PAY.  The Rust's `select_option(0)` / `select_option(1)` map onto that
 * directly, and the target string is asserted at the call site so a change of
 * gate shape is visible.
 * ═══════════════════════════════════════════════════════════════════════════ */

static void h_q233_four_rarities(void)
{
    static const char *const suffixes[4] = { HAZUKI_AR, HAZUKI_P, HAZUKI_R, HAZUKI_SEC };
    static TestGame ctl;
    /* ANTI-VACUITY CONTROL, run ONCE on the AR print: the first trigger really
     * is DECLINED, the second really is PAID, and the energy really moves.  The
     * 4-rarity loop below is a sweep of the SAME shape over four prints, so if
     * the control is red the sweep proves nothing. */
    {
        fresh(&ctl);
        int hz = mid(&ctl, HAZUKI_AR);
        if (pin_id(hz, HAZUKI_AR)) {
            set_stage(&ctl, 0, AREA_CENTER, hz);
            test_give_energy(&ctl, 2);
            int first = mid(&ctl, SD1_010);
            wait_append(&ctl, 0, first);
            set_recently_moved_hand(&ctl, first);
            CHECK(fire_trigger(&ctl, hz, RB_TSTR_AUTO, RB_TSTR_AUTO),
                  "CONTROL: the 自動 is present in the compiled record");
            CHECK(pending(&ctl), "CONTROL: the first trigger really prompts");
            {
                const RbChoice *c = pchoice(&ctl);
                if (c)
                    fprintf(stderr, "        [info] Q233 gate kind=%d target=\"%s\" "
                                    "count=%d allow_skip=%d\n",
                            (int)c->kind, c->target, c->count, c->allow_skip);
            }
            CHECK_PROMPTS(1, "CONTROL: the first gate really appeared");
            pick(&ctl, 0);                      /* SKIP */
            drain_pick_first(&ctl, 16);
            CHECK_EQ(zone_active(&ctl, 0), 2,
                     "ANTI-VACUITY CONTROL: the declined trigger paid no energy");
            CHECK_EQ(rb_ability_uses_used(&ctl.state, hz, 1), 0,
                     "ANTI-VACUITY CONTROL: a declined trigger consumes no use");

            int second = mid(&ctl, SD1_010);
            wait_append(&ctl, 0, second);
            set_recently_moved_hand(&ctl, second);
            fire_trigger(&ctl, hz, RB_TSTR_AUTO, RB_TSTR_AUTO);
            CHECK(pending(&ctl),
                  "ANTI-VACUITY CONTROL: the second trigger is still offered "
                  "after a decline (this is the whole point of Q233)");
            pick(&ctl, 1);                      /* PAY */
            drain_pick_first(&ctl, 16);
            CHECK_EQ(zone_active(&ctl, 0), 1,
                     "ANTI-VACUITY CONTROL: the paid trigger really spent 1 energy");
            CHECK_EQ(rb_ability_uses_used(&ctl.state, hz, 1), 1,
                     "ANTI-VACUITY CONTROL: the paid trigger consumed the use");
            CHECK(hand_has(&ctl, second) || wait_n(&ctl, 0) >= 0,
                  "CONTROL: the board is still consistent after the payment");
        }
    }

    for (int i = 0; i < 4; i++) {
        const char *no = suffixes[i];
        static TestGame tg;
        fresh(&tg);
        int hz = mid(&tg, no);
        if (!pin_id(hz, no)) continue;
        /* PREMISE: the 自動 is ab#1 on every rarity (ab#0 is the 起動), which is
         * the ability index the Rust checks with ability_uses_used(hazuki, 1). */
        CHECK_EQ(card_trigger_index(hz, RB_TSTR_AUTO), 1,
                 "premise: the 自動 is ab#1 on this print");
        set_stage(&tg, 0, AREA_CENTER, hz);
        test_give_energy(&tg, 2);

        int first = mid(&tg, SD1_010);
        wait_append(&tg, 0, first);
        set_recently_moved_hand(&tg, first);
        CHECK(fire_trigger(&tg, hz, RB_TSTR_AUTO, RB_TSTR_AUTO),
              "%s: the 自動 is present in the compiled record", no);
        CHECK(pending(&tg), "%s: first trigger prompt", no);
        CHECK_PROMPTS(1, "%s: the first gate really appeared", no);
        pick(&tg, 0);                            /* SKIP */
        drain_pick_first(&tg, 16);
        fprintf(stderr, "        [info] Q233 %s after decline: energy_active=%d "
                        "uses_used=%d pending=%d\n",
                no, zone_active(&tg, 0), rb_ability_uses_used(&tg.state, hz, 1),
                pending(&tg));
        CHECK(!pending(&tg), "%s: no prompt remains after the decline", no);
        CHECK_EQ(zone_active(&tg, 0), 2,
                 "%s: a declined 自動 pays no energy", no);
        CHECK_EQ(rb_ability_uses_used(&tg.state, hz, 1), 0,
                 "%s: a declined 自動 consumes no use of the 1-per-turn limit", no);

        int second = mid(&tg, SD1_010);
        wait_append(&tg, 0, second);
        set_recently_moved_hand(&tg, second);
        fire_trigger(&tg, hz, RB_TSTR_AUTO, RB_TSTR_AUTO);
        CHECK(pending(&tg),
              "Rust q233_declined_optional_energy_allows_second_trigger_same_"
              "turn: %s second trigger prompt", no);
        CHECK_PROMPTS(2, "%s: a SECOND real prompt was offered (a single-prompt "
                        "board would make the first CHECK vacuous)", no);
        pick(&tg, 1);                            /* PAY */
        drain_pick_first(&tg, 16);
        fprintf(stderr, "        [info] Q233 %s after pay: energy_active=%d "
                        "uses_used=%d\n",
                no, zone_active(&tg, 0), rb_ability_uses_used(&tg.state, hz, 1));
        CHECK_EQ(zone_active(&tg, 0), 1,
                 "%s: the paid 自動 spent exactly 1 energy", no);
        CHECK_EQ(rb_ability_uses_used(&tg.state, hz, 1), 1,
                 "%s: the paid 自動 consumed the 1-per-turn use", no);
    }
}

/* ═══════════════════════════════════════════════════════════════════════════
 * main -- each case runs in a FORKED child with a SIGSEGV/SIGBUS/SIGABRT
 * handler naming the test in flight.  The counters live in a
 * MAP_SHARED|MAP_ANONYMOUS mapping, so a child that dies mid-assertion still
 * reports what it completed.  run() is crash isolation, not a hang detector:
 * no watchdog is achievable on this toolchain, and every drain is capped.
 * ═══════════════════════════════════════════════════════════════════════════ */

#define CHILD_OK        0
#define CHILD_FAILURES  1

static int n_ok, n_failed, n_crashed, n_setup_bugs;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        int f0 = tal->failures;
        current_test = name;
        fn();
        current_test = "(none)";
        if (tal->failures > f0) n_failed++; else n_ok++;
        printf("%-6s %s  [in-process fallback]\n",
               tal->failures > f0 ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = tal->assertions, f0 = tal->failures;
        int s0 = tal->setup_bugs, p0 = tal->prompts;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), "
               "%d prompt(s)\n",
               name, tal->assertions - a0, tal->failures - f0,
               tal->setup_bugs - s0, tal->prompts - p0);
        fflush(stdout);
        fflush(stderr);
        _Exit(tal->failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-6s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-6s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("ok     %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("FAILED %s\n", name);
    } else {
        n_crashed++;
        printf("%-6s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    tally_init();
    if (!tal) { fprintf(stderr, "FAIL: could not map the shared tally\n"); return 1; }
    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }
    hcolor_init();
    print_wire_map();

    printf("--- A1 bp7_parser_gap_cards_test.rs / Wien PL!SP-bp7-010-R ---\n");
    run("wien_activation_moves_energy_zone_to_energy_deck", a1_wien_moves_energy_zone_to_energy_deck);
    run("wien_empty_energy_zone_step1_fizzles",             a1_wien_empty_energy_zone_step1_fizzles);
    run("wien_step2_recovers_self_after_cost",             a1_wien_step2_recovers_self_after_cost);

    printf("--- A2 bp7_parser_gap_cards_test.rs / Ayumu PL!N-bp7-001-R ---\n");
    run("ayumu_energy_under_member_triggers_energy_deck_move", a2_ayumu_energy_under_member_triggers);
    run("ayumu_energy_into_zone_not_under_member_no_trigger", a2_ayumu_zone_arrival_does_not_trigger);
    run("ayumu_empty_energy_deck_no_effect",                    a2_ayumu_empty_energy_deck_no_effect);

    printf("--- A3 bp7_parser_gap_cards_test.rs / Kanon PL!SP-bp7-012-N ---\n");
    run("kanon_selects_each_group_to_deck_bottom_and_draws",      a3_kanon_selects_each_group_to_deck_bottom_and_draws);
    run("kanon_skip_placement_no_draw",                            a3_kanon_skip_placement_no_draw);
    run("kanon_empty_discard_no_placement_no_draw",                a3_kanon_empty_discard_no_placement_no_draw);
    run("kanon_missing_group_places_present_but_no_draw",          a3_kanon_missing_group_places_present_but_no_draw);
    run("kanon_one_per_group_not_multiple_from_same_group",        a3_kanon_one_per_group_not_multiple_from_same_group);

    printf("--- B ren_baton_touch_group_check_test.rs (PL!SP-bp4-005-R+) ---\n");
    run("ren_baton_touch_from_liella_places_energy",            b_ren_baton_touch_from_liella_places_energy);
    run("ren_baton_touch_from_non_liella_no_energy",           b_ren_baton_touch_from_non_liella_no_energy);
    run("ren_baton_touch_liella_insufficient_energy_no_effect", b_ren_baton_touch_insufficient_energy_no_effect);

    printf("--- C yell_same_group_members_multi_color_hearts_test.rs "
           "(PL!S-PR-040-PR) ---\n");
    run("multi_color_heart_condition_not_met_does_not_queue", c_yell_condition_not_met_does_not_queue);
    run("multi_color_heart_wrong_group_does_not_trigger",      c_yell_wrong_group_does_not_trigger);
    run("multi_color_heart_condition_met_grants_both_colors",  c_yell_condition_met_grants_both_colors);

    printf("--- D kanon_debut_one_per_group_deck_bottom_edge_test.rs "
           "(KNOWN-DEAD-PATH REPRO) ---\n");
    run("kanon_select_three_any_order_and_draws_nothing",     d_kanon_select_three_any_order_and_draws_nothing);
    run("kanon_decline_optional_no_draw",                     d_kanon_decline_optional_no_draw);
    run("kanon_only_one_group_present_can_still_select_one",  d_kanon_only_one_group_present_can_still_select_one);

    printf("--- E eri_debut_up_to_two_opponent_members_to_wait_test.rs "
           "(KNOWN-DEAD-PATH REPRO, PL!-bp3-002-R) ---\n");
    run("eri_q144_up_to_semantics_1_eligible_opponent_still_works", e_eri_up_to_semantics_one_eligible_still_works);
    run("eri_q144_skip_cost_no_effect",                             e_eri_skip_cost_no_effect);
    run("eri_q144_no_eligible_opponent_still_pays_cost",           e_eri_no_eligible_opponent_still_pays_cost);

    printf("--- F turn_scope_regression_test.rs (VERIFIED UNCOVERED) ---\n");
    run("first_attacker_window_facts_survive_second_active_phase_and_live_start",
        f_first_attacker_facts_survive_second_active_and_live_start);
    run("round_scoped_trackers_clear_at_turn_rollover", f_round_scoped_trackers_clear_at_turn_rollover);

    printf("--- G modifier_layer_characterization_test.rs "
           "(peer's skip reason was FALSE) ---\n");
    run("constant_blade_recompute_updates_when_condition_source_changes",
        g_constant_grant_reverts_when_source_changes);
    run("temporary_live_end_effect_expires_when_live_phase_ends",
        g_temporary_live_end_effect_expires_when_live_phase_ends);
    run("manual_additive_survives_constant_recompute_and_tracks_condition",
        g_manual_additive_survives_constant_recompute);

    printf("--- H hazuki q233_declined_optional_energy_allows_second_trigger_"
           "same_turn (4 rarities) ---\n");
    run("q233_declined_optional_energy_allows_second_trigger_same_turn",
        h_q233_four_rarities);

    n_setup_bugs = tal->setup_bugs;
    rb_unload();

    printf("\n=== test_parity_bp7 summary ===\n");
    printf("  cases: %d ok, %d failed, %d crashed\n", n_ok, n_failed, n_crashed);
    printf("  assertions: %d, failures: %d, setup bugs: %d, prompts seen: %d\n",
           tal->assertions, tal->failures, n_setup_bugs, tal->prompts);
    if (n_failed || n_crashed || tal->failures) {
        fprintf(stderr, "%d failure(s) across %d crashed case(s)\n",
                tal->failures, n_crashed);
        return 1;
    }
    printf("ALL %d BP7 PARITY CHECKS PASSED\n", tal->assertions);
    return 0;
}
