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
   Mirrors engine/tests/test_modules/rules/targeting/target_selection_test.rs:127 */
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

    /* Answer the optional cost, then drain any follow-up choice. Whether the
       optional discard is paid or skipped changes how many cards the deck has
       to give, so the assertions below are stated as the draw_until_count
       invariant (hand reaches target_count=5, no card created or destroyed)
       rather than a fixed deck delta. */
    test_resume_choice(&game, 0);
    drain_choices(&game);
    if (test_has_pending_choice(&game)) {
        test_resume_choice(&game, 1);
        drain_choices(&game);
    }

    CHECK_EQ(game.state.p[0].hand.n, 5,
             "hand is refilled to target_count=5 after the optional cost resolves");
    CHECK_EQ(game.state.p[0].hand.n + game.state.p[0].deck.n + game.state.p[0].discard.n, 12,
             "draw_until_count moves cards only: hand + deck + discard stays 12");
    CHECK_EQ(game.state.p[0].stage[1], card, "the card stays on the centre stage");
    CHECK(game.state.p[0].deck.n < 10, "cards were actually drawn from the deck");
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

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 1;
    }
    setvbuf(stdout, NULL, _IONBF, 0);

    test_card_fills_hand_to_target_count();
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

    rb_unload();
    if (failures) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    printf("all draw-until-count parity checks passed\n");
    return 0;
}
