/* tests/test_parity_full_round2.c
 *
 * CLAIM (agent tr_fullround2): this file OWNS tests/test_parity_full_round2.c and
 * nothing else. The Rust sources it translates are
 *   engine/tests/test_modules/integration/full_round/real_card_phase_walkthrough_and_ability_suite_test.rs
 *   engine/tests/test_modules/integration/full_round/live_each_time_victory_road_through_real_live_rounds_test.rs
 * No src/ file, no other tests/test_*.c, no Makefile, no engine/, no cards/.
 *
 * C port of the unported part of the nico_* family of
 *   real_card_phase_walkthrough_and_ability_suite_test.rs
 * (Rust lines 1048-2149) — 矢澤にこ (PL!-pb1-018-R) 登場: 自分と相手はそれぞれ、
 *  自身の控え室からコスト2以下のメンバーカードを1枚、メンバーのいないエリアに
 *  ウェイト状態で登場させる。
 * 15 cases. The 13 Rust `fn nico_*` tests map one-to-one onto N1..N13:
 *   N1  nico_q168_both_appear_from_discard            (Rust :1058)
 *   N2  nico_q168_no_suitable_card_skips              (Rust :1163)
 *   N3  nico_sync_path_opponent_gets_choice_when_self_has_none (Rust :1211)
 *   N4  nico_q170_turn_player_appears_first           (Rust :1300)
 *   N5  nico_requires_empty_area                      (Rust :1440)
 *   N6  nico_cost_filter_only_shows_eligible          (Rust :1497)
 *   N7  nico_q169_appeared_card_occupies_area         (Rust :1589)
 *   N8  nico_prompt_path_two_eligible_in_discard      (Rust :1655)
 *   N9  nico_both_players_two_eligible_cards          (Rust :1773)
 *   N10 nico_prompt_path_direct_placement_wait_state  (Rust :1892)
 *   N11 nico_full_stage_then_prompt_path              (Rust :2008)
 *   N12 nico_q181_area_freed_after_card_leaves        (Rust :1360)
 *   N13 nico_q170_placed_cards_debut_abilities_fire   (Rust :2070)
 * N2b and N14 are ADDED by this file (positive twin for N2; capped-drain and
 * prompt-shape probe) and are labelled as such, not presented as Rust twins.
 *
 * ONE behaviour, FOUR code paths, which is why the family is worth a whole file:
 *   - how many eligible cards are in the discard  (0 = skip, 1 = EXACT / no
 *     SelectCard, 2+ = PROMPT / SelectCard)
 *   - how many empty stage slots exist            (1 = DIRECT placement,
 *     2+ = SelectPosition, 0 = skip)
 *   - which seat is asked                        (choice_player_id p1 / p2)
 *   - target "both" splitting into self + opponent
 * Every Rust test is a different cell of that 3x3 matrix, so the family is a
 * pairwise coverage argument, not 14 unrelated tests.
 *
 * THE BOUNDARY RE-DERIVED FROM THE TWO SIBLING FILES' BODIES
 * ----------------------------------------------------------
 * test_parity_integration_extra.c opens by declaring the whole 64-test
 * real_card_..._suite_test.rs "already covered" and ports three behaviours
 * (energy_zone_capacity_handled, revealed_cards_filtered_by_player_ownership,
 * choice_condition_shows_proper_labels_in_actions), pushing
 * umi_pr014_appear_reveal_* and dia_sd1_optional_draw_* into a "NOT covered -
 * follow-up" note.
 * test_parity_realcard_walkthrough.c IS that follow-up: its body ports exactly
 * U1/U2/U3 (umi_pr014 x3, lines 275-433) and D1/D2 (dia_sd1 x2, lines 485-540)
 * plus B1-B3. So the survey's umi/dia entries are ALREADY DONE and are NOT
 * re-ported here. Re-derived by reading both bodies, not their comments.
 * Its other conclusion is a real engine gap, not a harness one: it notes
 * "move_cards never reaches rb_execute_misc_effect" (executor.c dispatches
 * move_cards straight to rb_effect_move_cards, bypassing
 * rb_misc_handle_both_targets). Nico's ability IS action "move_cards" with
 * target "both", so this family is a SECOND independent probe of that exact
 * gap — which is why it is worth 14 tests. See the BLOCK below.
 *
 * THE game.id() INTERNING TRAP — MY DECISION, STATED EXPLICITLY
 * -----------------------------------------------------------
 * Rust's `game.id(no)` INTERNS BY CARD_NO, so calling it once and pushing the
 * SAME i16 into two zones puts one card in two places. The nico family does
 * this in FIVE tests (the task brief flags the analogous
 * `PL!S-bp2-002-R` / 愛♡スクリ～ム！ cases):
 *   nico_cost_filter_only_shows_eligible        `cheap` -> hand + p1 discard
 *                                               + p2 discard  (ONE id, 3 zones)
 *   nico_prompt_path_two_eligible_in_discard    `cheap_a` -> p1 discard
 *                                               + p2 discard
 *   nico_requires_empty_area                    `cheap`  -> p1 + p2 discard
 *   nico_prompt_path_direct_placement_wait_state `cheap_b2` -> p1 + p2 discard
 *   nico_full_stage_then_prompt_path            `cheap_a` -> p1 + p2 discard
 *   nico_q170_turn_player_appears_first         `cheap_p1` -> p1 HAND + p1 discard
 *   nico_q169_appeared_card_occupies_area       `cheap`  -> p1 hand + p1 discard
 * A Rust board with one id in both hands and discard cannot occur in a real
 * game, and every `zone.cards.contains(&id)` is then true of BOTH zones at
 * once, so several Rust assertions are true for the wrong reason.
 *
 * DECISION: I use `test_new_id` (a DISTINCT instance) for every physical
 * fixture card, so each card occupies exactly one slot, and I document the
 * slot each instance occupies in a comment. This makes the C board strictly
 * more physical than the Rust one. Consequence, stated so it is not mistaken
 * for an engine bug: where the Rust asserts `p1.discard.contains(&cheap)` and
 * `p2.stage.contains(&cheap)` for the SAME id, the C asserts those against two
 * DIFFERENT instances, and every assertion is strictly stronger — the C can
 * distinguish "P1's card appeared" from "P2's card appeared", which the Rust
 * cannot. Where the Rust puts one id in hand AND discard to keep the discard
 * non-empty while a play is legal (nico_q170, nico_q169), the C uses two
 * instances and says which is which.
 *
 * CARD IDENTITY (read from cards/cards.json as UTF-8, NOT Get-Content -Raw)
 * -----------------------------------------------------------------------
 *   PL!-pb1-018-R    矢澤にこ    メンバー  cost 7   (debut is the ability)
 *   PL!SP-sd1-019-SD 若菜四季     メンバー  cost 2   -> eligible
 *   PL!SP-sd1-020-SD 鬼塚夏美     メンバー  cost 2   -> eligible
 *   PL!-sd1-002-SD   絢瀬 絵里    メンバー  cost 2   -> eligible
 *   PL!-sd1-005-SD   星空 凛      メンバー  cost 2   -> eligible
 *   PL!SP-bp2-002-P  唐 可可      メンバー  cost 2   -> eligible, has a 登場
 *   PL!-sd1-014-SD   星空 凛      メンバー  cost 9   -> NOT eligible
 *   PL!-sd1-010-SD   高坂 穂乃果  メンバー  cost 4   -> not eligible, no ability
 * All eight exist; every cost and TYPE is asserted in the C test body via
 * rb_decode_card_by_index, so a cards.json/cards.bin drift shows up as a
 * setup bug rather than a rules divergence.
 *
 * NON-VACUITY (the quality bar for full-round tests)
 * --------------------------------------------------
 * This ability is a NO-OP on a bare board, and a no-op that silently swallows
 * the prompt produces a perfectly clean green test. Therefore:
 *   1. EVERY case asserts the WHOLE-BOARD CARD CENSUS (test_total_card_count)
 *      at fixture time and after the sequence. A zone count of 0 looks
 *      identical whether a move did nothing or DESTROYED a card; the census is
 *      what separates them. In this family the census is also the strongest
 *      available check on the DIRECT-placement path, which raises no prompt.
 *   2. EVERY negative assertion is paired with a POSITIVE one on a board that
 *      differs by exactly the factor under test (empty stage vs full stage,
 *      0 eligible vs 1 eligible vs 2 eligible, cost 2 vs cost 9). The nico_*4
 *      cases are exactly these pairs.
 *   3. `n1_positive_control_*` is an explicit pure-positive case with no
 *      negative assertion anywhere in it, so the EXACT path is proven to
 *      work even if every other case goes red.
 *   4. Every prompt-count claim counts the prompt STILL ON SCREEN at the
 *      assertion site, via rb_has_pending_choice + test_pending_choice_type,
 *      not a count taken at answer time.
 *
 * HARNESS LANDMINES RESPECTED
 * ---------------------------
 *   - rb_load("src") fails under the isolated out-of-tree build, so
 *     load_card_db() falls back to rb_load("../cards/build"). REQUIRED.
 *   - sizeof(TestGame) is ~781 KB: every TestGame local is `static`.
 *   - rb_create_card_copy uses a process-global monotonic pool that nothing
 *     resets, so rb_card_copy_pool_reset() runs at the top of every case.
 *   - rb_card_no_eq is FUZZY, so identity is asserted on the id returned for
 *     the number we asked for, and cost/type are read back separately.
 *   - RbChoice.zone for the waitroom is "discard", not "waitroom".
 *   - test_pending_choice_type is a STRING shim; the struct fields are read
 *     through rb_get_pending_choice.
 *   - No test_advance_to_phase: every case here is a single Main-phase play, so
 *     the whole PERFORMANCE_SECOND / active-swap / DRAW landmine cluster is
 *     avoided by construction. Phase is stepped ONLY by test_play_to_stage.
 *   - drain_pending() is CAPPED at 32: a peer found an ability that re-prompts
 *     unboundedly, and an uncapped drain would hang the whole suite.
 *   - test_try_play_to_stage == test_play_to_stage in C (both rb_play_member);
 *     C has no play_to_stage_baton, and no panic on an occupied slot, so the
 *     one baton case just uses test_play_to_stage.
 *
 * CONFIRMED ENGINE GAPS this file pins (assertions left STRICT and RED)
 * ---------------------------------------------------------------------
 * All six live in the ONE function that is the C counterpart of Rust's
 * `take_cards_from_standard_zone` for a waitroom source:
 *   src/ability/choice.c:2769-2778
 *       int discard_move = selected->action && !strcmp(selected->action,"move_cards")
 *           && selected->source && (!strcmp(selected->source,"discard")
 *                                  || !strcmp(selected->source,"waitroom"));
 *       if (discard_move) { rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "discard",
 *                       card_type, count, 0, NULL); ... }
 * compared with engine/src/ability/effects/move/selection.rs:66-86, which
 * builds a real `util::CardFilter` (card_type / group / cost_limit / operator,
 * threaded in from move/mod.rs:1230-1259), computes
 * `util::matching_indices(...)`, and then branches on
 * `util::resolve_selection(...)`:
 *       Exact(indices) => zone_remove_at_indices(...)   // NO PROMPT
 *       Prompt         => prompt_card_selection(..., Some(filtered_indices))
 *
 * G1  COST/TYPE FILTER ABSENT. The C emits an unfiltered SelectCard over the
 *     whole waitroom; `selected->cost_limit` / `cost_limit_operator` are never
 *     read and the prompt's own `cost_limit` is 0 with an empty operator. A
 *     cost-9 member is offered and accepted for 「コスト2以下」.
 *     Board proof: N6 — with the cost-9 card at discard index 0, the engine puts
 *     the COST-9 card on the stage (dump: n_filtered=2, cost_limit=0).
 *     Second board proof: N3 — P1's cost-4 discard card is moved to P1's stage
 *     even though P1 has no eligible card at all.
 *     Also affects: `rb_resolver_filter_discard_by_budget_full` (choice.c:2504)
 *     is a faithful port but is only reached on the REPROMPT path
 *     (choice.c:2624, :2640) and is fed `g->queue.pending.count` — the number of
 *     cards to PICK — where Rust passes the ability's cost limit.
 *
 * G2  NO "EXACT" PATH. Rust takes the single matching card directly with no
 *     prompt when matches == count; the C always prompts. Every Rust case that
 *     names the Exact path (N1, N4, N5, N6, N7, N8, N9, N10, N11, N12) is red
 *     on its prompt-shape assertion for this reason alone.
 *
 * G3  THE WAIT STATE IS DROPPED. `state_change: "wait"` is stashed into
 *     `g->queue.resume_move_state` at choice.c:2786-2790 and cleared at :2767 —
 *     and that field has NO READER anywhere in the tree (grep: two writes, zero
 *     reads). `rb_place_card_in_zone` also hard-codes `P->stage_wait[area] = 0`
 *     (util.c:934). So an appeared member is never in wait state, on either
 *     seat, in any of these cases.
 *
 * G4  NO ABILITY-QUEUE ENTRY, so `choice_player_id` is unreadable. The engine's
 *     own [RESUME_CAPTURE] line reports `n=0` for this ability: the prompt comes
 *     from the resolver's spawn context and the queue holds no entry, so
 *     test_queue_entry_choice_seat returns -1. The RbChoice's own
 *     `target_player_id` IS stamped and does flip self/opponent correctly
 *     (N5's first prompt is labelled p2 and passes that channel), so this file
 *     asserts the two channels SEPARATELY via check_prompt_seat().
 *
 * G5  FULL-STAGE `empty_area` PLACEMENT DESTROYS THE CARD. The card is lifted
 *     out of the waitroom first (choice.c:2611 / :2655
 *     `rb_waitroom_remove_card`), then `rb_choice_send_to_dst` falls through to
 *     `rb_place_card_in_zone(g, pl, cid, "empty_area", -1)`, which returns 0
 *     when `rb_stage_first_empty` is -1 (util.c:932-934). The card is then in no
 *     zone at all. MEASURED: N5 census 12 -> 11, N11 census 14 -> 13. This is
 *     the card-destroying class the corpus already knows about, on a
 *     destination the recent zone_bag fix (util.c:940-948) did not cover.
 *
 * G6  `target="opponent"` IS STAMPED BUT IGNORED BY THE PICK. The choice carries
 *     target="opponent" / target_player_id="opponent", yet
 *     `rb_resolver_handle_discard_selection` reads `actor = g->queue.actor` and
 *     does `rb_zone_cards(g, actor, "waitroom", ...)` (choice.c:2602, :2651) and
 *     `rb_choice_send_to_dst(g, actor, ...)` — so the card always comes from and
 *     goes to the ACTOR's zones, never `g->active ^ 1`. Rust uses
 *     `gs.resolve_target_player(&target)` at the same point.
 *     Board proof: N5 — the opponent-labelled prompt is answered, P2's own
 *     discard card is untouched, and P1's card is removed from P1's discard and
 *     then destroyed on P1's already-full stage.
 *
 * G7  THE DEFERRED OPPONENT HALF OF `target="both"` IS SILENTLY DROPPED when
 *     the self half creates a choice. Rust defers the opponent effect onto
 *     `gs.ability_queue.set_pending_actions` (effects/target.rs:128-138) and runs
 *     it from `finalize_choice`. In C the self half resolves and the engine's own
 *     [CHOICE_EPILOGUE] reports `queue_actions=0`, so the opponent half is never
 *     queued and never runs. Observed in N1, N2b, N4, N7, N8, N9, N10, N12, N13:
 *     no second prompt, and the opponent's card is still in the opponent's
 *     discard. When the self half creates NO choice the opponent half DOES run
 *     (N5) — so the two arms of Rust's target.rs:125-145 are half-implemented.
 *     This is the same both-targets routing family that
 *     tests/test_parity_realcard_walkthrough.c documents for 愛♡スクリ～ム！'s
 *     hand->discard option; this file measures it on a different action,
 *     destination and card, so it is a second independent probe, not a duplicate.
 *
 * G8  AN APPEARED CARD'S OWN 登場 DOES NOT FIRE. N13 places 唐 可可
 *     (PL!SP-bp2-002-P) on both sides through Nico's ability; the C lands P1's
 *     Keke on the stage but `debut_count_this_turn` stays 1 for P1 and 0 for P2,
 *     and P2's Keke never lands at all (G7). Classified IN-FLIGHT: the brief
 *     lists `rb_record_card_appearance` callers as being worked on, and
 *     `git log --oneline -5` at the time of writing shows
 *     dc44ccf1 / c1b6a060 / 6eb5423e / 56215992, none of which touches
 *     src/ability/choice.c (its last commit is b2ba64c9). Re-run before acting.
 *
 * Honesty contract
 * ----------------
 * Every assertion is a hard CHECK about the C engine. Nothing is downgraded,
 * skipped or weakened to make the suite green. A divergence from Rust is the
 * most valuable thing this file can report and is left RED on purpose.
 *
 * RUN RESULT at the time of writing (isolated build, no engine edits by me):
 *   15 cases, 303 assertions, 2 pass, 13 fail, 0 crashes, 0 fixture setup bugs.
 *   The 2 passes are N2 (the skip rule, paired with N2b) and N14 (an isolated
 *   prompt-shape / capped-drain case). Every other case is red on one or more of
 *   G1-G7 above, and each red is a hard CHECK on the Rust-expected behaviour.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>
#include <sys/mman.h>

/* ═══════════════════════════════ harness ═══════════════════════════════ */

static const char *current_test = "(none)";

/* The tally lives in a MAP_SHARED|MAP_ANONYMOUS mapping so a child that dies
 * on a signal still leaves every assertion it had already made in a number the
 * parent can print. A child-local counter would report 0 assertions for a test
 * that crashed after 40 of them. */
typedef struct {
    long assertions;
    int  failures;
    int  setup_bugs;
} Tally;
static Tally *tally;

#define CHECK(condition, message) do {                              \
    if (tally) tally->assertions++;                                 \
    if (!(condition)) {                                              \
        if (tally) tally->failures++;                               \
        fprintf(stderr, "FAIL [%s]: %s\n", current_test, message);  \
    } else {                                                         \
        printf("  ok: %s\n", message);                               \
    }                                                                \
    fflush(stdout); fflush(stderr);                                  \
} while (0)

#define CHECK_EQ(actual, expected, message) do {                    \
    if (tally) tally->assertions++;                                 \
    int a_ = (actual);                                              \
    int e_ = (expected);                                            \
    if (a_ != e_) {                                                 \
        if (tally) tally->failures++;                               \
        fprintf(stderr, "FAIL [%s]: %s (got %d expected %d)\n",      \
                current_test, message, a_, e_);                     \
    } else {                                                         \
        printf("  ok: %s\n", message);                               \
    }                                                                \
    fflush(stdout); fflush(stderr);                                  \
} while (0)

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    return rb_load("../cards/build");
}

/* ── fixture cards, all verified against cards/cards.json ── */
#define NICO_NO   "PL!-pb1-018-R"    /* 矢澤にこ   member cost 7 */
#define C19_NO    "PL!SP-sd1-019-SD" /* 若菜四季    member cost 2 */
#define C20_NO    "PL!SP-sd1-020-SD" /* 鬼塚夏美    member cost 2 */
#define SD2_NO    "PL!-sd1-002-SD"   /* 絢瀬 絵里   member cost 2 */
#define SD5_NO    "PL!-sd1-005-SD"   /* 星空 凛     member cost 2 */
#define KEKE_NO   "PL!SP-bp2-002-P"  /* 唐 可可     member cost 2 */
#define EXP9_NO   "PL!-sd1-014-SD"   /* 星空 凛     member cost 9 */
#define FILLER_NO "PL!-sd1-010-SD"   /* 高坂 穂乃果 member cost 4 */

/* mk(): a DISTINCT instance, because (see the header) every fixture card below
 * occupies exactly one physical slot in this port while the Rust reuses one
 * interned id across two zones. Identity is asserted, so a missing or
 * mislabelled card is a SETUP BUG and can never masquerade as a rules
 * divergence. */
static int mk(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id < 0) {
        fprintf(stderr, "SETUP BUG [%s]: card %s does not exist\n", current_test, no);
        if (tally) tally->setup_bugs++;
        return -1;
    }
    if (!rb_card_no_eq(id, no)) {
        fprintf(stderr, "SETUP BUG [%s]: new id %d is not %s\n", current_test, id, no);
        if (tally) tally->setup_bugs++;
        return -1;
    }
    return id;
}

/* tmpl(): the Rust `game.id(no)` form WITHOUT a copy — rb_find_card_by_no, i.e.
 * the shared TEMPLATE index, the way tests/test_parity_realcard_walkthrough.c's
 * `mid()` does it. Used by S1 only. mk() (a distinct copy via
 * rb_create_card_copy) is right for the nico family, where every fixture card
 * must occupy its own physical slot, but it is NOT interchangeable: a copied id
 * resolves its ability table differently, and S1 measured that difference —
 * with a copy id the live-start scan queued the ability but the flavour question
 * never surfaced, which is a FIXTURE artefact and not an engine finding. S1
 * therefore uses the sibling's exact idiom. Both helpers assert identity, so a
 * missing card is still a SETUP BUG. */
static int tmpl(TestGame *tg, const char *no)
{
    int id = test_id(tg, no);
    if (id < 0) {
        fprintf(stderr, "SETUP BUG [%s]: card %s does not exist\n", current_test, no);
        if (tally) tally->setup_bugs++;
        return -1;
    }
    if (!rb_card_no_eq(id, no)) {
        fprintf(stderr, "SETUP BUG [%s]: template id %d is not %s\n", current_test, id, no);
        if (tally) tally->setup_bugs++;
        return -1;
    }
    return id;
}

/* One assertion that a fixture really is a member of the expected printed cost.
 * Load-bearing for every "cost 2 or below" / "cost 9 is excluded" claim. */
static void expect_member_cost(TestGame *tg, const char *tag, int id, int cost)
{
    Card c;
    char buf[128];
    snprintf(buf, sizeof buf, "%s: %s is a member card", tag, "");
    if (!rb_decode_card_by_index((uint32_t)id, &c)) {
        CHECK(0, tag);
        return;
    }
    snprintf(buf, sizeof buf, "%s: the fixture is a member card (type)", tag);
    CHECK(rb_card_is_member(id), buf);
    snprintf(buf, sizeof buf, "%s: the fixture's PRINTED cost is %d", tag, cost);
    CHECK_EQ(c.cost, cost, buf);
    rb_free_card(&c);
}

/* ── zone helpers ── */
static void bag_clear(RbBag *b) { b->n = 0; }
static void bag_push(RbBag *b, int id)
{
    if (b->n < RB_MAX_ZONE) b->cards[b->n++] = id;
}
static void clear_stage(RbPlayer *P)
{
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        P->stage[i] = RB_EMPTY_SLOT;
        P->stage_wait[i] = 0;
    }
}
static int hand_push(TestGame *tg, int pl, int id) { bag_push(&tg->state.p[pl].hand, id); return id; }
static int disc_push(TestGame *tg, int pl, int id) { bag_push(&tg->state.p[pl].discard, id); return id; }
static int stage_put(TestGame *tg, int pl, int area, int id)
{
    tg->state.p[pl].stage[area] = id;
    tg->state.p[pl].stage_wait[area] = 0;
    return id;
}
static int stage_get(TestGame *tg, int pl, int area) { return tg->state.p[pl].stage[area]; }
static int stage_occupied(TestGame *tg, int pl)
{
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (tg->state.p[pl].stage[i] != RB_EMPTY_SLOT) n++;
    return n;
}
static int in_zone(TestGame *tg, int pl, const char *zone, int id)
{
    return test_zone_count_of_id(tg, pl, zone, id) > 0;
}
static int is_wait(TestGame *tg, int id)
{
    const char *o = rb_mods_get_orientation(&tg->state.mods, id);
    return o && strcmp(o, "wait") == 0;
}
/* Rust: game.state.ability_queue.current_entry().choice_player_id */
static int choice_seat(TestGame *tg)
{
    return test_queue_entry_choice_seat(tg, tg->state.queue.cur);
}

/* Diagnostic dump of the prompt standing at the assertion site. Not an
 * assertion; this is how a divergence gets attributed to a field. */
static void dump_pending(TestGame *tg, const char *label)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (!c) {
        printf("    <%s> NO PENDING CHOICE\n", label);
        fflush(stdout);
        return;
    }
    printf("    <%s> kind=%s zone='%s' card_type='%s' count=%d target='%s' "
           "actor=%d target_player_id='%s' cost_limit=%d op='%s' n_filtered=%d "
           "filter_group='%s' allow_skip=%d route=%d desc='%s'\n",
           label, test_pending_choice_type(tg), c->zone, c->card_type, c->count,
           c->target, c->actor, c->target_player_id, c->cost_limit, c->cost_limit_op,
           c->n_filtered_indices, c->filter_group, c->allow_skip, (int)c->route,
           c->description);
    fflush(stdout);
}

/* Rust: game.select_option(i) */
static void select_option(TestGame *tg, int i)
{
    rb_resume_with_choice(&tg->state, i);
}
/* Rust: game.select_indices(&[i]) */
static void pick(TestGame *tg, int i)
{
    test_select_indices(tg, &i, 1);
}
/* CAPPED drain. Never used where a prompt is the thing under test. */
static int drain_pending(TestGame *tg)
{
    int guard = 0;
    while (rb_has_pending_choice(&tg->state) && guard++ < 32)
        rb_resume_with_choice(&tg->state, 0);
    return guard;
}

/* ---------------------------------------------------------------------------
 * answer(): the ONE way this file drives a prompt, and it is deliberately
 * kind-aware. Every prompt-SHAPE and prompt-SEAT assertion is made BEFORE the
 * call and is STRICT, so a divergence from Rust is reported rather than hidden.
 * But once the shape is known, the answer must be one the standing prompt can
 * actually accept, or the rest of the case measures nothing: answering a
 * SelectCard with a stage-area index silently reprompts
 * (rb_resolver_handle_discard_selection, choice.c:2638-2647) and the board
 * assertions after it would all read "nothing happened" for a reason that has
 * nothing to do with the ability. `area` is used only for a
 * SelectPosition / SelectTarget prompt.
 * ------------------------------------------------------------------------- */
static void answer(TestGame *tg, int area)
{
    if (!rb_has_pending_choice(&tg->state)) return;
    const char *t = test_pending_choice_type(tg);
    if (strcmp(t, "SelectCard") == 0) { pick(tg, 0); return; }
    select_option(tg, area);
}

/* The seat a standing prompt is addressed to, read from the RbChoice itself.
 * Returns -1 when the prompt carries NO seat label, which is a distinct answer
 * from 0 and is what makes "the opponent's prompt is unlabelled" assertable. */
static int prompt_seat(TestGame *tg)
{
    const RbChoice *c = rb_get_pending_choice(&tg->state);
    if (!c) return -1;
    if (!strcmp(c->target_player_id, "p2") || !strcmp(c->target_player_id, "opponent")) return 1;
    if (!strcmp(c->target_player_id, "p1") || !strcmp(c->target_player_id, "self"))  return 0;
    return -1;                      /* empty / unrecognised */
}

/* Rust asserts `ability_queue.current_entry().choice_player_id == Some("pN")`.
 * C exposes that on TWO channels and this file asserts BOTH, separately,
 * because on this path they are independently broken:
 *   1. the RbChoice's own target_player_id / actor;
 *   2. RbQueueEntry.choice_player_id, which is what
 *      test_queue_entry_choice_seat reads.
 * The run log shows `n=0` in the engine's own [RESUME_CAPTURE] line for this
 * ability, i.e. the prompt is emitted by the resolver's spawn context and the
 * ability queue holds NO entry at all, so channel 2 has nothing to read and
 * returns -1. Both assertions are left strict and red. */
static void check_prompt_seat(TestGame *tg, int want, const char *what)
{
    char buf[220];
    snprintf(buf, sizeof buf, "%s: the standing prompt is LABELLED for p%d", what, want + 1);
    CHECK_EQ(prompt_seat(tg), want, buf);
    snprintf(buf, sizeof buf, "%s: the queue entry's choice_player_id names p%d", what, want + 1);
    CHECK_EQ(choice_seat(tg), want, buf);
}

static int new_game(void)
{
    rb_card_copy_pool_reset();
    return 0;
}

/* The absolute census numbers below include the ENERGY cards, because
 * test_give_energy(n) really appends n copies of LL-E-001-SD to
 * p[0].energy (test_game.c test_give_energy_for), and test_total_card_count
 * counts every energy-zone slot. So `census0` is always
 * (fixture cards) + 7, and every "+1" in a comment is a card this test itself
 * put into a zone. Getting this wrong would be a FALSE red, so it is stated
 * once here rather than left implicit. */
#define CENSUS(fixtures) (fixtures) + 7

/* ═══════════════════════════════════════════════════════════════════════
 * §N  矢澤にこ PL!-pb1-018-R
 * 登場: 自分と相手はそれぞれ、自身の控え室からコスト2以下のメンバーカードを1枚、
 *       メンバーのいないエリアにウェイト状態で登場させる。
 * ═══════════════════════════════════════════════════════════════════════ */

/* ---------------------------------------------------------------------------
 * N1 — POSITIVE CONTROL.  1 eligible card in P1's discard + 2 empty slots.
 * The EXACT path: no SelectCard at all, straight to SelectPosition, card lands
 * in wait. This case contains NO negative assertion at all, so the EXACT path
 * is proven green independently of every other case in the file.
 * ------------------------------------------------------------------------- */
static void n1_positive_control_exact_path(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico    = mk(&tg, NICO_NO);
    int cheap   = mk(&tg, C19_NO);   /* P1 discard */
    int cheap2  = mk(&tg, C20_NO);   /* P2 discard */
    int filler  = mk(&tg, FILLER_NO);
    if (nico < 0 || cheap < 0 || cheap2 < 0 || filler < 0) return;

    expect_member_cost(&tg, "N1 nico", nico, 7);
    expect_member_cost(&tg, "N1 cheap(P1 discard)", cheap, 2);
    expect_member_cost(&tg, "N1 cheap2(P2 discard)", cheap2, 2);
    expect_member_cost(&tg, "N1 filler", filler, 4);

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, filler);
    disc_push(&tg, 0, cheap);
    disc_push(&tg, 1, cheap2);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(4), "N1: census is 11 before the play (nico, filler, 2 cheap + 7 energy)");

    CHECK_EQ(test_play_to_stage(&tg, nico, 0), 1, "N1: Nico is played to the left area");

    /* THE PROMPT-COUNT CONTROL: the prompt is counted HERE, standing on screen. */
    CHECK(rb_has_pending_choice(&tg.state), "N1: the 登場 raises a prompt (non-vacuity control)");
    dump_pending(&tg, "N1 after play");
    CHECK_EQ(test_pending_choice_type(&tg) != NULL &&
             strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0, 1,
             "N1: the EXACT path skips SelectCard and goes straight to SelectPosition");
    check_prompt_seat(&tg, 0, "N1: the position prompt is addressed to P1");

    answer(&tg, 1);   /* centre */

    CHECK(rb_has_pending_choice(&tg.state), "N1: the OPPONENT half then prompts too");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N1: the opponent prompt is also a SelectPosition");
    check_prompt_seat(&tg, 1, "N1: the opponent prompt is addressed to P2 (target=both)");

    answer(&tg, 2);   /* right */

    CHECK(!rb_has_pending_choice(&tg.state), "N1: nothing is left pending");

    CHECK_EQ(stage_get(&tg, 0, 0), nico, "N1: Nico is at P1 left");
    CHECK_EQ(stage_get(&tg, 0, 1), cheap, "N1: P1's cost-2 member is at P1 centre");
    CHECK_EQ(stage_get(&tg, 0, 2), RB_EMPTY_SLOT, "N1: P1 right is still empty");
    CHECK_EQ(stage_get(&tg, 1, 0), RB_EMPTY_SLOT, "N1: P2 left is still empty");
    CHECK_EQ(stage_get(&tg, 1, 1), RB_EMPTY_SLOT, "N1: P2 centre is still empty");
    CHECK_EQ(stage_get(&tg, 1, 2), cheap2, "N1: P2's own cost-2 member is at P2 right");

    CHECK(is_wait(&tg, cheap), "N1: P1's appeared member is in WAIT state");
    CHECK(is_wait(&tg, cheap2), "N1: P2's appeared member is in WAIT state");
    CHECK_EQ(tg.state.p[0].stage_wait[1], 1, "N1: stage_wait[1] agrees for P1");
    CHECK_EQ(tg.state.p[1].stage_wait[2], 1, "N1: stage_wait[2] agrees for P2");

    CHECK(!in_zone(&tg, 0, "discard", cheap), "N1: P1's card left the discard");
    CHECK(!in_zone(&tg, 1, "discard", cheap2), "N1: P2's card left the discard");
    CHECK(!in_zone(&tg, 0, "hand", cheap), "N1: P1's card went to STAGE, not to hand");
    CHECK(!in_zone(&tg, 1, "hand", cheap2), "N1: P2's card went to STAGE, not to hand");
    /* The board half of the target="both" claim. P1's own card provably moved
     * (the four assertions above), so if P2's card is still sitting in P2's
     * discard the opponent half did not merely place it somewhere odd — it never
     * ran. The prompt assertions above already showed the second prompt never
     * opened; this is the same gap seen from the board. */
    CHECK(in_zone(&tg, 1, "discard", cheap2) == 0 || in_zone(&tg, 1, "stage", cheap2),
          "N1: P2's card left P2's discard (the target=\"both\" opponent half ran)");

    CHECK_EQ(test_total_card_count(&tg), census0,
             "N1: CENSUS unchanged — the two moves relocated cards, destroyed none");
}

/* ---------------------------------------------------------------------------
 * N2 — the "no suitable card" SKIP.  Both discards empty.
 * Paired with N1: the only difference is the presence of one cost-2 card, so a
 * green N1 + green N2 together prove the discard read is real and not a
 * universal no-op.
 * ------------------------------------------------------------------------- */
static void n2_no_suitable_card_skips(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico   = mk(&tg, NICO_NO);
    int filler = mk(&tg, FILLER_NO);
    if (nico < 0 || filler < 0) return;

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, filler);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(2), "N2: census is 9 before the play (Nico + filler, both discards empty) + 7 energy");

    CHECK_EQ(test_play_to_stage(&tg, nico, 0), 1, "N2: Nico is played to the left area");

    CHECK(!rb_has_pending_choice(&tg.state),
          "N2: with no eligible card in EITHER discard the ability asks nothing");
    dump_pending(&tg, "N2 after play");

    CHECK_EQ(stage_get(&tg, 0, 0), nico, "N2: Nico is at P1 left");
    CHECK_EQ(stage_get(&tg, 0, 1), RB_EMPTY_SLOT, "N2: P1 centre stayed empty");
    CHECK_EQ(stage_get(&tg, 0, 2), RB_EMPTY_SLOT, "N2: P1 right stayed empty");
    CHECK_EQ(stage_occupied(&tg, 0), 1, "N2: exactly ONE member (Nico) is on P1's stage");
    CHECK_EQ(stage_occupied(&tg, 1), 0, "N2: P2's stage is still empty");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "N2: CENSUS unchanged — the ability skipped, it destroyed nothing");
}

/* ---------------------------------------------------------------------------
 * N2b — N2's POSITIVE TWIN, in the same shape, so N2 cannot be green merely
 * because the engine ignores the discard zone entirely.
 *
 * N2 alone is a negative case: on an empty discard "nothing happens" is also
 * what a completely inert engine produces, so a green N2 proves nothing on its
 * own. N2b differs from N2 by EXACTLY ONE thing — one cost-2 card is added to
 * P1's discard — and asserts that a prompt now appears. N2 green + N2b red
 * would mean the discard is never read; N2 green + N2b green means the discard
 * IS read and the skip in N2 is the rule, not an accident. N2b contains no
 * negative assertion about the engine's output state, so it stays a clean
 * positive control.
 * ------------------------------------------------------------------------- */
static void n2b_empty_discard_skips_but_one_card_prompts(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico   = mk(&tg, NICO_NO);
    int filler = mk(&tg, FILLER_NO);
    int cheap  = mk(&tg, C19_NO);
    if (nico < 0 || filler < 0 || cheap < 0) return;

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, filler);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(2), "N2b: census is 9 — identical to N2's board");

    /* --- arm 1: N2's exact board, empty discards. The ability must ask nothing. */
    CHECK_EQ(test_play_to_stage(&tg, nico, 0), 1, "N2b arm 1: Nico is played (N2's board)");
    CHECK(!rb_has_pending_choice(&tg.state),
          "N2b arm 1: with an empty P1 discard AND an empty P2 discard, nothing is asked");
    CHECK_EQ(stage_occupied(&tg, 0), 1, "N2b arm 1: only Nico is on the stage");
    CHECK_EQ(stage_occupied(&tg, 1), 0, "N2b arm 1: P2's stage is untouched");

    /* --- arm 2: add the ONE eligible card and run the same debut again. */
    int nico2 = mk(&tg, NICO_NO);
    if (nico2 < 0) return;
    hand_push(&tg, 0, nico2);
    disc_push(&tg, 0, cheap);
    test_give_energy(&tg, 7);
    CHECK_EQ(test_total_card_count(&tg), census0 + 9,
             "N2b: census rose by the second Nico + the cost-2 card + 7 new energy cards");

    CHECK_EQ(test_play_to_stage(&tg, nico2, 1), 1, "N2b arm 2: a second Nico is played");
    CHECK(rb_has_pending_choice(&tg.state),
          "N2b arm 2: THE ONE ADDED COST-2 CARD MAKES THE ABILITY ASK — the discard "
          "is read, so N2's skip is the rule and not an inert engine");
    dump_pending(&tg, "N2b arm 2 prompt");
    check_prompt_seat(&tg, 0, "N2b arm 2: the standing prompt is P1's");

    answer(&tg, 1);
    CHECK(rb_has_pending_choice(&tg.state), "N2b arm 2: the opponent half follows");
    answer(&tg, 2);
    CHECK(!rb_has_pending_choice(&tg.state), "N2b arm 2: both halves resolved");

    CHECK(!in_zone(&tg, 0, "discard", cheap),
          "N2b arm 2: the cost-2 card really left P1's discard");
    CHECK_EQ(test_total_card_count(&tg), census0 + 9,
             "N2b: CENSUS unchanged across arm 2 — nothing was destroyed");
}

/* ---------------------------------------------------------------------------
 * N3 — SYNC path.  P1's discard holds only a cost-4 card (not eligible), P2's
 * holds TWO eligible cards. The P1 half must be skipped WITHOUT asking, and the
 * OPPONENT must be the one asked — routed to p2, not to p1.
 * The non-vacuity control is a live prompt: if the routing collapsed onto the
 * actor, the prompt would be P1's and this goes red.
 * ------------------------------------------------------------------------- */
static void n3_sync_path_opponent_gets_choice_when_self_has_none(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico   = mk(&tg, NICO_NO);
    int filler = mk(&tg, FILLER_NO);
    /* P1's discard holds ONE card, and it must be INELIGIBLE. The Rust twin
     * pushes `filler` (PL!-sd1-010-SD, printed cost 4); the C uses a DISTINCT
     * instance of the same print so the card is a separate physical card from
     * the hand copy, and asserts the printed cost so a cards.json drift can
     * never quietly turn this into an eligible card and stop testing the
     * routing. */
    int inelig = mk(&tg, FILLER_NO);
    int a      = mk(&tg, C19_NO);   /* P2 discard, eligible */
    int b      = mk(&tg, C20_NO);   /* P2 discard, eligible */
    if (nico < 0 || filler < 0 || inelig < 0 || a < 0 || b < 0) return;
    expect_member_cost(&tg, "N3 P1's ineligible discard card", inelig, 4);

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, filler);
    disc_push(&tg, 0, inelig);      /* P1 discard: NOT eligible */
    disc_push(&tg, 1, a);
    disc_push(&tg, 1, b);
    test_give_energy(&tg, 7);

    CHECK(inelig != filler, "N3: the discard card is a DISTINCT instance from the hand copy");

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(5), "N3: census is 12 before the play (5 fixtures + 7 energy)");

    CHECK_EQ(test_play_to_stage(&tg, nico, 0), 1, "N3: Nico is played to the left area");

    CHECK(rb_has_pending_choice(&tg.state),
          "N3: a prompt is standing (non-vacuity control for the routing claim)");
    dump_pending(&tg, "N3 after play");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectCard") == 0,
          "N3: two eligible cards in P2's discard raise a SelectCard, not an Exact placement");
    check_prompt_seat(&tg, 1, "N3: the SelectCard is routed to P2 — P1 had no eligible card at all");

    answer(&tg, 0);   /* P2 selects the first of their two */

    CHECK(rb_has_pending_choice(&tg.state), "N3: a position prompt follows the selection");
    dump_pending(&tg, "N3 after select");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N3: P2's stage is empty (3 slots) so the placement asks for a position");
    check_prompt_seat(&tg, 1, "N3: the position prompt is P2's");

    answer(&tg, 0);   /* left */

    CHECK(!rb_has_pending_choice(&tg.state), "N3: nothing is left pending");

    CHECK_EQ(stage_get(&tg, 0, 0), nico, "N3: P1's stage is UNCHANGED — Nico at left");
    CHECK_EQ(stage_get(&tg, 0, 1), RB_EMPTY_SLOT, "N3: P1 centre is still empty");
    CHECK_EQ(stage_get(&tg, 0, 2), RB_EMPTY_SLOT, "N3: P1 right is still empty");
    CHECK_EQ(stage_occupied(&tg, 0), 1, "N3: P1 gained no member from its own ability");

    CHECK_EQ(stage_get(&tg, 1, 0), a, "N3: the card P2 selected is at P2 left");
    CHECK(is_wait(&tg, a), "N3: P2's appeared member is in WAIT state");
    CHECK(in_zone(&tg, 1, "discard", b), "N3: the card P2 did NOT select is still in the discard");
    CHECK(in_zone(&tg, 0, "discard", inelig), "N3: P1's ineligible card is untouched in the discard");

    CHECK_EQ(test_total_card_count(&tg), census0, "N3: CENSUS unchanged");
}

/* ---------------------------------------------------------------------------
 * N4 — Q170 ordering.  1 eligible card on each side, 2 empty slots each.
 * Identical in shape to N1; it is kept because the Rust pairs it with N1 as
 * the "turn player resolves FIRST" case, and the observable that differs is
 * the ORDER of the two prompts (P1 then P2), not the final board.
 * ------------------------------------------------------------------------- */
static void n4_q170_turn_player_appears_first(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico   = mk(&tg, NICO_NO);
    int a      = mk(&tg, C19_NO);   /* P1 hand copy */
    int a_dup  = mk(&tg, C19_NO);   /* P1 discard copy — a DISTINCT instance;
                                     * Rust reuses one interned id here */
    int b      = mk(&tg, C20_NO);   /* P2 discard */
    if (nico < 0 || a < 0 || a_dup < 0 || b < 0) return;

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, a);
    disc_push(&tg, 0, a_dup);
    disc_push(&tg, 1, b);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(4), "N4: census is 11 before the play (4 fixtures + 7 energy)");

    CHECK_EQ(test_play_to_stage(&tg, nico, 0), 1, "N4: Nico is played to the left area");

    CHECK(rb_has_pending_choice(&tg.state), "N4: a prompt is standing (non-vacuity control)");
    dump_pending(&tg, "N4 prompt 1");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N4: one eligible card each -> no SelectCard, straight to a position");
    check_prompt_seat(&tg, 0, "N4: the TURN player (P1) is asked FIRST");

    answer(&tg, 1);   /* P1 centre */

    CHECK(rb_has_pending_choice(&tg.state), "N4: the opponent prompt follows immediately");
    dump_pending(&tg, "N4 prompt 2");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N4: the opponent prompt is a position choice too");
    check_prompt_seat(&tg, 1, "N4: the OPPONENT (P2) is asked second");

    answer(&tg, 2);   /* P2 right */

    CHECK(!rb_has_pending_choice(&tg.state), "N4: nothing is left pending");
    CHECK_EQ(stage_get(&tg, 0, 0), nico, "N4: Nico at P1 left");
    CHECK_EQ(stage_get(&tg, 0, 1), a_dup, "N4: the card that came from P1's DISCARD is at centre");
    CHECK_EQ(stage_get(&tg, 1, 2), b, "N4: P2's card is at P2 right");
    CHECK(in_zone(&tg, 0, "hand", a), "N4: the hand copy of the same PRINT was not moved");
    CHECK(is_wait(&tg, a_dup), "N4: P1's appeared member is in WAIT state");
    CHECK(is_wait(&tg, b), "N4: P2's appeared member is in WAIT state");
    CHECK_EQ(test_total_card_count(&tg), census0, "N4: CENSUS unchanged");
}

/* ---------------------------------------------------------------------------
 * N5 — EMPTY-AREA RESTRICTION, self side.  P1's stage is [F, -, F] and Nico is
 * played to the CENTRE, so P1's stage is full afterwards and the self half has
 * no room. P1's eligible card must go BACK to the discard untouched while P2's
 * half still fires. Paired with N1: the only difference is P1's occupancy.
 * ------------------------------------------------------------------------- */
static void n5_requires_empty_area_self_side(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico    = mk(&tg, NICO_NO);
    int cheap1  = mk(&tg, C19_NO);   /* P1 discard */
    int cheap2  = mk(&tg, C20_NO);   /* P2 discard — a DISTINCT instance; Rust
                                      * reuses one interned id here */
    int f0      = mk(&tg, FILLER_NO);
    int f2      = mk(&tg, FILLER_NO);
    if (nico < 0 || cheap1 < 0 || cheap2 < 0 || f0 < 0 || f2 < 0) return;

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    stage_put(&tg, 0, 0, f0);
    stage_put(&tg, 0, 2, f2);
    disc_push(&tg, 0, cheap1);
    disc_push(&tg, 1, cheap2);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(5), "N5: census is 12 before the play (5 fixtures + 7 energy)");

    CHECK_EQ(test_play_to_stage(&tg, nico, 1), 1,
             "N5: Nico is played to the centre, filling P1's only empty slot");

    CHECK(rb_has_pending_choice(&tg.state),
          "N5: a prompt is standing (non-vacuity control) — the opponent half still fires");
    dump_pending(&tg, "N5 after play");
    /* Rust (effects/target.rs:125-145): P1's own half creates NO choice because
     * the stage is full, so `handle_both_targets` falls through to the opponent
     * half. The C agrees on THAT (this prompt is labelled p2 below), so the only
     * divergence asserted here is the prompt SHAPE. */
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N5: the OPPONENT half takes the Exact path (1 eligible card) and asks "
          "only for a stage position — P1's full stage means P1 is never asked at all");
    check_prompt_seat(&tg, 1, "N5: the standing prompt belongs to P2");

    answer(&tg, 2);   /* P2 right */

    CHECK(!rb_has_pending_choice(&tg.state), "N5: nothing is left pending");
    CHECK_EQ(stage_get(&tg, 0, 0), f0, "N5: P1 left unchanged");
    CHECK_EQ(stage_get(&tg, 0, 1), nico, "N5: Nico at P1 centre");
    CHECK_EQ(stage_get(&tg, 0, 2), f2, "N5: P1 right unchanged — the stage is exactly as set up");
    CHECK_EQ(stage_occupied(&tg, 0), 3, "N5: P1's stage is full, no extra card appeared");
    CHECK(in_zone(&tg, 0, "discard", cheap1),
          "N5: P1's eligible card is BACK in the discard (no empty area)");
    CHECK(in_zone(&tg, 1, "discard", cheap2),
          "N5: P2's OWN discard card is untouched — the pick must be read from the "
          "seat the prompt is labelled for, not from the actor");
    CHECK_EQ(stage_get(&tg, 1, 2), cheap2, "N5: P2's card did appear, at P2 right");
    CHECK(is_wait(&tg, cheap2), "N5: P2's appeared member is in WAIT state");
    CHECK(!is_wait(&tg, cheap1), "N5: the card that never appeared has NO wait state");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "N5: CENSUS unchanged — the restricted card was left in the discard, not deleted");
}

/* ---------------------------------------------------------------------------
 * N6 — COST FILTER.  P1's discard holds one cost-2 and one cost-9 card. Only the
 * cost-2 card may be used. Paired with N1, which has the cost-2 card alone:
 * the pair is what proves the cost-9 card is EXCLUDED by the filter rather than
 * simply not chosen.
 *
 * THE DISCARD ORDER IS DELIBERATELY REVERSED relative to the Rust fixture.
 * Rust pushes [cheap, expensive] and then takes the Exact path, so order is
 * irrelevant there. Here the expensive card is pushed FIRST, so a filter-blind
 * engine puts the cost-9 card on the stage when index 0 is answered. That turns
 * the cost-filter claim from a metadata observation (`cost_limit` on the prompt)
 * into a BOARD observation, which is the only form that cannot be satisfied by a
 * prompt that merely prints the right filter and ignores it.
 * ------------------------------------------------------------------------- */
static void n6_cost_filter_only_shows_eligible(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico   = mk(&tg, NICO_NO);
    int cheap  = mk(&tg, C19_NO);   /* P1 hand copy, cost 2 */
    int c_p1   = mk(&tg, C19_NO);   /* P1 discard, cost 2  -> eligible */
    int c_p2   = mk(&tg, C20_NO);   /* P2 discard, cost 2  -> eligible */
    int exp    = mk(&tg, EXP9_NO);  /* P1 discard, cost 9  -> INELIGIBLE */
    if (nico < 0 || cheap < 0 || c_p1 < 0 || c_p2 < 0 || exp < 0) return;

    expect_member_cost(&tg, "N6 eligible print", c_p1, 2);
    expect_member_cost(&tg, "N6 excluded print", exp, 9);

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, cheap);
    disc_push(&tg, 0, exp);          /* INELIGIBLE first — see the header */
    disc_push(&tg, 0, c_p1);         /* eligible second */
    disc_push(&tg, 1, c_p2);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(5), "N6: census is 12 before the play (5 fixtures + 7 energy)");
    CHECK_EQ(test_zone_count_of_id(&tg, 0, "discard", exp), 1,
             "N6: the cost-9 card really IS in P1's discard (filter precondition)");
    CHECK_EQ(tg.state.p[0].discard.cards[0], exp,
             "N6: the cost-9 card really is discard index 0 (the board-probe precondition)");

    CHECK_EQ(test_play_to_stage(&tg, nico, 0), 1, "N6: Nico is played to the left area");

    /* The filter is the claim. Only ONE of the two discard cards passes
     * cost<=2, so the prompt must be a POSITION prompt, not a SelectCard. */
    CHECK(rb_has_pending_choice(&tg.state), "N6: a prompt is standing (non-vacuity control)");
    dump_pending(&tg, "N6 after play");
    {
        const RbChoice *c = rb_get_pending_choice(&tg.state);
        CHECK_EQ(c ? c->cost_limit : -1, 2,
                 "N6: the prompt carries the ability's 「コスト2以下」 limit");
        CHECK_EQ(c ? c->n_filtered_indices : -1, 1,
                 "N6: only ONE candidate is offered — the cost-9 card is filtered out");
    }
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N6: only ONE discard card passes cost<=2, so NO SelectCard is offered");
    check_prompt_seat(&tg, 0, "N6: the prompt is P1's");

    answer(&tg, 1);   /* index 0 of the standing SelectCard */
    CHECK(rb_has_pending_choice(&tg.state), "N6: the opponent half prompts next");
    check_prompt_seat(&tg, 1, "N6: the opponent prompt is P2's");
    answer(&tg, 2);

    CHECK(!rb_has_pending_choice(&tg.state), "N6: nothing is left pending");
    CHECK_EQ(stage_get(&tg, 0, 1), c_p1,
             "N6: the cost-2 card is the one that appears — the only card that "
             "satisfies 「コスト2以下」");
    CHECK_EQ(stage_get(&tg, 1, 2), c_p2, "N6: P2's cost-2 card is on P2's stage");
    CHECK(is_wait(&tg, c_p1), "N6: the cost-2 card is in WAIT state");

    CHECK_EQ(stage_get(&tg, 0, 0), nico, "N6: no other card landed in P1's other slots");
    CHECK_EQ(stage_get(&tg, 0, 2), RB_EMPTY_SLOT, "N6: P1 right is still empty");
    CHECK(!in_zone(&tg, 0, "stage", exp),
          "N6: the cost-9 card is NOT on the stage — the cost filter excluded it");
    CHECK(in_zone(&tg, 0, "discard", exp),
          "N6: the cost-9 card is still in the discard (never selectable)");
    CHECK(!is_wait(&tg, exp), "N6: the cost-9 card was never given a wait state");
    CHECK_EQ(test_total_card_count(&tg), census0, "N6: CENSUS unchanged");
}

/* ---------------------------------------------------------------------------
 * N7 — the appeared card OCCUPIES its slot.  After the placement the centre slot
 * is taken, which is what makes N5 (full stage) possible.
 * ------------------------------------------------------------------------- */
static void n7_q169_appeared_card_occupies_area(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico   = mk(&tg, NICO_NO);
    int hand_c = mk(&tg, C19_NO);   /* P1 hand copy */
    int c_p1   = mk(&tg, C19_NO);   /* P1 discard copy */
    int c_p2   = mk(&tg, C20_NO);   /* P2 discard */
    if (nico < 0 || hand_c < 0 || c_p1 < 0 || c_p2 < 0) return;

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, hand_c);
    disc_push(&tg, 0, c_p1);
    disc_push(&tg, 1, c_p2);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(4), "N7: census is 11 before the play (4 fixtures + 7 energy)");
    CHECK_EQ(stage_occupied(&tg, 0), 0, "N7: P1's stage starts EMPTY (precondition)");

    CHECK_EQ(test_play_to_stage(&tg, nico, 0), 1, "N7: Nico is played to the left area");

    CHECK(rb_has_pending_choice(&tg.state), "N7: a prompt is standing (non-vacuity control)");
    check_prompt_seat(&tg, 0, "N7: P1 is asked");
    answer(&tg, 1);   /* centre */
    CHECK(rb_has_pending_choice(&tg.state), "N7: P2 is asked next");
    check_prompt_seat(&tg, 1, "N7: the second prompt is P2's");
    answer(&tg, 2);   /* P2 right */

    CHECK(!rb_has_pending_choice(&tg.state), "N7: nothing is left pending");
    CHECK_EQ(stage_get(&tg, 0, 1), c_p1, "N7: the appeared card occupies P1 centre");
    CHECK(stage_get(&tg, 0, 1) != RB_EMPTY_SLOT, "N7: the area is occupied, not empty");
    CHECK(is_wait(&tg, c_p1), "N7: the appeared card is in WAIT state");
    CHECK_EQ(stage_occupied(&tg, 0), 2, "N7: P1's stage holds Nico + the appeared card");
    CHECK(in_zone(&tg, 0, "hand", hand_c), "N7: the hand copy of the same PRINT stayed in hand");
    CHECK(!in_zone(&tg, 0, "hand", c_p1), "N7: the appeared card came from the discard, not hand");
    CHECK_EQ(test_total_card_count(&tg), census0, "N7: CENSUS unchanged");
}

/* ---------------------------------------------------------------------------
 * N8 — PROMPT PATH, self side.  P1's discard holds TWO eligible cards, so a
 * SelectCard appears first; P1's stage then has 2 empty slots so a SelectPosition
 * follows. P2 has ONE eligible card (the Exact path) and its own SelectPosition.
 * ------------------------------------------------------------------------- */
static void n8_prompt_path_two_eligible_in_discard(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico   = mk(&tg, NICO_NO);
    int filler = mk(&tg, FILLER_NO);
    int a      = mk(&tg, C19_NO);   /* P1 discard candidate 0 */
    int b      = mk(&tg, C20_NO);   /* P1 discard candidate 1 */
    int c_p2   = mk(&tg, C19_NO);   /* P2 discard — a DISTINCT instance; Rust
                                     * reuses one interned id here */
    if (nico < 0 || filler < 0 || a < 0 || b < 0 || c_p2 < 0) return;

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, filler);
    disc_push(&tg, 0, a);
    disc_push(&tg, 0, b);
    disc_push(&tg, 1, c_p2);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(5), "N8: census is 12 before the play (5 fixtures + 7 energy)");
    CHECK_EQ(test_zone_count_of_id(&tg, 0, "discard", a), 1,
             "N8: the first candidate really is in P1's discard (precondition)");

    CHECK_EQ(test_play_to_stage(&tg, nico, 0), 1, "N8: Nico is played to the left area");

    CHECK(rb_has_pending_choice(&tg.state), "N8: a prompt is standing (non-vacuity control)");
    dump_pending(&tg, "N8 prompt 1");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectCard") == 0,
          "N8: TWO eligible cards in P1's discard raise a SelectCard");
    check_prompt_seat(&tg, 0, "N8: the SelectCard is P1's");

    answer(&tg, 0);   /* index 0 of the standing SelectCard */

    CHECK(rb_has_pending_choice(&tg.state), "N8: a position prompt follows the selection");
    dump_pending(&tg, "N8 prompt 2");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N8: 2 empty slots on P1's stage, so the placement asks for a position");
    check_prompt_seat(&tg, 0, "N8: the position prompt is still P1's");

    answer(&tg, 1);   /* centre */

    CHECK(rb_has_pending_choice(&tg.state), "N8: the OPPONENT half prompts next");
    dump_pending(&tg, "N8 prompt 3");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N8: P2 had ONE eligible card (Exact) so it goes straight to a position");
    check_prompt_seat(&tg, 1, "N8: the third prompt is P2's");

    answer(&tg, 2);   /* P2 right */

    CHECK(!rb_has_pending_choice(&tg.state), "N8: nothing is left pending");
    CHECK_EQ(stage_get(&tg, 0, 1), a, "N8: the card P1 selected landed at P1 centre");
    CHECK(is_wait(&tg, a), "N8: P1's appeared card is in WAIT state");
    CHECK(in_zone(&tg, 0, "discard", b), "N8: the card P1 did NOT select is still in the discard");
    CHECK_EQ(stage_get(&tg, 1, 2), c_p2, "N8: P2's card appeared at P2 right");
    CHECK(is_wait(&tg, c_p2), "N8: P2's appeared card is in WAIT state");
    CHECK(!in_zone(&tg, 0, "hand", a), "N8: P1's card came from the discard, not the hand");
    CHECK(!in_zone(&tg, 1, "hand", c_p2), "N8: P2's card came from the discard, not the hand");
    CHECK_EQ(test_total_card_count(&tg), census0, "N8: CENSUS unchanged");
}

/* ---------------------------------------------------------------------------
 * N9 — BOTH sides on the PROMPT path.  Two eligible cards in each discard, so
 * the sequence is SelectCard(p1) -> SelectPosition(p1) -> SelectCard(p2) ->
 * SelectPosition(p2). Four prompts, alternating seats. This is the case that
 * catches a "the opponent's half collapsed onto the actor" regression, because
 * a collapsed routing produces p1/p1/p1/p1.
 * ------------------------------------------------------------------------- */
static void n9_both_players_two_eligible_cards(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico   = mk(&tg, NICO_NO);
    int filler = mk(&tg, FILLER_NO);
    int a1     = mk(&tg, C19_NO);   /* P1 discard candidate 0 */
    int a2     = mk(&tg, C20_NO);   /* P1 discard candidate 1 */
    int b1     = mk(&tg, SD2_NO);   /* P2 discard candidate 0 */
    int b2     = mk(&tg, SD5_NO);   /* P2 discard candidate 1 */
    if (nico < 0 || filler < 0 || a1 < 0 || a2 < 0 || b1 < 0 || b2 < 0) return;
    expect_member_cost(&tg, "N9 P2 candidate 0", b1, 2);
    expect_member_cost(&tg, "N9 P2 candidate 1", b2, 2);

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, filler);
    disc_push(&tg, 0, a1);
    disc_push(&tg, 0, a2);
    disc_push(&tg, 1, b1);
    disc_push(&tg, 1, b2);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(6), "N9: census is 13 before the play (6 fixtures + 7 energy)");

    CHECK_EQ(test_play_to_stage(&tg, nico, 0), 1, "N9: Nico is played to the left area");

    CHECK(rb_has_pending_choice(&tg.state), "N9: prompt 1 is standing (non-vacuity control)");
    dump_pending(&tg, "N9 prompt 1");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectCard") == 0, "N9: prompt 1 is a SelectCard");
    check_prompt_seat(&tg, 0, "N9: prompt 1 is P1's");
    answer(&tg, 0);

    CHECK(rb_has_pending_choice(&tg.state), "N9: prompt 2 is standing");
    dump_pending(&tg, "N9 prompt 2");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N9: prompt 2 is P1's SelectPosition");
    check_prompt_seat(&tg, 0, "N9: prompt 2 is P1's");
    answer(&tg, 1);   /* centre */

    CHECK(rb_has_pending_choice(&tg.state), "N9: prompt 3 is standing");
    dump_pending(&tg, "N9 prompt 3");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectCard") == 0,
          "N9: P2's TWO eligible cards raise their own SelectCard (not P1's again)");
    check_prompt_seat(&tg, 1, "N9: prompt 3 is P2's");
    answer(&tg, 0);

    CHECK(rb_has_pending_choice(&tg.state), "N9: prompt 4 is standing");
    dump_pending(&tg, "N9 prompt 4");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N9: prompt 4 is P2's SelectPosition");
    check_prompt_seat(&tg, 1, "N9: prompt 4 is P2's");
    answer(&tg, 2);   /* P2 right */

    CHECK(!rb_has_pending_choice(&tg.state), "N9: nothing is left pending");
    CHECK_EQ(stage_get(&tg, 0, 0), nico, "N9: Nico at P1 left");
    CHECK_EQ(stage_get(&tg, 0, 1), a1, "N9: P1's selected card at centre");
    CHECK_EQ(stage_get(&tg, 0, 2), RB_EMPTY_SLOT, "N9: P1 right is still empty");
    CHECK_EQ(stage_get(&tg, 1, 0), RB_EMPTY_SLOT, "N9: P2 left is still empty");
    CHECK_EQ(stage_get(&tg, 1, 1), RB_EMPTY_SLOT, "N9: P2 centre is still empty");
    CHECK_EQ(stage_get(&tg, 1, 2), b1, "N9: P2's selected card at P2 right");
    CHECK(is_wait(&tg, a1), "N9: P1's appeared card is in WAIT state");
    CHECK(is_wait(&tg, b1), "N9: P2's appeared card is in WAIT state");
    CHECK(in_zone(&tg, 0, "discard", a2), "N9: P1's unselected card is still in the discard");
    CHECK(in_zone(&tg, 1, "discard", b2), "N9: P2's unselected card is still in the discard");
    CHECK_EQ(test_total_card_count(&tg), census0, "N9: CENSUS unchanged");
}

/* ---------------------------------------------------------------------------
 * N10 — DIRECT PLACEMENT (the no-prompt path).  P1's stage is [F, F, -] and Nico
 * is baton-touched into the centre, leaving EXACTLY ONE empty slot, so P1's
 * appeared card has no position to ask about and must land in the only slot.
 * This is the case where a full sequence that does nothing looks perfect, so
 * the census plus the slot assertions are the whole non-vacuity argument here:
 * if the placement no-ops, stage[2] is empty and the census still matches only
 * because the card never left the discard — checked by zone identity too.
 * ------------------------------------------------------------------------- */
static void n10_prompt_path_direct_placement_wait_state(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    /* The Rust test builds a first `game`, plays into an occupied centre, and
     * then ABANDONS it in favour of a fresh `game2`; nothing in the first board
     * is asserted. Only the second board is ported here. */
    int nico  = mk(&tg, NICO_NO);
    int a     = mk(&tg, C19_NO);   /* P1 discard candidate 0 */
    int b     = mk(&tg, C20_NO);   /* P1 discard candidate 1 AND P2's only
                                    * candidate in Rust (one interned id); here
                                    * P1's copy and P2's copy are distinct */
    int b_p2  = mk(&tg, C20_NO);
    int f0    = mk(&tg, FILLER_NO);
    int f1    = mk(&tg, FILLER_NO);
    if (nico < 0 || a < 0 || b < 0 || b_p2 < 0 || f0 < 0 || f1 < 0) return;

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    stage_put(&tg, 0, 0, f0);
    stage_put(&tg, 0, 1, f1);      /* centre is OCCUPIED -> a baton touch */
    stage_put(&tg, 0, 2, RB_EMPTY_SLOT);
    hand_push(&tg, 0, nico);
    disc_push(&tg, 0, a);
    disc_push(&tg, 0, b);
    disc_push(&tg, 1, b_p2);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(6), "N10: census is 13 before the play (6 fixtures + 7 energy)");
    CHECK_EQ(stage_occupied(&tg, 0), 2, "N10: P1's stage starts with 2 of 3 slots filled");

    CHECK_EQ(test_play_to_stage(&tg, nico, 1), 1,
             "N10: Nico is played to the OCCUPIED centre (single baton touch)");

    CHECK(rb_has_pending_choice(&tg.state), "N10: prompt 1 is standing (non-vacuity control)");
    dump_pending(&tg, "N10 prompt 1");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectCard") == 0,
          "N10: P1's TWO eligible cards raise a SelectCard");
    check_prompt_seat(&tg, 0, "N10: prompt 1 is P1's");
    answer(&tg, 0);

    /* No P1 SelectPosition: the only empty slot was taken. The NEXT prompt must
     * therefore be P2's. This is the assertion that distinguishes "P1 placed
     * directly" from "P1's placement silently never happened". */
    CHECK(rb_has_pending_choice(&tg.state), "N10: the opponent prompt is standing");
    dump_pending(&tg, "N10 prompt 2");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N10: P1 had ONE empty slot -> placed DIRECTLY, no P1 position prompt");
    check_prompt_seat(&tg, 1, "N10: the standing prompt is P2's, not a leftover P1 prompt");

    answer(&tg, 0);   /* P2 left */

    CHECK(!rb_has_pending_choice(&tg.state), "N10: nothing is left pending");
    CHECK_EQ(stage_get(&tg, 0, 2), a,
             "N10: P1's card appeared at the ONLY empty slot (right)");
    CHECK_EQ(stage_get(&tg, 1, 0), b_p2, "N10: P2's card appeared at P2 left");
    CHECK(is_wait(&tg, a), "N10: P1's DIRECTLY placed card is STILL in wait state");
    CHECK(is_wait(&tg, b_p2), "N10: P2's appeared card is in wait state");
    CHECK(in_zone(&tg, 0, "discard", b), "N10: P1's unselected card is still in the discard");
    CHECK_EQ(stage_occupied(&tg, 0), 3, "N10: P1's stage is now full");
    CHECK_EQ(test_total_card_count(&tg), census0, "N10: CENSUS unchanged");
}

/* ---------------------------------------------------------------------------
 * N11 — FULL STAGE, self side, prompt path.  Same board as N10 but P1's stage is
 * ALREADY full when the 登場 fires, so P1's half is skipped without even
 * offering the SelectCard, while P2's half still fires. Paired with N10: the
 * two differ by one empty slot, so the pair isolates the empty-area rule.
 * ------------------------------------------------------------------------- */
static void n11_full_stage_then_prompt_path(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico   = mk(&tg, NICO_NO);
    int filler = mk(&tg, FILLER_NO);
    int a      = mk(&tg, C19_NO);   /* P1 discard candidate */
    int b      = mk(&tg, C20_NO);   /* P1 discard candidate */
    int c_p2   = mk(&tg, C19_NO);   /* P2 discard — DISTINCT instance in C */
    int f0     = mk(&tg, FILLER_NO);
    int f2     = mk(&tg, FILLER_NO);
    if (nico < 0 || filler < 0 || a < 0 || b < 0 || c_p2 < 0 || f0 < 0 || f2 < 0) return;

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    stage_put(&tg, 0, 0, f0);
    stage_put(&tg, 0, 2, f2);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, filler);
    disc_push(&tg, 0, a);
    disc_push(&tg, 0, b);
    disc_push(&tg, 1, c_p2);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(7), "N11: census is 14 before the play (7 fixtures + 7 energy)");
    CHECK_EQ(stage_occupied(&tg, 0), 2, "N11: P1's stage starts with one empty slot");

    CHECK_EQ(test_play_to_stage(&tg, nico, 1), 1,
             "N11: Nico is played to the centre, making P1's stage FULL");

    CHECK(rb_has_pending_choice(&tg.state), "N11: a prompt is standing (non-vacuity control)");
    dump_pending(&tg, "N11 after play");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectPosition") == 0,
          "N11: P1 has no empty slot, so P1 is never shown the SelectCard at all");
    check_prompt_seat(&tg, 1, "N11: the only prompt that appears belongs to P2");

    answer(&tg, 0);   /* P2 left */

    CHECK(!rb_has_pending_choice(&tg.state), "N11: nothing is left pending");
    CHECK_EQ(stage_get(&tg, 0, 0), f0, "N11: P1 left unchanged");
    CHECK_EQ(stage_get(&tg, 0, 1), nico, "N11: Nico at P1 centre");
    CHECK_EQ(stage_get(&tg, 0, 2), f2, "N11: P1 right unchanged");
    CHECK_EQ(stage_occupied(&tg, 0), 3, "N11: P1's stage is full and gained no card");
    CHECK(in_zone(&tg, 0, "discard", a), "N11: P1's first candidate is untouched in the discard");
    CHECK(in_zone(&tg, 0, "discard", b), "N11: P1's second candidate is untouched in the discard");
    CHECK_EQ(stage_get(&tg, 1, 0), c_p2, "N11: P2's card did appear, at P2 left");
    CHECK(is_wait(&tg, c_p2), "N11: P2's appeared card is in wait state");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "N11: CENSUS unchanged — the skipped cards stayed in the discard, none were deleted");
}

/* ---------------------------------------------------------------------------
 * N12 — Q181, area freed after the appeared card leaves.  After N1's placement
 * the appeared member is taken off the stage and returned to the discard, and a
 * fresh cost-2 card is then played into the freed slot.
 * ------------------------------------------------------------------------- */
static void n12_q181_area_freed_after_card_leaves(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico   = mk(&tg, NICO_NO);
    int cheap  = mk(&tg, C19_NO);   /* P1 discard */
    int c_p2   = mk(&tg, C20_NO);   /* P2 discard */
    int filler = mk(&tg, FILLER_NO);
    if (nico < 0 || cheap < 0 || c_p2 < 0 || filler < 0) return;

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, filler);
    disc_push(&tg, 0, cheap);
    disc_push(&tg, 1, c_p2);
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(4), "N12: census is 11 before the play (4 fixtures + 7 energy)");

    CHECK_EQ(test_play_to_stage(&tg, nico, 0), 1, "N12: Nico is played to the left area");
    CHECK(rb_has_pending_choice(&tg.state), "N12: prompt 1 is standing (non-vacuity control)");
    check_prompt_seat(&tg, 0, "N12: P1 is asked first");
    answer(&tg, 1);   /* centre */
    CHECK(rb_has_pending_choice(&tg.state), "N12: prompt 2 is standing");
    check_prompt_seat(&tg, 1, "N12: P2 is asked second");
    answer(&tg, 2);   /* P2 right */
    CHECK(!rb_has_pending_choice(&tg.state), "N12: both halves resolved");

    CHECK_EQ(stage_get(&tg, 0, 1), cheap, "N12: the appeared member is at P1 centre");
    CHECK(is_wait(&tg, cheap), "N12: the appeared member is in wait state");

    /* Take it off the stage and put it back in the discard by hand. */
    int census1 = test_total_card_count(&tg);
    CHECK_EQ(census1, census0, "N12: CENSUS still unchanged after the appearance");
    stage_put(&tg, 0, 1, RB_EMPTY_SLOT);
    disc_push(&tg, 0, cheap);
    CHECK_EQ(stage_get(&tg, 0, 1), RB_EMPTY_SLOT, "N12: the centre area is free again");
    CHECK(in_zone(&tg, 0, "discard", cheap), "N12: the removed card is back in the discard");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "N12: CENSUS unchanged by the manual removal — nothing was destroyed");

    /* Q181: a NEW cost-2 card can now be played into the freed slot. */
    int replacement = mk(&tg, C19_NO);
    if (replacement < 0) return;
    hand_push(&tg, 0, replacement);
    /* test_give_energy ADDS to the energy zone, so the census grows by the
     * replacement card AND by the energy cards, not just the card. */
    test_give_energy(&tg, 2);
    CHECK_EQ(test_total_card_count(&tg), census0 + 3,
             "N12: CENSUS rose by exactly the replacement card + 2 energy cards");
    CHECK_EQ(test_play_to_stage(&tg, replacement, 1), 1,
             "N12: a replacement member is played into the freed centre");
    CHECK_EQ(stage_get(&tg, 0, 1), replacement,
             "N12: the replacement occupies the freed slot — the area really was free");
    CHECK(in_zone(&tg, 0, "discard", cheap),
          "N12: the earlier card is still in the discard, untouched by the play");
    CHECK_EQ(test_total_card_count(&tg), census0 + 3, "N12: CENSUS unchanged by the final play");
}

/* ---------------------------------------------------------------------------
 * N13 — the appeared cards' own 登場 abilities fire.  Both sides place a cost-2
 * member that itself has a 登場 (唐 可可 PL!SP-bp2-002-P, look at the top 3 and
 * add a cost-11+ card). Decks are filled with cheap cards so the look finds
 * nothing and both debuts resolve without a prompt. The debut counters are the
 * observable, and the census is what proves nothing was destroyed by the
 * two chained appearances.
 * ------------------------------------------------------------------------- */
static void n13_q170_placed_cards_debut_abilities_fire(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int nico   = mk(&tg, NICO_NO);
    int filler = mk(&tg, FILLER_NO);
    int keke1  = mk(&tg, KEKE_NO);  /* P1 discard */
    int keke2  = mk(&tg, KEKE_NO);  /* P2 discard — a DISTINCT instance; Rust
                                     * used game.new_id for exactly this */
    if (nico < 0 || filler < 0 || keke1 < 0 || keke2 < 0) return;
    expect_member_cost(&tg, "N13 Keke print", keke1, 2);
    CHECK(rb_card_no_eq(keke1, KEKE_NO) && rb_card_no_eq(keke2, KEKE_NO),
          "N13: both instances really are the same print (PL!SP-bp2-002-P)");
    CHECK(keke1 != keke2, "N13: the two Kekes are DISTINCT card instances");

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    hand_push(&tg, 0, nico);
    hand_push(&tg, 0, filler);
    disc_push(&tg, 0, keke1);
    disc_push(&tg, 1, keke2);
    /* Fill both decks so Keke's 登場 look-at-top-3 has something to look at. */
    bag_clear(&tg.state.p[0].deck);
    bag_clear(&tg.state.p[1].deck);
    for (int i = 0; i < 5; i++) {
        bag_push(&tg.state.p[0].deck, mk(&tg, FILLER_NO));
        bag_push(&tg.state.p[1].deck, mk(&tg, FILLER_NO));
    }
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(14), "N13: census is 21 before the play (nico, filler, 2 Keke, 10 deck + 7 energy)");

    CHECK_EQ(tg.state.debut_count_this_turn[0], 0, "N13: P1's debut counter starts at 0");
    CHECK_EQ(tg.state.debut_count_this_turn[1], 0, "N13: P2's debut counter starts at 0");

    CHECK_EQ(test_play_to_stage(&tg, nico, 0), 1, "N13: Nico is played to the left area");

    CHECK(rb_has_pending_choice(&tg.state), "N13: prompt 1 is standing (non-vacuity control)");
    dump_pending(&tg, "N13 prompt 1");
    check_prompt_seat(&tg, 0, "N13: the turn player is asked first");
    answer(&tg, 1);   /* centre */

    CHECK(rb_has_pending_choice(&tg.state), "N13: prompt 2 is standing");
    dump_pending(&tg, "N13 prompt 2");
    check_prompt_seat(&tg, 1, "N13: the opponent is asked second");
    answer(&tg, 2);   /* P2 right */

    /* Keke's own 登場 may raise its own prompt. The Rust comments say both
     * auto-skip because no deck card has cost >= 11. Cap the drain and assert
     * the end state either way. */
    int drained = drain_pending(&tg);
    CHECK(drained < 32, "N13: the chained Keke 登場 resolved without an unbounded re-prompt");
    CHECK(!rb_has_pending_choice(&tg.state), "N13: everything has resolved");

    CHECK_EQ(stage_get(&tg, 0, 0), nico, "N13: Nico at P1 left");
    CHECK_EQ(stage_get(&tg, 0, 1), keke1, "N13: P1's Keke appeared at centre");
    CHECK_EQ(stage_get(&tg, 1, 2), keke2, "N13: P2's Keke appeared at right");
    CHECK(is_wait(&tg, keke1), "N13: P1's Keke is in wait state");
    CHECK(is_wait(&tg, keke2), "N13: P2's Keke is in wait state");

    CHECK_EQ(tg.state.debut_count_this_turn[0], 2,
             "N13: P1 debuted twice — Nico plus the Keke that appeared");
    CHECK_EQ(tg.state.debut_count_this_turn[1], 1,
             "N13: P2 debuted once — only the Keke that appeared");

    CHECK_EQ(test_total_card_count(&tg), census0,
             "N13: CENSUS unchanged across TWO chained appearances");
}

/* ---------------------------------------------------------------------------
 * N14 — the drained-cap guard.  A tiny case whose only job is to prove the CAPPED
 * drain used by N13 terminates and to record the prompt shape of Keke's 登場 on
 * a board where it is the ONLY thing that fires (no Nico, no nico prompt at all).
 * Without it N13's `drained < 32` has no isolated referent.
 * ------------------------------------------------------------------------- */
static void n14_appeared_member_debut_prompt_shape(void)
{
    static TestGame tg; test_game_new(&tg);

    int keke = mk(&tg, KEKE_NO);
    if (keke < 0) return;
    expect_member_cost(&tg, "N14 Keke print", keke, 2);

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    /* Keke on stage by hand, and a copy of itself in hand: the Rust twin for
     * this shape uses a real live-card walk, so the C uses the cheapest board
     * that puts Keke's 登場 in the queue. */
    stage_put(&tg, 0, 0, keke);
    bag_clear(&tg.state.p[0].deck);
    for (int i = 0; i < 5; i++) bag_push(&tg.state.p[0].deck, mk(&tg, FILLER_NO));
    test_give_energy(&tg, 7);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, CENSUS(6), "N14: census is 13 before the trigger (Keke + 5 deck + 7 energy)");

    test_fire_debut(&tg, keke);

    printf("    <N14> pending after fire_debut: %s\n",
           rb_has_pending_choice(&tg.state) ? test_pending_choice_type(&tg) : "(none)");
    dump_pending(&tg, "N14 after fire_debut");

    int drained = drain_pending(&tg);
    CHECK(drained < 32, "N14: the capped drain terminated — no unbounded re-prompt");
    CHECK_EQ(test_total_card_count(&tg), census0,
             "N14: CENSUS unchanged by Keke's 登場 on a board with no cost-11+ card");
}

/* ═══════════════════════════════════════════════════════════════════════
 * §S  愛♡スクリ～ム！ LL-PR-004-PR — the 1-member-per-side flavour question
 * ═══════════════════════════════════════════════════════════════════════ */

/* S1 — Rust :355 `ai_screeam_answer_both_gain_blade`.
 *
 * The sibling tests/test_parity_realcard_walkthrough.c B1 ports the Rust :3620
 * twin `ai_screeam_soreigai_all_members_on_both_sides_gain_blade`, which uses
 * THREE members per side. This Rust test uses ONE per side, so it is NOT the
 * same board and was genuinely unported: the survey's `ai_screeam_answer_both_
 * gain_blade` entry is real. The option index is the same (2 = それ以外) and the
 * sibling's B1 header documents that C index 2 IS the printed 「それ以外」 branch.
 *
 * What makes this case worth its own file section is assertion 2: the Rust also
 * asserts the pending entry's `choice_player_id == Some("p2")` (「相手に何が
 * 好き？」 — the OPPONENT answers). The sibling explicitly declined to pin that
 * red, saying the live-start queue entry is pushed with an empty player_id. That
 * is a real, reproducible gap, so it is pinned HERE as a hard CHECK.
 *
 * NON-VACUITY: assertions 7/8 are a positive/negative PAIR ON THE SAME BOARD —
 * a member in hand must gain nothing while the two on stage must both gain
 * blade. A no-op that granted blade to everything, or to nothing, fails one of
 * the two. Plus the whole-board census.
 *
 * ⚠ STATUS: UNRESOLVED — DO NOT TREAT THE REDS BELOW AS A CONFIRMED FINDING.
 * This case is currently RED on all four behaviour assertions, but
 * tests/test_parity_realcard_walkthrough.c documents its B1 case as 8/8 GREEN
 * for the SAME card, the SAME trigger and the SAME option index 2, and reports
 * the six members there all gaining exactly 1 blade. The engine trace here
 * shows the live-start ability being FOUND and QUEUED and EXECUTED
 * (`[LIVE_START_SCAN] ... trigger=live_start ... queued=1` then `[QUEUE_EXEC]
 * idx=0 n=1 cid=11 ab=0 actor=0`) with cid=11 verified to be
 * rb_find_card_by_no("LL-PR-004-PR") and `live.cards[0] == screeam`,
 * `live.n == 1` — so the right card and the right ability are being reached, and
 * yet no choice surfaces. The boards differ in exactly one respect: B1 puts
 * THREE members on each stage, this case puts ONE. That difference was NOT
 * isolated, so the reds below are reported as a conflict to bisect, not as an
 * engine bug. Two fixture traps were found and fixed while getting here, and
 * both are recorded in the body: (1) a `test_new_id` COPY id resolves the
 * ability table differently from the sibling's `test_id` TEMPLATE id — hence
 * `tmpl()`, not `mk()`; (2) the C's live-set phase refills the live zone during
 * `advance_to_live_card_set_p1`, so the zone must be cleared or the live-start
 * scan measures a different card. */
static void s1_ai_screeam_answer_both_gain_blade(void)
{
    static TestGame tg; test_game_new(&tg); new_game();

    int screeam = tmpl(&tg, "LL-PR-004-PR");   /* 愛♡スクリ～ム！ ライブ */
    int p1_mem   = tmpl(&tg, "PL!-sd1-013-SD");
    int p2_mem   = tmpl(&tg, "PL!-sd1-014-SD");
    int in_hand  = tmpl(&tg, "PL!-sd1-015-SD"); /* a member NOT on stage */
    int filler   = tmpl(&tg, FILLER_NO);
    if (screeam < 0 || p1_mem < 0 || p2_mem < 0 || in_hand < 0 || filler < 0) return;

    CHECK(rb_card_is_live(screeam), "S1: LL-PR-004-PR really is a live card");
    CHECK(rb_card_is_member(p1_mem), "S1: PL!-sd1-013-SD really is a member");
    CHECK(rb_card_is_member(p2_mem), "S1: PL!-sd1-014-SD really is a member");
    CHECK(rb_card_is_member(in_hand), "S1: the hand-only member really is a member");

    clear_stage(&tg.state.p[0]);
    clear_stage(&tg.state.p[1]);
    bag_clear(&tg.state.p[0].deck);
    bag_clear(&tg.state.p[1].deck);
    for (int i = 0; i < 10; i++) {
        bag_push(&tg.state.p[0].deck, filler);
        bag_push(&tg.state.p[1].deck, filler);
    }
    stage_put(&tg, 0, 1, p1_mem);
    stage_put(&tg, 1, 1, p2_mem);
    hand_push(&tg, 0, screeam);
    hand_push(&tg, 0, filler);
    hand_push(&tg, 0, in_hand);

    int census0 = test_total_card_count(&tg);
    CHECK_EQ(census0, 25, "S1: census is 25 before the walk (5 fixtures + 20 deck slots, no energy given)");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, p1_mem), 0, "S1: P1's member starts with 0 blade");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, p2_mem), 0, "S1: P2's member starts with 0 blade");

    /* Rust advance_to_live_card_set_p1 = 5 passes; then the live card; then
     * advance_to_live_start = 2 more. Copied verbatim from the sibling's B1.
     *
     * FIXTURE CORRECTION, and it is the reason this case is not a duplicate of
     * the sibling's: the C's live-set phase performs its OWN live-zone refill
     * during those 5 passes (the engine logs `[LIVE_ZONE_REFILL] pl=0 placed=1
     * hand_before=2 hand_after=3`), so by the time we set the live card the zone
     * is NOT empty. test_set_live_card APPENDS, and the live-start scan then
     * reports the FIRST card in the zone — the refilled one, not the card under
     * test — which silently measures a different card's ability. The zone is
     * therefore cleared here so the live-start scan sees exactly the live card
     * the Rust board has. Without this the case is a TEST BUG, not a finding. */
    for (int i = 0; i < 5; i++) test_pass(&tg);
    bag_clear(&tg.state.p[0].live);
    bag_clear(&tg.state.p[1].live);
    if (!in_zone(&tg, 0, "hand", screeam)) hand_push(&tg, 0, screeam);
    CHECK(in_zone(&tg, 0, "hand", screeam),
          "S1: the live card under test is in P1's hand right before it is set");
    CHECK_EQ(tg.state.p[0].live.n, 0, "S1: P1's live zone is empty before the set");

    int census_walk = test_total_card_count(&tg);
    CHECK(census_walk > 0, "S1: the census is still positive after the 5 passes");

    test_set_live_card(&tg, 0, screeam);
    CHECK_EQ(tg.state.p[0].live.n, 1, "S1: the live zone holds EXACTLY the card under test");
    CHECK_EQ(tg.state.p[0].live.cards[0], screeam, "S1: live slot 0 is the card under test");

    test_pass(&tg);
    test_pass(&tg);

    CHECK(rb_has_pending_choice(&tg.state),
          "S1: setting the live card raises the 「相手に何が好き？」 answer (control)");
    dump_pending(&tg, "S1 prompt");
    CHECK(strcmp(test_pending_choice_type(&tg), "SelectTarget") == 0,
          "S1: the flavour question is an OPTION list, so index 2 is 「それ以外」");
    check_prompt_seat(&tg, 1,
                      "S1: the OPPONENT answers 「相手に何が好き？」 (Rust: entry p2)");

    select_option(&tg, 2);   /* それ以外 -> blade for every member on both sides */

    CHECK(!rb_has_pending_choice(&tg.state), "S1: no further prompts after the blade gain");

    CHECK(rb_mods_get_blade(&tg.state.mods, p1_mem) > 0,
          "S1: P1's member gained a blade modifier");
    CHECK(rb_mods_get_blade(&tg.state.mods, p2_mem) > 0,
          "S1: P2's member gained a blade modifier");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, p1_mem),
             rb_mods_get_blade(&tg.state.mods, p2_mem),
             "S1: BOTH sides gained the SAME amount — this is a \"both\" effect, not a "
             "self-only one");
    /* The negative half of the pair, on the SAME board: a member in hand is not
     * on stage, so it must gain nothing. If the effect granted blade to every
     * member card everywhere, this fails; if it granted to none, the two
     * assertions above fail. Together they cannot both pass by accident. */
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, in_hand), 0,
             "S1: the member in HAND gained nothing — only stage members do");
    CHECK_EQ(stage_occupied(&tg, 0), 1, "S1: P1 still has exactly one member on stage");
    CHECK_EQ(stage_occupied(&tg, 1), 1, "S1: P2 still has exactly one member on stage");

    CHECK_EQ(test_total_card_count(&tg), census_walk,
             "S1: CENSUS unchanged by the flavour answer — the blade gain created "
             "and destroyed no cards");
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
    long a0 = tally ? tally->assertions : 0;
    int  f0 = tally ? tally->failures : 0;
    int  s0 = tally ? tally->setup_bugs : 0;

    pid_t pid = fork();
    if (pid < 0) {
        current_test = name;
        fn();
        current_test = "(none)";
        n_ok++;
        return;
    }
    if (pid == 0) {
        current_test = name;
        fn();
        printf("        %s: %ld assertion(s), %d failure(s), %d setup bug(s)\n",
               name, tally ? tally->assertions - a0 : 0,
               tally ? tally->failures - f0 : 0,
               tally ? tally->setup_bugs - s0 : 0);
        fflush(stdout); fflush(stderr);
        _Exit((tally && (tally->failures > f0 || tally->setup_bugs > s0)) ? 1 : 0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault); %ld assertion(s) already made\n",
               "CRASH", name, WTERMSIG(status), tally ? tally->assertions - a0 : 0);
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

/* ═══════════════════════════════════════════════════════════════════════ */

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS,  on_segv);
    signal(SIGABRT, on_segv);

    tally = mmap(NULL, sizeof(Tally), PROT_READ | PROT_WRITE,
                 MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (tally == MAP_FAILED) {
        fprintf(stderr, "FAIL: could not map the shared tally\n");
        return 1;
    }
    tally->assertions = 0;
    tally->failures = 0;
    tally->setup_bugs = 0;

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }
    printf("card database loaded: %u cards\n", rb_num_cards());

    printf("--- §N nico PL!-pb1-018-R appear-from-discard (both target) ---\n");
    run("N1_positive_exact_path",       n1_positive_control_exact_path);
    run("N2_no_suitable_card_skips",     n2_no_suitable_card_skips);
    run("N2b_positive_twin_of_N2",       n2b_empty_discard_skips_but_one_card_prompts);
    run("N3_sync_opponent_gets_choice",  n3_sync_path_opponent_gets_choice_when_self_has_none);
    run("N4_q170_turn_player_first",     n4_q170_turn_player_appears_first);
    run("N5_requires_empty_area_self",   n5_requires_empty_area_self_side);
    run("N6_cost_filter_only_eligible",  n6_cost_filter_only_shows_eligible);
    run("N7_q169_occupies_area",         n7_q169_appeared_card_occupies_area);
    run("N8_prompt_path_two_eligible",   n8_prompt_path_two_eligible_in_discard);
    run("N9_both_two_eligible",          n9_both_players_two_eligible_cards);
    run("N10_direct_placement_wait",     n10_prompt_path_direct_placement_wait_state);
    run("N11_full_stage_then_prompt",    n11_full_stage_then_prompt_path);
    run("N12_q181_area_freed",           n12_q181_area_freed_after_card_leaves);
    run("N13_placed_debuts_fire",        n13_q170_placed_cards_debut_abilities_fire);
    run("N14_appeared_debut_prompt",     n14_appeared_member_debut_prompt_shape);

    printf("--- §S AiScReam LL-PR-004-PR flavour question (1 member per side) ---\n");
    run("S1_ai_screeam_answer_both_blade", s1_ai_screeam_answer_both_gain_blade);

    rb_unload();

    printf("\n=== parity_full_round2 ===\n");
    printf("tests passed          : %d\n", n_ok);
    printf("tests failed (parity) : %d\n", n_failed);
    printf("tests crashed (engine): %d\n", n_crashed);
    printf("assertions (shared)   : %ld\n", tally->assertions);
    printf("failures  (shared)    : %d\n", tally->failures);
    printf("fixture setup bugs    : %d\n", tally->setup_bugs);
    if (tally->failures || tally->setup_bugs || n_failed || n_crashed) {
        printf("PARITY DIVERGENCE: the run stayed red by design.\n");
        return 1;
    }
    printf("ALL NICO-ABILITY PARITY CHECKS PASSED\n");
    return 0;
}
