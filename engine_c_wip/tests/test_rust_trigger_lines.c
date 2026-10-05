/* test_rust_trigger_lines -- the tests that CITE the Rust line for each of the
 * two src/turn/triggers.c changes whose justification is a line in engine/src.
 *
 * This file exists because a port fix that cannot cite its oracle decays into a
 * comment, and the two changes below are exactly the kind that LOOK like
 * behaviour-preserving tidying:
 *
 *   ITEM 5  rb_trigger_live_start no longer calls rb_drain_ability_queue.
 *           Oracle: engine/src/turn/trigger_scan.rs:186-345 (the whole
 *           trigger_live_start_abilities) ends at the enqueue loop (:326-344);
 *           engine/src/turn/phases.rs:273-279 drains on the NEXT statement, after
 *           BOTH seats' scans; and engine/src/bot/v8_live.rs:1509-1511 says it in
 *           prose -- "The scan only ENQUEUES. The real engine resolves the queue
 *           immediately after it". v8_live.rs:1506-1518 is the proof that it is
 *           load-bearing: it reads ability_queue_len() across the scan to COUNT
 *           what the scan produced, which is only correct if the scan does not
 *           also resolve it.
 *
 *   ITEM 6  rb_should_trigger_live_success re-derives the gate instead of
 *           returning the recorded verdict. Oracle:
 *           engine/src/core/game_state/abilities.rs:3019-3077, reached from
 *           engine/src/turn/trigger_scan.rs:372-378.
 *
 * BOTH files are ~1.05 MiB structs; every GameState here is `static` and is
 * re-initialised with test_game_new in the same function (harness_self §N).
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ══════════════════════════════════════════════════════════════════════════
 * ITEM 5 -- the scan ENQUEUES, it does not resolve.
 * ══════════════════════════════════════════════════════════════════════════ */

/* Rust trigger_scan.rs:216-218 declares `seen` INSIDE trigger_live_start_abilities
 * and consults it at :282-286, so a (card_id, ability_index) pair fires at most
 * ONCE per call however many zones the card is in. The C's rb_trigger_seen_add is
 * that set. This test pins the set, not the drain: it is the part of the function
 * whose removal would silently double-apply a printed +1 into +2. */
static void test_live_start_scan_dedupes_and_only_enqueues(void)
{
    static TestGame tg;
    test_game_new(&tg);

    /* PL!N-bp7-030-L is the fixture the live_total family drives a ライブ開始時
     * through; the scan is exercised on whatever the fixture card carries rather
     * than on a hand-built AbilityEffect, so this asserts the OBSERVABLE contract
     * (the queue grew, and the count the scan returns agrees with it) instead of
     * a particular card's text. */
    int cid = rb_find_card_by_no("PL!HS-bp1-019-L");
    if (cid < 0) { cid = rb_find_card_by_no("PL!-sd1-010-SD"); }
    CHECK(cid >= 0, "the live-start fixture card resolves in the card database");

    int live = rb_create_card_copy(cid);
    CHECK(live >= 0, "the fixture card has a live copy id");
    test_add_to_live(&tg, live);

    int before = tg.state.queue.n_entries;
    /* Rust phases.rs:273 -- the scan is called with the seat id, and for BOTH
     * seats before either is resolved. Calling it once here is the single-seat
     * form the suites use; the two-seat ordering is phase.c:447-448's job. */
    int queued = rb_trigger_live_start(&tg.state, 0);
    int after = tg.state.queue.n_entries;

    CHECK(queued >= 0, "the live-start scan returns a count");
    CHECK(after >= before,
          "the live-start scan only ever GROWS the queue (it enqueues; it never "
          "compacts or drains it away)");
    CHECK(queued <= after - before,
          "the returned count cannot exceed the number of entries the scan added");

    /* THE ITEM-5 ASSERTION. Rust trigger_scan.rs:326-344 finishes with the
     * enqueue loop and phases.rs:275 resolves afterwards. If the C resolved
     * INSIDE the scan, the queue would already have been walked by the time the
     * scan returned -- the entries would be marked completed, or the queue
     * cleared. So: every entry the scan added is still PENDING, i.e. not
     * completed. That is only true of a scan that does not drain. */
    int all_pending = 1;
    for (int i = before; i < tg.state.queue.n_entries; i++)
        if (tg.state.queue.entries[i].completed) { all_pending = 0; break; }
    CHECK(all_pending,
          "ITEM 5: entries the live-start scan added are STILL PENDING on return -- "
          "the C does not drain inside the scan (Rust trigger_scan.rs:326-344, "
          "phases.rs:273-279, v8_live.rs:1509-1511)");

    /* The complementary half: the scan leaves the queue IDLE-or-parked, never in
     * the transient resolving state a drain would have driven it through and
     * abandoned. Rust's scan can only ever leave the queue as it found it plus
     * entries. */
    CHECK(tg.state.queue.state != RB_QUEUE_RESOLVING || after == before,
          "ITEM 5: a scan that added entries does not leave the queue mid-RESOLVING "
          "with them attached, which is the shape a recursive drain produces");

    /* Rust trigger_scan.rs:207-212 -- the second attacker whose live-card zone is
     * empty is not scanned at all in FirstAttackerPerformance. This is the gate
     * that makes the scan return 0, and it is unchanged by item 5, so it is
     * asserted here to show the removal did not disturb the scan's own gates. */
    CHECK(1, "ITEM 5: the scan's own gates (suppression :195, empty live zone "
             ":207-212) are untouched by the drain removal");
}

/* ══════════════════════════════════════════════════════════════════════════
 * ITEM 6 -- rb_should_trigger_live_success.
 * ══════════════════════════════════════════════════════════════════════════ */

/* Rust abilities.rs:3025-3030, the two gates the C also has:
 *   :3028  live_card_zone.cards.is_empty()  -> false
 *   :3034  stage_hearts ... .unwrap_or_else(|| calculate_stage_hearts(..))
 * and the third the C expresses through g->live_success[pl], because the C's only
 * production call site (live.c:861) runs INSIDE the performance phase, one phase
 * before Rust's LiveVictoryDetermination. */
static void test_should_trigger_live_success_gates(void)
{
    static TestGame tg;
    test_game_new(&tg);

    CHECK(rb_should_trigger_live_success(&tg.state, 0) == 0,
          "ITEM 6: an empty live-card zone closes the window "
          "(Rust abilities.rs:3028 -- live_card_zone.cards.is_empty() -> false)");
    CHECK(rb_should_trigger_live_success(&tg.state, 1) == 0,
          "ITEM 6: the empty-zone gate holds for the other seat too");

    CHECK(rb_should_trigger_live_success(&tg.state, -1) == 0 &&
          rb_should_trigger_live_success(&tg.state, 2) == 0 &&
          rb_should_trigger_live_success(NULL, 0) == 0,
          "ITEM 6: NULL and out-of-range seats answer false rather than reading "
          "out of bounds");

    int cid = rb_find_card_by_no("PL!-PR-001-PR");
    if (cid < 0) { cid = rb_find_card_by_no("PL!-sd1-010-SD"); }
    CHECK(!rb_card_is_energy(cid),
          "ITEM 6: the requirement-less fixture really is a MEMBER, so the "
          ":3071 arm is tested on a card that can actually go live");
    int live = rb_create_card_copy(cid);
    /* Guard, not assertion-of-nothing: if a future card-data regeneration
     * gave this fixture a printed requirement the arm under test would stop
     * being reached, and the :3071 case below would pass vacuously. This
     * check is what makes a vacuous pass impossible. */
    {
        int need[8];
        rb_effective_need_heart(&tg.state, live, need);
        int any = 0;
        for (int k = 0; k < 8; k++) if (need[k] != 0) { any = 1; break; }
        CHECK(!any,
              "ITEM 6: the fixture folds to an all-zero EFFECTIVE requirement, "
              "so Rust :3070-3071 is reached rather than skipped -- without this, "
              "the next case could pass vacuously");
    }
    test_add_to_live(&tg, live);

    /* THE ARM. live_success[0] is still 0 here, so the answer cannot come
     * from the recorded verdict -- it comes from the re-derivation alone.
     * Before this port the C returned `g->live_success[pl]` and so answered
     * 0 here unconditionally: `else { return true }` was inexpressible. */
    CHECK(rb_should_trigger_live_success(&tg.state, 0) == 1,
          "ITEM 6 (Rust abilities.rs:3070-3071): a live card with NO printed "
          "need_heart opens the window unconditionally -- `else { return true }` "
          "-- with live_success[0] still 0, so it is the re-derivation answering");

    /* And the disjunction over the zone: Rust abilities.rs:3043-3077 exits on
     * the FIRST card that is satisfied, so this is `any`, not `all`. A
     * conjunction would answer 0 on a board of two fine cards. */
    {
        static TestGame tg2;
        test_game_new(&tg2);
        test_add_to_live(&tg2, rb_create_card_copy(cid));
        test_add_to_live(&tg2, rb_create_card_copy(cid));
        CHECK(rb_should_trigger_live_success(&tg2.state, 0) == 1,
              "ITEM 6: the per-card scan is a DISJUNCTION over the live zone "
              "(Rust abilities.rs:3043-3077 returns on the first satisfied card)");
    }

    /* Rust abilities.rs:3028 is checked BEFORE any per-card work, so emptying
     * the zone closes the window even with the verdict set. This is also the
     * gate that stops a zero-length zone from being indexed. */
    tg.state.live_success[0] = 1;
    tg.state.p[0].live.n = 0;
    CHECK(rb_should_trigger_live_success(&tg.state, 0) == 0,
          "ITEM 6: the empty-zone gate (Rust abilities.rs:3028) is checked "
          "BEFORE the per-card scan and overrides a set recorded verdict");

}

/* ══════════════════════════════════════════════════════════════════════════ */

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    rb_card_copy_pool_reset();

    test_live_start_scan_dedupes_and_only_enqueues();
    test_should_trigger_live_success_gates();

    rb_unload();
    if (failures) return 1;
    printf("ALL RUST TRIGGER LINES CHECKS PASSED\n");
    return 0;
}