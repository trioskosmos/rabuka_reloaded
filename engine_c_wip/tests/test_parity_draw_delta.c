/* tests/test_parity_draw_delta.c
 *
 * The DELTA of the Rust draw clusters -- the test functions that have no
 * equivalent anywhere in the C suite.
 *
 * Scope (from the assignment):
 *   engine/tests/test_modules/effects/draw/conditional/  -- 11 #[test]
 *   engine/tests/test_modules/effects/draw/until_count/  -- 10 #[test]
 *   engine/tests/test_modules/effects/deck_order/       -- restore/ is a peer's
 *
 * WHAT IS ALREADY TRANSLATED, AND THEREFORE NOT REPEATED HERE
 *
 *   effects/draw/until_count/  -- test_parity_draw_until_count.c (16 fns) covers
 *     the engine-level surface: target_count sizes, saturating_sub, the short
 *     deck, the waitroom refresh, target=self/opponent/both, the wrapper's
 *     routing. test_parity_small_clusters.c SECTION D covers 7 of the 10 Rust
 *     card-level fns: miyashita_ai_pr_test.rs
 *       pr_ai_pays_cost_discards_2_draws_to_five   (draw_until_count.c:115)
 *       pr_ai_pays_cost_discards_2_draws_4         (draw_until_count.c:115/190)
 *       pr_ai_hand_almost_full_skips_cost_no_effect (small_clusters:3066)
 *       pr_ai_hand_at_seven_skips_cost_no_effect    (small_clusters:3099)
 *       pr_ai_empty_deck_no_draw_no_hang            (small_clusters:3126)
 *     and miyata_draw_until_test.rs, all four
 *       third_appearance_draws_to_five / hand_at_five_or_more_draws_nothing /
 *       two_appearances_no_draw / small_deck_draws_out_and_resolves
 *                                       (small_clusters:3146/3189/3219/3247)
 *
 *   effects/draw/conditional/ -- test_parity_small_clusters.c SECTION E covers
 *     all 11 fns by count (E1 x3, E2 x1, E3 x3, E4 x2, E5 x2).
 *
 *   effects/draw/flat/ and effects/draw/chains/ -- a peer's
 *     (test_parity_gain_score_draw_clusters.c).
 *   effects/deck_order/restore/ -- a peer's (test_parity_look_energy_position.c).
 *   deck_order/ has no subcluster outside restore/, so it contributes 0 here.
 *
 * WHAT THIS FILE ADDS -- the genuine gaps
 *
 *   A1  pr_ai_hand_4_pays_cost_draws_4 (miyashita_ai_pr_test.rs:238-267).
 *       The 10th Rust fn of until_count/, and the ONLY one with no C
 *       counterpart anywhere. SECTION D exercises hand_extra=3 and hand_extra=6
 *       but only ever SKIPS the cost; the Rust suite also pays it from a 4-card
 *       hand and measures a 4-card draw. Distinct arithmetic from every D case.
 *
 *   A2  The CARD-IDENTITY half of pr_ai_pays_cost_discards_2_draws_4
 *       (miyashita_ai_pr_test.rs:146-161): the two cards that were in hand
 *       before the play must be the two that reached the waitroom, and Ai must
 *       be in NEITHER hand nor waitroom. No C suite asserts this -- the existing
 *       conservation check (hand + deck + discard == 12) is far weaker and would
 *       pass with the wrong cards discarded. Per AGENTS.md the deck is stocked
 *       with DISTINCT card instances here, so "the right cards were discarded"
 *       is a real assertion rather than an alias of one shared template id.
 *
 *   A3  The waitroom DELTA for pr_ai_pays_cost_discards_2_draws_to_five
 *       (miyashita_ai_pr_test.rs:78-82, waitroom == discard_before + 2). The
 *       existing suite proves nothing was created or destroyed; it never proves
 *       the optional cost was actually paid.
 *
 *   A4  DECK ORDER after a draw_until_count, asserted explicitly. The suite
 *       convention is TOP-INDEX-0 (insert(0, x) == put_on_deck_top,
 *       push(x) == append), and draw_cards_for_player takes from index 0
 *       (draw.c:52-57) shifting the rest down. With 10 DISTINCT deck instances
 *       this asserts hand == [d0 d1 d2 d3] and deck == [d4 .. d9] in that
 *       order, so an implementation that draws from the bottom or reverses the
 *       bag fails loudly instead of passing on a count alone.
 *
 *   B1  e1_mus_live_in_success_zone, with the card the Rust twin ACTUALLY
 *       names. success_zone_group_pl_bp6_023_l_test.rs:14 uses
 *       `game.id("PL!-sd1-020-SD")` -- きっと青春が聞こえる, type ライブ,
 *       series ラブライブ！, i.e. a genuine μ's LIVE card. The existing C port
 *       (test_parity_small_clusters.c:3288-3309) substituted PL!-sd1-010-SD,
 *       which cards.json types as メンバー (高坂穂乃果, Printemps, cost 4), and
 *       then asserted that substitution was a "RUST FIXTURE BUG". That claim is
 *       wrong: the Rust file is correct and the C transcription is not. So the
 *       positive arm -- a real μ's LIVE card in the success zone -> draw 1 + 1
 *       -- has never actually been run. This file runs it, plus a zone control
 *       (the same card in HAND instead) that separates "wrong zone" from
 *       "wrong group".
 *
 *   B2  The DRAW_UNTIL_COUNT destination matrix. deck.rs:440-445 reads
 *         match Zone::from_str(destination) { Some(Hand) => hand.len(), _ => return }
 *       so EVERY non-hand destination is a silent no-op -- and draw.c:850-854
 *       does the same. The existing suite proves it for "stage" only
 *       (test_parity_draw_until_count.c:387). This file walks every wire
 *       spelling rb_zone_of_str accepts, plus an unrecognised string, so a
 *       destination that starts doing something is caught.
 *
 * CONVENTIONS HONOURED HERE
 *   * sizeof(GameState) is ~781 KB: every TestGame is `static`.
 *   * Card identity is pinned with a byte-exact card_no compare. rb_card_no_eq
 *     is exact; test_id / rb_find_card_by_no is deliberately forgiving
 *     (substring fallback) and must not be trusted where the PRINT is the point
 *     (bp2 vs pb2 are different cards).
 *   * Cards tracked by identity come from test_new_id, never test_id.
 *   * A draw test asserts the exact number of cards that left the deck, and the
 *     stock deck is counted before the draw so a vacuous pass is impossible.
 *   * Non-vacuity: every negative arm is paired with a CONTROL that proves the
 *     same ability DOES act on a board where its condition holds.
 *   * All Japanese literals are \xNN escapes so the file is pure ASCII.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;
static int assertions;

/* Trigger wire tokens (engine/src/ability/enums.rs), byte-exact UTF-8. */
#define TRIG_LIVE_SUCCESS \
    "\xE3\x83\xA9\xE3\x82\xA4\xE3\x83\x96\xE6\x88\x90\xE5\x8A\x9F\xE6\x99\x82"

/* ══════════════════════════════════════════════════════════════════════════
 * harness
 * ══════════════════════════════════════════════════════════════════════════ */

#define CHECK(cond, msg) do { \
    assertions++; \
    if (!(cond)) { \
        fprintf(stderr, "FAIL: %s\n", msg); \
        failures++; \
    } else { \
        printf("ok: %s\n", msg); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, msg) do { \
    assertions++; \
    int a_ = (actual), e_ = (expected); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", msg, a_, e_); \
        failures++; \
    } else { \
        printf("ok: %s\n", msg); \
    } \
} while (0)

/* Byte-exact card_no pin. test_id / rb_find_card_by_no falls back to a SUBSTRING
 * search (data.c:446), so "PL!SP-pb2-004-R" happily resolves to some other real
 * card; where the print itself is under test that is a silent wrong fixture. */
static int pin(int cid, const char *want)
{
    assertions++;
    if (!rb_card_no_eq(cid, want)) {
        fprintf(stderr, "FAIL: identity: want card_no '%s', got id %d\n", want, cid);
        failures++;
        return 0;
    }
    printf("ok: identity %s\n", want);
    return 1;
}

/* Rust game.new_id(): a DISTINCT instance of a print. test_id aliases one shared
 * template, which would make every identity assertion on a duplicated card
 * vacuously true. The copy pool is process-global and monotonic, so it is reset
 * per case (test_game.h documents the exhaustion at ~1457 allocations). */
static int nid(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    assertions++;
    if (id < 0) {
        fprintf(stderr, "FAIL: fixture card \"%s\" is not in the database\n", no);
        failures++;
    }
    return id;
}

static void reset_board(TestGame *tg)
{
    test_card_copy_pool_reset();
    test_game_new(tg);
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        P->hand.n = 0;
        P->discard.n = 0;
        P->live.n = 0;
        P->success.n = 0;
        P->energy.n = 0;
        P->energy_active = 0;
        P->energy_deck.n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            P->stage[i] = RB_EMPTY_SLOT;
            P->stage_wait[i] = 0;
            P->under_cards[i].n = 0;
        }
    }
    tg->state.n_revealed = 0;
    tg->state.last_draw_count = 0;
    tg->state.position_change_occurred_this_turn = 0;
}

static int hand_len(const TestGame *tg, int pl) { return tg->state.p[pl].hand.n; }
static int deck_len(const TestGame *tg, int pl) { return tg->state.p[pl].deck.n; }
static int disc_len(const TestGame *tg, int pl) { return tg->state.p[pl].discard.n; }

static int bag_contains(const RbBag *b, int cid)
{
    for (int i = 0; i < b->n; i++) if (b->cards[i] == cid) return 1;
    return 0;
}

static void bag_push_n(RbBag *b, const int *ids, int n)
{
    b->n = 0;
    for (int i = 0; i < n && i < RB_MAX_ZONE; i++) b->cards[b->n++] = ids[i];
}

/* Rust fill_decks: main_deck.cards.clear() then n x push(filler), both seats. */
static void fill_decks(TestGame *tg, int filler, int per_player)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < per_player && P->deck.n < RB_MAX_ZONE; i++)
            P->deck.cards[P->deck.n++] = filler;
    }
}

static int pending(TestGame *tg) { return test_has_pending_choice(tg); }

/* Rust select_indices(&[]) -- an explicit DECLINE. */
static void skip_pick(TestGame *tg)
{
    if (pending(tg)) rb_resume_with_choice(&tg->state, -1);
}

static void answer(TestGame *tg, int idx)
{
    if (pending(tg)) rb_resume_with_choice(&tg->state, idx);
}

/* PROCEED an auto-ability: index 0 on a card pick, index 1 on a
 * pay_optional_cost gate (option 0 there is SKIP), a skip otherwise. Returns the
 * number of prompts driven, which is the non-vacuity counter for a case whose
 * whole point is "nothing happened". */
static int drive(TestGame *tg, int guard_max)
{
    int n = 0, guard = 0;
    while (pending(tg) && guard++ < guard_max) {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        int idx = 0;
        if (c && c->kind == RB_CHOICE_SELECT_TARGET) idx = 1;
        rb_resume_with_choice(&tg->state, idx);
        n++;
    }
    return n;
}

/* ライブ成功時 must go through rb_trigger_live_success: the trigger needs BOTH a
 * non-empty live zone and live_success[pl] set, so a bare rb_trigger_auto_ability
 * would be measuring nothing. */
static int fire_live_success(TestGame *tg, int cid)
{
    tg->state.phase = RB_PHASE_VICTORY;
    tg->state.live_success[0] = 1;
    tg->state.activating_card = cid;
    int n = rb_trigger_live_success(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    return n;
}

/* ══════════════════════════════════════════════════════════════════════════
 * SECTION A -- effects/draw/until_count/  (the one missing Rust fn, plus the
 * card-identity and deck-order assertions no C suite makes)
 * ══════════════════════════════════════════════════════════════════════════ */

#define MIYASHITA "PL!N-PR-028-PR"      /* 宮下 愛 DiverDiva: 登場 discard2 → draw to 5 */
#define FILLER    "PL!-sd1-010-SD"      /* 高坂穂乃果, cost 4, member */

/* Rust setup_pr_ai(game, hand_extra, deck_count) (miyashita_ai_pr_test.rs:20-46),
 * with the deck stocked from DISTINCT instances so deck order is observable. */
static int setup_pr_ai(TestGame *tg, int hand_extra, int deck_count, int *deck_ids)
{
    int ai = nid(tg, MIYASHITA);
    if (ai < 0) return -1;
    if (!pin(ai, MIYASHITA)) return -1;

    test_give_energy(tg, 15);   /* the card costs 11 */

    test_add_to_hand(tg, ai);
    for (int i = 0; i < hand_extra; i++) {
        int f = nid(tg, FILLER);
        if (f < 0) return -1;
        test_add_to_hand(tg, f);
    }
    /* Rust: main_deck.cards.push(filler) xN -- append, so index 0 is the TOP
     * under the suite's TOP-INDEX-0 convention. */
    RbBag *D = &tg->state.p[0].deck;
    D->n = 0;
    for (int i = 0; i < deck_count && i < RB_MAX_ZONE; i++) {
        int c = nid(tg, FILLER);
        if (c < 0) return -1;
        D->cards[D->n++] = c;
        deck_ids[i] = c;
    }
    return ai;
}

/* ── A1. pr_ai_hand_4_pays_cost_draws_4 (miyashita_ai_pr_test.rs:238-267) ────
 * Four cards in hand (Ai + 3 filler). The optional discard-2 cost is PAID, so
 * the hand is 3 - 2 = 1 and draw_until_count draws 4 to reach 5.
 *
 * Rust asserts: hand == 5, deck == deck_before - 4, waitroom == discard_before + 2.
 * This is the arithmetic NO existing C case measures: SECTION D has hand_extra=3
 * and hand_extra=6 but SKIPS the cost in both, and draw_until_count.c's pay cases
 * start from hand_extra=2 (which empties the hand and draws the full 5). */
static void a1_pr_ai_hand_4_pays_cost_draws_4(void)
{
    static TestGame tg;
    reset_board(&tg);
    int deck_ids[16];
    int ai = setup_pr_ai(&tg, 3, 10, deck_ids);
    if (ai < 0) return;

    int hand_pre = hand_len(&tg, 0);
    CHECK_EQ(hand_pre, 4, "A1 precondition: Ai + 3 filler really are in hand");

    CHECK_EQ(test_play_to_stage(&tg, ai, 1 /* centre */), 1, "A1: the debut is accepted");
    CHECK_EQ(tg.state.p[0].stage[1], ai, "A1 CONTROL: Ai is on the centre stage");

    /* rb_play_member removes the played card, so the cost prompt sees 3 cards.
     * Reading the post-play baseline HERE is what makes the deltas below mean
     * "the cost" and not "the play". */
    int hand_before = hand_len(&tg, 0);
    int deck_before = deck_len(&tg, 0);
    int disc_before = disc_len(&tg, 0);
    CHECK_EQ(hand_before, 3,
             "A1 CONTROL: the play moved exactly Ai out of hand, so the remaining "
             "3 are the cost's candidates");
    CHECK_EQ(deck_before, 10, "A1 precondition: the stock deck really holds 10 cards");
    CHECK(pending(&tg), "A1 CONTROL: the optional discard-2 cost really is offered");

    /* Rust game.try_select_indices(&[0, 1]). The C choice model consumes one index
     * per resume and re-offers for the remainder (choice.c:850-860), so the same
     * selection is driven stepwise -- exactly as test_parity_draw_until_count.c:212
     * does for the same card. */
    for (int step = 0; step < 4 && pending(&tg); step++) {
        const int pick[1] = { 0 };
        rb_resume_with_choice_indices(&tg.state, pick, 1);
    }
    drive(&tg, 12);

    CHECK_EQ(disc_len(&tg, 0), disc_before + 2,
             "A1: paying the optional cost really discarded 2 to the waitroom");
    CHECK_EQ(hand_len(&tg, 0), 5,
             "A1: hand 3 - 2 + 4 drawn == 5 (draw_until_count target_count 5)");
    CHECK_EQ(deck_before - deck_len(&tg, 0), 4,
             "A1: exactly 4 cards left the deck -- the deficit, not the full target");
    CHECK_EQ(tg.state.last_draw_count, 4,
             "A1: last_draw_count is the requested deficit (deck.rs:446 saturating_sub)");

    /* No card was created or destroyed: 4 hand + 2 waitroom + 6 deck + Ai = 14. */
    CHECK_EQ(hand_len(&tg, 0) + deck_len(&tg, 0) + disc_len(&tg, 0) + 1, 14,
             "A1: the cost and the draw only MOVE cards (4 + 2 + 6 + 1 staged = 14)");
}

/* ── A2 + A3. pr_ai_pays_cost_discards_2_draws_4, identity half ─────────────
 * miyashita_ai_pr_test.rs:146-161 asserts, after paying the cost, that the two
 * filler ids captured from hand BEFORE the play are the ones in the waitroom,
 * that Ai is in neither hand nor waitroom, and (line 79-82) that the waitroom
 * gained exactly 2.
 *
 * The existing C port (test_parity_draw_until_count.c:115) asserts only
 * hand + deck + discard == 12, which is satisfied by discarding ANY two cards,
 * and it stocks the deck and the hand with one aliased template id, so no card
 * identity is observable at all. Here every hand and deck card is a distinct
 * pool instance. */
static void a2_pr_ai_pays_cost_discards_the_right_two(void)
{
    static TestGame tg;
    reset_board(&tg);
    int deck_ids[16];
    int ai = setup_pr_ai(&tg, 2, 10, deck_ids);
    if (ai < 0) return;

    /* Capture the hand contents BEFORE the play, like the Rust twin. */
    int hand_pre[4];
    int hand_pre_n = hand_len(&tg, 0);
    CHECK_EQ(hand_pre_n, 3, "A2 precondition: Ai + 2 filler in hand before the play");
    for (int i = 0; i < hand_pre_n; i++) hand_pre[i] = tg.state.p[0].hand.cards[i];

    int disc_before = disc_len(&tg, 0);
    CHECK_EQ(test_play_to_stage(&tg, ai, 1), 1, "A2: the debut is accepted");
    CHECK(pending(&tg), "A2 CONTROL: the optional cost really is offered");

    /* Rust game.try_select_indices(&[0, 1]) -- the two filler cards. */
    {
        const int pick[2] = { 0, 1 };
        rb_resume_with_choice_indices(&tg.state, pick, 2);
    }
    drive(&tg, 12);

    /* A3: the waitroom delta the existing suite never measures. */
    CHECK_EQ(disc_len(&tg, 0), disc_before + 2,
             "A3: paying the optional cost put exactly 2 cards in the waitroom");

    /* A2: the discarded ids are the ones that were in hand, not the template. */
    for (int i = 1; i < hand_pre_n; i++) {
        char msg[160];
        snprintf(msg, sizeof(msg),
                 "A2: hand card %d (id %d) that the cost selected really reached the waitroom",
                 i, hand_pre[i]);
        CHECK(bag_contains(&tg.state.p[0].discard, hand_pre[i]), msg);
    }
    CHECK(!bag_contains(&tg.state.p[0].discard, ai),
          "A2: Ai is NOT in the waitroom -- it is on stage, not discarded");
    CHECK(!bag_contains(&tg.state.p[0].hand, ai),
          "A2: Ai is NOT in hand after being played to the stage");

    CHECK_EQ(hand_len(&tg, 0), 5,
             "A2: hand 2 - 2 + 5 drawn == 5 (the hand emptied, so the full target drew)");
    CHECK_EQ(deck_len(&tg, 0), 5, "A2: 5 cards left the 10-card stock deck");

    /* A4: DECK ORDER. draw_cards_for_player takes index 0 and shifts down
     * (draw.c:52-57), so the hand received d0..d4 in that order and the deck
     * still holds d5..d9 in that order. */
    {
        int hand_ids[8];
        int n = test_zone_ids(&tg, 0, "hand", hand_ids, 8);
        CHECK_EQ(n, 5, "A4: five cards are in hand to order-check");
        char msg[160];
        for (int i = 0; i < n; i++) {
            snprintf(msg, sizeof(msg),
                     "A4: hand[%d] is deck instance %d -- the draw consumes the TOP of "
                     "the bag in order (TOP-INDEX-0)", i, deck_ids[i]);
            CHECK_EQ(hand_ids[i], deck_ids[i], msg);
        }
        int deck_ids_out[8];
        int dn = test_zone_ids(&tg, 0, "deck", deck_ids_out, 8);
        CHECK_EQ(dn, 5, "A4: five cards remain in the deck");
        for (int i = 0; i < dn; i++) {
            snprintf(msg, sizeof(msg),
                     "A4: deck[%d] is stock instance %d -- the REMAINDER keeps its order",
                     i, deck_ids[i + 5]);
            CHECK_EQ(deck_ids_out[i], deck_ids[i + 5], msg);
        }
    }
}

/* ── A2b. CONTROL for A2: the same card DOES draw ────────────────────────────
 * A negative result is only meaningful if the positive arm works. This proves
 * Ai really is a working draw_until_count host on the very same fixture. */
static void a2b_control_same_card_draws_to_five(void)
{
    static TestGame tg;
    reset_board(&tg);
    int deck_ids[16];
    int ai = setup_pr_ai(&tg, 2, 10, deck_ids);
    if (ai < 0) return;

    int hand_before = 0;
    CHECK_EQ(test_play_to_stage(&tg, ai, 1), 1, "A2b CONTROL: the debut is accepted");
    hand_before = hand_len(&tg, 0);
    CHECK_EQ(hand_before, 2, "A2b CONTROL: 2 filler remain after the play");
    CHECK(pending(&tg), "A2b CONTROL: the optional cost really is offered");

    const int pick[2] = { 0, 1 };
    rb_resume_with_choice_indices(&tg.state, pick, 2);
    drive(&tg, 12);

    CHECK_EQ(hand_len(&tg, 0), 5,
             "A2b CONTROL: the identical fixture fills the hand to 5, so A2's "
             "identity assertions are not passing on a dead ability");
    CHECK_EQ(disc_len(&tg, 0), 2, "A2b CONTROL: 2 cards really were paid to the waitroom");
}

/* ══════════════════════════════════════════════════════════════════════════
 * SECTION B -- effects/draw/conditional/  (the one arm never actually run)
 * ══════════════════════════════════════════════════════════════════════════ */

#define SWEET_SWEET "PL!-bp6-023-L"   /* sweet&sweet holiday: ライブ成功時 draw 1;   */
                                      /* if a μ's live card is in your success zone,  */
                                      /* draw 1 more.                                    */
#define MUS_LIVE    "PL!-sd1-020-SD"   /* きっと青春が聞こえる -- type ライブ,          */
                                      /* series ラブライブ！ -- the μ's LIVE card the   */
                                      /* Rust twin actually names (bp6_023_l_test.rs:14) */
#define NIJI_LIVE   "PL!N-sd1-025-SD"   /* Colorful Dreams! -- ライブ, 虹ヶ咲 (non-μ's)  */

/* The group names the ENGINE's card record carries, read back from cards.bin by
 * rb_card_group_name. These are the short forms, not cards.json's `series`. */
#define MUS_GROUP   "\xCE\xBC\x27\x73"                       /* "μ's"  (CE BC 27 73) */
#define NIJI_GROUP  "\xE8\x99\xB9\xE3\x83\xB6\xE5\x92\xB2"   /* U+8679 U+30F6 U+54B2 */

/* ── B1. sweet_sweet_holiday_pl_bp6_023_l_draws_extra_with_mus_live_in_
 *      success_zone (success_zone_group_pl_bp6_023_l_test.rs:10-33) ─────────
 * Rust: live = PL!-bp6-023-L in the live zone, mus_live = PL!-sd1-020-SD in the
 * SUCCESS zone, fire ライブ成功時, assert the deck lost exactly 2.
 *
 * The existing C port substituted PL!-sd1-010-SD (高坂穂乃果, a メンバー) and
 * then asserted that substitution was a defect in the Rust twin. It is not: the
 * Rust file names PL!-sd1-020-SD, which really is a μ's live card. So the
 * positive arm of this Rust test had never been executed against the printed
 * text. It is executed here, and the card's type/series are re-asserted from the
 * ENGINE's decoded Card so a cards.json / cards.bin disagreement cannot hide. */
static void b1_mus_live_in_success_zone_draws_two(void)
{
    static TestGame tg;
    reset_board(&tg);
    int live = nid(&tg, SWEET_SWEET);
    int mus  = nid(&tg, MUS_LIVE);
    if (live < 0 || mus < 0) return;
    if (!pin(live, SWEET_SWEET)) return;
    if (!pin(mus, MUS_LIVE)) return;

    /* CONTROL: the fixture really is a μ's LIVE card, which is the printed
     * 「自分の成功ライブカード置き場にある『μ's』のライブカード」. The group's
     * card-record field holds the short form "μ's" (CE BC 27 73), which is what
     * the engine's group filter reads -- cards.json's `series` ("ラブライブ！")
     * is a DIFFERENT, longer string and is not what the comparison sees. */
    CHECK(rb_card_is_live(mus),
          "B1 fixture: PL!-sd1-020-SD is type ライブ (a LIVE card), so the "
          "『ライブカード』filter is what this case actually exercises");
    {
        const char *grp = rb_card_group_name(mus);
        char msg[200];
        snprintf(msg, sizeof(msg),
                 "B1 fixture: PL!-sd1-020-SD group is '%s' -- must be the short "
                 "μ's form the group filter compares against", grp ? grp : "<null>");
        CHECK(grp && strcmp(grp, MUS_GROUP) == 0, msg);
        snprintf(msg, sizeof(msg),
                 "B1 fixture: PL!-sd1-020-SD group '%s' is NOT the 虹ヶ咲 group, "
                 "so a non-μ's card could not have produced this draw",
                 grp ? grp : "<null>");
        CHECK(grp && strcmp(grp, NIJI_GROUP) != 0, msg);
    }

    int filler = nid(&tg, FILLER);
    if (filler < 0) return;
    fill_decks(&tg, filler, 40);
    test_set_live_card(&tg, 0, live);
    test_add_to_success(&tg, mus);

    CHECK_EQ(tg.state.p[0].live.n, 1, "B1 CONTROL: the live card really is in the live zone");
    CHECK_EQ(tg.state.p[0].success.n, 1, "B1 CONTROL: a card really is in the success zone");
    CHECK_EQ(tg.state.p[0].success.cards[0], mus,
             "B1 CONTROL: the success-zone card is the pinned μ's live, not an alias");

    int deck_before = deck_len(&tg, 0);
    CHECK_EQ(deck_before, 40, "B1 precondition: the stock deck really holds 40 cards");
    int hand_before = hand_len(&tg, 0);

    int fired = fire_live_success(&tg, live);
    int prompts = drive(&tg, 12);
    fprintf(stderr, "  (B1 mus) trigger=%d prompts=%d deck %d -> %d\n",
            fired, prompts, deck_before, deck_len(&tg, 0));

    CHECK_EQ(deck_before - deck_len(&tg, 0), 2,
             "B1: a μ's LIVE card in the success zone -> base draw of 1 plus 1 more");
    CHECK_EQ(hand_len(&tg, 0), hand_before + 2,
             "B1 CONTROL: both drawn cards really reached hand");
}

/* ── B1b. zone control: the SAME μ's live card, but in HAND ─────────────────
 * Separates "the gate read the wrong zone" from "the gate read the wrong group".
 * Group and card type are identical to B1; only the zone differs, so the deck
 * delta must fall back to the base draw of 1. */
static void b1b_mus_live_in_hand_is_not_a_success_zone_card(void)
{
    static TestGame tg;
    reset_board(&tg);
    int live = nid(&tg, SWEET_SWEET);
    int mus  = nid(&tg, MUS_LIVE);
    if (live < 0 || mus < 0) return;
    if (!pin(live, SWEET_SWEET) || !pin(mus, MUS_LIVE)) return;

    int filler = nid(&tg, FILLER);
    if (filler < 0) return;
    fill_decks(&tg, filler, 40);
    test_set_live_card(&tg, 0, live);
    test_add_to_hand(&tg, mus);

    CHECK_EQ(tg.state.p[0].success.n, 0, "B1b CONTROL: the success zone really is EMPTY");
    CHECK_EQ(tg.state.p[0].hand.n, 1, "B1b CONTROL: the μ's live is in hand instead");

    int deck_before = deck_len(&tg, 0);
    fire_live_success(&tg, live);
    drive(&tg, 12);

    CHECK_EQ(deck_before - deck_len(&tg, 0), 1,
             "B1b: the same μ's LIVE card in the wrong zone -> base draw of 1 only, "
             "so B1's 2 really was the success-zone gate and not the card's group");
}

/* ── B2. draw_until_count destination matrix ────────────────────────────────
 * Rust deck.rs:440-445:
 *     let current_count = match Zone::from_str(destination) {
 *         Some(Zone::Hand) => player.hand.len(),
 *         _ => return,
 *     };
 * Every non-hand destination is a silent no-op. draw.c:850-854 gates the same
 * way (rb_zone_of_str(dst) must succeed AND be RB_ZONE_HAND). The existing suite
 * proves this for "stage" only (test_parity_draw_until_count.c:387). Every wire
 * spelling rb_zone_of_str accepts is walked here, plus a stage-area alias
 * ("center"), plus a string it does not recognise. */
static void b2_draw_until_count_destination_matrix(void)
{
    static const struct { const char *dst; const char *label; } CASES[] = {
        { "hand",              "hand (the one destination that DOES draw)" },
        { "deck",              "deck" },
        { "deck_top",          "deck_top" },
        { "deck_bottom",       "deck_bottom" },
        { "discard",           "discard" },
        { "waitroom",          "waitroom" },
        { "energy",            "energy" },
        { "energy_zone",       "energy_zone" },
        { "live_card_zone",    "live_card_zone" },
        { "live",              "live" },
        { "success_live_zone", "success_live_zone" },
        { "success",           "success" },
        { "success_zone",      "success_zone" },
        { "stage",             "stage" },
        { "center",            "center (a stage-area alias, not the hand)" },
        { "left",              "left" },
        { "under_member",      "under_member" },
        { "resolution",        "resolution" },
        { "nowhere",           "nowhere (an unrecognised wire name)" },
    };
    const int N = (int)(sizeof(CASES) / sizeof(CASES[0]));

    for (int i = 0; i < N; i++) {
        static TestGame tg;
        reset_board(&tg);
        int ids[12];
        for (int k = 0; k < 12; k++) {
            ids[k] = nid(&tg, FILLER);
            if (ids[k] < 0) return;
        }
        bag_push_n(&tg.state.p[0].deck, ids, 12);

        AbilityEffect e;
        memset(&e, 0, sizeof e);
        e.action      = (char *)"draw_until_count";
        e.target      = (char *)"self";
        e.source      = (char *)"deck";
        e.destination = (char *)CASES[i].dst;
        e.count       = 5;
        static char tc[8];
        snprintf(tc, sizeof tc, "5");
        e.extra_k[0] = (char *)"target_count";
        e.extra_v[0] = tc;
        e.n_extra    = 1;

        /* Prove the hand zone starts empty so a positive cannot be vacuous. */
        CHECK_EQ(hand_len(&tg, 0), 0,
                 "B2 precondition: the hand starts empty (target_count 5 > 0)");
        CHECK_EQ(deck_len(&tg, 0), 12, "B2 precondition: the stock deck holds 12");

        rb_effect_draw_until_count(&tg.state, 0, &e);

        int is_hand = (strcmp(CASES[i].dst, "hand") == 0);
        char msg[200];
        snprintf(msg, sizeof(msg),
                 "B2: destination '%s' (%s) -> hand %d, deck %d",
                 CASES[i].dst, CASES[i].label,
                 hand_len(&tg, 0), deck_len(&tg, 0));
        if (is_hand) {
            CHECK_EQ(hand_len(&tg, 0), 5, msg);
            CHECK_EQ(deck_len(&tg, 0), 7,
                     "B2: destination 'hand' really consumes 5 cards off the top");
        } else {
            CHECK_EQ(hand_len(&tg, 0), 0,
                     msg);   /* the whole point: a non-hand destination is a no-op */
            CHECK_EQ(deck_len(&tg, 0), 12,
                     "B2: a non-hand destination leaves the stock deck untouched -- "
                     "deck.rs:443 `_ => return` fires before any draw");
        }
        CHECK_EQ(tg.state.last_draw_count, is_hand ? 5 : 0,
                 "B2: last_draw_count reflects the requested deficit only when the "
                 "destination is a hand");
    }
}

/* ══════════════════════════════════════════════════════════════════════════ */

int main(void)
{
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0) {
        fprintf(stderr, "load_cards failed\n");
        return 1;
    }
    setvbuf(stdout, NULL, _IONBF, 0);

    a1_pr_ai_hand_4_pays_cost_draws_4();
    a2_pr_ai_pays_cost_discards_the_right_two();
    a2b_control_same_card_draws_to_five();
    b1_mus_live_in_success_zone_draws_two();
    b1b_mus_live_in_hand_is_not_a_success_zone_card();
    b2_draw_until_count_destination_matrix();

    rb_unload();
    fprintf(stderr, "\n%d assertion(s), %d failure(s)\n", assertions, failures);
    if (failures) return 1;
    printf("all draw-delta parity checks passed\n");
    return 0;
}
