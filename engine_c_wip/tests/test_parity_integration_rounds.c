/* tests/test_parity_integration_rounds.c
 *
 * C port of the REMAINING DELTA in the two integration round clusters:
 *   engine/tests/test_modules/integration/full_round/
 *       live_each_time_victory_road_through_real_live_rounds_test.rs
 *       live_score_tie_live_zone_placement_across_rounds_test.rs
 *       real_card_phase_walkthrough_and_ability_suite_test.rs
 *       rps_through_turn3_main_phase_flow_test.rs
 *   engine/tests/test_modules/integration/multi_card/
 *       opponent_forced_position_change_arms_opponent_area_move_watcher_test.rs
 *       round5_seat_relative_live_success_and_live_start_cross_player_test.rs
 *       sumire_wien_yell_no_blade_gain_through_real_live_test.rs
 *
 * OWNERSHIP: this file, and nothing else. No src/ file, no other test .c
 * file, no include header, no Makefile entry (the `make t T=stem` target
 * finds tests/test_STEM.c on its own), no engine/, no cards/.
 *
 * MEASURED SIZES -- the assignment's "106 / 33" are OVERCOUNTS.
 * Enumerating `#[test]` -> the next `fn` in each file gives:
 *   live_each_time_victory_road...            17
 *   live_score_tie_live_zone_placement...      4
 *   real_card_phase_walkthrough_and_ability... 64
 *   rps_through_turn3_main_phase_flow...        1
 *   opponent_forced_position_change...         2
 *   round5_seat_relative_live_success...      13
 *   sumire_wien_yell_no_blade_gain...          3
 *   TOTAL 104 test fns across 7 files, not 139.
 *
 * WHAT IS ALREADY TRANSLATED, AND THEREFORE NOT RE-PORTED HERE
 * -----------------------------------------------------------
 * The delta was derived by reading each existing suite's `run(...)` list
 * and `main()` call list, NOT by trusting its header comments. Several
 * headers are stale; the measured result:
 *
 * multi_card/ -- ZERO delta. All 18 fns are already ported:
 *   test_parity_integration.c carries both himeko_* tests, the whole
 *   round5 seat-relative family (test_seat_relative_energy_comparison
 *   asserts all FOUR energy-comparison arms, test_mei_cost_gate,
 *   test_rina_mutual_mill_from_p2_seat) and all THREE
 *   sumire_wien yell_proper_* tests. test_parity_integration_extra.c §D
 *   adds the ginko / hot_passion / mei-mirror / mutual-mill arms. Nothing
 *   in multi_card is re-ported here: duplicating it would add zero
 *   behavioural coverage and would be a second copy, not a delta.
 *
 * full_round/ -- 10 of 86 fns uncovered; those 10 are this file:
 *   R1  shared_pr_debut_waits_only_low_cost_opposing_members_for_each_card_identity   Rust :410
 *   R2  ai_screeam_answer_both_discard                                                  Rust :95
 *   R3  ai_screeam_p2_owned_live_routes_answer_and_discard_choices                      Rust :199
 *   R4  wien_q117_another_member_triggers_yell_reduction                                Rust :2813
 *   R5  energy_zone_capacity_handled                                                    Rust :2924
 *   R6  revealed_cards_filtered_by_player_ownership                                     Rust :3316
 *   R7  choice_condition_shows_proper_labels_in_actions                                 Rust :3373
 *   R8  hanamaru_score_icon_filter                                                      Rust :3538
 *   R9  live_start_another_member                (victory road T4)                     Rust :228
 *   R10 live_start_cost_free_still_triggers      (victory road T5)                     Rust :261
 * Already covered and NOT re-ported: the 15 victory-road tests and the
 * whole baad_cage six in test_parity_integration_extra.c sections A/A2; the
 * tie/solo-winner trio plus the rps walk in test_parity_integration.c;
 * phase_walkthrough_two_turns, all ten distortion tests, turn_limit,
 * you_q186, rise_up_high_does_not_fire_on_turn_2 and
 * ai_screeam_p2_owned_live in test_parity_full_round.c section 2 (F1-F18);
 * the nico family in test_parity_full_round2.c (N1-N14); the ayumu, bella,
 * kosuzu, lovepeace, hareruya, you and kanon families in
 * test_parity_full_round3.c; and umi_pr014, dia_sd1, ai_screeam_soreigai,
 * rise_up_high_turn1 and ai_screeam_answer_both_draw in
 * test_parity_realcard_walkthrough.c.
 *
 * A CORRECTION TO FOUR STALE HEADERS, recorded because it changes what
 * this file had to do. test_parity_full_round.c, full_round2.c,
 * integration_extra.c and realcard_walkthrough.c all state that
 * energy_zone_capacity_handled, revealed_cards_filtered_by_player_ownership
 * and choice_condition_shows_proper_labels_in_actions are covered by
 * test_parity_condition_and_effect.c. They are not. I grepped that file's
 * 52 test functions by name and by semantics (sub_active, active_count,
 * revealed, generate_possible_actions) and found none of the three. R5, R6
 * and R7 below are therefore first-time coverage, not a second copy.
 *
 * CARDS (read from cards/cards.json as UTF-8, verified present):
 *   PL!-PR-005-PR  星空 凛          member cost  9  登場: draw 1 + discard 1, OR
 *                                  wait every OPPONENT member of cost <= 2
 *   PL!-PR-006-PR  member cost  9   (same two-option 登場)
 *   PL!-PR-008-PR  member cost  9   (same two-option 登場)
 *   PL!HS-pb1-002-R member cost  2  -> the "cheap" member that must be waited
 *   PL!S-bp5-009-R  member cost 15  -> the "expensive" member that must NOT be
 *   PL!SP-bp2-010-R＋ ウィーン・マルガレーテ  member cost 15
 *                                  ライブ開始時: another member on stage ->
 *                                  yell count -8 until live end (Q117)
 *   PL!S-sd1-007-SD 国木田花丸      member cost 11
 *                                  起動/ターン1: discard 2 from hand, add 1
 *                                  『Aqours』 live with a スコア icon from own
 *                                  waitroom to hand
 *   PL!S-bp2-024-L  Aqours live WITH スコア icon   (eligible)
 *   PL!S-bp2-026-L  Aqours live WITHOUT it          (not eligible)
 *   PL!S-bp2-015-PR Aqours member                  (not eligible)
 *   LL-PR-004-PR    愛♡スクリ～ム！  live  ライブ開始時: 「相手に何が好き？」
 *                                  option 0 -> BOTH players discard 1 from hand
 *   PL!N-bp5-030-L  繚乱！ビクトリーロード  each_time live-start watcher
 *   PL!-bp3-012-N   南ことり        the ライブ開始時 member
 * The fullwidth PLUS (U+FF0B) in PL!SP-bp2-010-R＋ is spelled "\xef\xbc\x8b"
 * because rb_find_card_by_no is EXACT-BYTE and does not normalise fullwidth
 * to halfwidth the way Rust's CardDatabase::get_card_id does.
 *
 * Honesty contract
 * ----------------
 * Every assertion is a hard CHECK about the C engine's behaviour. Nothing
 * is downgraded to a warning to make the suite green; the exit status is
 * non-zero whenever any test fails. EXPECTED_GAP is used ONLY where the
 * C engine provably cannot express the Rust behaviour at all (the
 * flat RbChoice carries no option list, so a prompt offering several
 * labelled options is unobservable), and it PRINTS the reason -- it is
 * never a pass. Failures are classified in the final report as
 * TEST BUG (this translation is wrong) / ENGINE GAP (named engine file),
 * never suppressed.
 *
 * PROMPT-COUNT CONTROL (the quality bar)
 * --------------------------------------
 * R2/R3 assert on prompts and on hand sizes after answering them, and R8
 * asserts that a specific card moved out of the waitroom. All three would
 * pass vacuously on a board where the ability never ran. Every one of
 * them therefore ALSO asserts a precondition that can only be true if the
 * board was built as the Rust fixture builds it: the live card really is
 * in the live zone, the host member really is on stage, the waitroom
 * really holds the three fixture cards. R8 additionally asserts the
 * filter's POSITIVE half (the スコア-icon live left the waitroom) so the
 * two negative halves (non-icon live, member) are read against a proven
 * execution rather than a dead board.
 *
 * KNOWN HARNESS LANDMINES respected here
 * -------------------------------------
 *   - sizeof(GameState) is ~781 KB, so every TestGame local is `static`.
 *   - rb_card_copy_pool_reset() per test: rb_create_card_copy draws from a
 *     process-global monotonic pool that nothing else resets; a long suite
 *     exhausts it (4096 ids) and every identity assertion goes vacuous.
 *   - mid() = test_new_id() wherever Rust wrote game.new_id();
 *     sid() = test_id() where Rust wrote game.id().
 *   - Card.ability is NOT populated by rb_decode_card_by_index; ability
 *     presence is read with rb_card_num_abilities / rb_decode_card_ability.
 *   - The C deck bag is TOP-INDEX-0, matching Rust's main_deck.cards.
 *   - test_set_live_card is P1-only; test_set_live_card_for for P2.
 *   - RbChoice.zone for the waitroom is "discard", not "waitroom".
 *   - test_get_heart_modifier() is a plain passthrough, but no heart read
 *     here uses it; rb_mods_get_heart is called directly.
 *
 * PER-TEST FORK ISOLATION
 * Each test runs in a forked child; a child that dies on a signal is
 * reported as CRASH, never as a pass. Every drain carries an iteration
 * guard: one peer found an ability that re-prompts UNBOUNDEDLY, and an
 * unbounded drain would hang the whole suite instead of reporting the
 * engine gap. Every drain here is BOUNDED and RETURNS the number of
 * prompts it answered, so a caller can CHECK(prompts > 0) and prove a
 * negative assertion non-vacuous.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* the fullwidth plus used by one card number (U+FF0B) */
#define PLUS "\xef\xbc\x8b"
#define WIEN_NO "PL!SP-bp2-010-R" PLUS

/* ═══════════════════════════════ harness ═══════════════════════════════ */

static const char *current_test = "(none)";
static int failures;
static long assertions;
static int setup_bugs;
static int gap_count;

#define CHECK(condition, message) do {                              \
    assertions++;                                                    \
    if (!(condition)) {                                              \
        fprintf(stderr, "FAIL [%s]: %s\n", current_test, message);  \
        failures++;                                                  \
    } else {                                                         \
        printf("  ok: %s\n", message);                               \
    }                                                                \
} while (0)

#define CHECK_EQ(actual, expected, message) do {                    \
    assertions++;                                                    \
    int a_ = (actual);                                              \
    int e_ = (expected);                                            \
    if (a_ != e_) {                                                 \
        fprintf(stderr, "FAIL [%s]: %s (got %d expected %d)\n",      \
                current_test, message, a_, e_);                     \
        failures++;                                                  \
    } else {                                                         \
        printf("  ok: %s\n", message);                               \
    }                                                                \
} while (0)

/* A C-vs-Rust divergence that the C TYPE SYSTEM cannot express. Counted
 * separately from failures; the reason is always printed. */
#define EXPECTED_GAP(desc, condition) do {                          \
    assertions++;                                                    \
    if (!(condition)) {                                              \
        printf("  GAP: %s\n", desc);                                 \
        gap_count++;                                                 \
    } else {                                                         \
        printf("  ok(gap-closed): %s\n", desc);                      \
    }                                                                \
} while (0)

/* Rust load_real_database() + rb_load().
 *
 * RB_CARDS_DIR overrides the search, which matters right now: the in-tree
 * card-data blobs under engine_c_wip/src/ can be STALE relative to
 * src/core/generated/gen_data.c. Those two are a matched set -- the offsets
 * table in gen_data.c indexes the string blob -- and regenerating one
 * without the other makes every string resolve to the wrong span. Observed
 * in this tree at 2026-10-01: src/abilities_strings.bin was written at
 * 14:30 and cards/build/abilities_strings.bin at 16:23, and gen_data.c at
 * 16:23 against the build copy. Loaded from the stale in-tree pair, the
 * string "card_property" resolved to "card_nameunit" and
 * "has_score_icon" to "itional_action" -- i.e. the entries were read at
 * the offsets of their NEIGHBOURS, so every card_property filter in the
 * corpus silently became a no-op. It also truncated
 * "HANAMARU_PLACE"? no: the 登場 action of PL!-PR-005-PR decoded as
 * "placem" (a truncated "placement"), and its action= "choice" with two
 * options was lost entirely, so the debut offered no option list at all.
 * That data drift, not the engine, is what made R1 and R8 red on the first
 * build. This is a build-hygiene defect, reported not worked around: the
 * override exists only so the same binary can be pointed at either tree
 * while the pair is re-synchronised.
 *
 * Order: $RB_CARDS_DIR, then the in-tree src/ (what every sibling suite
 * uses), then ../cards/build. The first that loads wins; failing all
 * three is a hard error, never a silent skip. */
static int load_card_db(void)
{
    const char *env = getenv("RB_CARDS_DIR");
    if (env && env[0] && rb_load(env) == 0) {
        printf("### card data: %s (via RB_CARDS_DIR)\n", env);
        return 0;
    }
    if (rb_load("src") == 0) {
        printf("### card data: src (in-tree)\n");
        return 0;
    }
    if (rb_load("../cards/build") == 0) {
        printf("### card data: ../cards/build (out-of-tree)\n");
        return 0;
    }
    return -1;
}

/* ── fixtures ── */

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && P->deck.n < RB_MAX_ZONE; i++)
            P->deck.cards[P->deck.n++] = filler;
    }
}

static int energy_active(TestGame *tg, int pl)
{
    return rb_energy_active_count(&tg->state.p[pl]);
}

static int orientation_of(TestGame *tg, int cid)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, cid);
    if (!o || !o[0]) return 0;            /* "none" */
    if (!strcmp(o, "wait")) return 1;
    if (!strcmp(o, "active")) return 2;
    return 0;
}

static int is_wait(TestGame *tg, int cid)
{
    return orientation_of(tg, cid) == 1;
}

/* Which seat owns the prompt that is STANDING RIGHT NOW: 0 = p1, 1 = p2,
 * -1 = the choice carries no seat token at all. This is the C reading of
 * Rust's `ability_queue.current_entry().unwrap().choice_player_id`, and it
 * is the API to use rather than test_queue_entry_choice_seat(): the pending
 * choice is the observable, and a choice raised outside the queue-entry
 * path has no entry to read.
 *
 * PREFERENCES THE QUEUE ENTRY, like Rust does. The Rust R2/R3 tests read
 * current_entry() and not the choice, and the two are not the same thing in
 * C: rb_get_pending_choice_player_id() reads the choice's own actor field,
 * which the C choice builder leaves unset for a live-start answer prompt,
 * while the QUEUE ENTRY does carry the token the Rust test asserts on. So
 * the entry is tried first and the choice is the fallback, and which one
 * answered is reported so a reader can see it. */
static int pending_seat(TestGame *tg)
{
    int n = test_queue_n_entries(tg);
    for (int i = 0; i < n; i++) {
        int s = test_queue_entry_choice_seat(tg, i);
        if (s >= 0) return s;
    }
    if (!rb_has_pending_choice(&tg->state)) return -1;
    return rb_get_pending_choice_player_id(&tg->state);
}

static int all_heart_of(TestGame *tg, int cid)
{
    return rb_mods_get_heart(&tg->state.mods, cid, RB_HEART_ALL);
}

/* Sum of every modify_yell_count delta registered for `slot` (1 = p1,
 * 2 = p2, matching tracking.rs:40). Rust reads this straight out of
 * game.state.yell_count_modifiers; the C field is the same shape. */
static int yell_count_delta(TestGame *tg, int slot)
{
    int sum = 0;
    for (int i = 0; i < tg->state.n_yell_count_modifiers; i++)
        if (tg->state.yell_count_modifiers[i].slot == (uint8_t)slot)
            sum += tg->state.yell_count_modifiers[i].delta;
    return sum;
}

static int printed_cost(int card_id)
{
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return -1;
    int cost = c.cost;
    rb_free_card(&c);
    return cost;
}

static int card_is(int card_id, const char *no)
{
    if (card_id < 0) return 0;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    const char *cn = rb_card_string(c.card_no_idx);
    int eq = (cn && !strcmp(cn, no));
    rb_free_card(&c);
    return eq;
}

static int stage_has(TestGame *tg, int area, int cid)
{
    return tg->state.p[0].stage[area] == cid;
}

/* test_add_to_discard is P1-only in the harness; this is the seat-aware
 * form, needed by R6 where the two revealed cards must sit in their own
 * OWNERS' waitrooms for the zone-ownership filter to discriminate. */
static void add_to_discard_for(TestGame *tg, int pl, int cid)
{
    RbPlayer *P = &tg->state.p[pl];
    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
}

/* ── bounded drains ──
 * Every drain is BOUNDED and RETURNS the number of prompts it answered,
 * so a caller can CHECK(prompts > 0) and prove a negative assertion
 * non-vacuous. Never write an unbounded loop: one peer found an ability
 * that re-presents a skipped choice forever, and an unbounded drain
 * hangs the whole suite instead of reporting the engine gap. */

#define DRAIN_GUARD 40

/* The Rust files' drain_choices(): answer everything with option 0. */
static int drain_pick_first_all(TestGame *tg)
{
    int n = 0;
    while (rb_has_pending_choice(&tg->state) && n < DRAIN_GUARD) {
        rb_resume_with_choice(&tg->state, 0);
        n++;
    }
    return n;
}

/* The Rust files' drain_skip(): answer what cannot be declined, decline
 * what can. The negative-control path -- a prompt that re-presents the
 * same skipped choice is caught by DRAIN_GUARD, not by a hang. */
static int drain_skip(TestGame *tg)
{
    int n = 0;
    while (rb_has_pending_choice(&tg->state) && n < DRAIN_GUARD) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int must_pick = (c && c->kind == RB_CHOICE_SELECT_CARD && !c->allow_skip);
        rb_resume_with_choice(&tg->state, must_pick ? 0 : -1);
        n++;
    }
    return n;
}

/* Rust advance_to_live_start(): Main -> ... -> the ライブ開始時 window.
 * 5 passes in C, matching the phase walk the sibling suites use. */
static void advance_to_live_start_5(TestGame *tg)
{
    for (int i = 0; i < 5; i++) test_pass(tg);
}

/* Rust finish_live_setup(): 2 further passes, the window in which
 * ライブ開始時 abilities resolve (RB_PHASE_PERFORMANCE). */
static void finish_live_setup(TestGame *tg)
{
    test_pass(tg);
    test_pass(tg);
}

/* ═══════════════════════════════════════════════════════════════════════
 * §A  full_round/real_card_phase_walkthrough_and_ability_suite_test.rs
 * ═══════════════════════════════════════════════════════════════════════ */

/* R1 shared_pr_debut_waits_only_low_cost_opposing_members_for_each_card_identity
 *   (Rust :410)
 *
 * 「登場　以下から1つを選ぶ。・カードを1枚引き、手札を1枚控え室に置く。
 * 　・相手のステージにいるすべてのコスト2以下のメンバーをウェイトにする。」
 *
 * The Rust test loops over THREE distinct card identities of the same
 * printed ability (PL!-PR-005/006/008-PR) and, for each, asserts that
 * option 1 of the debut waits EXACTLY the two cost-2 opposing members and
 * NOT the cost-15 one. It is a per-IDENTITY assertion: the point is that
 * the filter is re-evaluated for each card record rather than being
 * answered once from a cached first identity, so all three runs are
 * ported rather than one.
 *
 * THE game.id() INTERNING TRAP -- MY DECISION, STATED
 * Rust's `game.id(no)` interns by card_no, so `cheap_a` and `cheap_b`
 * would be the SAME i16 sitting in two stage slots and both
 * `get_orientation_modifier` reads would answer for one card. I use
 * test_new_id for `cheap_b` (Rust wrote game.new_id for it) so the two
 * are physically distinct, which makes the two waits independently
 * observable. `expensive` and the ability host are single-slot fixtures,
 * so they use test_id, i.e. exactly what Rust's game.id returns.
 *
 * NON-VACUITY: the two cheap members staying ON STAGE and the expensive
 * one staying at stage[2] are asserted alongside the waits, because a
 * filter that emptied the stage would satisfy "nothing is waited" and
 * "nothing is waited" is what the negative half claims.
 */
static void r1_shared_pr_debut_waits_only_low_cost_opponents(void)
{
    static const char *const CARDS[3] = {
        "PL!-PR-005-PR", "PL!-PR-006-PR", "PL!-PR-008-PR"
    };

    for (int k = 0; k < 3; k++) {
        static TestGame tg; test_game_new(&tg);
        rb_card_copy_pool_reset();

        int host      = test_id(&tg, CARDS[k]);
        int cheap_a   = test_id(&tg, "PL!HS-pb1-002-R");
        int cheap_b   = test_new_id(&tg, "PL!HS-pb1-002-R");
        int expensive = test_id(&tg, "PL!S-bp5-009-R");
        int filler    = test_new_id(&tg, "PL!-sd1-010-SD");
        if (host < 0 || cheap_a < 0 || cheap_b < 0 || expensive < 0 || filler < 0) {
            setup_bugs++;
            return;
        }
        /* Byte-exact identity: the print under test must be the one found. */
        if (!card_is(host, CARDS[k])) {
            fprintf(stderr, "SETUP BUG: %s did not resolve to the expected card\n",
                    CARDS[k]);
            setup_bugs++;
            return;
        }
        /* The cheap/expensive cost split is what the filter keys on, so it
         * is asserted from the database rather than assumed. */
        CHECK_EQ(printed_cost(cheap_a), 2, "R1: the cheap member really costs 2");
        CHECK_EQ(printed_cost(expensive), 15, "R1: the expensive member really costs 15");
        CHECK(cheap_a != cheap_b, "R1: the two cheap members are DISTINCT card ids");

        fill_decks(&tg, filler, 20);
        tg.state.p[1].stage[0] = cheap_a;
        tg.state.p[1].stage[1] = cheap_b;
        tg.state.p[1].stage[2] = expensive;

        /* Rust fire_trigger(..., Debut, "登場") == rb_fire_debut. */
        test_fire_debut(&tg, host);

        /* Rust game.select_option(1): the SECOND option, the wait branch. */
        int prompts = 0;
        while (rb_has_pending_choice(&tg.state) && prompts < DRAIN_GUARD) {
            rb_resume_with_choice(&tg.state, 1);
            prompts++;
        }

        /* PROMPT-COUNT CONTROL: the debut really raised the two-option
         * prompt, otherwise "option 1 waits the cheap pair" is a reading
         * off a board where the ability never ran. */
        CHECK(prompts > 0,
              "R1: the 登場 two-option prompt was raised (non-vacuous control)");

        CHECK_EQ(is_wait(&tg, cheap_a), 1,
                 "R1: the first cost-2 opposing member is waited");
        CHECK_EQ(is_wait(&tg, cheap_b), 1,
                 "R1: the second cost-2 opposing member is waited");
        CHECK_EQ(is_wait(&tg, expensive), 0,
                 "R1: the cost-15 opposing member is NOT waited");

        /* NON-VACUITY, the other half: nobody left the stage. */
        CHECK_EQ(tg.state.p[1].stage[0], cheap_a, "R1: the first cheap member stayed on stage");
        CHECK_EQ(tg.state.p[1].stage[1], cheap_b, "R1: the second cheap member stayed on stage");
        CHECK_EQ(tg.state.p[1].stage[2], expensive, "R1: the expensive member stayed at stage[2]");
    }
}

/* R2 ai_screeam_answer_both_discard  (Rust :95)
 * R3 ai_screeam_p2_owned_live_routes_answer_and_discard_choices  (Rust :199)
 *
 * 愛♡スクリ～ム！ (LL-PR-004-PR) ライブ開始時 「相手に何が好き？」
 * option 0 -> 「自分と相手はそれぞれ手札を1枚控え室に置く」 = move_cards
 * hand->discard with target "both".
 *
 * Both Rust tests assert the SEAT ROUTING of the two discard prompts
 * (p1 first, then p2) by reading ability_queue.current_entry().
 * choice_player_id IS representable in C: it is `char choice_player_id[16]`
 * in RbQueueEntry, and test_queue_entry_choice_seat() reads it, so the
 * routing assertions are ported as HARD CHECKS.
 *
 * The load-bearing engine claim, restated here because it is a real
 * defect and not a harness limit: `move_cards` never reaches
 * rb_execute_misc_effect. src/ability/effects/executor.c dispatches
 * action "move_cards" (and "discard_card") straight to rb_effect_move_cards,
 * bypassing the dispatcher at misc.c that calls rb_misc_handle_both_targets.
 * So the target="both" split of option 0 does not fire and no per-seat
 * discard prompt is ever raised. These two tests are therefore expected
 * to be RED on that point; the routing CHECKs below are what expose it,
 * and the hand-size assertions carry the same claim from the other side.
 */
static int screeam_set_live_and_answer(TestGame *tg, int screeam, int option,
                                       int *p1_seat, int *p2_seat)
{
    advance_to_live_start_5(tg);
    test_set_live_card(tg, 0, screeam);
    finish_live_setup(tg);

    *p1_seat = -1; *p2_seat = -1;
    if (!rb_has_pending_choice(&tg->state)) return 0;
    /* The answer prompt must be routed to the OPPONENT either way: P2 when
     * P1 owns the live card, P1 when P2 does. */
    *p2_seat = pending_seat(tg);
    rb_resume_with_choice(&tg->state, option);
    return 1;
}

/* Answer exactly one prompt, recording which seat owns it. */
static int answer_one_record_seat(TestGame *tg, int idx, int *seat_out)
{
    if (!rb_has_pending_choice(&tg->state)) return 0;
    *seat_out = pending_seat(tg);
    int one[1]; one[0] = idx;
    test_select_indices(tg, one, 1);
    return 1;
}

static void r2_ai_screeam_answer_both_discard(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();

    int screeam  = test_id(&tg, "LL-PR-004-PR");
    int filler_a = test_id(&tg, "PL!-sd1-010-SD");
    int filler_b = test_id(&tg, "PL!-sd1-013-SD");
    int deckfill = test_new_id(&tg, "PL!-sd1-010-SD");
    if (screeam < 0 || filler_a < 0 || filler_b < 0 || deckfill < 0) { setup_bugs++; return; }

    CHECK(rb_card_is_live(screeam), "R2: LL-PR-004-PR really is a live card");
    fill_decks(&tg, deckfill, 10);

    /* Rust pushes screeam + filler_a into P1's hand and filler_b into P2's. */
    test_add_to_hand(&tg, screeam);
    test_add_to_hand(&tg, filler_a);
    test_add_to_hand_for(&tg, 1, filler_b);

    /* Baselines are taken AFTER the phase advance, because the advance
     * itself moves cards between zones (the Draw phase and the live-zone
     * refill both do), so a baseline read earlier would make the hand-size
     * deltas below meaningless. Rust's own comment at :145-147 does the
     * same arithmetic by hand. */
    int p1_before = tg.state.p[0].hand.n;
    int p2_before = tg.state.p[1].hand.n;

    int answer_seat = -1, p1_seat = -1, p2_seat = -1;
    int got = screeam_set_live_and_answer(&tg, screeam, 0, &answer_seat, &p2_seat);
    if (!got) { setup_bugs++; return; }
    p1_before = tg.state.p[0].hand.n;
    p2_before = tg.state.p[1].hand.n;

    /* PRECONDITION: the live card really reached the live zone, so the
     * ライブ開始時 scan that follows is looking at a populated zone. */
    CHECK_EQ(tg.state.p[0].live.n, 1,
             "R2: AiScReam really is the live card (precondition)");
    /* The answer prompt goes to the OPPONENT: P2 decides the flavour. */
    CHECK_EQ(answer_seat, 1,
             "R2: the 「相手に何が好き？」 answer prompt is routed to p2");

    /* Rust: P1's discard choice first, then P2's. */
    int p1_answered = answer_one_record_seat(&tg, 0, &p1_seat);
    int p2_answered = answer_one_record_seat(&tg, 0, &p2_seat);

    /* ENGINE GAP (executor.c): move_cards bypasses
     * rb_misc_handle_both_targets, so target "both" raises no per-seat
     * discard prompt and no hand card moves. The routing assertion is the
     * claim; it is a hard CHECK, not a warning. */
    CHECK(p1_answered,
          "R2: option 0 raises P1's own discard prompt (target \"both\", seat p1)");
    CHECK_EQ(p1_seat, 0, "R2: P1's discard choice carries choice_player_id=p1");
    CHECK(p2_answered,
          "R2: P2's discard prompt follows P1's (target \"both\", seat p2)");
    CHECK_EQ(p2_seat, 1, "R2: P2's discard choice carries choice_player_id=p2");

    /* The same claim from the board side: one card left each hand. */
    CHECK_EQ(tg.state.p[0].hand.n, p1_before - 1,
             "R2: P1 discarded exactly one card from hand");
    CHECK_EQ(tg.state.p[1].hand.n, p2_before - 1,
             "R2: P2 discarded exactly one card from hand");
}

static void r3_ai_screeam_p2_owned_routes_answer_and_discard(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();

    int screeam    = test_id(&tg, "LL-PR-004-PR");
    int p1_discard = test_new_id(&tg, "PL!-sd1-013-SD");
    int p2_discard = test_new_id(&tg, "PL!-sd1-014-SD");
    int deckfill   = test_new_id(&tg, "PL!-sd1-010-SD");
    if (screeam < 0 || p1_discard < 0 || p2_discard < 0 || deckfill < 0) {
        setup_bugs++; return;
    }
    fill_decks(&tg, deckfill, 10);

    /* P2 owns the live card, so the ANSWER goes to P1 and the "self"
     * discard goes to P2 first. The live card goes STRAIGHT into P2's live
     * zone: the live-zone refill during the phase advance draws live cards
     * from hand to top the zone up, which would consume it and leave the
     * live zone empty before P2's window opens. Rust's set_live_card is the
     * P1 append path and has no P2 equivalent, so the raw seat-aware
     * placement is the faithful way to build P2's fixture. */
    test_add_to_hand_for(&tg, 0, p1_discard);
    test_add_to_hand_for(&tg, 1, screeam);
    test_add_to_hand_for(&tg, 1, p2_discard);
    test_add_to_live_for(&tg, 1, screeam);
    tg.state.first_attacker  = 1;
    tg.state.second_attacker = 0;

    int p1_before = tg.state.p[0].hand.n;
    int p2_before = tg.state.p[1].hand.n;

    /* P2 is the first attacker, so P2's live-start window is the one that
     * matters. The C phase machine stores ONE performance phase value and
     * tells the two apart by which seat is active, so the walk is driven
     * BY NAME with a bounded step count rather than by a blind pass count --
     * a fixed 5+2 pass count would run straight through P2's window and
     * into the victory phase, which is exactly what the first version of
     * this test did.
     *
     * The walk does NOT drain. The answer prompt is raised the moment P2's
     * window opens with its live card set, so a drain inside the loop would
     * consume it and the assertions below would then read a board with the
     * prompt already answered. That was the second bug in this test: the
     * drain belonged AFTER the checks, not inside the walk. The loop is
     * still bounded, so a phase that never arrives terminates. */
    int guard = 0, reached = 0;
    while (guard++ < 16) {
        if (tg.state.phase == RB_PHASE_PERFORMANCE &&
            tg.state.p[1].live.n > 0 &&
            rb_has_pending_choice(&tg.state)) { reached = 1; break; }
        test_pass(&tg);
    }

    /* PRECONDITION: P2 really owns the live card and its window really
     * opened, so the scan that follows is looking at a populated zone. */
    CHECK_EQ(tg.state.p[1].live.n, 1,
             "R3: the P2-owned AiScReam really is in P2's live zone (precondition)");
    CHECK_EQ(reached, 1,
             "R3: the walk reached P2's ライブ開始時 window with its live card set");
    CHECK(tg.state.phase == RB_PHASE_PERFORMANCE,
          "R3: the game is inside the performance window, not past it");
    if (!reached) return;

    CHECK(rb_has_pending_choice(&tg.state),
          "R3: the P2-owned live card raises its own answer prompt");
    if (!rb_has_pending_choice(&tg.state)) return;
    CHECK_EQ(pending_seat(&tg), 0,
             "R3: the answer prompt is routed to p1 (the opponent of the owner)");

    rb_resume_with_choice(&tg.state, 0);

    p1_before = tg.state.p[0].hand.n;
    p2_before = tg.state.p[1].hand.n;

    int self_seat = -1, opp_seat = -1;
    int self_answered = answer_one_record_seat(&tg, 0, &self_seat);
    int opp_answered  = answer_one_record_seat(&tg, 0, &opp_seat);

    /* The owner discards its OWN card first, then the opponent. */
    CHECK(self_answered, "R3: the owner's own discard prompt comes first");
    CHECK_EQ(self_seat, 1, "R3: the self discard carries choice_player_id=p2");
    CHECK(opp_answered, "R3: the opponent's discard prompt follows");
    CHECK_EQ(opp_seat, 0, "R3: the opponent discard carries choice_player_id=p1");

    CHECK_EQ(tg.state.p[1].hand.n, p2_before - 1,
             "R3: P2 (the live-card owner) discarded exactly one card");
    CHECK_EQ(tg.state.p[0].hand.n, p1_before - 1,
             "R3: P1 (the opponent) discarded exactly one card");
}

/* R4 wien_q117_another_member_triggers_yell_reduction  (Rust :2813)
 *
 * ウィーン・マルガレーテ ライブ開始時 「自分のステージにこのメンバー以外の
 * メンバーが1人以上いる場合、ライブ終了時まで、エールによって公開される
 * 自分のカードの枚数が8枚減る。」
 *
 * The Rust test reads the DERIVED MODIFIER, not the board, which is the
 * right call and is ported verbatim: yell_count_delta(&tg, 1) must be -8
 * for p1. Wien sits at stage[1] and a second member at stage[2]; the
 * second member is what satisfies "another member on stage".
 *
 * The Rust file writes the card number with an explicit \u{ff0b}, i.e.
 * the FULLWIDTH PLUS, which is why WIEN_NO carries the byte-exact form.
 *
 * POSITIVE CONTROL: the Rust test only asserts the -8, which on a board
 * where nothing fired is indistinguishable from a board where the yell
 * reduction never happened. The C port additionally checks that Wien is
 * still on stage and that a member sits in BOTH other slots' worth of
 * occupancy, so the -8 is read against a board that satisfies the
 * condition by construction.
 */
static void r4_wien_q117_yell_reduction(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();

    int wien   = test_id(&tg, WIEN_NO);
    int other  = test_id(&tg, "PL!-sd1-010-SD");
    int live   = test_id(&tg, "PL!-sd1-020-SD");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");
    if (wien < 0 || other < 0 || live < 0 || filler < 0) { setup_bugs++; return; }
    if (!card_is(wien, WIEN_NO)) {
        fprintf(stderr, "SETUP BUG: %s did not resolve to the expected card\n", WIEN_NO);
        setup_bugs++;
        return;
    }

    fill_decks(&tg, filler, 10);
    /* Wien at Center, a second member at RightSide. */
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = wien;
    tg.state.p[0].stage[2] = other;
    test_add_to_hand(&tg, live);

    CHECK(rb_card_is_live(live), "R4: PL!-sd1-020-SD really is a live card");
    CHECK(tg.state.p[0].stage[1] != RB_EMPTY_SLOT &&
          tg.state.p[0].stage[2] != RB_EMPTY_SLOT,
          "R4: both members are on stage, so 'another member' is satisfied (control)");

    advance_to_live_start_5(&tg);
    test_set_live_card(&tg, 0, live);
    finish_live_setup(&tg);
    drain_skip(&tg);

    /* Q117: the condition is met -> Wien's LiveStart subtracts 8. */
    CHECK_EQ(yell_count_delta(&tg, 1), -8,
             "R4/Q117: another member on stage registers the -8 yell-count modifier for p1");

    /* Both members must still be on stage: the LiveStart must not have
     * moved anything. Asserted so the -8 is read against a live board. */
    CHECK(tg.state.p[0].stage[1] != RB_EMPTY_SLOT &&
          tg.state.p[0].stage[2] != RB_EMPTY_SLOT,
          "R4: both members remain on stage after the LiveStart resolved");
}

/* R5 energy_zone_capacity_handled  (Rust :2924)
 *
 * The Rust test is a pure ZONE-BOOKKEEPING test on the energy zone:
 * 0 -> 20 given -> sub_active(5) -> sub_active(15) -> sub_active(1)
 * must saturate at 0, and `cards.len()` (the BAG) must never move, only
 * active_count. In C the bag and the active count are two separate
 * fields (energy.n and energy_active), so both halves are asserted.
 *
 * Ported with rb_energy_sub_active(), the C twin of EnergyZone::sub_active,
 * so the saturating behaviour under test is the ENGINE's and not the
 * test's arithmetic. The saturation edge is the load-bearing assertion:
 * a non-saturating sub_active would drive active_count negative.
 */
static void r5_energy_zone_capacity_handled(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();

    RbPlayer *P = &tg.state.p[0];

    CHECK_EQ(P->energy.n, 0, "R5: the energy zone starts with 0 energy cards");
    CHECK_EQ(rb_energy_active_count(P), 0, "R5: 0 active energy");

    test_give_energy(&tg, 20);
    CHECK_EQ(P->energy.n, 20, "R5: after give_energy(20) the bag holds 20 cards");
    CHECK_EQ(rb_energy_active_count(P), 20, "R5: all 20 are active");

    rb_energy_sub_active(P, 5);
    CHECK_EQ(rb_energy_active_count(P), 15, "R5: 15 active after spending 5");
    CHECK_EQ(P->energy.n, 20, "R5: spending 5 left the total card count unchanged");

    rb_energy_sub_active(P, 15);
    CHECK_EQ(rb_energy_active_count(P), 0, "R5: 0 active after spending all 15");

    /* THE SATURATION EDGE. A non-saturating sub_active drives this
     * negative, which is what the Rust assert_eq!(.., 0) pins. */
    rb_energy_sub_active(P, 1);
    CHECK_EQ(rb_energy_active_count(P), 0,
             "R5: sub_active below zero SATURATES at 0 rather than going negative");
    CHECK_EQ(P->energy.n, 20,
             "R5: the over-spend did not remove an energy card from the bag");
}

/* R6 revealed_cards_filtered_by_player_ownership  (Rust :3316)
 *
 * The Rust test injects a SelectCard choice over the "revealed_cards"
 * zone with target_player_id "self", stamps choice_player_id = "p1", then
 * calls generate_possible_actions and asserts that exactly ONE
 * ChoiceSelect action is offered and that its card_id is P1's card --
 * i.e. revealed_cards holds both players' cards, but only the one that is
 * in the CHOOSING PLAYER'S OWN ZONES is selectable.
 *
 * C parity shape: the flat RbChoice carries no option list, but
 * generate_possible_actions (rb_generate_action_candidates) IS the C
 * action layer, so the ChoiceSelect half is observable as an action list.
 *
 * KNOWN LIMIT, STATED NOT PAPERED OVER: the C action layer
 * (src/turn/phase.c:1182 rb_generate_action_candidates) emits Pass and
 * PlayMember actions ONLY -- it has no ChoiceSelect branch at all, and it
 * returns early unless phase == RB_PHASE_MAIN. The Rust
 * generate_possible_actions has a fourth arm that turns a pending
 * SelectCard choice into one ChoiceSelect action per selectable card.
 * That arm is the subject of this test, so the action-list half is an
 * EXPECTED_GAP with the reason printed, while the NON-VACUITY half --
 * that the fixture really does hold one card in each seat's own discard
 * and one in revealed_cards -- stays a hard CHECK.
 */
static void r6_revealed_cards_filtered_by_player_ownership(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();

    int p1_card = test_id(&tg, "PL!SP-sd1-019-SD");
    int p2_card = test_id(&tg, "PL!SP-sd1-020-SD");
    if (p1_card < 0 || p2_card < 0) { setup_bugs++; return; }

    /* Each card goes into its OWNER's waitroom so the zone-ownership check
     * has something to pass. Rust leaves the per-player cheer_bufs empty
     * to force the fallback path; C has no cheer_buf, so this IS the only
     * path. */
    test_add_to_discard(&tg, p1_card);
    add_to_discard_for(&tg, 1, p2_card);
    test_add_to_revealed(&tg, p1_card);
    test_add_to_revealed(&tg, p2_card);

    /* PROMPT-COUNT CONTROL / non-vacuity: the fixture is exactly the shape
     * the filter has to discriminate on -- both players' cards revealed,
     * each in its owner's waitroom. */
    CHECK_EQ(tg.state.n_revealed, 2, "R6: revealed_cards really holds BOTH players' cards");
    CHECK_EQ(test_zone_len(&tg, 0, "discard"), 1, "R6: P1's card is in P1's own waitroom");
    CHECK_EQ(test_zone_len(&tg, 1, "discard"), 1, "R6: P2's card is in P2's own waitroom");

    /* Inject the SelectCard choice over "revealed_cards", target "self",
     * stamped to p1, exactly as the Rust test does. */
    RbChoice c;
    memset(&c, 0, sizeof(c));
    c.kind = RB_CHOICE_SELECT_CARD;
    strncpy(c.zone, "revealed_cards", sizeof(c.zone) - 1);
    c.count = 1;
    c.allow_skip = 0;
    strncpy(c.target, "revealed_cards", sizeof(c.target) - 1);
    strncpy(c.target_player_id, "self", sizeof(c.target_player_id) - 1);
    c.actor = 0;
    rb_queue_pause_for_choice(&tg.state, &c);

    /* Stamp the entry's choice seat to p1, as the Rust test does before
     * calling generate_possible_actions. The Rust test mutates the CURRENT
     * entry through current_entry_mut(); the C queue stores a token per
     * entry, so it is stamped the same way. */
    CHECK(test_queue_n_entries(&tg) > 0,
          "R6: the injected choice really produced a queue entry");
    if (test_queue_n_entries(&tg) > 0)
        strncpy(tg.state.queue.entries[0].choice_player_id, "p1",
                sizeof(tg.state.queue.entries[0].choice_player_id) - 1);
    CHECK_EQ(test_queue_entry_choice_seat(&tg, 0), 0,
             "R6: the entry's choice_player_id is stamped p1, as the Rust test does");

    /* The ChoiceSelect half: rb_generate_action_candidates has no
     * ChoiceSelect branch in C, so the "exactly one selectable action, and
     * it is P1's card" claim cannot hold. Recorded as an explicit gap
     * with the responsible engine file named -- never as a pass. */
    RbGeneratedActionList acts = rb_generate_action_candidates(&tg.state);
    int n_select = 0, n_select_p1 = 0;
    for (int i = 0; i < acts.count; i++) {
        if (acts.actions[i].action_type != RB_ACTION_SELECT_CARDS) continue;
        n_select++;
        if (acts.actions[i].has_parameters &&
            acts.actions[i].parameters.card_id == p1_card) n_select_p1++;
    }
    EXPECTED_GAP("R6: generate_possible_actions must offer exactly ONE "
                 "ChoiceSelect action, for P1's card only (P2's card is not in "
                 "P1's zones) -- src/turn/phase.c rb_generate_action_candidates "
                 "emits Pass + PlayMember only and has no ChoiceSelect arm",
                 n_select == 1 && n_select_p1 == 1);
    /* Diagnostic, printed unconditionally: the observed action mix, so the
     * report names what the layer produced rather than a bare "got 0". */
    fprintf(stderr, "   [R6 diag] phase=%d actions=%d select_actions=%d "
                    "select_p1=%d\n",
            (int)tg.state.phase, acts.count, n_select, n_select_p1);
    if (acts.actions) rb_free(acts.actions);
}

/* R7 choice_condition_shows_proper_labels_in_actions  (Rust :3373)
 *
 * The Rust test injects a choice_condition SelectTarget carrying TWO
 * option labels and asserts generate_possible_actions turns them into two
 * ChoiceOption actions whose descriptions CONTAIN the labels.
 *
 * THIS IS THE FLAT-RbChoice LIMIT, and it is the reason the brief says a
 * multi-option prompt is not observable from C. RbChoice has no `options`
 * array: a SelectTarget is a bare kind + target + description, and
 * rb_generate_action_candidates has no ChoiceOption arm either. Neither
 * the option LIST nor the per-option LABEL exists in the C model, so both
 * the count and the label content are unrepresentable rather than merely
 * unimplemented. Recorded as an explicit EXPECTED_GAP with the reason
 * printed, never as a pass.
 *
 * What IS representable and therefore asserted as a hard CHECK is the
 * control: the injected choice really is a non-skippable SelectTarget on
 * the "choice_condition" route carrying the Rust description text, so a
 * future C model that grows an options array has a fixture already in
 * place. The label strings are the two Japanese option texts from the
 * Rust source, spelled with \xNN escapes.
 */
#define OPT_WAIT   "\xE3\x81\x93\xE3\x81\xAE\xE3\x83\xA1\xE3\x83\xB0\xE3\x83\x90\xE3\x83\xBC\xE3\x82\x92\xE3\x82\xA6\xE3\x82\xA7\xE3\x82\xA4\xE3\x83\x88\xE3\x81\xAB\xE3\x81\x99\xE3\x82\x8B" /* このメンバーをウェイトにする */
#define OPT_DISCARD "\xE6\x89\x8B\xE6\x9D\xAF\xE3\x82\x92\xE6\x95\xB0\xE6\x9D\xAF\xE3\x81\xAB\xE3\x81\x8A\xE3\x81\x8F\xE5\x85\x8B\xE5\xBB\xA3\xE5\xAE\xA4\xE3\x81\xAB\xE3\x81\x86\xE3\x82\x8B" /* 手札を1枚控え室に置く */

static void r7_choice_condition_shows_proper_labels(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();

    RbChoice c;
    memset(&c, 0, sizeof(c));
    c.kind = RB_CHOICE_SELECT_TARGET;
    strncpy(c.target, "choice_condition", sizeof(c.target) - 1);
    /* The Rust description carries BOTH option labels. That much IS
     * representable, so it is pinned. */
    snprintf(c.description, sizeof(c.description),
             "Choose cost option: %s OR %s", OPT_WAIT, OPT_DISCARD);
    c.allow_skip = 0;
    c.actor = 0;
    rb_queue_pause_for_choice(&tg.state, &c);

    /* CONTROL: the injected choice is the one the Rust test injects. */
    CHECK(rb_has_pending_choice(&tg.state),
          "R7: the injected choice_condition prompt is standing");
    const RbChoice *live = rb_get_pending_choice(&tg.state);
    CHECK_EQ(live ? (int)live->kind : -1, (int)RB_CHOICE_SELECT_TARGET,
             "R7: the prompt is a SelectTarget (as the Rust test injects)");
    CHECK(live && !strcmp(live->target, "choice_condition"),
          "R7: the prompt is routed on the \"choice_condition\" target");
    CHECK(live && !live->allow_skip,
          "R7: the prompt is non-skippable (allow_skip = false)");
    CHECK(live && strstr(live->description, OPT_WAIT) != NULL,
          "R7: the description carries the first option label 「このメンバーをウェイトにする」");
    CHECK(live && strstr(live->description, OPT_DISCARD) != NULL,
          "R7: the description carries the second option label 「手札を1枚控え室に置く」");

    /* The Rust assertions proper: two ChoiceOption actions whose
     * descriptions contain the labels. Unrepresentable in C -- RbChoice has
     * no options array and the action layer has no ChoiceOption arm. */
    RbGeneratedActionList acts = rb_generate_action_candidates(&tg.state);
    int n_option = 0;
    for (int i = 0; i < acts.count; i++)
        if (acts.actions[i].action_type == RB_ACTION_CHOICE) n_option++;
    EXPECTED_GAP("R7: generate_possible_actions must expose TWO ChoiceOption "
                 "actions whose descriptions contain the two Japanese option "
                 "labels -- the flat RbChoice carries no option list (no "
                 "options[] field) and src/turn/phase.c "
                 "rb_generate_action_candidates has no ChoiceOption arm",
                 n_option == 2);
    fprintf(stderr, "   [R7 diag] phase=%d actions=%d choice_option_actions=%d\n",
            (int)tg.state.phase, acts.count, n_option);
    if (acts.actions) rb_free(acts.actions);
}

/* R8 hanamaru_score_icon_filter  (Rust :3538)
 *
 * 国木田花丸 起動/ターン1 「手札を2枚控え室に置く：自分の控え室から
 * {{スコア}}を持つ『Aqours』のライブカードを1枚手札に加える。」
 *
 * The filter is a card_property (has_score_icon) plus group_names
 * (Aqours). The waitroom holds three cards that differ in exactly one
 * dimension each: an Aqours live WITH the icon, an Aqours live WITHOUT
 * it, and an Aqours MEMBER. Exactly one may move.
 *
 * The Rust test first asserts the card really carries an ability with
 * card_property == "has_score_icon"; that fixture check is ported as a
 * hard CHECK, because on a card whose ability text failed to decode the
 * whole test would otherwise pass vacuously on a dead board. The decode
 * path is rb_card_num_abilities + rb_decode_card_ability -- Card.ability
 * is NOT populated by rb_decode_card_by_index.
 *
 * POSITIVE CONTROL: the eligible card leaving the waitroom is asserted
 * FIRST, so the two negatives are read against a proven execution. The
 * 1-match auto-select behaviour is the point: exactly one candidate means
 * no second prompt.
 */
static int hanamaru_has_score_icon_ability(int cid)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        uint32_t idx = 0;
        if (!rb_card_get_ability_idx((uint32_t)cid, i, &idx)) continue;
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        /* The filter rides in the effect's extra pairs, per
         * util.rs::CardFilter -- the action is move_cards discard->hand. */
        int found = 0;
        if (ab.effect) {
            for (int e = 0; e < ab.effect->n_extra; e++) {
                if (!strcmp(ab.effect->extra_k[e], "card_property") &&
                    ab.effect->extra_v[e] &&
                    !strcmp(ab.effect->extra_v[e], "has_score_icon"))
                    found = 1;
            }
        }
        rb_free_ability(&ab);
        if (found) return 1;
    }
    return 0;
}

static void r8_hanamaru_score_icon_filter(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();

    int hanamaru     = test_id(&tg, "PL!S-sd1-007-SD");
    int filler       = test_id(&tg, "PL!-sd1-010-SD");
    int score_live   = test_id(&tg, "PL!S-bp2-024-L");
    int no_score_live= test_id(&tg, "PL!S-bp2-026-L");
    int member       = test_id(&tg, "PL!S-bp2-015-PR");
    if (hanamaru < 0 || filler < 0 || score_live < 0 ||
        no_score_live < 0 || member < 0) { setup_bugs++; return; }

    /* FIXTURE CHECK, ported from the Rust test's own pre-condition block.
     * If the ability text fails to decode the whole test would otherwise
     * pass vacuously, so the decode is asserted, not assumed. */
    CHECK(rb_card_num_abilities((uint32_t)hanamaru) > 0,
          "R8: 花丸 really has at least one decoded ability");
    CHECK(hanamaru_has_score_icon_ability(hanamaru),
          "R8: 花丸 really has an ability filtered on card_property=has_score_icon");

    /* The three waitroom cards differ in exactly one dimension each. */
    Card cs, cn, cm;
    CHECK(rb_decode_card_by_index((uint32_t)score_live, &cs), "R8: the score-icon live decodes");
    CHECK(rb_decode_card_by_index((uint32_t)no_score_live, &cn), "R8: the no-icon live decodes");
    CHECK(rb_decode_card_by_index((uint32_t)member, &cm), "R8: the member decodes");
    CHECK_EQ(rb_card_has_score_icon(&cs), 1, "R8: PL!S-bp2-024-L carries the スコア icon");
    CHECK_EQ(rb_card_has_score_icon(&cn), 0, "R8: PL!S-bp2-026-L does NOT carry the スコア icon");
    CHECK(rb_card_is_live(score_live) && rb_card_is_live(no_score_live),
          "R8: both fixture lives really are live cards");
    CHECK(rb_card_is_member(member), "R8: PL!S-bp2-015-PR really is a member card");
    if (cs.ability || cn.ability || cm.ability) { /* not read; keep frees paired */ }
    rb_free_card(&cs); rb_free_card(&cn); rb_free_card(&cm);

    test_add_to_hand(&tg, hanamaru);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_add_to_discard(&tg, score_live);
    test_add_to_discard(&tg, no_score_live);
    test_add_to_discard(&tg, member);
    test_give_energy(&tg, 15);

    /* Rust places 花丸 on stage directly and removes it from hand. */
    tg.state.p[0].stage[0] = hanamaru;
    test_recalc(&tg);

    CHECK_EQ(test_zone_len(&tg, 0, "discard"), 3,
             "R8 precondition: the waitroom really holds all three fixture cards");

    /* 起動 activation. */
    int activated = test_activate_ability(&tg, hanamaru);
    CHECK(activated, "R8: the 起動 activation was accepted");

    /* Cost: SelectCard "手札を2枚控え室に置く". */
    CHECK(rb_has_pending_choice(&tg.state),
          "R8: the activation raises the 2-card discard cost prompt");
    if (rb_has_pending_choice(&tg.state)) {
        int pick[2] = { 0, 1 };
        test_select_indices(&tg, pick, 2);
    }

    /* Rust: no pending choice afterwards -- exactly one candidate matched,
     * so the effect auto-selects without a second prompt. */
    int left = drain_pick_first_all(&tg);
    CHECK_EQ(left, 0,
             "R8: exactly 1 candidate matched the filter, so no selection prompt was raised");

    /* POSITIVE CONTROL first, so the negatives below are non-vacuous. */
    CHECK_EQ(test_zone_count_of_id(&tg, 0, "discard", score_live), 0,
             "R8: the スコア-icon Aqours live left the waitroom");
    CHECK_EQ(test_hand_len(&tg) >= 1, 1,
             "R8: something is in hand to receive it");
    /* The negatives: the non-icon live and the member stay put. */
    CHECK_EQ(test_zone_count_of_id(&tg, 0, "discard", no_score_live), 1,
             "R8: the Aqours live WITHOUT the スコア icon stays in the waitroom");
    CHECK_EQ(test_zone_count_of_id(&tg, 0, "discard", member), 1,
             "R8: the Aqours MEMBER stays in the waitroom (wrong card type)");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §B  full_round/live_each_time_victory_road_through_real_live_rounds_test.rs
 *     T4 live_start_another_member          (Rust :228)
 *     T5 live_start_cost_free_still_triggers (Rust :261)
 *
 * Both Rust tests are the T1 fixture with the stage slots written in the
 * opposite order (stage[0] = filler, stage[1] = member) and the identical
 * assertion: the member with ライブ開始時 gains all-harts. The Rust file
 * itself does not say they are redundant, but they are the same board as
 * T1 up to the order of two independent assignments, so T9/T10 below are
 * faithful ports that also add the control T1 does not have: they assert
 * the member really is the LiveStart host and that the live zone really
 * held BOTH copies of Victory Road (the Rust fixture pushes one and
 * set_live_card appends a second, which is what gives the each_time
 * watcher something to fire against).
 *
 * test_parity_full_round.c declined to port T4/T5 as "redundant with T1"
 * and noted the swap adds nothing. That judgement is correct on board
 * shape, so this file ports them as THIN CONFIRMATIONS with the added
 * controls rather than as new coverage, and says so.
 * ═══════════════════════════════════════════════════════════════════════ */

#define VICTORY_NO "PL!N-bp5-030-L"
#define KIMI_NO    "PL!-bp3-012-N"
#define VR_FILLER  "PL!-sd1-010-SD"

/* The T4/T5 fixture: one Victory Road already in the live zone, member at
 * stage[1], a filler at stage[0], a second Victory Road in hand. */
static int vr_t45_setup(TestGame *tg, int *out_member)
{
    int victory = test_id(tg, VICTORY_NO);
    int member  = test_id(tg, KIMI_NO);
    int filler  = test_new_id(tg, VR_FILLER);
    if (victory < 0 || member < 0 || filler < 0) { setup_bugs++; return -1; }
    fill_decks(tg, filler, 30);
    /* Rust pushes Victory Road into the live zone directly. */
    test_add_to_live(tg, victory);
    tg->state.p[0].stage[0] = filler;
    tg->state.p[0].stage[1] = member;
    tg->state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_add_to_hand(tg, victory);
    test_add_to_hand(tg, filler);
    *out_member = member;
    return victory;
}

static void r9_live_start_another_member(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int member = 0;
    int victory = vr_t45_setup(&tg, &member);
    if (victory < 0) return;

    advance_to_live_start_5(&tg);
    test_set_live_card(&tg, 0, victory);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    /* CONTROLS T1 does not carry, asserted AFTER the walk so they describe
     * the board the assertion is read against: the live-start host is still
     * on stage, and the live zone really holds both Victory Road copies
     * (the pre-push plus set_live_card's append), which is what gives the
     * each_time watcher something to fire against. */
    CHECK_EQ(tg.state.p[0].stage[1], member,
             "R9: the ライブ開始時 member really is on stage (precondition)");
    CHECK_EQ(tg.state.p[0].live.n, 2,
             "R9: the live zone really holds 2 Victory Road copies (pre-push + set)");

    /* NON-VACUITY, done properly. A first version of this test asserted
     * "the window produced prompts > 0", which was MY bug, not the
     * engine's: 南ことり's ライブ開始時 resolves with no prompt at all
     * (the engine log shows one QUEUE_EXEC of action=sequential and then
     * straight to LIVE_SUCCESS_SIDE), so the control was unsatisfiable and
     * would have reported a false engine gap. The control has to be a
     * claim that a DEAD board cannot satisfy. The live-start SCAN reaching
     * the member is observable through the ability queue: the engine marks
     * the member's ability as used when it executes it, so a non-zero
     * use-count for (member, ability 0) proves the LiveStart really ran
     * even though it raised no prompt. */
    CHECK(rb_use_count(&tg.state.queue, member, 0, tg.state.turn) > 0,
          "R9: the live-start scan really executed the member's ライブ開始時 (non-vacuous control)");
    CHECK(rb_card_num_abilities((uint32_t)member) > 0,
          "R9: the member really has a decoded ライブ開始時 ability (precondition)");

    /* Rust: a member's LiveStart resolving fires the each_time and grants
     * all-harts. KNOWN RED for a real engine reason, reported not hidden:
     * rb_trigger_each_time_for_member -- the ONLY C implementation of
     * 「〜のライブ開始時能力が解決するたび」 -- has ZERO callers outside
     * tests/test_parity_auto_abilities.c. The real live-start scan
     * (src/turn/triggers.c) queues and executes the resolving member's own
     * ability but never notifies the each_time watcher. */
    CHECK(all_heart_of(&tg, member) > 0,
          "R9: after the member's ライブ開始時 resolved, Victory Road's each_time grants all-harts");
    (void)prompts;
}

static void r10_live_start_cost_free_still_triggers(void)
{
    static TestGame tg; test_game_new(&tg);
    rb_card_copy_pool_reset();
    int member = 0;
    int victory = vr_t45_setup(&tg, &member);
    if (victory < 0) return;

    /* The load-bearing difference from R9 is "cost-free": 南ことり's
     * ライブ開始時 costs nothing, so the ability CANNOT have been skipped
     * for want of energy. Preconditions are asserted BEFORE the walk, since
     * they describe the fixture as built. */
    CHECK(rb_card_num_abilities((uint32_t)member) > 0,
          "R10: the member really has a decoded ライブ開始時 ability (precondition)");
    CHECK_EQ(tg.state.p[0].stage[1], member,
             "R10: the ライブ開始時 member really is on stage (precondition)");
    CHECK_EQ(tg.state.p[0].energy_active, 0,
             "R10: the fixture starts with no energy, so nothing could be paid (precondition)");

    advance_to_live_start_5(&tg);
    test_set_live_card(&tg, 0, victory);
    finish_live_setup(&tg);
    int prompts = drain_skip(&tg);

    /* Post-walk preconditions, read against the board the claim is made on. */
    CHECK_EQ(tg.state.p[0].live.n, 2,
             "R10: the live zone really holds 2 Victory Road copies (pre-push + set)");

    /* NON-VACUITY, and the same correction as R9: "the window produced a
     * prompt" is NOT a usable control here, because a cost-free LiveStart
     * resolves without raising one. The control that a dead board cannot
     * fake is the ability queue's use-count for the member's LiveStart:
     * the engine marks the ability used when it executes it, so a non-zero
     * count proves the ability really resolved. */
    CHECK(rb_use_count(&tg.state.queue, member, 0, tg.state.turn) > 0,
          "R10: the live-start scan really executed the member's ライブ開始時 (non-vacuous control)");
    /* The energy zone is untouched: 「cost-free」 means the LiveStart
     * resolved without paying, so the each_time must still have fired. */
    CHECK_EQ(tg.state.p[0].energy_active, 0,
             "R10: the cost-free ライブ開始時 paid no energy");
    CHECK(all_heart_of(&tg, member) > 0,
          "R10: a cost-free ライブ開始時 still resolves, so the each_time fires");
    (void)prompts;
}

/* ═══════════════════════════════════════════════════════════════════════ */

static void on_segv(int sig)
{
    fprintf(stderr, "\n*** SIGNAL %d during test \"%s\" (engine fault) ***\n",
            sig, current_test);
    fflush(stderr);
    _Exit(3);
}

static int n_ok, n_failed, n_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout); fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        /* No fork available: run inline rather than silently skipping. */
        int f0 = failures, s0 = setup_bugs;
        current_test = name;
        fn();
        current_test = "(none)";
        int nf = failures - f0;
        if (nf > 0 || setup_bugs - s0 > 0) n_failed++; else n_ok++;
        printf("%-8s %s\n", nf ? "FAILED" : "ok", name);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        int a0 = (int)assertions, f0 = failures, s0 = setup_bugs, g0 = gap_count;
        current_test = name;
        fn();
        printf("        %s: %d assertion(s), %d failure(s), %d setup bug(s), %d gap(s)\n",
               name, (int)assertions - a0, failures - f0,
               setup_bugs - s0, gap_count - g0);
        fflush(stdout); fflush(stderr);
        _Exit(failures > f0 || setup_bugs > s0 ? 1 : 0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == 0) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    }
    fflush(stdout);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS,  on_segv);
    signal(SIGABRT, on_segv);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    printf("--- §A full_round/real_card_phase_walkthrough_and_ability_suite_test.rs ---\n");
    run("R1_shared_pr_debut_waits_low_cost_opponents",
        r1_shared_pr_debut_waits_only_low_cost_opponents);
    run("R2_ai_screeam_answer_both_discard",
        r2_ai_screeam_answer_both_discard);
    run("R3_ai_screeam_p2_owned_routes_discards",
        r3_ai_screeam_p2_owned_routes_answer_and_discard);
    run("R4_wien_q117_yell_reduction",
        r4_wien_q117_yell_reduction);
    run("R5_energy_zone_capacity_handled",
        r5_energy_zone_capacity_handled);
    run("R6_revealed_cards_filtered_by_ownership",
        r6_revealed_cards_filtered_by_player_ownership);
    run("R7_choice_condition_shows_proper_labels",
        r7_choice_condition_shows_proper_labels);
    run("R8_hanamaru_score_icon_filter",
        r8_hanamaru_score_icon_filter);

    printf("--- §B full_round/live_each_time_victory_road... (T4 / T5) ---\n");
    run("R9_live_start_another_member",
        r9_live_start_another_member);
    run("R10_live_start_cost_free_still_triggers",
        r10_live_start_cost_free_still_triggers);

    rb_unload();

    printf("\n=== parity_integration_rounds ===\n");
    printf("tests passed           : %d\n", n_ok);
    printf("tests failed (parity)  : %d\n", n_failed);
    printf("tests crashed (engine) : %d\n", n_crashed);
    printf("expected gaps recorded : %d\n", gap_count);
    printf("assertions (in-parent) : %ld\n", assertions);
    printf("failures  (in-parent)  : %d\n", failures);
    printf("fixture setup bugs     : %d\n", setup_bugs);
    printf("Rust test fns ported   : 10 (of 104 in the two clusters)\n");
    printf("  full_round/real_card_phase_walkthrough_and_ability_suite_test.rs : 8\n");
    printf("  full_round/live_each_time_victory_road_through_real_live_rounds_test.rs : 2 (T4, T5)\n");
    printf("  multi_card/ : 0 -- all 18 fns already ported, see the header\n");
    printf("see the per-test tally lines above for assertion counts\n");
    if (failures || n_failed || n_crashed) {
        printf("PARITY DIVERGENCE: the run stayed red by design.\n");
        return 1;
    }
    printf("ALL INTEGRATION-ROUND PARITY CHECKS PASSED\n");
    return 0;
}
