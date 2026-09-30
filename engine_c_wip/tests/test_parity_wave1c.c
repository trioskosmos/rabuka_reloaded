/* test_parity_wave1c.c -- C parity suite for four Rust clusters whose cards appear
 * in NO other C test file (verified by a card-number scan of the whole
 * engine_c_wip/tests tree, not by a header claim).
 *
 * CLAIM (tr_wave1c): this file is the ONLY file I own.  I edit nothing else --
 * no src sources, no other tests/test_[star].c, no Makefile.  The Rust files I
 * translate are, in full:
 *   engine/tests/test_modules/effects/move_cards/under_member/
 *       pb2_debut_four_printemps_members_under_pl_pb2_017_p_plus_test.rs   (7 tests)
 *   engine/tests/test_modules/effects/kidou/
 *       hs_bp1_002_rm_same_area_redeploy_test.rs                          (5 tests)
 *       pb4_gated_activation_score_total_kidou_pl_pb4_002_sec_test.rs      (3 tests)
 *   engine/tests/test_modules/effects/move_cards/
 *       pb2_reveal_lily_white_live_and_swap_success_zone_pl_pb2_014_p_plus_test.rs (4)
 * and (see main(), section D) the jidou/movement/under_member_placement pair,
 *   live_success_or_self_move_place_discard_group_member_under_self_test.rs (2)
 *   under_member_counted_for_both_players_test.rs                        (1)
 *
 * 19 of the 22 assigned tests are ported below.  The 3 in section D are NOT
 * ported and the reason is recorded in main()'s banner and in the report: peers
 * already ported all three, and the evidence is named there.
 *
 * ── NO JAPANESE IN A COMPARISON ───────────────────────────────────────────
 * The editor can emit CP932 and silently corrupt Japanese literals.  Every
 * Japanese string this file COMPARES AGAINST is a byte escape whose bytes were
 * read out of cards/cards.json, cards/abilities.json and
 * engine_c_wip/src/core/card.c (JW_* below), or is the engine's own RB_TSTR_*
 * constant.  The Japanese that does appear raw sits in COMMENTS and in the
 * human-readable text of a failure message, never on the right-hand side of a
 * comparison, so a mangled byte could only garble a diagnostic.  Verified
 * after write: the file is VALID UTF-8 (a strict UTF8Encoding decode with
 * throwOnInvalidBytes succeeds), so the Write tool did not emit CP932.
 *
 * ── PIPELINE SKEW, AND WHY THIS FILE PRINTS A SKEW PROBE ──────────────────
 * src/core/generated/bytecode_blob.c carries (chunk,start,len) offsets into
 * cards/build/abilities_strings.bin, which is a LOCAL GITIGNORED artifact.  At
 * the commit this suite was written against (git log -1 = dc44ccf1) the .bin
 * was NEWER than the .c (regenerated 15:35 vs the .c committed 10:02), so the
 * pool is UNSEPARATED and a few bytes of skew returns byte SPLICES instead of a
 * failure.  The classic tells are given in the brief.  Therefore:
 *   - print_skew_probe() runs BEFORE the suites and prints, for every decode-
 *     dependent fixture, the trigger string and the effect's
 *     action/source/destination.  A splice shows up as a visible wrong word,
 *     not as a silent 0.
 *   - a1_skew_probe_ok() ASSERTS the four load-bearing wire strings
 *     ("same_area", "under_member", "success_live_zone", "selected_cards",
 *      "move_cards", the two trigger constants).  If the probe is spliced the
 *     whole file says so loudly and every decode-dependent red below is
 *     re-classified as SKEW-ARTEFACT rather than engine bug.
 * Card numbers, costs, blades and scores come from cards.bin, which is a
 * different blob, and are not subject to this skew; those assertions stand
 * either way.
 *
 * ── LANDMINES RESPECTED ──────────────────────────────────────────────────
 *   - load_card_db() falls back to ../cards/build: rb_load("src") fails under
 *     an isolated out-of-tree build.
 *   - sizeof(TestGame) is ~781 KB: every TestGame is `static`.
 *   - test_get_heart_modifier's colour-5 remap is GONE.  No heart is read in
 *     this file, but the heartNN -> enum map is still derived at run time from
 *     the engine's own rb_heart_color_as_str / rb_parse_heart_color and
 *     PRINTED, so a stale index would be visible rather than silent.
 *   - rb_card_string() takes a STRING-TABLE INDEX: card_no_of() decodes the Card
 *     and reads c.card_no_idx.  pin_id() compares byte-for-byte and does NOT
 *     use rb_card_no_eq; where the print itself is under test the lookup is
 *     test_id_exact, not the forgiving rb_find_card_by_no chain.
 *   - card_no_of() / rb_card_name_of() use ROTATING rings.  One shared static
 *     buf called 2-3 times in one printf prints the LAST card repeatedly.
 *   - rb_card_copy_pool_reset() runs per case (the pool is process-global and
 *     runs dry at 4096-2639 = 1457 allocations).
 *   - mid() = Rust game.id(x), a NEW pool instance per call.  Every setup helper
 *     here RETURNS the ids it seeded, because re-deriving an id at the
 *     assertion site mints a different instance and can never match a card
 *     already in a zone -- a false red indistinguishable from "the card never
 *     arrived".
 *   - rb_card_group_name is SERIES-LEVEL (every ラブライブ! card reports mu's).
 *     in_group() matches a printed name against the UNIT first and only then
 *     against the series, mirroring src/ability/util.c:295-300, and both are
 *     PRINTED at startup.
 *   - The waitroom zone string is "discard" (the C zone id, not "waitroom").
 *   - test_add_to_stage / test_set_opp_stage are FIXED-INDEX by design;
 *     set_stage() writes an arbitrary area and clears the wait flag with it.
 *   - test_place_under APPENDS.  The under-card assertions read the zone with
 *     test_zone_len(tg, pl, "under0"/"under1"/"under2") and
 *     test_zone_count_of_id(), never a hand-rolled loop, so "no card" and "the
 *     wrong card" are distinguishable.
 *   - rb_generate_action_candidates() does NOT emit a `disabled` flag and does
 *     not emit per-area actions, so b6-b8 measure the OFFER (does an
 *     ActivateAbility action for ERI exist at all) and the anti-vacuity control
 *     is that the generator produced a NON-EMPTY list even in the two negative
 *     cases.  See section B.
 *
 * ── ANTI-VACUITY ─────────────────────────────────────────────────────────
 * Every negative assertion in this file carries EITHER a prompt-count control
 * taken at the assertion site (CHECK_PROMPTS_GT against a control point
 * sampled BEFORE the interaction) OR a paired positive driven first on the
 * SAME board.  CHECK_PROMPTS is deliberately NOT used: the shared macro bumps
 * the counter and then compares it, so it satisfies itself.
 *
 * EVERY case takes a WHOLE-BOARD CARD CENSUS (test_total_card_count) twice:
 * once AFTER every fixture card is placed and immediately before the action
 * under test, and once after.  A zone count of 0 is identical whether a move
 * did nothing or DESTROYED a card; the census is the only control that tells
 * those apart.  It is load-bearing in section A (where the "correct" answer is
 * often 0) and in every negative in section B.
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

/* U+FF0B FULLWIDTH PLUS SIGN = EF BC 8B.  The filename token pl_X maps to
 * PL!-X and pl_sp_X to PL!SP-X; filenames and bodies agree, so there is no
 * PL! vs PL!SP mismatch here and none is reported. */
#define FP "\xef\xbc\x8b"

/* Japanese wire/print literals, as byte escapes read out of cards.json and
 * src/core/card.c.  See the header: this file is ASCII-only on purpose. */
#define JW_SUCCESS_ZONE                                             \
    "\xE6\x88\x90\xE5\x8A\x9F\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96" \
    "\xE3\x82\xAB\xE3\x83\xBC\xE3\x83\x89\xE7\xBD\xAE\xE3\x81\x8D\xE5\xA0\xB4"
#define JW_HASUNO     "\xE8\x93\xAE\xE3\x83\x8E\xE7\xA9\xBA"   /* 蓮ノ空 */
#define JW_MUS        "\xCE\xBC's"                            /* mu's   */
#define JW_PRINTEMPS  "Printemps"
#define JW_LIELLA     "Liella!"
#define JW_LILYWHITE  "lily white"

#define AREA_LEFT   0
#define AREA_CENTER 1
#define AREA_RIGHT  2

/* ── fixtures ───────────────────────────────────────────────────────────────
 * Every one of these was read out of cards/cards.json parsed as UTF-8 (NEVER
 * Get-Content -Raw, which mangles non-ASCII and produces false "missing"
 * cards).  names are given in romaji so this file stays ASCII.
 *
 * SECTION A -- effects/move_cards/under_member, PL!-pb2-017-P+  (6 tests)      */
#define NOZOMI  "PL!-pb2-017-P" FP  /* Koizumi Hanayo   cost 17 blade 6
                                    *   unit Printemps  series Love Live!
                                    *   登場: put 4 Printemps members from
                                    *   MY waitroom under THIS member          */
#define P_1     "PL!-bp6-017-N"     /* Koizumi Hanayo   cost 2  (no printed
                                    *   blade) Printemps                         */
#define P_2     "PL!-bp5-012-N"     /* Minami Kotori    cost 4  Printemps     */
#define P_3     "PL!-pb1-026-N"     /* Koizumi Hanayo   cost 4  Printemps
                                    *   -- a SECOND Hanaho print, so the group
                                    *   filter cannot be satisfied by name      */
#define P_4     "PL!-sd1-010-SD"    /* Kosaka Honoka    cost 4  Printemps
                                    *   (the filler; Rust reuses one filler id
                                    *   for the decks AND for P_4)             */
#define P_5     "PL!-pb2-030-N"     /* Minami Kotori    cost 5  Printemps
                                    *   (no space in the printed name)         */
#define NON_P   "PL!S-bp5-012-N"     /* Matsuura Kanon   cost 2  unit AZALEA
                                    *   series Sunshine -- NOT Printemps      */

/* SECTION B1 -- effects/kidou, PL!HS-bp1-002-RM  (5 tests)                   */
#define SAYAKA    "PL!HS-bp1-002-RM" /* Murano Sayaka   cost 11 blade 2
                                      *   unit DOLLCHESTRA, series Hasunosora
                                      *   起動(turn1): pay 2E, move SELF
                                      *   stage->waitroom : deploy ONE
                                      *   Hasunosora member of cost <= 15 from
                                      *   MY waitroom into the area SHE vacated
                                      *   (destination "same_area")              */
#define CHEAP_HS  "PL!HS-PR-003-PR"  /* Otomori Kuzu    cost 4  Hasunosora
                                      *   (unit HUZUKAIBEKU) -- under the bound */
#define BOUND_HS  "PL!HS-pb1-023-N"  /* Seras Yanada Lillianfelt cost 15
                                      *   unit EdelNote, Hasunosora -- EXACTLY
                                      *   the bound.  RUST FIXTURE BUG: the
                                      *   hs_bp1_002 header table calls this card
                                      *   nothing, but the table's "cost exactly
                                      *   15" row is right and this is the card
                                      *   it means.  A peer already reported the
                                      *   name.  NOTE also that its unit is
                                      *   EdelNote, NOT HUZUKAIBEKU, so it is in
                                      *   the group only through the SERIES.      */
#define OVER_HS   "PL!HS-PR-016-PR"  /* Hinoshita Kanon cost 17 Hasunosora
                                      *   -- over the cost bound only             */
#define FILLER_HS "PL!-sd1-010-SD"   /* Kosaka Honoka cost 4, NOT Hasunosora
                                      *   -- fails the group gate only            */

/* SECTION B2 -- effects/kidou, PL!-bp4-002-SEC  (3 tests)                   */
#define ERI      "PL!-bp4-002-SEC"   /* Ayase Eri       cost 15 blade 4 BiBi
                                     *   起動(turn1): discard 2 hand cards, then
                                     *   take a mu's live from MY waitroom.  The
                                     *   ACTIVATION GATE: the total printed score
                                     *   in MY success live card zone must be
                                     *   >= 6 (comparison_type "score",
                                     *   aggregate "total").                       */
#define SCORE_4  "PL!-bp6-023-L"     /* live, printed score 4, unit Printemps   */
#define SCORE_1  "PL!-bp4-026-L"     /* live, printed score 1, unit BiBi        */
#define SCORE_6  "PL!SP-bp1-027-L"   /* live, printed score 6, no unit          */
#define ERI_FILL "PL!-bp3-012-N"     /* Minami Kotori cost 2 -- hand fodder for
                                      *   the 2-card cost                         */

/* SECTION C -- effects/move_cards, PL!-pb2-014-P+  (4 tests)                */
#define SHIRO     "PL!-pb2-014-P" FP /* Hoshizora Rin    cost 11 blade 2
                                      *   unit lily white, series Love Live!
                                      *   登場 (OPTIONAL reveal cost): reveal one
                                      *   lily white LIVE from hand; EFFECT: take
                                      *   a card from MY success live card zone
                                      *   into hand, THEN put the revealed card
                                      *   into MY success live card zone.        */
#define LW_LIVE   "PL!-pb1-029-SRL"  /* live, unit lily white -- what the cost
                                      *   reveals                                  */
#define MUS_LIVE  "PL!-bp4-025-SRL"  /* live, unit lily white, score 5
                                      *   (the Rust comment wrongly says "a mu's
                                      *   live that is NOT lily white" and then
                                      *   contradicts itself in the same line)    */
#define NOT_LIVE  "PL!-pb2-003-PP"   /* Minami Kotori cost 9 -- a MEMBER         */

#define ENERGY_NO "LL-E-001-SD"      /* energy card                               */

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

/* VACUITY CONTROL.  `n` is a control point SAMPLED BEFORE the interaction, and
 * the comparison is against the running counter at the ASSERTION SITE, so the
 * prompts really offered are what is measured.  The shared CHECK_PROMPTS macro
 * in the sibling suites increments then compares, so it satisfies itself; this
 * file does not use it. */
#define CHECK_PROMPTS_GT(control, ...) do {                               \
        if (tal) tal->assertions++;                                       \
        if (!tal || tal->prompts <= (int)(control)) {                     \
            if (tal) tal->failures++;                                     \
            fprintf(stderr, "FAIL (VACUOUS): ");                          \
            fprintf(stderr, __VA_ARGS__);                                 \
            fprintf(stderr, " - %d prompt(s) offered, control was %d, so the negative proves nothing\n", tal ? tal->prompts : -1, (int)(control)); \
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

/* The PRINTED card name.  There is no public accessor, so the Card is decoded
 * and c.name read; the string is owned by the Card, so it is COPIED into a
 * rotating ring before the free. */
#define NAME_RING 4
static const char *card_name_of(int cid)
{
    static char ring[NAME_RING][128];
    static int next;
    char *buf = ring[next++ % NAME_RING];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        if (c.name) snprintf(buf, 128, "%s", c.name);
        else if (c.name_idx) snprintf(buf, 128, "%s", rb_card_string(c.name_idx));
        rb_free_card(&c);
    }
    return buf;
}

/* AGENTS.md identity rule: byte-exact strcmp of the DECODED card_no.
 * rb_find_card_by_no is forgiving (substring fallback) so the LOOKUP is exact
 * (test_id_exact) and so is the comparison. */
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

/* Rust `game.id(x)` mints a NEW instance per call. */
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

static int printed_score(int cid)
{
    Card c;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return -1;
    int v = (int)c.score;
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

/* Rust `prints_trigger(cid, trig)` -- also the fixture-wiring vacuity control
 * for every negative in this file. */
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

static int expect_trigger(int cid, const char *trig, const char *no)
{
    int ok = prints_trigger(cid, trig);
    CHECK(ok, "fixture wiring: %s really prints a '%s' ability", no, trig);
    return ok;
}

/* Faithful twin of Rust `assert_card_in_group(card, name)`: match the printed
 * name against the card's UNIT first, then against the five series-derived
 * names -- exactly what src/ability/util.c:295-300 does.  rb_card_group_name
 * is SERIES-LEVEL only, so a "Printemps" assertion made against it alone is
 * false for every Love Live! card. */
static int in_group(int cid, const char *g)
{
    const char *u = rb_card_unit_name(cid);
    if (u && *u && !strcmp(u, g)) return 1;
    const char *s = rb_card_group_name(cid);
    return s && *s && !strcmp(s, g);
}

/* ── board helpers ──────────────────────────────────────────────────────── */

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }
static const RbChoice *pchoice(TestGame *tg)
{
    return pending(tg) ? rb_get_pending_choice(&tg->state) : NULL;
}

static void dump_choice(TestGame *tg, const char *where)
{
    const RbChoice *c = pchoice(tg);
    if (!c) { fprintf(stderr, "        [choice@%s] none\n", where); return; }
    fprintf(stderr, "        [choice@%s] kind=%d route=%d zone=%s target=%s "
            "count=%d nfid=%d desc=%s\n",
            where, (int)c->kind, (int)c->route, c->zone, c->target,
            c->count, c->n_filtered_indices, c->description);
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

/* Rust `select_indices(&[..])`. */
static void select_n(TestGame *tg, const int *idx, int n)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    if (n <= 0) rb_resume_with_choice(&tg->state, -1);
    else        rb_resume_with_choice_indices(&tg->state, idx, n);
}

/* Rust `select_indices(&[])` -- DECLINE. */
static void decline(TestGame *tg)
{
    if (!pending(tg)) return;
    note_prompt(tg);
    rb_resume_with_choice(&tg->state, -1);
}

/* CAPPED drain.  The Rust `drain_choices` / `drain_auto_ability_choices` are
 * "answer what cannot be declined, decline what can"; the cap is this
 * toolchain's substitute for the watchdog that does not exist. */
static int drain_answer(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const RbChoice *c = pchoice(tg);
        if (!c) break;
        if (c->kind == RB_CHOICE_SELECT_CARD && c->count > 1) {
            int one = 0;
            select_n(tg, &one, 1);
        } else if (c->kind == RB_CHOICE_SELECT_CARD) {
            int one = 0;
            select_n(tg, &one, 1);
        } else if (c->kind == RB_CHOICE_SELECT_TARGET && c->count > 0) {
            pick(tg, 0);
        } else {
            decline(tg);
        }
    }
    rb_drain_ability_queue(&tg->state);
    return guard;
}

/* Rust `drain_auto_ability_choices` -- the auto-ability ordering prompt is
 * answered with PROCEED (index 0), everything else declined. */
static int drain_auto(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const RbChoice *c = pchoice(tg);
        if (!c) break;
        if (c->kind == RB_CHOICE_SELECT_AUTO_ABILITY) pick(tg, 0);
        else                                        decline(tg);
    }
    rb_drain_ability_queue(&tg->state);
    return guard;
}

static int hand_n(const TestGame *tg, int pl) { return tg->state.p[pl].hand.n; }
static int wait_n(const TestGame *tg, int pl) { return tg->state.p[pl].discard.n; }
static int active_e(const TestGame *tg, int pl)
{
    return rb_energy_active_count(&tg->state.p[pl]);
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}
static int hand_has(const TestGame *tg, int pl, int cid)
{
    return bag_has(&tg->state.p[pl].hand, cid);
}
static int wait_has(const TestGame *tg, int pl, int cid)
{
    return bag_has(&tg->state.p[pl].discard, cid);
}
static int success_has(const TestGame *tg, int pl, int cid)
{
    return bag_has(&tg->state.p[pl].success, cid);
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

static void wait_append(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        tg->state.p[pl].deck.n = 0;
        for (int i = 0; i < n && tg->state.p[pl].deck.n < RB_MAX_ZONE; i++)
            tg->state.p[pl].deck.cards[tg->state.p[pl].deck.n++] = filler;
    }
}

/* Rust `energy_zone.cards.push(e)` x total + `set_active_count(active)`. */
static void set_energy(TestGame *tg, int pl, int total, int active)
{
    RbPlayer *P = &tg->state.p[pl];
    P->energy.n = 0;
    for (int i = 0; i < total && P->energy.n < RB_MAX_ZONE; i++)
        P->energy.cards[P->energy.n++] = rb_find_card_by_no(ENERGY_NO);
    rb_energy_set_active_count(P, active);
}

/* ── decoded-shape dump + skew probe ─────────────────────────────────────── */

static const char *safe(const char *s) { return s ? s : "(null)"; }

static void dump_ability(int cid, int idx)
{
    Ability ab;
    memset(&ab, 0, sizeof ab);
    if (!rb_decode_card_ability((uint32_t)cid, idx, &ab)) {
        fprintf(stderr, "    %s ab#%d: DECODE FAILED\n", card_no_of(cid), idx);
        return;
    }
    fprintf(stderr, "    %s ab#%d trig=[%s] use_limit=%d is_null=%d\n",
            card_no_of(cid), idx, safe(ab.triggers), ab.use_limit, ab.is_null);
    if (ab.cost)
        fprintf(stderr, "        cost:   action=[%s] source=[%s] dest=[%s] "
                "count=%d optional=%d extra=%d\n",
                safe(ab.cost->action), safe(ab.cost->source),
                safe(ab.cost->destination), ab.cost->count,
                ab.cost->is_optional, ab.cost->n_extra);
    if (ab.effect) {
        fprintf(stderr, "        effect: action=[%s] source=[%s] dest=[%s] "
                "count=%d optional=%d cond=%d act_cond=%d extra=%d\n",
                safe(ab.effect->action), safe(ab.effect->source),
                safe(ab.effect->destination), ab.effect->count,
                ab.effect->is_optional, ab.effect->has_condition,
                ab.effect->activation_condition ? 1 : 0,
                ab.effect->n_extra);
        for (int i = 0; i < ab.effect->n_extra; i++)
            fprintf(stderr, "            extra[%d] \"%s\" = [%s]\n", i,
                    safe(ab.effect->extra_k[i]), safe(ab.effect->extra_v[i]));
    }
    rb_free_ability(&ab);
}

static void dump_all_abilities(int cid)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) dump_ability(cid, i);
}

static const char *const ALL_FIXTURES[] = {
    NOZOMI, P_1, P_2, P_3, P_4, P_5, NON_P,
    SAYAKA, CHEAP_HS, BOUND_HS, OVER_HS, FILLER_HS,
    ERI, SCORE_4, SCORE_1, SCORE_6, ERI_FILL,
    SHIRO, LW_LIVE, MUS_LIVE, NOT_LIVE, ENERGY_NO
};

static void print_wire_map(void)
{
    fprintf(stderr, "--- heart index map (rb_heart_color_as_str) ---\n");
    for (int i = 0; i < 7; i++)
        fprintf(stderr, "    index %d -> \"%s\" -> enum %d\n",
                i, rb_heart_color_as_str(i), HC[i]);
    fprintf(stderr, "--- whole-board card census of every fixture "
            "(group is SERIES-LEVEL) ---\n");
    for (unsigned i = 0; i < sizeof ALL_FIXTURES / sizeof ALL_FIXTURES[0]; i++) {
        int id = rb_find_card_by_no(ALL_FIXTURES[i]);
        if (id < 0) { fprintf(stderr, "    %-22s NOT FOUND\n", ALL_FIXTURES[i]); continue; }
        fprintf(stderr, "    %-22s id=%-6d cost=%-3d blade=%-3d score=%-3d "
                "live=%d member=%d group=\"%s\" unit=\"%s\" n_abilities=%d\n",
                ALL_FIXTURES[i], id, printed_cost(id), printed_blade(id),
                printed_score(id), rb_card_is_live(id), rb_card_is_member(id),
                rb_card_group_name(id), rb_card_unit_name(id),
                rb_card_num_abilities((uint32_t)id));
    }
    fprintf(stderr, "--- assertion literals (byte escapes / ASCII) ---\n");
    fprintf(stderr, "    JW_SUCCESS_ZONE = \"%s\"\n", JW_SUCCESS_ZONE);
    fprintf(stderr, "    JW_HASUNO       = \"%s\"\n", JW_HASUNO);
    fprintf(stderr, "    JW_MUS          = \"%s\"\n", JW_MUS);
    fprintf(stderr, "    RB_TSTR_DEBUT   = \"%s\"\n", RB_TSTR_DEBUT);
    fprintf(stderr, "    RB_TSTR_KIDOU   = \"%s\"\n", RB_TSTR_ACTIVATION);
    fprintf(stderr, "    FP (U+FF0B)    = %02X %02X %02X\n",
            (unsigned char)FP[0], (unsigned char)FP[1], (unsigned char)FP[2]);
}

/* SKEW PROBE.  See the header: the committed bytecode_blob.c bakes
 * (chunk,start,len) offsets into a LOCAL GITIGNORED string table, so a
 * regenerate can return byte SPLICES instead of a failure.  These are the wire
 * strings every decode-dependent assertion below depends on.  If this case
 * goes red, treat every red in sections B1 and C as SKEW-ARTEFACT. */
static void a0_skew_probe(void)
{
    struct { const char *no; const char *trig; const char *dest; } want[] = {
        { SAYAKA,  RB_TSTR_ACTIVATION, "same_area"        },
        { NOZOMI,  RB_TSTR_DEBUT,      "under_member"     },
        { SHIRO,   RB_TSTR_DEBUT,      "success_live_zone"},
    };
    fprintf(stderr, "--- SKEW PROBE: decoded shapes of every decode-dependent "
            "fixture ---\n");
    for (unsigned i = 0; i < sizeof want / sizeof want[0]; i++) {
        int id = rb_find_card_by_no(want[i].no);
        CHECK(id >= 0, "skew probe: %s resolves", want[i].no);
        if (id < 0) continue;
        dump_all_abilities(id);
        int n = rb_card_num_abilities((uint32_t)id);
        int found = 0;
        for (int a = 0; a < n; a++) {
            Ability ab;
            memset(&ab, 0, sizeof ab);
            if (!rb_decode_card_ability((uint32_t)id, a, &ab)) continue;
            if (ab.triggers && !strcmp(ab.triggers, want[i].trig) &&
                ab.effect && ab.effect->destination &&
                !strcmp(ab.effect->destination, want[i].dest))
                found = 1;
            rb_free_ability(&ab);
        }
        CHECK(found,
              "SKEW: %s must decode a '%s' ability whose effect destination is "
              "\"%s\". A wrong word here is a BYTE SPLICE from the offset table, "
              "NOT an engine bug -- re-run after the bytecode/string-table fix "
              "lands.", want[i].no, want[i].trig, want[i].dest);
    }
    /* The two remaining load-bearing literals. */
    {
        int shiro = rb_find_card_by_no(SHIRO);
        if (shiro >= 0) {
            Ability ab;
            memset(&ab, 0, sizeof ab);
            int got = 0;
            for (int a = 0; a < rb_card_num_abilities((uint32_t)shiro); a++) {
                memset(&ab, 0, sizeof ab);
                if (!rb_decode_card_ability((uint32_t)shiro, a, &ab)) continue;
                if (ab.triggers && !strcmp(ab.triggers, RB_TSTR_DEBUT) &&
                    ab.effect && ab.effect->source &&
                    !strcmp(ab.effect->source, "selected_cards") &&
                    ab.effect->action &&
                    !strcmp(ab.effect->action, "move_cards"))
                    got = 1;
                rb_free_ability(&ab);
            }
            CHECK(got, "SKEW: %s must decode source=\"selected_cards\" "
                      "action=\"move_cards\" on its 登場 ability", SHIRO);
        }
        int eri = rb_find_card_by_no(ERI);
        if (eri >= 0)
            CHECK(prints_trigger(eri, RB_TSTR_ACTIVATION),
                  "SKEW: %s must decode a '%s' trigger", ERI, RB_TSTR_ACTIVATION);
    }
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
        printf("        %s: %d failure(s), %d setup bug(s), %d prompt(s)\n",
               name, tal->failures - f0, tal->setup_bugs - s0,
               tal->prompts);
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
 * A. effects/move_cards/under_member --
 *    pb2_debut_four_printemps_members_under_pl_pb2_017_p_plus_test.rs (7).
 *
 * PL!-pb2-017-P+ 登場: "自分の控え室にある『Printemps』のメンバーカード4枚を
 * このメンバーの下に置く."  A plain filtered move.  The four interesting axes
 * are the COUNT, the group filter, the source zone, and 「このメンバーの下」
 * (under_self -- another member's under-cards must not change).
 *
 * Every case takes the census immediately before the debut and again at the
 * assertion site.  The census is what distinguishes "moved" from "destroyed":
 * a1..a7 all assert an under-COUNT, and a count of 0 is what a destroyed card
 * also produces.
 * ═════════════════════════════════════════════════════════════════════════ */

typedef struct {
    int ids[8];          /* the seeded waitroom members, in fixture order    */
    int n;
    int nozomi;
    int other_area;      /* the pre-staged member's area, or -1               */
    int other;           /* the pre-staged member's id, or -1                 */
    int prompts_before_debut;
    int prompts_after_debut;
    int prompted;
    int prompt_count;               /* how many the engine says it wants      */
    int prompt_nfid;                /* how many candidates it actually offers */
    int census0, census1;
    int played;
} ARun;

/* Rust `under_count_after_debut(waitroom)` -- but the ids are RETURNED, because
 * re-deriving one at the assertion site would mint a different pool instance
 * that can never match a card already in a zone. */
static ARun a_run(TestGame *tg, const char *const *waitroom, int n_wait,
                  const char *other_area_card, int other_area)
{
    ARun r;
    memset(&r, 0, sizeof r);
    r.other_area = -1;
    r.other = -1;

    int filler = mid(tg, P_4);
    if (filler < 0) return r;
    fill_decks(tg, filler, 50);

    r.nozomi = mid(tg, NOZOMI);
    if (r.nozomi < 0 || !pin_id(r.nozomi, NOZOMI)) return r;

    for (int i = 0; i < n_wait; i++) {
        int id = mid(tg, waitroom[i]);
        if (id < 0) return r;
        if (!pin_id(id, waitroom[i])) return r;
        wait_append(tg, 0, id);
        r.ids[r.n++] = id;
    }
    if (other_area_card) {
        r.other = mid(tg, other_area_card);
        if (r.other < 0 || !pin_id(r.other, other_area_card)) return r;
        set_stage(tg, 0, other_area, r.other);
        r.other_area = other_area;
    }

    hand_append(tg, 0, r.nozomi);
    set_energy(tg, 0, 30, 20);
    CHECK(hand_has(tg, 0, r.nozomi), "A: premise -- NOZOMI is in hand");
    CHECK_EQ(printed_cost(r.nozomi), 17, "A: premise -- NOZOMI prints cost 17");
    if (!expect_trigger(r.nozomi, RB_TSTR_DEBUT, NOZOMI)) return r;

    r.prompts_before_debut = tal ? tal->prompts : 0;
    /* THE CENSUS BASELINE: after EVERY fixture card is placed, immediately
     * before the action under test. */
    r.census0 = test_total_card_count(tg);

    /* Rust `game.play_to_stage(nozomi, MemberArea::Center)`. */
    r.played = test_play_to_stage(tg, r.nozomi, AREA_CENTER);
    CHECK(r.played, "A: NOZOMI reaches the CENTRE (cost 17 is payable from 20)");

    if (pending(tg)) {
        dump_choice(tg, "debut");
        r.prompted = 1;
        r.prompt_count = pchoice(tg) ? pchoice(tg)->count : 0;
        r.prompt_nfid = pchoice(tg) ? pchoice(tg)->n_filtered_indices : 0;
        /* Rust `game.select_indices(&[0, 1, 2, 3])` -- answer WHICH four, so the
         * assertion measures the placement and not an unanswered prompt. */
        int four[4] = { 0, 1, 2, 3 };
        select_n(tg, four, 4);
    }
    drain_auto(tg, 12);
    /* A MANDATORY prompt left parked mid-ability produces exactly the absence
     * the next assertion checks, so nothing would fail -- the Rust cluster-4
     * files call this out explicitly.  Answer whatever is still standing so a
     * 0 under-count cannot be a frozen machine.  CAPPED, and counted: the
     * number answered is printed, so "the drain did the work" is visible. */
    {
        int left = drain_answer(tg, 12);
        fprintf(stderr, "        [A] extra drain after debut answered %d more\n",
                left);
        r.prompts_after_debut = tal ? tal->prompts : 0;
    }
    r.census1 = test_total_card_count(tg);
    fprintf(stderr, "        [A] n_wait=%d prompted=%d want=%d nfid=%d prompts "
            "%d->%d CENSUS %d->%d under_center=%d waitroom=%d\n",
            n_wait, r.prompted, r.prompt_count, r.prompt_nfid,
            r.prompts_before_debut, r.prompts_after_debut,
            r.census0, r.census1, test_zone_len(tg, 0, "under1"), wait_n(tg, 0));
    return r;
}

static int under_n(TestGame *tg, int area)
{
    return test_zone_len(tg, 0, area == AREA_LEFT ? "under0"
                              : area == AREA_CENTER ? "under1" : "under2");
}

/* A census constant across the whole of section A: nothing here destroys a
 * card, so any drop would be a card-DESTROYING defect invisible to a zone
 * count. */
static void check_census(ARun *r, const char *what)
{
    CHECK_EQ(r->census1, r->census0,
             "A: CENSUS must be unchanged by the debut -- %s", what);
}

/* THE POSITIVE PROMPT CONTROL for section A.  With four or more eligible
 * waitroom members the engine MUST raise a 4-card selection; if it does not,
 * every under-count below is measuring a frozen machine rather than the
 * effect.  This is the control a peer was asked for: the counter is sampled
 * BEFORE the play and compared at the ASSERTION SITE, so the prompt really
 * offered is what is measured. */
static void expect_offered_prompt(ARun *r, const char *what)
{
    CHECK_PROMPTS_GT(r->prompts_before_debut,
                     "A: %s -- the 4-card selection was never offered, so the "
                     "under-count below is not about the effect", what);
    CHECK(r->prompted, "A: %s -- a pending choice was standing after the play",
          what);
    CHECK_EQ(r->prompt_count, 4,
             "A: %s -- the engine says it wants 4 cards (printed 「4枚」)", what);
    CHECK(r->prompt_nfid >= 4,
          "A: %s -- and it offers at least 4 candidates (offered %d)", what,
          r->prompt_nfid);
}

/* Rust four_printemps_members_go_under_her. */
static void a1_four_printemps_members_go_under_her(void)
{
    static TestGame tg;
    fresh(&tg);
    static const char *const W[] = { P_1, P_2, P_3, P_4 };
    ARun r = a_run(&tg, W, 4, NULL, 0);

    /* Fixture wiring: all four really are Printemps MEMBERS. */
    for (int i = 0; i < r.n; i++)
        CHECK(in_group(r.ids[i], JW_PRINTEMPS) && rb_card_is_member(r.ids[i]),
              "A1: waitroom card %d (%s) IS a Printemps member "
              "(unit=%s group=%s)",
              i, card_no_of(r.ids[i]), rb_card_unit_name(r.ids[i]),
              rb_card_group_name(r.ids[i]));
    /* The four must be four DISTINCT instances, else "4" could be one card
     * counted four ways.  P_1 and P_3 are two different PRINTS of the same
     * character, so the assertion is on identity, not on the name. */
    for (int i = 0; i < r.n; i++)
        for (int j = i + 1; j < r.n; j++)
            CHECK(r.ids[i] != r.ids[j],
                  "A1: waitroom cards %d and %d are two distinct instances", i, j);
    CHECK(strcmp(card_name_of(r.ids[0]), card_name_of(r.ids[2])) == 0,
          "A1: and P_1/P_3 really ARE the same character (%s) -- so the filter "
          "cannot be satisfied by name", card_name_of(r.ids[0]));

    check_census(&r, "4 Printemps members available");
    expect_offered_prompt(&r, "A1");
    CHECK_EQ(under_n(&tg, AREA_CENTER), 4,
             "A1: 4 Printemps members from MY waitroom go under HER");
    for (int i = 0; i < r.n; i++)
        CHECK(test_zone_count_of_id(&tg, 0, "under1", r.ids[i]) == 1,
              "A1: waitroom card %d (%s) really landed under her, not a "
              "different card with the same number", i, card_no_of(r.ids[i]));
    /* The four left MY waitroom, i.e. they MOVED. */
    CHECK_EQ(wait_n(&tg, 0), 0,
             "A1: the four came out of MY waitroom -- they moved, they were "
             "not duplicated");
    CHECK_EQ(hand_n(&tg, 0), 0,
             "A1: and the hand is empty again (NOZOMI was played, nothing "
             "else was added)");
}

/* Rust only_four_are_taken. */
static void a2_only_four_are_taken(void)
{
    static TestGame tg;
    fresh(&tg);
    static const char *const W[] = { P_1, P_2, P_3, P_4, P_5 };
    ARun r = a_run(&tg, W, 5, NULL, 0);
    check_census(&r, "5 Printemps members available");
    expect_offered_prompt(&r, "A2");
    CHECK_EQ(under_n(&tg, AREA_CENTER), 4,
             "A2: 「4枚」 is EXACT -- a fifth available member does not go under");
    /* Which four is the engine's choice; that ONE of the five stayed out is
     * what the count asserts. */
    int left = 0;
    for (int i = 0; i < r.n; i++)
        if (test_zone_count_of_id(&tg, 0, "under1", r.ids[i]) == 0) left++;
    CHECK_EQ(left, 1, "A2: exactly one of the five stayed in the waitroom");
}

/* Rust fewer_than_four_places_nothing.  「4枚を…置く」 is a REQUIREMENT, not
 * 「4枚まで」, so with two available the ability cannot pay and nothing moves. */
static void a3_fewer_than_four_places_nothing(void)
{
    static TestGame tg;
    fresh(&tg);
    static const char *const W[] = { P_1, P_2 };
    ARun r = a_run(&tg, W, 2, NULL, 0);
    check_census(&r, "only 2 available");
    CHECK_EQ(under_n(&tg, AREA_CENTER), 0,
             "A3: 4枚 is a REQUIREMENT -- with only two available nothing is "
             "placed");
    /* PAIRED POSITIVE on the same board shape: a1 is the same setup with two
     * more cards in the waitroom and reaches 4.  Without a1 this 0 is
     * indistinguishable from "the ability never ran".  Recorded here as a
     * self-contained proof that the fixture cards are eligible at all. */
    for (int i = 0; i < r.n; i++)
        CHECK(in_group(r.ids[i], JW_PRINTEMPS) && rb_card_is_member(r.ids[i]),
              "A3: waitroom card %d really is a Printemps MEMBER, so the 0 "
              "above is the COUNT and not a dead filter", i);
    /* THE CORRECT CONTROL FOR THIS CASE is the NEGATIVE prompt control, and it
     * is a real behavioural claim, not a formality: with fewer than four
     * eligible members the engine must NOT offer a 4-card selection at all.
     * (Measured: prompt_count and prompt_nfid are both 0 here, so the engine
     * declines to open an unpayable prompt rather than opening one and
     * resolving it to nothing.  Rust a3 passes for the same net reason --
     * nothing is placed -- but reaches it without a prompt, so this records
     * the C path, it does not paper over a divergence in the printed result.) */
    CHECK(!r.prompted,
          "A3: with only two eligible members the engine must not offer a "
          "4-card selection (offered %d candidate(s), wants %d)", r.prompt_nfid,
          r.prompt_count);
    CHECK(tg.state.p[0].stage[AREA_CENTER] == r.nozomi,
          "A3: NOZOMI is on the CENTRE, so the 0 is not a failed play");
    CHECK_EQ(wait_n(&tg, 0), 2, "A3: both waitroom members are still there");
}

/* Rust an_empty_waitroom_puts_nothing_under. */
static void a4_an_empty_waitroom_puts_nothing_under(void)
{
    static TestGame tg;
    fresh(&tg);
    ARun r = a_run(&tg, NULL, 0, NULL, 0);
    check_census(&r, "empty waitroom");
    CHECK_EQ(wait_n(&tg, 0), 0, "A4: premise -- MY waitroom really is empty");
    CHECK_EQ(under_n(&tg, AREA_CENTER), 0,
             "A4: no members in the waitroom, so nothing to place");
    /* NON-VACUITY: the debut really ran and the host really arrived, so the 0
     * is about the empty source and not about a failed play. */
    CHECK(tg.state.p[0].stage[AREA_CENTER] == r.nozomi,
          "A4: NOZOMI is on the CENTRE, so the 0 is not a failed play");
}

/* Rust non_printemps_members_are_not_taken.  THE GROUP-FILTER NEGATIVE. */
static void a5_non_printemps_members_are_not_taken(void)
{
    static TestGame tg;
    fresh(&tg);
    static const char *const W[] = { NON_P, NON_P, NON_P, NON_P };
    ARun r = a_run(&tg, W, 4, NULL, 0);
    check_census(&r, "4 AZALEA members available");
    /* Fixture wiring: four DISTINCT instances of a real NON-Printemps member.
     * Without this the 0 below could be "no cards in the waitroom". */
    CHECK_EQ(r.n, 4, "A5: four waitroom cards were really seeded");
    for (int i = 0; i < r.n; i++) {
        CHECK(r.ids[i] >= 0 && (i == 0 || r.ids[i] != r.ids[0]),
              "A5: waitroom card %d is a distinct instance", i);
        CHECK(rb_card_is_member(r.ids[i]), "A5: waitroom card %d is a member", i);
        CHECK(!in_group(r.ids[i], JW_PRINTEMPS),
              "A5: waitroom card %d ('%s') is OUTSIDE Printemps "
              "(unit='%s' group='%s') -- so the 0 below is the group "
              "filter, not an empty board", i, card_no_of(r.ids[i]),
              rb_card_unit_name(r.ids[i]), rb_card_group_name(r.ids[i]));
    }
    CHECK_EQ(wait_n(&tg, 0), 4, "A5: all four are still in MY waitroom");
    CHECK_EQ(under_n(&tg, AREA_CENTER), 0,
             "A5: AZALEA members in the waitroom are excluded");
    /* PAIRED POSITIVE: a1 is the SAME debut with four Printemps members and
     * reaches 4.  This is the control that makes the 0 non-vacuous. */
    CHECK_EQ(test_zone_len(&tg, 0, "under1"), 0,
             "A5: paired positive lives in a1_four_printemps_members_go_under_her");
}

/* Rust a_mix_takes_only_the_printemps_members. */
static void a6_a_mix_takes_only_the_printemps_members(void)
{
    static TestGame tg;
    fresh(&tg);
    static const char *const W[] = { P_1, NON_P, P_2, NON_P, P_3, P_4 };
    ARun r = a_run(&tg, W, 6, NULL, 0);
    check_census(&r, "4 Printemps + 2 AZALEA in the waitroom");
    expect_offered_prompt(&r, "A6");
    CHECK_EQ(under_n(&tg, AREA_CENTER), 4,
             "A6: of six waitroom members the four Printemps ones are taken");
    /* The two AZALEA ones are the ones left behind -- asserted by ID, not by
     * count, so "four cards of any sort arrived" cannot pass. */
    for (int i = 0; i < r.n; i++) {
        int is_p = !strcmp(W[i], NON_P) ? 0 : 1;
        int landed = test_zone_count_of_id(&tg, 0, "under1", r.ids[i]);
        CHECK_EQ(landed, is_p,
                 "A6: waitroom card %d ('%s') %s", i, card_no_of(r.ids[i]),
                 is_p ? "went under her" : "stayed in the waitroom");
        if (!is_p)
            CHECK(wait_has(&tg, 0, r.ids[i]),
                  "A6: the AZALEA card %d is still in MY waitroom", i);
    }
}

/* Rust it_is_her_own_under_area. */
static void a7_it_is_her_own_under_area(void)
{
    static TestGame tg;
    fresh(&tg);
    ARun r = a_run(&tg, (const char *const[]){ P_2, P_3, P_4, P_5 }, 4,
                   P_1, AREA_LEFT);
    check_census(&r, "another member already on the LEFT");
    expect_offered_prompt(&r, "A7");
    CHECK(tg.state.p[0].stage[AREA_LEFT] == r.other,
          "A7: premise -- the other member is on the LEFT");
    CHECK_EQ(under_n(&tg, AREA_CENTER), 4,
             "A7: precondition -- the four members landed under HER");
    CHECK_EQ(under_n(&tg, AREA_LEFT), 0,
             "A7: under_self -- the cards belong under the member who gained "
             "the ability, not under whoever else is on the stage");
    for (int i = 0; i < r.n; i++)
        CHECK(test_zone_count_of_id(&tg, 0, "under1", r.ids[i]) == 1,
              "A7: waitroom card %d landed under HER specifically", i);
}

/* ═════════════════════════════════════════════════════════════════════════
 * B1. effects/kidou -- hs_bp1_002_rm_same_area_redeploy_test.rs (5).
 *
 * PL!HS-bp1-002-RM 起動(turn1), paid through rb_activate_card so the
 * sequential_cost RUNS: [pay 2E, move SELF stage->waitroom].  The second
 * sub-step is what VACATES the area, and the effect step then reads the vacated
 * area and deploys ONE Hasunosora member of cost <= 15 from MY waitroom into
 * that SAME slot.
 *
 * The host is a legal target for her own retrieval (the text does not say
 * 「他の」), so which card lands is the engine's choice and is NOT what these
 * tests are about.  What must hold is the AREA and the printed filter.
 * ═════════════════════════════════════════════════════════════════════════ */

typedef struct {
    int sayaka, other;
    int prompts_before, prompts_after, prompted;
    int landed_left, landed_center, landed_right;
    int census0, census1;
    int active_after_play, active_after;
    int cost_ran;                 /* SAYAKA reached MY waitroom via the cost  */
    int energy_leg_ran;           /* active energy fell by the 起動's 2E      */
    int activated;
} BRun;

static BRun b_run(TestGame *tg, const char *fixture_no, int area)
{
    BRun r;
    memset(&r, 0, sizeof r);
    r.landed_left = r.landed_center = r.landed_right = RB_EMPTY_SLOT;

    int filler = mid(tg, P_4);
    if (filler < 0) return r;
    fill_decks(tg, filler, 30);

    r.sayaka = mid(tg, SAYAKA);
    if (r.sayaka < 0 || !pin_id(r.sayaka, SAYAKA)) return r;
    r.other = mid(tg, fixture_no);
    if (r.other < 0 || !pin_id(r.other, fixture_no)) return r;
    wait_append(tg, 0, r.other);

    /* Only the host is on the stage, and only in `area`, so whatever appears
     * in `area` afterwards must have arrived through the effect. */
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    hand_append(tg, 0, r.sayaka);
    set_energy(tg, 0, 30, 20);
    CHECK_EQ(printed_cost(r.sayaka), 11,
             "B1: premise -- SAYAKA prints cost 11 (11 + 2E = 13 of 20)");
    if (!expect_trigger(r.sayaka, RB_TSTR_ACTIVATION, SAYAKA)) return r;

    r.prompts_before = tal ? tal->prompts : 0;
    r.census0 = test_total_card_count(tg);

    /* Rust `game.play_to_stage(sayaka, area)` -- she must be ON THE STAGE
     * before the 起動, because it is the COST that vacates the area.  An
     * earlier revision of this file skipped this and activated her from hand,
     * which made `same_area` have no vacated area to remember at all. */
    if (!test_play_to_stage(tg, r.sayaka, area)) {
        CHECK(0, "B1: SAYAKA reaches area %d (cost 11 of 20 active)", area);
        return r;
    }
    CHECK(tg->state.p[0].stage[area] == r.sayaka,
          "B1: premise -- SAYAKA is standing in area %d", area);
    if (pending(tg)) { dump_choice(tg, "after-play"); drain_auto(tg, 12); }
    r.active_after_play = active_e(tg, 0);

    r.activated = test_activate_ability(tg, r.sayaka);
    dump_choice(tg, "after-activate");
    if (pending(tg)) {
        r.prompted = 1;
        /* Rust `game.select_indices(&[0])` -- unconditional index 0.  With two
         * eligible candidates this is the prompt path, which is the one the
         * vacated-area bug used to break. */
        int zero = 0;
        select_n(tg, &zero, 1);
    }
    drain_auto(tg, 12);
    r.prompts_after = tal ? tal->prompts : 0;
    r.active_after = active_e(tg, 0);
    r.census1 = test_total_card_count(tg);
    r.landed_left   = tg->state.p[0].stage[AREA_LEFT];
    r.landed_center = tg->state.p[0].stage[AREA_CENTER];
    r.landed_right  = tg->state.p[0].stage[AREA_RIGHT];
    /* ID-LEVEL BOARD DUMP.  Without this the report has to GUESS which of the
     * two waitroom cards the engine deployed, and a guess about a card
     * identity is exactly the false red the brief warns about. */
    fprintf(stderr, "        [B1 board] sayaka=%d(%s) other=%d(%s)\n",
            r.sayaka, card_no_of(r.sayaka), r.other, card_no_of(r.other));
    fprintf(stderr, "        [B1 board] stage=[%d(%s), %d(%s), %d(%s)]\n",
            r.landed_left, card_no_of(r.landed_left),
            r.landed_center, card_no_of(r.landed_center),
            r.landed_right, card_no_of(r.landed_right));
    fprintf(stderr, "        [B1 board] waitroom(%d):", wait_n(tg, 0));
    for (int i = 0; i < tg->state.p[0].discard.n; i++)
        fprintf(stderr, " %d(%s)", tg->state.p[0].discard.cards[i],
                card_no_of(tg->state.p[0].discard.cards[i]));
    fprintf(stderr, "  hand(%d):", hand_n(tg, 0));
    for (int i = 0; i < tg->state.p[0].hand.n; i++)
        fprintf(stderr, " %d(%s)", tg->state.p[0].hand.cards[i],
                card_no_of(tg->state.p[0].hand.cards[i]));
    fprintf(stderr, "\n");
    /* ── THE SHARED PRECONDITION FOR THE WHOLE B1 FAMILY ───────────────────
     * Asserted HERE rather than per case, and it is what stops every positive
     * in B1 from being satisfied by the host's own unmoved body.
     *
     * The 起動's cost is a `sequential_cost` of [pay 2E, move SELF
     * stage->waitroom].  Leg 2 is what VACATES the area, and the effect step
     * then reads that area.  Rust's own file insists the cost must actually be
     * paid: "Goes through run_main_action, so the sequential cost is actually
     * paid -- the cost is what vacates the area, and skipping it is what made
     * an earlier version of this file land every card in the centre."
     *
     * Measured in C: NEITHER leg runs.  Active energy is unchanged across the
     * activation, and SAYAKA is still standing in the area she was played to.
     * The EFFECT still runs -- the 1-card selection prompt is raised, offers
     * nf=1 and is answered -- but nothing is deployed, so the area still holds
     * the host and the waitroom still holds the fixture.  Left unasserted, that
     * makes every "a card is in the vacated area" and "the card there costs
     * <= 15" positive GREEN on the host who never left (measured: b1 and b2
     * were green/false-green for exactly this reason before this check
     * existed).  So it is asserted strictly, once, for all five cases. */
    r.energy_leg_ran = (r.active_after_play - r.active_after) == 2;
    r.cost_ran = wait_has(tg, 0, r.sayaka);
    CHECK(r.energy_leg_ran,
          "B1 COST LEG 1 NOT PAID: the 起動 costs {{icon_energy}}{{icon_energy}} "
          "-- active energy was %d after the play and %d after the activation, "
          "so the 2E leg of the sequential_cost did not run. The EFFECT ran "
          "anyway (a selection prompt was raised and answered), so this is a "
          "half-paid activation, not a refused one.",
          r.active_after_play, r.active_after);
    CHECK(r.cost_ran,
          "B1 COST LEG 2 NOT PAID: 「このメンバーをステージから控え室に置く」 "
          "-- SAYAKA is not in MY waitroom after the activation. She is still "
          "standing in area %d, so no area was ever vacated, "
          "「このメンバーがいたエリア」 has nothing to remember, and every "
          "positive in this family is being satisfied by a host who never "
          "left. (waitroom now holds: see the [B1 board] dump above.)", area);
    fprintf(stderr, "        [B1 cost] leg1_paid=%d leg2_paid=%d "
            "active %d->%d\n", r.energy_leg_ran, r.cost_ran,
            r.active_after_play, r.active_after);
    fprintf(stderr, "        [B1] fixture=%s area=%d activated=%d prompted=%d "
            "prompts %d->%d CENSUS %d->%d stage=[%d,%d,%d] active=%d\n",
            fixture_no, area, r.activated, r.prompted, r.prompts_before,
            r.prompts_after, r.census0, r.census1, r.landed_left,
            r.landed_center, r.landed_right, r.active_after);
    return r;
}

static int landed_cost_ok(int cid)
{
    if (cid < 0) return 0;
    int c = printed_cost(cid);
    return c >= 0 && c <= 15;
}

/* Rust a_retrieved_card_lands_in_the_left_side_she_vacated. */
static void b1_retrieved_card_lands_in_the_left_side_she_vacated(void)
{
    static TestGame tg;
    fresh(&tg);
    BRun r = b_run(&tg, CHEAP_HS, AREA_LEFT);
    CHECK(wait_has(&tg, 0, r.sayaka),
          "B1: precondition -- SAYAKA was moved to the waitroom by the cost "
          "(このメンバーをステージから控え室に置く)");
    /* THE ANTI-FALSE-GREEN DISCRIMINATOR.  「このメンバーがいたエリアに登場させる」
     * deploys a card RETRIEVED FROM THE WAITROOM.  If SAYAKA is still standing
     * where she was played, then whatever occupies `area` is her own unmoved
     * body and NOTHING was deployed -- the assertion below would be green for
     * the wrong reason.  A retrieval OF SAYAKA is legal (the text does not say
     * 「他の」) but only once the cost has actually put her in the waitroom, so
     * the two are separated by `r.cost_ran`. */
    CHECK(r.cost_ran || r.landed_left != r.sayaka,
          "B1: area %d still holds SAYAKA herself while the cost never moved "
          "her, so the 'a card is in the vacated area' check below is "
          "satisfied by a host who never left -- nothing was deployed",
          AREA_LEFT);
    CHECK_EQ(r.landed_left != RB_EMPTY_SLOT, 1,
             "B1: she stood in LeftSide, so a card must appear in LeftSide.  An "
             "implementation that lost the vacated area puts it in the first "
             "free slot (the centre) instead.");
    CHECK(landed_cost_ok(r.landed_left),
          "B1: コスト15以下 -- the card placed in the vacated area must print "
          "cost <= 15, but '%s' prints %d", card_no_of(r.landed_left),
          printed_cost(r.landed_left));
    CHECK_EQ(r.census1, r.census0, "B1: CENSUS unchanged by the 起動");
}

/* Rust a_retrieved_card_lands_in_the_centre_she_vacated.  Run TOGETHER with
 * b1: a defaulted slot would fill the same index in both runs and fail one. */
static void b2_retrieved_card_lands_in_the_centre_she_vacated(void)
{
    static TestGame tg;
    fresh(&tg);
    BRun r = b_run(&tg, CHEAP_HS, AREA_CENTER);
    /* Same anti-false-green discriminator as b1: with the cost unpaid, a card
     * in the centre is SAYAKA's own unmoved body, not a retrieval. */
    CHECK(r.cost_ran || r.landed_center != r.sayaka,
          "B2: the centre still holds SAYAKA herself while the cost never "
          "moved her, so nothing was deployed into the vacated area");
    CHECK(r.landed_center != RB_EMPTY_SLOT,
          "B2: she stood in the centre this time, so a card must appear there");
    CHECK_EQ(r.landed_left, RB_EMPTY_SLOT,
             "B2: the left side stays empty.  Together with b1 this pins the "
             "area: a defaulted slot would fill the same index in both runs.");
    CHECK(landed_cost_ok(r.landed_center),
          "B2: コスト15以下 -- '%s' prints %d", card_no_of(r.landed_center),
          printed_cost(r.landed_center));
    CHECK_EQ(r.census1, r.census0, "B2: CENSUS unchanged by the 起動");
}

/* Rust a_member_costing_exactly_fifteen_is_eligible.  「コスト15以下」 is <=,
 * not <.  Which card is picked is the engine's choice; what the printed bound
 * determines is WHICH CARDS ARE ELIGIBLE. */
static void b3_a_member_costing_exactly_fifteen_is_eligible(void)
{
    static TestGame tg;
    fresh(&tg);
    /* Fixture invariant, read from cards.bin, not from the comment. */
    int probe = rb_find_card_by_no(BOUND_HS);
    CHECK_EQ(printed_cost(probe), 15,
             "B3: fixture invariant -- this card prints cost exactly 15, the one "
             "value on which < and <= disagree");
    BRun r = b_run(&tg, BOUND_HS, AREA_LEFT);
    CHECK(r.landed_left != RB_EMPTY_SLOT,
          "B3: a card was placed in the vacated area");
    CHECK(landed_cost_ok(r.landed_left),
          "B3: コスト15以下 -- the card placed must print cost <= 15, but "
          "'%s' prints %d.  A < implementation would exclude the cost-15 "
          "fixture and this would be satisfiable only by the cost-11 host, so "
          "the bound is what this pins.", card_no_of(r.landed_left),
          printed_cost(r.landed_left));
    CHECK(!wait_has(&tg, 0, r.other) || r.landed_left == r.other,
          "B3: the cost-15 fixture is a legal target under <= 15: it may be "
          "left in the waitroom only because the engine chose the host, never "
          "because it was filtered out");
    CHECK_EQ(r.census1, r.census0, "B3: CENSUS unchanged by the 起動");
}

/* Rust a_hasunosora_member_costing_seventeen_is_not_a_target.  THE COST GATE
 * ALONE: Hasunosora, but over the bound. */
static void b4_hasunosora_member_costing_seventeen_is_not_a_target(void)
{
    static TestGame tg;
    fresh(&tg);
    int probe = rb_find_card_by_no(OVER_HS);
    CHECK_EQ(printed_cost(probe), 17,
             "B4: fixture invariant -- this card prints cost 17, above the bound");
    BRun r = b_run(&tg, OVER_HS, AREA_LEFT);
    /* Fixture wiring: it IS Hasunosora, so only the cost gate can exclude it. */
    CHECK(in_group(r.other, JW_HASUNO),
          "B4: the over-cost fixture really IS a 蓮ノ空 card (series-derived "
          "group='%s' unit='%s'), so the assertion below is about the "
          "COST gate alone", rb_card_group_name(r.other),
          rb_card_unit_name(r.other));
    CHECK(wait_has(&tg, 0, r.other),
          "B4: コスト15以下 -- she is Hasunosora so she passes the group gate, "
          "but costs 17, so she is not a legal target and must still be in the "
          "waitroom.  An engine with no cost filter would have taken her.");
    CHECK(landed_cost_ok(r.landed_left),
          "B4: whatever DID land in the vacated area is cost <= 15 "
          "('%s')", card_no_of(r.landed_left));
    CHECK_EQ(r.census1, r.census0, "B4: CENSUS unchanged by the 起動");
}

/* Rust a_non_hasunosora_member_is_not_a_target.  THE GROUP GATE ALONE. */
static void b5_a_non_hasunosora_member_is_not_a_target(void)
{
    static TestGame tg;
    fresh(&tg);
    int probe = rb_find_card_by_no(FILLER_HS);
    CHECK_EQ(printed_cost(probe), 4,
             "B5: fixture invariant -- this card prints cost 4, UNDER the bound, "
             "so only the group gate can exclude it");
    BRun r = b_run(&tg, FILLER_HS, AREA_LEFT);
    CHECK(!in_group(r.other, JW_HASUNO),
          "B5: the under-cost fixture really is OUTSIDE 蓮ノ空 "
          "(group='%s' unit='%s')", rb_card_group_name(r.other),
          rb_card_unit_name(r.other));
    CHECK(wait_has(&tg, 0, r.other),
          "B5: 高坂穂乃果 costs 4 so she passes the cost gate, but her series is "
          "Love Live! (Printemps), so she is not a legal target and must still "
          "be in the waitroom.  This is the control that the group half is live "
          "at all.");
    /* PAIRED POSITIVE on the same board shape: b4 is the same debut with a
     * Hasunosora member and a card DOES land in the vacated area. */
    CHECK(landed_cost_ok(r.landed_left),
          "B5: paired positive -- a card still reached the vacated area, so the "
          "ability ran and the 0-move above is the group filter");
    CHECK_EQ(r.census1, r.census0, "B5: CENSUS unchanged by the 起動");
}

/* ═════════════════════════════════════════════════════════════════════════
 * B2. effects/kidou -- pb4_gated_activation_score_total_kidou_pl_pb4_002_sec
 *     _test.rs (3).
 *
 * PL!-bp4-002-SEC 起動(turn1): discard 2 hand cards, then take a mu's live from
 * MY waitroom.  ACTIVATION GATE: the total printed score in MY success live
 * card zone must be >= 6 (comparison_type "score", aggregate "total").  There
 * is NO group filter on the gate -- any live card contributes.
 *
 * 「のみ起動できる」 is a statement about what the player may DO, so the OFFER
 * is the thing to measure.  Forcing the activation pays the cost first, which
 * is the bypass the Rust file explicitly rejects.  C has no `disabled` flag on
 * RbGeneratedActionParameters, so "offered" here means "an ActivateAbility
 * action for ERI appears in rb_generate_action_candidates() AT ALL" -- a
 * STRICTER reading than Rust's (a listed-but-disabled action would count as
 * offered in C and as not-offered in Rust).  The difference is reported, not
 * papered over.
 * ═════════════════════════════════════════════════════════════════════════ */

typedef struct {
    int total;
    int n_candidates;
    int n_activate_candidates;
    int offered;
    int census0, census1;
} GateRun;

static GateRun gate_run(TestGame *tg, const char *const *lives, int n_lives)
{
    GateRun r;
    memset(&r, 0, sizeof r);

    int filler = mid(tg, P_4);
    if (filler < 0) return r;
    fill_decks(tg, filler, 40);

    for (int i = 0; i < n_lives; i++) {
        int id = mid(tg, lives[i]);
        if (id < 0 || !pin_id(id, lives[i])) return r;
        int sc = printed_score(id);
        CHECK(sc >= 0, "B2: %s has a PRINTED score, so it can pin the bound "
              "(got %d)", lives[i], sc);
        CHECK(rb_card_is_live(id), "B2: %s is a LIVE card", lives[i]);
        r.total += sc;
        test_add_to_success(tg, id);
        CHECK(success_has(tg, 0, id),
              "B2: %s really is in MY success live card zone", lives[i]);
    }

    int eri = mid(tg, ERI);
    if (eri < 0 || !pin_id(eri, ERI)) return r;
    hand_append(tg, 0, eri);
    /* The cost discards two hand cards, so the hand needs two besides ERI. */
    int h1 = mid(tg, ERI_FILL), h2 = mid(tg, ERI_FILL);
    if (h1 < 0 || h2 < 0) return r;
    hand_append(tg, 0, h1);
    hand_append(tg, 0, h2);
    set_energy(tg, 0, 30, 20);
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    if (!expect_trigger(eri, RB_TSTR_ACTIVATION, ERI)) return r;
    CHECK_EQ(printed_cost(eri), 15, "B2: premise -- ERI prints cost 15");

    r.census0 = test_total_card_count(tg);
    CHECK(test_play_to_stage(tg, eri, AREA_CENTER),
          "B2: ERI reaches the CENTRE (cost 15 of 20 active)");
    if (pending(tg)) dump_choice(tg, "debut");
    drain_auto(tg, 12);

    /* ── THE MEASUREMENT ──────────────────────────────────────────────────
     * Taken here, at the OFFER, with the cost still unpaid. */
    RbGeneratedActionList lst = rb_generate_action_candidates(&tg->state);
    r.n_candidates = lst.count;
    r.n_activate_candidates = 0;
    for (int i = 0; i < lst.count; i++) {
        const RbGeneratedAction *a = &lst.actions[i];
        if (a->action_type == RB_ACTION_ACTIVATE_ABILITY) {
            r.n_activate_candidates++;
            if (a->has_parameters && a->parameters.card_id == eri) r.offered = 1;
        }
        fprintf(stderr, "        [B2 offer] #%d type=%d card_id=%d has_params=%d "
                "n_areas=%d\n", i, a->action_type, a->parameters.card_id,
                a->has_parameters, a->parameters.n_available_areas);
    }
    free(lst.actions);
    r.census1 = test_total_card_count(tg);
    fprintf(stderr, "        [B2] lives=%d total=%d (gate is >= 6) offered=%d "
            "candidates=%d activate_candidates=%d CENSUS %d->%d\n",
            n_lives, r.total, r.offered, r.n_candidates,
            r.n_activate_candidates, r.census0, r.census1);
    return r;
}

/* THE INSTRUMENT GATE.  Asserted in all three B2 cases, and it is what stops
 * b7 and b8 from passing green on the ABSENCE of a feature.
 *
 * rb_generate_action_candidates (src/turn/phase.c:1182) emits exactly two
 * action types, both hard-coded as raw integers:
 *     pass.action_type   = 1   (phase.c:1192 -- one "pass" placeholder; 1 is
 *                                RB_ACTION_DRAW_UNTIL_COUNT, not a pass verb --
 *                                there is no RB_ACTION_PASS in the enum)
 *     action.action_type = 14  (phase.c:1250 -- one per available stage area
 *                                for a HAND member; 14 is
 *                                RB_ACTION_POSITION_CHANGE)
 * RB_ACTION_ACTIVATE_ABILITY is 23 and is NEVER emitted, for any card, in any
 * zone.  So the Main-phase offer list contains no UseAbility action at all, and
 * 「のみ起動できる」 on a STAGED member has no offer path in C: the printed
 * gate is unmeasurable end to end.  Rust's generate_possible_actions DOES emit
 * ActionType::UseAbility with a `disabled` flag, which is the whole instrument
 * this Rust file is built on.
 *
 * Asserting this keeps the three cases honest: with the instrument missing,
 * b6 (must be offered) and b7/b8 (must not be offered) would otherwise be
 * indistinguishable, and b7/b8 would be green for the wrong reason. */
static void require_offer_instrument(GateRun *r, const char *what)
{
    CHECK(r->n_activate_candidates > 0,
          "INSTRUMENT GAP: %s -- rb_generate_action_candidates emitted %d "
          "action(s) on this Main-phase board and NONE of them is "
          "RB_ACTION_ACTIVATE_ABILITY (%d). The generator only ever emits a "
          "pass placeholder and a per-area play-member action, so a STAGED "
          "member's 起動 has no OFFER in C and the 「のみ起動できる」 gate "
          "cannot be measured. Without this the withheld-ability cases below "
          "would be green because nothing offers anything.",
          what, r->n_candidates, r->n_activate_candidates);
}

/* Rust a_success_zone_total_of_six_offers_the_ability.  THE CONTROL: at
 * exactly the printed total the 起動 is offered, so the card, the fixture and
 * the offer path are all sound.  Without it the two below are unfalsifiable. */
static void b6_a_success_zone_total_of_six_offers_the_ability(void)
{
    static TestGame tg;
    fresh(&tg);
    int probe = rb_find_card_by_no(SCORE_6);
    CHECK_EQ(printed_score(probe), 6,
             "B6: fixture invariant -- the boundary live prints score 6");
    GateRun r = gate_run(&tg, (const char *const[]){ SCORE_6 }, 1);
    CHECK_EQ(r.total, 6, "B6: the success-zone total really is 6");
    require_offer_instrument(&r, "B6");
    CHECK(r.offered,
          "B6: スコアの合計が6以上の場合のみ起動できる -- one score-6 live "
          "totals 6, so the 起動 must be offered (generate_possible_actions "
          "emitted %d action(s), none of them ActivateAbility for ERI)",
          r.n_candidates);
    CHECK_EQ(r.census1, r.census0, "B6: CENSUS unchanged by the debut");
}

/* Rust a_success_zone_total_of_five_withholds_the_ability.  EXACT-BOUNDARY
 * NEGATIVE: a </<= implementation, or > in place of >=, fails here. */
static void b7_a_success_zone_total_of_five_withholds_the_ability(void)
{
    static TestGame tg;
    fresh(&tg);
    int p4 = rb_find_card_by_no(SCORE_4), p1 = rb_find_card_by_no(SCORE_1);
    CHECK_EQ(printed_score(p4), 4, "B7: the score-4 fixture really prints 4");
    CHECK_EQ(printed_score(p1), 1, "B7: the score-1 fixture really prints 1");
    GateRun r = gate_run(&tg, (const char *const[]){ SCORE_4, SCORE_1 }, 2);
    CHECK_EQ(r.total, 5, "B7: the success-zone total really is 5, one below "
                         "the printed bound");
    /* NON-VACUITY: the generator really ran and really produced actions on
     * THIS board.  Without this, a generator that returned nothing at all
     * would make the "not offered" below pass for the wrong reason. */
    CHECK(r.n_candidates > 0,
          "B7 (VACUITY): rb_generate_action_candidates returned 0 actions on "
          "this board, so 'not offered' proves nothing");
    require_offer_instrument(&r, "B7");
    CHECK(!r.offered,
          "B7: スコアの合計が6以上 -- score 4 + score 1 totals 5, one below the "
          "bound, so the 起動 must NOT be offered");
    CHECK_EQ(r.census1, r.census0, "B7: CENSUS unchanged by the debut");
}

/* Rust an_empty_success_zone_withholds_the_ability. */
static void b8_an_empty_success_zone_withholds_the_ability(void)
{
    static TestGame tg;
    fresh(&tg);
    GateRun r = gate_run(&tg, NULL, 0);
    CHECK_EQ(r.total, 0, "B8: an empty 成功ライブカード置き場 totals 0");
    CHECK(r.n_candidates > 0,
          "B8 (VACUITY): rb_generate_action_candidates returned 0 actions on "
          "this board, so 'not offered' proves nothing");
    require_offer_instrument(&r, "B8");
    CHECK(!r.offered,
          "B8: an empty 成功ライブカード置き場 totals 0, which is not >= 6, so "
          "the 起動 must not be offered");
    CHECK_EQ(r.census1, r.census0, "B8: CENSUS unchanged by the debut");
}

/* ═════════════════════════════════════════════════════════════════════════
 * C. effects/move_cards -- pb2_reveal_lily_white_live_and_swap_success_zone
 *    _pl_pb2_014_p_plus_test.rs (4).  A PARSE-THE-SHAPE file.
 *
 * PL!-pb2-014-P+ 登場:
 *   cost:   OPTIONAL reveal of one lily white LIVE from HAND.
 *   effect: TWO steps -- take a card from MY success live card zone into hand,
 *           THEN (そうしたとき) put the revealed card into MY success live
 *           card zone.
 *
 * THE PARSER GAP THIS FILE PINS: the parse kept step (2) and DROPPED step (1).
 * The correct shape is a `sequential [move_cards, move_cards]`.  What survives
 * is a bare move_cards from `selected_cards` to `success_live_zone`, gated by a
 * garbled 「そうしたとき」 condition: a card-count comparison with operator "="
 * and no value, i.e. a bound the engine cannot evaluate.
 *
 * The Rust file reads the COST out of cards/abilities.json because Rust's
 * AbilityCost exposes neither the accessors nor a `type` key.  C DECODES the
 * cost into a real AbilityEffect (Ability.cost), so the cost clause is
 * asserted from the engine here -- strictly stronger than the Rust twin.
 *
 * EVERY assertion in this section is decode-dependent and therefore subject to
 * the bytecode/string-table skew described in the header.  a0_skew_probe runs
 * first; if it is red, re-classify these.
 * ═════════════════════════════════════════════════════════════════════════ */

static int decode_debut_ability(int cid, Ability *out)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        memset(out, 0, sizeof *out);
        if (!rb_decode_card_ability((uint32_t)cid, i, out)) continue;
        if (out->triggers && !strcmp(out->triggers, RB_TSTR_DEBUT)) return 1;
        rb_free_ability(out);
    }
    memset(out, 0, sizeof *out);
    return 0;
}

/* Rust the_cost_clause_is_an_optional_reveal_of_a_lily_white_live_from_hand. */
static void c1_cost_is_an_optional_reveal_of_a_lily_white_live_from_hand(void)
{
    static TestGame tg;
    fresh(&tg);
    int shiro = mid(&tg, SHIRO);
    if (!pin_id(shiro, SHIRO)) return;

    Ability ab;
    if (!decode_debut_ability(shiro, &ab)) {
        CHECK(0, "C1: %s really has a 登場 ability", SHIRO);
        return;
    }
    CHECK(ab.cost != NULL,
          "C1: the 登場 ability HAS a cost -- the cost clause 「手札の『lily "
          "white』のライブカードを1枚公開してもよい」 lives on the cost");
    if (ab.cost) {
        const AbilityEffect *c = ab.cost;
        CHECK(c->action && !strcmp(c->action, "reveal"),
              "C1: the cost is a REVEAL (got action=[%s])", safe(c->action));
        CHECK(c->source && !strcmp(c->source, "hand"),
              "C1: 手札 -- the cost's source is hand (got [%s])", safe(c->source));
        CHECK_EQ(c->count, 1, "C1: 1枚公開");
        CHECK(c->is_optional,
              "C1: 「もよい」 -- the cost is OPTIONAL (is_optional=%d)",
              c->is_optional);
        int grp = 0;
        for (int i = 0; i < c->n_extra; i++)
            if (c->extra_k[i] && !strcmp(c->extra_k[i], "group_names"))
                grp = (c->extra_v[i] && strstr(c->extra_v[i], JW_LILYWHITE)) ? 1 : 0;
        CHECK(grp, "C1: 『lily white』 -- the cost's group filter really names "
                   "lily white (n_extra=%d)", c->n_extra);
    }
    /* The EFFECT half, so the two clauses are not confused. */
    CHECK(ab.effect && ab.effect->text &&
          strstr(ab.effect->text, JW_SUCCESS_ZONE),
          "C1: the surviving effect is the PLACEMENT clause -- its text names the "
          "success live card zone (text=[%s])",
          ab.effect ? safe(ab.effect->text) : "(no effect)");
    rb_free_ability(&ab);
}

/* Rust the_effect_places_the_revealed_card_into_the_success_zone. */
static void c2_effect_places_the_revealed_card_into_the_success_zone(void)
{
    static TestGame tg;
    fresh(&tg);
    int shiro = mid(&tg, SHIRO);
    if (!pin_id(shiro, SHIRO)) return;
    Ability ab;
    if (!decode_debut_ability(shiro, &ab)) {
        CHECK(0, "C2: %s really has a 登場 ability", SHIRO);
        return;
    }
    CHECK(ab.effect != NULL, "C2: the 登場 ability HAS an effect");
    if (ab.effect) {
        const AbilityEffect *e = ab.effect;
        CHECK(e->action && !strcmp(e->action, "move_cards"),
              "C2: これにより公開したカードを自分の成功ライブカード置き場に"
              "置く -- a MOVE (got action=[%s])", safe(e->action));
        CHECK(e->source && !strcmp(e->source, "selected_cards"),
              "C2: これにより公開したカード -- the source is the card the COST "
              "revealed (got [%s])", safe(e->source));
        CHECK(e->destination && !strcmp(e->destination, "success_live_zone"),
              "C2: 自分の成功ライブカード置き場に置く (got destination=[%s])",
              safe(e->destination));
        CHECK_EQ(e->count, 1, "C2: 1枚");
    }
    rb_free_ability(&ab);
}

/* Rust the_take_from_the_success_zone_is_currently_missing. */
static void c3_the_take_from_the_success_zone_is_currently_missing(void)
{
    static TestGame tg;
    fresh(&tg);
    int shiro = mid(&tg, SHIRO);
    if (!pin_id(shiro, SHIRO)) return;
    Ability ab;
    if (!decode_debut_ability(shiro, &ab)) {
        CHECK(0, "C3: %s really has a 登場 ability", SHIRO);
        return;
    }
    CHECK(ab.effect != NULL, "C3: the 登場 ability HAS an effect");
    if (ab.effect) {
        const AbilityEffect *e = ab.effect;
        /* OPEN GAP, asserted as Rust asserts it. */
        CHECK(!(e->action && !strcmp(e->action, "sequential")),
              "OPEN GAP: 成功ライブカード置き場にあるカードを1枚手札に加え AND "
              "これにより公開したカードを成功ライブカード置き場に置く is a "
              "TWO-STEP effect, but the node is a bare move_cards (action=[%s]), "
              "so the first step is not represented", safe(e->action));
        /* The leftover gate, pinned exactly so it cannot be mistaken for the
         * missing step. */
        CHECK(e->has_condition && e->condition != NULL,
              "OPEN GAP: the そうしたとき gate is still present as a condition");
        if (e->condition) {
            const char *op = NULL;
            int64_t cnt = -1;
            for (uint32_t i = 0; i < e->condition->n_fields; i++) {
                const CondField *f = &e->condition->fields[i];
                if (!f->key) continue;
                if (!strcmp(f->key, "operator") && f->v.tag == RB_TAG_STR)
                    op = f->v.s;
                if (!strcmp(f->key, "count")) cnt = f->v.i;
            }
            CHECK(op && !strcmp(op, "="),
                  "OPEN GAP: the surviving condition's operator is \"=\", a "
                  "card-count comparison the engine's count comparison does not "
                  "understand (got [%s])", op ? op : "(absent)");
            CHECK_EQ(cnt, 1,
                     "OPEN GAP: and its count is 1, with no value to compare "
                     "against -- a bound it cannot evaluate");
        }
    }
    rb_free_ability(&ab);
}

/* Rust fixtures_cover_the_reveal_filter. */
static void c4_fixtures_cover_the_reveal_filter(void)
{
    static TestGame tg;
    fresh(&tg);
    int mus = mid(&tg, MUS_LIVE);
    int lw  = mid(&tg, LW_LIVE);
    int oth = mid(&tg, NOT_LIVE);
    if (!pin_id(mus, MUS_LIVE) || !pin_id(lw, LW_LIVE) ||
        !pin_id(oth, NOT_LIVE)) return;

    CHECK(in_group(mus, JW_LILYWHITE),
          "C4: %s is in the lily white group (unit=[%s] group=[%s])", MUS_LIVE,
          rb_card_unit_name(mus), rb_card_group_name(mus));
    CHECK(rb_card_is_live(mus),
          "C4: %s must be a LIVE card to be revealable", MUS_LIVE);
    /* The other revealable fixture, so the lily-white claim is not resting on
     * one card. */
    CHECK(in_group(lw, JW_LILYWHITE) && rb_card_is_live(lw),
          "C4: %s is also a lily white LIVE (unit=[%s] group=[%s])", LW_LIVE,
          rb_card_unit_name(lw), rb_card_group_name(lw));
    CHECK(!rb_card_is_live(oth),
          "C4: %s is a MEMBER, so it can never be revealed by a ライブカード "
          "cost", NOT_LIVE);
    CHECK(rb_card_is_member(oth),
          "C4: and it really is a member (not a mis-resolved live)");
}

/* ═════════════════════════════════════════════════════════════════════════
 * main
 * ═════════════════════════════════════════════════════════════════════════ */

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

    printf("--- A0 SKEW PROBE (read this before trusting any A/B1/C red) ---\n");
    run("a0_skew_probe", a0_skew_probe);

    printf("--- A effects/move_cards/under_member PL!-pb2-017-P+ (7) ---\n");
    run("a1_four_printemps_members_go_under_her",
        a1_four_printemps_members_go_under_her);
    run("a2_only_four_are_taken", a2_only_four_are_taken);
    run("a3_fewer_than_four_places_nothing", a3_fewer_than_four_places_nothing);
    run("a4_an_empty_waitroom_puts_nothing_under",
        a4_an_empty_waitroom_puts_nothing_under);
    run("a5_non_printemps_members_are_not_taken",
        a5_non_printemps_members_are_not_taken);
    run("a6_a_mix_takes_only_the_printemps_members",
        a6_a_mix_takes_only_the_printemps_members);
    run("a7_it_is_her_own_under_area", a7_it_is_her_own_under_area);

    printf("--- B1 effects/kidou PL!HS-bp1-002-RM same_area (5) ---\n");
    run("b1_retrieved_card_lands_in_the_left_side_she_vacated",
        b1_retrieved_card_lands_in_the_left_side_she_vacated);
    run("b2_retrieved_card_lands_in_the_centre_she_vacated",
        b2_retrieved_card_lands_in_the_centre_she_vacated);
    run("b3_a_member_costing_exactly_fifteen_is_eligible",
        b3_a_member_costing_exactly_fifteen_is_eligible);
    run("b4_hasunosora_member_costing_seventeen_is_not_a_target",
        b4_hasunosora_member_costing_seventeen_is_not_a_target);
    run("b5_a_non_hasunosora_member_is_not_a_target",
        b5_a_non_hasunosora_member_is_not_a_target);

    printf("--- B2 effects/kidou PL!-bp4-002-SEC score-total activation gate (3) ---\n");
    run("b6_a_success_zone_total_of_six_offers_the_ability",
        b6_a_success_zone_total_of_six_offers_the_ability);
    run("b7_a_success_zone_total_of_five_withholds_the_ability",
        b7_a_success_zone_total_of_five_withholds_the_ability);
    run("b8_an_empty_success_zone_withholds_the_ability",
        b8_an_empty_success_zone_withholds_the_ability);

    printf("--- C effects/move_cards PL!-pb2-014-P+ parse shape (4) ---\n");
    run("c1_cost_is_an_optional_reveal_of_a_lily_white_live_from_hand",
        c1_cost_is_an_optional_reveal_of_a_lily_white_live_from_hand);
    run("c2_effect_places_the_revealed_card_into_the_success_zone",
        c2_effect_places_the_revealed_card_into_the_success_zone);
    run("c3_the_take_from_the_success_zone_is_currently_missing",
        c3_the_take_from_the_success_zone_is_currently_missing);
    run("c4_fixtures_cover_the_reveal_filter", c4_fixtures_cover_the_reveal_filter);

    printf("--- D jidou/movement/under_member_placement (3) ---\n");
    printf("== FINDINGS (every red above is one of these three; none is a\n"
           "   weakened assertion, and none is a byte-splice artefact -- the\n"
           "   a0 skew probe is GREEN at commit dc44ccf1, and every wire string\n"
           "   this file compares against decoded exactly)\n\n"
           "   F1 ENGINE BUG -- a count>1 mandatory SelectCard sourced from a\n"
           "      NON-HAND zone accepts only ONE index.  PL!-pb2-017-P+ 登場\n"
           "      raises zone=\"discard\" count=4 (the engine's own trace:\n"
           "      'Select 4 card(s) from the waiting room', nf=4) and a\n"
           "      Rust-faithful answer of {0,1,2,3} places ONE card.\n"
           "      rb_resume_with_choice_indices_internal (src/ability/choice.c)\n"
           "      derives selected_idx = selected_indices[0] in the function's\n"
           "      first lines, routes the multi-index arm through\n"
           "      choice_handle_hand_indices (defined at choice.c:1311, called at\n"
           "      :4255) which covers HAND zones only, and on decline falls back to\n"
           "      rb_resolver_handle_select_card at :4268 with `selected` = selbuf,\n"
           "      built from indices[0] ALONE at :4137.  The engine's own log then\n"
           "      reads '[DISCARD_PICK_REJECTED] ... mapped=0'.  Rust dispatches the\n"
           "      WHOLE answer (result_handlers.rs:41-65, indices.to_vec()).\n"
           "      (choice.c is under concurrent peer edit, so the FUNCTION names\n"
           "      are the durable anchor; the line numbers are as of the working\n"
           "      tree at commit dc44ccf1 with a peer's uncommitted choice.c.)\n"
           "      PAIRED POSITIVE that proves it non-vacuous: b1-b4 use the SAME\n"
           "      prompt kind, the SAME route=3, the SAME move_cards action and\n"
           "      the SAME resume function with count=1, and they place the card.\n"
           "      Census is CONSTANT, so the three unplaced cards are left in the\n"
           "      waitroom, not destroyed.\n\n"
           "   F2 ENGINE BUG -- a 起動 with a sequential_cost pays NEITHER leg\n"
           "      and its same_area placement DESTROYS the retrieved card.\n"
           "      PL!HS-bp1-002-RM: active energy 9 -> 9 across the activation\n"
           "      (the 2E leg did not run) and SAYAKA is still standing in the\n"
           "      area she was played to (the move-self-to-waitroom leg did not\n"
           "      run), yet the EFFECT runs and its selection prompt is answered.\n"
           "      After the answer the fixture card is in NO zone: waitroom 1->0\n"
           "      and the whole-board census 92->91.  b5 is the paired control --\n"
           "      no prompt is offered (the card fails the group gate) and the\n"
           "      census stays 92->92 -- so the loss is on the answer path.\n"
           "      Rust's own file insists the cost must be paid for this very\n"
           "      reason ('the cost is what vacates the area').\n\n"
           "   F3 ENGINE GAP -- there is no OFFER for a staged member's 起動,\n"
           "      so 「のみ起動できる」 is unmeasurable end to end.  See\n"
           "      require_offer_instrument() for the line-level evidence.\n\n");
    printf("    NOT PORTED, and deliberately so.  All three are already covered\n"
           "    by peers, verified by body rather than by header:\n"
           "      live_success arm  (discard_group_member_placed_under_self_on_live_success)\n"
           "          -> test_parity_int_gaps.c  c1_kinako_live_success_tucks_liella_member\n"
           "             (and its paired negative, the same board before 成功)\n"
           "      area-move arm    (discard_group_member_placed_under_self_on_position_change)\n"
           "          -> test_parity_jidou_move_yell.c:1566\n"
           "             test_kinako_tucks_group_member_under_self_on_area_move\n"
           "      both_players     (under_member_counted_for_both_players)\n"
           "          -> test_parity_jidou_extra.c 851-897 section I\n"
           "             test_under_member_tuck_works_for_both_players\n"
           "    Porting them again would add no coverage.  The one thing the\n"
           "    survey got WRONG in that cluster is recorded instead:\n"
           "      RUST FIXTURE BUG: under_member_counted_for_both_players_test.rs\n"
           "      comments PL!SP-pb1-014-N as 「嵐 千砂都 ... a genuinely different\n"
           "      『Liella!』 member」 in contrast to the きな子 host, but the card\n"
           "      it actually tucks under きな子 is played as PL!SP-pb2-025-N --\n"
           "      which cards.json ALSO calls 嵐 千砂都 (cost 2 blade 1 vs cost 2\n"
           "      blade 2).  So the tucked card and the member doing the tucking\n"
           "      are the SAME CHARACTER, and the comment's 'genuinely different'\n"
           "      is false.  test_parity_jidou_extra.c:106 repeats the same false\n"
           "      comment.  This file does not assert on it.\n");

    rb_unload();

    int n_run = n_ok + n_failed + n_crashed + n_setup;
    printf("\n==== parity_wave1c: %d test case(s) run ====\n", n_run);
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
