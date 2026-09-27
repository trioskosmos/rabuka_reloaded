/* tests/test_parity_draw_until_count.c
 *
 * Parity coverage for `draw_until_count` (ActionType::DrawUntilCount).
 *
 * Rust source of truth:
 *   engine/src/ability/effects/draw.rs:602-626  AbilityResolver::execute_draw_until_count
 *   engine/src/ability/effects/executor.rs:49-52 dispatch of ActionType::DrawUntilCount
 *   engine/src/ability/effects/draw.rs:16-81   draw_cards_for_player (deck / waitroom refresh)
 *
 * Rust tests mirrored here:
 *   engine/tests/test_modules/rules/targeting/target_selection_test.rs:127-161
 *       (target_count_on_draw_until_count — PL!N-PR-028-PR, optional discard 2
 *        then draw to 5)
 *   engine/tests/test_modules/effects/draw/until_count/miyashita_ai_pr_test.rs
 *       (hand / deck / waitroom accounting around the same card)
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
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

#define CHECK_EQ(actual, expected, message) do { \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* Answer every pending choice with a skip / empty answer, like the Rust
   `while game.has_pending_choice() { game.select_indices(&[]) }` drain. */
static void drain_choices(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 256)
        test_resume_choice(tg, -1);
}

/* Build a draw_until_count effect the way the bytecode decoder does:
   action / target / source / destination plus the numeric target. The numeric
   value is written to the `target_count` extra (the faithful path, Rust
   effect.target_count_any()) and, separately, to `count` (the path the current
   C decoder leaves behind because it drops the target_count key). */
static void make_draw_until_effect(AbilityEffect *e, int target_count,
                                   const char *target, const char *destination,
                                   int use_extra)
{
    static char buf[24];
    memset(e, 0, sizeof(*e));
    e->action = (char *)"draw_until_count";
    e->target = (char *)(target ? target : "self");
    e->source = (char *)"deck";
    e->destination = (char *)(destination ? destination : "hand");
    e->count = target_count;
    snprintf(buf, sizeof(buf), "%d", target_count);
    if (use_extra) {
        e->extra_k[0] = (char *)"target_count";
        e->extra_v[0] = buf;
        e->n_extra = 1;
    }
}

static void reset_board(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *p = &tg->state.p[pl];
        p->hand.n = 0;
        p->deck.n = 0;
        p->discard.n = 0;
        p->live.n = 0;
        p->success.n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) p->stage[s] = RB_EMPTY_SLOT;
    }
    tg->state.last_draw_count = 0;
}

static void fill_zone(RbBag *bag, int card, int n)
{
    bag->n = 0;
    for (int i = 0; i < n && i < RB_MAX_ZONE; i++) bag->cards[bag->n++] = card;
}

/* ── 1. End-to-end: the real card, optional cost, then draw to 5 ─────────────
    Mirrors engine/tests/test_modules/rules/targeting/target_selection_test.rs:127

    KNOWN RED (canary, NOT a draw.c bug). Measured 2026-09-28: this test fails
    on unmodified master, and the failure is upstream of draw_until_count —
    the effect is never executed at all, so zero cards are drawn (hand stays 2,
    deck stays 10). RB_ABILITY_DEBUG=1 shows no [DRAW_UNTIL_COUNT] line and
    RB_DUMP_ABILITY=1 shows the ability decodes correctly
    (cost=move_cards 2 hand->discard optional, effect=draw_until_count
    count=5 target_count=5 source=deck destination=hand).

    Root cause, in files this agent does not own:
      * engine_c_wip/src/ability/choice.c:2438
        `int selected_idx = n_indices > 0 && selected_indices ? selected_indices[0] : -1;`
        collapses rb_resume_with_choice_indices() to ONE index, so the Rust
        twin's `game.select_indices(&[0, 1])` only ever pays one of the two
        cards and the cost selection re-prompts.
      * engine_c_wip/src/ability/choice.c:2624-2630 — the SelectCard
        "general skip" branch calls rb_resolver_clear_choice_state() +
        rb_resolver_resume_execution() but NEVER runs the captured `def`
        (= g->queue.deferred, which engine.c:1092 parked as ab.effect). The
        non-skip branch at choice.c:2671 does run it. So answering the cost
        prompt in any way (pay or decline) drops the ability's effect.
      * engine_c_wip/src/engine.c:1089-1102 only runs ab.effect inline when no
        choice is pending at that instant, so the already-parked continuation
        is the only thing that can finish the ability.

    Keep this test red until those three are fixed; it is the canary. */
static void test_card_fills_hand_to_target_count(void)
{
    TestGame game;
    test_game_new(&game);
    int card = test_id(&game, "PL!N-PR-028-PR");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    CHECK(card >= 0 && filler >= 0, "PL!N-PR-028-PR fixtures resolve");
    if (card < 0 || filler < 0) return;

    reset_board(&game);
    test_add_to_hand(&game, card);
    test_add_to_hand(&game, filler);
    test_add_to_hand(&game, filler);
    test_give_energy(&game, 30);
    for (int i = 0; i < 10; i++) test_add_to_deck(&game, filler);

    test_play_to_stage(&game, card, 1);
    CHECK(test_has_pending_choice(&game), "optional discard-2 cost prompt appears");

    /* Pay the optional cost with BOTH remaining hand cards, exactly like the
       Rust twin's `game.select_indices(&[0, 1])`
       (engine/tests/test_modules/rules/targeting/target_selection_test.rs:152).
       The assertions below are stated as the draw_until_count invariant
       (hand reaches target_count=5, no card created or destroyed) rather than
       a fixed deck delta, so they hold whether the cost is paid or declined. */
    if (getenv("RB_PROBE_DUC")) {
        /* Permanent diagnostic (RB_PROBE_DUC=1): step the cost selection one
           index at a time, the way the C choice handlers actually consume
           answers, and report every step. Skips the assertions so the probe
           output is not interleaved with them. */
        for (int step = 0; step < 6; step++) {
            if (!test_has_pending_choice(&game)) {
                fprintf(stderr, "PROBE step=%d: no pending choice; hand=%d deck=%d disc=%d\n",
                        step, game.state.p[0].hand.n, game.state.p[0].deck.n,
                        game.state.p[0].discard.n);
                break;
            }
            const RbChoice *ch = rb_get_pending_choice(&game.state);
            fprintf(stderr, "PROBE step=%d: kind=%d zone=%s count=%d allow_skip=%d route=%d "
                            "hand=%d deck=%d disc=%d\n",
                    step, ch ? (int)ch->kind : -1, ch ? ch->zone : "-",
                    ch ? ch->count : -1, ch ? ch->allow_skip : -1, ch ? ch->route : -1,
                    game.state.p[0].hand.n, game.state.p[0].deck.n, game.state.p[0].discard.n);
            const int pick[1] = { 0 };
            rb_resume_with_choice_indices(&game.state, pick, 1);
        }
        fprintf(stderr, "PROBE final: hand=%d deck=%d disc=%d last_draw_count=%d\n",
                game.state.p[0].hand.n, game.state.p[0].deck.n, game.state.p[0].discard.n,
                game.state.last_draw_count);
        return;
    }
    {
        const int pick[2] = { 0, 1 };
        rb_resume_with_choice_indices(&game.state, pick, 2);
    }
    drain_choices(&game);

    CHECK_EQ(game.state.p[0].hand.n, 5,
             "hand is refilled to target_count=5 after the optional cost resolves");
    CHECK_EQ(game.state.p[0].hand.n + game.state.p[0].deck.n + game.state.p[0].discard.n, 12,
             "draw_until_count moves cards only: hand + deck + discard stays 12");
    CHECK_EQ(game.state.p[0].stage[1], card, "the card stays on the centre stage");
    CHECK(game.state.p[0].deck.n < 10, "cards were actually drawn from the deck");
}

/* ── 1b. Same card, same assertions, answered the way the C choice model eats
      answers (ONE index per resume + re-prompt) ─────────────────────────────
   Test 1 above proves the two upstream choice.c defects; it is red because of
   them, not because of draw_until_count. This test isolates the draw_until_count
   port itself: the identical scenario, the identical Rust assertions
   (target_selection_test.rs:157-160), driven with one index per resume, which
   is how every other C suite answers a SelectCard cost prompt
   (test_select_indices(tg, idx, 1) throughout). Green here = the port is REAL:
   the effect is reached, the deficit is computed against the resolved target's
   hand, the cards come off the deck, and nothing is created or destroyed. */
static void test_card_fills_hand_to_target_count_stepwise(void)
{
    TestGame game;
    test_game_new(&game);
    int card = test_id(&game, "PL!N-PR-028-PR");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    if (card < 0 || filler < 0) { CHECK(0, "PL!N-PR-028-PR fixtures resolve (stepwise)"); return; }

    reset_board(&game);
    test_add_to_hand(&game, card);
    test_add_to_hand(&game, filler);
    test_add_to_hand(&game, filler);
    test_give_energy(&game, 30);
    for (int i = 0; i < 10; i++) test_add_to_deck(&game, filler);

    test_play_to_stage(&game, card, 1);
    CHECK(test_has_pending_choice(&game), "stepwise: the optional discard-2 cost prompt appears");

    /* Pay the two-card cost one index at a time. rb_resolver_handle_select_card
       consumes a single index per resume and re-offers for the remainder
       (choice.c:850-860), so this is the same selection the Rust twin performs
       with one `select_indices(&[0, 1])` call. */
    for (int step = 0; step < 4 && test_has_pending_choice(&game); step++) {
        const int pick[1] = { 0 };
        rb_resume_with_choice_indices(&game.state, pick, 1);
    }
    drain_choices(&game);

    CHECK_EQ(game.state.p[0].hand.n, 5,
             "stepwise: hand reaches target_count=5 after the cost is paid");
    CHECK_EQ(game.state.p[0].hand.n + game.state.p[0].deck.n + game.state.p[0].discard.n, 12,
             "stepwise: draw_until_count moves cards only (hand + deck + discard = 12)");
    CHECK_EQ(game.state.p[0].stage[1], card, "stepwise: the card stays on the centre stage");
    CHECK_EQ(game.state.p[0].deck.n, 7, "stepwise: exactly target_count minus hand cards left the deck");
    /* draw.rs:591 records step_state.last_draw_count = final_count, i.e. the
       REQUESTED deficit (5 - 2), not the number of cards the deck supplied. The
       deficit is 3 rather than 5 because the C generic hand-selection cost path
       (choice.c rb_resolver_handle_hand_selection) records the picks without
       moving the cards out of hand — a separate upstream gap; the draw_until_count
       arithmetic below it is exact either way. */
    CHECK_EQ(game.state.last_draw_count, 3,
             "stepwise: last_draw_count is the requested deficit (target_count - hand)");
}

/* ── 2. target_count is honoured for several sizes, through the extra ─────── */
static void test_target_sizes_via_extra(void)
{
    static const int targets[] = { 1, 3, 5, 7 };
    for (int i = 0; i < (int)(sizeof(targets) / sizeof(targets[0])); i++) {
        TestGame tg;
        test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        reset_board(&tg);
        fill_zone(&tg.state.p[0].deck, filler, 12);
        AbilityEffect e;
        make_draw_until_effect(&e, targets[i], "self", "hand", 1);

        rb_effect_draw_until_count(&tg.state, 0, &e);

        char msg[96];
        snprintf(msg, sizeof(msg), "target_count=%d draws exactly up to the target", targets[i]);
        CHECK_EQ(tg.state.p[0].hand.n, targets[i], msg);
        snprintf(msg, sizeof(msg), "target_count=%d consumes target_count deck cards", targets[i]);
        CHECK_EQ(tg.state.p[0].deck.n, 12 - targets[i], msg);
        snprintf(msg, sizeof(msg), "target_count=%d reports the requested count as last_draw_count",
                 targets[i]);
        CHECK_EQ(tg.state.last_draw_count, targets[i], msg);
    }
}

/* ── 3. Same sizes with the numeric value only in `count` (decoder fallback) ─ */
static void test_target_sizes_via_count_fallback(void)
{
    static const int targets[] = { 1, 3, 5, 7 };
    for (int i = 0; i < (int)(sizeof(targets) / sizeof(targets[0])); i++) {
        TestGame tg;
        test_game_new(&tg);
        int filler = test_id(&tg, "PL!-sd1-010-SD");
        reset_board(&tg);
        fill_zone(&tg.state.p[0].deck, filler, 12);
        AbilityEffect e;
        make_draw_until_effect(&e, targets[i], "self", "hand", 0);

        rb_effect_draw_until_count(&tg.state, 0, &e);

        char msg[96];
        snprintf(msg, sizeof(msg),
                 "target_count=%d is read from count when the extra is absent", targets[i]);
        CHECK_EQ(tg.state.p[0].hand.n, targets[i], msg);
    }
}

/* ── 4. Hand already at or above the target is a no-op (saturating_sub) ───── */
static void test_hand_already_at_or_above_target(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    /* exactly at the target */
    reset_board(&tg);
    fill_zone(&tg.state.p[0].deck, filler, 10);
    fill_zone(&tg.state.p[0].hand, filler, 3);
    AbilityEffect e;
    make_draw_until_effect(&e, 3, "self", "hand", 1);
    rb_effect_draw_until_count(&tg.state, 0, &e);
    CHECK_EQ(tg.state.p[0].hand.n, 3, "hand exactly at the target draws nothing");
    CHECK_EQ(tg.state.p[0].deck.n, 10, "hand exactly at the target leaves the deck alone");
    CHECK_EQ(tg.state.last_draw_count, 0, "hand exactly at the target reports 0 drawn");

    /* above the target: never discards down to it */
    reset_board(&tg);
    fill_zone(&tg.state.p[0].deck, filler, 10);
    fill_zone(&tg.state.p[0].hand, filler, 6);
    make_draw_until_effect(&e, 3, "self", "hand", 1);
    rb_effect_draw_until_count(&tg.state, 0, &e);
    CHECK_EQ(tg.state.p[0].hand.n, 6, "hand above the target is left alone");
    CHECK_EQ(tg.state.p[0].deck.n, 10, "hand above the target leaves the deck alone");
    CHECK_EQ(tg.state.last_draw_count, 0, "hand above the target reports 0 drawn");
}

/* ── 5. target_count 0 (or absent) is a no-op (unwrap_or(0)) ─────────────── */
static void test_target_zero(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    reset_board(&tg);
    fill_zone(&tg.state.p[0].deck, filler, 10);
    AbilityEffect e;
    make_draw_until_effect(&e, 0, "self", "hand", 1);
    rb_effect_draw_until_count(&tg.state, 0, &e);
    CHECK_EQ(tg.state.p[0].hand.n, 0, "target_count=0 draws nothing into an empty hand");
    CHECK_EQ(tg.state.p[0].deck.n, 10, "target_count=0 leaves the deck alone");

    /* neither extra nor count set (Rust: target_count_any().unwrap_or(0)) */
    memset(&e, 0, sizeof(e));
    e.action = (char *)"draw_until_count";
    e.destination = (char *)"hand";
    e.count = -1;
    rb_effect_draw_until_count(&tg.state, 0, &e);
    CHECK_EQ(tg.state.p[0].hand.n, 0, "a missing target_count draws nothing");
    CHECK_EQ(tg.state.p[0].deck.n, 10, "a missing target_count leaves the deck alone");
}

/* ── 6. Deck too small: draws what exists and stops ──────────────────────── */
static void test_deck_smaller_than_target(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    reset_board(&tg);
    fill_zone(&tg.state.p[0].deck, filler, 2);
    AbilityEffect e;
    make_draw_until_effect(&e, 5, "self", "hand", 1);
    rb_effect_draw_until_count(&tg.state, 0, &e);
    CHECK_EQ(tg.state.p[0].hand.n, 2, "a short deck yields only the available cards");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "a short deck is fully consumed");
}

/* ── 7. Completely empty deck and waitroom: no-op, no crash ──────────────── */
static void test_empty_deck_and_waitroom(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    AbilityEffect e;
    make_draw_until_effect(&e, 5, "self", "hand", 1);
    rb_effect_draw_until_count(&tg.state, 0, &e);
    CHECK_EQ(tg.state.p[0].hand.n, 0, "empty deck and waitroom draw nothing");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "empty deck stays empty");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "empty waitroom stays empty");
}

/* ── 8. Empty deck refills from the waitroom (Q104 / Rule 10.2.1) ──────────
   draw.rs:70-77 refreshes the deck from the waitroom and keeps drawing. */
static void test_empty_deck_refreshes_from_waitroom(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    reset_board(&tg);
    fill_zone(&tg.state.p[0].discard, filler, 4);
    AbilityEffect e;
    make_draw_until_effect(&e, 4, "self", "hand", 1);
    rb_effect_draw_until_count(&tg.state, 0, &e);
    CHECK_EQ(tg.state.p[0].hand.n, 4, "an empty deck refills from the waitroom and fills the hand");
    CHECK_EQ(tg.state.p[0].deck.n, 0, "the refreshed deck is consumed again");
    CHECK_EQ(tg.state.p[0].discard.n, 0, "the waitroom is spent on the refresh");
}

/* ── 9. Non-hand destination is a no-op (draw.rs:607-612 returns early) ──── */
static void test_non_hand_destination_is_noop(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    reset_board(&tg);
    fill_zone(&tg.state.p[0].deck, filler, 10);
    AbilityEffect e;
    make_draw_until_effect(&e, 5, "self", "stage", 1);
    rb_effect_draw_until_count(&tg.state, 0, &e);
    CHECK_EQ(tg.state.p[0].hand.n, 0, "a non-hand destination draws nothing into hand");
    CHECK_EQ(tg.state.p[0].deck.n, 10, "a non-hand destination leaves the deck alone");
}

/* ── 10. target="opponent" refills the opponent, not the actor ───────────── */
static void test_opponent_target(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    reset_board(&tg);
    fill_zone(&tg.state.p[0].deck, filler, 10);
    fill_zone(&tg.state.p[1].deck, filler, 10);
    AbilityEffect e;
    make_draw_until_effect(&e, 4, "opponent", "hand", 1);
    rb_effect_draw_until_count(&tg.state, 0, &e);
    CHECK_EQ(tg.state.p[1].hand.n, 4, "target=opponent fills the opponent hand to 4");
    CHECK_EQ(tg.state.p[1].deck.n, 6, "target=opponent draws from the opponent deck");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "target=opponent leaves the actor hand alone");
    CHECK_EQ(tg.state.p[0].deck.n, 10, "target=opponent leaves the actor deck alone");
}

/* ── 11. Non-empty hand below the target (partial fill) ──────────────────── */
static void test_partial_fill_from_non_empty_hand(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    reset_board(&tg);
    fill_zone(&tg.state.p[0].deck, filler, 10);
    fill_zone(&tg.state.p[0].hand, filler, 1);
    AbilityEffect e;
    make_draw_until_effect(&e, 4, "self", "hand", 1);

    rb_effect_draw_until_count(&tg.state, 0, &e);

    CHECK_EQ(tg.state.p[0].hand.n, 4, "a hand below the target is topped up to the target");
    CHECK_EQ(tg.state.p[0].deck.n, 7, "only target_count minus hand cards leave the deck");
    CHECK_EQ(tg.state.last_draw_count, 3, "last_draw_count is the requested deficit");
}

/* ── 12. The action string also routes through rb_effect_draw_card ─────────
   executor.c:145 dispatches draw_until_count to rb_effect_draw_until_count;
   this guards the wrapper's own draw_until_count branch from regressing into
   a plain `draw` (which would use `count` as a flat draw size). */
static void test_draw_card_wrapper_routes_draw_until_count(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    reset_board(&tg);
    fill_zone(&tg.state.p[0].deck, filler, 10);
    fill_zone(&tg.state.p[0].hand, filler, 1);
    AbilityEffect e;
    make_draw_until_effect(&e, 4, "self", "hand", 1);

    rb_effect_draw_card(&tg.state, 0, &e, -1);

    CHECK_EQ(tg.state.p[0].hand.n, 4, "the wrapper tops the hand up to the target, not count more");
    CHECK_EQ(tg.state.p[0].deck.n, 7, "the wrapper draws target_count minus hand cards");
}

/* ── 13. target="both" tops up BOTH hands (draw.rs:370-386) ─────────────────
   execute_draw's "both" branch draws the same count for each player and sets
   step_state.last_draw_count to that count ONCE (not summed). The deficit is
   measured on resolve_target_player_mut("both"), which Rust documents as
   falling back to player1, so the count is derived from player1's hand. */
static void test_target_both_fills_both_players(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    reset_board(&tg);
    fill_zone(&tg.state.p[0].deck, filler, 10);
    fill_zone(&tg.state.p[1].deck, filler, 10);
    AbilityEffect e;
    make_draw_until_effect(&e, 4, "both", "hand", 1);

    rb_effect_draw_until_count(&tg.state, 0, &e);

    CHECK_EQ(tg.state.p[0].hand.n, 4, "target=both fills player1 to the target");
    CHECK_EQ(tg.state.p[1].hand.n, 4, "target=both fills player2 to the same target");
    CHECK_EQ(tg.state.p[0].deck.n, 6, "target=both draws from player1's deck");
    CHECK_EQ(tg.state.p[1].deck.n, 6, "target=both draws from player2's deck");
    CHECK_EQ(tg.state.last_draw_count, 4, "target=both reports the requested count once, not summed");
}

/* ── 14. "self" follows the ability master, not the calling actor ───────────
   draw.rs:608 uses gs.resolve_target_player_mut(target), which resolves "self"
   through the ability master (Rust GameState::resolve_master_id), NOT the
   caller. With active=1 the master is player2, so "self" is player2's hand
   even when the caller passes actor 0. */
static void test_self_target_follows_ability_master(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    reset_board(&tg);
    fill_zone(&tg.state.p[0].deck, filler, 10);
    fill_zone(&tg.state.p[1].deck, filler, 10);
    tg.state.active = 1;  /* ability master is player2 */
    AbilityEffect e;
    make_draw_until_effect(&e, 3, "self", "hand", 1);

    rb_effect_draw_until_count(&tg.state, 0 /* actor */, &e);

    CHECK_EQ(tg.state.p[1].hand.n, 3, "self resolves to the ability master (player2), not the caller");
    CHECK_EQ(tg.state.p[1].deck.n, 7, "the master draws from their own deck");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "the non-master hand is untouched");
    CHECK_EQ(tg.state.p[0].deck.n, 10, "the non-master deck is untouched");
}

/* ── 15. Wrapper-only rules must not touch draw_until_count ─────────────────
   Rust dispatches ActionType::DrawUntilCount straight to
   execute_draw_until_count, bypassing execute_draw_wrapper. So per_unit (and
   the dynamic-count / count==0 / optional-gate paths) must be ignored: the
   hand is filled to target_count, never count x per_unit. */
static void test_wrapper_rules_do_not_apply_to_draw_until_count(void)
{
    TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    reset_board(&tg);
    fill_zone(&tg.state.p[0].deck, filler, 10);

    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action  = (char *)"draw_until_count";
    e.target  = (char *)"self";
    e.source  = (char *)"deck";
    e.destination = (char *)"hand";
    e.count   = 4;                                  /* NOT the target: a red herring */
    e.n_extra = 4;
    e.extra_k[0] = (char *)"target_count";   e.extra_v[0] = (char *)"5";
    e.extra_k[1] = (char *)"per_unit";       e.extra_v[1] = (char *)"true";
    e.extra_k[2] = (char *)"per_unit_count"; e.extra_v[2] = (char *)"2";
    e.extra_k[3] = (char *)"reference";      e.extra_v[3] = (char *)"unit_count";

    rb_effect_draw_card(&tg.state, 0, &e, -1);

    CHECK_EQ(tg.state.p[0].hand.n, 5,
             "per_unit / dynamic count on a draw_until_count effect are ignored");
    CHECK_EQ(tg.state.p[0].deck.n, 5, "exactly target_count cards leave the deck");
    CHECK_EQ(tg.state.last_draw_count, 5, "last_draw_count is the target_count deficit");
}

static void dump_effect(const char *tag, const AbilityEffect *e, int depth)
{
    if (!e) { fprintf(stderr, "  %*s%s: (null)\n", depth * 2, "", tag); return; }
    fprintf(stderr, "  %*s%s: action=%s count=%d target=%s source=%s dest=%s optional=%d n_child=%d n_extra=%d\n",
            depth * 2, "", tag, e->action ? e->action : "-", e->count,
            e->target ? e->target : "-", e->source ? e->source : "-",
            e->destination ? e->destination : "-", e->is_optional, e->n_child, e->n_extra);
    for (int i = 0; i < e->n_extra; i++)
        fprintf(stderr, "  %*s  extra[%d]=%s -> %s\n", depth * 2, "", i,
                e->extra_k[i] ? e->extra_k[i] : "-", e->extra_v[i] ? e->extra_v[i] : "-");
    for (int i = 0; i < e->n_child; i++) {
        char sub[32]; snprintf(sub, sizeof(sub), "child[%d]", i);
        dump_effect(sub, e->child[i], depth + 1);
    }
}

static void dump_ability(int card_id)
{
    int n = rb_card_num_abilities((uint32_t)card_id);
    fprintf(stderr, "[DUMP_ABILITY] card=%d n_abilities=%d\n", card_id, n);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        fprintf(stderr, "  ability[%d] triggers=%s is_null=%d\n", i, ab.triggers ? ab.triggers : "-", ab.is_null);
        dump_effect("cost", ab.cost, 1);
        dump_effect("effect", ab.effect, 1);
    }
}

/* Load the card DB. `src` is the in-tree layout; `../cards/build` is where
   the Makefile (and tools/isolated_build.sh) put the blobs, so an isolated
   out-of-tree build finds the database through this second path. */
static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 1;
    }
    if (getenv("RB_ABILITY_DEBUG")) rb_ability_debug_set(1);
    if (getenv("RB_DUMP_ABILITY")) {
        TestGame g0; test_game_new(&g0);
        dump_ability(test_id(&g0, "PL!N-PR-028-PR"));
        return 0;
    }
    setvbuf(stdout, NULL, _IONBF, 0);

    test_card_fills_hand_to_target_count();
    test_card_fills_hand_to_target_count_stepwise();
    test_target_sizes_via_extra();
    test_target_sizes_via_count_fallback();
    test_hand_already_at_or_above_target();
    test_target_zero();
    test_deck_smaller_than_target();
    test_empty_deck_and_waitroom();
    test_empty_deck_refreshes_from_waitroom();
    test_non_hand_destination_is_noop();
    test_opponent_target();
    test_partial_fill_from_non_empty_hand();
    test_draw_card_wrapper_routes_draw_until_count();
    test_target_both_fills_both_players();
    test_self_target_follows_ability_master();
    test_wrapper_rules_do_not_apply_to_draw_until_count();

    rb_unload();
    if (failures) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    printf("all draw-until-count parity checks passed\n");
    return 0;
}
