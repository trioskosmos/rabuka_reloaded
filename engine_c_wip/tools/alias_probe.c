/* tools/alias_probe.c -- the ALIAS-CLASS PROBE for the parked-effect slots.
 *
 * WHAT IT PROVES, AND WHY IT IS A LINK-TIME WRAP RATHER THAN A SOURCE EDIT
 * ------------------------------------------------------------------------
 * The defect class is: a producer parks a RAW `AbilityEffect *` into one of the
 * seven continuation slots, and some OTHER owner frees the tree that pointer
 * aliases into while a prompt is open. The previous audit expressed that as
 * "wrap rb_effect_free and rb_free_ability, then test all 7 alias slots against
 * every freed subtree while a prompt is open".
 *
 * The wrap is done with the LINKER (-Wl,--wrap=...) rather than by editing
 * alloc.c / vm.c, for three reasons that all matter here:
 *
 *   1. It touches no engine source, so the probe measures the tree as it is and
 *      cannot itself perturb the thing being measured.
 *   2. It measures the CALL-SITE graph, not the body: --wrap redirects every
 *      UNDEFINED reference, which is every call from another translation unit.
 *      The two intra-TU recursions (alloc.c's own rb_effect_free(child[i]) and
 *      vm.c's static effect_free) are not redirected, and must not be, because
 *      the probe walks the WHOLE subtree itself before delegating, so a
 *      partial interception would under-report.
 *   3. rb_free_ability's release path is vm.c's STATIC effect_free, which no
 *      wrapper can name. Intercepting rb_free_ability and enumerating a->cost
 *      and a->effect before delegating covers exactly the node set that
 *      effect_free is about to release.
 *
 * WHY rb_has_pending_choice IS WRAPPED TOO
 * ------------------------------------------
 * The check needs the GameState, and neither free function takes one. So the
 * probe also wraps rb_has_pending_choice(g), the most-called predicate in the
 * prompt protocol (~190 sites across 13 files), purely to CACHE the live g. The
 * cached pointer is then validated by reading g->queue.has_pending directly, so
 * the "is a prompt open" half of the condition never recurses back through the
 * wrapper. A cached g whose has_pending is 0 contributes nothing, which is also
 * what makes a stale cache harmless: the probe reports a hit only when that
 * exact state currently has an open prompt.
 *
 * FALSE POSITIVES
 * ---------------
 * There are none by construction, and that is the reason this design was chosen
 * over the alternative (record every freed address in a set, then test the slots
 * against the set). The address-set design reports a hit whenever a slot happens
 * to hold an address that was freed at SOME point in the run, including after
 * malloc has recycled that address for a live node. This design asks a question
 * with no time dimension: "at the instant this tree is released, does one of the
 * seven slots point INTO it?" A recycled address cannot answer that.
 *
 * THE NEGATIVE CONTROL IS NOT OPTIONAL
 * ------------------------------------
 * A probe that cannot fire has never been tested.
 * `bash tools/alias_probe.sh --negative-control <outdir> <suite>...` reverts one
 * landed lifetime fix, recompiles that translation unit fresh, and REQUIRES a
 * non-zero hit count. Measured: 22 hits over parity_cce_gaps + parity_int_gaps
 * with the pca_resolve_entry keep-alive transfer reversed, against 0 with it in
 * place over the same suites.
 *
 * NO SANITIZER RAN. gcc 15.3.0 (MSYS2/PE) in this tree has no asan, no ubsan,
 * no valgrind and no gdb. See ALIAS_PROBE.md. This probe is the substitute.
 */

#include "rabuka.h"

#include <stdio.h>

/* The two look.c parked slots. They were `static`, so no linker could name
 * them; the conversion to OWNING CLONES added two read-only accessors in
 * look.c (rb_look_slot_pending_select / rb_look_slot_reprompt_followup)
 * precisely so this probe -- and tests/test_alias_ownership.c -- can ASSERT on
 * what is parked instead of inferring it. An earlier revision of this probe
 * worked around `static` by compiling a second copy of look.c with the two
 * names macro-renamed; the accessors removed that need, so the probe now links
 * the ordinary look.o and measures the shipped object. */
const AbilityEffect *rb_look_slot_pending_select(void);
const AbilityEffect *rb_look_slot_reprompt_followup(void);

void __real_rb_effect_free(AbilityEffect *e);
void __real_rb_free_ability(Ability *a);
int  __real_rb_has_pending_choice(const GameState *g);

/* ── state ─────────────────────────────────────────────────────────────── */

static int subtree_contains(const AbilityEffect *root, const AbilityEffect *needle);

static const GameState *ap_g;          /* last g seen by the predicate */
static long ap_frees;                  /* trees this probe saw released */
static long ap_frees_with_prompt;      /* ... while a prompt was open */
static long ap_hits;                   /* ALIAS HITS: the number that must stay 0 */
static long ap_reports_detailed;       /* hits reported with the full payload */

#define AP_MAX_REPORTED 40

static void ap_report(const char *slot, const AbilityEffect *slot_value,
                      const AbilityEffect *root)
{
    ap_hits++;
    if (ap_hits <= AP_MAX_REPORTED) {
        ap_reports_detailed++;
        fprintf(stderr,
                "ALIAS_HIT n=%ld slot=%s slot_value=%p freed_root=%p action=%s text=%s\n",
                ap_hits, slot, (const void *)slot_value, (const void *)root,
                (slot_value && slot_value->action) ? slot_value->action : "<null>",
                (slot_value && slot_value->text) ? slot_value->text : "<null>");
    } else {
        /* Keep printing past the cap, one line per hit but without the payload:
           the LINE COUNT is the measurement, so suppressing lines would make a
           real defect look like a smaller one. */
        fprintf(stderr,
                "ALIAS_HIT n=%ld slot=%s slot_value=%p freed_root=%p (detail suppressed)\n",
                ap_hits, slot, (const void *)slot_value, (const void *)root);
    }
    fflush(stderr);
}

/* ── the seven slots, named ────────────────────────────────────────────── */

static void ap_check_slots(const AbilityEffect *root)
{
    if (!ap_g || !root) return;
    if (!ap_g->queue.has_pending) return;   /* the "while a prompt is open" half */
    ap_frees_with_prompt++;

    const AbilityEffect *s;
    if ((s = ap_g->queue.resume_eff) && subtree_contains(root, s))
        ap_report("queue.resume_eff", s, root);
    if ((s = ap_g->queue.resume_after_look) && subtree_contains(root, s))
        ap_report("queue.resume_after_look", s, root);
    if ((s = rb_look_slot_pending_select()) && subtree_contains(root, s))
        ap_report("look.g_look_pending_select", s, root);
    if ((s = rb_look_slot_reprompt_followup()) && subtree_contains(root, s))
        ap_report("look.g_look_reprompt_followup", s, root);
    if ((s = ap_g->queue.target_selection_eff) && subtree_contains(root, s))
        ap_report("queue.target_selection_eff", s, root);
    if ((s = ap_g->queue.resume_parent) && subtree_contains(root, s))
        ap_report("queue.resume_parent", s, root);
    if ((s = ap_g->queue.deferred) && subtree_contains(root, s))
        ap_report("queue.deferred", s, root);
}

/* ── subtree membership, mirroring rb_effect_free's release set ─────────── */

static int subtree_contains(const AbilityEffect *root, const AbilityEffect *needle)
{
    if (!root || !needle) return 0;
    if (root == needle) return 1;
    for (int i = 0; i < root->n_child; i++)
        if (subtree_contains(root->child[i], needle)) return 1;
    for (int i = 0; i < root->n_options; i++)
        if (subtree_contains(root->options[i], needle)) return 1;
    if (subtree_contains(root->primary_effect, needle))     return 1;
    if (subtree_contains(root->alternative_effect, needle)) return 1;
    if (subtree_contains(root->look_action, needle))         return 1;
    if (subtree_contains(root->select_action, needle))      return 1;
    if (subtree_contains(root->followup_action, needle))     return 1;
    if (subtree_contains(root->optional_action, needle))     return 1;
    if (subtree_contains(root->conditional_action, needle))  return 1;
    if (subtree_contains(root->gained_effect, needle))       return 1;
    if (subtree_contains(root->resource_on_select, needle))  return 1;
    if (subtree_contains(root->opponent_action, needle))     return 1;
    return 0;
}

/* ── the wraps ─────────────────────────────────────────────────────────── */

void __wrap_rb_effect_free(AbilityEffect *e)
{
    ap_frees++;
    ap_check_slots(e);
    __real_rb_effect_free(e);
}

void __wrap_rb_free_ability(Ability *a)
{
    /* effect_free (vm.c:443, static) is about to release BOTH subtrees. The
     * check runs per subtree so a report names the one that actually aliased. */
    ap_frees++;
    if (a) ap_check_slots(a->cost);
    if (a) ap_check_slots(a->effect);
    __real_rb_free_ability(a);
}

int __wrap_rb_has_pending_choice(const GameState *g)
{
    ap_g = g;
    return __real_rb_has_pending_choice(g);
}

/* ── banner + summary ──────────────────────────────────────────────────── */

__attribute__((constructor))
static void ap_start(void)
{
    fprintf(stderr,
            "ALIAS_PROBE_READY slots=7 wraps=rb_effect_free,rb_free_ability,"
            "rb_has_pending_choice\n");
    fflush(stderr);
}

/* WHY THE RUNNER COUNTS ALIAS_HIT LINES AND NOT THIS COUNTER.
 * Several suites (parity_cce_gaps:3551, parity_required_hearts) run each case
 * in a FORKED child. A child that leaves through _exit() never reaches a
 * destructor, so the parent's own hits reads 0 while the children's hits are
 * already in the shared stderr. The exit line is therefore only ever a LOWER
 * BOUND, and the authoritative number is the count of ALIAS_HIT lines. The
 * runner prints both and flags any disagreement rather than quietly reporting
 * the smaller one: a suite whose summary says 0 while its log carries hit lines
 * is exactly the "measured nothing but looks clean" trap this probe exists to
 * survive. */
__attribute__((destructor))
static void ap_finish(void)
{
    fprintf(stderr,
            "ALIAS_PROBE frees=%ld frees_with_prompt_open=%ld hits=%ld detailed=%ld\n",
            ap_frees, ap_frees_with_prompt, ap_hits, ap_reports_detailed);
    fflush(stderr);
}