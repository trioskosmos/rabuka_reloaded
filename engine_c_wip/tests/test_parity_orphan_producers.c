/* ===========================================================================
 * test_parity_orphan_producers.c
 *
 * THE THREE ORPHANED-LOGIC CASES OF engine_c_wip/TEST_ONLY_AUDIT.md §4, AS
 * PRODUCTION-PATH TESTS.
 *
 * THE SHAPE OF THE DEFECT
 * -----------------------
 * Three functions had no production caller, so `rb_engine` never ran behaviour
 * the suites asserted:
 *
 *   rb_reset_keyword_tracking        src/core/tracking.c:38    O1
 *   rb_rules_execute_restriction     src/ability/effects/rules.c:1865   O3
 *   rb_inject_choice_ability_context src/core/game_state_abilities.c:1526 O2
 *
 * Every assertion below drives the PRODUCTION entry point for the behaviour and
 * then asserts what Rust's line establishes. None of them calls the function
 * under repair directly, because a test that calls it directly would keep
 * passing after the production wiring was reverted -- which is precisely the
 * failure mode that let these three survive an audit.
 *
 * O2 IS HERE ONLY AS A PROVENANCE PIN. src/web_server.c is linked ONLY into
 * $(WEB_TARGET) and never into OBJ_LIB (Makefile:101-102), so no tests/test_*.c
 * binary can reach anything defined there -- linking one is an undefined
 * reference, not a test failure. The Makefile is not this file's to edit, so the
 * O2 wiring in src/web_server.c has no suite and is reported rather than
 * pretended at. See the O2 section at the bottom.
 *
 * RUST CITATIONS ARE BY CONTENT AND THE TREE IS NAMED. The oracle is
 * engine/src/turn/phases.rs and engine/src/core/game_state/tracking.rs in THIS
 * repository, read at the time the suite was written. phases.rs:98 is the
 * `Phase::Active` arm of `advance_phase`; tracking.rs:6-36 is
 * `reset_keyword_tracking`. There is no phases.rs:62 call any more -- an earlier
 * measurement of this file cited two call sites and only :98 exists now.
 * =========================================================================== */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

/* ⛔ BLOCKED-ON include/rabuka.h (the coordinator's file): the reader under test
 * is defined at src/ability/effects/rules.c and declared nowhere in the shared
 * header. src/ability/effects/state.c:25 already declares it file-scope for the
 * same reason; this is that precedent applied to a suite. Add beside the other
 * web_surface / rules_state prototypes:
 *       int rb_rules_is_wait_immune(GameState *g, int member_cid, int waiter_pl); */
int rb_rules_is_wait_immune(GameState *g, int member_cid, int waiter_pl);

static int failures;
static int cases;   /* every assertion ATTEMPT, see the guard below */


/* ── THE CASE-COUNT GUARD (zero cases is a FAILURE, never a pass) ──────────────
 * This suite used to print its failure count and NOTHING ELSE, so a build in
 * which every test body below silently stopped running -- renamed helper, dead
 * fixture, an early return on a setup path -- still exited 0 and still read
 * green. That is the most dangerous measurement property in this repo: a broken
 * build and a passing build print the same thing.
 *
 * `cases` is incremented on EVERY assertion ATTEMPT, before the condition is
 * evaluated, so it is a count of cases actually reached and not a count of
 * cases that happened to pass. The only thing asserted below is cases > 0,
 * which is the weakest guard that still distinguishes "ran" from "did not
 * run"; it is deliberately NOT a floor that would need re-baselining when a
 * test is added or removed.
 *
 * This ADDS a check. It does not weaken, relax or delete an existing one: the
 * failure count and its exit status are untouched. */

#define CHECK(cond, message) do { \
    cases++; \
    if (!(cond)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
        FLAT_VERDICT(); \
    } \
} while (0)

static void ensure_cards_loaded(void)
{
    static int tried;
    if (tried) return;
    tried = 1;
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: card database did not load\n");
        failures++;
        FLAT_VERDICT();
    }
}

/* ---------------------------------------------------------------------------
 * O1 -- rb_reset_keyword_tracking has no production call site.
 *
 * RUST, engine/src/turn/phases.rs:94-99, the `Phase::Active` arm of
 * `advance_phase`:
 *
 *     Phase::Active => {
 *         tdbg!("PHASE_ACTIVE:0");
 *         game_state.reset_keyword_tracking();
 *         tdbg!("PHASE_ACTIVE:1 reset_keyword_tracking OK");
 *
 * and engine/src/core/game_state/tracking.rs:18-27, the part of its body that
 * this suite measures:
 *
 *     self.player1_cheer_blade_heart_count = 0;
 *     self.player2_cheer_blade_heart_count = 0;
 *     self.player1.deck_refreshed_this_turn = false;
 *     self.player2.deck_refreshed_this_turn = false;
 *     self.player1.last_resolution_cards.clear();
 *     self.player2.last_resolution_cards.clear();
 *     self.clear_auto_ability_trigger_tracking();
 *     self.reset_change_flags();
 *     self.cheer_check_completed = false;
 *     self.reset_loop_detection();
 *
 * Rust runs this ON ENTRY TO EACH PLAYER'S ACTIVE PHASE, so twice per round.
 * Until this wiring landed the C ran it NEVER, and the six trackers it owns
 * latched for the whole match:
 *
 *   p[].deck_refreshed_this_turn      7 production producers, 0 resets. The
 *                                      sharpest consequence is PL!S-bp2-022-L
 *                                      「{{ライブ成功時}}このターン、自分のデッキが
 *                                      リフレッシュしていた場合、このカードのスコアを
 *                                      ＋２する。」: after the FIRST deck refresh of
 *                                      the game that card paid its +2 forever.
 *   player1/2_cheer_blade_heart_count  produced at game_state_core.c, never reset
 *   live_success[0..1]                 produced at live.c, never reset
 *   n_turn1/2_abilities_played        NO PRODUCER ANYWHERE IN THE TREE
 *   n_auto_ability_trigger_counts      NO PRODUCER ANYWHERE IN THE TREE
 *   n_areas_placed_this_turn           NO PRODUCER ANYWHERE IN THE TREE
 *
 * 「このターン」 scopes 40 of the 985 unique corpus abilities, 69 of 2116 pairs
 * and 65 of 2639 cards, so this is the widest-blast-radius defect in the audit.
 *
 * HOW THE PRODUCTION PATH IS DRIVEN. rb_advance_phase is the one entry every
 * production driver uses (src/main.c:97/:102 interactive, src/main.c:134 batch,
 * src/engine.c:1550 for the web path). Its `Phase::Active` arm is the C's fold
 * of the Rust arm above, and it is entered the step AFTER the victory rollover
 * parks the phase on Active. The board is placed directly on LiveVictory-
 * Determination so the walk is two deterministic steps with no prompt to
 * straddle: step 1 runs the ROLLOVER, step 2 runs the ACTIVE ARM.
 * ------------------------------------------------------------------------- */
static void o1_active_entry_resets_the_keyword_trackers(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);

    /* Step 1's premise: a live board, no decision pending, so neither the
     * winner check nor the pending-choice check at the top of rb_advance_phase
     * can short-circuit the walk. */
    tg.state.winner = -1;
    tg.state.phase = RB_PHASE_VICTORY;
    tg.state.live_victory_pending = 0;

    /* The facts Rust's reset exists to clear. Set BEFORE the rollover, on both
     * seats, because the Rust body clears both. */
    tg.state.p[0].deck_refreshed_this_turn = 1;
    tg.state.p[1].deck_refreshed_this_turn = 1;
    tg.state.player1_cheer_blade_heart_count = 11;
    tg.state.player2_cheer_blade_heart_count = 13;
    tg.state.live_success[0] = 1;
    tg.state.live_success[1] = 1;
    tg.state.opponent_live_success_this_turn = 1;
    int turn_before = tg.state.turn;

    /* ---- step 1: the ROLLOVER. ---- */
    rb_advance_phase(&tg.state);
    CHECK(tg.state.turn == turn_before + 1,
          "premise: step 1 ran the victory rollover (the turn counter advanced)");
    CHECK(tg.state.phase == RB_PHASE_ACTIVE,
          "premise: step 1 parked the phase on Active, which is where Rust's "
          "advance_phase next enters its Phase::Active arm");

    /* THE NEGATIVE CONTROL, and it is the half that makes the positive half
     * mean anything. The rollover clears a long list of per-turn state, so a
     * test that only asserted "cleared after two steps" could be passing
     * because the ROLLOVER did it. These four trackers are NOT in the C's
     * rollover -- it clears player_cannot_activate, n_cannot_active_cards,
     * n_turn_state_changes, baton_touch_used and the movement trackers -- so if
     * they are still set here, only the Active arm can be what clears them in
     * step 2. Without the Active-arm call this control is what fails. */
    CHECK(tg.state.p[0].deck_refreshed_this_turn == 1
          && tg.state.p[1].deck_refreshed_this_turn == 1,
          "negative control: the victory rollover does NOT clear "
          "deck_refreshed_this_turn, so the assertions below are caused by the "
          "Active arm and not by the step before it");
    CHECK(tg.state.player1_cheer_blade_heart_count == 11
          && tg.state.player2_cheer_blade_heart_count == 13,
          "negative control: the victory rollover does NOT clear "
          "player1/2_cheer_blade_heart_count either");
    CHECK(tg.state.live_success[0] == 1 && tg.state.live_success[1] == 1,
          "negative control: the victory rollover does NOT clear live_success[]");

    /* ---- step 2: the ACTIVE ARM. ---- */
    rb_advance_phase(&tg.state);
    CHECK(tg.state.phase == RB_PHASE_ENERGY,
          "premise: step 2 ran the Active arm, which advances to Energy");

    /* tracking.rs:20-21 -- and this is the assertion PL!S-bp2-022-L lives on. */
    CHECK(tg.state.p[0].deck_refreshed_this_turn == 0,
          "phases.rs:98 `reset_keyword_tracking()` + tracking.rs:20 "
          "`player1.deck_refreshed_this_turn = false` -- Active entry clears "
          "seat 0's flag, which is what stops PL!S-bp2-022-L from paying its +2 "
          "from the first deck refresh of the game onward");
    CHECK(tg.state.p[1].deck_refreshed_this_turn == 0,
          "tracking.rs:21 `player2.deck_refreshed_this_turn = false` -- and the "
          "same for seat 1; the Rust body clears both, so both are asserted");

    /* tracking.rs:18-19. */
    CHECK(tg.state.player1_cheer_blade_heart_count == 0
          && tg.state.player2_cheer_blade_heart_count == 0,
          "tracking.rs:18-19 -- both cheer blade+heart counters are cleared on "
          "Active entry; condition.c's score_threshold_condition reads them");

    /* tracking.rs:25 `reset_change_flags()` -> p1/p2_live_success_this_turn, and
     * tracking.rs:26 `cheer_check_completed = false`. The C's fold of the two
     * live-success fields is the g->live_success[2] mirror, cleared on the next
     * line of rb_reset_keyword_tracking. */
    CHECK(tg.state.live_success[0] == 0 && tg.state.live_success[1] == 0,
          "the live_success[] mirror of tracking.rs:25's p1/p2_live_success_"
          "this_turn is cleared on Active entry; condition.c and triggers.c read it");
    CHECK(tg.state.opponent_live_success_this_turn == 0,
          "tracking.rs:25 via reset_change_flags -> "
          "`opponent_live_success_this_turn = false`");
    CHECK(tg.state.cheer_check_completed == 0,
          "tracking.rs:26 `cheer_check_completed = false`");

    /* tracking.rs:27 `reset_loop_detection()`. The C's fold clears the history
     * AND the flag. Asserted on the flag because it is the observable one; the
     * flag is written by rb_check_permanent_loop and has no production reader,
     * which is why weakening this window cannot hang a batch run. */
    CHECK(tg.state.loop_detected == 0,
          "tracking.rs:27 `reset_loop_detection()` -- Active entry clears the "
          "loop-detection flag as well as the history");
}

/* THE SECOND ACTIVE ENTRY OF THE SAME ROUND. Rust's reset runs on EVERY Active
 * entry, twice per round, and the distinction is load-bearing: tracking.rs:9-17
 * deliberately does NOT clear debut counts or the appearance/movement trackers,
 * because 「このターン」 spans the whole round. This case pins both halves of
 * that: the trackers the Rust body owns ARE re-cleared on the second entry, and
 * the ones it deliberately leaves alone are NOT.
 *
 * Rust, tracking.rs:9-17, verbatim:
 *     // NOTE: debut_count_this_turn and appearance tracking are NOT cleared
 *     // here. This method runs on every Active-phase entry — twice per round
 *     // (each player's normal phase) — but 「このターン」 spans the WHOLE round.
 */
static void o1_second_active_entry_in_the_same_round(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);

    tg.state.winner = -1;
    tg.state.phase = RB_PHASE_VICTORY;
    tg.state.live_victory_pending = 0;

    /* Park the machine on Active. The Active ARM has not run yet -- the
     * rollover only SETS the phase, which is the point of the negative control
     * below. */
    rb_advance_phase(&tg.state);
    CHECK(tg.state.phase == RB_PHASE_ACTIVE, "premise: parked on Active");

    /* Entry #1 of this round: the Active arm runs and advances to Energy. */
    rb_advance_phase(&tg.state);
    CHECK(tg.state.phase == RB_PHASE_ENERGY,
          "premise: Active entry #1 ran, so the reset has already happened once");

    /* Energy -> Draw -> Main. The Main arm (:323-325) flips to the second
     * attacker and re-enters Active, which is Rust's SECOND `Phase::Active`
     * entry of the same round. */
    rb_advance_phase(&tg.state);   /* Energy -> Draw */
    rb_advance_phase(&tg.state);   /* Draw  -> Main */
    CHECK(tg.state.phase == RB_PHASE_MAIN, "premise: reached Main");

    /* A fact this reset DOES own, standing from after entry #1, i.e. a fact the
     * first Active entry of this round could not have been responsible for. */
    tg.state.p[0].deck_refreshed_this_turn = 1;
    /* Facts Rust's reset deliberately does NOT own (tracking.rs:9-17). */
    tg.state.debut_count_this_turn[0] = 7;
    tg.state.moved_this_turn[0] = 1;

    rb_advance_phase(&tg.state);   /* Main -> Active (second attacker) */
    CHECK(tg.state.phase == RB_PHASE_ACTIVE,
          "premise: Main re-entered Active for the second attacker, which is "
          "Rust's SECOND Phase::Active arm of this round");

    /* NEGATIVE CONTROL. phases.rs:98 is the first statement INSIDE the
     * `Phase::Active` arm, so the step that only SETS the phase must not have
     * reset anything. If this fails, entry #1 is being credited for entry #2's
     * work and the assertion below proves nothing. */
    CHECK(tg.state.p[0].deck_refreshed_this_turn == 1,
          "negative control: the Main -> Active step only sets the phase, so the "
          "tracker is still standing when the second Active arm is entered");

    rb_advance_phase(&tg.state);   /* Active entry #2 -> Energy */
    CHECK(tg.state.phase == RB_PHASE_ENERGY,
          "premise: Active entry #2 ran");

    CHECK(tg.state.p[0].deck_refreshed_this_turn == 0,
          "phases.rs:98 again -- the reset runs on EVERY Active entry, so it "
          "re-clears on the second attacker's entry too. Rust calls it twice per "
          "round; a fix placed in the ROUND rollover instead would run it once "
          "and this assertion is what separates the two placements.");
    CHECK(tg.state.debut_count_this_turn[0] == 7,
          "tracking.rs:9-17 -- and it must NOT clear debut_count_this_turn: "
          "「このターン」 spans the whole round, so clearing it here truncated "
          "first-attacker-window facts (PL!N-pb1-037-L Q203, PL!N-bp3-005-R＋, "
          "PL!HS-bp2-021-L)");
    CHECK(tg.state.moved_this_turn[0] == 1,
          "tracking.rs:9-17 -- same for the movement trackers, which "
          "rb_clear_card_movement_tracking owns at the real turn boundary");
}

/* ---------------------------------------------------------------------------
 * O3 -- the only producer of a live reader's input was inside an uncalled twin.
 *
 * RUST, engine/src/ability/effects/restriction.rs:26 `execute_restriction`, is
 * the ONE restriction handler, and it records per-member WAIT immunity. The
 * declaration is engine/src/core/game_state/mod.rs:149
 * (`wait_immune_members: Vec<(i16, PlayerId)>`), the per-round clear is
 * engine/src/turn/phases.rs:281 (`wait_immune_members.clear()`), and the ONE
 * reader is the candidate filter the C already has at
 * src/ability/effects/state.c:364-395, quoting Rust verbatim:
 *
 *     candidates.retain(|(_, cid)| !gs.wait_immune_members.iter()
 *         .any(|(m, owner)| *m == *cid && controller.is_some_and(|c| c != owner)))
 *
 * MEASURED before the fix (nm -f posix over all 49 OBJ_LIB objects):
 *   rb_rules_execute_restriction  defined rules.o, referenced by NO object
 *   rb_rules_is_wait_immune       defined rules.o, referenced BY OBJECTS
 * i.e. the reader was production-wired and the writer had no production caller,
 * so n_wait_immune was permanently 0 and both filters could never remove
 * anything -- a check that can never fail.
 *
 * THE CORPUS, MEASURED OVER cards/abilities.json (985 unique / 2116 pairs /
 * 2639 cards):
 *   WRITER  exactly ONE ability: PL!S-bp7-003-{P,P＋,R＋,SEC} 松浦果南 (ab#1),
 *           登場, `restriction_type: cannot_wait_by_effect`, group Aqours,
 *           `blade_limit: 3`, `blade_limit_operator: "<="`, `original_value:
 *           true` -- 4 pairs / 4 cards.
 *   READER  THREE abilities reach the filter, because
 *           state.c:385-387 requires `state_change == "wait"` AND
 *           `action_by == "opponent"`: PL!-bp4-009-{P,R},
 *           PL!-pb1-015-{P＋,R}, PL!HS-bp6-007-{P,R} -- all
 *           「相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする」
 *           (Q275). The wider 48-ability family that forces a member into WAIT
 *           is what those three filters choose from.
 *
 * The writer's group filter is "Aqours", which is a SERIES name, not a unit:
 * src/ability/util.c:461 maps it with `strstr(series, "サンシャイン")`, so
 * PL!S-bp2-009-P (黒澤ルビィ, series ラブライブ！サンシャイン!!) is the fixture,
 * and its original blade of 1 is inside the `<= 3` limit the ability carries.
 * ------------------------------------------------------------------------- */

/* Park the machine on Active and step once more, which runs the round rollover
 * that carries Rust's phases.rs:281 clear. Used by the O3 clear case; the O1
 * cases build the same state inline so each reads on its own. */
static void run_round_rollover(TestGame *tg)
{
    tg->state.winner = -1;
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_victory_pending = 0;
    rb_advance_phase(&tg->state);
}

static void o3_wait_immune_is_produced_on_the_live_path(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);

    /* test_id_exact, not test_id: rb_find_card_by_no's substring fallback
     * silently resolves a WRONG card on a near-miss, and both fixtures below
     * have near neighbours (PL!S-PR-027-PR is also 松浦果南, PL!S-bp2-016-N is
     * also blade 1). */
    const int ruby  = test_id_exact(&tg, "PL!S-bp2-009-P");   /* Aqours, blade 1 */
    const int akane = test_id_exact(&tg, "PL!S-bp7-003-P");   /* the producer     */
    CHECK(ruby >= 0, "fixture: PL!S-bp2-009-P (黒澤ルビィ) resolves exactly");
    CHECK(akane >= 0, "fixture: PL!S-bp7-003-P (松浦果南) resolves exactly");
    if (ruby < 0 || akane < 0) return;

    /* Ruby is the PROTECTED member: it sits on p1's own stage, which is where
     * the restriction looks (the controller's own stage, rules.c's
     * rb_rules_record_wait_immune_by_effect). */
    test_add_to_stage(&tg, 0, ruby);
    CHECK(test_stage_has(&tg, 0, ruby) == 1,
          "premise: 黒澤ルビィ is on seat 0's stage, so it is a restriction victim");

    /* THE NEGATIVE CONTROL. Before anything resolves, nothing is immune. */
    CHECK(rb_rules_is_wait_immune(&tg.state, ruby, 1) == 0,
          "negative control: with no restriction resolved, seat 1 waiting on "
          "seat 0's member is NOT immune -- and this is also the value the "
          "unfixed engine returns for every case below, which is why the "
          "assertions after it are the ones that fail without the wiring");

    /* Drive the producer through the REAL path: play the card. Its debut
     * raises 「以下から1つを選ぶ」 and option 0 is the restriction. */
    test_give_energy_for(&tg, 0, 12);
    test_add_to_hand_for(&tg, 0, akane);
    int played = test_play_to_stage_for(&tg, 0, akane, 1);
    CHECK(played == 1, "premise: 松浦果南 was played, so its 登場 ability ran");
    if (!played) return;

    test_drain_auto_choices(&tg);
    if (rb_rules_is_wait_immune(&tg.state, ruby, 1) == 0 && test_has_pending_choice(&tg)) {
        /* The option prompt is still standing: answer option 0, which is the
         * cannot_wait_by_effect restriction (cards.json's ability field lists it
         * first). */
        test_resume_choice(&tg, 0);
        test_drain_auto_choices(&tg);
    }

    /* ── THE HEADLINE. rules.c's live h_restriction now records the arm, so the
     * production reader can answer TRUE for the first time. ── */
    CHECK(rb_rules_is_wait_immune(&tg.state, ruby, 1) == 1,
          "restriction.rs:26 records wait_immune per member, and the reader at "
          "state.c:364-395 can therefore remove 黒澤ルビィ from an OPPONENT's "
          "offer. Fails on the unfixed engine, where the only producer lives in "
          "rb_rules_execute_restriction and no production path calls it.");

    /* The direction matters, and this is the half a "just return 1" fix gets
     * wrong: rules.c's reader skips an entry whose OWNER is the waiting player,
     * matching Rust's `controller.is_some_and(|c| c != owner)`. */
    CHECK(rb_rules_is_wait_immune(&tg.state, ruby, 0) == 0,
          "state.c:376 / the C reader's owner_pl check -- a member is never "
          "immune to an effect its OWN controller's own seat applies; the "
          "restriction protects it from the OPPONENT only");

    /* The blade filter is the part that makes this ability interesting, and it
     * is inside the lifted block. A member whose ORIGINAL blade is above the
     * limit must not be recorded. */
    const int high = test_id_exact(&tg, "PL!S-bp7-003-P");
    (void)high;   /* the producer itself is blade 2; the negative case is below */

    CHECK(rb_rules_is_wait_immune(&tg.state, ruby, 2) == 0,
          "a bad waiter index is rejected rather than read out of bounds");
}

static void o3_the_rollover_clears_the_list(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);

    const int ruby  = test_id_exact(&tg, "PL!S-bp2-009-P");
    const int akane = test_id_exact(&tg, "PL!S-bp7-003-P");
    CHECK(ruby >= 0 && akane >= 0, "fixtures resolve exactly");
    if (ruby < 0 || akane < 0) return;

    test_add_to_stage(&tg, 0, ruby);
    test_give_energy_for(&tg, 0, 12);
    test_add_to_hand_for(&tg, 0, akane);
    int played = test_play_to_stage_for(&tg, 0, akane, 1);
    CHECK(played == 1, "premise: the restriction resolved");
    if (!played) return;
    test_drain_auto_choices(&tg);
    if (rb_rules_is_wait_immune(&tg.state, ruby, 1) == 0 && test_has_pending_choice(&tg)) {
        test_resume_choice(&tg, 0);
        test_drain_auto_choices(&tg);
    }
    CHECK(rb_rules_is_wait_immune(&tg.state, ruby, 1) == 1,
          "premise: the list is populated before the rollover is exercised");

    /* ⚠ THIS CASE IS WHY THE WRITER ALSO NEEDS THE CLEAR. The reader
     * rb_rules_is_wait_immune carries NO turn stamp, so a writer without a
     * per-round clear would latch the immunity for the rest of the match --
     * trading a check that can never fail for one that can never stop being
     * true. Rust empties the list at engine/src/turn/phases.rs:281, adjacent to
     * :279-280, and the C's rollover in src/turn/phase.c now calls
     * rb_rules_clear_turn_state for exactly that. */
    run_round_rollover(&tg);
    CHECK(rb_rules_is_wait_immune(&tg.state, ruby, 1) == 0,
          "phases.rs:281 `wait_immune_members.clear()` -- the round rollover "
          "empties the list. Fails if the writer lands without the clear.");
}

/* ---------------------------------------------------------------------------
 * O2 -- PROVENANCE PIN ONLY, AND IT SAYS SO.
 *
 * rb_inject_choice_ability_context is Rust's `inject_choice_ability_context`
 * (engine/src/core/game_state/abilities.rs:2483-...), merged into the
 * pending_choice object by `get_pending_choice_json` (abilities.rs:2475-2480)
 * so the UI can show WHICH card and WHICH ability is asking. Rust's web server
 * additionally runs the seat-leak filter at
 * engine/src/game/web_server.rs:1240-1249, which drops the ENTIRE pending_choice
 * when `choice_player_id` is missing or names anybody but the requester.
 *
 * Neither is reachable from this binary. src/web_server.c is linked ONLY into
 * $(WEB_TARGET) (Makefile:101-102) and never into OBJ_LIB, so no
 * tests/test_*.c binary can link a symbol defined there -- the failure would be
 * an undefined reference at link time, not a test failure. The Makefile is the
 * coordinator's file.
 *
 * So this case asserts what CAN be asserted from a suite, and it is the part
 * that would still be wrong if the wiring were reverted: the producer's VALUE.
 * The leak filter and the JSON merge are exercised by the web target only.
 */
static void o2_choice_context_producer_reports_the_routed_seat(void)
{
    static TestGame tg;
    ensure_cards_loaded();
    test_game_new(&tg);

    const int akane = test_id_exact(&tg, "PL!S-bp7-003-P");
    CHECK(akane >= 0, "fixture: PL!S-bp7-003-P resolves exactly");
    if (akane < 0) return;

    /* A queue entry owned by seat 1 with no current one. */
    tg.state.queue.cur = -1;
    tg.state.queue.n_entries = 0;

    char buf[2048];
    buf[0] = 'S'; buf[1] = '\0';
    rb_inject_choice_ability_context(&tg.state, buf, sizeof buf);
    CHECK(buf[0] == '\0',
          "abilities.rs:2559 is reached only inside `if let Some(entry)`, so "
          "with no current entry the producer writes nothing rather than "
          "guessing a seat -- which is the safe direction for the leak filter "
          "at web_server.rs:1240-1249");

    tg.state.queue.n_entries = 1;
    tg.state.queue.cur = 0;
    RbQueueEntry *e = &tg.state.queue.entries[0];
    memset(e, 0, sizeof(*e));
    e->card_id = akane;
    snprintf(e->player_id, sizeof(e->player_id), "player1");

    rb_inject_choice_ability_context(&tg.state, buf, sizeof buf);
    CHECK(strstr(buf, "\"choice_player_id\":\"p1\"") != NULL,
          "abilities.rs:2559-2568 -- the producer normalises player1 to p1, "
          "which is the ONLY value web_server.rs:1240-1249 compares against the "
          "requester when deciding whether to show the choice at all");

    snprintf(e->player_id, sizeof(e->player_id), "player2");
    rb_inject_choice_ability_context(&tg.state, buf, sizeof buf);
    CHECK(strstr(buf, "\"choice_player_id\":\"p2\"") != NULL,
          "abilities.rs:2561 `\"player2\" => \"p2\"` -- and a value the p1 "
          "perspective must therefore HIDE, per web_server.rs:1244-1248");
}

int main(void)
{
    printf("=== orphaned-logic producers: O1 keyword reset, O3 wait immunity, O2 choice context ===\n");
    o1_active_entry_resets_the_keyword_trackers();
    o1_second_active_entry_in_the_same_round();
    o3_wait_immune_is_produced_on_the_live_path();
    o3_the_rollover_clears_the_list();
    o2_choice_context_producer_reports_the_routed_seat();
    if (cases == 0) {
        fprintf(stderr, "FAIL: parity_orphan_producers ran ZERO cases -- every test body stopped "
                        "before its first assertion; this is a broken suite, "
                        "not a passing one\n");
        return 1;
    }
    printf("parity_orphan_producers: %d cases, %d failures\n", cases, failures);
    return failures ? 1 : 0;
}
