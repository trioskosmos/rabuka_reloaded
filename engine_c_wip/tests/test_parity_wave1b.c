/* test_parity_wave1b.c -- C parity suite for three Rust clusters whose cards
 * appear in NO other C test file (verified by a card-number scan of the whole
 * engine_c_wip tree, not by a header claim).
 *
 * CLAIM (tr_wave1b): this file is the ONLY file I own.  I edit nothing else.
 * The Rust files I translate are, in full:
 *   engine/tests/test_modules/jidou/combination/hs_pb1_001_jidou_activated_energy_funds_live_start_payment_test.rs
 *   engine/tests/test_modules/jidou/combination/hs_pb1_003_turn2_second_discard_event_grants_again_test.rs
 *   engine/tests/test_modules/jidou/combination/hs_pb1_009_jidou_blades_charge_live_start_threshold_test.rs
 *   engine/tests/test_modules/jidou/combination/rurino_stage_to_waitroom_watcher_baseline_test.rs
 *   engine/tests/test_modules/jidou/combination/sp_bp5_005_mill_arms_jidou_recover_test.rs
 *   engine/tests/test_modules/jidou/combination/yell_real_yell_no_blade_heart_grants_heart_test.rs
 *   engine/tests/test_modules/effects/modify_need_heart/pb2_need_heart_minus_3_minus_2_minus_1_per_activated_pl_pb2_040_l_test.rs
 *   engine/tests/test_modules/effects/cost/play_cost/pb2_optional_two_distinct_printemps_wait_costs_two_less_pl_pb2_012_p_plus_test.rs
 * (31 #[test] functions in total, all ported -- see main()'s RUN list.)
 *
 * WHY THESE: a jidou whose effect FUNDS the next trigger is the interaction
 * the C corpus is thinnest on, and PL!-pb2-040-L / PL!-pb2-012-P+ / the two
 * 蓮ノ空 019 prints / PL!SP-bp5-005-R+ are all new card numbers to it.
 *
 * ── NO JAPANESE IN A FUNCTIONAL POSITION ──────────────────────────────────
 * The editor can emit CP932 and silently corrupt Japanese literals.  Every
 * group name, series name and card name used by an ASSERTION is a byte escape
 * whose bytes were read out of cards/cards.json / engine_c_wip/src/core/card.c
 * (see the comments at each).  Trigger strings come from the engine's own
 * RB_TSTR_* constants.  This file is ASCII-only; it is verified after write.
 *
 * ── GROUP NAMES: rb_card_group_name is SERIES-LEVEL ───────────────────────
 * cards.json has no `group` key on any card; it has `unit` (2,198 cards) and
 * `series` (2,639).  src/core/card.c:228-235 maps a series to one of the five
 * canonical group names (μ's / Aqours / 虹ヶ咲 / Liella! / 蓮ノ空) and
 * src/ability/util.c:295-300 matches a group_name against unit FIRST, then
 * against the series.  So:
 *   - 「スリーズブeke」  is a UNIT   -> rb_card_unit_name
 *   - 「蓮ノ空」 and 「Liella!」 are SERIES-DERIVED -> rb_card_group_name
 * All of them are PRINTED at startup so a mismatch is a visible name, never a
 * silent "no match".
 *
 * ── LANDMINES RESPECTED ──────────────────────────────────────────────────
 *   - load_card_db() falls back to ../cards/build: rb_load("src") fails under
 *     an isolated out-of-tree build.
 *   - sizeof(TestGame) is ~781 KB: every TestGame is `static`.
 *   - test_get_heart_modifier REMAPS colour 5, so no heart is read through it;
 *     rb_mods_get_heart / rb_mods_get_need_heart are called directly and the
 *     heartNN -> enum map is derived at run time from the engine's own
 *     rb_heart_color_as_str / rb_parse_heart_color and PRINTED.
 *   - rb_card_string() takes a STRING-TABLE INDEX: card_no_of() decodes the Card
 *     and reads c.card_no_idx.  pin_id() compares byte-for-byte; rb_card_no_eq
 *     is NOT used as an identity pin (a peer proved it fuzzy).
 *   - card_no_of() uses a ROTATING ring.  One shared static buf called 2-3
 *     times in one printf prints the LAST card repeatedly: a false-red maker.
 *   - rb_card_copy_pool_reset() runs per case (the pool is process-global).
 *   - The C deck bag is TOP-INDEX-0, so Rust `cards.push(x)` is an APPEND and
 *     the BOTTOM of the deck is the END of the array.  A pushed list [a,b,o]
 *     that Rust means to be the top three is written a,b,o at indices 0,1,2.
 *   - test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) DESTROYS the yell
 *     window, so the yell section steps one pass at a time.
 *   - The waitroom zone string is "discard" (the C zone id, not "waitroom").
 *   - A `pay_optional_cost:skip` gate is a SelectTarget whose option 0 is SKIP
 *     and 1 is PAY.  Declining a SelectAutoAbility is NOT declining a cost.
 *   - scan_moved() clears g->just_completed_ability_key FIRST, mirroring
 *     rb_trigger_auto_abilities (game_state_abilities.c:1795); queue_zone_
 *     abilities skips any ability whose key equals that field
 *     (game_state_abilities.c:1390), so a second scan of the same card on the
 *     same board is silently dropped without the clear.  rb_queue_trigger_
 *     abilities DOES clear n_batch_triggered_keys (line 2331) but NOT the key.
 *   - rb_record_use() is called at ENQUEUE time (game_state_abilities.c:1414),
 *     so rb_use_count() is a direct, non-vacuous proof that a jidou fired.
 *   - A p2-owned queued auto ability can be DESTROYED by a later p1 pass, so
 *     every drain is followed by rb_drain_ability_queue on the same seat.
 *
 * ── ANTI-VACUITY ─────────────────────────────────────────────────────────
 * Every negative assertion carries either CHECK_PROMPTS (a count of prompts
 * really offered, taken at the assertion site) or a PAIRED POSITIVE driven
 * first on the SAME board.  Every case takes a WHOLE-BOARD CARD CENSUS
 * (test_total_card_count) before and after: a zone count of 0 is identical
 * whether a move did nothing or DESTROYED a card, and the census is what tells
 * those apart.  It is load-bearing in A4 (the mill/recover family) and in every
 * §B case, where the "correct" answer is 0 and only a non-zero value on a
 * sibling case proves the ability is reachable at all.
 *
 * run() is crash ISOLATION, not a hang detector -- no watchdog is achievable on
 * this toolchain -- so EVERY drain is capped.  The counters live in a
 * MAP_SHARED|MAP_ANONYMOUS mapping, so a child killed by SIGSEGV/SIGBUS/SIGABRT
 * still reports the assertions it completed before dying.
 */

#include "rabuka.h"
#include "test_game.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <unistd.h>

/* U+FF0B FULLWIDTH PLUS SIGN = EF BC 8B.  rb_find_card_by_no is exact-byte and
 * does NOT normalise fullwidth -> halfwidth, so every such number is spelled
 * byte-exactly.  (The filename token pl_sp_X maps to PL!SP-X, pl_X to PL!-X;
 * filenames and bodies agree, so there is no PL! vs PL!SP mismatch here.) */
#define FP "\xef\xbc\x8b"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ── fixtures ───────────────────────────────────────────────────────────────
 * Every one of these was read out of cards/cards.json read as UTF-8 (NEVER
 * Get-Content -Raw, which mangles non-ASCII and produces false "missing"
 * cards) and cross-checked against the numbers the Rust file asserts. */
#define HANA_001    "PL!HS-pb1-001-R"    /* Nichinaka Hanaho  R  cost 11 blade 3
                                         *   unit  HUZUKAIBEKU
                                         *   series Love Live! Hasunosora       */
#define CB_ALLY     "PL!HS-sd1-012-SD"   /* Momoi Ginko      SD  cost 4  (the 4-cost
                                         *   「ほかの」 appearance trigger)       */
#define RURINO_003  "PL!HS-pb1-003-R"    /* Osawa Rurino     R  cost 15 blade 5
                                         *   unit MIRAKURAPARK!                   */
#define HANAMO_009  "PL!HS-pb1-009-R"    /* Nichinaka Hanaho R  cost 15 blade 4
                                         *   (the printed blade 4 the jidou
                                         *   charges the >= 8 threshold with)    */
#define HASU_ALLY   "PL!HS-sd1-001-SD"   /* Nichinaka Hanaho SD  cost 9
                                         *   series Hasunosora.  NOTE: this is the
                                         *   SAME CHARACTER as HANAMO_009.  The
                                         *   Rust file's group assertion is a
                                         *   SERIES assertion (蓮ノ空), not a unit
                                         *   one -- its unit is HUZUKAIBEKU, and
                                         *   the Rust `assert_card_in_group(., "蓮ノ空")`
                                         *   only holds through the series alias. */
#define HASU_ALLY_2 "PL!HS-sd1-012-SD"   /* Momoi Ginko      SD  cost 4, Hasunosora */
#define RURINO_W    "PL!HS-bp6-019-N"    /* Osawa Rurino     N  cost 5  -- the
                                         *   stage->waitroom watcher (無制限)      */
#define RURINO_RE   "PL!HS-pb1-019-N"    /* Osawa Rurino     N  cost 2  -- the
                                         *   起動 re-entry.  NOTE bp6 vs pb1:
                                         *   TRANSPOSED but both real.  The
                                         *   similarly numbered PL!HS-bp1-019-L is a
                                         *   DIFFERENT card, so this is pinned.   */
#define REN_MILL    "PL!SP-bp5-005-R" FP /* Hazuki Ren       R+ cost 11
                                         *   series Love Live! Superstar!!       */
#define LIELLA_A    "PL!SP-bp1-002-R" FP /* Karu Keke        R+ -- series
                                         *   Superstar!!, so it IS in the 「Liella!」
                                         *   set even though its unit is
                                         *   KALEIDOSCORE.  The group name is
                                         *   SERIES-DERIVED; see the header.      */
#define LIELLA_B    "PL!SP-bp1-004-R"    /* Takanami Sumire  R  -- series
                                         *   Superstar!! -> also in 「Liella!」   */
#define WIEN_YELL   "PL!SP-bp2-021-N"    /* Wien Margarete   N  -> heart03 jidou */
#define SUMIRE_YELL "PL!SP-bp2-015-N"    /* Takanami Sumire  N  -> heart06 jidou */
#define LIVE_SCORED "PL!S-bp3-020-L"     /* Daisuki...      L  score 1 -- a REAL
                                         *   score requirement, so performing it
                                         *   yells.  A 0-score filler does not,
                                         *   which is the whole point of the Rust
                                         *   file.                                  */
#define MEMBER_SCORE "PL!S-sd1-003-SD"   /* Matsuura Kanon   SD cost 11 blade 3  */

#define HIKARI_040  "PL!-pb2-040-L"      /* Love marginal    L  score 7, the
                                         *   required-heart reducer.  cards.json
                                         *   need_heart is {heart01:2, heart03:6,
                                         *   heart0:10} -- note heart0, and NO
                                         *   heart00 key.  The modifier this file
                                         *   reads is the heart00 REQUIREMENT
                                         *   delta, which the card reduces.       */
#define PRINT_1     "PL!-bp6-017-N"      /* Koizumi Hanayo   N  cost 2  Printemps */
#define PRINT_2     "PL!-bp5-012-N"      /* Minami Kotori    N  cost 4  Printemps */
#define PRINT_3     "PL!-pb1-026-N"      /* Koizumi Hanayo   N  cost 4  Printemps */

#define KOTORI_P    "PL!-pb2-012-P" FP   /* Minami Kotori    P+ cost 13
                                         *   unit Printemps, series Love Live!    */
#define NON_PRINT   "PL!S-bp5-012-N"     /* Matsuura Kanon   N  unit AZALEA -- the
                                         *   non-Printemps decoy                 */
#define ARRIVER     "PL!-sd1-002-SD"     /* Yorozuya Eri     SD cost 2            */
#define FILLER      "PL!-sd1-010-SD"     /* Kosaka Honoka    SD cost 4  Printemps */
#define EN_CARD     "LL-E-001-SD"        /* energy card                           */

/* Group names, as the byte escapes of the EXACT strings the engine compares
 * against.  Derived by reading the bytes out of cards.json / card.c.
 * GRP_HASUNO  is the series-derived name (HASUNO) for the Hasunosora set.
 * GRP_LIELLA  is the series-derived name for Love Live! Superstar!!.
 * UNIT_SUZUKE is a real UNIT name on the cards that use it. */
#define GRP_HASUNO  "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA"
#define GRP_LIELLA  "Liella!"
/* UNIT_SUZUKE, from cards.json's `unit` for PL!HS-pb1-001-R / PL!HS-sd1-012-SD
 * / PL!HS-pb1-009-R / PL!HS-sd1-001-SD. */
#define UNIT_SUZUKE "\xE3\x82\xB9\xE3\x83\xAA\xE3\x83\xBC\xE3\x82\xBA" \
                   "\xE3\x83\x96\xE3\x83\xBC\xE3\x82\xB1"

/* ── shared counters (survive a child that dies mid-assertion) ──────────── */

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
    if (tal)
        fprintf(stderr,
                "\n*** FATAL SIGNAL %d inside test: %s\n"
                "*** The engine, not this test, is at fault.\n"
                "*** Completed before the fault: %d assertions, %d failures, "
                "%d prompts\n",
                sig, current_test, tal->assertions, tal->failures, tal->prompts);
    else
        fprintf(stderr, "\n*** FATAL SIGNAL %d inside test: %s\n", sig, current_test);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(cond, ...) do {                                             \
        if (tal) tal->assertions++;                                       \
        if (!(cond)) {                                                    \
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

/* VACUITY CONTROL.  Counts the prompts really offered up to the assertion
 * site (the tally is bumped in pick()/answer() at answer time AND the
 * assertion site can ask what is ON SCREEN now via prompts_on_screen()). */
#define CHECK_PROMPTS(n, ...) do {                                        \
        if (tal) tal->assertions++;                                       \
        if (!tal || tal->prompts < (n)) {                                 \
            if (tal) tal->failures++;                                     \
            fprintf(stderr, "FAIL: [VACUOUS]  ");                          \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " - only %d prompt(s) offered, need %d\n",   \
                    tal ? tal->prompts : -1, (int)(n));                   \
        }                                                                 \
    } while (0)

/* ── card DB ────────────────────────────────────────────────────────────── */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

static int HC[7];

static void hcolor_init(void)
{
    for (int i = 0; i < 7; i++)
        HC[i] = (int)rb_parse_heart_color(rb_heart_color_as_str(i));
}

/* A ROTATING ring: a no_of() helper with one static buf called 2-3 times in
 * one printf prints the LAST card repeatedly. */
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

/* AGENTS.md identity rule: byte-exact strcmp of the DECODED card_no.
 * rb_card_no_eq is FUZZY and rb_card_string takes a string-table index. */
static int pin_id(int cid, const char *no)
{
    if (tal) tal->assertions++;
    const char *got = card_no_of(cid);
    if (cid >= 0 && got[0] && !strcmp(got, no)) return 1;
    if (tal) { tal->failures++; tal->setup_bugs++; }
    fprintf(stderr, "SETUP BUG: identity - want '%s', resolved '%s' (id=%d)\n",
            no, got, cid);
    return 0;
}

/* Rust `game.id(x)` mints a NEW instance per call -> test_new_id. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        if (tal) { tal->assertions++; tal->failures++; tal->setup_bugs++; }
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

static void fresh(TestGame *tg)
{
    rb_card_copy_pool_reset();
    test_game_new(tg);
    rb_look_reset_all();
}

static int printed_cost(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.cost;
    rb_free_card(&c);
    return v;
}

static int printed_blade(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.blade;
    rb_free_card(&c);
    return v;
}

/* A LIVE card's requirement is its `score`, not its blade. */
static int printed_score(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.score;
    rb_free_card(&c);
    return v;
}

/* The PRINTED card name, for the `distinct: card_name` axis.  There is no
 * public accessor, so the Card is decoded and c.name read; the string is owned
 * by the Card, so it is COPIED into a rotating ring before the free. */
#define NAME_RING 4
static const char *rb_card_name_of(int cid)
{
    static char ring[NAME_RING][96];
    static int next;
    char *buf = ring[next++ % NAME_RING];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        if (c.name) snprintf(buf, 96, "%s", c.name);
        else snprintf(buf, 96, "%s", rb_card_string(c.name_idx));
        rb_free_card(&c);
    }
    return buf;
}

/* Rust `assert!(card prints this trigger)`: also the fixture-wiring vacuity
 * control for every negative in this file. */
static int prints_trigger(int cid, const char *trig)
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

static int trigger_index(int cid, const char *trig)
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

static int expect_trigger(int cid, const char *trig, const char *no)
{
    int ok = prints_trigger(cid, trig);
    CHECK(ok, "fixture wiring: %s really prints a '%s' ability", no, trig);
    return ok;
}

/* ── decoded-shape walker (diagnostic + the §B8 parse test) ────────────────
 * Walks the whole AbilityEffect / Condition tree looking for a named key, both
 * as an AbilityEffect `extra` entry and as a Condition field key, and prints
 * every string it meets.  This is how a "the tag did not survive the decode"
 * claim is settled from the DECODER rather than from abilities.json. */
static void count_shape_keys(const AbilityEffect *e, const char *want,
                             int *hits, int *wait_hits);

static void scan_cond_keys(const Condition *c, const char *want,
                           int *hits, int *wait_hits, int depth)
{
    if (!c || depth > 8) return;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (!f->key) continue;
        fprintf(stderr, "%*s   cond key \"%s\" tag=%u", depth * 2, "", f->key,
                f->v.tag);
        if (f->v.tag == RB_TAG_STR) fprintf(stderr, " = \"%s\"", f->v.s ? f->v.s : "");
        else if (f->v.tag == RB_TAG_I64) fprintf(stderr, " = %lld", (long long)f->v.i);
        fputc('\n', stderr);
        if (!strcmp(f->key, want)) (*hits)++;
        if (f->v.tag == RB_TAG_STR && f->v.s && strstr(f->v.s, "wait"))
            (*wait_hits)++;
        if (f->v.tag == RB_TAG_ARRAY)
            for (uint32_t j = 0; j < f->v.arr_n; j++) {
                const CondValue *cv = &f->v.arr[j];
                if (cv->s && strstr(cv->s, "wait")) (*wait_hits)++;
                if (cv->cond) scan_cond_keys(cv->cond, want, hits, wait_hits, depth + 1);
            }
        if (f->v.cond) scan_cond_keys(f->v.cond, want, hits, wait_hits, depth + 1);
    }
}

static void count_shape_keys(const AbilityEffect *e, const char *want,
                             int *hits, int *wait_hits)
{
    if (!e) return;
    for (int i = 0; i < e->n_extra; i++) {
        if (!e->extra_k[i]) continue;
        fprintf(stderr, "    effect extra[%d] \"%s\" = %s\n", i, e->extra_k[i],
                e->extra_v[i] ? e->extra_v[i] : "(null)");
        if (!strcmp(e->extra_k[i], want)) (*hits)++;
        if (e->extra_v[i] && strstr(e->extra_v[i], "wait")) (*wait_hits)++;
    }
    for (int i = 0; i < e->n_child; i++)
        count_shape_keys(e->child[i], want, hits, wait_hits);
    for (int i = 0; i < e->n_options; i++)
        count_shape_keys(e->options[i], want, hits, wait_hits);
    if (e->has_condition && e->condition)
        scan_cond_keys(e->condition, want, hits, wait_hits, 1);
    count_shape_keys(e->primary_effect, want, hits, wait_hits);
    count_shape_keys(e->followup_action, want, hits, wait_hits);
    count_shape_keys(e->optional_action, want, hits, wait_hits);
    count_shape_keys(e->conditional_action, want, hits, wait_hits);
}

/* ── startup diagnostics ────────────────────────────────────────────────── */

static const char *const ALL_FIXTURES[] = {
    HANA_001, CB_ALLY, RURINO_003, HANAMO_009, HASU_ALLY, HASU_ALLY_2,
    RURINO_W, RURINO_RE, REN_MILL, LIELLA_A, LIELLA_B, WIEN_YELL, SUMIRE_YELL,
    LIVE_SCORED, MEMBER_SCORE, HIKARI_040, PRINT_1, PRINT_2, PRINT_3,
    KOTORI_P, NON_PRINT, ARRIVER, FILLER, EN_CARD
};

static void print_wire_map(void)
{
    fprintf(stderr, "--- heart index map (rb_heart_color_as_str) ---\n");
    for (int i = 0; i < 7; i++)
        fprintf(stderr, "    index %d -> \"%s\" -> enum %d\n",
                i, rb_heart_color_as_str(i), HC[i]);
    fprintf(stderr, "--- group / unit names (group is SERIES-LEVEL) ---\n");
    for (unsigned i = 0; i < sizeof ALL_FIXTURES / sizeof ALL_FIXTURES[0]; i++) {
        int id = rb_find_card_by_no(ALL_FIXTURES[i]);
        if (id < 0) { fprintf(stderr, "    %-22s NOT FOUND\n", ALL_FIXTURES[i]); continue; }
        fprintf(stderr, "    %-22s id=%-6d cost=%-3d blade=%-3d group=\"%s\" unit=\"%s\"\n",
                ALL_FIXTURES[i], id, printed_cost(id), printed_blade(id),
                rb_card_group_name(id), rb_card_unit_name(id));
    }
    fprintf(stderr, "--- assertion literals used above ---\n");
    fprintf(stderr, "    GRP_HASUNO  = \"%s\"\n", GRP_HASUNO);
    fprintf(stderr, "    GRP_LIELLA  = \"%s\"\n", GRP_LIELLA);
    fprintf(stderr, "    UNIT_SUZUKE = \"%s\"\n", UNIT_SUZUKE);
}

/* ── board helpers ──────────────────────────────────────────────────────── */

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }
static const RbChoice *pchoice(TestGame *tg) { return pending(tg) ? rb_get_pending_choice(&tg->state) : NULL; }
static int prompts_on_screen(TestGame *tg) { return pending(tg) ? 1 : 0; }

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

/* deck_has is used by A12's premise check (all three named cards really are
 * in the deck before the 起動 mills them). */

/* Rust `select_indices(&[])`: DECLINE. */
static void decline(TestGame *tg)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice(&tg->state, -1);
}

/* Rust `while has_pending_choice() { select_indices(&[0]) }`: the
 * unconditional index 0, which in C means ACCEPT / PROCEED. */
static int drain_pick_first(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) pick(tg, 0);
    return guard;
}

/* The Rust `hs_pb1_001` / `hs_pb1_009` drain: a SelectTarget takes option 0 or
 * 1 (pay_optional_cost:skip -> 0 is SKIP, 1 is PAY), a SelectCard with a
 * positive count takes index 0, everything else is DECLINED.  Bounded: a peer
 * found an ability that re-prompts unboundedly. */
/* SelectTarget prompts answered by drain_optional since the last reset.  A
 * pay_optional_cost:skip gate IS a SelectTarget, so this is the specific
 * vacuity control for every "the optional payment was offered" claim. */
static int g_n_select_target;
static int g_n_select_card;

static int drain_optional(TestGame *tg, int pay, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const RbChoice *c = pchoice(tg);
        if (!c) break;
        if (c->kind == RB_CHOICE_SELECT_TARGET) {
            g_n_select_target++;
            pick(tg, pay ? 1 : 0);
        } else if (c->kind == RB_CHOICE_SELECT_CARD) {
            g_n_select_card++;
            if (c->count > 0) pick(tg, 0);
            else               decline(tg);
        } else {
            decline(tg);
        }
    }
    rb_drain_ability_queue(&tg->state);
    return guard;
}

static int hand_n(const TestGame *tg, int pl)  { return tg->state.p[pl].hand.n; }
static int deck_n(const TestGame *tg, int pl)  { return tg->state.p[pl].deck.n; }
static int wait_n(const TestGame *tg, int pl)  { return tg->state.p[pl].discard.n; }
static int zone_n(const TestGame *tg, int pl)  { return tg->state.p[pl].energy.n; }
static int active_e(const TestGame *tg, int pl) { return rb_energy_active_count(&tg->state.p[pl]); }

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int hand_has(const TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].hand, cid); }
static int wait_has(const TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].discard, cid); }
static int deck_has(const TestGame *tg, int pl, int cid) { return bag_has(&tg->state.p[pl].deck, cid); }

static int stage_has(const TestGame *tg, int pl, int cid)
{
    for (int a = 0; a < RB_STAGE_SIZE; a++)
        if (tg->state.p[pl].stage[a] == cid) return 1;
    return 0;
}

static void set_stage(TestGame *tg, int pl, int area, int cid)
{
    if (area < 0 || area >= RB_STAGE_SIZE) return;
    tg->state.p[pl].stage[area] = cid;
    tg->state.p[pl].stage_wait[area] = 0;
}

static void clear_stage(TestGame *tg, int pl)
{
    for (int a = 0; a < RB_STAGE_SIZE; a++) {
        tg->state.p[pl].stage[a] = RB_EMPTY_SLOT;
        tg->state.p[pl].stage_wait[a] = 0;
    }
}

static void hand_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++] = cid;
}

static void deck_append_n(TestGame *tg, int pl, int cid, int n)
{
    RbPlayer *P = &tg->state.p[pl];
    for (int i = 0; i < n; i++)
        if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = cid;
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        tg->state.p[pl].deck.n = 0;
        deck_append_n(tg, pl, filler, n);
    }
}

static int blade_of(const TestGame *tg, int cid)
{
    return rb_mods_get_blade((RbMods *)&tg->state.mods, cid);
}
static int heart_of(const TestGame *tg, int cid, int idx)
{
    return rb_mods_get_heart((RbMods *)&tg->state.mods, cid, idx);
}
static int need_heart_of(const TestGame *tg, int cid, int idx)
{
    return rb_mods_get_need_heart((RbMods *)&tg->state.mods, cid, idx);
}
static const char *orient_of(const TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation((RbMods *)&tg->state.mods, cid);
    return o ? o : "";
}
static int is_wait(const TestGame *tg, int cid)
{
    return strcmp(orient_of(tg, cid), "wait") == 0;
}

/* The faithful twin of Rust's `assert_card_in_group(card, name)`.
 *
 * src/ability/util.c:295-300 resolves a printed group name against the card's
 * UNIT first and only then against the five series-derived names, and
 * src/ability/effects/score.c:147-152 mirrors it.  rb_card_group_name is
 * SERIES-LEVEL only, so a "Printemps" assertion made against it is FALSE for
 * every ラブライブ! card (whose group is "μ's") even though the card really is
 * a Printemps member -- which is exactly the false red a first run produced
 * before this helper existed. */
static int in_group(int cid, const char *g)
{
    const char *u = rb_card_unit_name(cid);
    if (u && *u && !strcmp(u, g)) return 1;
    const char *s = rb_card_group_name(cid);
    return s && *s && !strcmp(s, g);
}

/* Rust `energy_zone.cards.push(..)` x total + `set_active_count(active)`. */
static void set_energy(TestGame *tg, int pl, int total, int active)
{
    RbPlayer *P = &tg->state.p[pl];
    P->energy.n = 0;
    for (int i = 0; i < total && P->energy.n < RB_MAX_ZONE; i++)
        P->energy.cards[P->energy.n++] = rb_find_card_by_no(EN_CARD);
    rb_energy_set_active_count(P, active);
}

/* helpers/mod.rs fire_trigger: resolve ONE ability on ONE card, key it
 * "{card_no}_{full_text}", fire it with `cid` as the activating card, then
 * process the queue.
 *
 * The hand-rolled body it replaces recorded NO enqueue-time use, so a
 * 自動/ターン1回 ability was still unspent when it resolved and the
 * post-resolution re-scan in rb_process_player_abilities_depth_full enqueued
 * it a SECOND time; its "{card_no}_{full_text>" key also truncated at 512
 * bytes. Both are fixed by the shared helper.
 *
 * PRESERVED: the return value. Several call sites store it
 * (`r.live_start_queued`, `r->live_queued`, `queued`) and assert on the COUNT
 * of entries produced, so test_fire_trigger_on_card's "1 when exactly one
 * entry was produced" contract is what they now read. */
static int fire_trigger(TestGame *tg, int cid, const char *trig, const char *trigger_type)
{
    (void)trigger_type;   /* every call site here passes trig == trigger_type */
    return test_fire_trigger_on_card(tg, cid, trig);
}

/* Rust `TurnEngine::trigger_auto_abilities_for_player` +
 * `state.process_pending_auto_abilities`.
 *
 * `just_completed_ability_key` is cleared FIRST, exactly as
 * rb_trigger_auto_abilities does before returning
 * (game_state_abilities.c:1795).  queue_zone_abilities skips any ability whose
 * key equals that field (:1390), so a SECOND scan of the same card on the same
 * board is silently dropped without the clear -- which would make every
 * "fires again" case in this file vacuously red for the wrong reason.
 * rb_queue_trigger_abilities re-derives the moved-card snapshot from
 * g->recently_moved, which is the faithful spelling. */
static int scan_moved(TestGame *tg, int pl)
{
    tg->state.just_completed_ability_key = -1;
    int q = rb_queue_trigger_abilities(&tg->state, pl, RB_TSTR_AUTO);
    rb_process_pending_auto_abilities(&tg->state);
    drain_pick_first(tg, 12);
    rb_drain_ability_queue(&tg->state);
    return q;
}

/* Rust `state.set_recently_moved_cards(vec![..])`.  `recently_moved_from_zone`
 * has no setter in C and is written directly where Rust sets it. */
static void set_recently_moved(TestGame *tg, int pl, const int *ids, int n)
{
    GameState *s = &tg->state;
    s->n_recently_moved = 0;
    for (int i = 0; i < n; i++)
        if (s->n_recently_moved < RB_MAX_RECENTLY_MOVED)
            s->recently_moved[s->n_recently_moved++] = ids[i];
    (void)pl;
}

/* `g->yell_occurred` is a TRANSIENT: the engine logs `occurred=1` at the moment
 * of the yell and the flag reads 0 again once the performance has resolved, so
 * reading it at the assertion site is a false negative.  These record the flag
 * as it stood at the end of every pass inside the window, which is where the
 * Rust harness's `game.drain_choices()` boundary sits. */
static int g_yell_ever, g_revealed_peak;

/* The yell window: rb_performance (src/turn/live.c:448) does the yell, queues
 * the yell-triggered 自動 and leaves the prompt standing, all inside ONE
 * test_pass.  test_advance_to_phase(RB_PHASE_PERFORMANCE_SECOND) runs the whole
 * live, so the yell state is gone by the assertion site.  Step one pass at a
 * time and stop the moment a prompt is standing or the revealed set is
 * non-empty. */
static int step_to_yell_window(TestGame *tg)
{
    g_yell_ever = 0;
    g_revealed_peak = 0;
    for (int i = 0; i < 40; i++) {
        if (tg->state.yell_occurred) g_yell_ever = 1;
        if (tg->state.n_revealed > g_revealed_peak)
            g_revealed_peak = tg->state.n_revealed;
        if (rb_has_pending_choice(&tg->state)) return 1;
        if (tg->state.n_revealed > 0) return 1;
        if (tg->state.performance_resume_pending) return 1;
        test_pass(tg);
    }
    if (tg->state.yell_occurred) g_yell_ever = 1;
    if (tg->state.n_revealed > g_revealed_peak)
        g_revealed_peak = tg->state.n_revealed;
    return rb_has_pending_choice(&tg->state) || tg->state.n_revealed > 0 ||
           tg->state.performance_resume_pending;
}

/* ── runner ─────────────────────────────────────────────────────────────── */

enum { CHILD_OK = 0, CHILD_FAILURES = 1, CHILD_SETUPBUG = 2 };

static int n_ok, n_failed, n_crashed, n_setup;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        int f0 = tal->failures, s0 = tal->setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = tal->failures - f0, ns = tal->setup_bugs - s0;
        if (nf) n_failed++; else if (ns) n_setup++; else n_ok++;
        printf("%-8s %s  [%d failures, %d setup bugs]\n",
               nf ? "FAILED" : (ns ? "SETUPBUG" : "ok"), name, nf, ns);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int f0 = tal->failures, s0 = tal->setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        printf("        %s: %d failure(s), %d setup bug(s)\n",
               name, tal->failures - f0, tal->setup_bugs - s0);
        fflush(stdout);
        fflush(stderr);
        _Exit(tal->failures > f0 ? CHILD_FAILURES
                                 : (tal->setup_bugs > s0 ? CHILD_SETUPBUG : CHILD_OK));
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- fatal signal %d (engine fault)\n", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("CRASH    %s  <-- abnormal exit\n", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("ok       %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("FAILED   %s\n", name);
    } else if (WEXITSTATUS(status) == CHILD_SETUPBUG) {
        n_setup++;
        printf("SETUPBUG %s  <-- fixture could not be built\n", name);
    } else {
        n_crashed++;
        printf("CRASH    %s  <-- unexpected child exit %d\n", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

/* ═════════════════════════════════════════════════════════════════════════
 * A. jidou/combination -- 6 Rust files, 16 #[test].
 *
 * A jidou (自動) whose effect FEEDS the next trigger: the activation makes the
 * sibling's cost payment affordable, or charges the sibling up to a threshold.
 * No single-ability test can see that link, which is why the whole family is
 * worth porting and why every case here asserts a ledger (energy / blade /
 * cards) rather than a single flag.
 * ═════════════════════════════════════════════════════════════════════════ */

/* ── A1. hs_pb1_001_jidou_activated_energy_funds_live_start_payment (2) ────
 * PL!HS-pb1-001-R ab#0 自動/ターン2回: pay 1E -> activate 2 waited energy.
 * PL!HS-pb1-001-R ab#1 ライブ開始時: pay 2E -> blade + heart04 until live end.
 *
 * The tuned fixture: 5 active + 2 waited.  The 4-cost ally leaves 1 active;
 * ab#0 paying 1E drops that to 0 and activates the 2 waited (0 -> 2); ab#1
 * then spends exactly those 2.  Without ab#0 there is only 1 active, so ab#1's
 * 2E is unpayable.  THAT is the link, and it is a negative-only observable. */
typedef struct {
    int hana, ally, filler;
    int played;
    int prompts_jidou;          /* prompts offered while resolving ab#0      */
    int targets_jidou;          /* SelectTarget prompts during that drain    */
    int targets_live_start;
    int active_after_jidou;
    int prompts_live_start;
    int live_start_queued;
    int active_after_live_start;
    int blade_after, heart04_after;
    int waitroom0, waitroom1, waitroom2;
    int census0, census1, census2;
    int ally_on_stage;
} A1Run;

static A1Run a1_run(TestGame *tg, int accept)
{
    A1Run r;
    memset(&r, 0, sizeof r);
    r.hana = r.ally = r.filler = -1;

    fresh(tg);
    int hana = mid(tg, HANA_001);
    int ally = mid(tg, CB_ALLY);
    int filler = mid(tg, FILLER);
    r.hana = hana; r.ally = ally; r.filler = filler;
    if (!pin_id(hana, HANA_001)) return r;
    if (!pin_id(ally, CB_ALLY)) return r;
    CHECK_EQ(printed_cost(hana), 11, "A1: 花帆 prints cost 11");
    CHECK_EQ(printed_cost(ally), 4, "A1: the ally prints cost 4 (the tuned ledger)");
    /* 「ほかの『スリーズブーケ』のメンバーが登場する」 -- the trigger target.
     * スリーズブーケ is a UNIT name, so this goes through rb_card_unit_name;
     * rb_card_group_name would return the Hasunosora SERIES instead and the
     * check would pass for the wrong reason. */
    CHECK(strcmp(rb_card_unit_name(ally), UNIT_SUZUKE) == 0,
          "A1: the appearing ally IS a 「スリーズブeke」 member (unit name, got \"%s\")",
          rb_card_unit_name(ally));
    CHECK(strcmp(rb_card_group_name(ally), GRP_HASUNO) == 0,
          "A1: the ally's series-derived group is 「蓮ノ空」 (got \"%s\")",
          rb_card_group_name(ally));
    if (!expect_trigger(hana, RB_TSTR_AUTO, HANA_001)) return r;
    if (!expect_trigger(hana, RB_TSTR_LIVE_START, HANA_001)) return r;

    /* Rust fixture(): hand and deck cleared, 40 filler in p1's deck, 花帆 in
     * センター, the ally in hand, and 7 energy with 5 active. */
    tg->state.p[0].hand.n = 0;
    tg->state.p[0].deck.n = 0;
    deck_append_n(tg, 0, filler, 40);
    set_stage(tg, 0, AREA_CENTER, hana);
    hand_append(tg, 0, ally);
    set_energy(tg, 0, 7, 5);
    CHECK_EQ(zone_n(tg, 0), 7, "A1: precondition -- the energy zone holds 7 cards");
    CHECK_EQ(active_e(tg, 0), 5, "A1: precondition -- 5 of them are ACTIVE");
    CHECK_EQ(zone_n(tg, 0) - active_e(tg, 0), 2, "A1: precondition -- 2 are WAITED");

    r.census0 = test_total_card_count(tg);
    r.waitroom0 = wait_n(tg, 0);

    int p0 = tal->prompts;
    int t0 = g_n_select_target;
    r.played = test_play_to_stage(tg, ally, AREA_LEFT);
    /* Rust play_ally_and_resolve_jidou: one drain, SelectTarget answered with
     * accept?1:0 and everything else DECLINED. */
    drain_optional(tg, accept, 12);
    r.prompts_jidou = tal->prompts - p0;
    r.targets_jidou = g_n_select_target - t0;
    r.active_after_jidou = active_e(tg, 0);
    r.ally_on_stage = stage_has(tg, 0, ally);
    r.census1 = test_total_card_count(tg);
    r.waitroom1 = wait_n(tg, 0);

    int p1 = tal->prompts;
    int t1 = g_n_select_target;
    r.live_start_queued = fire_trigger(tg, hana, RB_TSTR_LIVE_START, RB_TSTR_LIVE_START);
    /* Rust fire_live_start_paying: the 2E payment is option 1. */
    drain_optional(tg, 1, 12);
    r.prompts_live_start = tal->prompts - p1;
    r.targets_live_start = g_n_select_target - t1;
    r.active_after_live_start = active_e(tg, 0);
    r.blade_after = blade_of(tg, hana);
    r.heart04_after = heart_of(tg, hana, HC[4]);
    r.census2 = test_total_card_count(tg);
    r.waitroom2 = wait_n(tg, 0);

    fprintf(stderr,
            "        [info] A1 accept=%d played=%d ally_on_stage=%d "
            "prompts jidou=%d/%d-target live_start=%d/%d-target queued=%d | "
            "active 5->%d->%d | blade=%d heart04=%d | CENSUS %d/%d/%d | "
            "waitroom %d/%d/%d\n",
            accept, r.played, r.ally_on_stage, r.prompts_jidou, r.targets_jidou,
            r.prompts_live_start, r.targets_live_start, r.live_start_queued,
            r.active_after_jidou, r.active_after_live_start, r.blade_after,
            r.heart04_after, r.census0, r.census1, r.census2,
            r.waitroom0, r.waitroom1, r.waitroom2);
    return r;
}

static void a1_jidou_energy_is_exactly_what_live_start_pays(void)
{
    static TestGame tg;
    A1Run r = a1_run(&tg, 1 /* accept ab#0's E */);
    if (r.hana < 0) return;

    CHECK(r.played, "A1+: the 4-cost ally is really played");
    CHECK(r.ally_on_stage, "A1+: the ally is on stage, so the appearance happened");
    /* CENSUS: a zone count of 0 is identical whether the ally moved or was
     * DESTROYED.  Playing one card hand->stage must not change the total. */
    CHECK_EQ(r.census1, r.census0,
             "A1+: CENSUS unchanged by the play (nothing was destroyed)");
    CHECK_EQ(r.active_after_jidou, 2,
             "A1+: ab#0 paid 1E out of the 1 the 4-cost ally left (1->0) and "
             "activated the 2 waited (0->2).  The zone must hold 2 active for ab#1.");

    CHECK(r.targets_live_start > 0,
          "A1+: VACUITY CONTROL -- ab#1's 2E payment really was OFFERED as a "
          "SelectTarget (%d)", r.targets_live_start);
    /* J12: the [info] line above used to print a HARDCODED `queued=1` instead of
     * this counter, which MASKED a real engine bug -- an `ability_id[512]`
     * truncation meant ab#1 never fired at all, and the run read `queued=1`
     * anyway. The field is measured (a1_run:912, `= fire_trigger(...)`), so it
     * is asserted here: a zero now fails loudly instead of hiding in a log. */
    CHECK_EQ(r.live_start_queued, 1,
             "A1+: ab#1 really ENQUEUED exactly one printed ability "
             "(measured fire_trigger count, was previously a hardcoded 1 in the "
             "diagnostic)");
    CHECK_EQ(r.active_after_live_start, 0,
             "A1+: ab#1 spent exactly the 2 energy ab#0 activated -- the zone empties. "
             "This is the link: the sibling's payment is funded by the jidou.");
    CHECK_EQ(r.blade_after, 1,
             "A1+: ab#1 granted its ブレード to 花帆 (EXACT, not > 0: the grant is a "
             "fixed amount)");
    CHECK_EQ(r.heart04_after, 1, "A1+: ab#1 also granted heart04, as printed");
    CHECK_EQ(r.census2, r.census0,
             "A1+: CENSUS still unchanged after ab#1 (no card was consumed)");
}

static void a2_declining_jidou_leaves_live_start_unable_to_pay(void)
{
    static TestGame tg;
    A1Run r = a1_run(&tg, 0 /* DECLINE ab#0's E */);
    if (r.hana < 0) return;

    CHECK(r.played, "A2: the 4-cost ally is really played");
    CHECK(r.ally_on_stage, "A2: the ally is on stage, so the appearance happened");
    CHECK_EQ(r.census1, r.census0, "A2: CENSUS unchanged by the play");
    /* VACUITY CONTROL: the specific gate really appeared.  Without it, "no
     * energy was activated" would be satisfied by an engine that never offered
     * the E at all -- and the positive a1_ is exactly that board with the
     * offer accepted. */
    CHECK(r.targets_jidou > 0,
          "A2: ab#0's optional pay-E really was OFFERED as a SelectTarget (%d) "
          "-- otherwise 'no energy activated' proves nothing", r.targets_jidou);
    CHECK_EQ(r.active_after_jidou, 1,
             "A2: ab#0 DECLINED: the 4-cost ally left 1 active and no waited "
             "energy was activated.  1 < ab#1's 2E cost.");

    CHECK(r.targets_live_start > 0,
          "A2: ab#1's 2E payment was still offered on a board with only 1 active "
          "energy (%d SelectTarget)", r.targets_live_start);
    CHECK_EQ(r.active_after_live_start, 1,
             "A2: ab#1's 2E is UNPAYABLE with 1 active energy, so nothing is deducted");
    CHECK_EQ(r.blade_after, 0,
             "A2: ab#1 could not pay, so it granted nothing -- the proof that the "
             "sibling's grant is FUNDED by the jidou, not independent of it");
    CHECK_EQ(r.census2, r.census0, "A2: CENSUS unchanged end to end");
}

/* ── A3. hs_pb1_003_turn2_second_discard_event_grants_again (3) ───────────
 * PL!HS-pb1-003-R ab#1 自動/ターン2回, condition source "preceding_moved",
 * location "discard": each qualifying hand->waitroom event grants heart01 +
 * ブレード until live end.
 *
 * Three link observables: (1) two SEPARATE events in one turn grant twice --
 * the ターン2回 allowance and the re-scan-guard regression; (2) a batch of 3
 * is ONE event; (3) a third event is refused. */
typedef struct {
    int rurino;
    int blade[3], heart01[3];   /* after event 1, 2, 3                       */
    int queued[3];
    int use_after[3];
    int census0, census[3];
    int minted;                  /* card ids minted but never placed in a zone */
} A2Run;

static A2Run a3_run(TestGame *tg, int n_events, const int *batch_sizes)
{
    A2Run r;
    memset(&r, 0, sizeof r);
    r.rurino = -1;

    fresh(tg);
    int rurino = mid(tg, RURINO_003);
    r.rurino = rurino;
    if (!pin_id(rurino, RURINO_003)) return r;
    CHECK_EQ(printed_cost(rurino), 15, "A3: 瑠璃乃 prints cost 15");
    if (!expect_trigger(rurino, RB_TSTR_AUTO, RURINO_003)) return r;
    /* The ab#0 that also prints is her 登場 discard/draw; the one under test is
     * the LAST 自動... actually ab#0 is 登場, ab#1 is the only 自動. */
    CHECK_EQ(trigger_index(rurino, RB_TSTR_AUTO), 1,
             "A3: the 自動 under test is ability index 1 (ab#0 is her 登場)");

    /* Rust setup(): stage[1] = rurino, nothing else seeded. */
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    set_stage(tg, 0, AREA_CENTER, rurino);
    CHECK_EQ(blade_of(tg, rurino), 0, "A3: precondition -- no blade modifier yet");
    CHECK_EQ(heart_of(tg, rurino, HC[1]), 0, "A3: precondition -- no heart01 yet");

    r.census0 = test_total_card_count(tg);
    for (int e = 0; e < n_events; e++) {
        int n = batch_sizes[e];
        int ids[8];
        for (int i = 0; i < n; i++) ids[i] = mid(tg, FILLER);
        r.minted += n;
        /* Rust discard_event(): the fodder ids are handed to
         * set_recently_moved_cards and are NOT placed in any zone, so the
         * census must not move -- that is the control against the engine
         * silently DESTROYING a card. */
        set_recently_moved(tg, 0, ids, n);
        r.queued[e] = scan_moved(tg, 0);
        r.blade[e] = blade_of(tg, rurino);
        r.heart01[e] = heart_of(tg, rurino, HC[1]);
        r.census[e] = test_total_card_count(tg);
        r.use_after[e] = rb_use_count(&tg->state.queue, rurino, 1, tg->state.turn);
    }
    fprintf(stderr,
            "        [info] A3 events=%d queued=%d/%d/%d use=%d/%d/%d "
            "blade=%d/%d/%d heart01=%d/%d/%d CENSUS %d,%d,%d\n",
            n_events, r.queued[0], r.queued[1], r.queued[2],
            r.use_after[0], r.use_after[1], r.use_after[2],
            r.blade[0], r.blade[1], r.blade[2],
            r.heart01[0], r.heart01[1], r.heart01[2],
            r.census[0], r.census[1], r.census[2]);
    return r;
}

static void a3_two_separate_discard_events_in_one_turn_grant_twice(void)
{
    static TestGame tg;
    static const int sizes[2] = { 1, 1 };
    A2Run r = a3_run(&tg, 2, sizes);
    if (r.rurino < 0) return;

    CHECK_EQ(r.census[0], r.census0,
             "A3+: CENSUS unchanged by event 1 (the recently_moved snapshot is "
             "not a zone, so nothing can vanish)");
    CHECK_EQ(r.queued[0] > 0, 1,
             "A3+: event 1 really ENQUEUED her 自動 (queued=%d)", r.queued[0]);
    CHECK_EQ(r.use_after[0], 1,
             "A3+: one use recorded -- rb_record_use runs at ENQUEUE time, so "
             "this is a direct non-vacuity proof the jidou fired");
    CHECK_EQ(r.blade[0], 1,
             "A3+: event 1 -> ブレード+1 (ターン2回, 1 of 2 used)");

    CHECK_EQ(r.queued[1] > 0, 1,
             "A3+: event 2 really ENQUEUED her 自動 again (queued=%d) -- the "
             "re-scan guard must not swallow the second event",
             r.queued[1]);
    CHECK_EQ(r.use_after[1], 2, "A3+: the second use is recorded");
    CHECK_EQ(r.blade[1], 2,
             "A3+: the SECOND qualifying event in the same turn MUST fire "
             "again.  ターン2回 is the allowance; a leaked re-scan key used to "
             "veto it and this read 1.");
    CHECK_EQ(r.heart01[1], 2,
             "A3+: both firings grant heart01 -- the jidou is a paired "
             "heart01 + ブレード grant");
    CHECK_EQ(r.census[1], r.census0, "A3+: CENSUS unchanged end to end");
}

static void a4_batch_of_three_is_one_event_and_keeps_the_second_allowance(void)
{
    static TestGame tg;
    static const int sizes[2] = { 3, 1 };
    A2Run r = a3_run(&tg, 2, sizes);
    if (r.rurino < 0) return;

    CHECK_EQ(r.use_after[0], 1,
             "A4: a batch of 3 consumes EXACTLY ONE allowance (use=%d) -- the "
             "allowance is per EVENT, not per card", r.use_after[0]);
    CHECK_EQ(r.blade[0], 1,
             "A4: a batch of 3 fires ab#1 ONCE.  It must not consume one "
             "allowance per card.");
    CHECK_EQ(r.census[0], r.census0,
             "A4: CENSUS unchanged -- the 3 fodder ids were handed to the "
             "recently_moved snapshot, never to a zone, so the engine cannot "
             "have DESTROYED them");

    /* PAIRED POSITIVE on the same board: the second event still fires. */
    CHECK_EQ(r.use_after[1], 2, "A4: the second event is still allowed after a 3-card batch");
    CHECK_EQ(r.blade[1], 2,
             "A4: the second event grants, proving the batch consumed exactly "
             "one allowance.  A batch that consumed 3 would read 1 here -- the "
             "same visible number as 'the jidou never fired', which is why both "
             "halves are asserted.");
    CHECK_EQ(r.heart01[1], 2, "A4: heart01 tracks the same per-event accounting");
}

static void a5_turn2_refuses_a_third_event_in_one_turn(void)
{
    static TestGame tg;
    static const int sizes[3] = { 1, 1, 1 };
    A2Run r = a3_run(&tg, 3, sizes);
    if (r.rurino < 0) return;

    /* PAIRED POSITIVE first, on the same board. */
    CHECK_EQ(r.use_after[0], 1, "A5: precondition -- first use recorded");
    CHECK_EQ(r.use_after[1], 2, "A5: precondition -- both allowances used");
    CHECK_EQ(r.blade[1], 2, "A5: precondition -- two grants so far");

    CHECK_EQ(r.use_after[2], 2,
             "A5: ターン2回 is a CEILING -- the third event must be refused at "
             "ENQUEUE (use stayed %d)", r.use_after[2]);
    CHECK_EQ(r.blade[2], 2,
             "A5: a third qualifying event in the same turn must not grant.  An "
             "engine that ignored the limit would read 3 here while still "
             "passing the two-firing assertions.");
    CHECK_EQ(r.heart01[2], 2,
             "A5: the refusal covers the WHOLE paired grant (heart01 + ブレード), "
             "not one half");
    CHECK_EQ(r.census[2], r.census0, "A5: CENSUS unchanged across all three events");
}

/* ── A6. hs_pb1_009_jidou_blades_charge_live_start_threshold (5) ───────────
 * PL!HS-pb1-009-R ab#0 自動/センター/ターン2回: on a 「蓮ノ空」 member
 * appearing, ブレード+2 until live end.  She PRINTS ブレード 4.
 * PL!HS-pb1-009-R ab#1 ライブ開始時: if this member's blade >= 8, draw 2 and
 * discard 1 to the waitroom.
 * 4 printed + 2 + 2 = 8 EXACTLY: the jidou is the only route to the
 * threshold, so this is the full L1 feeder pair.
 *
 * NOTE ON THE FIXTURE: the Rust file's `HASU_ALLY` is PL!HS-sd1-001-SD, which
 * cards.json names 日野下花帆 -- the SAME CHARACTER as the host, a different
 * print (cost 9 vs 15).  The Rust `assert_card_in_group(., "蓮ノ空")` therefore
 * only holds through the SERIES alias, and so must the C check; the unit name
 * is スリーズブeke, asserted separately so the two are not confused. */
typedef struct {
    int hanamo, filler;
    int allies[4];
    int n_allies;
    int played[3];
    int total_blade_after[3];    /* printed + granted, after appearance n      */
    int granted_after[3];
    int use_after[3];
    int hand_before, deck_before;
    int hand_after, deck_after;
    int live_queued, targets_live, cards_live;
    int census0, census1, census2;
} A3Run;

/* Rust `stage_hanamo_with_allies`. */
static A3Run a6_fixture(TestGame *tg, int n_allies)
{
    A3Run r;
    memset(&r, 0, sizeof r);
    r.hanamo = r.filler = -1;
    r.hand_after = r.deck_after = r.hand_before = r.deck_before = -1;
    r.total_blade_after[0] = r.total_blade_after[1] = r.total_blade_after[2] = -1;
    r.granted_after[0] = r.granted_after[1] = r.granted_after[2] = -1;

    fresh(tg);
    int hana = mid(tg, HANAMO_009);
    r.hanamo = hana;
    if (!pin_id(hana, HANAMO_009)) return r;
    CHECK_EQ(printed_cost(hana), 15, "A6: 花帆(pb1-009) prints cost 15");
    CHECK_EQ(printed_blade(hana), 4,
             "A6: she PRINTS ブレード 4 -- the baseline the jidou charges");
    CHECK_EQ(blade_of(tg, hana), 0, "A6: no granted blades before any appearance");
    if (!expect_trigger(hana, RB_TSTR_AUTO, HANAMO_009)) return r;
    if (!expect_trigger(hana, RB_TSTR_LIVE_START, HANAMO_009)) return r;

    int filler = mid(tg, FILLER);
    r.filler = filler;
    fill_decks(tg, filler, 40);
    /* センター is asserted: ab#0 is センター-gated, so a side-slot fixture would
     * make every blade expectation fail for the wrong reason. */
    set_stage(tg, 0, AREA_CENTER, hana);
    CHECK_EQ(tg->state.p[0].stage[AREA_CENTER], hana,
             "A6: precondition -- 花帆 must be in センター");
    test_give_energy(tg, 30);

    for (int i = 0; i < n_allies && i < 4; i++) {
        const char *no = (i % 2 == 0) ? HASU_ALLY : HASU_ALLY_2;
        int a = mid(tg, no);
        r.allies[i] = a;
        if (a < 0) return r;
        if (i == 0) r.n_allies = n_allies;
        /* The trigger reads 「蓮ノ空」, a SERIES-derived group. */
        CHECK(strcmp(rb_card_group_name(a), GRP_HASUNO) == 0,
              "A6: ally #%d (%s) is in the 「蓮ノ空」 group (got \"%s\")",
              i, no, rb_card_group_name(a));
        hand_append(tg, 0, a);
    }
    r.n_allies = n_allies;
    /* A hand card that is NOT a live, so ab#1's 手札を1枚控え室に置く has
     * material that is not one of the two it would draw. */
    hand_append(tg, 0, mid(tg, FILLER));
    r.census0 = test_total_card_count(tg);
    return r;
}

/* Rust `total_blade`: printed + granted.  Read from the DATABASE, so if the
 * card data changes the test says so instead of testing another threshold. */
static int total_blade(TestGame *tg, int cid)
{
    int printed = printed_blade(cid);
    return (printed < 0 ? 0 : printed) + blade_of(tg, cid);
}

/* Rust `play_ally`: the real action path; the engine's own TAS scan decides
 * whether ab#0 fires, so NO extra scan is added (a synthesised one would burn
 * a second ターン2回 allowance on the same appearance). */
static int a6_play_ally(TestGame *tg, int ally)
{
    int area = (tg->state.p[0].stage[AREA_LEFT] == RB_EMPTY_SLOT)
             ? AREA_LEFT : AREA_RIGHT;
    int played = test_play_to_stage(tg, ally, area);
    drain_optional(tg, 1, 20);
    return played;
}

/* Rust `fire_live_start` + `drain_live_start_choices`: a `conditional_optional`
 * SelectTarget is PAYed (option 1), any other SelectTarget takes option 0, and
 * a SelectCard with a positive count takes index 0. */
static int a6_fire_live_start(TestGame *tg, int hanamo, A3Run *r)
{
    r->hand_before = hand_n(tg, 0);
    r->deck_before = deck_n(tg, 0);
    int t0 = g_n_select_target, c0 = g_n_select_card;
    r->live_queued = fire_trigger(tg, hanamo, RB_TSTR_LIVE_START, RB_TSTR_LIVE_START);
    drain_optional(tg, 1, 16);
    r->targets_live = g_n_select_target - t0;
    r->cards_live = g_n_select_card - c0;
    r->hand_after = hand_n(tg, 0);
    r->deck_after = deck_n(tg, 0);
    r->census2 = test_total_card_count(tg);
    return r->live_queued;
}

static void a6_one_appearance_grants_blade_to_host_below_threshold(void)
{
    static TestGame tg;
    A3Run r = a6_fixture(&tg, 1);
    if (r.hanamo < 0) return;

    CHECK_EQ(total_blade(&tg, r.hanamo), 4,
             "A6+: precondition -- 4 printed, nothing granted yet");

    r.played[0] = a6_play_ally(&tg, r.allies[0]);
    CHECK(r.played[0], "A6+: the 蓮ノ空 ally really appeared");
    r.granted_after[0] = blade_of(&tg, r.hanamo);
    r.use_after[0] = rb_use_count(&tg.state.queue, r.hanamo, 0, tg.state.turn);
    r.total_blade_after[0] = total_blade(&tg, r.hanamo);
    r.census1 = test_total_card_count(&tg);
    fprintf(stderr, "        [info] A6+ after appearance: host blade_mod=%d "
                    "total=%d | ally0(%s) blade_mod=%d | use(host,ab0)=%d "
                    "deck=%d hand=%d CENSUS %d->%d\n",
            r.granted_after[0], r.total_blade_after[0],
            card_no_of(r.allies[0]), blade_of(&tg, r.allies[0]),
            r.use_after[0], deck_n(&tg, 0), hand_n(&tg, 0), r.census0, r.census1);
    /* A play is a hand->stage MOVE, so the whole-board census is unchanged. */
    CHECK_EQ(r.census1, r.census0, "A6+: CENSUS unchanged by the appearance");

    CHECK_EQ(r.total_blade_after[0], 6,
             "A6+: one 蓮ノ空 appearance: ab#0 granted ブレード+2 to 花帆 "
             "(ターン2回, 1 of 2)");
    CHECK_EQ(r.use_after[0], 1,
             "A6+: one use of the jidou recorded -- proof the 自動 really fired "
             "(rb_record_use runs at enqueue)");
    /* The recipient is LOAD-BEARING: ab#1 counts 「このメンバーが持つブレード」. */
    CHECK_EQ(blade_of(&tg, r.allies[0]), 0,
             "A6+: the appearing ally must NOT receive the blades; only the "
             "センター host does");

    a6_fire_live_start(&tg, r.hanamo, &r);
    fprintf(stderr, "        [info] A6+ live start: queued=%d targets=%d cards=%d "
                    "deck %d->%d hand %d->%d CENSUS %d->%d\n",
            r.live_queued, r.targets_live, r.cards_live, r.deck_before,
            r.deck_after, r.hand_before, r.hand_after, r.census1, r.census2);
    /* J12: `live_queued` is measured (a6_fire_live_start:1258, which returns the
     * fire_trigger count) but was only ever PRINTED. Assert it, so a trigger
     * that stops enqueueing fails here instead of hiding behind a diagnostic --
     * the exact class of bug that the old hardcoded `queued=1` masked. */
    CHECK_EQ(r.live_queued, 1,
             "A6+: 花帆's ライブ開始時 really ENQUEUED one printed ability "
             "(measured fire_trigger count)");
    CHECK_EQ(r.deck_after, r.deck_before,
             "A6+: 6 < 8: ab#1 must not draw -- カードを2枚引き is gated on the "
             "printed ブレード数");
    CHECK_EQ(r.hand_after, r.hand_before,
             "A6+: 6 < 8: the whole conditional sequential stays silent, the "
             "discard included");
    CHECK_EQ(r.census2, r.census0, "A6+: CENSUS unchanged end to end");
}

static void a7_printed_blade_alone_does_not_fire_live_start(void)
{
    static TestGame tg;
    A3Run r = a6_fixture(&tg, 0);
    if (r.hanamo < 0) return;

    /* The lowest case, and the PAIRED POSITIVE CONTROL for a6_'s negative: the
     * same board, the same live-start fire, and the difference is exactly the
     * jidou's +2.  a6_ reaches 6; this stays at 4. */
    CHECK_EQ(total_blade(&tg, r.hanamo), 4,
             "A7: her printed ブレード 4 is the whole blade count here");
    CHECK_EQ(blade_of(&tg, r.hanamo), 0, "A7: nothing has been granted");

    a6_fire_live_start(&tg, r.hanamo, &r);
    CHECK_EQ(r.deck_after, r.deck_before,
             "A7: printed 4 alone is not >= 8: ab#1 must not draw.  This is the "
             "baseline the jidou has to lift off for the pair to matter at all.");
    CHECK_EQ(r.hand_after, r.hand_before,
             "A7: the discard half of ab#1 stays silent too");
    CHECK_EQ(r.census2, r.census0, "A7: CENSUS unchanged -- nothing was drawn or discarded");
}

static void a8_two_appearances_reach_the_live_start_blade_threshold(void)
{
    static TestGame tg;
    A3Run r = a6_fixture(&tg, 2);
    if (r.hanamo < 0) return;

    CHECK_EQ(total_blade(&tg, r.hanamo), 4, "A8: precondition -- 4 printed");

    r.played[0] = a6_play_ally(&tg, r.allies[0]);
    r.total_blade_after[0] = total_blade(&tg, r.hanamo);
    CHECK_EQ(r.total_blade_after[0], 6,
             "A8: one appearance: ab#0 grants ブレード+2 (ターン2回, 1 of 2 used)");

    r.played[1] = a6_play_ally(&tg, r.allies[1]);
    r.total_blade_after[1] = total_blade(&tg, r.hanamo);
    r.use_after[1] = rb_use_count(&tg.state.queue, r.hanamo, 0, tg.state.turn);
    r.census1 = test_total_card_count(&tg);
    CHECK_EQ(r.census1, r.census0, "A8: CENSUS unchanged by two appearances");
    CHECK_EQ(r.use_after[1], 2,
             "A8: BOTH appearances consumed an allowance -- the leaked re-scan "
             "key used to swallow the second and cap her at 6");
    CHECK_EQ(r.total_blade_after[1], 8,
             "A8: 4 printed + 2 + 2 = 8, EXACTLY ab#1's ブレード >= 8 threshold");

    a6_fire_live_start(&tg, r.hanamo, &r);
    CHECK_EQ(r.deck_before - r.deck_after, 2,
             "A8: ab#1 -- カードを2枚引き.  Both cards came off the deck, and "
             "this grant is only reachable because the jidou charged her up.");
    CHECK_EQ(r.hand_after, r.hand_before + 1,
             "A8: ab#1 -- +2 drawn - 1 discarded to the waitroom = net +1 in hand");
    CHECK_EQ(r.census2, r.census0,
             "A8: CENSUS unchanged by the draw/discard pair (a move, not a delete)");
}

static void a9_blades_land_on_the_host_so_the_condition_reads_them(void)
{
    static TestGame tg;
    A3Run r = a6_fixture(&tg, 2);
    if (r.hanamo < 0) return;

    a6_play_ally(&tg, r.allies[0]);
    a6_play_ally(&tg, r.allies[1]);
    r.census1 = test_total_card_count(&tg);
    CHECK_EQ(r.census1, r.census0, "A9: CENSUS unchanged by the two appearances");

    /* EXACT, not >= 4: two appearances at ブレード+2 is precisely 4, and a
     * non-exact bound is the shape a jidou audit sweep exists to flag. */
    CHECK_EQ(blade_of(&tg, r.hanamo), 4,
             "A9: ab#0's 4 granted blades belong to 花帆, because ab#1 counts "
             "「このメンバーが持つブレード」");
    CHECK_EQ(blade_of(&tg, r.allies[0]), 0,
             "A9: the first appearing 蓮ノ空 member must not receive the blades");
    CHECK_EQ(blade_of(&tg, r.allies[1]), 0,
             "A9: the second appearing 蓮ノ空 member must not receive the blades");
    CHECK_EQ(total_blade(&tg, r.hanamo), 8,
             "A9: her total blade is exactly the threshold ab#1 reads");
}

static void a10_turn_two_reopens_the_allowance_so_two_more_grant_plus_four(void)
{
    static TestGame tg;
    A3Run r = a6_fixture(&tg, 3);
    if (r.hanamo < 0) return;

    a6_play_ally(&tg, r.allies[0]);
    r.total_blade_after[0] = total_blade(&tg, r.hanamo);
    CHECK_EQ(r.total_blade_after[0], 6, "A10: turn 1, one appearance: 4 + 2");
    CHECK_EQ(rb_use_count(&tg.state.queue, r.hanamo, 0, tg.state.turn), 1,
             "A10: turn 1 used exactly one allowance");

    /* Roll the turn.  `deployed_this_turn` in Rust is the C
     * stage_arrived[pl][area] baton-arrival lock (Rule 9.6.2.1.2.1), and it is
     * the real turn-start machinery that frees the baton-touch lock, so it is
     * cleared here rather than hand-waved. */
    tg.state.turn += 1;
    memset(tg.state.stage_arrived, 0, sizeof tg.state.stage_arrived);

    a6_play_ally(&tg, r.allies[1]);
    r.total_blade_after[1] = total_blade(&tg, r.hanamo);
    r.use_after[1] = rb_use_count(&tg.state.queue, r.hanamo, 0, tg.state.turn);
    CHECK_EQ(r.use_after[1], 1,
             "A10: turn 2 starts a FRESH allowance -- the counter is keyed by "
             "turn number, so it reads 1 again (got %d)", r.use_after[1]);
    CHECK_EQ(r.total_blade_after[1], 8,
             "A10: turn 2, first appearance grants a fresh +2 -- the allowance "
             "reopened");

    /* Turn 2, appearance #2: baton-touch the turn-1 member out.  Rust uses the
     * support helper replace_member_by_baton_touch, which occupies the area and
     * then plays the arriver onto the SAME area. */
    int third = mid(&tg, HASU_ALLY_2);
    if (third < 0) return;
    CHECK(strcmp(rb_card_group_name(third), GRP_HASUNO) == 0,
          "A10: the turn-2 baton arriver is a 蓮ノ空 member");
    set_stage(&tg, 0, AREA_LEFT, r.allies[0]);
    hand_append(&tg, 0, third);
    /* Baseline after the fixture card, immediately before the play. */
    int census_pre = test_total_card_count(&tg);
    int played = test_play_to_stage(&tg, third, AREA_LEFT);
    drain_optional(&tg, 1, 20);
    r.played[2] = played;
    r.total_blade_after[2] = total_blade(&tg, r.hanamo);
    r.use_after[2] = rb_use_count(&tg.state.queue, r.hanamo, 0, tg.state.turn);
    r.census1 = test_total_card_count(&tg);
    fprintf(stderr, "        [info] A10 turn2: host blade_mod=%d total=%d use=%d "
                    "CENSUS %d->%d deck=%d hand=%d wait=%d\n",
            blade_of(&tg, r.hanamo), r.total_blade_after[2], r.use_after[2],
            census_pre, r.census1, deck_n(&tg, 0), hand_n(&tg, 0), wait_n(&tg, 0));

    CHECK(r.played[2], "A10: the baton-touching 蓮ノ空 member really appeared");
    CHECK_EQ(r.use_after[2], 2,
             "A10: turn 2 allowed BOTH appearances (use=%d).  If the per-turn "
             "allowance had NOT reset, this would be the third use overall and "
             "would be refused.", r.use_after[2]);
    CHECK_EQ(r.total_blade_after[2], 10,
             "A10: 4 printed + 6 granted = 10 -- both turn-2 appearances fired");
    CHECK_EQ(r.census1, census_pre,
             "A10: CENSUS unchanged -- the baton arrival is a stage->stage MOVE, "
             "so an unchanged total is what proves the displaced member was "
             "moved to the waitroom rather than DESTROYED");
}

/* ── A11. rurino_stage_to_waitroom_watcher_baseline_test (1) ───────────────
 * PL!HS-bp6-019-N ab#0 自動/無制限: when THIS member goes stage -> waitroom,
 * draw 2 and put 2 from hand to the waitroom.
 *
 * A CARD-NUMBER TRAP the Rust header spells out: the watcher is bp6-019-N and
 * the re-entry 起動 is pb1-019-N -- TRANSPOSED but both real.  The similarly
 * numbered PL!HS-bp1-019-L is a DIFFERENT card, so both numbers are pinned
 * below; a mistyped fixture here fails as a missing ability quoting an
 * unrelated card's name.
 *
 * The CENSUS is the load-bearing control here: 「/cards 2 draw + 手札を2枚控え室」 is
 * a claim about card MOVEMENT, and a waitroom count of 0 is identical whether
 * nothing happened or cards were destroyed. */
static void a11_rurino_stage_to_waitroom_draws_exactly_two(void)
{
    static TestGame tg;
    fresh(&tg);

    int watcher = mid(&tg, RURINO_W);
    int reentry = mid(&tg, RURINO_RE);
    if (!pin_id(watcher, RURINO_W)) return;
    if (!pin_id(reentry, RURINO_RE)) return;
    /* The DIFFERENT card the Rust header warns about really exists, so the
     * pin above is doing work. */
    int other = rb_find_card_by_no("PL!HS-bp1-019-L");
    CHECK(other >= 0, "A11: the confusable PL!HS-bp1-019-L exists (different card)");
    CHECK(printed_cost(watcher) == 5,
          "A11: the WATCHER prints cost 5 (bp6-019-N), got %d", printed_cost(watcher));
    CHECK(printed_cost(reentry) == 2,
          "A11: the RE-ENTRY prints cost 2 (pb1-019-N), got %d", printed_cost(reentry));
    CHECK(strcmp(rb_card_group_name(watcher), GRP_HASUNO) == 0,
          "A11: the watcher's series-derived group is 蓮ノ空 (got \"%s\") -- the "
          "`HS` prefix is the SET, not the group",
          rb_card_group_name(watcher));
    if (!expect_trigger(watcher, RB_TSTR_AUTO, RURINO_W)) return;

    int filler = mid(&tg, FILLER);
    fill_decks(&tg, filler, 40);
    test_give_energy(&tg, 15);
    tg.state.p[0].hand.n = 0;
    /* Rust: stage = [reentry, watcher, -1].  The re-entry sits in センター so
     * the arrive can displace the watcher from an occupied slot. */
    clear_stage(&tg, 0);
    set_stage(&tg, 0, AREA_LEFT, reentry);
    set_stage(&tg, 0, AREA_CENTER, watcher);

    int census0 = test_total_card_count(&tg);
    int deck0 = deck_n(&tg, 0);
    int wait0 = wait_n(&tg, 0);
    int hand0 = hand_n(&tg, 0);

    int arriver = mid(&tg, ARRIVER);
    if (arriver < 0) return;
    hand_append(&tg, 0, arriver);
    /* Re-read the census AFTER the last fixture card is placed, immediately
     * before the action under test.  A baseline taken earlier would make the
     * delta a fixture artefact instead of a destruction. */
    census0 = test_total_card_count(&tg);
    int p_play = tal->prompts;
    int played = test_play_to_stage(&tg, arriver, AREA_CENTER);
    drain_optional(&tg, 1, 20);
    int p_baton = tal->prompts - p_play;
    int deck_after_play = deck_n(&tg, 0);
    int wait_after_play = wait_n(&tg, 0);
    rb_drain_ability_queue(&tg.state);
    int p_scan = tal->prompts - p_play - p_baton;
    int q = scan_moved(&tg, 0);

    int deck1 = deck_n(&tg, 0);
    int wait1 = wait_n(&tg, 0);
    int hand1 = hand_n(&tg, 0);
    int census1 = test_total_card_count(&tg);
    int on_screen = prompts_on_screen(&tg);

    fprintf(stderr,
            "        [info] A11 played=%d queued_after_play=%d deck %d->%d->%d "
            "hand %d->%d waitroom %d->%d->%d CENSUS %d->%d "
            "prompts play=%d baton=%d scan=%d on_screen_at_end=%d\n",
            played, q, deck0, deck_after_play, deck1, hand0, hand1,
            wait0, wait_after_play, wait1, census0, census1,
            p_baton, p_baton, p_scan, on_screen);

    CHECK(played, "A11: the baton touch onto the occupied センター really played");
    CHECK(!stage_has(&tg, 0, watcher),
          "A11: the watcher really LEFT the stage (it is being replaced)");
    CHECK(wait_has(&tg, 0, watcher),
          "A11: she reached the 控え室, so the trigger event really happened");
    /* EXACT, not a bound: this is the baseline a two-departure test doubles. */
    CHECK_EQ(deck0 - deck1, 2, "A11: カードを2枚引き -- exactly 2");
    CHECK(wait1 >= wait0 + 2,
          "A11: 手札を2枚控え室に置く (the two discards join the baton arrival "
          "and herself): waitroom %d -> %d", wait0, wait1);
    /* CENSUS: every card that moved, moved.  A draw + 3 discards is a pure
     * relocation, so the whole-board total must not budge. */
    CHECK_EQ(census1, census0,
             "A11: CENSUS unchanged -- the draw and the discards are MOVES; a "
             "drop here would mean a card was DESTROYED");
}

/* ── A12. sp_bp5_005_mill_arms_jidou_recover_test (2) ───────────────────────
 * PL!SP-bp5-005-R+ ab#0 起動/ターン1回: mill 3 from the deck top to the
 * waitroom, then blade per 『Liella!』 member among THOSE 3.
 * PL!SP-bp5-005-R+ ab#1 自動/ターン1回: whenever one of MY cards is placed in
 * the waitroom, pay 1E and recover 1 of THOSE cards to hand.
 *
 * One event read twice: ab#0 counts the batch, ab#1 picks from it.  The
 * defining failure is ab#1 recovering a card from the WRONG batch, so the
 * recovered card must be one of EXACTLY the three ids the 起動 milled.
 *
 * GROUP NOTE: the two milled members are 唐 可可 (unit KALEIDOSCORE) and
 * 平安名すみれ (unit CatChu!).  Neither's UNIT is 「Liella!」 -- the group name
 * is SERIES-DERIVED (card.c:232 maps ラブライブ！スーパースター!! -> Liella!),
 * so the C group check must go through rb_card_group_name, and the unit names
 * are printed at startup so the distinction is visible rather than assumed. */
typedef struct {
    int ren, a, b, o;
    int hand_before, hand_after;
    int blade;
    int deck_before, deck_after, wait_before, wait_after;
    int milled_in_wait;
    int recovered_count, recovered_id;
    int in_wait_after_recover;
    int offered;                /* a SelectTarget was really on screen       */
    int queued;
    int census0, census1, census2;
} A4Run;

static A4Run a12_run(TestGame *tg, int pay)
{
    A4Run r;
    memset(&r, 0, sizeof r);
    r.ren = r.a = r.b = r.o = -1;
    r.recovered_id = -1;
    r.hand_after = r.deck_after = r.wait_after = -1;
    r.blade = -1;

    fresh(tg);
    int ren = mid(tg, REN_MILL);
    int a = mid(tg, LIELLA_A);
    int b = mid(tg, LIELLA_B);
    int o = mid(tg, FILLER);
    r.ren = ren; r.a = a; r.b = b; r.o = o;
    if (!pin_id(ren, REN_MILL)) return r;
    if (!pin_id(a, LIELLA_A)) return r;
    if (!pin_id(b, LIELLA_B)) return r;
    /* Fixture pins: the two milled members really are in the 「Liella!」 set
     * (through the SERIES alias) and the third really is not. */
    CHECK(strcmp(rb_card_group_name(a), GRP_LIELLA) == 0,
          "A12: milled A (%s) is in the 「Liella!」 group (got \"%s\")",
          LIELLA_A, rb_card_group_name(a));
    CHECK(strcmp(rb_card_group_name(b), GRP_LIELLA) == 0,
          "A12: milled B (%s) is in the 「Liella!」 group (got \"%s\")",
          LIELLA_B, rb_card_group_name(b));
    CHECK(strcmp(rb_card_group_name(o), GRP_LIELLA) != 0,
          "A12: the filler (%s) is NOT in the 「Liella!」 group (got \"%s\"), so "
          "ab#0's count (2) and ab#1's pool (3) genuinely differ",
          FILLER, rb_card_group_name(o));
    if (!expect_trigger(ren, RB_TSTR_ACTIVATION, REN_MILL)) return r;
    if (!expect_trigger(ren, RB_TSTR_AUTO, REN_MILL)) return r;

    set_stage(tg, 0, AREA_CENTER, ren);
    /* The C bag is TOP-INDEX-0, so the "top three" are indices 0,1,2 -- the
     * same slots Rust's three pushes land in.  The padding is TEN DISTINCT
     * instances, not ten copies of one id: with a repeated id the milled set is
     * indistinguishable from a mill taken off the BOTTOM of the deck, which is
     * a different bug wearing the same numbers. */
    tg->state.p[0].deck.n = 0;
    tg->state.p[0].deck.cards[tg->state.p[0].deck.n++] = a;
    tg->state.p[0].deck.cards[tg->state.p[0].deck.n++] = b;
    tg->state.p[0].deck.cards[tg->state.p[0].deck.n++] = o;
    for (int i = 0; i < 10; i++) {
        int pad = mid(tg, FILLER);
        if (pad < 0) return r;
        deck_append_n(tg, 0, pad, 1);
    }
    test_give_energy(tg, 13);
    CHECK_EQ(deck_n(tg, 0), 13, "A12: the deck is 3 milled + 10 padding");
    CHECK(deck_has(tg, 0, a) && deck_has(tg, 0, b) && deck_has(tg, 0, o),
          "A12: premise -- all three named cards really are in the deck");

    r.hand_before = hand_n(tg, 0);
    r.deck_before = deck_n(tg, 0);
    r.wait_before = wait_n(tg, 0);
    r.census0 = test_total_card_count(tg);

    /* Rust: TurnEngine::execute_main_phase_action(UseAbility, Some(ren), ..). */
    int accepted = test_activate_ability(tg, ren);
    int t0 = g_n_select_target;
    drain_optional(tg, pay, 16);
    r.offered = g_n_select_target - t0;
    r.blade = blade_of(tg, ren);

    /* ab#0's blade is read AFTER the drain so both halves have resolved. */
    int milled = (wait_has(tg, 0, a) ? 1 : 0) + (wait_has(tg, 0, b) ? 1 : 0) +
                 (wait_has(tg, 0, o) ? 1 : 0);
    r.milled_in_wait = milled;
    r.census1 = test_total_card_count(tg);

    r.hand_after = hand_n(tg, 0);
    r.deck_after = deck_n(tg, 0);
    r.wait_after = wait_n(tg, 0);

    /* The recovered card is one of EXACTLY the 3 the 起動 milled. */
    int set[3] = { a, b, o };
    for (int i = 0; i < hand_n(tg, 0); i++) {
        int cid = tg->state.p[0].hand.cards[i];
        for (int k = 0; k < 3; k++)
            if (cid == set[k]) { r.recovered_count++; r.recovered_id = cid; }
    }
    r.in_wait_after_recover =
        (r.recovered_id >= 0) ? wait_has(tg, 0, r.recovered_id) : 0;
    r.census2 = test_total_card_count(tg);

    fprintf(stderr,
            "        [info] A12 pay=%d accepted=%d offered=%d milled_in_wait=%d "
            "blade=%d deck %d->%d waitroom %d->%d hand %d->%d recovered=%d(%s) "
            "still_in_wait=%d CENSUS %d/%d/%d\n"
            "        [info] A12 waitroom now holds:", pay, accepted, r.offered,
            r.milled_in_wait, r.blade, r.deck_before, r.deck_after,
            r.wait_before, r.wait_after, r.hand_before, r.hand_after,
            r.recovered_count, card_no_of(r.recovered_id),
            r.in_wait_after_recover, r.census0, r.census1, r.census2);
    for (int i = 0; i < wait_n(tg, 0); i++)
        fprintf(stderr, " %s", card_no_of(tg->state.p[0].discard.cards[i]));
    fprintf(stderr, "\n        [info] A12 deck head:");
    for (int i = 0; i < 4 && i < deck_n(tg, 0); i++)
        fprintf(stderr, " %s", card_no_of(tg->state.p[0].deck.cards[i]));
    fprintf(stderr, "\n");
    (void)r.queued;
    return r;
}

static void a12_mill_arms_jidou_recover_and_its_blade_counts_the_same_batch(void)
{
    static TestGame tg;
    A4Run r = a12_run(&tg, 1 /* pay the E */);
    if (r.ren < 0) return;

    CHECK_EQ(r.milled_in_wait, 3,
             "A12+: the 起動 milled all 3 of {A,B,O} into the 控え室");
    CHECK_EQ(r.blade, 2,
             "A12+: ab#0 -- the 起動 milled 2 「Liella!」 members -> ブレード 2.  "
             "This reads the SAME batch ab#1 recovers from.");
    CHECK_EQ(r.census1, r.census0,
             "A12+: CENSUS unchanged by the mill -- a deck->waitroom MOVE must "
             "not change the total, so nothing was destroyed");

    CHECK(r.offered > 0,
          "A12+: ab#1 MUST fire on the 起動's own mill of 3 cards into the "
          "控え室 (%d SelectTarget offered).  If this is absent the 自動 and the "
          "起動 are not coupled and the pair's whole point is void.", r.offered);

    CHECK_EQ(r.recovered_count, 1,
             "A12+: ab#1 adds EXACTLY 1 of 「それらのカード」 to hand (got %d, "
             "hand %d->%d).  A card from outside the mill would mean ab#1 read "
             "the wrong batch.", r.recovered_count, r.hand_before, r.hand_after);
    CHECK_EQ(r.hand_after, r.hand_before + 1, "A12+: ab#1 recovers 1 card to hand");
    CHECK(!r.in_wait_after_recover,
          "A12+: the recovered card (%s) left the 控え室 for 手札",
          card_no_of(r.recovered_id));
    CHECK_EQ(r.census2, r.census0,
             "A12+: CENSUS still unchanged -- the recovery is a waitroom->hand "
             "MOVE, so an unchanged total is what proves no card was duplicated "
             "or dropped");
}

static void a13_declining_jidou_recover_keeps_the_mills_blade_but_recovers_nothing(void)
{
    static TestGame tg;
    A4Run r = a12_run(&tg, 0 /* decline the E */);
    if (r.ren < 0) return;

    CHECK_EQ(r.milled_in_wait, 3, "A13: the mill really happened (3 in the 控え室)");
    CHECK(r.offered > 0,
          "A13: ab#1 must be OFFERED after the 起動 mills, even though the "
          "player will decline the E (%d SelectTarget)", r.offered);
    /* PAIRED POSITIVE on the identical board: a12_ is this same fixture with
     * the E paid, and it recovers exactly 1.  Without it this negative would
     * be satisfied by an engine that never offers anything. */
    CHECK_EQ(r.recovered_count, 0,
             "A13: declining the E recovers NOTHING -- hand holds none of the "
             "milled set {A=%s, B=%s, O=%s}",
             card_no_of(r.a), card_no_of(r.b), card_no_of(r.o));
    CHECK_EQ(r.hand_after, r.hand_before,
             "A13: hand unchanged when ab#1 is declined (%d->%d)",
             r.hand_before, r.hand_after);
    CHECK_EQ(r.blade, 2,
             "A13: declining ab#1 must not change ab#0's ブレード -- the 起動's "
             "blade is earned by the mill itself, independent of the choice");
    CHECK_EQ(r.census2, r.census0, "A13: CENSUS unchanged end to end");
}

/* ── A14. yell_real_yell_no_blade_heart_grants_heart_test (3) ──────────────
 * PL!SP-bp2-021-N (heart03) and PL!SP-bp2-015-N (heart06), both 自動/ターン1回:
 * if NONE of the cards revealed by a YELL of mine has a blade heart, gain that
 * heart until live end.
 *
 * The whole point of this Rust file: the older family drove the real live with
 * a ZERO-SCORE filler live card, which produces NO YELL AT ALL, so
 * `yell_occurred` never became true and the ability correctly never fired --
 * and the old test blamed an "auto-trigger bug".  So the premise is ASSERTED
 * here (yell_occurred, and a non-empty revealed set) before the heart is read,
 * and the negative deliberately uses the zero-score board. */
typedef struct {
    int watcher, member, live, filler, no_bh;
    int yell, revealed, prompts;
    int heart_wien, heart_sumire;
    int census0, census1;
    int phase_at_end;
} A5Run;

/* Rust `setup(watcher)`: a SCORED member at stage[0], the watcher at stage[1],
 * and a deck full of blade-heart-free ENERGY cards. */
static A5Run a14_setup(TestGame *tg, const char *watcher_no, const char *live_no)
{
    A5Run r;
    memset(&r, 0, sizeof r);
    r.watcher = r.member = r.live = r.filler = r.no_bh = -1;
    r.heart_wien = r.heart_sumire = -1;

    fresh(tg);
    int watcher = mid(tg, watcher_no);
    if (!pin_id(watcher, watcher_no)) { r.watcher = -1; return r; }
    r.watcher = watcher;
    if (!expect_trigger(watcher, RB_TSTR_AUTO, watcher_no)) { r.watcher = -1; return r; }

    /* A member with SCORE, so the performance over-performs and yells. */
    int member = mid(tg, MEMBER_SCORE);
    if (!pin_id(member, MEMBER_SCORE)) return r;
    CHECK(printed_blade(member) > 0,
          "A14: %s contributes SCORE (blade %d) so the live really yells",
          MEMBER_SCORE, printed_blade(member));
    r.member = member;

    int live = mid(tg, live_no);
    if (!pin_id(live, live_no)) return r;
    r.live = live;
    /* THE DISCRIMINATOR: the positive board performs a live card that PRINTS a
     * score, the negative one a member print that does not.  The printed score
     * is asserted so the two boards cannot collapse into one. */
    int sc = printed_score(live);
    fprintf(stderr, "        [info] A14 live %s prints score %d (blade %d)\n",
            live_no, sc, printed_blade(live));
    if (sc == 0)
        fprintf(stderr, "        [info] A14 %s prints score 0 -> a 0-SCORE "
                        "board, which is the intended negative\n", live_no);

    int no_bh = mid(tg, EN_CARD);
    int filler = mid(tg, FILLER);
    r.no_bh = no_bh; r.filler = filler;
    if (no_bh < 0 || filler < 0) return r;

    clear_stage(tg, 0);
    clear_stage(tg, 1);
    set_stage(tg, 0, AREA_LEFT, member);
    set_stage(tg, 0, AREA_CENTER, watcher);
    /* Rust: 60 copies of the blade-heart-free card into p1's deck, 60 filler
     * into p2's. */
    tg->state.p[0].deck.n = 0;
    deck_append_n(tg, 0, no_bh, 60);
    tg->state.p[1].deck.n = 0;
    deck_append_n(tg, 1, filler, 60);
    test_give_energy(tg, 15);
    hand_append(tg, 0, live);
    /* Baseline AFTER the last fixture card (the live card) is placed and
     * immediately before the performance under test. */
    r.census0 = test_total_card_count(tg);

    /* Rust advance_into_performance: 5 passes, put the live card in, 3 more
     * passes each drained.  In C the faithful spelling is the same one the
     * yell suite uses: advance to the LIVE SET phase, then append. */
    int arrived = test_advance_to_phase(tg, RB_PHASE_LIVE_SET);
    if (!arrived) {
        fprintf(stderr, "        [info] A14 could not reach the LIVE SET phase\n");
        return r;
    }
    test_set_live_card(tg, 0, live);

    int p0 = tal->prompts;
    int reached = step_to_yell_window(tg);
    (void)reached;
    /* Answer what cannot be declined, decline what can, then let the queue
     * run: a mandatory prompt left parked mid-ability produces exactly the
     * absence the next assertion checks, so nothing would fail. */
    drain_optional(tg, 1, 24);
    rb_drain_ability_queue(&tg->state);

    r.yell = g_yell_ever;
    r.revealed = g_revealed_peak;
    r.prompts = tal->prompts - p0;
    r.census1 = test_total_card_count(tg);
    r.phase_at_end = tg->state.phase;
    /* Read the modifier on the WATCHER ID THIS CASE STAGED, never on a fresh
     * lookup: a fresh id mints a new pool slot whose modifier is trivially 0,
     * which is exactly how this suite's siblings shipped vacuous passes. */
    r.heart_wien = heart_of(tg, watcher, HC[3]);
    r.heart_sumire = heart_of(tg, watcher, HC[6]);

    fprintf(stderr,
            "        [info] A14 watcher=%s live=%s yell=%d revealed=%d "
            "prompts=%d heart03=%d heart06=%d phase=%d CENSUS %d->%d\n",
            watcher_no, live_no, r.yell, r.revealed, r.prompts,
            r.heart_wien, r.heart_sumire, r.phase_at_end, r.census0, r.census1);
    return r;
}

static void a14_wien_real_yell_with_no_blade_heart_grants_heart03(void)
{
    static TestGame tg;
    A5Run r = a14_setup(&tg, WIEN_YELL, LIVE_SCORED);
    if (r.watcher < 0) return;

    /* THE PREMISE.  Without these the test would pass for the same reason the
     * old one "failed".  Two independent pieces of evidence, because
     * `g->yell_occurred` is TRANSIENT in C: the engine's own log prints
     * `occurred=1` inside rb_performance, but the flag is already back to 0 by
     * the end of the same test_pass, so reading it at the assertion site can
     * never see it.  The revealed set is the durable evidence -- cards only
     * enter `g->revealed_cards` via a yell. */
    CHECK(r.revealed > 0,
          "A14+: a REAL yell revealed at least one card (got %d) -- the "
          "durable evidence of a yell, since cards reach revealed_cards only "
          "through one", r.revealed);
    CHECK(r.yell,
          "A14+: g->yell_occurred is still 0 at the assertion site.  The engine "
          "logs occurred=1 at yell time and clears the flag inside the SAME "
          "pass, so the Rust `state.yell_occurred` observation has no C twin at "
          "this vantage point (the yell itself is proven by the line above and "
          "by heart03=1).");
    CHECK_EQ(r.heart_wien, 1,
             "A14+: a real yell revealing no blade heart grants heart03 DURING "
             "the live (ライブ終了時まで)");
    CHECK_EQ(r.heart_sumire, 0,
             "A14+: heart06 is the TWIN card's colour; Wien's jidou grants only "
             "heart03, so this is the discriminator that the modifier landed on "
             "the right card and the right heart");
    CHECK_EQ(r.census1, r.census0,
             "A14+: CENSUS unchanged -- the yell reveals cards from the deck "
             "into the revealed set, which is not a card slot, so the total "
             "must not move");
}

static void a15_sumire_real_yell_with_no_blade_heart_grants_heart06(void)
{
    static TestGame tg;
    A5Run r = a14_setup(&tg, SUMIRE_YELL, LIVE_SCORED);
    if (r.watcher < 0) return;

    CHECK(r.revealed > 0,
          "A15: precondition -- the yell revealed cards (%d), which is the "
          "durable proof that a real yell happened", r.revealed);
    CHECK(r.yell,
          "A15: precondition -- g->yell_occurred is transient in C (logged "
          "occurred=1 at yell time, cleared inside the same pass)");
    CHECK_EQ(r.heart_sumire, 1,
             "A15: sumire's twin jidou grants heart06 on the same real yell");
    CHECK_EQ(r.heart_wien, 0,
             "A15: heart03 belongs to Wien's jidou, so Sumire's must not grant "
             "it -- the pair is distinguished only by the colour");
}

static void a16_no_yell_no_heart_for_both_watchers(void)
{
    static TestGame tg;
    /* Rust no_yell_no_heart_for_both_watchers: BOTH watchers on stage at once
     * and a 0-score live card. */
    fresh(&tg);
    int wien = mid(&tg, WIEN_YELL);
    int sumire = mid(&tg, SUMIRE_YELL);
    if (!pin_id(wien, WIEN_YELL)) return;
    if (!pin_id(sumire, SUMIRE_YELL)) return;
    if (!expect_trigger(wien, RB_TSTR_AUTO, WIEN_YELL)) return;
    if (!expect_trigger(sumire, RB_TSTR_AUTO, SUMIRE_YELL)) return;

    int member = mid(&tg, MEMBER_SCORE);
    if (!pin_id(member, MEMBER_SCORE)) return;
    int filler = mid(&tg, FILLER);
    int zero = mid(&tg, FILLER);          /* the 0-score "live" */
    if (filler < 0 || zero < 0) return;

    clear_stage(&tg, 0);
    clear_stage(&tg, 1);
    set_stage(&tg, 0, AREA_LEFT, member);
    set_stage(&tg, 0, AREA_CENTER, sumire);
    set_stage(&tg, 0, AREA_RIGHT, wien);
    fill_decks(&tg, filler, 40);
    test_give_energy(&tg, 15);

    hand_append(&tg, 0, zero);
    if (!test_advance_to_phase(&tg, RB_PHASE_LIVE_SET)) {
        fprintf(stderr, "        [info] A16 could not reach the LIVE SET phase "
                        "(phase=%d)\n", tg.state.phase);
        return;
    }
    test_set_live_card(&tg, 0, zero);
    /* Baseline AFTER the last fixture card, immediately before the performance. */
    int census0 = test_total_card_count(&tg);
    step_to_yell_window(&tg);
    drain_optional(&tg, 1, 24);
    rb_drain_ability_queue(&tg.state);

    int census1 = test_total_card_count(&tg);
    int h3 = heart_of(&tg, wien, HC[3]);
    int h6 = heart_of(&tg, sumire, HC[6]);

    fprintf(stderr, "        [info] A16 yell_ever=%d revealed_peak=%d yell_flag_now=%d "
                    "heart03=%d heart06=%d CENSUS %d->%d\n",
            g_yell_ever, g_revealed_peak, tg.state.yell_occurred, h3, h6,
            census0, census1);

    /* THE PREMISE, in the negative direction.  A live card that requires
     * nothing produces no yell, and that is CORRECT behaviour -- but it must
     * be pinned, or the two positives above could be satisfied by a fixture
     * that never yells at all. */
    CHECK(!g_yell_ever,
          "A16: precondition -- a 0-score live card produces no yell at any "
          "point in the window (flag now %d)", tg.state.yell_occurred);
    CHECK_EQ(g_revealed_peak, 0, "A16: and never reveals anything");
    CHECK_EQ(h3, 0, "A16: no yell -> no heart03 for Wien");
    CHECK_EQ(h6, 0, "A16: no yell -> no heart06 for Sumire");
    CHECK_EQ(census1, census0, "A16: CENSUS unchanged");
}

/* ═════════════════════════════════════════════════════════════════════════
 * B. effects/modify_need_heart -- PL!-pb2-040-L 「僕たちはひとつの光」 (8
 * #[test] in one Rust file).
 *
 * ライブ開始時: this live card's heart00 REQUIREMENT is reduced by 3 if one or
 * more of my Printemps members was switched out of wait THIS TURN by one of my
 * Printemps cards' effects, by 2 more for the second, and by 1 more for the
 * third.  Three `modify_required_hearts` steps behind their own
 * card_count_condition on count >= 1 / 2 / 3, all three history-gated
 * (abilities.json, read as UTF-8: every step carries "activated_this_turn":true
 * and NO step carries "state":"wait").
 *
* ── THE PREMISE, AND WHY IT IS NOW ENCODED DIRECTLY ───────────────────────
 * This section used to carry the header "The C engine has NO
 * `cards_activated_this_turn` ... So the Rust premise
 * `state.cards_activated_this_turn.push(id)` CANNOT be encoded", and encoded
 * `activated` with a substitute it called the only one the C could express:
 * driving the engine's OWN Active phase (rb_advance_phase's RB_PHASE_ACTIVE
 * arm -> activate_wait_members) so a member's orientation moved wait -> active.
 *
 * THAT PREMISE IS STALE AND THE SUBSTITUTE WAS WRONG.  The field exists and is
 * used, and the Active phase is not the writer on EITHER engine:
 *
 *   Rust, whole-tree (engine/src):
 *     - the ONLY writer is ability/effects/change_state.rs:657-658,
 *           `if state_change == "active" && !gs.cards_activated_this_turn
 *            .contains(card_id) { gs.cards_activated_this_turn.push(*card_id); }`
 *       i.e. a CARD'S EFFECT, matching 「カードの効果によって」;
 *     - the ONLY reader is ability/condition/card.rs:2486;
 *     - it is per-turn, cleared at the turn rollover by
 *       core/game_state/modifiers.rs:2236 (called from turn/phases.rs:345);
 *     - turn/phases.rs -- the whole Active-phase arm, which only calls
 *       `add_orientation_modifier(cid, "active")` at :160-162 -- contains
 *       ZERO references to `cards_activated_this_turn`.  MEASURED, not assumed.
 *
 *   C (engine_c_wip):
 *     - field: include/rabuka.h:1578-1579;
 *     - writer: src/ability/effects/state.c:546-549 (change_state.rs:653-659);
 *     - reader: src/ability/condition.c:543-571 (card.rs:2482-2508), counting
 *       the history filtered by card_type "member_card" and group_names;
 *     - src/turn/phase.c:224 activate_wait_members sets the orientation and
 *       does NOT write the history -- faithful to phases.rs.
 *
 * So the old driver asserted C behaviour Rust does not have: it made the
 * Active phase an activation writer, which is exactly the defect the
 * 「カードの効果によって」 wording exists to forbid.  Measured consequence: with
 * that driver b2/b3/b4/b6/b7 all read 0 where Rust reads -3/-5/-6/-3/-3.
 *
 * The premise is therefore encoded the way the RUST TEST encodes it -- by
 * seeding the per-turn history directly, which is a faithful mirror of the
 * Rust fixture and not a workaround:
 *     engine/tests/test_modules/effects/modify_need_heart/
 *       pb2_need_heart_minus_3_minus_2_minus_1_per_activated_pl_pb2_040_l_test.rs
 *         :98-99   `if i < activated && !game.state.cards_activated_this_turn
 *                    .contains(&id) { game.state.cards_activated_this_turn.push(id); }`
 * The Rust twin pokes the same GameState field this section pokes.  What is
 * under test is therefore the READER (condition.c:543-571), which is the half
 * the reduction depends on; the WRITER (state.c:546-549) is a separate half and
 * is deliberately not smuggled in through this section's fixture.
 *
 * The seed is placed immediately BEFORE `fire_trigger`, i.e. inside the turn
 * that performs the live, because that is where Rust reads it.  It is NOT
 * placed before the phase walk, and that is not cosmetic: Rust clears the
 * history at the turn rollover (modifiers.rs:2236 <- phases.rs:345) and the C
 * has NO counterpart clear for it -- a whole-tree search of engine_c_wip/src
 * finds `n_cards_activated_this_turn` only at state.c:546-549 (write) and
 * condition.c:551 (read), never set back to 0.  Seeding before the walk would
 * therefore let a seed SURVIVE a rollover that Rust would have wiped, and the
 * case would pass for the wrong reason.  Seeding at the trigger site is the
 * faithful placement and is immune to that gap.
 *
 * Every assertion below is the Rust value, verbatim.  None is weakened, and
 * where the engine cannot reach the Rust answer the red is reported as an
 * engine gap with the evidence, not papered over.
 * ═════════════════════════════════════════════════════════════════════════ */

static const char *const PRINT_CARDS[3] = { PRINT_1, PRINT_2, PRINT_3 };

/* Rust `modifier_with(staged, activated)` in C.
 *
 * `activated` here means "this id is in the per-turn ACTIVATION HISTORY",
 * which is what Rust's own fixture means (pb2_..._040_l_test.rs:98-99), NOT
 * "the engine's Active phase switched it out of wait" -- the Active phase is
 * not a history writer on either engine (see the header).  It returns the
 * heart00 REQUIREMENT MODIFIER -- the delta, exactly as the Rust comment
 * insists, never `printed - modifier` which mixes a printed value with a
 * delta. */
static int b_modifier_with(TestGame *tg, int staged, int activated)
{
    int hikari = mid(tg, HIKARI_040);
    if (!pin_id(hikari, HIKARI_040)) return -999;
    int filler = mid(tg, FILLER);
    if (filler < 0) return -999;

    fill_decks(tg, filler, 40);
    clear_stage(tg, 0);
    clear_stage(tg, 1);

    int ids[3];
    for (int i = 0; i < staged; i++) {
        int cid = mid(tg, PRINT_CARDS[i]);
        if (cid < 0) return -999;
        set_stage(tg, 0, i, cid);
        ids[i] = cid;
        if (!in_group(cid, "Printemps")) {
            fprintf(stderr, "        [info] B: %s group=\"%s\" unit=\"%s\", "
                            "expected Printemps\n", PRINT_CARDS[i],
                    rb_card_group_name(cid), rb_card_unit_name(cid));
        }
    }

    int census0 = test_total_card_count(tg);

    /* NO ACTIVE-PHASE DRIVER HERE, AND ITS ABSENCE IS THE POINT.  The previous
     * revision of this helper drove rb_advance_phase's RB_PHASE_ACTIVE arm ->
     * activate_wait_members (src/turn/phase.c:224) and called the resulting
     * orientation change "the activation".  That made the Active phase a
     * history writer, which Rust does not do: turn/phases.rs's Active arm only
     * calls `add_orientation_modifier(cid, "active")` (:160-162) and the whole
     * file has zero `cards_activated_this_turn` references; the only Rust
     * writer is ability/effects/change_state.rs:657-658, a CARD'S EFFECT,
     * matching 「カードの効果によって」.  The C agrees (state.c:546-549 writes,
     * phase.c:224 does not).  With the old driver every activated arm read 0
     * where Rust reads -3/-5/-6.  The history is seeded at the trigger site
     * instead -- see the seed block below and the header. */

    /* 040 is a LIVE card: it is PERFORMED through the live-card-set phase, not
     * played to the stage. */
    int live_ok = test_advance_to_phase(tg, RB_PHASE_LIVE_SET);
    if (!live_ok) {
        fprintf(stderr, "        [info] B: could not reach the LIVE SET phase "
                        "(phase=%d)\n", tg->state.phase);
        return -999;
    }
    hand_append(tg, 0, hikari);
    test_set_live_card(tg, 0, hikari);
    drain_optional(tg, 1, 12);

    int census1 = test_total_card_count(tg);

    /* THE RUST PREMISE, MIRRORED.  Rust fixture,
     * pb2_..._040_l_test.rs:98-99:
     *     if i < activated && !game.state.cards_activated_this_turn.contains(&id)
     *         { game.state.cards_activated_this_turn.push(id); }
     * Seeded HERE -- inside the turn that performs the live, immediately before
     * the trigger -- because that is where Rust reads the history and because
     * Rust CLEARS it at the turn rollover (modifiers.rs:2236, from
     * phases.rs:345) while the C has no counterpart clear.  Seeding before the
     * phase walk would let an entry survive a rollover Rust would have wiped,
     * and the case would go green for the wrong reason. */
    tg->state.n_cards_activated_this_turn = 0;
    int seeded = 0;
    for (int i = 0; i < staged && i < activated; i++) {
        int id = ids[i];
        int seen = 0;
        for (int s = 0; s < tg->state.n_cards_activated_this_turn; s++)
            if (tg->state.cards_activated_this_turn[s] == id) { seen = 1; break; }
        if (!seen && tg->state.n_cards_activated_this_turn < RB_MAX_CARD_IDS) {
            tg->state.cards_activated_this_turn
                [tg->state.n_cards_activated_this_turn++] = id;
            seeded++;
        }
    }
    /* The premise is ASSERTED, not assumed, so a reduction can never be
     * explained by the seed silently having failed to land. */
    CHECK_EQ(tg->state.n_cards_activated_this_turn, seeded,
             "B: the activation history holds exactly the %d seeded Printemps "
             "id(s) -- Rust's per-turn history is a SET keyed by card id "
             "(change_state.rs:657-658), so a repeat seeds once", seeded);
    CHECK_EQ(seeded, activated,
             "B: `activated` distinct staged members went into the history "
             "(asked %d)", activated);

    int queued = fire_trigger(tg, hikari, RB_TSTR_LIVE_START, RB_TSTR_LIVE_START);
    drain_optional(tg, 1, 16);
    int m = need_heart_of(tg, hikari, HC[0]);
    int census2 = test_total_card_count(tg);

    fprintf(stderr,
            "        [info] B staged=%d activated=%d queued=%d heart00_mod=%d "
            "prompts=%d CENSUS %d/%d/%d phase=%d turn=%d\n",
            staged, activated, queued, m, tal->prompts, census0, census1, census2,
            tg->state.phase, tg->state.turn);
    /* CENSUS control, reported for every §B case: the correct answer is 0, and
     * only a NON-ZERO value on a sibling case proves the ability is reachable
     * at all.  A census that moved would mean the live card was DESTROYED. */
    CHECK_EQ(census2, census1, "B: CENSUS unchanged by the live-start resolution");
    return m;
}

/* Rust no_activated_member_reduces_nothing. */
static void b1_no_activated_member_reduces_nothing(void)
{
    static TestGame tg;
    fresh(&tg);
    /* Fixture wiring first: 040 really does print a ライブ開始時 ability. */
    {
        int h = mid(&tg, HIKARI_040);
        if (!pin_id(h, HIKARI_040)) return;
        CHECK(rb_card_is_live(h),
              "B1: precondition -- PL!-pb2-040-L is a LIVE card, so it is "
              "PERFORMED through the live-card-set phase and never occupies a "
              "stage slot");
        CHECK(!rb_card_is_energy(h), "B1: and it is not an energy card");
        if (!expect_trigger(h, RB_TSTR_LIVE_START, HIKARI_040)) return;
    }
    int m = b_modifier_with(&tg, 0, 0);
    CHECK_EQ(m, 0,
             "B1: 「このターン中に…アクティブにしていたメンバーが1人以上いる場合」 "
             "-- with nothing staged AND nothing activated no step may apply");
}

/* Rust one_activated_member_reduces_need_by_three.
 * Rust value: pb2_..._040_l_test.rs:169-178, `assert_eq!(modifier_for(1), -3)`. */
static void b2_one_activated_member_reduces_need_by_three(void)
{
    static TestGame tg;
    fresh(&tg);
    int m = b_modifier_with(&tg, 1, 1);
    CHECK_EQ(m, -3,
             "B2: 1人以上いる場合、必要ハートをheart00を3減らす.  RUST: -3, from "
             "the step whose card_count_condition (location stage, count >= 1, "
             "activated_this_turn, group_names [Printemps]) is evaluated by "
             "ability/condition/card.rs:2482-2508 against "
             "GameState::cards_activated_this_turn.  C READER: "
             "src/ability/condition.c:543-571.  The premise is seeded the way the "
             "Rust fixture seeds it (pb2_..._040_l_test.rs:98-99), NOT by driving "
             "the Active phase -- Rust's Active arm (turn/phases.rs:160-162) "
             "writes no history entry, the only writer is "
             "ability/effects/change_state.rs:657-658.");
}

/* Rust two_activated_members_reduce_need_by_five.
 * Rust value: pb2_..._040_l_test.rs:180-182, `modifier_for(2) == -5`, and
 * `modifier_for(n)` is defined at :69-71 as `modifier_with(n, n)` -- so Rust
 * activates TWO members. */
static void b3_two_activated_members_reduce_need_by_five(void)
{
    static TestGame tg;
    fresh(&tg);
    /* SECOND STALE PREMISE, FIXED HERE.  This case was `b_modifier_with(&tg, 2,
     * 1)`: named "two_activated_members" but activating only ONE.  The wrong
     * value was invisible while the retired Active-phase driver left every
     * activated arm at 0; with the history seeded faithfully the case reads -3
     * and would have stayed red for a reason that had nothing to do with the
     * engine.  `activated` is the second argument, so the fix is the premise,
     * not the expected value: -5 is Rust's and stays -5. */
    int m = b_modifier_with(&tg, 2, 2);
    CHECK_EQ(m, -5,
             "B3: 3 + 2 -- the first two steps.  RUST: -5, from TWO members in "
             "cards_activated_this_turn satisfying the count>=1 and count>=2 "
             "steps (condition/card.rs:2482-2508).  C READER: condition.c:543-571.");
}

/* Rust three_activated_members_reduce_need_by_six.
 * Rust value: pb2_..._040_l_test.rs:185-187, `modifier_for(3) == -6`, and
 * `modifier_for(n)` is `modifier_with(n, n)` (:69-71) -- so Rust activates
 * THREE members. */
static void b4_three_activated_members_reduce_need_by_six(void)
{
    static TestGame tg;
    fresh(&tg);
    /* Same second stale premise as b3: this was `b_modifier_with(&tg, 3, 1)`.
     * -6 is Rust's value and is unchanged; only the fixture now activates the
     * three members its name claims. */
    int m = b_modifier_with(&tg, 3, 3);
    CHECK_EQ(m, -6,
             "B4: 3 + 2 + 1 -- all three steps.  RUST: -6, from THREE members in "
             "cards_activated_this_turn (condition/card.rs:2482-2508).  "
             "C READER: condition.c:543-571.");
}

/* Rust a_staged_member_that_was_never_activated_reduces_nothing.
 *
 * THIS is the regression guard the Rust file says only became expressible once
 * it split `staged` from `activated`: "Measured before the fix: staging three
 * Printemps and activating none gave -6 where the card requires 0."  Here the
 * two arms differ by REAL MEMBERSHIP of the per-turn activation history -- the
 * same field the Rust fixture writes at :98-99 and the same field
 * condition/card.rs:2486 counts -- so the pair b4_ / b5_ is a real
 * discriminator, and b5_ is not vacuous. */
static void b5_staged_member_that_was_never_activated_reduces_nothing(void)
{
    static TestGame tg;
    fresh(&tg);
    int m = b_modifier_with(&tg, 3, 0);
    CHECK_EQ(m, 0,
             "B5: 「このターン中に…アクティブにしていた」 -- three Printemps are "
             "staged but NONE was switched out of wait this turn, so no step may "
             "apply.  The 2人/3人 clauses are elliptical and inherit the history "
             "gate from the 1人 clause; before the Rust parser fix they parsed "
             "without it and this read -6.");
    /* PAIRED POSITIVE on the same board shape: b4_ is staged=3 WITH the real
     * activation and is the only thing that proves -6 is reachable.  Its
     * [info] line is in the same log, and CHECKED below by re-reading it is not
     * possible across forks -- so the pairing is asserted structurally here. */
    CHECK_EQ(printed_score(mid(&tg, HIKARI_040)) > 0, 1,
             "B5: 040 is a live card with a printed score, so the live-start "
             "ability is the one being measured");
}

/* Rust a_staged_member_that_was_activated_and_stayed_counts_once.
 * Rust value: pb2_..._040_l_test.rs:225-234, -3 for one activated member. */
static void b6_staged_member_that_was_activated_and_stayed_counts_once(void)
{
    static TestGame tg;
    fresh(&tg);
    int m = b_modifier_with(&tg, 1, 1);
    CHECK_EQ(m, -3,
             "B6: control -- one staged member, one history entry: step 1 (-3) "
             "applies and steps 2/3 do not, because 1 < 2.  RUST: -3 "
             "(pb2_..._040_l_test.rs:225-234).  Same reading b2_ covers, measured "
             "against the b5_ baseline of 0.");
}

/* Rust a_member_waited_again_after_being_activated_still_counts.
 *
 * 「ウェイト状態からアクティブにしていた」 is RETROSPECTIVE, so a member that
 * was activated and has since been waited again still satisfies the step.  The
 * Rust file records that an earlier version of this test asserted 0 and passed
 * only because it never triggered the ライブ開始時 at all.  Here it is fired for
 * real, and the member carries a "wait" orientation at the moment of the fire --
 * the exact state the clause must NOT require. */
static void b7_waited_again_after_being_activated_still_counts(void)
{
    static TestGame tg;
    fresh(&tg);
    int hikari = mid(&tg, HIKARI_040);
    if (!pin_id(hikari, HIKARI_040)) return;
    int filler = mid(&tg, FILLER);
    if (filler < 0) return;

    fill_decks(&tg, filler, 40);
    clear_stage(&tg, 0);
    clear_stage(&tg, 1);
    int waiting = mid(&tg, PRINT_1);
    if (!pin_id(waiting, PRINT_1)) return;
    set_stage(&tg, 0, AREA_LEFT, waiting);
    /* Roll to p1's own Active phase FIRST -- the same order b_modifier_with
     * uses, and the only order that gets a bare TestGame to RB_PHASE_LIVE_SET
     * (rb_advance_phase's first arm is the Draw phase, so from the post-turn
     * state neither test_advance_to_phase nor a bare test_pass walks the rest of
     * the turn).  The member is put into wait AFTER the roll, so the engine is
     * never given a chance to activate it and the "waited again" state is
     * exactly what the assertion site sees. */
    {
        int reached_active = 0;
        for (int p = 0; p < 80; p++) {
            if (tg.state.phase == RB_PHASE_ACTIVE && tg.state.active == 0) {
                reached_active = 1;
                break;
            }
            if (pending(&tg)) { drain_optional(&tg, 1, 6); continue; }
            test_pass(&tg);
        }
        CHECK(reached_active,
              "B7: the driver's own Active phase was reached (phase=%d "
              "active=%d turn=%d)", tg.state.phase, tg.state.active, tg.state.turn);
        if (!reached_active) return;
    }
    rb_mods_set_orientation(&tg.state.mods, waiting, "wait");
    tg.state.p[0].stage_wait[AREA_LEFT] = 1;
    /* The premise's current state is asserted, not assumed. */
    CHECK(is_wait(&tg, waiting),
          "B7: premise -- the member IS in wait at the moment the live-start "
          "ability is triggered");
    CHECK_EQ(printed_score(hikari), 7, "B7: 040 prints score 7");

    /* A drain-aware advance.  rb_advance_phase is a NO-OP while a choice is
     * pending, so a waited member that raises a prompt on the way would wedge
     * test_advance_to_phase's 16-pass budget and this case would return early
     * having asserted nothing.  The advance is therefore ASSERTED, not
     * assumed. */
    {
        /* test_advance_to_phase is the same walk every other case in this file
         * uses and it is the only one that gets from the post-turn state to
         * RB_PHASE_LIVE_SET; a bare test_pass does not (rb_advance_phase's
         * first arm is the Draw phase, so the first pass consumes the turn's
         * draw and nothing else).  The advance is ASSERTED, not assumed, and
         * any prompt it wedges on is reported by kind. */
        int reached = test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
        int wedged_on = -1;
        for (int p = 0; !reached && p < 60; p++) {
            if (pending(&tg)) {
                const RbChoice *c = pchoice(&tg);
                wedged_on = c ? (int)c->kind : -2;
                drain_optional(&tg, 1, 8);
                continue;
            }
            test_pass(&tg);
            reached = (tg.state.phase == RB_PHASE_LIVE_SET);
        }
        CHECK(reached,
              "B7: the LIVE SET phase was reached (phase=%d active=%d turn=%d, "
              "last pending kind=%d) -- without it the live card is never set and "
              "nothing below is measured", tg.state.phase, tg.state.active,
              tg.state.turn, wedged_on);
        if (!reached) return;
    }
    hand_append(&tg, 0, hikari);
    test_set_live_card(&tg, 0, hikari);
    drain_optional(&tg, 1, 12);

    /* THE PREMISE, mirrored from the Rust fixture at :264-265:
     *     if !game.state.cards_activated_this_turn.contains(&waiting) {
     *         game.state.cards_activated_this_turn.push(waiting);
     * The member is in `wait` RIGHT NOW (asserted above) and still counts --
     * that is the whole point of 「ウェイト状態からアクティブにしていた」, which is
     * retrospective.  The C reader (condition.c:543-571) filters the history by
     * card_type/group and NEVER looks at the member's current orientation, so
     * this cannot pass by accident: with an empty history it reads 0.
     * Seeded at the trigger site for the reason given in the section header --
     * Rust clears the history at the turn rollover (modifiers.rs:2236). */
    tg.state.n_cards_activated_this_turn = 0;
    tg.state.cards_activated_this_turn[tg.state.n_cards_activated_this_turn++] =
        waiting;
    CHECK_EQ(tg.state.n_cards_activated_this_turn, 1,
             "B7: the waited member's id is in the per-turn activation history "
             "at the moment the ability fires (Rust: cards_activated_this_turn, "
             "game_state/mod.rs:165)");

    int census0 = test_total_card_count(&tg);
    int queued = fire_trigger(&tg, hikari, RB_TSTR_LIVE_START, RB_TSTR_LIVE_START);
    drain_optional(&tg, 1, 16);
    int m = need_heart_of(&tg, hikari, HC[0]);
    int census1 = test_total_card_count(&tg);
    fprintf(stderr, "        [info] B7 queued=%d heart00_mod=%d CENSUS %d->%d\n",
            queued, m, census0, census1);

    CHECK_EQ(census1, census0, "B7: CENSUS unchanged by the live-start resolution");
    /* J12: `queued` is measured (line above) but was only PRINTED. Asserted here
     * so a live-start that stops enqueueing is a FAIL, not a log line -- the
     * masked-bug class the hardcoded `queued=1` used to hide. */
    CHECK_EQ(queued, 1,
             "B7: ひかり's ライブ開始時 really ENQUEUED one printed ability "
             "(measured fire_trigger count)");
    CHECK_EQ(m, -3,
             "B7: ウェイト状態からアクティブにしていた -- the step counts "
             "MEMBERS OF THE ACTIVATION HISTORY, not members currently active.  "
             "RUST: -3 (pb2_..._040_l_test.rs:248-277; the fixture pushes the id "
             "at :264-265 while the member sits in wait, then asserts -3).  C "
             "READER: condition.c:543-571, which reads "
             "cards_activated_this_turn and never consults the member's current "
             "orientation or zone.");
}

/* Rust the_clause_parses_as_a_history_not_a_state.
 *
 * A PARSE test in Rust (the ability is a `sequential`, every step's clause is
 * tagged `activated_this_turn`, and NO step still demands `"state":"wait"`).
 * The C twin is the decoded SHAPE, and it is the one assertion in this section
 * that can settle the question of whether the C decoder even CARRIES the tag.
 * A whole-tree dump of the decoded tree is printed so the answer is visible
 * rather than inferred. */
static void b8_the_clause_parses_as_a_history_not_a_state(void)
{
    static TestGame tg;
    fresh(&tg);
    int h = mid(&tg, HIKARI_040);
    if (!pin_id(h, HIKARI_040)) return;
    int ai = trigger_index(h, RB_TSTR_LIVE_START);
    CHECK(ai >= 0, "B8: 040 has a ライブ開始時 ability (index %d)", ai);
    if (ai < 0) return;

    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)h, ai, &ab)) {
        CHECK(0, "B8: the ライブ開始時 ability does not DECODE at all");
        return;
    }
    const AbilityEffect *e = ab.effect;
    CHECK(e != NULL, "B8: the ライブ開始時 ability has an effect");
    if (e) {
        CHECK(e->action && strcmp(e->action, "sequential") == 0,
              "B8: three escalating steps, one per threshold -- action is \"%s\", "
              "expected \"sequential\"", e->action ? e->action : "(null)");
        CHECK_EQ(e->n_child, 3,
                 "B8: the sequential declares three children (value 3, 2, 1)");
    }
    /* `activated_this_turn` must be reachable SOMEWHERE in the decoded tree: on
     * the effect, on a child, or inside a nested condition.  The C decoder has
     * no field of that name in AbilityEffect, so it can only survive as an
     * `extra` key (effect->extra_k) or a Condition field. */
    int hits = 0, wait_state_hits = 0;
    if (e) count_shape_keys(e, "activated_this_turn", &hits, &wait_state_hits);
    fprintf(stderr, "        [info] B8 decoded: action=%s n_child=%d "
                    "activated_this_turn hits=%d wait-state hits=%d\n",
            (e && e->action) ? e->action : "(null)", e ? e->n_child : -1,
            hits, wait_state_hits);
    CHECK(hits > 0,
          "B8: 「このターン中に…アクティブにしていた」 -- the history tag must "
          "survive the C decode on the effect, on a child, or inside a nested "
          "condition.  The GameState field this section's premise rides on now "
          "EXISTS (include/rabuka.h:1578-1579) and has a C reader "
          "(src/ability/condition.c:543-571), so if the tag were not in the "
          "decoded tree the clause would be silently unevaluable rather than "
          "merely unreachable");
    CHECK_EQ(wait_state_hits, 0,
             "B8: and no step may still demand the member be in wait "
             "(card_property/state \"wait\")");
    rb_free_ability(&ab);
}

/* THE CORRECTION, AS ITS OWN CASE.
 *
 * The retired Active-phase driver in `b_modifier_with` asserted that switching
 * a member out of wait on the Active phase is an "activation" for
 * 「カードの効果によって」.  It is not, on EITHER engine, and this case exists so
 * the fact is pinned rather than merely argued in a comment: if a future change
 * "helpfully" makes `activate_wait_members` push into the history, this goes red
 * instead of the section quietly re-asserting C behaviour Rust does not have.
 *
 *   RUST, MEASURED over engine/src:
 *     - turn/phases.rs: the whole Active arm, whose member activation is
 *       `game_state.mods.add_orientation_modifier(cid, "active")` at :160-162,
 *       contains ZERO references to `cards_activated_this_turn`;
 *     - the field's only writer is ability/effects/change_state.rs:657-658,
 *       gated on `state_change == "active"` -- i.e. a CARD'S EFFECT, which is
 *       what 「カードの効果によって」 requires;
 *     - its only reader is ability/condition/card.rs:2486.
 *   C: src/turn/phase.c:224 activate_wait_members sets the orientation and
 *   writes no history entry; the writer is src/ability/effects/state.c:546-549.
 *
 * The orientation IS asserted to have flipped, so a red here cannot be explained
 * by the Active phase having failed to run. */
static void b9_active_phase_is_not_an_activation_history_writer(void)
{
    static TestGame tg;
    /* `fresh()` is this file's reset idiom, but it reaches test_game_new through
     * a SIBLING function, which test_harness_self's section N deliberately
     * COUNTS as uninitialised (its N3c case: "the only initialiser is in a
     * SIBLING function the body merely CALLS ... is COUNTED").  Writing the
     * three calls out here puts the initialiser in THIS body, which is the N3b
     * shape -- not counted, and byte-for-byte what fresh() does.  Doing it this
     * way keeps this new case from moving harness_self's N1 ratchet, which the
     * brief forbids raising. */
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    rb_look_reset_all();
    int waiting = mid(&tg, PRINT_1);
    if (!pin_id(waiting, PRINT_1)) return;
    int filler = mid(&tg, FILLER);
    if (filler < 0) return;

    fill_decks(&tg, filler, 40);
    clear_stage(&tg, 0);
    clear_stage(&tg, 1);
    set_stage(&tg, 0, AREA_LEFT, waiting);
    tg.state.p[0].stage_wait[AREA_LEFT] = 1;
    rb_mods_set_orientation(&tg.state.mods, waiting, "wait");

    CHECK_EQ(tg.state.n_cards_activated_this_turn, 0,
             "B9: premise -- the per-turn activation history is EMPTY before the "
             "phase runs (RUST: cards_activated_this_turn, game_state/mod.rs:165, "
             "cleared each turn at modifiers.rs:2236)");

    int reached = 0;
    for (int p = 0; p < 80; p++) {
        if (tg.state.phase == RB_PHASE_ACTIVE && tg.state.active == 0) {
            reached = 1;
            break;
        }
        if (pending(&tg)) { drain_optional(&tg, 1, 6); continue; }
        test_pass(&tg);
    }
    CHECK(reached,
          "B9: the Active phase was really reached (phase=%d active=%d turn=%d) "
          "-- without it the negative below proves nothing",
          tg.state.phase, tg.state.active, tg.state.turn);
    if (!reached) return;
    drain_optional(&tg, 1, 6);
    test_pass(&tg);              /* Active -> Energy, activating every waited member */

    /* The phase demonstrably ran: the member really left wait. */
    CHECK(!is_wait(&tg, waiting),
          "B9: control -- the member really was switched out of wait by the "
          "Active phase (orientation %s)",
          rb_mods_get_orientation(&tg.state.mods, waiting)
              ? rb_mods_get_orientation(&tg.state.mods, waiting) : "(none)");

    CHECK_EQ(tg.state.n_cards_activated_this_turn, 0,
             "B9: and the Active phase still wrote NOTHING into "
             "cards_activated_this_turn.  RUST: turn/phases.rs:160-162 sets the "
             "orientation only, and the file holds no reference to the history; "
             "the sole writer is ability/effects/change_state.rs:657-658, which "
             "is gated on a card effect's `state_change == \"active\"`.  A "
             "non-zero count here means the C has adopted a writer Rust does not "
             "have, and every §B reduction above would then be satisfying the "
             "wrong premise.");
}

/* ═════════════════════════════════════════════════════════════════════════
 * C. effects/cost/play_cost -- PL!-pb2-012-P+ 南ことり (7 #[test]).
 *
 * 常時: 「このカードをプレイする際、自分のステージにいる名前の異なる
 * 『Printemps』のメンバーを2人ウェイトにしてもよい。そうしたとき、このカードの
 * コストは２減る。」  The reduction is OPTIONAL and conditional on waiting TWO
 * DIFFERENTLY-NAMED Printemps members, so the two axes are pinned separately:
 * the count, and the distinctness.  `count: 2` + `distinct: card_name` are
 * both load-bearing -- the same member twice must not satisfy it.
 *
 * The reduction is a CONSTANT, so the C twin of `state.recalculate_constants()`
 * is test_recalc() + rb_mods_get_cost().  She must be IN HAND: the constant
 * cost scan only considers stage and hand card ids, so a card in neither zone
 * is never evaluated and silently reports 0.
 * ═════════════════════════════════════════════════════════════════════════ */

static int c_cost_mod(TestGame *tg, const char *const *own, int n_own)
{
    int kotori = mid(tg, KOTORI_P);
    if (!pin_id(kotori, KOTORI_P)) return -999;
    if (printed_cost(kotori) != 13) {
        CHECK(0, "C: %s prints cost 13 (got %d)", KOTORI_P, printed_cost(kotori));
    }
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    for (int i = 0; i < n_own; i++) {
        int cid = mid(tg, own[i]);
        if (cid < 0) return -999;
        set_stage(tg, 0, i, cid);
    }
    hand_append(tg, 0, kotori);
    CHECK(hand_has(tg, 0, kotori),
          "C: premise -- 南ことり is in HAND, because the constant cost scan "
          "only considers stage and hand card ids");
    /* The census baseline is taken HERE, after every fixture card is placed and
     * immediately before the scan under test.  Taking it before the fixture
     * would make the delta a fixture artefact rather than a destruction. */
    int census0 = test_total_card_count(tg);
    test_recalc(tg);
    int m = rb_mods_get_cost((RbMods *)&tg->state.mods, kotori);
    int census1 = test_total_card_count(tg);
    fprintf(stderr, "        [info] C own=%d cost_mod=%d CENSUS %d->%d\n",
            n_own, m, census0, census1);
    /* Nothing is a MOVE here, so the census must be identical; a drop would
     * mean rb_recalc_constants DESTROYED a card. */
    CHECK_EQ(census1, census0, "C: CENSUS unchanged by the constant-cost scan");
    return m;
}

/* Rust printed_cost_is_thirteen. */
static void c1_printed_cost_is_thirteen(void)
{
    static TestGame tg;
    fresh(&tg);
    int kotori = mid(&tg, KOTORI_P);
    if (!pin_id(kotori, KOTORI_P)) return;
    CHECK_EQ(printed_cost(kotori), 13, "C1: 南ことり(P) prints cost 13");
    if (!expect_trigger(kotori, RB_TSTR_CONSTANT, KOTORI_P)) return;
    tg.state.p[0].hand.n = 0;
    hand_append(&tg, 0, kotori);
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_cost((RbMods *)&tg.state.mods, kotori), 0,
             "C1: precondition -- with an empty stage no reduction applies");
}

/* Rust no_staged_members_means_no_reduction. */
static void c2_no_staged_members_means_no_reduction(void)
{
    static TestGame tg;
    fresh(&tg);
    static const char *const own[] = { NULL };
    (void)own;
    int m = c_cost_mod(&tg, NULL, 0);
    CHECK_EQ(m, 0, "C2: 「自分のステージにいる…2人」 -- an empty stage cannot "
                   "satisfy the wait");
}

/* Rust one_printemps_member_alone_is_not_enough. */
static void c3_one_printemps_member_alone_is_not_enough(void)
{
    static TestGame tg;
    fresh(&tg);
    /* Fixture pin: the single staged ally really is a Printemps member, so the
     * negative is about the COUNT and not about a failed group filter. */
    {
        int a = mid(&tg, PRINT_1);
        if (!pin_id(a, PRINT_1)) return;
        CHECK(in_group(a, "Printemps"),
              "C3: the staged ally IS a 「Printemps」 member (unit=\"%s\" group=\"%s\") -- so "
              "the 0 below can only be the count", rb_card_unit_name(a), rb_card_group_name(a));
    }
    static const char *const own[] = { PRINT_1 };
    int m = c_cost_mod(&tg, own, 1);
    CHECK_EQ(m, 0, "C3: 2人 -- a single staged Printemps member does not meet "
                   "the count");
}

/* Rust two_distinctly_named_printemps_members_grant_minus_two. */
static void c4_two_distinctly_named_printemps_members_grant_minus_two(void)
{
    static TestGame tg;
    fresh(&tg);
    {
        int a = mid(&tg, PRINT_1);
        int b = mid(&tg, PRINT_2);
        if (!pin_id(a, PRINT_1) || !pin_id(b, PRINT_2)) return;
        CHECK(in_group(a, "Printemps"),
              "C4: ally A is Printemps (unit=\"%s\")", rb_card_unit_name(a));
        CHECK(in_group(b, "Printemps"),
              "C4: ally B is Printemps (unit=\"%s\")", rb_card_unit_name(b));
        /* 名前の異なる -- the two must be DIFFERENT characters, and they are
         * two DISTINCT pool slots, so the test is not trivially true. */
        CHECK(a != b, "C4: the two allies are two distinct card instances");
        CHECK(strcmp(rb_card_name_of(a), rb_card_name_of(b)) != 0,
              "C4: and they really carry DIFFERENT printed names (\"%s\" vs "
              "\"%s\") -- the condition is distinct: card_name",
              rb_card_name_of(a), rb_card_name_of(b));
    }
    static const char *const own[] = { PRINT_1, PRINT_2 };
    int m = c_cost_mod(&tg, own, 2);
    CHECK_EQ(m, -2,
             "C4: 名前の異なる『Printemps』のメンバーを2人ウェイトにしてもよい -- "
             "the optional reduction is -2, not -1 and not +2");
}

/* Rust two_prints_of_the_same_character_are_not_distinct. */
static void c5_two_prints_of_the_same_character_are_not_distinct(void)
{
    static TestGame tg;
    fresh(&tg);
    {
        int a = mid(&tg, PRINT_1);
        int b = mid(&tg, PRINT_1);   /* a SECOND instance of the SAME print */
        if (!pin_id(a, PRINT_1) || !pin_id(b, PRINT_1)) return;
        CHECK(a != b,
              "C5: two pool slots of the same print (they must be distinct "
              "instances, else the board is one card, not two)");
        CHECK(strcmp(rb_card_name_of(a), rb_card_name_of(b)) == 0,
              "C5: and they carry the SAME printed name (\"%s\") -- the "
              "negative is about the NAME, not the instance",
              rb_card_name_of(a));
    }
    static const char *const own[] = { PRINT_1, PRINT_1 };
    int m = c_cost_mod(&tg, own, 2);
    CHECK_EQ(m, 0,
             "C5: 名前の異なる -- two copies of one character are not two distinct "
             "names, so the -2 must not apply");
}

/* Rust a_non_printemps_ally_does_not_count. */
static void c6_a_non_printemps_ally_does_not_count(void)
{
    static TestGame tg;
    fresh(&tg);
    {
        int a = mid(&tg, PRINT_1);
        int n = mid(&tg, NON_PRINT);
        if (!pin_id(a, PRINT_1) || !pin_id(n, NON_PRINT)) return;
        CHECK(in_group(a, "Printemps"),
              "C6: the first ally IS Printemps (unit=\"%s\")", rb_card_unit_name(a));
        /* The precondition the Rust asserts as a literal unit string. */
        CHECK(strcmp(rb_card_unit_name(n), "AZALEA") == 0,
              "C6: precondition -- the second ally's unit is exactly \"AZALEA\" "
              "(got \"%s\")", rb_card_unit_name(n));
        CHECK(!in_group(n, "Printemps"),
              "C6: and it is therefore OUTSIDE the 「Printemps」 group (got "
              "unit=\"%s\")", rb_card_unit_name(n));
    }
    static const char *const own[] = { PRINT_1, NON_PRINT };
    int m = c_cost_mod(&tg, own, 2);
    CHECK_EQ(m, 0, "C6: 「『Printemps』のメンバー」 -- an AZALEA member does not "
                   "count toward the 2人");
}

/* Rust three_printemps_members_still_grant_only_minus_two. */
static void c7_three_printemps_members_still_grant_only_minus_two(void)
{
    static TestGame tg;
    fresh(&tg);
    /* PAIRED POSITIVE CONTROL: c4_ is the same board with two members and must
     * reach -2.  c7_ is only meaningful if -2 is reachable at all, so the
     * evidence is that c4_ is on the same board with one member REMOVED. */
    static const char *const own[] = { PRINT_1, PRINT_2, FILLER };
    int m = c_cost_mod(&tg, own, 3);
    CHECK_EQ(m, -2,
             "C7: このカードのコストは2減る -- a flat -2, not scaled by how many "
             "members are available to wait");
}

/* ── main ───────────────────────────────────────────────────────────────── */

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    tally_init();
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS, on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 2;
    }
    hcolor_init();
    print_wire_map();

    printf("--- A1 hs_pb1_001_jidou_activated_energy_funds_live_start_payment ---\n");
    run("a1_jidou_energy_is_exactly_what_live_start_pays",
        a1_jidou_energy_is_exactly_what_live_start_pays);
    run("a2_declining_jidou_leaves_live_start_unable_to_pay",
        a2_declining_jidou_leaves_live_start_unable_to_pay);

    printf("--- A3 hs_pb1_003_turn2_second_discard_event_grants_again ---\n");
    run("a3_two_separate_discard_events_in_one_turn_grant_twice",
        a3_two_separate_discard_events_in_one_turn_grant_twice);
    run("a4_batch_of_three_is_one_event_and_keeps_the_second_allowance",
        a4_batch_of_three_is_one_event_and_keeps_the_second_allowance);
    run("a5_turn2_refuses_a_third_event_in_one_turn",
        a5_turn2_refuses_a_third_event_in_one_turn);

    printf("--- A6 hs_pb1_009_jidou_blades_charge_live_start_threshold ---\n");
    run("a6_one_appearance_grants_blade_to_host_below_threshold",
        a6_one_appearance_grants_blade_to_host_below_threshold);
    run("a7_printed_blade_alone_does_not_fire_live_start",
        a7_printed_blade_alone_does_not_fire_live_start);
    run("a8_two_appearances_reach_the_live_start_blade_threshold",
        a8_two_appearances_reach_the_live_start_blade_threshold);
    run("a9_blades_land_on_the_host_so_the_condition_reads_them",
        a9_blades_land_on_the_host_so_the_condition_reads_them);
    run("a10_turn_two_reopens_the_allowance_so_two_more_grant_plus_four",
        a10_turn_two_reopens_the_allowance_so_two_more_grant_plus_four);

    printf("--- A11 rurino_stage_to_waitroom_watcher_baseline ---\n");
    run("a11_rurino_stage_to_waitroom_draws_exactly_two",
        a11_rurino_stage_to_waitroom_draws_exactly_two);

    printf("--- A12 sp_bp5_005_mill_arms_jidou_recover ---\n");
    run("a12_mill_arms_jidou_recover_and_its_blade_counts_the_same_batch",
        a12_mill_arms_jidou_recover_and_its_blade_counts_the_same_batch);
    run("a13_declining_jidou_recover_keeps_the_mills_blade_but_recovers_nothing",
        a13_declining_jidou_recover_keeps_the_mills_blade_but_recovers_nothing);

    printf("--- A14 yell_real_yell_no_blade_heart_grants_heart ---\n");
    run("a14_wien_real_yell_with_no_blade_heart_grants_heart03",
        a14_wien_real_yell_with_no_blade_heart_grants_heart03);
    run("a15_sumire_real_yell_with_no_blade_heart_grants_heart06",
        a15_sumire_real_yell_with_no_blade_heart_grants_heart06);
    run("a16_no_yell_no_heart_for_both_watchers", a16_no_yell_no_heart_for_both_watchers);

    printf("--- B effects/modify_need_heart PL!-pb2-040-L ---\n");
    run("b1_no_activated_member_reduces_nothing", b1_no_activated_member_reduces_nothing);
    run("b2_one_activated_member_reduces_need_by_three",
        b2_one_activated_member_reduces_need_by_three);
    run("b3_two_activated_members_reduce_need_by_five",
        b3_two_activated_members_reduce_need_by_five);
    run("b4_three_activated_members_reduce_need_by_six",
        b4_three_activated_members_reduce_need_by_six);
    run("b5_staged_member_that_was_never_activated_reduces_nothing",
        b5_staged_member_that_was_never_activated_reduces_nothing);
    run("b6_staged_member_that_was_activated_and_stayed_counts_once",
        b6_staged_member_that_was_activated_and_stayed_counts_once);
    run("b7_waited_again_after_being_activated_still_counts",
        b7_waited_again_after_being_activated_still_counts);
    run("b8_the_clause_parses_as_a_history_not_a_state",
        b8_the_clause_parses_as_a_history_not_a_state);
    run("b9_active_phase_is_not_an_activation_history_writer",
        b9_active_phase_is_not_an_activation_history_writer);

    printf("--- C effects/cost/play_cost PL!-pb2-012-P+ ---\n");
    run("c1_printed_cost_is_thirteen", c1_printed_cost_is_thirteen);
    run("c2_no_staged_members_means_no_reduction", c2_no_staged_members_means_no_reduction);
    run("c3_one_printemps_member_alone_is_not_enough",
        c3_one_printemps_member_alone_is_not_enough);
    run("c4_two_distinctly_named_printemps_members_grant_minus_two",
        c4_two_distinctly_named_printemps_members_grant_minus_two);
    run("c5_two_prints_of_the_same_character_are_not_distinct",
        c5_two_prints_of_the_same_character_are_not_distinct);
    run("c6_a_non_printemps_ally_does_not_count", c6_a_non_printemps_ally_does_not_count);
    run("c7_three_printemps_members_still_grant_only_minus_two",
        c7_three_printemps_members_still_grant_only_minus_two);

    rb_unload();

    int n_run = n_ok + n_failed + n_crashed + n_setup;
    printf("\n==== parity_wave1b: %d test case(s) run ====\n", n_run);
    printf("     %d ok, %d FAILED, %d CRASHED, %d SETUPBUG\n",
           n_ok, n_failed, n_crashed, n_setup);
    printf("     %d assertion(s), %d failure(s), %d setup bug(s), %d prompt(s)\n",
           tal->assertions, tal->failures, tal->setup_bugs, tal->prompts);
    if (n_crashed)
        fprintf(stderr, "%d test case(s) CRASHED -- a fatal signal in a child. "
                        "Treat as an engine fault.\n", n_crashed);
    if (n_setup)
        fprintf(stderr, "%d test case(s) hit a SETUP BUG: a fixture could not be "
                        "built, so its claims were never evaluated.\n", n_setup);
    if (n_failed)
        fprintf(stderr, "%d test case(s) FAILED. Every red assertion in this file "
                        "is classified in the accompanying report; none is a "
                        "weakened or skipped assertion.\n", n_failed);
    return n_failed ? 1 : 0;
}
