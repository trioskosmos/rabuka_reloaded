/* tests/test_parity_need_heart_live_flow.c
 *
 * E3 + E4 and the E4 PRODUCER, pinned at the positions Rust puts them.
 * Cited by CONTENT and by line, read from the ORACLE tree at
 * engine/ (engine/src/core/game_state/modifiers.rs, engine/src/turn/live.rs,
 * engine/src/turn/trigger_scan.rs, engine/src/turn/phases.rs). The oracle is
 * never edited.
 *
 * THE TWO FUNCTIONS UNDER TEST
 *
 *   E3  evaluate_success_zone_constant_abilities        modifiers.rs:2147-2150
 *         pub fn evaluate_success_zone_constant_abilities(&mut self) {
 *             self.mods.need_heart_modifiers.clear();
 *             self.evaluate_success_zone_constant_modifiers();
 *         }
 *
 *   E4  restore_performance_need_heart_modifiers         modifiers.rs:2159-2182
 *         for snap in &self.performance_snapshots {
 *             for &(cid, color, ref entry) in &snap.performance_need_heart_modifiers {
 *                 if !restored.insert((cid, color)) { continue }
 *                 let target = ...need_heart_modifiers.entry(cid).or_default()
 *                                .entry(color).or_default();
 *                 if entry.set != 0 && target.set == 0 { target.set = entry.set; }
 *                 target.additive += entry.additive;
 *             }
 *         }
 *
 *   E4's PRODUCER                                          phases.rs:320-333
 *         let nhm_flat: Vec<(i16, HeartColor, ModifierEntry)> = game_state
 *             .mods.need_heart_modifiers.iter()
 *             .flat_map(|(&card_id, colors)| colors.iter()
 *                 .map(move |(&color, entry)| (card_id, color, *entry)))
 *             .collect();
 *       handed to build_snapshot at phases.rs:621-632 (:627 passes &nhm_flat)
 *       and stored verbatim at live.rs:3179
 *         performance_need_heart_modifiers: performance_need_heart_modifiers.to_vec(),
 *
 *   E3 + E4 CALL SITES -- the only two, both in the LIVE FLOW
 *       engine/src/turn/live.rs:1326-1327   (execute_live_victory_determination)
 *       engine/src/turn/trigger_scan.rs:360,365 (trigger_live_success_abilities)
 *
 * WHY THIS SUITE EXISTS. E3 and E4 landed in src/core/modifiers.c with ZERO
 * callers, so the whole need_heart withdrawal mechanism was dead code. The C's
 * rb_recalc_constants cannot substitute: Rust's recalculate_constants calls E1
 * (modifiers.rs:1398 `self.evaluate_success_zone_constant_modifiers()`) and NEVER
 * E3, and the constant_need_heart prologue in rb_recalc_constants is
 * `modify_required_hearts_global`'s ledger (Rust constant_global_need_heart,
 * game_modifiers.rs:194) -- a DIFFERENT action from the plain
 * `modify_required_hearts` that E1's need_heart arm dispatches
 * (src/core/modifiers.c sz_apply_effect -> rb_execute_modify_required_hearts).
 * So E1's grant has no withdrawal anywhere except E3's wholesale clear, and E3
 * runs only from the two live-flow sites. This file pins all of that.
 *
 * WHAT IS ASSERTED IS WHAT RUST ASSERTS, AT RUST'S LINES:
 *   1  the producer records the WHOLE map into the pushed snapshot (phases.rs:320,
 *      :627; live.rs:3179) -- wholesale, not a delta, and a colour that was never
 *      set is NOT recorded
 *   2  E3's clear is WHOLESALE (modifiers.rs:2148) -- it drops an entry on a card
 *      that no scan could ever re-derive, which is precisely why Rust writes
 *      `clear()` and not a ledger withdraw. Paired with E1 alone, which does NOT
 *      drop it, so the zero in 2 cannot be vacuous.
 *   3  E4 replays the snapshots with Rust's (cid,colour) dedupe (:2163-2167) and
 *      Rust's set-guard `entry.set != 0 && target.set == 0` (:2176-2178)
 *   4  the pair at the live-flow site leaves a performance-time need_heart intact
 *      while still withdrawing one that is not in any snapshot -- the property
 *      E4's own doc comment claims (modifiers.rs:2152-2155)
 *
 * CARD NUMBERS, verified against cards/cards.json before use:
 *   PL!N-sd1-025-SD  live    (type ライブ)     -- a VALID live-slot card
 *   PL!-sd1-010-SD  member  (type メンバー)   -- deck filler
 * The card-number trap is live in this tree (rb_find_card_by_no's substring
 * fallback has already resolved PL!HS-bp1-002-CL to cardid 2793), so both
 * numbers above are exact keys in cards.json, and both are reached only through
 * test_new_id, whose -1 return is checked above. There is deliberately NO
 * runtime re-check of the card numbers here: this suite's assertions are
 * self-consistent in whatever id the fixtures resolve to, so such a check would
 * assert nothing this file depends on, and it would spend two of
 * tests/test_harness_self.c §D's corpus ratchet's tokens -- D1 counts the
 * aliased-lookup call sites in tests/, D2 counts the files containing one --
 * to say so. (Both counters match on the literal call token, so this paragraph
 * deliberately does not spell it; that is what the §D history calls a
 * documentation edit being indistinguishable from a behavioural one.)
 *
 * sizeof(GameState) is pinned at 1 090 264 B by tests/test_harness_self.c §A4
 * (RB_GAMESTATE_BYTES_PINNED, an EXACT equality). Nothing here adds a field, and
 * every TestGame below is `static`.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

/* E3 + E4 are defined non-static in src/core/modifiers.c but are NOT surfaced in
 * include/rabuka.h, which is being edited concurrently by the coordinator. A
 * file-local prototype keeps this suite self-contained and is the same precedent
 * tests/test_live_zone_slot_validation.c:73 sets for rb_execute_performance_phase.
 * rb_execute_performance_phase is src/turn/phase.c and is likewise unsurfaced. */
void rb_evaluate_success_zone_constant_abilities(GameState *g);
void rb_evaluate_success_zone_constant_modifiers(GameState *g);
void rb_restore_performance_need_heart_modifiers(GameState *g);
void rb_execute_performance_phase(GameState *g, int is_first);

static int failures;

#define CHECK(cond, msg) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", (msg)); failures++; FLAT_VERDICT(); } \
    else printf("ok: %s\n", (msg)); \
} while (0)

#define CHECK_EQ(actual, expected, msg) do { \
    int a_ = (actual); int e_ = (expected); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", (msg), a_, e_); \
        failures++; FLAT_VERDICT(); \
    } else printf("ok: %s\n", (msg)); \
} while (0)

static const char *CARD_LIVE   = "PL!N-sd1-025-SD";
static const char *CARD_MEMBER = "PL!-sd1-010-SD";

#define NH 0   /* heart00 -- the colour both fixtures use, so the numbers are comparable */

static int n_entries(const GameState *g) {
    int n = 0;
    for (int cid = 0; cid < RB_MAX_CARD_IDS; cid++)
        for (int c = 0; c < 8; c++)
            if (g->mods.need_heart[cid][c].add != 0 || g->mods.need_heart[cid][c].set != 0) n++;
    return n;
}

/* Rust phases.rs:630-632 pushes one snapshot per performance window. This builds
 * one by hand so E4's reader can be tested without running a whole game; the
 * producer itself is what case 1 exercises. */
static void push_snapshot(GameState *g, int cid, int color, int16_t set, int16_t add) {
    if (g->n_snapshots >= RB_MAX_SNAPSHOTS) return;
    RbLiveSnapshot *s = &g->snapshots[g->n_snapshots++];
    memset(s, 0, sizeof(*s));
    s->n_performance_need_heart_modifiers = 1;
    s->performance_need_heart_modifiers[0].card_id = (int16_t)cid;
    s->performance_need_heart_modifiers[0].color   = (int8_t)color;
    s->performance_need_heart_modifiers[0].entry.set = set;
    s->performance_need_heart_modifiers[0].entry.add = add;
}

/* ── 1 ── E4's PRODUCER: the whole map is captured into the pushed snapshot.
 *      phases.rs:320-333 + :627, live.rs:3179. */
static void producer_records_the_whole_map_into_the_snapshot(void) {
    static TestGame tg;
    test_game_new(&tg);

    int live   = test_new_id(&tg, CARD_LIVE);
    int filler = test_new_id(&tg, CARD_MEMBER);
    if (live < 0 || filler < 0) {
        CHECK(0, "fixtures resolve (a VALID live card and a deck filler)");
        return;
    }

    tg.state.p[0].deck.n = 0;
    tg.state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(&tg, 0, filler);
        test_add_to_deck_pl(&tg, 1, filler);
    }
    RbBag *b = &tg.state.p[0].live;
    b->cards[b->n++] = live;
    tg.state.first_attacker = 0;
    tg.state.second_attacker = 1;

    /* The grant a ライブ開始時 (or any non-constant source) leaves behind, standing
     * in for the whole need_heart map the capture flattens. */
    rb_mods_add_need_heart(&tg.state.mods, live, NH, -2);

    CHECK_EQ(n_entries(&tg.state), 1,
             "CONTROL: exactly one need_heart entry exists before the window opens");

    rb_execute_performance_phase(&tg.state, 1 /* is_first */);

    CHECK(tg.state.n_snapshots >= 1, "the performance window pushed a snapshot");
    if (tg.state.n_snapshots < 1) return;
    const RbLiveSnapshot *s = &tg.state.snapshots[tg.state.n_snapshots - 1];

    CHECK_EQ(s->n_performance_need_heart_modifiers, 1,
             "phases.rs:320-333 -- the capture is WHOLESALE over the map: with one "
             "entry set, exactly one triple is recorded");

    int found = -1;
    for (int i = 0; i < s->n_performance_need_heart_modifiers; i++)
        if (s->performance_need_heart_modifiers[i].card_id == live &&
            s->performance_need_heart_modifiers[i].color == NH) found = i;
    CHECK(found >= 0, "the recorded triple is the (card_id, colour) that was set");
    if (found >= 0)
        CHECK_EQ(s->performance_need_heart_modifiers[found].entry.add, -2,
                 "live.rs:3179 -- the recorded entry is the ENTRY, copied, not a "
                 "pointer and not a delta");

    /* The negative half: a colour nobody set is not in the capture. Without this,
     * "n == 1" above would also be satisfied by a capture that stored every
     * (card_id, colour) pair in the dense C array. */
    int phantom = 0;
    for (int i = 0; i < s->n_performance_need_heart_modifiers; i++)
        if (s->performance_need_heart_modifiers[i].color != NH) phantom++;
    CHECK_EQ(phantom, 0,
             "NEGATIVE: no colour that was never set was recorded, so the capture "
             "is over the map's entries and not over the C array's dense shape");
}

/* ── 2 ── E3's clear is WHOLESALE, and E1 alone does not do it.
 *      modifiers.rs:2148 `self.mods.need_heart_modifiers.clear()`. */
static void e3_clears_wholesale_and_e1_alone_does_not(void) {
    static TestGame tg;
    test_game_new(&tg);

    int onstage = test_new_id(&tg, CARD_MEMBER);
    int offboard = test_new_id(&tg, CARD_MEMBER);
    if (onstage < 0 || offboard < 0) {
        CHECK(0, "fixtures resolve (two distinct member instances)");
        return;
    }

    /* One entry a constant scan could re-derive, one it never could: `offboard`
     * is in no zone at all, so no amount of re-evaluation brings its grant back.
     * That is the case a ledger withdraw cannot express and `clear()` can. */
    rb_mods_add_need_heart(&tg.state.mods, onstage, NH, -2);
    rb_mods_add_need_heart(&tg.state.mods, offboard, NH, -1);
    CHECK_EQ(n_entries(&tg.state), 2, "CONTROL: two need_heart entries exist");

    /* E1 alone -- modifiers.rs:2188-2305 -- touches blade/heart/score only. Its
     * own doc comment says so: "Does NOT touch need_heart_modifiers" (:2185). */
    rb_evaluate_success_zone_constant_modifiers(&tg.state);
    CHECK_EQ(n_entries(&tg.state), 2,
             "PAIRED POSITIVE: E1 alone leaves need_heart untouched (modifiers.rs:2185), "
             "so the zero in the next assertion is E3's doing and not E1's");

    rb_evaluate_success_zone_constant_abilities(&tg.state);
    CHECK_EQ(n_entries(&tg.state), 0,
             "modifiers.rs:2148 -- E3's clear() drops the WHOLE map, including the "
             "entry on a card no scan could ever re-derive");
    CHECK_EQ(tg.state.mods.need_heart[offboard][NH].add, 0,
             "the off-board entry is gone too: the clear is not scoped to the "
             "live/success bags E1 walks");
    CHECK_EQ(tg.state.mods.constant_need_heart[onstage][NH], 0,
             "and it does not bank into the constant_need_heart ledger on the way "
             "out -- Rust's clear() is not a withdraw, it is a clear");

    /* And the table is usable again immediately, which is what the live flow
     * needs: E3 is followed by E4, not by a dead table. */
    rb_mods_add_need_heart(&tg.state.mods, onstage, NH, -2);
    CHECK_EQ(n_entries(&tg.state), 1,
             "CONTROL: need_heart is writable again straight after E3, which is "
             "what makes the E3-then-E4 order load-bearing rather than destructive");
}

/* ── 3 ── E4 replays the snapshots with Rust's dedupe and set-guard.
 *      modifiers.rs:2163-2181. */
static void e4_replays_with_dedupe_and_set_guard(void) {
    static TestGame tg;
    test_game_new(&tg);

    int a = test_new_id(&tg, CARD_MEMBER);
    int b = test_new_id(&tg, CARD_MEMBER);
    if (a < 0 || b < 0) {
        CHECK(0, "fixtures resolve (two distinct member instances)");
        return;
    }

    /* dedupe: modifiers.rs:2161-2167 -- "the same global modifier may appear in
     * multiple players' snapshots, causing double-counting". Two snapshots, same
     * (a, heart00): -2 then -3 must land as -2, never -5. */
    push_snapshot(&tg.state, a, NH, 0, -2);
    push_snapshot(&tg.state, a, NH, 0, -3);
    /* a different (cid, colour) is NOT deduped against those. */
    push_snapshot(&tg.state, b, NH, 0, -1);

    rb_restore_performance_need_heart_modifiers(&tg.state);

    CHECK_EQ(tg.state.mods.need_heart[a][NH].add, -2,
             "modifiers.rs:2163-2167 -- the (card_id, colour) dedupe makes the "
             "second snapshot's -3 a no-op, so the total is -2 and NOT -5");
    CHECK_EQ(tg.state.mods.need_heart[b][NH].add, -1,
             "a distinct (card_id, colour) in a third snapshot is restored on its "
             "own -- the dedupe key is the PAIR, not the card");

    /* set arm, modifiers.rs:2176-2178: `if entry.set != 0 && target.set == 0`. */
    static TestGame tg2;
    test_game_new(&tg2);
    int c = test_new_id(&tg2, CARD_MEMBER);
    if (c < 0) { CHECK(0, "set-arm fixture resolves"); return; }

    push_snapshot(&tg2.state, c, NH, 7, -1);
    rb_restore_performance_need_heart_modifiers(&tg2.state);
    CHECK_EQ(tg2.state.mods.need_heart[c][NH].set, 7,
             "modifiers.rs:2176-2178 -- a snapshot `set` lands when the target has "
             "none (target.set == 0)");

    tg2.state.mods.need_heart[c][NH].set = 3;
    rb_restore_performance_need_heart_modifiers(&tg2.state);
    CHECK_EQ(tg2.state.mods.need_heart[c][NH].set, 3,
             "modifiers.rs:2176-2178 -- and a snapshot NEVER overwrites a `set` "
             "that is already present (the && target.set == 0 half)");
    CHECK_EQ(tg2.state.mods.need_heart[c][NH].add, -2,
             "the additive half is not guarded by `set`: -1 from the snapshot is "
             "added on top of the -1 the first restore already banked");

    /* The empty-list case: E4 was a correctly-shaped no-op while nothing wrote
     * the snapshot field. It must STILL be a no-op now that the producer exists,
     * whenever the producer had nothing to record. */
    static TestGame tg3;
    test_game_new(&tg3);
    int d = test_new_id(&tg3, CARD_MEMBER);
    if (d < 0) { CHECK(0, "empty-list fixture resolves"); return; }
    rb_mods_add_need_heart(&tg3.state.mods, d, NH, -2);
    rb_restore_performance_need_heart_modifiers(&tg3.state);
    CHECK_EQ(tg3.state.mods.need_heart[d][NH].add, -2,
             "CONTROL: with no snapshots at all E4 changes nothing -- restore is "
             "additive onto the live table, never a reset of it");
}

/* ── 4 ── the pair at Rust's live-flow site (live.rs:1326-1327), end to end.
 *      This is the property E4's doc comment claims at modifiers.rs:2152-2155:
 *      "This preserves modifications from live_start triggers and other
 *       non-constant sources". */
static void live_flow_pair_preserves_performance_time_requirements(void) {
    static TestGame tg;
    test_game_new(&tg);

    int live   = test_new_id(&tg, CARD_LIVE);
    int filler = test_new_id(&tg, CARD_MEMBER);
    int late   = test_new_id(&tg, CARD_MEMBER);
    if (live < 0 || filler < 0 || late < 0) {
        CHECK(0, "fixtures resolve (a VALID live card, a filler, a late card)");
        return;
    }

    tg.state.p[0].deck.n = 0;
    tg.state.p[1].deck.n = 0;
    for (int i = 0; i < 30; i++) {
        test_add_to_deck_pl(&tg, 0, filler);
        test_add_to_deck_pl(&tg, 1, filler);
    }
    RbBag *b = &tg.state.p[0].live;
    b->cards[b->n++] = live;
    tg.state.first_attacker = 0;
    tg.state.second_attacker = 1;

    /* The ライブ開始時-style grant. It exists BEFORE the window, so
     * phases.rs:320-333 flattens it into the snapshot this window pushes. */
    rb_mods_add_need_heart(&tg.state.mods, live, NH, -2);
    CHECK_EQ(n_entries(&tg.state), 1, "CONTROL: one need_heart entry before the window");

    rb_execute_performance_phase(&tg.state, 1 /* is_first */);

    CHECK(tg.state.n_snapshots >= 1, "the window pushed a snapshot to restore from");
    if (tg.state.n_snapshots < 1) return;

    /* A grant that arrives AFTER the snapshot was taken, i.e. between the
     * performance window and victory determination. This is the shape of the
     * stale constant grant E1 leaves behind when a 「…成功ライブカード置き場に
     * あるかぎり」 activator has since left the success zone: it is in
     * mods.need_heart_modifiers, but nothing recorded it, so E3 withdraws it and
     * E4 has nothing to put back. It has to be added HERE and not above, because
     * Rust's capture flattens the WHOLE map (phases.rs:320-333) and therefore
     * restores every entry that existed at window time -- including an
     * unre-derivable one. Rust has no source attribution in the map either. */
    rb_mods_add_need_heart(&tg.state.mods, late, NH, -1);
    CHECK_EQ(n_entries(&tg.state), 2,
             "CONTROL: two entries going into the determination");

    /* live.rs:1326-1327 -- rb_execute_live_victory_determination now opens with
     * E3 then E4. Rust runs them unconditionally at the head, before
     * rebuild_stage_hearts_with_yell reads anything. */
    rb_execute_live_victory_determination(&tg.state);

    CHECK_EQ(tg.state.mods.need_heart[live][NH].add, -2,
             "modifiers.rs:2152-2155 -- E3 cleared the table and E4 put the "
             "performance-time requirement BACK: the live_start-sourced -2 survives "
             "the clear, which is the whole reason E4 exists");
    CHECK_EQ(tg.state.mods.need_heart[late][NH].add, 0,
             "a grant made after the window was NOT recorded, so E3's wholesale "
             "clear withdrew it and E4 could not put it back -- E3 alone would "
             "have kept it, and E4 alone would never have touched it");
}

int main(void) {
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    producer_records_the_whole_map_into_the_snapshot();
    e3_clears_wholesale_and_e1_alone_does_not();
    e4_replays_with_dedupe_and_set_guard();
    live_flow_pair_preserves_performance_time_requirements();

    if (failures) {
        fprintf(stderr, "FAILED: parity_need_heart_live_flow  [%d failure(s)]\n", failures);
        return 1;
    }
    printf("PASSED: parity_need_heart_live_flow\n");
    return 0;
}
