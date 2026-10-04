/* test_null_effect_guard.c
 *
 * THE NULL-EFFECT DEREFERENCE IN src/ability/effects/move.c
 * ==========================================================
 *
 * WHAT BROKE
 * ----------
 * `cmf_extra` (move.c) is the accessor every one of move.c's effect lookups goes
 * through. Its body dereferenced the effect pointer with no guard:
 *
 *     static const char *cmf_extra(const AbilityEffect *e, const char *k){
 *         for(int i=0;i<e->n_extra;i++) ...        <-- reads e->n_extra
 *
 * Its own sibling -- same name, same port, one file over -- already had the
 * guard (move_missing.c:432 `cmf_extra`). Of the tree's 28 effect-extra
 * accessors, move.c's was one of six with no guard at all.
 *
 * WHY IT WAS REACHABLE, NOT COSMETIC
 * ----------------------------------
 * `rb_move_take_cards_from_standard_zone` is NOT static (it is forward-declared
 * in two translation units) and its whole body is written to tolerate a NULL
 * effect: `if (e && ...)`, `e ? cmf_extra(e, ...) : NULL`, and a
 * `move_build_standard_zone_filter` whose every single read is `e ? ... : NULL`.
 * Exactly one reference to `e` in that function broke the contract:
 *
 *     move.c:1379  int any_number = e && cmf_extra_true(e, "any_number");
 *     move.c:1380  if (is_discard) can_skip = cmf_extra_true(e, "max") || (e && e->is_optional);
 *                                                 ^^^^^^^^^^^^^^^^^^^^^ no `e &&`
 *     move.c:1381  else if (is_hand) can_skip = (e && e->is_optional) || any_number;
 *     move.c:1384  if (e && cmf_extra_true(e, "max")) can_skip = 1;
 *
 * Line 1380 is the only `e` reference in the block without the guard its three
 * neighbours all carry, and it is reached whenever the source zone is the
 * discard pile. So `rb_move_take_cards_from_standard_zone(g, actor, "discard",
 * NULL, ...)` faulted. The fix is in the accessor, not on that line: guarding
 * cmf_extra makes all ~80 call sites in the file safe rather than one.
 *
 * WHAT THIS SUITE ASSERTS
 * -----------------------
 * 1. NO CRASH. Every case runs in its own forked child, so a NULL dereference
 *    is reported as `FAIL [crash] <case>` naming the case in flight instead of
 *    killing the suite. Without the fork this suite would simply die and read
 *    as a suite that produced no output -- which is BROKEN, not PASSING.
 *
 * 2. THE GUARD IS TRANSPARENT, NOT A BEHAVIOUR CHANGE. A NULL effect is compared
 *    against a zeroed AbilityEffect carrying no extras, and the two must agree
 *    on return value AND on final zone contents. That is the only oracle-free
 *    way to pin what a NULL effect is supposed to mean: "an effect with no
 *    keys". Pinning an invented expected number instead would assert nothing
 *    about the guard -- it would just re-state whatever the code happens to do.
 *
 *    A guard that returned early / bailed out would pass case 1 and fail here.
 *
 * PROCESS MODEL: fork() per case, counters in a MAP_SHARED|MAP_ANONYMOUS
 * mapping zeroed in the CHILD and read by the parent AFTER waitpid, so a child
 * that dies mid-case still reports what it reached.
 *
 * FILE IS PURE ASCII: no Japanese wire values appear here, and every string is
 * ASCII, so the file survives a tool that rewrites sources in a legacy code page.
 */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE 1
#endif
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* move.c surface not declared in include/rabuka.h. Declared here the same way
 * src/turn/triggers.c and the sibling parity suites declare what they use. */
int rb_move_take_cards_from_standard_zone(GameState *g, int actor,
                                          const char *zone_name,
                                          AbilityEffect *e,
                                          int count, int is_all,
                                          int can_skip, int *out_ids, int max);

/* ------------------------------------------------------------------ harness */

typedef struct {
    volatile int assertions;
    volatile int failures;
    volatile int crashes;
    volatile int last_signal;
    volatile int last_ret;      /* what the call under test returned */
    volatile int last_zn;       /* ... and the zone size afterwards */
    volatile int last_out0;     /* ... and the first id it handed back */
} Shared;

/* Card ids used as zone contents. These are CARD-NO INDICES into the loaded
 * database, not printable card numbers, so this suite cannot fall into the
 * rb_find_card_by_no substring trap: it never looks a card up by number at all.
 * They only need to be distinct valid indices so the zone is non-empty. */
#define CID_A 100
#define CID_B 101

static void bag_set(RbBag *b, int a, int bcard)
{
    memset(b, 0, sizeof *b);
    b->cards[0] = a;
    b->n = 1;
    if (bcard >= 0) { b->cards[1] = bcard; b->n = 2; }
}

static void bag_copy(const RbBag *src, RbBag *dst)
{
    memcpy(dst, src, sizeof *dst);
}

/* One call under test. Runs with the effect pointer the case chose -- NULL for
 * the guarded path, or a zeroed effect for the reference path. */
static void run_call(GameState *g, const char *zone, AbilityEffect *e,
                     Shared *sh, int *out_ids)
{
    RbBag *target = !strcmp(zone, "hand")    ? &g->p[0].hand
                  : !strcmp(zone, "energy")  ? &g->p[0].energy
                  : !strcmp(zone, "live_card_zone") ? &g->p[0].live
                                                   : &g->p[0].discard;
    bag_set(target, CID_A, CID_B);
    memset(out_ids, 0, 4 * sizeof(int));
    sh->last_ret = rb_move_take_cards_from_standard_zone(g, 0, zone, e,
                                                         1, 0, 0, out_ids, 4);
    sh->last_zn  = target->n;
    sh->last_out0 = out_ids[0];
}

typedef void (*CaseFn)(GameState *g, Shared *sh);

/* ------------------------------------------------------------------- cases */

/* THE TRIGGER. Source zone = discard, effect = NULL. This is the line-1380
 * path. Any effect pointer other than NULL here would not test the defect. */
static void case_trigger_null_discard(GameState *g, Shared *sh)
{
    int out[4];
    run_call(g, "discard", NULL, sh, out);
    sh->assertions = 1;
}

/* Same call through the reference path: a zeroed effect with no extras. */
static void case_reference_empty_discard(GameState *g, Shared *sh)
{
    int out[4];
    AbilityEffect empty;
    memset(&empty, 0, sizeof empty);
    run_call(g, "discard", &empty, sh, out);
    sh->assertions = 1;
}

/* The other zones the function handles through the same accessor, so the
 * contract is pinned zone by zone rather than only on the one that crashed. */
static void case_null_hand(GameState *g, Shared *sh)
{
    int out[4];
    run_call(g, "hand", NULL, sh, out);
    sh->assertions = 1;
}
static void case_reference_empty_hand(GameState *g, Shared *sh)
{
    int out[4];
    AbilityEffect empty;
    memset(&empty, 0, sizeof empty);
    run_call(g, "hand", &empty, sh, out);
    sh->assertions = 1;
}
static void case_null_energy(GameState *g, Shared *sh)
{
    int out[4];
    run_call(g, "energy", NULL, sh, out);
    sh->assertions = 1;
}
static void case_reference_empty_energy(GameState *g, Shared *sh)
{
    int out[4];
    AbilityEffect empty;
    memset(&empty, 0, sizeof empty);
    run_call(g, "energy", &empty, sh, out);
    sh->assertions = 1;
}
static void case_null_live(GameState *g, Shared *sh)
{
    int out[4];
    run_call(g, "live_card_zone", NULL, sh, out);
    sh->assertions = 1;
}
static void case_reference_empty_live(GameState *g, Shared *sh)
{
    int out[4];
    AbilityEffect empty;
    memset(&empty, 0, sizeof empty);
    run_call(g, "live_card_zone", &empty, sh, out);
    sh->assertions = 1;
}

/* Empty zone: a NULL effect on an already-empty discard must be a clean no-op,
 * not a fault. Guards against a "fix" that returned early for every NULL. */
static void case_null_empty_zone(GameState *g, Shared *sh)
{
    RbBag save = g->p[0].discard;
    memset(&g->p[0].discard, 0, sizeof g->p[0].discard);
    int out[4];
    run_call(g, "discard", NULL, sh, out);
    bag_copy(&save, &g->p[0].discard);
    sh->assertions = 1;
}

/* -------------------------------------------------------------------- main */

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: rb_load(src) failed\n");
        return 1;
    }

    int failures = 0, assertions = 0, crashes = 0;

    /* Fresh template per case: rb_move_take_cards_from_standard_zone mutates
     * the zone, so reusing one GameState across cases would make each case
     * depend on the last. */
    GameState *tmpl = malloc(sizeof *tmpl);
    if (!tmpl) { fprintf(stderr, "FAIL: cannot allocate template GameState\n"); rb_unload(); return 1; }

    struct { const char *name; CaseFn fn; } cases[] = {
        { "trigger_null_discard",        case_trigger_null_discard },
        { "reference_empty_discard",      case_reference_empty_discard },
        { "null_hand",                    case_null_hand },
        { "reference_empty_hand",         case_reference_empty_hand },
        { "null_energy",                  case_null_energy },
        { "reference_empty_energy",       case_reference_empty_energy },
        { "null_live_card_zone",          case_null_live },
        { "reference_empty_live_card",    case_reference_empty_live },
        { "null_empty_zone",              case_null_empty_zone },
    };
    const int n_cases = (int)(sizeof cases / sizeof cases[0]);

    /* Pass 1: run every case, recording each child's result. */
    Shared *rec = calloc(n_cases, sizeof(Shared));
    if (!rec) { fprintf(stderr, "FAIL: cannot allocate result array\n"); rb_unload(); return 1; }

    for (int i = 0; i < n_cases; i++) {
        Shared *sh = &rec[i];
        memset(sh, 0, sizeof *sh);

        GameState *g = malloc(sizeof *g);
        if (!g) { fprintf(stderr, "FAIL: cannot allocate GameState\n"); rb_unload(); return 1; }
        memcpy(g, tmpl, sizeof *g);

        pid_t pid = fork();
        if (pid == 0) {
            signal(SIGSEGV, SIG_DFL);
            signal(SIGBUS,  SIG_DFL);
            signal(SIGABRT, SIG_DFL);
            cases[i].fn(g, sh);
            _exit(0);
        }
        int st = 0;
        waitpid(pid, &st, 0);
        if (WIFSIGNALED(st)) {
            sh->crashes++;
            sh->last_signal = WTERMSIG(st);
            crashes++;
            fprintf(stderr, "FAIL [crash] %s: died on signal %d -- the NULL effect "
                            "deref in move.c cmf_extra is still present\n",
                    cases[i].name, WTERMSIG(st));
            failures++;
        }
        assertions++;
        free(g);
    }

/* Pass 2: the transparency assertion. A NULL effect must agree with an
     * effect carrying no extras, on all four things run_call recorded. */
    struct { const char *null_name; const char *ref_name; } pairs[] = {
        { "trigger_null_discard",     "reference_empty_discard" },
        { "null_hand",                "reference_empty_hand" },
        { "null_energy",              "reference_empty_energy" },
        { "null_live_card_zone",      "reference_empty_live_card" },
    };
    for (int i = 0; i < 4; i++) {
        int ia = -1, ib = -1;
        for (int k = 0; k < n_cases; k++) {
            if (!strcmp(cases[k].name, pairs[i].null_name)) ia = k;
            if (!strcmp(cases[k].name, pairs[i].ref_name))  ib = k;
        }
        assertions++;
        if (ia < 0 || ib < 0) { failures++; continue; }
        if (rec[ia].crashes || rec[ib].crashes) continue; /* already reported */
        if (rec[ia].last_ret != rec[ib].last_ret) {
            failures++;
            fprintf(stderr, "FAIL: %s: NULL effect returned %d but an effect with no "
                            "extras returned %d -- the guard is not transparent\n",
                    pairs[i].null_name, rec[ia].last_ret, rec[ib].last_ret);
        } else if (rec[ia].last_zn != rec[ib].last_zn) {
            failures++;
            fprintf(stderr, "FAIL: %s: NULL effect left %d cards in the zone but no-extras "
                            "left %d -- the guard changed the mutation\n",
                    pairs[i].null_name, rec[ia].last_zn, rec[ib].last_zn);
        } else if (rec[ia].last_out0 != rec[ib].last_out0) {
            failures++;
            fprintf(stderr, "FAIL: %s: NULL effect returned id %d but no-extras returned %d\n",
                    pairs[i].null_name, rec[ia].last_out0, rec[ib].last_out0);
        }
    }

    /* The no-op case must actually be a no-op. */
    assertions++;
    if (!rec[n_cases - 1].crashes && rec[n_cases - 1].last_ret != 0) {
        failures++;
        fprintf(stderr, "FAIL: null_empty_zone: a NULL effect on an empty discard returned "
                        "%d, expected 0 (an early-return 'fix' would pass the crash check "
                        "and fail this)\n", rec[n_cases - 1].last_ret);
    }

    fprintf(stdout, "null-effect guard: %d assertions, %d failures, %d crashes\n",
            assertions, failures, crashes);
    if (failures == 0) fprintf(stdout, "PASSED\n");
    else                fprintf(stdout, "FAILED\n");

    free(rec);
    free(tmpl);
    rb_unload();
    return failures ? 1 : 0;
}