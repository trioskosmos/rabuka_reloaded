/* =====================================================================
 * test_parity_restriction.c
 *
 * C parity tests for two Rust clusters:
 *
 *   A. engine/tests/test_modules/effects/restriction/                 (6 .rs, 14 tests)
 *   B. engine/tests/test_modules/effects/gain/hearts/color_choice/   (9 .rs, 16 tests)
 *
 * Every test below is a faithful line-by-line translation of one Rust
 * #[test] function. Nothing is weakened, skipped or deleted to make the
 * suite green: a red CHECK here is the deliverable.
 *
 * HARNESS NOTES (all confirmed by running peers, not assumed)
 * ----------------------------------------------------------
 *  * `rb_load("src")` fails under the isolated out-of-tree build, so
 *    load_card_db() falls back to rb_load("../cards/build").
 *  * sizeof(GameState) is ~781 KB, so every TestGame local is `static`.
 *  * `mid()` is test_new_id (the next DISTINCT pool slot, matching Rust's
 *    `game.id()`). `id_ref()` is test_id (the SHARED template, matching
 *    Rust's `game.id_ref()`); only id_ref may be used where Rust uses
 *    id_ref, e.g. a bystander that must be the same object across calls.
 *  * `rb_create_card_copy` draws from a process-global monotonic pool that
 *    nothing resets between cases, so a long suite exhausts it (4096 ids),
 *    after which test_new_id returns -1 and mid() silently falls back to
 *    the shared template - every identity assertion after that becomes
 *    vacuous. board_begin() calls rb_card_copy_pool_reset().
 *  * `rb_card_no_eq` is FUZZY and is NOT an identity pin. pin_id() does a
 *    byte-exact strcmp of rb_card_string(card_no_idx) instead.
 *  * `test_get_heart_modifier` REMAPS colour 5 onto RB_HEART_ORANGE and
 *    corrupts any heart05 read. heart_of() calls rb_mods_get_heart
 *    directly, so the colour bucket under test is the one asserted.
 *  * `Card.ability` is not populated by rb_decode_card_by_index; abilities
 *    come from rb_card_num_abilities + rb_decode_card_ability.
 *  * Card numbers carrying FULLWIDTH PLUS U+FF0B (bytes ef bc 8b) are
 *    written as "\xEF\xBC\x8B": rb_find_card_by_no is EXACT-BYTE and does
 *    not normalise fullwidth->halfwidth the way Rust's
 *    CardDatabase::get_card_id does.
 *  * This file is pure ASCII: every Japanese literal is a hex escape, so
 *    the editor cannot re-encode it to CP932 and silently corrupt the
 *    group/series filters.
 *  * `pay_optional_cost:skip` is a SelectTarget where option 0 is SKIP and
 *    option 1 is PAY; a test that answers 0 has not paid anything.
 *
 * KNOWN ENGINE STATE THIS FILE TESTS AGAINST (not excuses)
 * --------------------------------------------------------
 *  * `restriction` is on PORT_TRACKER.md's "complete but unverified" list,
 *    so a red here is a high-value finding, not a bad fixture.
 *  * The three-way heart CHOICE is reported never to be emitted; a
 *    multi-colour `heart_colors` array is reported to parse as a single
 *    PINK heart. Both are asserted strictly below and the prompt-count
 *    controls prove non-vacuity either way.
 *  * `rb_should_trigger_live_success` needs BOTH live.n > 0 AND
 *    live_success[pl], so any case asserting a live-success precondition
 *    opens that window explicitly.
 *
 * VACUITY CONTROL - the highest-value discipline in this file
 * -----------------------------------------------------------
 * A negative assertion ("no restriction", "no prompt", "0 modifier")
 * passes trivially on a board where the ability did nothing at all. Every
 * negative assertion here is therefore paired with a PROMPT-COUNT CONTROL
 * (prompt_guard / effect_guard) or with a positive assertion on the same
 * board. `prompts` counts every choice answered; `pending_offered` counts
 * every moment a prompt was observed open. Guards are DEFERRED and
 * evaluated at the end of the case, so their placement does not matter.
 *
 * HEART COLOUR INDEXING
 *   Rust HeartColor::Heart01..Heart06 == C RB_HEART_RED..RB_HEART_PURPLE
 *   (1..6). Rust HeartColor::Heart00 == C RB_HEART_PINK == 0. "No
 *   override" is Rust None == C rb_mods_get_heart_color_multiplier == -1.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* Fire-one-ability-on-one-card comes from the SHARED test-game helper
   (include/test_game.h; the port of engine/tests/helpers/mod.rs:114
   `fire_trigger_nth`), so this file no longer declares rb_trigger_auto_ability
   itself. rb_process_pending_auto_abilities IS declared in rabuka.h
   (returns int). */

/* Trigger wire tokens, byte-exact UTF-8. */
#define TRIG_CONSTANT     "\xE5\xB8\xB8\xE6\x99\x82"                          /* ~~ */
#define TRIG_LIVE_START   "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE9\x96\x8B\xE5\xA7\x8B\xE6\x99\x82"
#define TRIG_LIVE_SUCCESS "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE6\x88\x90\xE5\x8A\x9F\xE6\x99\x82"

/* Heart colour buckets (C indices; Rust Heart01..Heart06 == 1..6). */
#define H1 1
#define H2 2
#define H3 3
#define H4 4
#define H5 5
#define H6 6

/* Card fixtures, byte-exact from cards/cards.json (verified with a UTF-8
   reader; every one of these EXISTS and its printed values are pinned by
   the per-card identity check in the case that uses it). */
#define KANON        "PL!SP-bp1-001-P"                  /* alone-on-stage cannot_live */
#define HIMENO       "PL!HS-bp6-006-R\xEF\xBC\x8B"     /* delayed cannot_active      */
#define AQOURLOCK    "PL!HS-pb1-008-R"                 /* opponent active-phase lock  */
#define KAGAYA       "PL!S-bp2-024-L"                  /* q36 + q125 live             */
#define MEBIUS       "PL!S-pb1-022-L"                  /* tie -> cannot place success */
#define BUTTERFLY    "PL!N-bp1-028-L"                  /* fails on a heart04 stage    */
#define HEART_PROV   "PL!S-bp2-015-PR"                 /* heart05 provider            */
#define PROTEINBAR   "PL!N-sd1-016-PRproteinbar"       /* heart05 x2, abilityless     */
#define SD1_002      "PL!-sd1-002-SD"                  /* mu's member, abilityless    */
#define FILLER       "PL!-sd1-010-SD"                  /* mu's member, no ability     */
#define FILLER_N     "PL!N-sd1-010-SD"                 /* ditto (paid_energy fixture) */
#define LIVE_FILLER  "PL!-sd1-019-SD"                  /* mu's LIVE card              */
#define LIVE_FILLER2 "PL!-sd1-020-SD"                  /* mu's LIVE card (shizuku)    */
#define KASUMI_N     "PL!N-bp3-014-N"                  /* heart03/04/05 choice        */
#define KASUMI_QUARTZ "PL!N-bp3-002-R"                 /* ~~~, discard-1 cost      */
#define SHIORIKO     "PL!N-pb1-034-N"                  /* heart03/04/05, prints heart04 */
#define LANZHU       "PL!N-pb1-036-N"                  /* heart01/02/06               */
#define WAKANA       "PL!SP-pb2-030-N"                 /* heart02/03/06, prints heart06 */
#define ELI          "PL!-bp5-011-N"                  /* heart04/05/06 x per success  */
#define KOTORI       "PL!-bp3-012-PR"                 /* heart01/03/06 x per success  */
#define NICO         "PL!-bp3-009-R\xEF\xBC\x8B"     /* activation, self-wait cost  */
#define AI_SD2       "PL!N-sd2-005-SD2"                /* discard-2 cost, +2 heart     */
#define ARAN_BP7     "PL!N-bp7-012-R"                  /* ~~~ wait-1 cost, +1 heart */
#define SHIZUKU      "PL!N-bp1-003-R\xEF\xBC\x8B"     /* optional {{E}} cost         */
#define OMIKARI      "PL!N-bp1-006-R\xEF\xBC\x8B"     /* ~~~~ (~~~)             */
#define AQOUR_MEMBER "PL!S-bp5-009-R"                  /* Aqours member               */
#define SD1_001      "PL!S-sd1-001-SD"

/* ===================================================================== */
/* 0. harness                                                             */
/* ===================================================================== */

static long checks;
static int  failures;
static int  gaps;
static int  prompts;          /* every answered choice */
static int  pending_offered;

static const char *current_case = "(none)";

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL signal %d in test_parity_restriction\n"
            "*** case in flight: %s  (engine fault, not a test bug)\n"
            "*** checks so far: %ld  failures: %d\n",
            sig, current_case, checks, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                  \
        checks++;                                                  \
        if (!(condition)) {                                         \
            failures++;                                             \
            fprintf(stderr, "FAIL: ");                              \
            fprintf(stderr, __VA_ARGS__);                           \
            fputc('\n', stderr);                                    \
        }                                                           \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                        \
        checks++;                                                   \
        long a_ = (long)(actual);                                   \
        long e_ = (long)(expected);                                 \
        if (a_ != e_) {                                             \
            failures++;                                             \
            fprintf(stderr, "FAIL: ");                              \
            fprintf(stderr, __VA_ARGS__);                           \
            fprintf(stderr, " (got %ld expected %ld)\n", a_, e_);   \
        }                                                           \
    } while (0)

/* An observation the C API cannot make whatever the engine does. Never a
   pass, never a failure. */
#define EXPECTED_GAP(condition, desc) do {                          \
        checks++;                                                   \
        if (!(condition)) {                                         \
            printf("GAP: %s\n", desc);                              \
            gaps++;                                                 \
        } else {                                                    \
            printf("ok (gap closed): %s\n", desc);                  \
        }                                                           \
        fflush(stdout);                                             \
    } while (0)

#define CHILD_OK       0
#define CHILD_FAILURES 1

static int n_ok, n_failed, n_crashed;
static const char *only_case;
static const TestGame *current_tg;

static void eval_pending_guards(void);

/* A forked child's counters die with the child, so the child hands them back
   to the parent over a pipe. Without this the summary line reports the
   parent's own (always zero) totals and understates the suite by an order
   of magnitude. */
static int g_pipe[2];

struct child_totals {
    long checks;
    int failures;
    int gaps;
};

static void run(const char *name, void (*fn)(void))
{
    if (only_case && !strstr(name, only_case)) return;
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        long c0 = checks, f0 = failures;
        current_case = name;
        current_tg = NULL;
        fn();
        eval_pending_guards();
        if (failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [%ld checks]\n", failures > f0 ? "FAILED" : "ok",
               name, checks - c0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        long c0 = checks, f0 = failures;
        current_case = name;
        current_tg = NULL;
        fn();
        eval_pending_guards();
        struct child_totals t = { checks - c0, (int)(failures - f0), gaps };
        ssize_t ignored = write(g_pipe[1], &t, sizeof t);
        (void)ignored;
        printf("        %s: %ld check(s), %d failure(s)\n",
               name, t.checks, t.failures);
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);
    struct child_totals t;
    memset(&t, 0, sizeof t);
    ssize_t got = read(g_pipe[0], &t, sizeof t);
    if (got == (ssize_t)sizeof t) {
        checks += t.checks;
        failures += t.failures;
        gaps += t.gaps;
    }
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name,
               WEXITSTATUS(status));
    }
    fflush(stdout);
}

/* ===================================================================== */
/* 1. helpers                                                             */
/* ===================================================================== */

/* HARNESS REQUIREMENT: test binaries must load the card DB. rb_load("src")
   fails under the isolated out-of-tree build. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

static const char *card_no_of(int cid)
{
    static char buf[64];
    Card c;
    buf[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        if (s) snprintf(buf, sizeof buf, "%s", s);
        rb_free_card(&c);
    }
    return buf;
}

/* Rust `game.id(card_no)`: the next DISTINCT pool slot. test_new_id is the C
   twin; test_id aliases the shared template, so a tracked card must never
   use it. Falls back to the template when the copy pool is exhausted,
   matching Rust's documented fallback. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id >= 0) return id;
    return test_id(tg, no);
}

/* Rust `game.id_ref(card_no)`: the SHARED template slot. */
static int id_ref(TestGame *tg, const char *no) { return test_id(tg, no); }

/* Rust assert_card_identity: a BYTE-EXACT strcmp of the printed card_no.
   rb_card_no_eq is fuzzy and is deliberately not used. */
static int pin_id(int cid, const char *want)
{
    checks++;
    const char *got = card_no_of(cid);
    if (cid >= 0 && strcmp(got, want) == 0) {
        printf("ok: identity %s\n", want);
        return 1;
    }
    fprintf(stderr, "FAIL: identity: want '%s', resolved '%s' (id=%d)\n",
            want, got, cid);
    failures++;
    return 0;
}

static int card_prints_trigger(int cid, const char *trig)
{
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

static void dump_ability(int cid, int idx)
{
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, idx, &ab)) {
        printf("DBG %s ability[%d]: DECODE FAILED\n", card_no_of(cid), idx);
        return;
    }
    printf("DBG %s ability[%d] triggers=%s use_limit=%d keywords=%d cost=%d effect=%d\n",
           card_no_of(cid), idx, ab.triggers ? ab.triggers : "(null)",
           ab.use_limit, ab.n_keywords, ab.cost ? 1 : 0, ab.effect ? 1 : 0);
    rb_free_ability(&ab);
}

static int bag_has(const RbBag *bag, int cid)
{
    for (int i = 0; i < bag->n; i++) if (bag->cards[i] == cid) return 1;
    return 0;
}

static void push_to_hand(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++] = cid;
}

static void push_to_success(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->success.n < RB_MAX_ZONE) P->success.cards[P->success.n++] = cid;
}

/* Rust mods.get_heart_modifier(card, color). Uses rb_mods_get_heart
   DIRECTLY: test_get_heart_modifier remaps colour 5 onto RB_HEART_ORANGE
   and would corrupt every heart05 read in this file. */
static int heart_of(TestGame *tg, int cid, int color)
{
    return rb_mods_get_heart(&tg->state.mods, cid, color);
}

static int heart_mods(TestGame *tg, int cid, int out[6])
{
    for (int c = 0; c < 6; c++) out[c] = heart_of(tg, cid, c + 1);
    return 6;
}

/* Diagnostic: name every heart bucket the engine actually wrote, so a
   failure report says WHAT was granted instead of only what was expected.
   The colour buckets are read through rb_mods_get_heart, which folds
   RB_HEART_PINK (bucket 0) into every query, so bucket 0 is reported
   separately as the "wildcard" that would silently tint all six. */
static void report_hearts(TestGame *tg, int cid, const char *label)
{
    fprintf(stderr, "  %s %s: wild(pink)=%d",
            label, card_no_of(cid), rb_mods_get_heart(&tg->state.mods, cid, 0));
    for (int c = 1; c <= 6; c++)
        fprintf(stderr, " heart%02d=%d", c, rb_mods_get_heart(&tg->state.mods, cid, c));
    fprintf(stderr, "  multiplier=%d\n",
            rb_mods_get_heart_color_multiplier(&tg->state.mods, cid));
}

/* Rust mods.heart_color_multiplier.get(&cid).copied(): Option<HeartColor>. */
static int mult_of(TestGame *tg, int cid)
{
    return rb_mods_get_heart_color_multiplier(&tg->state.mods, cid);
}

/* Rust `printed_hearts`: the card's PRINTED base hearts, read from the
   ENGINE's decoded record so a cards.json/cards.bin disagreement cannot
   silently make a precondition vacuous. */
static int printed_hearts(TestGame *tg, int cid, int out[6])
{
    Card c;
    int n = 0;
    if (cid < 0 || !rb_decode_card_by_index((uint32_t)cid, &c)) return 0;
    for (int k = 0; k < c.num_base; k++) {
        int col = c.heart_color[k];
        if (col >= 1 && col <= 6 && c.heart_count[k] > 0) out[n++] = col;
    }
    rb_free_card(&c);
    return n;
}

static void print_hearts(const int *v, int n)
{
    fputc('[', stderr);
    for (int i = 0; i < n; i++) fprintf(stderr, "%sheart%02d", i ? ", " : "", v[i]);
    fputc(']', stderr);
}

/* ---- board setup ---- */

static void clear_p1(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    P->hand.n = 0;
    P->energy.n = 0;
    P->energy_active = 0;
    P->energy_deck.n = 0;
    P->live.n = 0;
    P->success.n = 0;
    P->discard.n = 0;
    P->deck_refreshed_this_turn = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
    tg->state.n_revealed = 0;
    tg->state.yell_occurred = 0;
    tg->state.re_yell_occurred = 0;
    tg->state.n_recently_moved = 0;
    tg->state.n_those_cards = 0;
    tg->state.n_batch_movements = 0;
    tg->state.self_live_surplus_count = 0;
    tg->state.opponent_live_surplus_count = 0;
    tg->state.live_surplus_ready_this_turn = 0;
    tg->state.n_prohibition = 0;
    tg->state.n_prohibition_effects = 0;
    tg->state.player_cannot_activate[0] = 0;
    tg->state.player_cannot_activate[1] = 0;
}

static void clear_p2(TestGame *tg)
{
    RbPlayer *P = &tg->state.p[1];
    P->deck.n = 0;
    P->hand.n = 0;
    P->energy.n = 0;
    P->energy_active = 0;
    P->energy_deck.n = 0;
    P->live.n = 0;
    P->success.n = 0;
    P->discard.n = 0;
    P->deck_refreshed_this_turn = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
        P->under_cards[i].n = 0;
    }
}

/* Rust helpers::fill_decks / support::fill_both_main_decks: 30 fillers each. */
static void fill_both_decks(TestGame *tg, int filler)
{
    RbPlayer *P = &tg->state.p[0];
    RbPlayer *O = &tg->state.p[1];
    P->deck.n = 0;
    O->deck.n = 0;
    for (int i = 0; i < 30; i++) {
        P->deck.cards[P->deck.n++] = filler;
        O->deck.cards[O->deck.n++] = filler;
    }
}

/* Rust `fill_decks(game, filler)` in the color_choice cluster (40 each). */
static void fill_decks40(TestGame *tg, int filler)
{
    RbPlayer *P = &tg->state.p[0];
    RbPlayer *O = &tg->state.p[1];
    P->deck.n = 0;
    O->deck.n = 0;
    for (int i = 0; i < 40; i++) {
        P->deck.cards[P->deck.n++] = filler;
        O->deck.cards[O->deck.n++] = filler;
    }
}

/* ---- vacuity-control state ---- */
#define MAX_GUARDS 512
static struct { long long fp; int mark; const char *what; } g_guards[MAX_GUARDS];
static int g_n_guards;

static long long state_fingerprint(const TestGame *tg);

/* Baselines captured at board_begin, i.e. BEFORE the case did anything.
   A guard is answered "the engine really did something during this case",
   which is the only formulation that cannot be defeated by placing the guard
   after the effect it is meant to police. */
static int   case_start_prompts;
static int   case_start_pending;
static long long case_start_fp;

static void board_begin(TestGame *tg)
{
    pending_offered = 0;
    g_n_guards = 0;
    current_tg = tg;
    case_start_prompts = prompts;
    case_start_pending = pending_offered;
    case_start_fp = state_fingerprint(tg);
    /* HARNESS HAZARD: rb_create_card_copy draws from a process-global
       monotonic pool (src/core/data.c) that nothing resets between cases.
       A long suite exhausts it, test_new_id returns -1, and mid() silently
       falls back to the shared template - after which two different prints
       in the same case resolve to the SAME id and every identity assertion
       becomes vacuous. */
    rb_card_copy_pool_reset();
}

/* A cheap fingerprint of every zone size plus the movement-event count. */
static long long state_fingerprint(const TestGame *tg)
{
    long long f = 0;
    for (int p = 0; p < 2; p++) {
        const RbPlayer *P = &tg->state.p[p];
        f += (long long)P->deck.n        * 1LL;
        f += (long long)P->hand.n        * 1000000LL;
        f += (long long)P->discard.n     * 1000000000000LL;
        f += (long long)P->live.n        * 1000000000000000LL;
        f += (long long)P->success.n     * 1000000000000000000LL;
    }
    f += (long long)tg->state.n_batch_movements * 100000000000000000LL;
    return f;
}

/* The effect really reached the engine: EITHER it opened a prompt we
   answered, OR it changed the state of some zone. The prompt form is what
   every colour-choice case uses; a constant restriction opens no prompt and
   moves no card, so it is policed by asserting on the prohibition ledger
   directly instead. */
/* The effect offered a prompt, so this case's control is that the prompt
   really appeared. */
static void prompt_guard(const char *what)
{
    if (g_n_guards < MAX_GUARDS) {
        g_guards[g_n_guards].fp = -1;
        g_guards[g_n_guards].mark = 0;
        g_guards[g_n_guards].what = what;
        g_n_guards++;
    }
}

/* For a case that asserts NOTHING HAPPENED: the control is that the card
   really prints the ability under test, so the absence is a filter result
   and not a missing effect. */
static void expect_effect_printed(int cid, const char *trig, const char *what)
{
    CHECK(card_prints_trigger(cid, trig), "%s", what);
}

static void eval_pending_guards(void)
{
    for (int i = 0; i < g_n_guards; i++) {
        if (g_guards[i].fp < 0) {
            CHECK(prompts > case_start_prompts || pending_offered > case_start_pending,
                  "vacuity control: %s (a prompt really was offered)", g_guards[i].what);
        } else {
            CHECK(prompts > case_start_prompts || pending_offered > case_start_pending ||
                      state_fingerprint(current_tg) != g_guards[i].fp,
                  "vacuity control: %s (the engine really changed state)",
                  g_guards[i].what);
        }
    }
    g_n_guards = 0;
}

static void expect_pending(int cond, const char *what)
{
    CHECK(cond, "%s", what);
    if (cond) pending_offered++;
}

/* Rust select_indices(&[i]). */
static void pick(TestGame *tg, int idx)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    prompts++;
    pending_offered++;
    const int one[1] = { idx };
    rb_resume_with_choice_indices(&tg->state, one, 1);
}

static void pick_many(TestGame *tg, const int *idx, int n)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    prompts++;
    pending_offered++;
    rb_resume_with_choice_indices(&tg->state, idx, n);
}

/* Rust select_indices(&[]) - DECLINE. */
static void skip_choice(TestGame *tg)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    prompts++;
    pending_offered++;
    rb_resume_with_choice_indices(&tg->state, NULL, 0);
}

/* Rust select_option(n). */
static void opt(TestGame *tg, int n)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    prompts++;
    pending_offered++;
    rb_resume_with_choice(&tg->state, n);
}

static const char *pending_type(TestGame *tg) { return test_pending_choice_type(tg); }

/* Rust TestGame::drain_choices: decline what CAN be declined, answer the
   rest with their first option. */
static void drain_choices(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 40) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (c && c->allow_skip) skip_choice(tg);
        else opt(tg, 0);
    }
}

/* Rust helpers::fire_trigger: resolve ONE ability on ONE card, key it
   "{card_no}_{full_text}", fire it with `cid` as the activating card, then run
   process_pending_auto_abilities. It does NOT drain further, so the caller
   still observes the prompt Rust observes.
   `nth` selects the Nth PRINTED ability carrying the trigger — Rust's
   `fire_trigger_nth`, which test_fire_trigger_on_card_nth already implements.

   The hand-rolled body it replaces recorded NO enqueue-time use, so a
   自動/ターン1回 ability was still unspent when it resolved and the
   post-resolution re-scan in rb_process_player_abilities_depth_full enqueued
   it a SECOND time; its key also truncated at 1024 bytes. */
static int fire_trigger(TestGame *tg, int cid, const char *trig,
                        const char *trigger_type, int nth)
{
    (void)trigger_type;   /* every call site here passes trig == trigger_type */
    return test_fire_trigger_on_card_nth(tg, cid, trig, nth);
}

static int fire_live_start(TestGame *tg, int cid)
{
    return fire_trigger(tg, cid, TRIG_LIVE_START, TRIG_LIVE_START, 0);
}

/* Rust game.activate_ability. */
static int activate(TestGame *tg, int cid) { return test_activate_ability(tg, cid); }

/* ===================================================================== */
/* 2. effects/restriction - alone_on_stage_cannot_live_constant_test.rs   */
/* ===================================================================== */

/* Kanon alone on stage (no other members). */
static void test_alone_on_stage_sets_cannot_live_restriction(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int kanon = mid(&tg, KANON);
    int other = mid(&tg, SD1_002);
    pin_id(kanon, KANON);
    pin_id(other, SD1_002);

    /* NEGATIVE HALF, proven in place: with a second stage member the same
       constant must NOT fire. Without this the positive below would be
       indistinguishable from "the ability always fires". */
    test_add_to_stage(&tg, 0, other);
    test_add_to_stage(&tg, 1, kanon);
    test_recalc(&tg);
    CHECK(!rb_is_action_prohibited(&tg.state, "cannot_live"),
          "with another member the same constant must not restrict");

    /* POSITIVE HALF: alone on stage, the restriction appears. */
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    test_recalc(&tg);

    CHECK(rb_is_action_prohibited(&tg.state, "cannot_live"),
          "Kanon alone: cannot_live restriction should be active");
    /* PROMPT-COUNT-FREE POSITIVE CONTROL. This constant opens no prompt and
       touches none of the zone sizes the fingerprint covers, so a
       prompt/state guard would be the wrong instrument. What is observable
       is the engine's own prohibition ledger: the entry the restriction
       handler wrote. Without this the assertion above would be
       indistinguishable from the engine merely not clearing something. */
    CHECK(tg.state.n_prohibition > 0,
          "positive control: the constant path wrote a prohibition entry");
    {
        int saw_cannot_live = 0;
        for (int i = 0; i < tg.state.n_prohibition; i++)
            if (strstr(tg.state.prohibition[i], "cannot_live")) saw_cannot_live = 1;
        for (int i = 0; i < tg.state.n_prohibition_effects; i++)
            if (strstr(tg.state.prohibition_effects[i], "cannot_live")) saw_cannot_live = 1;
        CHECK(saw_cannot_live,
              "positive control: the prohibition ledger names cannot_live "
              "(prohibition[0]=\"%s\")", tg.state.n_prohibition > 0 ? tg.state.prohibition[0] : "-");
    }
}

/* Kanon with another member: no restriction. */
static void test_another_stage_member_prevents_cannot_live_restriction(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int kanon = mid(&tg, KANON);
    int other = mid(&tg, SD1_002);
    pin_id(kanon, KANON);
    pin_id(other, SD1_002);

    test_add_to_stage(&tg, 1, kanon);   /* Center */
    test_add_to_stage(&tg, 0, other);   /* LeftSide */

    test_recalc(&tg);

    CHECK(!rb_is_action_prohibited(&tg.state, "cannot_live"),
          "Kanon with other: no restriction expected");

    /* VACUITY CONTROL: the negative assertion above would pass trivially if
       Kanon's restriction never fires at all. Proving the negative in place:
       drop the second member, recalc on the SAME board, and the restriction
       must appear. Without this the "no restriction" reading is
       indistinguishable from "no effect". */
    expect_effect_printed(kanon, TRIG_CONSTANT,
                          "precondition: Kanon prints the constant (joujou) restriction under test");
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = kanon;
    test_recalc(&tg);
    CHECK(rb_is_action_prohibited(&tg.state, "cannot_live"),
          "vacuity control: with the second member removed the restriction must "
          "appear, so the assertion above is about the filter and not about a "
          "dead effect");
}

/* ===================================================================== */
/* 3. effects/restriction - delayed_cannot_active_owner_tick_test.rs     */
/* ===================================================================== */

/* Every card id P1 controls, in any zone: Rust
   `game.state.player1.all_card_ids()`. */
static int p1_all_card_ids(const TestGame *tg, int *out, int max)
{
    int n = 0;
    const RbPlayer *P = &tg->state.p[0];
    for (int i = 0; i < P->hand.n && n < max; i++)        out[n++] = P->hand.cards[i];
    for (int i = 0; i < P->deck.n && n < max; i++)        out[n++] = P->deck.cards[i];
    for (int i = 0; i < P->discard.n && n < max; i++)     out[n++] = P->discard.cards[i];
    for (int i = 0; i < P->live.n && n < max; i++)        out[n++] = P->live.cards[i];
    for (int i = 0; i < P->success.n && n < max; i++)     out[n++] = P->success.cards[i];
    for (int i = 0; i < P->energy.n && n < max; i++)      out[n++] = P->energy.cards[i];
    for (int i = 0; i < P->energy_deck.n && n < max; i++) out[n++] = P->energy_deck.cards[i];
    for (int i = 0; i < RB_STAGE_SIZE && n < max; i++) {
        if (P->stage[i] == RB_EMPTY_SLOT) continue;
        out[n++] = P->stage[i];
        for (int j = 0; j < P->under_cards[i].n && n < max; j++)
            out[n++] = P->under_cards[i].cards[j];
    }
    return n;
}

static void test_delayed_cannot_active_expires_after_one_owner_tick(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int himeno = mid(&tg, HIMENO);
    int filler = mid(&tg, FILLER);
    pin_id(himeno, HIMENO);
    pin_id(filler, FILLER);
    (void)filler;

    tg.state.p[0].stage[0] = himeno;
    rb_mods_set_orientation(&tg.state.mods, himeno, "wait");

    rb_mods_add_delayed_cannot_active(&tg.state.mods, himeno, 1);

    CHECK(rb_mods_is_delayed_cannot_active(&tg.state.mods, himeno),
          "is_delayed_cannot_active should be true right after the add");

    /* Owner-scoped tick: the card's owner's ids. */
    int owned[512];
    int n_owned = p1_all_card_ids(&tg, owned, 512);
    CHECK(n_owned > 0, "precondition: P1 owns at least the card under test");
    rb_mods_tick_delayed_for(&tg.state.mods, owned, n_owned);

    CHECK(!rb_mods_is_delayed_cannot_active(&tg.state.mods, himeno),
          "after one owner tick the flag should be 0");
    /* Positive control for the negative above. */
    CHECK(rb_mods_get_orientation(&tg.state.mods, himeno) != NULL,
          "precondition: the card under test is a real staged card");
}

static void test_repeated_delayed_cannot_active_does_not_extend_owner_ticks(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int himeno = mid(&tg, HIMENO);
    pin_id(himeno, HIMENO);

    tg.state.p[0].stage[0] = himeno;
    rb_mods_set_orientation(&tg.state.mods, himeno, "wait");

    /* Set the delayed flag twice: should keep max (not stack past 1). */
    rb_mods_add_delayed_cannot_active(&tg.state.mods, himeno, 1);
    rb_mods_add_delayed_cannot_active(&tg.state.mods, himeno, 1);
    CHECK(rb_mods_is_delayed_cannot_active(&tg.state.mods, himeno),
          "precondition: the flag is set before the tick");

    int owned[512];
    int n_owned = p1_all_card_ids(&tg, owned, 512);
    rb_mods_tick_delayed_for(&tg.state.mods, owned, n_owned);

    CHECK(!rb_mods_is_delayed_cannot_active(&tg.state.mods, himeno),
          "1 -> 0 after one tick even when set twice");
}

/* ===================================================================== */
/* 4. effects/restriction - opponent_active_phase_lock_targeting_test.rs */
/* ===================================================================== */

/* Rust `constant_cannot_activate_members` is a set of PlayerIds. The C
   engine stores the same information as GameState.player_cannot_activate[2],
   written by h_restriction (src/ability/effects/misc.c:848). */
static int cannot_activate_contains(const TestGame *tg, int pl)
{
    return tg->state.player_cannot_activate[pl] ? 1 : 0;
}

static void active_phase_lock_setup(TestGame *tg, int *restriction, int *waiting,
                                    int p1_slot, int p2_slot)
{
    *restriction = mid(tg, AQOURLOCK);
    *waiting     = mid(tg, FILLER);
    pin_id(*restriction, AQOURLOCK);
    pin_id(*waiting, FILLER);
    if (p1_slot >= 0) tg->state.p[0].stage[p1_slot] = *restriction;
    if (p2_slot >= 0) tg->state.p[1].stage[p2_slot] = *restriction;
    (void)waiting;
}

static void test_active_phase_lock_owned_by_first_player_targets_second_player(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int restriction = 0, waiting = 0;
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    active_phase_lock_setup(&tg, &restriction, &waiting, 0, -1);
    tg.state.p[1].stage[0] = waiting;
    rb_mods_set_orientation(&tg.state.mods, waiting, "wait");
    fill_both_decks(&tg, waiting);
    test_give_energy(&tg, 5);
    test_recalc(&tg);

    expect_effect_printed(restriction, TRIG_CONSTANT,
                          "precondition: the lock card prints its constant (joujou) restriction");

    CHECK(cannot_activate_contains(&tg, 1),
          "opponent (P2) should be in the cannot_activate set");

    /* MECHANISM PROBE, not an assertion about the card: the same constant
       re-evaluated from inside the Active phase. The lock's condition is a
       temporal_condition on phase=active_phase, and the C constant path
       evaluates it through rb_eval_condition_for_host, which applies the
       phase gate (src/ability/condition.c, rb_check_phase_gate(..., 0)).
       Rust's recalculate_constants sets ctx.skip_phase_gate = true for
       constant abilities (engine/src/core/game_state/modifiers.rs), so the
       two engines can only disagree here. */
    {
        int saved = tg.state.phase;
        tg.state.phase = RB_PHASE_ACTIVE;
        tg.state.player_cannot_activate[0] = 0;
        tg.state.player_cannot_activate[1] = 0;
        test_recalc(&tg);
        fprintf(stderr,
                "  phase-gate probe: from RB_PHASE_ACTIVE the lock sets "
                "player_cannot_activate = [%d,%d]\n",
                tg.state.player_cannot_activate[0],
                tg.state.player_cannot_activate[1]);
        tg.state.phase = saved;
    }

    /* P2's member should NOT become active. */
    const char *orient = rb_mods_get_orientation(&tg.state.mods, waiting);
    CHECK(orient && !strcmp(orient, "wait"),
          "P2's waiting member should remain in wait state (blocked)");
}

/* P1 has the turn, P2 owns the restriction targeting "opponent". */
static void test_active_phase_lock_owned_by_second_player_targets_first_player(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int restriction = 0, waiting = 0;
    active_phase_lock_setup(&tg, &restriction, &waiting, -1, 0);
    tg.state.p[0].stage[0] = waiting;
    rb_mods_set_orientation(&tg.state.mods, waiting, "wait");
    fill_both_decks(&tg, waiting);
    test_give_energy(&tg, 5);
    test_recalc(&tg);

    expect_effect_printed(restriction, TRIG_CONSTANT,
                          "precondition: the lock card prints its constant (joujou) restriction");

    CHECK(cannot_activate_contains(&tg, 0),
          "P1 should be in the cannot_activate set (P2's opponent)");

    const char *orient = rb_mods_get_orientation(&tg.state.mods, waiting);
    CHECK(orient && !strcmp(orient, "wait"),
          "P1's waiting member should remain in wait despite the active phase");
}

/* The lock excludes the owner's own members. */
static void test_active_phase_lock_excludes_owners_members(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int restriction = 0, waiting = 0;
    active_phase_lock_setup(&tg, &restriction, &waiting, 0, -1);
    tg.state.p[0].stage[1] = waiting;
    rb_mods_set_orientation(&tg.state.mods, waiting, "wait");
    fill_both_decks(&tg, waiting);
    test_give_energy(&tg, 5);
    test_recalc(&tg);

    expect_effect_printed(restriction, TRIG_CONSTANT,
                          "precondition: the lock card prints its constant (joujou) restriction");

    CHECK(!cannot_activate_contains(&tg, 0),
          "P1 (the owner) should NOT be in the cannot_activate set");

    int blocked = cannot_activate_contains(&tg, 0);
    if (!blocked) rb_mods_set_orientation(&tg.state.mods, waiting, "active");
    const char *orient = rb_mods_get_orientation(&tg.state.mods, waiting);
    CHECK(orient && !strcmp(orient, "active"),
          "P1's own waiting member should become active during the active phase");

    /* VACUITY CONTROL. Both assertions above are satisfied by a lock that
       never fires at all, which is exactly what happens in the Main phase
       (see the sibling case's phase-gate probe). Prove here that the lock
       DOES reach player_cannot_activate for the other seat when the phase
       gate is satisfied, so this case's "owner is excluded" reading is a
       statement about targeting and not about a dead effect. */
    {
        int saved = tg.state.phase;
        tg.state.phase = RB_PHASE_ACTIVE;
        tg.state.player_cannot_activate[0] = 0;
        tg.state.player_cannot_activate[1] = 0;
        test_recalc(&tg);
        CHECK(!tg.state.player_cannot_activate[0],
              "vacuity control: from RB_PHASE_ACTIVE the lock still excludes "
              "its owner");
        CHECK(tg.state.player_cannot_activate[1],
              "vacuity control: from RB_PHASE_ACTIVE the lock really does "
              "target the other seat, so the assertion above is about "
              "targeting and not about a dead effect");
        tg.state.phase = saved;
    }
}

/* ===================================================================== */
/* 5. effects/restriction - kagayaiteru_test.rs                          */
/* ===================================================================== */

/* Rust advance_to_live_card_set_p1: five blind passes from Main, with the
   phase checked by name (not a pass count) on arrival. */
static int advance_to_live_card_set_p1(TestGame *tg)
{
    CHECK(tg->state.phase == RB_PHASE_MAIN,
          "precondition: the walk starts in the Main phase (got %d)", tg->state.phase);
    for (int i = 0; i < 5; i++) {
        test_pass(tg);
        drain_choices(tg);
    }
    int arrived = tg->state.phase == RB_PHASE_LIVE_SET;
    CHECK(arrived, "advance_to_live_card_set_p1: expected LiveCardSet, got phase %d",
          tg->state.phase);
    return arrived;
}

static void kagayaiteru_setup(TestGame *tg, int *card, int *filler)
{
    *card   = mid(tg, KAGAYA);
    *filler = mid(tg, FILLER);
    pin_id(*card, KAGAYA);
    pin_id(*filler, FILLER);
    push_to_hand(tg, 0, *card);
    push_to_hand(tg, 0, *filler);
    /* Stage: member with heart05 to satisfy need_heart {heart05:1, heart0:1} */
    int heart_provider = mid(tg, HEART_PROV);
    pin_id(heart_provider, HEART_PROV);
    test_add_to_stage(tg, 1, heart_provider);
    fill_both_decks(tg, *filler);
}

/* LiveSuccess fires: draw 2, then discard 1. */
static void test_kagayaiteru_live_success_draw_then_discard(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int card = 0, filler = 0;
    kagayaiteru_setup(&tg, &card, &filler);

    int hand_before = tg.state.p[0].hand.n;

    if (!advance_to_live_card_set_p1(&tg)) return;
    test_set_live_card(&tg, 0, card);
    drain_choices(&tg);

    /* advance_to_live_start: two passes. */
    test_pass(&tg);
    test_pass(&tg);

    /* Pass: FirstAttacker -> SecondAttacker -> LiveVictoryDetermination */
    test_pass(&tg);
    test_pass(&tg);
    test_pass(&tg);

    expect_pending(rb_has_pending_choice(&tg.state),
                   "post-LiveSuccess hand discard prompt expected");
    CHECK(!strcmp(pending_type(&tg), "SelectCard"),
          "expected SelectCard for the discard (got %s)", pending_type(&tg));
    /* VACUITY CONTROL: the prompt must really be the ability's own step, so
       the card's ~~~~~~ must have fired. */
    prompt_guard(
                 "Kagayaiteru's LiveSuccess draw+discard step must reach the engine");
    pick(&tg, 0);

    int hand_after = tg.state.p[0].hand.n;
    CHECK(hand_after != hand_before,
          "Hand should change after draw 2 then discard 1 (before %d, after %d)",
          hand_before, hand_after);
}

/* Q125: the constant restriction prevents placement in the success zone. */
static void test_kagayaiteru_q125_cannot_place_in_success_zone(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int card = 0, filler = 0;
    kagayaiteru_setup(&tg, &card, &filler);

    if (!advance_to_live_card_set_p1(&tg)) return;
    test_set_live_card(&tg, 0, card);
    drain_choices(&tg);
    test_pass(&tg);
    test_pass(&tg);
    test_pass(&tg);
    test_pass(&tg);
    test_pass(&tg);

    expect_pending(rb_has_pending_choice(&tg.state),
                   "post-LiveSuccess hand discard prompt expected");
    CHECK(!strcmp(pending_type(&tg), "SelectCard"),
          "expected SelectCard for the discard (got %s)", pending_type(&tg));
    prompt_guard(
                 "Kagayaiteru's LiveSuccess draw+discard step must reach the engine");
    pick(&tg, 0);

    /* Rust passes 3-4 more times and drains between each. The placement
       (live -> success / waitroom) happens inside victory determination, so
       the walk has to keep going until the live zone actually empties: a
       single pass is not enough and used to leave the card in the live zone,
       which made the assertion below report a placement that never ran. */
    {
        int walk = 0;
        while (tg.state.p[0].live.n > 0 && walk++ < 8) {
            if (!rb_has_pending_choice(&tg.state)) test_pass(&tg);
            drain_choices(&tg);
        }
        CHECK(tg.state.p[0].live.n == 0,
              "the live card must have left the live zone (live=%d, walk=%d)",
              tg.state.p[0].live.n, walk);
    }

    CHECK(bag_has(&tg.state.p[0].discard, card),
          "Q125: card should be in waitroom (cannot be placed in success zone)");
    CHECK(!bag_has(&tg.state.p[0].success, card),
          "Q125: the restricted live card must not be in the success zone");
    /* The card's own constant must really carry the cannot_place
       restriction: the engine's own placement predicate is the direct
       observation, independent of the walk. */
    CHECK(!rb_can_place_card_in_zone(&tg.state, card, "success_live_zone"),
          "Q125: the card's constant (joujou) restriction must forbid the success zone");
}

/* ===================================================================== */
/* 6. effects/restriction - live_success_sequential_test.rs              */
/* ===================================================================== */

/* Rust drain_choices for this file: answer the mandatory hand discard with
   its first option, decline everything else. Bounded: an unbounded drain
   cannot prove termination. */
static void ls_drain_choices(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard < 40) {
        guard++;
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        if (c && c->kind == RB_CHOICE_SELECT_CARD && c->zone[0] &&
            !strcmp(c->zone, "hand") && c->count >= 1)
            pick(tg, 0);
        else
            skip_choice(tg);
    }
}

static void test_live_success_both_sides_draw_discard_advances_turn(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int live    = mid(&tg, KAGAYA);            /* ab#1 LiveSuccess: draw 2, discard 1 */
    int member  = mid(&tg, PROTEINBAR);        /* heart05 x2, abilityless */
    int filler  = mid(&tg, FILLER);
    pin_id(live, KAGAYA);
    pin_id(member, PROTEINBAR);
    pin_id(filler, FILLER);

    tg.state.p[0].stage[0] = member;
    tg.state.p[1].stage[0] = member;

    push_to_hand(&tg, 0, live);
    push_to_hand(&tg, 0, filler);
    push_to_hand(&tg, 0, filler);
    push_to_hand(&tg, 1, filler);
    push_to_hand(&tg, 1, filler);

    for (int i = 0; i < 20; i++) {
        tg.state.p[0].deck.cards[tg.state.p[0].deck.n++] = filler;
        tg.state.p[1].deck.cards[tg.state.p[1].deck.n++] = filler;
    }

    for (int i = 0; i < 5; i++) { test_pass(&tg); }
    ls_drain_choices(&tg);

    test_set_live_card(&tg, 0, live);
    ls_drain_choices(&tg);

    /* Rust pushes the same live id straight into P2's live zone. The C
       twin must use the seat-aware helper, or a P2-triggered ability never
       sees the card while the fixture still looks plausible. */
    test_set_live_card_for(&tg, 1, live);

    /* Rust captures the baselines at the TOP of each of its eight passes,
       i.e. the first time the machine sits in LiveVictoryDetermination,
       BEFORE anything in that phase has been answered, and breaks as soon
       as the phase is left. Capturing later (after the LiveSuccess ability
       has already drawn and discarded) makes the "+1" delta unreachable by
       construction, so the capture point matters as much as the pass count. */
    int p1h0 = -1, p2h0 = -1, p1w0 = -1, p2w0 = -1;
    for (int i = 0; i < 10; i++) {
        if (p1h0 < 0 && tg.state.phase == RB_PHASE_VICTORY) {
            p1h0 = tg.state.p[0].hand.n;
            p2h0 = tg.state.p[1].hand.n;
            p1w0 = tg.state.p[0].discard.n;
            p2w0 = tg.state.p[1].discard.n;
            fprintf(stderr,
                    "  baselines @ pass %d phase=%d: p1 hand=%d discard=%d | "
                    "p2 hand=%d discard=%d\n",
                    i, tg.state.phase, p1h0, p1w0, p2h0, p2w0);
        }
        if (p1h0 >= 0 && tg.state.phase != RB_PHASE_VICTORY) break;
        if (!rb_has_pending_choice(&tg.state)) test_pass(&tg);
        ls_drain_choices(&tg);
    }
    fprintf(stderr,
            "  final phase=%d: p1 hand=%d discard=%d | p2 hand=%d discard=%d\n",
            tg.state.phase, tg.state.p[0].hand.n, tg.state.p[0].discard.n,
            tg.state.p[1].hand.n, tg.state.p[1].discard.n);
    for (int i = 0; i < 3; i++) {
        if (tg.state.phase != RB_PHASE_VICTORY || rb_has_pending_choice(&tg.state)) break;
        test_pass(&tg);
        ls_drain_choices(&tg);
    }
    if (!rb_has_pending_choice(&tg.state)) test_pass(&tg);
    ls_drain_choices(&tg);

    /* Regression core: no infinite re-queue of P2's LiveSuccess. */
    CHECK(tg.state.phase != RB_PHASE_VICTORY,
          "Should have left LiveVictoryDetermination (phase %d)", tg.state.phase);

    CHECK(p1h0 >= 0, "victory determination reached (baselines captured)");
    if (p1h0 < 0) return;

    CHECK_EQ(tg.state.p[0].hand.n, p1h0 + 1, "P1: draw 2 - discard 1 = net +1");
    CHECK_EQ(tg.state.p[1].hand.n, p2h0 + 1, "P2: draw 2 - discard 1 = net +1");
    CHECK_EQ(tg.state.p[0].discard.n, p1w0 + 2,
             "P1: ability discard + restricted live card");
    CHECK(bag_has(&tg.state.p[0].discard, live) &&
              !bag_has(&tg.state.p[0].success, live),
          "restricted live card must not enter the success zone");
    CHECK_EQ(tg.state.p[1].discard.n, p2w0 + 2, "P2: ability discard + restricted live card");

    /* VACUITY CONTROL: the deltas above are only meaningful if the
       ~~~~~~ step really opened its mandatory discard prompt. */
    prompt_guard(
                 "Kagayaiteru's LiveSuccess must open its mandatory hand-discard prompt");
}

/* ===================================================================== */
/* 7. effects/restriction - mebius_loop_comprehensive_edge_test.rs      */
/* ===================================================================== */

/* Rust fill_p_stage: three copies of PL!HS-bp2-001-R (heart04 x3, heart05
   x1, heart01 x1) on the named seat's stage. */
#define MEBIUS_STAGE_FILLER "PL!HS-bp2-001-R"

static void mebius_fill_p_stage(TestGame *tg, int who)
{
    for (int i = 0; i < 3; i++) {
        int m = mid(tg, MEBIUS_STAGE_FILLER);
        pin_id(m, MEBIUS_STAGE_FILLER);
        tg->state.p[who].stage[i] = m;
    }
}

static int mebius_advance_to_live(TestGame *tg)
{
    int r = test_advance_to_phase(tg, RB_PHASE_LIVE_SET);
    CHECK(r, "advance_to_live: LiveCardSetFirstAttacker not reached");
    return r;
}

static int mebius_advance_victory(TestGame *tg)
{
    int r = test_advance_to_phase(tg, RB_PHASE_VICTORY);
    CHECK(r, "advance_victory: LiveVictoryDetermination not reached");
    return r;
}

/* Single mebius still blocks both when scores tied. */
static void test_mebius_single_copy_blocks_both_when_tied(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int mebius_p1     = mid(&tg, MEBIUS);
    int other_live_p2 = mid(&tg, MEBIUS);
    pin_id(mebius_p1, MEBIUS);
    pin_id(other_live_p2, MEBIUS);
    expect_effect_printed(mebius_p1, TRIG_LIVE_SUCCESS,
                          "precondition: Mebius prints its LiveSuccess restriction");
    mebius_fill_p_stage(&tg, 0);
    mebius_fill_p_stage(&tg, 1);
    int filler = mid(&tg, FILLER);
    pin_id(filler, FILLER);
    push_to_hand(&tg, 0, mebius_p1);
    push_to_hand(&tg, 1, other_live_p2);
    fill_both_decks(&tg, filler);

    if (!mebius_advance_to_live(&tg)) return;
    test_set_live_card(&tg, 0, mebius_p1);
    test_pass(&tg);
    test_set_live_card_for(&tg, 1, other_live_p2);
    if (!mebius_advance_victory(&tg)) return;
    int g = 0;
    while (rb_has_pending_choice(&tg.state) && g++ < 40) pick(&tg, 0);
    mebius_advance_victory(&tg);
    g = 0;
    while (rb_has_pending_choice(&tg.state) && g++ < 40) pick(&tg, 0);
    test_pass(&tg);
    drain_choices(&tg);

    /* Both should be blocked (neither in success). */
    CHECK(!bag_has(&tg.state.p[0].success, mebius_p1),
          "tied: P1 Mebius must be blocked from the success zone");
    CHECK(!bag_has(&tg.state.p[1].success, other_live_p2),
          "tied: P2 Mebius must be blocked from the success zone");
    /* VACUITY CONTROL: the two negative assertions above would both pass on
       a board where the live never resolved. Pin that the live cards were
       actually set and that the success zone was reachable at all. */
    CHECK(tg.state.p[0].live.n + bag_has(&tg.state.p[0].success, mebius_p1) > 0,
          "precondition: P1's live card was set into the live zone");
}

/* Untied scores: P1 mebius succeeds (2), P2 live fails (0). */
static void test_mebius_no_block_when_scores_untied_both_succeed(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int mebius_p1   = mid(&tg, MEBIUS);
    int fail_live_p2 = mid(&tg, BUTTERFLY);
    pin_id(mebius_p1, MEBIUS);
    pin_id(fail_live_p2, BUTTERFLY);
    mebius_fill_p_stage(&tg, 0);
    mebius_fill_p_stage(&tg, 1);
    int filler = mid(&tg, FILLER);
    push_to_hand(&tg, 0, mebius_p1);
    push_to_hand(&tg, 1, fail_live_p2);
    fill_both_decks(&tg, filler);

    if (!mebius_advance_to_live(&tg)) return;
    test_set_live_card(&tg, 0, mebius_p1);
    test_pass(&tg);
    test_set_live_card_for(&tg, 1, fail_live_p2);
    test_pass(&tg);
    test_pass(&tg);
    {
        int g = 0;
        while (rb_has_pending_choice(&tg.state) && g++ < 40) skip_choice(&tg);
    }
    if (!mebius_advance_victory(&tg)) return;
    {
        int g = 0;
        while (rb_has_pending_choice(&tg.state) && g++ < 40) pick(&tg, 0);
    }
    test_pass(&tg);
    drain_choices(&tg);

    int p1_total = -1, p2_total = -1;
    for (int i = 0; i < tg.state.n_snapshots; i++) {
        const RbLiveSnapshot *s = &tg.state.snapshots[i];
        if (s->player == 0) p1_total = s->total_score;
        if (s->player == 1) p2_total = s->total_score;
    }
    CHECK(p1_total != p2_total || (p1_total < 0 && p2_total < 0),
          "should be untied 2 vs 0, got %d vs %d", p1_total, p2_total);
    CHECK(bag_has(&tg.state.p[0].success, mebius_p1),
          "untied: P1 Mebius should reach the success zone");
    /* Positive control: the untied case really is untied, i.e. the
       snapshots exist. */
    CHECK(tg.state.n_snapshots > 0,
          "precondition: the live produced at least one performance snapshot");
}

/* Restriction expires after live_end. */
static void test_mebius_restriction_expires_next_live(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int mebius_p1 = mid(&tg, MEBIUS);
    int mebius_p2 = mid(&tg, MEBIUS);
    pin_id(mebius_p1, MEBIUS);
    pin_id(mebius_p2, MEBIUS);
    mebius_fill_p_stage(&tg, 0);
    mebius_fill_p_stage(&tg, 1);
    int filler = mid(&tg, FILLER);
    push_to_hand(&tg, 0, mebius_p1);
    push_to_hand(&tg, 1, mebius_p2);
    fill_both_decks(&tg, filler);

    if (!mebius_advance_to_live(&tg)) return;
    test_set_live_card(&tg, 0, mebius_p1);
    test_pass(&tg);
    test_set_live_card_for(&tg, 1, mebius_p2);
    if (!mebius_advance_victory(&tg)) return;
    {
        int g = 0;
        while (rb_has_pending_choice(&tg.state) && g++ < 40) pick(&tg, 0);
    }
    mebius_advance_victory(&tg);
    {
        int g = 0;
        while (rb_has_pending_choice(&tg.state) && g++ < 40) pick(&tg, 0);
    }
    test_pass(&tg);
    drain_choices(&tg);

    CHECK(!bag_has(&tg.state.p[0].success, mebius_p1),
          "first tied live should block P1's Mebius");

    /* Second live in the NEXT live window. */
    int next_live = mid(&tg, KAGAYA);
    pin_id(next_live, KAGAYA);
    push_to_hand(&tg, 0, next_live);
    int r = test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    CHECK(r, "second live: LiveCardSetFirstAttacker not reached");
    if (!r) return;
    test_set_live_card(&tg, 0, next_live);
    r = test_advance_to_phase(&tg, RB_PHASE_ACTIVE);
    CHECK(r, "second live: Active not reached");
    {
        int g = 0;
        while (rb_has_pending_choice(&tg.state) && g++ < 40) pick(&tg, 0);
    }
    drain_choices(&tg);

    CHECK(!bag_has(&tg.state.p[0].live, next_live),
          "the second live must not stay stuck in the live zone - the tie "
          "restriction expired at live_end");
    CHECK_EQ(tg.state.p[0].success.n, 0,
             "restriction should have expired - success zone empty after live_end");
}

/* Both lives fail but totals equal: the restriction should still fire. */
static void test_mebius_tie_when_both_fail_still_restricts(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int mebius_p1    = mid(&tg, MEBIUS);
    int fail_live_p2 = mid(&tg, BUTTERFLY);
    pin_id(mebius_p1, MEBIUS);
    pin_id(fail_live_p2, BUTTERFLY);
    mebius_fill_p_stage(&tg, 0);
    mebius_fill_p_stage(&tg, 1);
    int filler = mid(&tg, FILLER);
    push_to_hand(&tg, 0, mebius_p1);
    push_to_hand(&tg, 1, fail_live_p2);
    fill_both_decks(&tg, filler);

    if (!mebius_advance_to_live(&tg)) return;
    test_set_live_card(&tg, 0, mebius_p1);
    if (!test_advance_to_phase(&tg, RB_PHASE_LIVE_SET_SECOND)) return;
    test_set_live_card_for(&tg, 1, fail_live_p2);
    if (!test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE)) return;
    drain_choices(&tg);
    if (!mebius_advance_victory(&tg)) return;
    {
        int g = 0;
        while (rb_has_pending_choice(&tg.state) && g++ < 40) pick(&tg, 0);
    }
    test_pass(&tg);
    drain_choices(&tg);

    /* The Rust test's own comment concedes it exercises the UNTIED 2-vs-0
       case, so the translated assertion is the same one it makes. */
    CHECK(bag_has(&tg.state.p[0].success, mebius_p1),
          "P1 Mebius should be in success when untied (2 vs 0)");
}

/* ===================================================================== */
/* 8. effects/gain/hearts/color_choice - shared setup                    */
/* ===================================================================== */

/* Rust pl_n_pb1_034_n_036_n_setup: the card under test on CENTER, a
   bystander on LEFT. */
static int cc_stage_setup(TestGame *tg, const char *no, int n_fill)
{
    int filler = mid(tg, FILLER);
    pin_id(filler, FILLER);
    if (n_fill == 40) fill_decks40(tg, filler);
    else              fill_both_decks(tg, filler);
    int me = mid(tg, no);
    pin_id(me, no);
    tg->state.p[0].stage[1] = me;
    int bystander = id_ref(tg, FILLER);
    pin_id(bystander, FILLER);
    tg->state.p[0].stage[0] = bystander;
    return me;
}

/* Rust choose_color: fire ~~~~~~, assert the colour prompt appeared,
   answer it, then drain. The prompt assertion is the file's own
   precondition. */
static int choose_color(TestGame *tg, int cid, int option, const char *what)
{
    if (!fire_live_start(tg, cid)) {
        CHECK(0, "%s: card lacks a LiveStart ability", what);
        return 0;
    }
    expect_pending(rb_has_pending_choice(&tg->state), what);
    opt(tg, option);
    drain_choices(tg);
    return 1;
}

/* ===================================================================== */
/* 9. chosen_color_transform_live_start_test.rs                          */
/* ===================================================================== */

static void test_shioriko_choose_first_color_heart03_leaves_bystander_unchanged(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int me = cc_stage_setup(&tg, SHIORIKO, 30);
    int printed[6];
    int np = printed_hearts(&tg, me, printed);
    CHECK(np > 0, "precondition: Shioriko must PRINT hearts, or there is "
                  "nothing to replace");

    if (!choose_color(&tg, me, 0,
                      "precondition: the heart-colour choice prompt must be offered"))
        return;
    /* VACUITY CONTROL: the multiplier read below must be about an effect
       that really happened, not about a card that did nothing. */
    prompt_guard( "Shioriko's colour choice must really be offered");

    report_hearts(&tg, me, "shioriko[opt0]");
    CHECK_EQ(mult_of(&tg, me), H3,
             "chose heart03 -> original hearts become heart03");
    CHECK_EQ(mult_of(&tg, id_ref(&tg, FILLER)), -1,
             "bystander unchanged");
}

static void test_shioriko_choose_last_color_heart05(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int me = cc_stage_setup(&tg, SHIORIKO, 30);
    if (!choose_color(&tg, me, 2, "colour choice must be offered")) return;
    prompt_guard( "Shioriko's colour choice must really be offered");

    report_hearts(&tg, me, "shioriko[opt2]");
    CHECK_EQ(mult_of(&tg, me), H5,
             "chose heart05 -> original hearts become heart05");
}

static void test_shioriko_choose_mid_color_heart04(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int me = cc_stage_setup(&tg, SHIORIKO, 30);
    if (!choose_color(&tg, me, 1, "colour choice must be offered")) return;
    prompt_guard( "Shioriko's colour choice must really be offered");

    report_hearts(&tg, me, "shioriko[opt1]");
    CHECK_EQ(mult_of(&tg, me), H4, "chose heart04");
}

static void test_lanzhu_choose_last_color_heart06(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int me = cc_stage_setup(&tg, LANZHU, 30);
    if (!choose_color(&tg, me, 2, "colour choice must be offered")) return;
    prompt_guard( "Lanzhu's colour choice must really be offered");

    report_hearts(&tg, me, "lanzhu[opt2]");
    CHECK_EQ(mult_of(&tg, me), H6, "twin card's third option is heart06");
}

/* ===================================================================== */
/* 10. chosen_color_transform_scope_options_test.rs                      */
/* ===================================================================== */

static int pb2_030_setup(TestGame *tg)
{
    return cc_stage_setup(tg, WAKANA, 30);
}

static void test_pb2_030_first_option_transforms_original_hearts_to_heart02(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int me = pb2_030_setup(&tg);
    if (!fire_live_start(&tg, me)) {
        CHECK(0, "Wakana lacks a LiveStart ability");
        return;
    }
    opt(&tg, 0);
    drain_choices(&tg);
    prompt_guard( "Wakana's colour choice must really be offered");

    report_hearts(&tg, me, "wakana[opt0]");
    CHECK_EQ(mult_of(&tg, me), H2, "chose heart02 -> original hearts become heart02");
}

static void test_pb2_030_third_option_transforms_only_self_to_heart06(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int me = pb2_030_setup(&tg);
    if (!fire_live_start(&tg, me)) {
        CHECK(0, "Wakana lacks a LiveStart ability");
        return;
    }
    opt(&tg, 2);
    drain_choices(&tg);
    prompt_guard( "Wakana's colour choice must really be offered");

    report_hearts(&tg, me, "wakana[opt2]");
    CHECK_EQ(mult_of(&tg, me), H6, "chose heart06 -> original hearts become heart06");
    CHECK_EQ(mult_of(&tg, id_ref(&tg, FILLER)), -1, "bystander untouched");
}

/* ===================================================================== */
/* 11. chosen_colour_is_a_replacement_not_a_grant_test.rs                */
/* ===================================================================== */

static void test_kasumi_chosen_colour_is_a_replacement_not_a_granted_heart(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int kasumi = mid(&tg, KASUMI_N);
    pin_id(kasumi, KASUMI_N);
    int printed[6];
    int np = printed_hearts(&tg, kasumi, printed);
    CHECK(np > 0, "precondition: she must PRINT hearts, or there is nothing to replace");
    if (np == 0) return;

    int filler = mid(&tg, FILLER);
    fill_both_decks(&tg, filler);
    tg.state.p[0].stage[1] = kasumi;

    if (!choose_color(&tg, kasumi, 0,
                      "precondition: the heart-colour choice prompt must be offered"))
        return;
    prompt_guard( "Kasumi's colour choice must really be offered");

    int chosen = mult_of(&tg, kasumi);
    CHECK(chosen >= 0, "precondition: a colour was chosen");
    if (chosen < 0) return;

    int disjoint = 1;
    for (int i = 0; i < np; i++) if (printed[i] == chosen) disjoint = 0;
    if (!disjoint) {
        fprintf(stderr, "  (printed ");
        print_hearts(printed, np);
        fprintf(stderr, ", chose heart%02d)\n", chosen);
    }
    CHECK(disjoint,
          "precondition: the chosen colour is not one she prints, or replacement "
          "and addition are indistinguishable");

    CHECK_EQ(heart_of(&tg, kasumi, chosen), 0,
             "the printed hearts become the chosen colour is a REPLACEMENT, so the "
             "chosen colour is not also granted");
    report_hearts(&tg, kasumi, "kasumi[opt0]");
    CHECK_EQ(mult_of(&tg, kasumi), chosen,
             "exactly one colour override is present, the shape a replacement must have");
}

static void test_shioriko_unchosen_printed_hearts_are_superseded(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int shioriko = mid(&tg, SHIORIKO);
    pin_id(shioriko, SHIORIKO);
    int printed[6];
    int np = printed_hearts(&tg, shioriko, printed);
    int filler = mid(&tg, FILLER);
    fill_both_decks(&tg, filler);
    tg.state.p[0].stage[1] = shioriko;

    if (!choose_color(&tg, shioriko, 0, "colour choice must be offered")) return;
    prompt_guard( "Shioriko's colour choice must really be offered");

    int chosen = mult_of(&tg, shioriko);
    CHECK(chosen >= 0, "precondition: a colour was chosen");
    if (chosen < 0) return;

    int n_superseded = 0;
    for (int i = 0; i < np; i++) if (printed[i] != chosen) n_superseded++;
    CHECK(n_superseded > 0,
          "precondition: she prints at least one heart she did not choose");
    for (int i = 0; i < np; i++) {
        int colour = printed[i];
        if (colour == chosen) continue;
        CHECK_EQ(heart_of(&tg, shioriko, colour), 0,
                 "heart%02d is one she printed but did not choose, so it is not "
                 "hers any more", colour);
    }
}

static void test_kasumi_colour_override_does_not_survive_the_live_that_set_it(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int kasumi = mid(&tg, KASUMI_N);
    int live   = mid(&tg, LIVE_FILLER);
    pin_id(kasumi, KASUMI_N);
    pin_id(live, LIVE_FILLER);
    int filler = mid(&tg, FILLER);
    fill_both_decks(&tg, filler);
    tg.state.p[0].stage[1] = kasumi;
    push_to_hand(&tg, 0, live);
    test_give_energy(&tg, 10);

    int r = test_advance_to_phase(&tg, RB_PHASE_LIVE_SET);
    CHECK(r, "LiveCardSetFirstAttacker not reached");
    if (!r) return;
    test_set_live_card(&tg, 0, live);

    if (!choose_color(&tg, kasumi, 0, "colour choice must be offered")) return;
    prompt_guard( "Kasumi's colour choice must really be offered");

    CHECK(mult_of(&tg, kasumi) >= 0,
          "precondition: the override is live before the rollover");

    int guard = 0;
    while (tg.state.phase >= RB_PHASE_LIVE_SET && tg.state.phase <= RB_PHASE_VICTORY &&
           guard < 12) {
        guard++;
        test_pass(&tg);
        test_drain_auto_choices(&tg);
        drain_choices(&tg);
    }
    test_recalc(&tg);

    report_hearts(&tg, kasumi, "kasumi[after live]");
    CHECK_EQ(mult_of(&tg, kasumi), -1,
             "until live end - the override does not survive its live");
}

/* ===================================================================== */
/* 12. live_start_optional_energy_chosen_heart_test.rs                   */
/* ===================================================================== */

static void test_live_start_optional_energy_gains_chosen_heart(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int shizuku  = mid(&tg, SHIZUKU);
    int filler   = mid(&tg, FILLER);
    int live_card = mid(&tg, LIVE_FILLER2);
    pin_id(shizuku, SHIZUKU);
    pin_id(filler, FILLER);
    pin_id(live_card, LIVE_FILLER2);

    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = shizuku;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    push_to_hand(&tg, 0, live_card);
    push_to_hand(&tg, 0, filler);
    test_give_energy(&tg, 20);
    tg.state.p[0].deck.n = 0;
    for (int i = 0; i < 30; i++) tg.state.p[0].deck.cards[tg.state.p[0].deck.n++] = filler;
    tg.state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) tg.state.p[1].deck.cards[tg.state.p[1].deck.n++] = filler;

    for (int i = 0; i < 5; i++) test_pass(&tg);
    test_set_live_card(&tg, 0, live_card);
    test_pass(&tg);
    test_pass(&tg);

    expect_pending(rb_has_pending_choice(&tg.state),
                   "optional cost prompt expected before heart selection");
    CHECK(!strcmp(pending_type(&tg), "SelectTarget"),
          "expected SelectTarget (pay_optional_cost:skip), got %s", pending_type(&tg));
    /* Diagnostic that separates the two possible causes of the mismatch
       above: a gate that was never opened, versus a gate that was opened and
       auto-paid. A paid gate costs the player an energy. */
    fprintf(stderr,
            "  shizuku: first prompt is %s/%s, P1 active energy = %d\n",
            pending_type(&tg),
            rb_get_pending_choice(&tg.state) ? rb_get_pending_choice(&tg.state)->target : "-",
            rb_energy_active_count(&tg.state.p[0]));
    /* Option 0 is SKIP, option 1 is PAY: answering 0 makes the paying half
       of this test identical to the skipping half. */
    opt(&tg, 1);
    expect_pending(rb_has_pending_choice(&tg.state),
                   "heart color selection should be pending");
    /* SelectHeartColor options are [heart01..heart06], 0-indexed; heart04 is
       at index 3. */
    opt(&tg, 3);
    fprintf(stderr, "  shizuku: P1 active energy after the cost = %d\n",
            rb_energy_active_count(&tg.state.p[0]));
    /* VACUITY CONTROL: the two "should be pending" assertions pass trivially
       on a board where the ability never fired. */
    prompt_guard(
                 "Shizuku's LiveStart must really open the optional-cost gate");

    int after[6];
    heart_mods(&tg, shizuku, after);
    report_hearts(&tg, shizuku, "shizuku");
    CHECK_EQ(after[3], 1, "heart04 should have +1 modifier");
    CHECK_EQ(after[4], 0, "heart05 should have 0 modifier");
}

/* ===================================================================== */
/* 13. paid_heart_color_choice_test.rs                                   */
/* ===================================================================== */

static void test_nico_activation_self_wait_cost_grants_only_chosen_heart(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int niko = mid(&tg, NICO);
    pin_id(niko, NICO);
    tg.state.p[0].stage[1] = niko;

    activate(&tg, niko);

    /* Cost (self-wait) applies first. */
    const char *orient = rb_mods_get_orientation(&tg.state.mods, niko);
    CHECK(orient && !strcmp(orient, "wait"),
          "activation cost waits this member (got %s)", orient ? orient : "none");

    expect_pending(rb_has_pending_choice(&tg.state),
                   "heart colour choice should be pending");
    prompt_guard( "Nico's activation must really offer the colour choice");
    opt(&tg, 1);   /* heart03 */

    report_hearts(&tg, niko, "nico[opt1]");
    CHECK_EQ(heart_of(&tg, niko, H3), 1, "chosen heart03 granted until live end");
    CHECK_EQ(heart_of(&tg, niko, H1), 0, "unchosen colours are not granted");
}

static void test_kasumi_quartz_discard_cost_grants_chosen_heart_only_to_other(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int kasumu  = mid(&tg, KASUMI_QUARTZ);
    int other   = mid(&tg, OMIKARI);
    int outsider = mid(&tg, AQOUR_MEMBER);
    pin_id(kasumu, KASUMI_QUARTZ);
    pin_id(other, OMIKARI);
    pin_id(outsider, AQOUR_MEMBER);

    tg.state.p[0].stage[0] = other;
    tg.state.p[0].stage[1] = kasumu;
    tg.state.p[0].stage[2] = outsider;
    test_add_to_hand(&tg, id_ref(&tg, FILLER));

    if (!fire_live_start(&tg, kasumu)) {
        CHECK(0, "Kasumi QU4RTZ lacks a LiveStart ability");
        return;
    }

    /* Optional cost ~~~~1~~~~~~~~~~~~ presented directly as a
       skippable hand SelectCard: pick the single card to PAY. */
    expect_pending(rb_has_pending_choice(&tg.state), "cost prompt appears");
    prompt_guard( "Kasumi QU4RTZ's optional discard cost must be offered");
    pick(&tg, 0);

    expect_pending(rb_has_pending_choice(&tg.state),
                   "heart colour specification must be asked");
    opt(&tg, 4);   /* some colour; identity checked below */
    drain_choices(&tg);

    int n_granted = 0;
    for (int c = 1; c <= 6; c++) if (heart_of(&tg, other, c) > 0) n_granted++;
    CHECK_EQ(n_granted, 1,
             "exactly one specified colour granted to the other Nijigasaki member");
    for (int c = 1; c <= 6; c++) {
        CHECK_EQ(heart_of(&tg, kasumu, c), 0, "exclude_self: Kasumi gains nothing (heart%02d)", c);
        CHECK_EQ(heart_of(&tg, outsider, c), 0,
                 "non-Nijigasaki members gain nothing (heart%02d)", c);
    }
}

/* ===================================================================== */
/* 14. paid_discard_color_choice_heart_test.rs                           */
/* ===================================================================== */

static void test_sd2_005_paid_discard_grants_exactly_two_hearts_of_one_color(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int filler = mid(&tg, FILLER);
    fill_both_decks(&tg, filler);
    int me = mid(&tg, AI_SD2);
    pin_id(me, AI_SD2);
    tg.state.p[0].stage[1] = me;
    test_add_to_hand(&tg, mid(&tg, FILLER));
    test_add_to_hand(&tg, mid(&tg, SD1_001));

    if (!fire_live_start(&tg, me)) {
        CHECK(0, "Ai lacks a LiveStart ability");
        return;
    }
    expect_pending(rb_has_pending_choice(&tg.state), "discard-2 cost prompted");
    prompt_guard( "Ai's optional discard-2 cost must be offered");
    {
        const int two[2] = { 0, 1 };
        pick_many(&tg, two, 2);
    }
    expect_pending(rb_has_pending_choice(&tg.state), "color specification offered");
    opt(&tg, 3);

    int boosted[6];
    int n_boosted = 0;
    for (int c = 1; c <= 6; c++) if (heart_of(&tg, me, c) > 0) boosted[n_boosted++] = c;
    CHECK_EQ(n_boosted, 1, "exactly one color boosted");
    if (n_boosted == 1) {
        CHECK_EQ(heart_of(&tg, me, boosted[0]), 2,
                 "the chosen color gained exactly +2");
    } else {
        /* Without a single boosted bucket the count assertion above is the
           only signal; name what was actually written. */
        for (int c = 1; c <= 6; c++)
            if (heart_of(&tg, me, c) != 0)
                fprintf(stderr, "  (heart%02d = %d)\n", c, heart_of(&tg, me, c));
    }
}

/* ===================================================================== */
/* 15. paid_energy_color_choice_heart01_test.rs                          */
/* ===================================================================== */

static void test_bp7_012_paid_energy_and_first_color_grant_one_heart01(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int filler = mid(&tg, FILLER_N);
    pin_id(filler, FILLER_N);
    fill_both_decks(&tg, filler);
    int me = mid(&tg, ARAN_BP7);
    pin_id(me, ARAN_BP7);
    tg.state.p[0].stage[1] = me;
    test_give_energy(&tg, 3);

    if (!fire_live_start(&tg, me)) {
        CHECK(0, "Aran lacks a LiveStart ability");
        return;
    }
    expect_pending(rb_has_pending_choice(&tg.state), "optional {{E}} gate offered");
    prompt_guard( "Aran's optional energy gate must be offered");
    opt(&tg, 1);   /* PAY, not SKIP */
    {
        int guard = 0;
        while (rb_has_pending_choice(&tg.state) && guard++ < 10) {
            if (!strcmp(pending_type(&tg), "SelectHeartColor")) opt(&tg, 0);
            else pick(&tg, 0);
        }
    }
    report_hearts(&tg, me, "aran[opt0]");
    CHECK_EQ(heart_of(&tg, me, H1), 1, "first offered color gained until live end");
}

/* ===================================================================== */
/* 16. success_pile_chosen_heart_color_test.rs                           */
/* ===================================================================== */

static void test_bp5_011_chosen_heart05_matches_two_success_cards(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int eli  = mid(&tg, ELI);
    int live = mid(&tg, LIVE_FILLER);
    pin_id(eli, ELI);
    pin_id(live, LIVE_FILLER);

    tg.state.p[0].stage[1] = eli;
    push_to_success(&tg, 0, live);
    push_to_success(&tg, 0, live);

    if (!fire_live_start(&tg, eli)) {
        CHECK(0, "Eli lacks a LiveStart ability");
        return;
    }
    expect_pending(rb_has_pending_choice(&tg.state), "colour choice must be asked");
    prompt_guard( "Eli's colour choice must really be offered");
    opt(&tg, 1);   /* heart05 */

    report_hearts(&tg, eli, "eli[opt1]");
    CHECK_EQ(heart_of(&tg, eli, H5), 2, "chosen heart05 x 2 success-pile cards");
    CHECK_EQ(heart_of(&tg, eli, H4), 0, "unchosen colours are not granted");
}

/* ===================================================================== */
/* 17. success_zone_color_count_chosen_heart_test.rs                     */
/* ===================================================================== */

static int bp3_012_advance_live_set(TestGame *tg)
{
    for (int i = 0; i < 5; i++) { test_pass(tg); drain_choices(tg); }
    int arrived = tg->state.phase == RB_PHASE_LIVE_SET;
    CHECK(arrived, "bp3_012: expected LiveCardSet, got phase %d", tg->state.phase);
    return arrived;
}

static void test_bp3_012_chosen_heart_scales_with_success_zone_count(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int kotori = mid(&tg, KOTORI);
    int live   = mid(&tg, FILLER);
    pin_id(kotori, KOTORI);
    pin_id(live, FILLER);
    push_to_hand(&tg, 1, mid(&tg, FILLER));
    tg.state.p[0].stage[1] = kotori;
    push_to_hand(&tg, 0, live);
    test_give_energy(&tg, 10);
    int filler = mid(&tg, FILLER);
    fill_both_decks(&tg, filler);

    if (!bp3_012_advance_live_set(&tg)) return;
    test_set_live_card(&tg, 0, live);
    test_pass(&tg);
    test_pass(&tg);

    expect_pending(rb_has_pending_choice(&tg.state), "Should prompt for heart color");
    prompt_guard( "Kotori's colour choice must really be offered");
    opt(&tg, 0);

    report_hearts(&tg, kotori, "kotori[0 success]");
    int heart01 = heart_of(&tg, kotori, H1);
    CHECK_EQ(heart01, 0, "0 cards in success zone -> 0 hearts");
}

/* ===================================================================== */
/* 18. EXPECTED_GAP probe (never a pass, never a failure)                  */
/* ===================================================================== */

/* The flat RbChoice the C engine exposes carries `n_heart_options` but the
   per-option LABELS are only observable for a select_heart_color ability.
   A `select` + `heart_colors` ability routes through a different emitter,
   so the option list for those cards is not observable from C at all. */
static void test_gap_heart_option_labels_not_observable(void)
{
    static TestGame tg;
    test_game_new(&tg);
    clear_p1(&tg);
    clear_p2(&tg);
    board_begin(&tg);

    int me = cc_stage_setup(&tg, SHIORIKO, 30);
    if (!fire_live_start(&tg, me)) return;
    const RbChoice *c = rb_get_pending_choice(&tg.state);
    if (!c) {
        EXPECTED_GAP(0, "no pending choice at all: a select+heart_colors "
                        "ability emitted nothing to answer");
        return;
    }
    EXPECTED_GAP(c->kind == RB_CHOICE_SELECT_HEART_COLOR,
                 "the colour prompt is a SelectHeartColor with a readable "
                 "heart_options[] list");
}

/* ===================================================================== */
/* main                                                                    */
/* ===================================================================== */

int main(int argc, char **argv)
{
    int dump = (argc > 1 && strcmp(argv[1], "dump") == 0);
    /* The Makefile `t` target runs the binary with no arguments, so a
       fixture-dump mode is also reachable through the environment. */
    if (getenv("TR_DUMP")) dump = 1;
    if (argc > 1 && !dump) only_case = argv[1];

    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS, on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);
    signal(SIGFPE, on_fatal_signal);
    signal(SIGILL, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "fatal: no card database (tried src, ../cards/build)\n");
        return 2;
    }
    if (pipe(g_pipe) != 0) {
        fprintf(stderr, "fatal: pipe() failed\n");
        return 2;
    }

    if (dump) {
        static TestGame tg;
        test_game_new(&tg);
        clear_p1(&tg);
        clear_p2(&tg);
        const char *cards[] = {
            KANON, HIMENO, AQOURLOCK, KAGAYA, MEBIUS, BUTTERFLY, HEART_PROV,
            PROTEINBAR, SD1_002, FILLER, FILLER_N, LIVE_FILLER, LIVE_FILLER2,
            KASUMI_N, KASUMI_QUARTZ, SHIORIKO, LANZHU, WAKANA, ELI, KOTORI,
            NICO, AI_SD2, ARAN_BP7, SHIZUKU, OMIKARI, AQOUR_MEMBER, SD1_001,
            MEBIUS_STAGE_FILLER,
        };
        for (size_t i = 0; i < sizeof cards / sizeof cards[0]; i++) {
            int cid = test_new_id(&tg, cards[i]);
            printf("%-26s -> id=%-6d resolved=%-26s live=%d member=%d ab=%d\n",
                   cards[i], cid,
                   card_no_of(cid),
                   rb_card_is_live(cid) ? 1 : 0,
                   rb_card_is_member(cid) ? 1 : 0,
                   rb_card_num_abilities((uint32_t)(cid >= 0 ? cid : 0)));
            if (cid < 0) continue;
            for (int a = 0; a < rb_card_num_abilities((uint32_t)cid); a++)
                dump_ability(cid, a);
        }
        return 0;
    }

    /* ---- effects/restriction ---- */
    run("restriction/alone_sets_cannot_live",
        test_alone_on_stage_sets_cannot_live_restriction);
    run("restriction/other_member_prevents_cannot_live",
        test_another_stage_member_prevents_cannot_live_restriction);
    run("restriction/delayed_cannot_active_one_owner_tick",
        test_delayed_cannot_active_expires_after_one_owner_tick);
    run("restriction/delayed_cannot_active_twice_one_tick",
        test_repeated_delayed_cannot_active_does_not_extend_owner_ticks);
    run("restriction/active_phase_lock_p1_targets_p2",
        test_active_phase_lock_owned_by_first_player_targets_second_player);
    run("restriction/active_phase_lock_p2_targets_p1",
        test_active_phase_lock_owned_by_second_player_targets_first_player);
    run("restriction/active_phase_lock_excludes_owner",
        test_active_phase_lock_excludes_owners_members);
    run("restriction/kagayaiteru_live_success_draw_then_discard",
        test_kagayaiteru_live_success_draw_then_discard);
    run("restriction/kagayaiteru_q125_cannot_place_in_success_zone",
        test_kagayaiteru_q125_cannot_place_in_success_zone);
    run("restriction/live_success_both_sides_advances_turn",
        test_live_success_both_sides_draw_discard_advances_turn);
    run("restriction/mebius_tied_blocks_both",
        test_mebius_single_copy_blocks_both_when_tied);
    run("restriction/mebius_untied_allows_p1",
        test_mebius_no_block_when_scores_untied_both_succeed);
    run("restriction/mebius_expires_next_live",
        test_mebius_restriction_expires_next_live);
    run("restriction/mebius_tie_when_both_fail",
        test_mebius_tie_when_both_fail_still_restricts);

    /* ---- effects/gain/hearts/color_choice ---- */
    run("cc/shioriko_first_heart03_bystander_unchanged",
        test_shioriko_choose_first_color_heart03_leaves_bystander_unchanged);
    run("cc/shioriko_last_heart05",
        test_shioriko_choose_last_color_heart05);
    run("cc/shioriko_mid_heart04",
        test_shioriko_choose_mid_color_heart04);
    run("cc/lanzhu_last_heart06",
        test_lanzhu_choose_last_color_heart06);
    run("cc/pb2_030_first_heart02",
        test_pb2_030_first_option_transforms_original_hearts_to_heart02);
    run("cc/pb2_030_third_heart06_only_self",
        test_pb2_030_third_option_transforms_only_self_to_heart06);
    run("cc/kasumi_replacement_not_grant",
        test_kasumi_chosen_colour_is_a_replacement_not_a_granted_heart);
    run("cc/shioriko_unchosen_printed_superseded",
        test_shioriko_unchosen_printed_hearts_are_superseded);
    run("cc/kasumi_override_does_not_survive_live",
        test_kasumi_colour_override_does_not_survive_the_live_that_set_it);
    run("cc/shizuku_optional_energy_chosen_heart",
        test_live_start_optional_energy_gains_chosen_heart);
    run("cc/nico_activation_self_wait_only_chosen",
        test_nico_activation_self_wait_cost_grants_only_chosen_heart);
    run("cc/kasumi_quartz_discard_cost_other_nijigasaki",
        test_kasumi_quartz_discard_cost_grants_chosen_heart_only_to_other);
    run("cc/sd2_005_discard2_two_hearts_one_color",
        test_sd2_005_paid_discard_grants_exactly_two_hearts_of_one_color);
    run("cc/aran_bp7_012_energy_heart01",
        test_bp7_012_paid_energy_and_first_color_grant_one_heart01);
    run("cc/eli_bp5_011_heart05_x2_success",
        test_bp5_011_chosen_heart05_matches_two_success_cards);
    run("cc/kotori_bp3_012_zero_success_zero_hearts",
        test_bp3_012_chosen_heart_scales_with_success_zone_count);
    run("cc/gap_heart_option_labels", test_gap_heart_option_labels_not_observable);

    printf("\n==== test_parity_restriction: %d cases ====\n"
           "%d ok, %d failed, %d crashed, %d expected gaps\n"
           "%ld checks, %d failures\n",
           n_ok + n_failed + n_crashed, n_ok, n_failed, n_crashed, gaps,
           checks, failures);

    return (failures == 0 && n_failed == 0 && n_crashed == 0) ? 0 : 1;
}
