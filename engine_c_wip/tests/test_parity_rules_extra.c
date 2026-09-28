/* test_parity_rules_extra.c — C port of the rules-cluster tests that
 * tests/test_parity_rules.c deliberately declared "no C entry point".
 *
 * Scope (Rust source -> section here):
 *   A. engine/tests/test_modules/rules/concession_rule_1_2_3_test.rs
 *   B. engine/tests/test_modules/rules/permanent_loop_rule_12_1_test.rs
 *   C. engine/tests/test_modules/rules/zones/
 *      resolution_zone_rule_5_8_swap_test.rs
 *   D. engine/tests/test_modules/rules/deck_construction_test.rs
 *
 * tests/test_parity_rules.c already pinned three of these as bare
 * `EXPECTED_GAP(0, ...)` markers. That is not enough: each one is re-opened
 * here, the ABSENCE is asserted by sweeping the real enum / the real function
 * table rather than by a single lookup, and wherever a genuinely WRONG engine
 * behaviour is found instead of a missing API the assertion is a hard CHECK
 * and is left RED.
 *
 * FINDINGS (all reproduced by this file; see the per-test comments):
 *   ENGINE BUG 1 - src/turn/phase.c:264 `rb_check_invalid_resolution_zone`
 *                 routes EVERY resolution-zone card to `g->p[g->active].discard`.
 *                 Rust (engine/src/turn/actions/mod.rs:1832) routes each card to
 *                 the seat recorded in `ResolutionZone::owners[i]` and only falls
 *                 back to the active seat when that array is short. The C
 *                 `RbBag` has no owners array, so a card revealed into the
 *                 resolution zone on behalf of the NON-active performer
 *                 (`add_card_for_owner(card_id, owner)` in
 *                 engine/src/core/game_state/tracking.rs:93 and
 *                 engine/src/turn/live.rs:1946) is returned to the WRONG
 *                 player's waitroom. Pinned red by
 *                 test_c_rule_5_8_returns_to_owner_not_active_seat.
 *   ENGINE BUG 2 - src/core/game_state_abilities.c:860
 *                 `rb_check_permanent_loop` TRUNCATES the 64-bit fingerprint to
 *                 a 32-bit history slot and then compares it back as 64-bit:
 *                   game_state_history[] is declared `int` (rabuka.h:1464),
 *                   h is a 64-bit FNV-1a value,
 *                   stored  : g->game_state_history[i] = (int)h;
 *                   compared: if ((uint64_t)g->game_state_history[i] == h)
 *                 The int is SIGN-EXTENDED on the way back, so the comparison
 *                 can only succeed when the high 32 bits of h are 0x00000000
 *                 (or 0xFFFFFFFF with a negative low word) -- roughly a 1 in
 *                 2^31 chance for an arbitrary state. The Rule 12.1 loop
 *                 detector therefore NEVER fires: `loop_detected` stays 0 no
 *                 matter how many identical samples are taken. Pinned red by
 *                 test_b_loop_detector_never_fires.
 *   ENGINE BUG 3 (LATENT, masked by BUG 2) - the same function's
 *                 `rb_generate_state_hash` (game_state_abilities.c:833) feeds
 *                 the detector only zone LENGTHS (`P->hand.n`, ...), never the
 *                 card IDs, and never `g->phase`. Rust
 *                 `GameState::generate_state_hash`
 *                 (engine/src/core/game_state/abilities.rs:3332) hashes the
 *                 ordered card-id vectors of every zone plus `current_phase`
 *                 and `current_turn_phase`. Once BUG 2 is fixed this becomes a
 *                 live false-positive: any two states with the same zone SIZES
 *                 will read as a loop. Recorded by
 *                 test_b_latent_fingerprint_omits_card_ids /
 *                 test_b_latent_fingerprint_omits_phase, which today observe
 *                 the MASKED behaviour and do NOT go red.
 *   ENGINE BUG 4 - src/turn/phase.c:288 calls `rb_check_permanent_loop` from
 *                 `rb_check_timing`, i.e. on EVERY timing check, while Rust
 *                 scopes it to the resolution loop. The in-tree comment at
 *                 phase.c:285-287 acknowledges the false positive but only
 *                 suppresses the DRAW, not the flag.
 *   API GAP     - no `rb_concede` / `rb_concede_both`, no RB_ACTION_CONCEDE,
 *                 no `rb_validate_deck_construction`, no
 *                 `ResolutionZone::swap_slots`, no
 *                 `GameState::record_action_boundary` /
 *                 `pending_loop_protocol` / `loop_last_action`.
 *
 * C-API landmines honoured here (AGENTS.md + peer findings):
 *   - load_card_db() falls back to rb_load("../cards/build"); rb_load("src")
 *     alone fails under the isolated out-of-tree build.
 *   - `mid()` == test_new_id(), not test_id(): Rust's game.id() allocates the
 *     next DISTINCT pool slot, C's template lookup aliases.
 *   - sizeof(TestGame) is ~781 KB, so EVERY TestGame local in this file is
 *     `static`. A second stack TestGame in one test segfaults.
 *   - Group identity keys off `unit`, not the printed group name.
 *   - Card.ability is not populated by rb_decode_card_by_index.
 */
#include "rabuka.h"
#include "test_game.h"
#include "deck_builder.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ── crash reporting + fork isolation ─────────────────────────────────────
 *
 * Every test runs in a forked child so a SIGSEGV/SIGBUS/SIGABRT in the engine
 * is attributed to the test in flight instead of truncating the run and making
 * every later section look like it passed.
 *
 * There is deliberately NO time-based watchdog: a process watchdog does not
 * work on this toolchain (an in-child alarm() never fires; a parent-side
 * waitpid(WNOHANG) deadline fires but its kill(SIGKILL) does not take effect,
 * wedging the blocking waitpid). A child that HANGS is not caught here. That
 * is a recorded limitation, not an oversight.
 */
static const char *current_test = "(none)";
static int failures;
static int assertions;
static int gaps;
static int setup_bugs;

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL SIGNAL %d inside test: %s\n"
            "*** The engine, not this test, is at fault.\n"
            "*** Assertions evaluated before the fault: %d, failures so far: %d\n",
            sig, current_test, assertions, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

/* A real assertion: failing it fails the suite. */
#define CHECK(condition, ...) do {                                          \
        assertions++;                                                       \
        if (!(condition)) {                                                 \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fputc('\n', stderr);                                            \
        }                                                                   \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                                \
        assertions++;                                                       \
        long long a_ = (long long)(actual);                                 \
        long long e_ = (long long)(expected);                               \
        if (a_ != e_) {                                                     \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fprintf(stderr, " (got %lld expected %lld)\n", a_, e_);         \
        }                                                                   \
    } while (0)

/* A Rust assertion with NO reachable C entry point. Counted separately; never
 * fails the suite, and never stands in for a real CHECK. */
#define EXPECTED_GAP(condition, desc) do {                                  \
        assertions++;                                                       \
        if (!(condition)) {                                                 \
            printf("GAP: %s\n", desc);                                      \
            gaps++;                                                         \
        }                                                                   \
    } while (0)

/* RbActionType's last enumerator; the enum has no RB_ACTION_CONCEDE. */
#define ACTION_TYPE_LAST RB_ACTION_CHOICE_CONDITION

/* ── fixtures ───────────────────────────────────────────────────────────── */

/* `src` is the in-tree layout; the isolated out-of-tree build root used by
 * tools/isolated_build.sh only has `../cards/build`. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* Rust `game.id("X")` -> the next DISTINCT pool slot. A miss is a SETUP bug
 * (the Rust fixture names a card_no that does not exist) and is counted
 * separately from parity failures. */
static int mid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        assertions++;
        setup_bugs++;
        failures++;
        fprintf(stderr, "SETUP BUG: card \"%s\" is not in the database\n", no);
    }
    return id;
}

static int bag_has(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static int waitroom_has(TestGame *tg, int pl, int cid)
{
    return bag_has(&tg->state.p[pl].discard, cid);
}

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }

/* Clear the per-player pieces test_game_new leaves behind so a test can drive
 * a phase or a zone in isolation. */
static void clear_stage(TestGame *tg, int pl)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        tg->state.p[pl].stage[i] = RB_EMPTY_SLOT;
        tg->state.p[pl].stage_wait[i] = 0;
        tg->state.p[pl].under_cards[i].n = 0;
    }
}

static void reset_boards(TestGame *tg)
{
    clear_stage(tg, 0);
    clear_stage(tg, 1);
    tg->state.resolution.n = 0;
}

/* ===================================================================== */
/* A. rules/concession_rule_1_2_3_test.rs (rule 1.2.3 — 投了)             */
/* ===================================================================== */

/* Rust concession_rule_1_2_3_test.rs:26 concession_action_is_wire_
 * representable:
 *   ActionType::Concede.to_string() == "concede"
 *   ActionType::from_str("concede") == Ok(Concede)
 *   ActionType::from_tag(Concede.to_tag()) == Concede
 *
 * The C enum has no RB_ACTION_CONCEDE. Rather than a single from_str probe
 * (which is all tests/test_parity_rules.c does) the WHOLE enum is swept in
 * both directions, so the gap is pinned against the entire vocabulary and
 * cannot be closed by adding a half-converted enumerator that only round-trips
 * one way.
 */
static void test_a_concede_is_absent_from_the_action_vocabulary(void)
{
    int from_str = rb_action_type_from_str("concede");
    CHECK_EQ(from_str, -1,
             "ENGINE GAP (API): \"concede\" does not parse to any RbActionType — "
             "ActionType::Concede was never ported");

    int found_by_to_str = 0;
    for (int at = 0; at <= ACTION_TYPE_LAST; at++) {
        const char *s = rb_action_type_to_str(at);
        if (s && strcmp(s, "concede") == 0) {
            found_by_to_str = 1;
            fprintf(stderr, "  (rb_action_type_to_str(%d) == \"concede\")\n", at);
        }
    }
    CHECK_EQ(found_by_to_str, 0,
             "ENGINE GAP (API): no RbActionType prints as \"concede\" — the "
             "to_string() half of the Rust wire round-trip is missing too");

    /* Every other action type the Rust wire protocol does carry must still
     * round-trip, so the sweep above is not vacuously red. */
    const char *probes[] = { "play_member_to_stage", "activate_ability",
                             "play_live_card", "pass", "end_turn" };
    int round_tripped = 0;
    for (size_t i = 0; i < sizeof probes / sizeof probes[0]; i++) {
        int at = rb_action_type_from_str(probes[i]);
        if (at < 0) continue;
        const char *back = rb_action_type_to_str(at);
        if (back && strcmp(back, probes[i]) == 0) round_tripped++;
    }
    CHECK(round_tripped > 0,
          "control: the action-type wire vocabulary DOES round-trip for the "
          "actions that were ported, so the concede gap is real");
}

/* Rust :36 concession_gives_the_game_to_the_opponent /
 * Rust :69 simultaneous_concessions_produce_a_draw /
 * Rust :77 concession_is_rejected_after_the_game_ends
 *
 * The C engine's terminal state is the absolute-seat `winner` field
 * (-1 none, 0 p1, 1 p2, 2 draw) plus `phase == RB_PHASE_DONE`. There is no
 * `concede`, no `game_ended` flag and no `game_result` enum, so the whole
 * concession decision surface is unreachable. The control below proves the
 * terminal state itself IS observable, so the gap is about the API and not
 * about an unobservable winner.
 */
static void test_a_concede_gives_the_game_to_the_opponent(void)
{
    static TestGame game;
    test_game_new(&game);
    reset_boards(&game);

    /* Control: a fresh game has no winner and is not terminal. */
    CHECK_EQ(game.state.winner, -1, "control: a fresh game has no winner");
    CHECK(game.state.phase != RB_PHASE_DONE, "control: a fresh game is not terminal");

    /* Rust concede(0) -> SecondAttackerWins (player1.is_first_attacker == true),
     * i.e. "the opponent of the conceder takes the game". */
    EXPECTED_GAP(0,
                 "GameState::concede has no C counterpart: the C winner field can "
                 "only be written by rb_check_victory_condition, so a concession "
                 "can neither be performed nor rejected");
    EXPECTED_GAP(0,
                 "GameState::concede_both has no C counterpart, so the "
                 "simultaneous-concession DRAW is not expressible");
    EXPECTED_GAP(0,
                 "there is no `game_ended` flag to reject a post-game concede "
                 "(Rust reject_if_ended), and RB_PHASE_DONE is not a guard any "
                 "admission path consults");
}

/* Rust :49 concession_dispatch_bypasses_pending_choice_and_replacement:
 * a Concede action dispatched for a player who CANNOT act must still apply.
 * The C engine has no action dispatcher for Concede, so the "bypasses the
 * pending choice" half cannot be driven. The queue-clearing half CAN be
 * observed: a concession in Rust empties ability_queue and clears both
 * pending_success_replacement fields. Nothing in C clears the queue.
 */
static void test_a_concede_does_not_bypass_a_pending_choice(void)
{
    static TestGame game;
    test_game_new(&game);
    reset_boards(&game);

    /* A plain deployment into an EMPTY area prompts nothing. A reliable
     * pending prompt comes from a named-replacement debut: 三船栞子's
     * 「PL!N-pb1-022-P＋」 replaces 三船栞子, draws 2 and then prompts for a
     * hand discard. That is the same fixture tests/test_parity_rules.c uses,
     * so the prompt is known to survive test_play_to_stage. */
    const char *MIA = "PL!N-pb1-022-P＋";
    const char *SHIOKO = "PL!N-sd1-010-SD";
    const char *FILLER = "PL!-sd1-010-SD";
    int mia = mid(&game, MIA);
    int shioko = mid(&game, SHIOKO);
    int filler = mid(&game, FILLER);
    CHECK(rb_card_no_eq(mia, MIA), "fixture: the baton toucher is the fullwidth-plus print");
    CHECK(rb_card_no_eq(shioko, SHIOKO), "fixture: the replaced member is the PL!N-sd1-010-SD print");

    test_give_energy(&game, 20);
    clear_stage(&game, 0);
    game.state.p[0].stage[1] = shioko;
    test_add_to_hand(&game, mia);
    for (int i = 0; i < 3; i++) test_add_to_deck_pl(&game, 0, filler);

    int played = test_play_to_stage(&game, mia, 1);
    CHECK_EQ(played, 1, "control: the named-replacement debut was accepted");
    CHECK(rb_has_pending_choice(&game.state),
          "control: the debut's hand-discard prompt is pending, so the "
          "concede-bypasses-the-prompt scenario is reachable");

    /* Rust: execute_action_for_player(state, concede, 1) succeeds even though
     * !state.can_player_act(1), and afterwards has_pending_choice() is false. */
    EXPECTED_GAP(!pending(&game),
                 "ENGINE GAP (API): there is no Concede action to dispatch, so a "
                 "concession cannot be shown to bypass the pending choice; the "
                 "prompt is still live");

    /* rb_execute_main_phase_action is the only C admission path; it refuses
     * while a choice is pending, so the two behaviours are not equivalent. */
    CHECK(rb_has_pending_choice(&game.state),
          "the pending choice is still pending after the (absent) concede");
}

/* ===================================================================== */
/* B. rules/permanent_loop_rule_12_1_test.rs (rule 12.1 — 永久ループ)        */
/* ===================================================================== */

/* Rust :7 repeated_action_protocol_routes_to_non_active_player_and_stops_
 * only_after_exact_repetition is driven by
 * `GameState::record_action_boundary(action)`, which:
 *   - clears the history when the ACTION changes,
 *   - pushes one hash per boundary,
 *   - raises a rule_12_1 SelectTarget only on the THIRD identical sample
 *     (`repetition_count < 3 => return`).
 * None of that exists in C. The C loop detector is
 * `rb_check_permanent_loop`, the sibling of Rust's
 * `GameState::check_permanent_loop` (the simpler 2-sample form), so the two
 * thresholds are deliberately different and must not be conflated.
 */
static void test_b_check_permanent_loop_needs_an_identical_repetition(void)
{
    static TestGame game;
    test_game_new(&game);
    reset_boards(&game);

    CHECK_EQ(rb_check_permanent_loop(&game.state), 0,
             "the first sample of a state is never a loop");
    CHECK_EQ(rb_is_loop_detected(&game.state), 0,
             "the first sample leaves loop_detected clear");

    CHECK_EQ(rb_check_permanent_loop(&game.state), 1,
             "ENGINE BUG: an immediately repeated, otherwise identical state IS a "
             "repetition, so rb_check_permanent_loop must return 1. It returns 0 "
             "because the 64-bit fingerprint is stored into the 32-bit "
             "`int game_state_history[64]` (rabuka.h:1464) and then compared "
             "sign-extended as 64-bit -- see the header FINDINGS, bug 2");
    CHECK_EQ(rb_is_loop_detected(&game.state), 1,
             "ENGINE BUG: a detected repetition sets loop_detected");
}

static void test_b_a_moved_on_state_is_not_a_repetition(void)
{
    static TestGame game;
    test_game_new(&game);
    reset_boards(&game);

    CHECK_EQ(rb_check_permanent_loop(&game.state), 0, "baseline sample");
    game.state.turn = 2;
    CHECK_EQ(rb_check_permanent_loop(&game.state), 0,
             "advancing the turn changes the fingerprint, so it is not a repetition");
    CHECK_EQ(rb_is_loop_detected(&game.state), 0, "no false positive from a turn bump");
}

/* ENGINE BUG 2 from another angle: the history slot cannot even hold the value
 * it is given, so the detector's whole mechanism is inert. Rust's
 * `game_state_history: Vec<u64>` (abilities.rs) keeps the full 64 bits and
 * `check_permanent_loop` therefore does fire.
 *
 * This test does NOT rely on the (unreachable) first repetition having been
 * recorded: it drives MANY identical samples and shows the flag never comes up.
 * A detector that cannot fire after 64 samples cannot be salvaged by a
 * different call pattern. */
static void test_b_loop_detector_never_fires(void)
{
    static TestGame game;
    test_game_new(&game);
    reset_boards(&game);

    for (int i = 0; i < 32; i++)
        rb_check_permanent_loop(&game.state);

    CHECK_EQ(game.state.n_game_state_history, 32,
             "control: the history did accumulate 32 samples, so the calls ran");
    CHECK_EQ(rb_is_loop_detected(&game.state), 0,
             "ENGINE BUG: 32 identical samples in a row must trip the Rule 12.1 "
             "repetition detector; the 32-bit history slot cannot represent the "
             "64-bit hash so the history scan never matches");
}

/* Rust :43 fingerprint_distinguishes_ordered_card_ids_with_equal_zone_
 * lengths is the exact regression the Rust fingerprint was hardened against:
 * Rust hashes the ordered card-id VECTOR of the hand, so replacing one card
 * with a different card of the same length is a DIFFERENT state.
 *
 * rb_generate_state_hash (game_state_abilities.c:833) pushes `P->hand.n` and
 * no card ids at all, so once bug 2 is fixed a -> b in a one-card hand WILL
 * read as a repetition. Today bug 2 masks it, so this test records the LATENT
 * defect and asserts only the behaviour that is actually observable: no
 * false positive. The line marked LATENT is the defect; it is deliberately not
 * a red assertion because claiming a false positive that does not occur would
 * be a worse report than an acknowledged latent one.
 */
static void test_b_latent_fingerprint_omits_card_ids(void)
{
    static TestGame game;
    test_game_new(&game);
    reset_boards(&game);

    const char *A = "PL!N-bp1-001-R";
    const char *B = "PL!N-pb1-001-R";
    int a1 = mid(&game, A);
    int b1 = mid(&game, B);
    CHECK(rb_card_no_eq(a1, A), "fixture: hand slot 1 is the PL!N-bp1-001-R print");
    CHECK(rb_card_no_eq(b1, B), "fixture: hand slot 2 is the PL!N-pb1-001-R print");
    CHECK(a1 != b1, "the two fixtures are distinct pool slots, not one aliased card");

    /* Rust: push a; record; clear; push b; record; clear; push a; record;
     * assert no repetition. Three DIFFERENT hands of equal length. */
    game.state.p[0].hand.cards[0] = a1;
    game.state.p[0].hand.n = 1;
    CHECK_EQ(rb_check_permanent_loop(&game.state), 0, "hand {a}: first sample");

    game.state.p[0].hand.cards[0] = b1;
    game.state.p[0].hand.n = 1;
    CHECK_EQ(rb_check_permanent_loop(&game.state), 0,
             "observed today: swapping a different card into a same-length hand "
             "does not read as a repetition (but only because the 32-bit "
             "truncation disables the detector entirely)");

    game.state.p[0].hand.cards[0] = a1;
    game.state.p[0].hand.n = 1;
    CHECK_EQ(rb_check_permanent_loop(&game.state), 0,
             "observed today: returning to a previously seen state is NOT "
             "reported, which is the same masking bug seen from the other side");

    EXPECTED_GAP(0,
                 "LATENT ENGINE BUG (masked by the history truncation): "
                 "rb_generate_state_hash pushes P->hand.n and never the card ids, "
                 "while Rust hashes the ordered id vector. The moment the 32-bit "
                 "history slot is widened to 64 bits, a->b in a one-card hand "
                 "becomes a FALSE loop detection");

    game.state.p[0].hand.n = 0;
}

/* Rust generate_state_hash also hashes `current_phase` and
 * `current_turn_phase` (abilities.rs:3347-3348). The C fingerprint does not,
 * so once bug 2 is fixed two different phases with identical zone sizes
 * collide. Combined with phase.c:288 -- which calls the detector from
 * rb_check_timing on EVERY timing check, whereas Rust scopes it to the
 * resolution loop -- that is a false positive on ordinary board states. */
static void test_b_latent_fingerprint_omits_phase(void)
{
    static TestGame game;
    test_game_new(&game);
    reset_boards(&game);

    game.state.phase = RB_PHASE_MAIN;
    CHECK_EQ(rb_check_permanent_loop(&game.state), 0, "Main phase: first sample");

    game.state.phase = RB_PHASE_LIVE_SET;
    CHECK_EQ(rb_check_permanent_loop(&game.state), 0,
             "observed today: a PHASE change alone does not read as a repetition");

    game.state.phase = RB_PHASE_PERFORMANCE;
    CHECK_EQ(rb_check_permanent_loop(&game.state), 0,
             "observed today: three different phases do not collapse into one "
             "detected repetition");

    EXPECTED_GAP(0,
                 "LATENT ENGINE BUG (masked by the history truncation): "
                 "rb_generate_state_hash never reads g->phase, while Rust hashes "
                 "current_phase and current_turn_phase. Once the history slot is "
                 "widened, advancing a phase with an unchanged board becomes a "
                 "FALSE loop detection, and rb_check_timing calls the detector on "
                 "every timing check (src/turn/phase.c:288) while Rust scopes it "
                 "to the resolution loop");
}

/* Rust :26 protocol_continuation_and_different_action_clear_the_repetition_
 * path -- the "Continue" answer clears the history, and a DIFFERENT action
 * clears it too. C has no rule_12_1 choice and no loop_last_action, so
 * neither clearing path is reachable. */
static void test_b_rule_12_1_protocol_and_clearing_are_absent(void)
{
    static TestGame game;
    test_game_new(&game);
    reset_boards(&game);

    rb_check_permanent_loop(&game.state);
    rb_check_permanent_loop(&game.state);
    EXPECTED_GAP(rb_is_loop_detected(&game.state),
                 "ENGINE BUG (consequence of the history truncation): the "
                 "repetition is not recorded at all, so even the flag the Rust "
                 "protocol keys off is never raised");

    /* Rust raises a SelectTarget with target == "rule_12_1" and
     * allow_skip == false on the THIRD identical boundary. The C detector
     * raises no choice at all -- it only sets a flag. */
    EXPECTED_GAP(pending(&game),
                 "ENGINE GAP (API): rule 12.1 has no prompt. Rust raises a "
                 "non-skippable SelectTarget(target=\"rule_12_1\", options="
                 "[\"Stop\",\"Continue\"]) on the third identical boundary; "
                 "rb_check_permanent_loop only sets the loop_detected flag");
    EXPECTED_GAP(0,
                 "GameState::resolve_loop_protocol has no C counterpart, so the "
                 "\"Stop\" answer (draw the game) and the \"Continue\" answer "
                 "(clear the history, clear loop_detected) are both unreachable");
    EXPECTED_GAP(0,
                 "GameState::record_action_boundary has no C counterpart, so the "
                 "history cannot be CLEARED by a different action and the "
                 "3-sample threshold of rule 12.1 cannot be reproduced");
    EXPECTED_GAP(0,
                 "engine/src/core/game_state/abilities.rs:3285 only clears the "
                 "history when loop_last_action changes; the C side has no "
                 "loop_last_action, so every C repetition is action-agnostic");
}

/* ===================================================================== */
/* C. rules/zones/resolution_zone_rule_5_8_swap_test.rs (rule 5.8)         */
/* ===================================================================== */

/* Characterisation of what the C resolution zone actually does, so the red
 * assertion below cannot be mistaken for a mis-set-up test.
 *
 * engine/src/core/zones.rs:1045 ResolutionZone = { cards: SmallVec<i16>,
 * owners: SmallVec<u8> }. The C twin (rabuka.h:1418) is a bare `RbBag
 * resolution`, so the owner vector is simply absent.
 */
static void test_c_resolution_zone_is_an_unowned_bag(void)
{
    static TestGame game;
    test_game_new(&game);
    reset_boards(&game);

    int filler = mid(&game, "PL!-sd1-010-SD");
    CHECK(filler >= 0, "fixture: the generic filler resolves");

    game.state.active = 0;
    game.state.resolution.cards[0] = filler;
    game.state.resolution.cards[1] = filler;
    game.state.resolution.n = 2;

    CHECK_EQ(game.state.resolution.n, 2, "the resolution zone holds both entries");
    CHECK_EQ(game.state.resolution.cards[0], filler,
             "the resolution zone stores card ids");

    rb_check_invalid_resolution_zone(&game.state);
    CHECK_EQ(game.state.resolution.n, 0, "the drain empties the resolution zone");
    CHECK(waitroom_has(&game, 0, filler),
          "C behaviour: rb_check_invalid_resolution_zone routes to the ACTIVE seat");
    CHECK(!waitroom_has(&game, 1, filler),
          "C behaviour: the non-active seat receives nothing");

    /* Rust ResolutionZone::swap_slots(0, 1) exists only because the two
     * parallel vectors must stay index-aligned; with no owners vector there is
     * nothing to keep aligned and nothing to preflight. */
    EXPECTED_GAP(0,
                 "ResolutionZone::swap_slots (rule 5.8) has no C counterpart: "
                 "without an owners vector the C zone cannot swap owners with "
                 "their cards, and its preflight checks (same slot / out of "
                 "bounds / short owners array) have nothing to validate");
}

/* ENGINE BUG 1, strict. engine/src/turn/actions/mod.rs:1832:
 *
 *   let owners = mem::take(&mut gs.resolution_zone.owners);
 *   for (index, card_id) in cards.into_iter().enumerate() {
 *       let owner = owners.get(index).copied().unwrap_or(active_seat);
 *       ... push card_id to that player's waitroom ...
 *
 * i.e. the OWNER wins and the active seat is only a fallback for a short
 * owners array. C (src/turn/phase.c:264) unconditionally uses
 * `g->p[g->active].discard`.
 *
 * This is reachable: tracking.rs:93 (perform_cheer_check) and live.rs:1946
 * (reveal_yell_cards) both stamp the performing seat, and the performance
 * phase runs for BOTH seats, so the non-active performer's revealed yell cards
 * are returned to the other player's waitroom.
 */
static void test_c_rule_5_8_returns_to_owner_not_active_seat(void)
{
    static TestGame game;
    test_game_new(&game);
    reset_boards(&game);

    int filler = mid(&game, "PL!-sd1-010-SD");
    CHECK(filler >= 0, "fixture: the generic filler resolves");

    /* Seat 0 is active; the card entered the zone stamped for seat 1 (the
     * non-active performer). RbBag cannot record that stamp, which is the
     * whole point of this assertion. */
    game.state.active = 0;
    game.state.resolution.cards[0] = filler;
    game.state.resolution.n = 1;

    rb_check_invalid_resolution_zone(&game.state);

    CHECK(waitroom_has(&game, 1, filler),
          "ENGINE BUG: rule 5.8 — a resolution-zone card returns to the waitroom "
          "of the seat that OWNS it, not to the active seat. rb_check_invalid_"
          "resolution_zone (src/turn/phase.c:264) hard-codes g->p[g->active], so "
          "the non-active performer's revealed yell cards land in the wrong "
          "player's waitroom");
    CHECK(!waitroom_has(&game, 0, filler),
          "ENGINE BUG: the ACTIVE seat must not receive a card owned by the "
          "other performer (Rust only falls back to active_seat when the owners "
          "array is SHORTER than the card list)");
}

/* Rust :16 rule_5_8_swap_preflight_failure_leaves_zone_unchanged — every
 * failing swap must leave the zone byte-identical. There is no C swap to
 * preflight, and no way to provoke a partial mutation. */
static void test_c_rule_5_8_preflight_leaves_the_zone_unchanged(void)
{
    static TestGame game;
    test_game_new(&game);
    reset_boards(&game);

    int a = mid(&game, "PL!N-bp1-001-R");
    int b = mid(&game, "PL!N-pb1-001-R");
    CHECK(a >= 0 && b >= 0, "fixtures resolve");

    /* Rust builds [11(owner 1), 12(owner 2)] then swap_slots(1, 2) fails and
     * the zone is unchanged. The C zone is only ever appended to and drained,
     * so the observable equivalent is the drain: it must be all-or-nothing. */
    game.state.active = 0;
    game.state.resolution.cards[0] = a;
    game.state.resolution.cards[1] = b;
    game.state.resolution.n = 2;
    rb_check_invalid_resolution_zone(&game.state);

    CHECK_EQ(game.state.resolution.n, 0, "the drain is complete");
    CHECK_EQ(game.state.p[0].discard.n, 2, "both entries were routed, none dropped");
    EXPECTED_GAP(0,
                 "ResolutionZone::swap_slots preflight (same slot / out of bounds "
                 "/ short owners) has no C counterpart, so 'a rejected swap leaves "
                 "the zone unchanged' cannot be asserted");
}

/* ===================================================================== */
/* D. rules/deck_construction_test.rs (rule 6.1.1 — デッキ構築)            */
/* ===================================================================== */

/* engine/src/game/deck_builder.rs:31 validate_deck_construction enforces
 *   - max 4 copies of any CANONICAL card_no, counted by the card's own
 *     card_no (so a differently-cased request still counts toward the limit),
 *   - exactly 48 members, exactly 12 live cards, exactly 12 energy cards.
 * There is no C equivalent: deck_builder.h exposes only
 * rb_build_deck_from_*. */
static void test_d_rule_6_1_1_deck_legality_is_not_validated(void)
{
    int a = rb_find_card_by_no("PL!N-bp1-001-R");
    int b = rb_find_card_by_no("PL!N-pb1-001-R");
    int live = rb_find_card_by_no("PL!-sd1-019-SD");
    int energy = rb_find_card_by_no("LL-E-001-SD");
    CHECK(a >= 0 && b >= 0 && live >= 0 && energy >= 0,
          "control: the fixture prints all resolve in the real database");

    /* Rust counts_aliases_toward_canonical_copy_limit: a 5th copy is
     * rejected with "... has 5 copies (maximum 4)". */
    int five[5];
    for (int i = 0; i < 5; i++) five[i] = a;
    RbBuiltDeck deck;
    memset(&deck, 0, sizeof deck);
    int rc = rb_build_deck_from_card_ids(five, 5, NULL, 0, &deck);
    CHECK_EQ(rc, 0,
             "C behaviour: the builder accepts 5 copies of one card — the "
             "rule 6.1.1 four-copy cap is not enforced anywhere in deck_builder.c");
    CHECK_EQ(deck.main_count, 5, "C behaviour: all 5 copies land in the main deck");
    rb_built_deck_clear(&deck);

    /* Rust rejects_under_and_over_category_totals: a 59-card or 61-card deck
     * is rejected. The C builder only rejects past RB_MAX_DECK (60). */
    int sixty[60];
    for (int i = 0; i < 60; i++) sixty[i] = a;
    CHECK_EQ(rb_build_deck_from_card_ids(sixty, 60, NULL, 0, &deck), 0,
             "C behaviour: exactly 60 main-deck cards is accepted (RB_MAX_DECK)");
    rb_built_deck_clear(&deck);

    EXPECTED_GAP(0,
                 "validate_deck_construction was never ported: the 48/12/12 "
                 "category totals of rule 6.1.1 are not checked, so a deck with "
                 "0 live cards builds exactly like a legal one");
    EXPECTED_GAP(0,
                 "the 4-copy limit is not enforced, and the error text "
                 "(\"Card X has N copies (maximum 4)\") that the Rust test "
                 "matches on has no C counterpart");
}

/* Rust counts_aliases_toward_canonical_copy_limit depends on the id resolver
 * folding a differently-cased request onto the same CANONICAL card. That
 * resolver step IS observable in C (rb_find_card_by_no / rb_card_get_card_id),
 * so this half of the Rust test is genuinely portable even though the
 * validator is not. */
static void test_d_case_variant_resolves_to_the_canonical_print(void)
{
    int canonical = rb_find_card_by_no("PL!N-bp1-001-R");
    int lower = rb_find_card_by_no("pl!n-bp1-001-r");
    int mixed = rb_find_card_by_no("Pl!N-Bp1-001-R");

    CHECK(canonical >= 0, "control: the canonical print resolves");
    CHECK_EQ(lower, canonical,
             "a lower-cased request folds onto the canonical template, which is "
             "what makes a case variant count toward the same copy limit");
    CHECK_EQ(mixed, canonical,
             "a mixed-case request folds onto the same canonical template");

    /* The ASYNC half of the fold: the resolved copy must still report the
     * CANONICAL card_no, not the spelling that was asked for. */
    if (canonical >= 0) {
        Card card;
        memset(&card, 0, sizeof card);
        if (rb_decode_card_by_index((uint32_t)canonical, &card)) {
            CHECK(rb_card_no_eq(canonical, "PL!N-bp1-001-R"),
                  "the resolved template reports the canonical card_no");
            rb_free_card(&card);
        } else {
            CHECK(0, "the canonical template decodes");
        }
    }
}

/* Rust :121 legacy_builder_still_accepts_starter_sized_lists:
 *   DeckBuilder::build_deck_from_database(vec!["TEST-M-000-N"]) succeeds with
 *   main_deck.len() == 1 and energy_deck EMPTY (deck_builder.rs:101-127 never
 *   fills the energy deck; only add_default_energy_cards_from_database does).
 *
 * The C twin rb_build_deck_from_card_numbers instead pads the energy deck to
 * RB_MAX_ENERGY_CARDS from a default template (deck_builder.c:54-60). That is
 * a deliberate, already-pinned C design (tests/test_deck_builder.c:54), so it
 * is characterised green here and the divergence is reported rather than
 * asserted against.
 */
static void test_d_legacy_builder_accepts_a_starter_sized_list(void)
{
    const char *nos[1] = { "PL!N-bp1-001-R" };
    RbBuiltDeck deck;
    memset(&deck, 0, sizeof deck);

    int rc = rb_build_deck_from_card_numbers(nos, 1, NULL, 0, &deck);
    CHECK_EQ(rc, 0, "a one-card starter list builds (Rust: expect(\"starter-sized deck\"))");
    CHECK_EQ(deck.main_count, 1, "the single member is the only main-deck entry");
    CHECK(deck.main_cards[0] >= 0, "the main-deck entry is a live card id");
    CHECK(rb_card_no_eq(deck.main_cards[0], "PL!N-bp1-001-R"),
          "the built copy resolves back to the requested print");
    CHECK_EQ(deck.energy_count, RB_MAX_ENERGY_CARDS,
             "C DIVERGENCE: the energy deck is padded to 12 from a default "
             "template; Rust's build_deck_from_database leaves it EMPTY");
    rb_built_deck_clear(&deck);
}

/* Rust :90 rejects_mixed_totals_with_correct_grand_total — the member/live
 * error must be the one whose category is actually wrong, and the grand total
 * stays 60. C has no category accounting at all, so the only observable part
 * is that the grand-total overflow guard fires at RB_MAX_DECK. */
static void test_d_grand_total_overflow_is_the_only_legality_guard(void)
{
    int a = rb_find_card_by_no("PL!N-bp1-001-R");
    int live = rb_find_card_by_no("PL!-sd1-019-SD");
    CHECK(a >= 0 && live >= 0, "control: the fixture prints resolve");

    /* Rust's deck_construction_test builds 48 member + 12 live = 60 main-deck
     * cards; the builder treats live cards as main-deck entries too. */
    int main_ids[60];
    int n = 0;
    for (int i = 0; i < 48; i++) main_ids[n++] = a;
    for (int i = 0; i < 12; i++) main_ids[n++] = live;
    CHECK_EQ(n, 60, "the rule 6.1.1 shape is 48 members + 12 live = 60");

    RbBuiltDeck deck;
    memset(&deck, 0, sizeof deck);
    CHECK_EQ(rb_build_deck_from_card_ids(main_ids, 60, NULL, 0, &deck), 0,
             "an exactly-60-card list of the right CATEGORY SHAPE builds");
    CHECK_EQ(deck.main_count, 60, "all 60 entries stay in the main deck");
    rb_built_deck_clear(&deck);

    /* One past the cap is the only rejection the C builder performs. */
    int main_ids_61[61];
    for (int i = 0; i < 61; i++) main_ids_61[i] = a;
    CHECK_EQ(rb_build_deck_from_card_ids(main_ids_61, 61, NULL, 0, &deck),
             RB_DECK_BUILD_EOVERFLOW,
             "61 main-deck cards is the only rule 6.1.1-adjacent rejection that exists");

    EXPECTED_GAP(0,
                 "the C builder keeps NO per-category counters, so the Rust "
                 "assertion that the MEMBER error (not the live error) is the "
                 "one reported for a mixed-total deck cannot be reproduced");
}

/* ===================================================================== */
/* main                                                                    */
/* ===================================================================== */

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_CRASHED   2

static int n_tests_ok, n_tests_failed, n_tests_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        int a0 = assertions, f0 = failures;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = (int)(failures - f0);
        if (nf > 0) n_tests_failed++; else n_tests_ok++;
        printf("%-8s %s  [%d assertions, in-process fallback]\n",
               nf ? "FAILED" : "ok", name, (int)(assertions - a0));
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = assertions, f0 = failures, s0 = setup_bugs;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s)\n",
               name, (int)(assertions - a0), (int)(failures - f0),
               (int)(setup_bugs - s0));
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);

    if (WIFSIGNALED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n",
               "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_tests_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_tests_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_tests_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_tests_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

int main(void)
{
    /* stdout is block-buffered when redirected to a file, so a crash mid-run
     * would swallow every ok/FAILED line and the summary. */
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    printf("--- §A concession_rule_1_2_3_test.rs ---\n");
    run("a_concede_absent_from_action_vocabulary",
        test_a_concede_is_absent_from_the_action_vocabulary);
    run("a_concede_gives_game_to_opponent",
        test_a_concede_gives_the_game_to_the_opponent);
    run("a_concede_does_not_bypass_pending_choice",
        test_a_concede_does_not_bypass_a_pending_choice);

    printf("--- §B permanent_loop_rule_12_1_test.rs ---\n");
    run("b_loop_needs_identical_repetition",
        test_b_check_permanent_loop_needs_an_identical_repetition);
    run("b_moved_on_state_not_a_repetition",
        test_b_a_moved_on_state_is_not_a_repetition);
    run("b_loop_detector_never_fires",
        test_b_loop_detector_never_fires);
    run("b_latent_fingerprint_omits_card_ids",
        test_b_latent_fingerprint_omits_card_ids);
    run("b_latent_fingerprint_omits_phase",
        test_b_latent_fingerprint_omits_phase);
    run("b_rule_12_1_protocol_absent",
        test_b_rule_12_1_protocol_and_clearing_are_absent);

    printf("--- §C resolution_zone_rule_5_8_swap_test.rs ---\n");
    run("c_resolution_zone_is_unowned_bag",
        test_c_resolution_zone_is_an_unowned_bag);
    run("c_rule_5_8_returns_to_owner",
        test_c_rule_5_8_returns_to_owner_not_active_seat);
    run("c_rule_5_8_preflight_leaves_zone_unchanged",
        test_c_rule_5_8_preflight_leaves_the_zone_unchanged);

    printf("--- §D deck_construction_test.rs ---\n");
    run("d_rule_6_1_1_legality_not_validated",
        test_d_rule_6_1_1_deck_legality_is_not_validated);
    run("d_case_variant_canonicalises",
        test_d_case_variant_resolves_to_the_canonical_print);
    run("d_legacy_builder_starter_list",
        test_d_legacy_builder_accepts_a_starter_sized_list);
    run("d_grand_total_overflow_guard",
        test_d_grand_total_overflow_is_the_only_legality_guard);

    rb_unload();

    /* The per-test assertion / failure / setup-bug / gap tallies live in the
     * CHILD, whose memory copy dies with it, so they are printed by each child
     * on its own line. The parent can only carry the test-level tally. */
    printf("\n==== parity_rules_extra ====\n");
    printf("tests run              : %d\n",
           n_tests_ok + n_tests_failed + n_tests_crashed);
    printf("tests passed           : %d\n", n_tests_ok);
    printf("tests failed (parity)  : %d\n", n_tests_failed);
    printf("tests crashed (engine) : %d\n", n_tests_crashed);
    printf("per-test assertion / failure / setup-bug / gap counts are on the\n"
           "child lines above (GAP: lines are the documented API gaps).\n");

    if (failures) {
        fprintf(stderr, "%d runtime failure(s) in the PARENT process\n", failures);
        return 1;
    }
    if (n_tests_crashed) {
        fprintf(stderr, "%d test(s) crashed in the engine.\n", n_tests_crashed);
        return 1;
    }
    if (n_tests_failed) {
        fprintf(stderr,
                "PARITY RULES EXTRA: %d failing test(s) — every FAIL line is a\n"
                "strict Rust expectation the C engine does not meet. See the\n"
                "FINDINGS list at the top of this file. Not a bug in this test.\n",
                n_tests_failed);
        return n_tests_failed > 125 ? 125 : n_tests_failed;
    }
    printf("ALL PARITY_RULES_EXTRA CHECKS PASSED\n");
    return 0;
}
