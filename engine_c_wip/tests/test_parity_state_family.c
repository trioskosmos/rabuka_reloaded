/* test_parity_state_family.c - C parity suite for the STATE effect family.
 *
 *   engine/src/ability/effects/change_state.rs   (the former state.rs; renamed
 *       in commit cf125b15 "refactor(engine): reorganize ability effects into
 *       per-kind modules")
 *   engine/src/ability/effects/score.rs
 *   engine/src/ability/effects/ability_ops.rs    (gain_ability_from_source only)
 *
 * Every case cites the Rust line it asserts. Nothing here was weakened: the
 * file is new and every CHECK is a positive contract.
 *
 * -- why this file exists ---------------------------------------------------
 * The 2026-10-03 exhaustive audit of the state family found eight places where
 * the C disagreed with Rust inside `rb_effect_change_state`
 * (src/ability/effects/state.c), plus two divergent C copies of one Rust
 * function (`gain_ability_from_source`, src/ability/effects/ability.c and
 * ability_effects.c). The fix that landed with the audit adds a
 * `recalculate_constants()` immediately after the orientation write
 * (change_state.rs:662-669); the A/B over 63 suites moved
 * test_parity_condition_and_effect from 51 FAIL lines to 50 and nothing else,
 * because NO suite asserted the other seven. This file is where they get an
 * assertion each, so the next agent cannot silently revert them.
 *
 * ASCII-only on purpose: the Japanese card text a few cases need is written as
 * \xNN escapes so the source survives any toolchain codepage.
 *
 * -- harness facts (established by peers; copied verbatim) --------------------
 *   `rb_load("src")` fails under an isolated out-of-tree build, so every test
 *   binary needs the `../cards/build` fallback.
 *   Rust `select_indices(&[i])` -> rb_resume_with_choice(g, i)
 *   Rust `select_indices(&[])`  -> rb_resume_with_choice(g, -1)  (decline)
 *   Every drain is BOUNDED (AGENTS.md: an unbounded drain spins forever on a
 *   prompt the engine refuses to consume).
 *   `test_id()` ALIASES a shared template - measured, not assumed:
 *   rb_find_card_by_no("PL!S-bp5-016-R") and rb_find_card_by_no(
 *   "PL!S-bp5-016-N") both return 1740, so those are DIFFERENT cards sharing one
 *   lookup. Nothing here asserts identity through test_id().
 *   `sizeof(GameState)` is ~781 KB, so every TestGame is `static`.
 *
 * Build:  make t T=parity_state_family
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

/* ── per-test verdict layer (see test_parity_integration.c for the rationale):
 * a FLAT suite's FAIL lines name an assertion, never a test, so this suite
 * reported no per-test verdict and its FAIL lines were unattributable. This
 * adds exactly one `FAILED: <test body>` per failing test body, in the shape
 * every FORK suite prints. No assertion is added, removed or relaxed. */
static const char *g_flat_failed[512];
static int g_flat_failed_n;
static void flat_verdict_failed(const char *fn)
{
    for (int i = 0; i < g_flat_failed_n; i++)
        if (g_flat_failed[i] == fn) return;
    if (g_flat_failed_n >= 512) return;
    g_flat_failed[g_flat_failed_n++] = fn;
    printf("FAILED: %s\n", fn);
    fflush(stdout);
}
#define FLAT_VERDICT() flat_verdict_failed(__func__)

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    checks++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        FLAT_VERDICT(); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* =======================================================================
 * 0. harness helpers
 * ======================================================================= */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

static void reset_board(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *p = &tg->state.p[pl];
        p->deck.n = 0;
        p->hand.n = 0;
        p->discard.n = 0;
        p->live.n = 0;
        p->success.n = 0;
        for (int a = 0; a < RB_STAGE_SIZE; a++) {
            p->stage[a] = -1;
            p->under_cards[a].n = 0;
            p->stage_wait[a] = 0;
        }
    }
    tg->state.n_revealed = 0;
    tg->state.n_selected_cards = 0;
    tg->state.n_recently_moved = 0;
    tg->state.n_cards_activated_this_turn = 0;
    tg->state.n_turn_state_changes = 0;
    tg->state.n_recently_state_changed = 0;
    tg->state.activating_card = -1;
    tg->state.n_prohibition = 0;
    tg->state.n_cannot_active_cards = 0;
    tg->state.player_cannot_activate[0] = 0;
    tg->state.player_cannot_activate[1] = 0;
}

/* Two DIFFERENT prints. PL!HS-sd1-008-SD carries two abilities (debut,
 * live_start) and PL!HS-sd1-010-SD carries none - measured, so the under-card
 * fixture in section E is known to have something to copy. */
static int member_with_abilities(TestGame *tg) { return test_id(tg, "PL!HS-sd1-008-SD"); }
static int member_without_abilities(TestGame *tg) { return test_id(tg, "PL!HS-sd1-010-SD"); }

static void set_extra(AbilityEffect *e, const char *k, const char *v)
{
    if (e->n_extra >= RB_MAX_EXTRA) return;
    e->extra_k[e->n_extra] = (char *)k;
    e->extra_v[e->n_extra] = (char *)v;
    e->n_extra++;
}

/* The `change_state` shape the parser emits for a printed wait / activate step. */
static AbilityEffect mk_change_state(const char *state_change, const char *target,
                                     const char *count_text)
{
    AbilityEffect e;
    memset(&e, 0, sizeof e);
    e.action = (char *)"change_state";
    e.target = (char *)target;
    e.source = (char *)"stage";
    memcpy(e.card_type_field, "member_card", 12);
    e.count = count_text ? atoi(count_text) : 0;
    set_extra(&e, "state_change", state_change);
    return e;
}

static int pending(TestGame *tg) { return rb_has_pending_choice(&tg->state); }
static void answer(TestGame *tg, int idx)
{
    if (rb_has_pending_choice(&tg->state)) rb_resume_with_choice(&tg->state, idx);
}
/* BOUNDED drain, per AGENTS.md. Returns how many prompts it answered. */
static int drain_skip(TestGame *tg, int guard_max)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < guard_max) answer(tg, -1);
    return guard;
}

static int is_waited(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    return o && !strcmp(o, "wait");
}

static int activated_this_turn_contains(TestGame *tg, int cid)
{
    for (int i = 0; i < tg->state.n_cards_activated_this_turn; i++)
        if (tg->state.cards_activated_this_turn[i] == cid) return 1;
    return 0;
}

/* =======================================================================
 * A. A `state_change = "active"` step re-derives the constant modifiers
 *    BEFORE the auto-ability sweep.
 *
 * change_state.rs:662-669
 *     // Conditional constants read wait/active orientations, so they must be
 *     // re-evaluated as soon as the orientation change is applied -- otherwise
 *     // a card whose own effect satisfies its own constant-ji stays stale.
 *     gs.recalculate_constants();
 * and then :762-768 sweeps BOTH seats:
 *     gs.trigger_auto_abilities_for_player(&p1);
 *     gs.trigger_auto_abilities_for_player(&p2);
 *
 * rb_recalc_constants (triggers.c:1326-1343) works by CLEARING every
 * `constant_*` accumulator and re-deriving them, so a hand-planted constant is a
 * direct witness that the recalculation ran: it cannot survive the call. That
 * makes this case assert the recalc itself rather than one card's rider.
 *
 * The pre-existing end-of-play recalc (engine.c:1061) fires AFTER
 * rb_drain_ability_queue, far too late for the sweep immediately below it in
 * the Rust source; before the fix the C had no recalc anywhere in
 * rb_effect_change_state at all.
 *
 * MEASURED: 69 of the 96 effect-side `change_state` nodes are member steps and
 * all of them reach this line.
 * ======================================================================= */

static void test_state_change_recalculates_constants_immediately(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int a = member_with_abilities(&tg);
    CHECK(a >= 0, "the recalculation fixture resolves");
    if (a < 0) return;
    reset_board(&tg);
    tg.state.p[0].stage[0] = a;

    /* Plant a constant that only a recalculation can clear. */
    tg.state.mods.constant_score[a] = 7;
    CHECK_EQ(tg.state.mods.constant_score[a], 7,
             "setup: a planted constant_score is present before the step");

    AbilityEffect eff = mk_change_state("wait", "self", "1");
    tg.state.activating_card = a;
    rb_effect_change_state(&tg.state, 0, &eff, a);
    drain_skip(&tg, 16);

    CHECK(is_waited(&tg, a), "the member really did change state");
    CHECK_EQ(tg.state.mods.constant_score[a], 0,
             "change_state.rs:662-669: the orientation write is followed by a constant recalculation");
}

/* =======================================================================
 * B. A `state_change = "active"` step records the per-turn ACTIVATION history.
 *
 * change_state.rs:653-659
 *     if state_change == "active"
 *         && !gs.cards_activated_this_turn.contains(card_id) {
 *         gs.cards_activated_this_turn.push(*card_id);
 *     }
 *
 * The C carried the FIELD (rabuka.h:1495-1496, added by audit A8) but no
 * WRITER anywhere in src/**: rb_effect_change_state is the only place Rust
 * writes it, and the field therefore stayed permanently empty, so the
 * `activated_this_turn` condition (condition/card.rs:2417-2424, 3 nodes in the
 * 985-ability corpus) could never be true. 47 of the 96 effect-side
 * `change_state` nodes carry state_change="active".
 *
 * It is a SET, not a tally: activating the same member twice in one turn leaves
 * exactly one entry (the `.contains()` guard).
 * ======================================================================= */

static void test_active_step_records_the_activation_history(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int a = member_with_abilities(&tg);
    int b = member_without_abilities(&tg);
    CHECK(a >= 0 && b >= 0 && a != b, "two distinct members resolve");
    if (a < 0 || b < 0 || a == b) return;
    reset_board(&tg);
    tg.state.p[0].stage[0] = a;
    tg.state.p[0].stage[1] = b;

    CHECK_EQ(tg.state.n_cards_activated_this_turn, 0,
             "the per-turn activation history starts empty");

    AbilityEffect wait2 = mk_change_state("wait", "self", "2");
    tg.state.activating_card = a;
    rb_effect_change_state(&tg.state, 0, &wait2, a);
    drain_skip(&tg, 16);
    CHECK(is_waited(&tg, a) && is_waited(&tg, b),
          "a wait step waits both members");
    CHECK_EQ(tg.state.n_cards_activated_this_turn, 0,
             "a WAIT step records nothing (change_state.rs:653 keys on \"active\")");

    AbilityEffect back = mk_change_state("active", "self", "2");
    rb_effect_change_state(&tg.state, 0, &back, a);
    drain_skip(&tg, 16);
    CHECK(!is_waited(&tg, a) && !is_waited(&tg, b),
          "change_state.rs:390-391: an \"active\" step only re-activates already-WAITED members, and it did");
    CHECK_EQ(tg.state.n_cards_activated_this_turn, 2,
             "change_state.rs:655-659: the activate step records BOTH members");
    CHECK(activated_this_turn_contains(&tg, a) && activated_this_turn_contains(&tg, b),
          "the history holds the exact card ids that changed state");

    /* The `.contains()` guard: a second activate of the same member must not add
     * a second entry, so the history counts MEMBERS, not EVENTS. */
    rb_effect_change_state(&tg.state, 0, &back, a);
    drain_skip(&tg, 16);
    CHECK_EQ(tg.state.n_cards_activated_this_turn, 2,
             "change_state.rs:655: re-activating an already-recorded member does not duplicate it");
}

/* The history has no seat: an OPPONENT-targeted activate records the member it
 * touched. The `activated_this_turn` CONDITION itself lives in
 * src/ability/condition.c and is still BLOCKED-ON there; this asserts the
 * writer half, which is state.c's. */
static void test_opponent_targeted_activate_records_the_history(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int a = member_with_abilities(&tg);
    int b = member_without_abilities(&tg);
    CHECK(a >= 0 && b >= 0 && a != b, "two distinct members resolve");
    if (a < 0 || b < 0 || a == b) return;
    reset_board(&tg);
    tg.state.p[1].stage[0] = a;                 /* the card sits on P2's stage */

    tg.state.activating_card = a;
    AbilityEffect wait1 = mk_change_state("wait", "opponent", "1");
    rb_effect_change_state(&tg.state, 0, &wait1, a);
    drain_skip(&tg, 16);
    CHECK(is_waited(&tg, a), "an opponent-targeted wait reached P2's member");

    AbilityEffect back = mk_change_state("active", "opponent", "1");
    rb_effect_change_state(&tg.state, 0, &back, a);
    drain_skip(&tg, 16);
    CHECK(!is_waited(&tg, a), "the opponent-targeted activate brought it back");
    CHECK(activated_this_turn_contains(&tg, a),
          "change_state.rs:653-659: an OPPONENT-targeted activate records the member too");
}

/* =======================================================================
 * C. `per_unit` over an empty zone changes nothing and asks nothing.
 *
 * change_state.rs:143-145
 *     if effect.per_unit_any().unwrap_or(false) && count == 0 { return Ok(()); }
 *
 * A per-unit step scales by how many cards its zone holds, so a one-card step
 * over a zone with nothing in it resolves to zero and must be a complete no-op.
 * The C computed the zero and then carried on through the deck->energy
 * short-circuit, the member branch and the "no candidates" exit.
 *
 * MEASURED: `per_unit` is on 5 of the 96 effect-side `change_state` nodes
 * (per_unit_type \xE6\xAE\x8B and per-member, per_unit_source previous_moved_cards), so the exit
 * is reachable. The fix is not separately observable from outside -- the old
 * fall-through and the new early return leave the board identical, because the
 * only extra thing the old path did was the recalculation -- so this case locks
 * the CONTRACT rather than reproducing the defect. What it forbids is the two
 * things a per-unit-zero step must never do: prompt, and touch a card.
 * ======================================================================= */

static AbilityEffect mk_per_unit_wait(void)
{
    AbilityEffect e = mk_change_state("wait", "self", "1");
    set_extra(&e, "per_unit", "true");
    set_extra(&e, "per_unit_type", "\xE6\xAE\x8B");            /* \xE6\xAE\x8B */
    set_extra(&e, "location", "live_card_zone");
    set_extra(&e, "value", "1");
    return e;
}

static void test_per_unit_over_an_empty_zone_is_a_total_no_op(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int a = member_with_abilities(&tg);
    int b = member_without_abilities(&tg);
    CHECK(a >= 0 && b >= 0 && a != b, "two distinct members resolve");
    if (a < 0 || b < 0 || a == b) return;
    reset_board(&tg);
    tg.state.p[0].stage[0] = a;

    AbilityEffect eff = mk_per_unit_wait();
    tg.state.activating_card = a;
    rb_effect_change_state(&tg.state, 0, &eff, a);
    int prompts = drain_skip(&tg, 16);

    CHECK_EQ(prompts, 0,
             "change_state.rs:143-145: a per-unit step with no card in its zone asks nothing");
    CHECK(!is_waited(&tg, a),
          "change_state.rs:143-145: ...and waits nothing");
    CHECK_EQ(tg.state.n_turn_state_changes, 0,
             "...and records no state transition");
    CHECK_EQ(tg.state.n_recently_state_changed, 0,
             "...and pushes nothing onto recently_state_changed");
    CHECK_EQ(tg.state.n_selected_cards, 0,
             "...and adds nothing to the selection");

    /* Accepting arm: the SAME effect with a card in the live-card zone does act,
     * so a green negative above can never be a dead gate. */
    tg.state.p[0].live.cards[tg.state.p[0].live.n++] = b;
    AbilityEffect eff2 = mk_per_unit_wait();
    rb_effect_change_state(&tg.state, 0, &eff2, a);
    drain_skip(&tg, 16);
    CHECK(is_waited(&tg, a) || tg.state.n_turn_state_changes > 0,
          "the same per-unit step does act once the live-card zone holds a card");
}

/* =======================================================================
 * D. KNOWN OPEN GAP -- `changed_state_members` (RED ON PURPOSE, do NOT weaken).
 *
 * change_state.rs:684-687
 *     // Record the members this step changed so a following delayed
 *     // restriction step can key its flags on exactly these victims.
 *     self.selection.changed_state_members =
 *         actual_targets.iter().map(|(_, cid)| *cid).collect();
 *
 * The WRITER is now in place -- rb_effect_change_state calls
 * rb_rules_note_changed_state_members (state.c), which fills the same slot
 * rules.c:2271 writes. The READER is unreachable:
 *
 *   - executor.c routes the `restriction` action to execute_misc ->
 *     misc.c:2059 -> misc.c:887 h_restriction.
 *   - h_restriction computes the delayed cannot-activate victims ITSELF
 *     (misc.c:905-918): first g->recently_moved, then the activating card.
 *     It has no `changed_state_members` arm at all.
 *   - rules.c:1819 rb_rules_execute_restriction DOES have the arm
 *     (rules.c:1848-1866, with the right precedence) and has ZERO callers
 *     tree-wide, as do rb_rules_execute_activation_restriction,
 *     rb_rules_execute_re_yell and rb_rules_execute_modify_yell_count.
 *
 * So for a wait effect that moved nothing, n_recently_moved is empty and the
 * flag lands on the ACTIVATING card instead of on the members that were waited.
 *
 * BLOCKED-ON src/ability/effects/misc.c:905 -- insert the same precedence the
 * rules.c copy already has, ahead of the recently_moved loop:
 *     if (st->n_changed_state_members > 0) { for each cid ->
 *         rb_mods_add_delayed_cannot_active(&g->mods, cid, 1);
 *         st->n_changed_state_members = 0; }
 *     else if (g->n_recently_moved > 0) { ...unchanged... }
 * (st is a rules.c file-static, so misc.c also needs the slot lifted to an
 * exported accessor, or rules.c:1819 made the live dispatch target instead.)
 *
 * REACHABILITY: `restriction` is on 14 of the 985 abilities, and the delayed
 * cannot-activate clause is the "that member cannot be active next turn" form.
 * Until misc.c is fixed these two lines are the suite's red.
 * ======================================================================= */

static void test_delayed_restriction_keys_on_the_changed_members(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int a = member_with_abilities(&tg);
    int b = member_without_abilities(&tg);
    CHECK(a >= 0 && b >= 0 && a != b, "two distinct members resolve");
    if (a < 0 || b < 0 || a == b) return;
    reset_board(&tg);
    tg.state.p[0].stage[0] = a;
    tg.state.p[0].stage[1] = b;

    /* A wait effect that moves no card: n_recently_moved stays empty, so the
     * live handler has nothing to key on and falls through to the activating
     * card -- which this case deliberately leaves unset. */
    AbilityEffect eff = mk_change_state("wait", "self", "2");
    tg.state.activating_card = -1;
    tg.state.n_recently_moved = 0;
    rb_effect_change_state(&tg.state, 0, &eff, -1);
    drain_skip(&tg, 16);
    CHECK(is_waited(&tg, a) && is_waited(&tg, b), "both members are waited");
    CHECK_EQ(tg.state.n_recently_moved, 0,
             "the wait step moved no card, so the recently-moved fallback is empty");

    AbilityEffect res;
    memset(&res, 0, sizeof res);
    res.action = (char *)"restriction";
    set_extra(&res, "restriction_type", "cannot_activate_by_effect");
    set_extra(&res, "delayed", "true");
    int resolved = 0;
    rb_execute_misc_effect(&tg.state, 0, &tg.state.p[0], &res, &resolved);
    drain_skip(&tg, 16);

    CHECK_EQ(rb_mods_is_delayed_cannot_active(&tg.state.mods, a), 1,
             "OPEN GAP (misc.c:905): the FIRST waited member carries the flag");
    CHECK_EQ(rb_mods_is_delayed_cannot_active(&tg.state.mods, b), 1,
             "OPEN GAP (misc.c:905): the SECOND waited member carries the flag too");
}

/* =======================================================================
 * E. `gain_ability_from_source` treats trigger_filter as a LIST.
 *
 * ability_ops.rs:679-681
 *     if let Some(filters) = effect.trigger_filter_any() {
 *         if !filters.iter().any(|f| ability.triggers.as_ref()
 *                 .is_some_and(|t| t.contains(f.as_str()) || f.contains(t.as_str()))) {
 *             continue;
 *         }
 *     }
 *
 * Until 2026-10-03 there were TWO live C copies of that function and they
 * disagreed here: ability_effects.c split the CSV on ',' and tested each token
 * (right), ability.c compared the WHOLE comma-joined string against the whole
 * trigger string with two strstr calls (wrong). Both corpus nodes carry a
 * single-token filter, so nothing real diverged -- but a two-token filter would
 * have registered nothing on the constant-ji path and everything on the executor path.
 * They are now ONE implementation (ability_effects.c
 * rb_ability_gain_from_source_core) reached from both call sites.
 *
 * MEASURED: `gain_ability_from_source` = 2 nodes of 985 and
 * rb_canonical_trigger says both are `constant`, so the constant-ji route is the one
 * exercised here.
 *
 * rb_gain_ability_from_source is declared by include/rabuka.h:2184.
 * ======================================================================= */

static void copy_cards_triggers(int cid, char buf[2][128], int *out_n)
{
    *out_n = 0;
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n && *out_n < 2; i++) {
        Ability ab;
        memset(&ab, 0, sizeof ab);
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        buf[*out_n][0] = 0;
        if (ab.triggers && *ab.triggers)
            snprintf(buf[*out_n], 128, "%s", ab.triggers);
        rb_free_ability(&ab);
        if (buf[*out_n][0]) (*out_n)++;
    }
}

static void test_gain_ability_from_source_trigger_filter_is_a_list(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int with_ab = member_with_abilities(&tg);
    int without_ab = member_without_abilities(&tg);
    CHECK(with_ab >= 0 && without_ab >= 0 && with_ab != without_ab,
          "two distinct members resolve");
    if (with_ab < 0 || without_ab < 0 || with_ab == without_ab) return;
    reset_board(&tg);
    tg.state.p[0].stage[0] = without_ab;                 /* the activating member */
    tg.state.p[0].under_cards[0].cards[tg.state.p[0].under_cards[0].n++] = with_ab;

    int source_abilities = rb_card_num_abilities((uint32_t)with_ab);
    CHECK(source_abilities >= 2,
          "the under-card carries at least two abilities, so a multi-token filter is meaningful");
    if (source_abilities < 2) return;

    char trig[2][128];
    int ntrig = 0;
    copy_cards_triggers(with_ab, trig, &ntrig);
    CHECK_EQ(ntrig, 2, "both abilities of the under-card carry a trigger string");
    if (ntrig < 2) return;

    tg.state.activating_card = without_ab;

    /* A filter naming BOTH of the source card's triggers, as a CSV. Under Rust's
     * `any(|f| ...)` this keeps every ability whose trigger matches either token.
     * Under the old ability.c body the whole "T0,T1" string had to appear inside
     * the trigger text, so nothing matched and the gained set stayed empty. */
    char csv[300];
    snprintf(csv, sizeof csv, "%s,%s", trig[0], trig[1]);

    AbilityEffect eff;
    memset(&eff, 0, sizeof eff);
    eff.action = (char *)"gain_ability_from_source";
    set_extra(&eff, "trigger_filter", csv);

    rb_gain_ability_from_source(&tg.state, 0, &eff, without_ab);
    CHECK_EQ(rb_card_num_gained_abilities(&tg.state, without_ab), source_abilities,
             "ability_ops.rs:679-681: a two-token trigger_filter keeps every ability matching EITHER token");

    /* Re-running REPLACES the copy set rather than stacking it:
     * ability_ops.rs:614-619 clears the previous set first. */
    rb_gain_ability_from_source(&tg.state, 0, &eff, without_ab);
    CHECK_EQ(rb_card_num_gained_abilities(&tg.state, without_ab), source_abilities,
             "ability_ops.rs:614-619: re-resolving replaces the set instead of stacking it");

    /* A filter that matches nothing registers nothing. */
    AbilityEffect none;
    memset(&none, 0, sizeof none);
    none.action = (char *)"gain_ability_from_source";
    set_extra(&none, "trigger_filter", "NO_SUCH_TRIGGER_TOKEN");
    rb_gain_ability_from_source(&tg.state, 0, &none, without_ab);
    CHECK_EQ(rb_card_num_gained_abilities(&tg.state, without_ab), 0,
             "a trigger_filter that matches no ability copies nothing");
}

/* =======================================================================
 * F. KNOWN OPEN GAP -- `modify_score` on live_total must REFUSE
 *    `operation: "set"` (RED ON PURPOSE, do NOT weaken).
 *
 * score.rs:274-287
 *     if operation == "set" {
 *         log::error!("... is not implemented and would silently behave as
 *                      'add' ...");
 *         return Ok(());            // <-- applies NOTHING
 *     }
 * and score.rs:251-256 computes the `"set"` arm of the delta match and then
 * never uses it, because the `return Ok(())` above fires first.
 *
 * The C applies it as a delta instead, so a live-total "set" MOVES the total.
 *
 * BLOCKED-ON tests/test_parity_score_effects.c:1125 -- that suite asserts the
 * OPPOSITE ("live_total operation=set adds 4 onto the existing +1
 * (score.rs:142-146)"), citing line numbers from a pre-split score.rs. Its
 * expectation has to be dropped before this branch can be corrected; adding the
 * refusal was tried during the audit and reverted because it turned that suite
 * red (0 -> 1 FAIL lines) for zero reachable card.
 *
 * MEASURED: `operation` = "set" occurs on 1 of the 130 `modify_score` nodes and
 * that node targets a CARD, not live_total, so `set` on live_total is
 * 0 of 985. 36 of the 130 DO target live_total, so the arm itself is firmly live
 * and the add / remove arms below are the reachable part.
 * ======================================================================= */

static int live_total_bonus(TestGame *tg)
{
    return (int)tg->state.mods.p1_constant_total_score_bonus;
}

static void test_live_total_set_is_refused(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int a = member_with_abilities(&tg);
    CHECK(a >= 0, "the live-total fixture resolves");
    if (a < 0) return;
    reset_board(&tg);
    tg.state.p[0].live.cards[tg.state.p[0].live.n++] = a;
    rb_recalc_constants(&tg.state);
    int before = live_total_bonus(&tg);
    CHECK_EQ(before, 0, "a bare live card carries no constant live-total bonus to begin with");

    AbilityEffect add;
    memset(&add, 0, sizeof add);
    add.action = (char *)"modify_score";
    add.target = (char *)"live_total";
    set_extra(&add, "operation", "add");
    set_extra(&add, "value", "5");
    rb_execute_modify_score(&tg.state, 0, &add);
    drain_skip(&tg, 16);
    int after_add = live_total_bonus(&tg);
    CHECK_EQ(after_add, before + 5,
             "score.rs:251-256: an ADD on live_total moves the total");

    AbilityEffect set;
    memset(&set, 0, sizeof set);
    set.action = (char *)"modify_score";
    set.target = (char *)"live_total";
    set_extra(&set, "operation", "set");
    set_extra(&set, "value", "99");
    rb_execute_modify_score(&tg.state, 0, &set);
    drain_skip(&tg, 16);
    CHECK_EQ(live_total_bonus(&tg), after_add,
             "OPEN GAP (test_parity_score_effects.c:1125): a SET on live_total is refused and applies nothing");

    /* score.rs:251-256 -- a REMOVE applies a negative delta. Re-derived from a
     * clean board, because the `set` probe above may still have moved the total
     * (that is the open gap) and this arm must not inherit its damage. */
    reset_board(&tg);
    tg.state.p[0].live.cards[tg.state.p[0].live.n++] = a;
    rb_recalc_constants(&tg.state);
    rb_execute_modify_score(&tg.state, 0, &add);
    drain_skip(&tg, 16);
    int base = live_total_bonus(&tg);

    AbilityEffect rem;
    memset(&rem, 0, sizeof rem);
    rem.action = (char *)"modify_score";
    rem.target = (char *)"live_total";
    set_extra(&rem, "operation", "remove");
    set_extra(&rem, "value", "2");
    rb_execute_modify_score(&tg.state, 0, &rem);
    drain_skip(&tg, 16);
    CHECK_EQ(live_total_bonus(&tg), base - 2,
             "score.rs:252: a REMOVE on live_total applies a negative delta");
}

/* ======================================================================= */

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    test_state_change_recalculates_constants_immediately();

    test_active_step_records_the_activation_history();
    test_opponent_targeted_activate_records_the_history();

    test_per_unit_over_an_empty_zone_is_a_total_no_op();

    test_delayed_restriction_keys_on_the_changed_members();

    test_gain_ability_from_source_trigger_filter_is_a_list();

    test_live_total_set_is_refused();

    rb_unload();
    printf("\n%d checks, %d failures\n", checks, failures);
    if (failures == 0) printf("ALL STATE-FAMILY PARITY CHECKS PASSED\n");
    else printf("STATE-FAMILY PARITY: %d FAIL line(s) - see the OPEN GAP markers above\n", failures);
    return failures == 0 ? 0 : 1;
}