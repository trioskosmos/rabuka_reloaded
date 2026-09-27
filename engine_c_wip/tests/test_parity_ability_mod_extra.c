/* test_parity_ability_mod_extra.c -- C parity suite for the GAPS in the Rust
 * cluster engine/tests/test_modules/effects/ability_mod/ (15 files).
 *
 * tests/test_parity_ability_mod.c already covers:
 *   rina_gain_live_success_from_under_member_test.rs      (all 6)
 *   genki_zenkai_invalidates_own_live_success_test.rs      (test 1 only)
 *   discard_low_cost_..._q108_q240_test.rs                (2 of 5)
 *   chisato_test.rs / reveal_hand_cost_total_...rs         (ladder + Q78 + empty hand)
 *   nonempty_success_zone_..._test.rs                      (3 of 4)
 *   live_total_threshold_pl_pr_020_pr_test.rs             (1 of 1)
 *   energy_cost_aqours_score_...rs                         (the matrix only)
 *   heart02_gated_live_success_invalidation_setup_test.rs  (2 of 2)
 *   live_success_invalidation_and_..._suppression_test.rs  (2 of 4, predicates only)
 *   center_gated_wait_member_gain_total_score_test.rs     (position gates, not the use record)
 *
 * This file ports only what that file does NOT already assert:
 *   1. chisato_bp5_test.rs                            (7 tests, entirely uncovered)
 *   2. constant_deck_bottom_yell_source_test.rs       (7 tests, entirely uncovered)
 *   3. full_distinct_group_stage_grants_yell_live_score_test.rs
 *                                                   (9 of 10 tests uncovered)
 *   4. discard_low_cost_..._q108_q240_test.rs         (the 3 remaining cases)
 *   5. the single-case gaps: Hanamaru's card identity, Chisato's turn-1 limit,
 *      Nozomi's P rarity, Chika's use record, Genki's LiveSuccess control,
 *      Butterfly's LiveSuccess score
 *
 * Harness notes (established by peers):
 *   - rb_load("src") fails under an isolated out-of-tree build, so load_card_db()
 *     falls back to "../cards/build".
 *   - deck index 0 is the TOP for the C deck bag (matches Rust main_deck insert(0)).
 *   - a NULL indices pointer with n == 0 is Rust's select_indices(&[]) (decline).
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
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
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* -- card DB ---------------------------------------------------------- */

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* -- helpers ---------------------------------------------------------- */

static int card_printed_cost(int cid)
{
    Card c;
    int cost = 0;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) { cost = c.cost; rb_free_card(&c); }
    return cost;
}

static int card_printed_score(int cid)
{
    Card c;
    int score = 0;
    if (rb_decode_card_by_index((uint32_t)cid, &c)) { score = c.score; rb_free_card(&c); }
    return score;
}

static int card_prints_trigger(int cid, const char *trig)
{
    int n = rb_card_num_abilities((uint32_t)cid);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)cid, i, &ab)) continue;
        int hit = ab.triggers && strstr(ab.triggers, trig);
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

static void fill_decks(TestGame *tg, int filler, int n)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = 0;
        for (int i = 0; i < n && P->deck.n < RB_MAX_DECK; i++) P->deck.cards[P->deck.n++] = filler;
    }
}

static void decline_choices(TestGame *tg)
{
    int guard = 0;
    while (test_has_pending_choice(tg) && guard++ < 64) {
        test_select_indices(tg, NULL, 0);
    }
}

/* fire_trigger(game, cid, trig) -- helpers/mod.rs fire_trigger: set
 * activating_card, queue the one ability printing `trig`, then run
 * process_pending_auto_abilities. */
static int fire_trigger(TestGame *tg, int cid, const char *trig)
{
    if (!card_prints_trigger(cid, trig)) return 0;
    tg->state.activating_card = cid;
    int queued = rb_queue_trigger_abilities(&tg->state, 0, trig);
    if (queued <= 0) return 0;
    rb_process_pending_auto_abilities(&tg->state);
    return 1;
}

static const RbLiveSnapshot *snapshot_for(const TestGame *tg, int pl)
{
    for (int i = tg->state.n_snapshots - 1; i >= 0; i--)
        if (tg->state.snapshots[i].player == pl && tg->state.snapshots[i].n_lives > 0)
            return &tg->state.snapshots[i];
    return NULL;
}

/* =====================================================================
 * 1. chisato_bp5_test.rs -- PL!SP-bp5-003-P / PL!SP-bp5-003-R+
 *
 * ab#0 (constant): a cost-10 Liella! member card appearing from hand costs 2 less
 * ab#1 (live start, centre): activate every Liella! member and all energy
 *
 * The Rust picks PL!SP-bp2-006-R+ (Kinako) as the cost-10 target because the
 * printed rule only says "a cost-10 Liella! member". The C tree fires EVERY
 * 登场 ability on a plain (non-baton) play, and Kinako's 登场 is gated on
 * appearing by BATON TOUCH, so playing her from hand pulls her straight back
 * off the stage. That defect has its own test below
 * (test_kinako_baton_only_debut_fires_on_a_plain_play); the reduction tests
 * here use PL!SP-bp4-022-N, another cost-10 Liella! member with no 登场, so
 * the cost arithmetic is the only thing under test.
 * ===================================================================== */

#define LIELLA_COST10 "PL!SP-bp4-022-N"   /* cost 10, Liella!, no 登场 */
#define KINAKO_COST10 "PL!SP-bp2-006-R＋"  /* cost 10, Liella!, baton-only 登场 */
#define CHISATO_R     "PL!SP-bp5-003-R＋"
#define CHISATO_P     "PL!SP-bp5-003-P"

static void assert_cost10_liella_member(TestGame *tg, int cid, const char *card_no)
{
    CHECK(rb_card_no_eq(cid, card_no), "setup: the cost-10 target resolves to its print");
    CHECK_EQ(card_printed_cost(cid), 10, "setup: the target really costs 10");
    (void)tg;
}

/* Without the reduction a cost-10 member needs exactly 10 energy. */
static void test_chisato_bp5_cost_10_needs_10_energy(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int liella = test_id(&tg, LIELLA_COST10);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    CHECK(card_printed_cost(liella) == 10,
          "setup: the reduction target really costs 10");
    CHECK(!card_prints_trigger(liella, RB_TSTR_DEBUT),
          "setup: the target has no 登場 ability, so the play is not perturbed by it");

    test_add_to_hand(&tg, liella);
    test_add_to_hand(&tg, filler);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 10);

    CHECK(test_play_to_stage(&tg, liella, 0), "cost-10 needs 10 energy: the play succeeds");
    CHECK_EQ(tg.state.p[0].stage[0], liella, "the member is on stage");
    CHECK_EQ(tg.state.p[0].energy_active, 0, "all 10 energy was spent");
}

/* 9 energy is NOT enough for a cost-10 member. */
static void test_chisato_bp5_cost_10_fails_with_9(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int liella = test_id(&tg, LIELLA_COST10);
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    test_add_to_hand(&tg, liella);
    test_add_to_hand(&tg, filler);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 9);

    CHECK(!test_play_to_stage(&tg, liella, 0), "9 energy must NOT be enough for cost-10");
    /* "Refused" must mean "cannot afford", not some unrelated failure. */
    CHECK_EQ(tg.state.p[0].energy_active, 9, "the 9 energy is untouched after the refusal");
    CHECK_EQ(tg.state.p[0].stage[0], RB_EMPTY_SLOT, "a refused play leaves the area empty");
    CHECK(test_hand_has(&tg, liella), "a refused play leaves the card in hand");
}

/* Chisato on stage -> the cost-10 member needs only 8 (10 - 2). */
static void test_chisato_bp5_cross_card_reduction_8_works(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, CHISATO_R);
    int liella = test_id(&tg, LIELLA_COST10);
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    CHECK(rb_card_no_eq(chisato, CHISATO_R), "setup: the R+ print of Chisato resolves");
    CHECK_EQ(card_printed_cost(liella), 10, "setup: the target really costs 10");
    CHECK(!card_prints_trigger(liella, RB_TSTR_DEBUT),
          "setup: the target has no 登場 ability, so a refused play cannot be a debut side effect");
    tg.state.p[0].stage[0] = chisato;
    test_recalc(&tg);
    CHECK_EQ(test_get_cost_modifier(&tg, liella), -2,
             "while Chisato is on stage the 常時 must lower the hand cost-10 Liella! member's "
             "play cost by 2 (expected -2, got 0 means the reduction is not implemented)");
    test_add_to_hand(&tg, liella);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 8);

    CHECK(test_play_to_stage(&tg, liella, 1),
          "Chisato on stage: a cost-10 member needs only 8 energy (10 - 2)");
    CHECK_EQ(tg.state.p[0].stage[1], liella, "the member is on stage after the cost reduction");
    CHECK_EQ(tg.state.p[0].energy_active, 0, "exactly the reduced 8 was spent");
}

/* Chisato on stage, but only 7 energy -> NOT enough. */
static void test_chisato_bp5_cross_card_reduction_7_fails(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, CHISATO_R);
    int liella = test_id(&tg, LIELLA_COST10);
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    tg.state.p[0].stage[0] = chisato;
    test_add_to_hand(&tg, liella);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 7);

    CHECK(!test_play_to_stage(&tg, liella, 1),
          "Chisato on stage: 7 energy is one short of the reduced cost 8");
    CHECK_EQ(tg.state.p[0].energy_active, 7, "the 1-energy shortfall is the whole reason: 7 remain active");
    CHECK_EQ(tg.state.p[0].stage[1], RB_EMPTY_SLOT, "a refused play leaves the centre empty");
    CHECK(test_hand_has(&tg, liella), "the refused card is still in hand");
}

/* PL!SP-bp5-003-P costs 17 and does NOT get its own -2 (that is for cost-10). */
static void test_chisato_bp5_self_cost_not_reduced(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int promo = test_id(&tg, CHISATO_P);
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    CHECK_EQ(card_printed_cost(promo), 17, "setup: the promo really costs 17");
    test_add_to_hand(&tg, promo);
    test_add_to_hand(&tg, filler);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 16);

    CHECK(!test_play_to_stage(&tg, promo, 0), "16 energy must not be enough to play a 17-cost card");
    CHECK_EQ(tg.state.p[0].energy_active, 16, "the 16 energy is untouched by the refusal");
    CHECK_EQ(tg.state.p[0].stage[0], RB_EMPTY_SLOT, "the refused card never reached the stage");

    test_give_energy(&tg, 1); /* 16 + 1 = 17 */
    CHECK(test_play_to_stage(&tg, promo, 0), "17 energy is enough: the 17-cost is NOT reduced to 15");
    CHECK_EQ(tg.state.p[0].stage[0], promo, "the promo is on stage");
}

/* The P print carries the same ab#0. */
static void test_chisato_promo_ab0_cross_card_reduction(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int promo = test_id(&tg, CHISATO_P);
    int liella = test_id(&tg, LIELLA_COST10);
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    CHECK(card_prints_trigger(promo, RB_TSTR_CONSTANT),
          "setup: the P print really prints the constant cost reduction");
    CHECK_EQ(card_printed_cost(liella), 10, "setup: the target really costs 10");
    CHECK(!card_prints_trigger(liella, RB_TSTR_DEBUT),
          "setup: the target has no 登場 ability, so a refused play cannot be a debut side effect");
    tg.state.p[0].stage[0] = promo;
    test_add_to_hand(&tg, liella);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 8);

    CHECK(test_play_to_stage(&tg, liella, 1),
          "promo on stage: a cost-10 member needs only 8 energy");
    CHECK_EQ(tg.state.p[0].stage[1], liella, "the member is on stage after the cost reduction");
    CHECK_EQ(tg.state.p[0].energy_active, 0, "exactly the reduced 8 was spent");
}

/* ab#1 (live start, centre only): activate every Liella! member and all energy. */
static void test_chisato_promo_ab1_live_start(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int promo = test_id(&tg, CHISATO_P);
    int liella = test_id(&tg, LIELLA_COST10);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int live_card = test_id(&tg, "PL!-sd1-019-SD");

    /* Chisato must be in the CENTRE for her centre-gated live start. */
    tg.state.p[0].stage[0] = liella;
    tg.state.p[0].stage[1] = promo;
    tg.state.p[0].stage[2] = filler;
    rb_mods_set_orientation(&tg.state.mods, liella, "wait");
    rb_mods_set_orientation(&tg.state.mods, promo, "wait");

    test_give_energy(&tg, 3);
    test_set_energy_active(&tg, 0, 0);   /* all three rest */

    test_add_to_hand(&tg, live_card);
    fill_decks(&tg, filler, 30);

    CHECK(test_advance_to_phase(&tg, RB_PHASE_LIVE_SET), "the turn reaches the live-card set phase");
    test_set_live_card(&tg, 0, live_card);
    CHECK(test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE),
          "the turn reaches the performance phase (the live start fires here)");
    decline_choices(&tg);

    const char *liella_ori = rb_mods_get_orientation(&tg.state.mods, liella);
    const char *promo_ori = rb_mods_get_orientation(&tg.state.mods, promo);
    CHECK(liella_ori && strcmp(liella_ori, "wait") != 0,
          "the Liella! member should no longer be in wait state");
    CHECK(promo_ori && strcmp(promo_ori, "wait") != 0, "Chisato should no longer be in wait state");
    CHECK_EQ(tg.state.p[0].energy_active, 3, "all 3 energy should be active");
}

/* The 登場 side of the same defect, isolated. rb_play_member (engine.c:1083)
 * runs every ability whose trigger string contains 登场 with no baton-touch
 * gate, so a member whose 登场 text opens with バトンタッチして登場した場合は
 * has its 登場 fired on a plain play and is moved straight back off the stage. */
static void test_kinako_baton_only_debut_fires_on_a_plain_play(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int kinako = test_id(&tg, KINAKO_COST10);
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    assert_cost10_liella_member(&tg, kinako, KINAKO_COST10);
    CHECK(card_prints_trigger(kinako, RB_TSTR_DEBUT),
          "setup: the card DOES print a 登场, but that 登場 is gated on appearing "
          "by baton touch, so a plain play must not fire it");

    test_add_to_hand(&tg, kinako);
    test_add_to_hand(&tg, filler);
    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 10);

    CHECK(test_play_to_stage(&tg, kinako, 0), "a plain (non-baton) play is accepted");
    CHECK_EQ(tg.state.p[0].stage[0], kinako,
             "a plain play must leave the member on the stage: the baton-only 登場 "
             "must not fire and pull her back to hand");
    CHECK(!test_hand_has(&tg, kinako),
          "a plain play removes the member from hand; the 登場 must not put her back");
}

/* =====================================================================
 * 2. constant_deck_bottom_yell_source_test.rs -- PL!S-bp7-022-L
 *    the yell is performed from the BOTTOM of the deck instead of the top
 * ===================================================================== */

#define AQUARIUM    "PL!S-bp7-022-L"
#define DECK_TOP    "PL!S-sd1-001-SD"
#define DECK_BOTTOM "PL!N-bp1-001-R"

/* The reveal pool is set-based: the order inside the pool has no game
 * semantics, so compare as multisets (the Rust same_set idiom). */
static int same_set(const int *a, int na, const int *b, int nb)
{
    if (na != nb) return 0;
    if (na > 64) return 0;
    int tmp[64];
    for (int i = 0; i < na; i++) tmp[i] = a[i];
    for (int i = 0; i < nb; i++) {
        int hit = 0;
        for (int j = 0; j < na; j++) if (tmp[j] == b[i]) { tmp[j] = -2147483647 - 1; hit = 1; break; }
        if (!hit) return 0;
    }
    return 1;
}

/* deck index 0 is the TOP for the C deck bag (matches Rust main_deck insert(0)). */
static void set_deck_order(TestGame *tg, const int *ids, int n)
{
    RbPlayer *P = &tg->state.p[0];
    P->deck.n = 0;
    for (int i = 0; i < n && P->deck.n < RB_MAX_DECK; i++) P->deck.cards[P->deck.n++] = ids[i];
}

static void fill_p2_deck(TestGame *tg, int filler, int n)
{
    RbPlayer *P = &tg->state.p[1];
    P->deck.n = 0;
    for (int i = 0; i < n && P->deck.n < RB_MAX_DECK; i++) P->deck.cards[P->deck.n++] = filler;
}

static void test_bottom_yell_source_yell_from_bottom(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int aquarium = test_id(&tg, AQUARIUM);
    int top = test_id(&tg, DECK_TOP);
    int bottom = test_id(&tg, DECK_BOTTOM);

    CHECK(card_prints_trigger(aquarium, RB_TSTR_CONSTANT),
          "setup: the AQUARIUM live card prints the constant that changes the yell source");
    test_add_to_live(&tg, aquarium);
    const int order[6] = { top, top, top, bottom, bottom, bottom };
    set_deck_order(&tg, order, 6);
    fill_p2_deck(&tg, top, 20);
    test_recalc(&tg);

    CHECK(tg.state.p[0].yell_from_bottom, "setup: the constant registered a deck-bottom yell source");
    CHECK_EQ(rb_perform_cheer_check(&tg.state, "p1", 3), 0, "the yell resolves");
    const int want[3] = { bottom, bottom, bottom };
    CHECK(same_set(tg.state.resolution.cards, tg.state.resolution.n, want, 3),
          "the yell must reveal the 3 BOTTOM cards, not the top three");
}

static void test_no_yell_source_modifier_reveals_from_top(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int top = test_id(&tg, DECK_TOP);
    int bottom = test_id(&tg, DECK_BOTTOM);

    const int order[6] = { top, top, top, bottom, bottom, bottom };
    set_deck_order(&tg, order, 6);
    fill_p2_deck(&tg, top, 20);
    test_recalc(&tg);

    CHECK(!tg.state.p[0].yell_from_bottom, "setup: no AQUARIUM in play, so the source is the deck top");
    CHECK_EQ(rb_perform_cheer_check(&tg.state, "p1", 3), 0, "the yell resolves");
    CHECK(same_set(tg.state.resolution.cards, tg.state.resolution.n, order, 3),
          "without AQUARIUM the yell reveals the TOP cards");
}

static void test_opponent_yell_source_modifier_does_not_change_own_yell(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int aquarium = test_id(&tg, AQUARIUM);
    int top = test_id(&tg, DECK_TOP);
    int bottom = test_id(&tg, DECK_BOTTOM);

    /* Only the OPPONENT holds the AQUARIUM. */
    tg.state.p[1].live.cards[tg.state.p[1].live.n++] = aquarium;
    const int order[6] = { top, top, top, bottom, bottom, bottom };
    set_deck_order(&tg, order, 6);
    fill_p2_deck(&tg, top, 6);
    test_recalc(&tg);

    CHECK(tg.state.p[1].yell_from_bottom, "setup: the opponent's own source IS the deck bottom");
    CHECK(!tg.state.p[0].yell_from_bottom, "setup: p1 has no AQUARIUM, so p1's source is the top");
    CHECK_EQ(rb_perform_cheer_check(&tg.state, "p1", 3), 0, "p1's yell resolves");
    CHECK(same_set(tg.state.resolution.cards, tg.state.resolution.n, order, 3),
          "only the OWN player's AQUARIUM changes the yell source");
}

static void test_bottom_yell_source_short_deck(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int aquarium = test_id(&tg, AQUARIUM);
    int bottom = test_id(&tg, DECK_BOTTOM);

    test_add_to_live(&tg, aquarium);
    const int only[2] = { bottom, bottom };
    set_deck_order(&tg, only, 2);
    fill_p2_deck(&tg, bottom, 10);
    test_recalc(&tg);

    CHECK_EQ(rb_perform_cheer_check(&tg.state, "p1", 5), 0, "a short deck must not fail the yell check");
    CHECK(same_set(tg.state.resolution.cards, tg.state.resolution.n, only, 2),
          "the short deck reveals only the 2 available cards");
}

static void test_bottom_yell_source_two_yells(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int aquarium = test_id(&tg, AQUARIUM);
    int t = test_id(&tg, DECK_TOP);
    int b = test_id(&tg, DECK_BOTTOM);

    test_add_to_live(&tg, aquarium);
    const int order[6] = { t, t, t, b, b, b };
    set_deck_order(&tg, order, 6);
    fill_p2_deck(&tg, t, 10);
    test_recalc(&tg);

    CHECK_EQ(rb_perform_cheer_check(&tg.state, "p1", 2), 0, "the first yell resolves");
    int first[8], nfirst = tg.state.resolution.n;
    for (int i = 0; i < nfirst && i < 8; i++) first[i] = tg.state.resolution.cards[i];
    tg.state.resolution.n = 0;

    CHECK_EQ(rb_perform_cheer_check(&tg.state, "p1", 1), 0, "the second yell resolves");
    int second[8], nsecond = tg.state.resolution.n;
    for (int i = 0; i < nsecond && i < 8; i++) second[i] = tg.state.resolution.cards[i];

    CHECK_EQ(nfirst, 2, "the first yell (blade 2) reveals 2");
    CHECK_EQ(nsecond, 1, "the second yell (blade 1) reveals 1");
    int first_all_b = 1, second_all_b = 1;
    for (int i = 0; i < nfirst; i++) if (first[i] != b) first_all_b = 0;
    for (int i = 0; i < nsecond; i++) if (second[i] != b) second_all_b = 0;
    CHECK(first_all_b && second_all_b,
          "both yells consume from the bottom first, the deck is not reset");
    CHECK_EQ(tg.state.p[0].deck.n, 3, "3 bottom cards consumed across the two yells");
}

static void test_bottom_yell_source_leaves_zone_reverts_to_top(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int aquarium = test_id(&tg, AQUARIUM);
    int top = test_id(&tg, DECK_TOP);
    int bottom = test_id(&tg, DECK_BOTTOM);

    test_add_to_live(&tg, aquarium);
    const int order[6] = { top, top, top, bottom, bottom, bottom };
    set_deck_order(&tg, order, 6);
    fill_p2_deck(&tg, top, 10);
    test_recalc(&tg);
    CHECK(tg.state.p[0].yell_from_bottom, "setup: the source is the bottom while AQUARIUM is in the zone");

    tg.state.p[0].live.n = 0;                 /* AQUARIUM leaves the live zone */
    test_recalc(&tg);
    CHECK(!tg.state.p[0].yell_from_bottom, "after AQUARIUM leaves, the source reverts to the top");

    CHECK_EQ(rb_perform_cheer_check(&tg.state, "p1", 3), 0, "the yell resolves");
    CHECK(same_set(tg.state.resolution.cards, tg.state.resolution.n, order, 3),
          "after AQUARIUM leaves, the yell reveals the TOP cards again");
}

static void test_bottom_yell_source_in_success_zone_applies(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int aquarium = test_id(&tg, AQUARIUM);
    int top = test_id(&tg, DECK_TOP);
    int bottom = test_id(&tg, DECK_BOTTOM);

    test_add_to_success(&tg, aquarium);        /* the SUCCESS zone, not the live zone */
    const int order[6] = { top, top, top, bottom, bottom, bottom };
    set_deck_order(&tg, order, 6);
    fill_p2_deck(&tg, top, 10);
    test_recalc(&tg);

    CHECK(tg.state.p[0].yell_from_bottom, "setup: AQUARIUM in the SUCCESS zone still forces the bottom");
    CHECK_EQ(rb_perform_cheer_check(&tg.state, "p1", 3), 0, "the yell resolves");
    const int want[3] = { bottom, bottom, bottom };
    CHECK(same_set(tg.state.resolution.cards, tg.state.resolution.n, want, 3),
          "AQUARIUM in the success zone still forces yell-from-bottom");
}

/* =====================================================================
 * 3. discard_low_cost_group_member_trigger_debut_q108_q240_test.rs -- the
 *    three cases test_parity_ability_mod.c does NOT cover.
 *
 * Fixture note: Kinako is staged, NOT left in hand as well. The Rust plays
 * her to the stage, which takes her out of the hand; keeping her in hand as
 * well makes her cost-10 self an eligible-looking hand card.
 *
 * The cost prompt is not asserted here: with exactly one hand card inside the
 * cost-4 limit the C engine resolves it without opening a SelectCard (the
 * sibling suite asserts that prompt and is red for it). The behaviour under
 * test in these three cases is the WAITROOM debut and its centre gate.
 * ===================================================================== */

#define KINAKO_P "PL!SP-bp2-006-P"
#define SUMIRE  "PL!SP-bp5-015-N"   /* cost 4, Liella!, centre-gated debut -> blade+2 */
#define WAKANA  "PL!SP-sd1-008-SD"   /* cost 4, Liella!, debut not centre-gated */

static int kinako_on_center_with_cost_card(TestGame *tg, int cost_card)
{
    int kinako = test_id(tg, KINAKO_P);
    int filler = test_id(tg, "PL!-sd1-010-SD");
    fill_decks(tg, filler, 20);
    test_give_energy(tg, 10);
    test_add_to_hand(tg, cost_card);
    tg->state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg->state.p[0].stage[1] = kinako;
    tg->state.p[0].stage[2] = RB_EMPTY_SLOT;
    return kinako;
}

/* Q108/Q240: Sumire's debut fires from the waitroom -> the centre check fails
 * -> no blade+2. Proves the triggered ability belongs to SUMIRE, not Kinako. */
static void test_kinako_waitroom_sumire_debut_center_gate_fails(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int sumire = test_id(&tg, SUMIRE);
    int kinako = kinako_on_center_with_cost_card(&tg, sumire);

    CHECK_EQ(card_printed_cost(sumire), 4, "setup: Sumire is inside the cost-4 limit");
    CHECK(card_prints_trigger(sumire, RB_TSTR_DEBUT), "setup: Sumire really prints a debut ability");
    int blade_before = test_get_blade_modifier(&tg, kinako);

    CHECK(test_activate_ability(&tg, kinako), "Kinako's activation ability activates");
    decline_choices(&tg);

    CHECK(test_zone_has_id(&tg, 0, "discard", sumire), "Q240: Sumire is discarded as the cost");
    CHECK_EQ(test_get_blade_modifier(&tg, kinako), blade_before,
             "Q240: Sumire's centre-gated debut from the waitroom must NOT grant blade+2");
    CHECK_EQ(test_get_blade_modifier(&tg, sumire), 0,
             "Q240: Sumire must not gain blade+2 from a waitroom debut");
}

/* The same, across every Kinako rarity the Rust test enumerates. */
static void test_kinako_waitroom_debut_gate_all_rarities(void)
{
    static const char *const suffixes[] = { "P", "P＋", "R＋", "SEC" };
    for (int i = 0; i < 4; i++) {
        char card_no[40];
        char msg[192];
        snprintf(card_no, sizeof card_no, "PL!SP-bp2-006-%s", suffixes[i]);
        static TestGame tg;
        test_game_new(&tg);
        int kinako = test_id(&tg, card_no);
        int sumire = test_id(&tg, SUMIRE);
        int filler = test_id(&tg, "PL!-sd1-010-SD");

        snprintf(msg, sizeof msg, "%s: the rarity resolves to its own print", card_no);
        CHECK(rb_card_no_eq(kinako, card_no), msg);

        fill_decks(&tg, filler, 20);
        test_give_energy(&tg, 10);
        test_add_to_hand(&tg, sumire);
        tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
        tg.state.p[0].stage[1] = kinako;
        tg.state.p[0].stage[2] = RB_EMPTY_SLOT;

        int blade_before = test_get_blade_modifier(&tg, kinako);
        snprintf(msg, sizeof msg, "%s: the activation ability activates", card_no);
        CHECK(test_activate_ability(&tg, kinako), msg);
        decline_choices(&tg);

        snprintf(msg, sizeof msg, "%s: Q240 Sumire is discarded as the cost", card_no);
        CHECK(test_zone_has_id(&tg, 0, "discard", sumire), msg);
        snprintf(msg, sizeof msg, "%s: Q240 a waitroom debut must not grant blade to Kinako", card_no);
        CHECK_EQ(test_get_blade_modifier(&tg, kinako), blade_before, msg);
        snprintf(msg, sizeof msg, "%s: Q240 Sumire gains no blade in the waitroom", card_no);
        CHECK_EQ(test_get_blade_modifier(&tg, sumire), 0, msg);
    }
}

/* Control: a Liella! cost card whose debut is NOT centre-gated is discarded
 * and the followup path runs, so the negative above is the centre gate and not
 * a broken triggered-debut path. */
static void test_kinako_non_center_cost_card_reaches_waitroom(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int wakana = test_id(&tg, WAKANA);
    int kinako = kinako_on_center_with_cost_card(&tg, wakana);

    CHECK_EQ(card_printed_cost(wakana), 4, "setup: the control card is inside the cost-4 limit");
    CHECK(card_prints_trigger(wakana, RB_TSTR_DEBUT), "setup: the control card really prints a debut ability");

    CHECK(test_activate_ability(&tg, kinako), "Kinako's activation ability activates");
    decline_choices(&tg);

    CHECK(test_zone_has_id(&tg, 0, "discard", wakana),
          "Q240 control: the non-centre-gated cost card is discarded as the cost");
}

/* =====================================================================
 * 4. energy_cost_aqours_score_pl_s_bp6_007_r_test.rs -- the identity half.
 *    The success-count matrix is already in test_parity_ability_mod.c; what
 *    is NOT covered is the card-identity pin and the hand/energy invariants.
 * ===================================================================== */

static void test_hanamaru_q_energy_cost_distinct_aqours_identities(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int hanamaru = test_id(&tg, "PL!S-bp6-007-R");
    int friend = test_id(&tg, "PL!S-pb1-010-PR");
    int outsider = test_id(&tg, "PL!-sd1-010-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");

    /* The identity pin that caught the old fixture staging two copies of one
     * character: PL!S-bp6-007-R and PL!S-bp1-007-R are one bp number apart. */
    CHECK(rb_card_no_eq(hanamaru, "PL!S-bp6-007-R"), "setup: Hanamaru resolves to her own print");
    CHECK(rb_card_no_eq(friend, "PL!S-pb1-010-PR"), "setup: the friend resolves to her own print");
    CHECK(strcmp(test_card_name(hanamaru), test_card_name(friend)) != 0,
          "setup: Hanamaru and her Aqours friend really are two different characters");
    CHECK(strcmp(test_card_name(hanamaru), test_card_name(outsider)) != 0,
          "setup: the non-Aqours fixture is a different character again");

    test_add_to_stage(&tg, 1, hanamaru);
    test_add_to_stage(&tg, 0, friend);
    test_add_to_stage(&tg, 2, outsider);
    test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_add_to_opp_success(&tg, test_new_id(&tg, "PL!-sd1-019-SD"));
    test_give_energy(&tg, 2);
    fill_decks(&tg, filler, 20);
    int hand_before = tg.state.p[0].hand.n;
    test_recalc(&tg);

    CHECK(fire_trigger(&tg, hanamaru, RB_TSTR_LIVE_START), "the live-start ability is queued and runs");
    decline_choices(&tg);

    CHECK(tg.state.p[0].energy_active <= 2, "sanity: the energy zone is never over-activated");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before, "the energy payment leaves the hand alone");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 2,
             "up to TWO Aqours members each gain live-total score +1 "
             "(the non-Aqours member on the right is not counted)");
}

/* =====================================================================
 * 5. reveal_hand_cost_total_..._test.rs -- the turn-1 half.
 * ===================================================================== */

static void test_chisato_turn1_blocks_second_activation(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chisato = test_id(&tg, "PL!SP-bp1-003-P");
    int cost4 = test_id(&tg, "PL!S-bp2-002-R");
    int cost2 = test_id(&tg, "PL!-sd1-002-SD");
    int filler = test_new_id(&tg, "PL!-sd1-010-SD");

    CHECK_EQ(card_printed_cost(cost4), 4, "setup: the cost-4 member really costs 4");
    CHECK_EQ(card_printed_cost(cost2), 2, "setup: the cost-2 member really costs 2");

    tg.state.p[0].stage[0] = chisato;
    tg.state.p[0].hand.n = 0;
    fill_decks(&tg, filler, 20);
    test_add_to_hand(&tg, cost4);
    test_add_to_hand(&tg, cost4);
    test_add_to_hand(&tg, cost2);
    test_give_energy(&tg, 20);

    /* First activation: 4 + 4 + 2 = 10, which IS on the printed ladder. */
    CHECK(test_activate_ability(&tg, chisato), "the first activation is accepted");
    CHECK(test_has_pending_choice(&tg), "the reveal prompt opens");
    test_select_indices(&tg, NULL, 0);
    decline_choices(&tg);
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1, "the first activation granted the +1");

    /* once-per-turn -- the use is recorded, so the second activation is refused. */
    CHECK(rb_use_count(&tg.state.queue, chisato, 0, tg.state.turn) >= 1,
          "the once-per-turn use is recorded for this turn");
    CHECK(!test_activate_ability(&tg, chisato),
          "once-per-turn: the second activation in the same turn must be refused");
    decline_choices(&tg);
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "a refused second activation must not grant a second +1");
}

/* =====================================================================
 * 6. nonempty_success_zone_..._test.rs -- the P-rarity case.
 * ===================================================================== */

static void test_nozomi_p_rarity_same_behaviour(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int nozomi = test_id(&tg, "PL!-bp4-007-P");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int live1 = test_id(&tg, "PL!-sd1-019-SD");

    CHECK(rb_card_no_eq(nozomi, "PL!-bp4-007-P"),
          "setup: the P print is a DIFFERENT card from the R print");
    CHECK(card_prints_trigger(nozomi, RB_TSTR_DEBUT), "setup: the P print really prints a debut ability");

    test_add_to_success(&tg, live1);
    test_add_to_stage(&tg, 1, nozomi);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);

    CHECK(fire_trigger(&tg, nozomi, RB_TSTR_DEBUT), "the P print's debut ability runs on the gate board");
    decline_choices(&tg);
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "P rarity: success zone non-empty AND total score 1 <= 1 -> the +1 is granted");
}

/* =====================================================================
 * 7. center_gated_wait_member_gain_total_score_test.rs -- the use record.
 * ===================================================================== */

static void test_chika_center_activation_records_the_use(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int chika = test_id(&tg, "PL!S-bp3-001-R＋");

    tg.state.p[0].stage[0] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[1] = chika;
    tg.state.p[0].stage[2] = RB_EMPTY_SLOT;
    test_give_energy(&tg, 10);
    test_recalc(&tg);

    CHECK_EQ(rb_use_count(&tg.state.queue, chika, 0, tg.state.turn), 0,
             "setup guard: nothing has used the activation ability yet");
    CHECK(test_activate_ability(&tg, chika), "activating from the centre succeeds");
    decline_choices(&tg);

    const char *ori = rb_mods_get_orientation(&tg.state.mods, chika);
    CHECK(ori && !strcmp(ori, "wait"), "the centre cost waits Chika herself");
    CHECK(rb_use_count(&tg.state.queue, chika, 0, tg.state.turn) >= 1,
          "activating from the centre must record the use, so the two "
          "out-of-position negatives fail for position and not for a missing ability");
}

/* =====================================================================
 * 8. full_distinct_group_stage_grants_yell_live_score_test.rs
 *    PL!S-bp2-008-R+ -- all three stage areas hold Aqours members with
 *    DIFFERENT names -> the yell's live-card count adds to the live total.
 *
 * The C model keeps this grant in GameState.delayed_gained_effects, not in the
 * gained-ability text table, so the positive case is asserted on the delayed
 * table (the C counterpart of Rust delayed_gained_effects) and the negatives on
 * both tables.
 * ===================================================================== */

#define MARI    "PL!S-bp2-008-R＋"
#define RIKO    "PL!S-bp2-011-N"
#define DIA     "PL!S-bp2-013-N"
#define CHIKA_SD "PL!S-sd1-001-SD"

static void setup_stage_with_3_aqours(TestGame *tg, int *out_mari, int *out_ids)
{
    int mari = test_id(tg, MARI);
    int riko = test_id(tg, RIKO);
    int dia = test_id(tg, DIA);
    test_add_to_stage(tg, 0, mari);
    test_add_to_stage(tg, 1, riko);
    test_add_to_stage(tg, 2, dia);
    if (out_mari) *out_mari = mari;
    if (out_ids) { out_ids[0] = mari; out_ids[1] = riko; out_ids[2] = dia; }
}

/* the delayed gained effect owned by `cid`, or -1 */
static int delayed_index_for(const GameState *g, int cid)
{
    for (int i = 0; i < g->n_delayed_gained_effects; i++)
        if (g->delayed_gained_effects[i].card_id == cid) return i;
    return -1;
}

static void test_all_areas_aqours_diff_names_gains_ability(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int ids[3];
    setup_stage_with_3_aqours(&tg, NULL, ids);

    CHECK(rb_card_no_eq(ids[0], MARI), "setup: the host resolves to its own print");
    CHECK(strcmp(test_card_name(ids[0]), test_card_name(ids[1])) != 0 &&
              strcmp(test_card_name(ids[0]), test_card_name(ids[2])) != 0 &&
              strcmp(test_card_name(ids[1]), test_card_name(ids[2])) != 0,
          "setup: the three staged members really have DIFFERENT names");

    test_add_to_hand(&tg, filler);
    fill_decks(&tg, filler, 30);
    test_recalc(&tg);

    CHECK(delayed_index_for(&tg.state, ids[0]) >= 0,
          "the different-names Aqours condition is met, so the host's gained "
          "ability is registered");
    CHECK_EQ(tg.state.n_delayed_gained_effects, 1,
             "the delayed gained effect is stored for the performance, not applied now");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "no live-total bonus before any yell has revealed anything");
}

static void test_repeated_recalculation_registers_delayed_ability_once(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int mari = 0;
    setup_stage_with_3_aqours(&tg, &mari, NULL);
    fill_decks(&tg, test_id(&tg, "PL!-sd1-010-SD"), 30);

    char msg[96];
    for (int i = 0; i < 5; i++) {
        test_recalc(&tg);
        snprintf(msg, sizeof msg, "recalc #%d registers exactly one delayed gained effect", i);
        CHECK_EQ(tg.state.n_delayed_gained_effects, 1, msg);
        snprintf(msg, sizeof msg, "recalc #%d: the delayed gained effect belongs to the host card", i);
        CHECK(delayed_index_for(&tg.state, mari) == 0, msg);
        snprintf(msg, sizeof msg, "recalc #%d grants no live-total bonus yet", i);
        CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0, msg);
        snprintf(msg, sizeof msg, "recalc #%d grants the opponent nothing", i);
        CHECK_EQ(tg.state.mods.p2_constant_total_score_bonus, 0, msg);
    }
}

static void test_condition_loss_removes_delayed_registration(void)
{
    for (int changed_area = 0; changed_area <= 2; changed_area += 2) {
        static TestGame tg;
        test_game_new(&tg);
        int mari = 0;
        setup_stage_with_3_aqours(&tg, &mari, NULL);
        fill_decks(&tg, test_id(&tg, "PL!-sd1-010-SD"), 30);
        test_recalc(&tg);
        CHECK_EQ(tg.state.n_delayed_gained_effects, 1, "setup: the condition holds to start with");

        int removed = tg.state.p[0].stage[changed_area];
        tg.state.p[0].stage[changed_area] = RB_EMPTY_SLOT;
        test_add_to_discard(&tg, removed);

        char msg[96];
        for (int step = 0; step < 3; step++) {
            test_recalc(&tg);
            snprintf(msg, sizeof msg, "area %d removed, step %d: the delayed registration is gone",
                     changed_area, step);
            CHECK_EQ(tg.state.n_delayed_gained_effects, 0, msg);
            snprintf(msg, sizeof msg, "area %d removed, step %d: the host gains nothing",
                     changed_area, step);
            CHECK_EQ(rb_card_num_gained_abilities(&tg.state, mari), 0, msg);
        }

        /* restore */
        tg.state.p[0].discard.n = 0;
        tg.state.p[0].stage[changed_area] = removed;
        for (int step = 0; step < 3; step++) {
            test_recalc(&tg);
            snprintf(msg, sizeof msg, "area %d restored, step %d: exactly one registration",
                     changed_area, step);
            CHECK_EQ(tg.state.n_delayed_gained_effects, 1, msg);
        }
    }
}

static void test_empty_area_fails_condition(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int mari = test_id(&tg, MARI);
    int chika = test_id(&tg, CHIKA_SD);
    CHECK(rb_card_no_eq(mari, MARI), "setup: the host resolves to its own print");
    test_add_to_stage(&tg, 0, mari);
    test_add_to_stage(&tg, 1, chika);
    test_recalc(&tg);

    /* Setup guard: the RIGHT area must really be the empty one, or the
     * assertion below would pass for any reason. */
    CHECK_EQ(tg.state.p[0].stage[2], RB_EMPTY_SLOT, "setup guard: the right area is the empty one");
    CHECK_EQ(tg.state.n_delayed_gained_effects, 0, "an empty area fails the condition");
    CHECK_EQ(rb_card_num_gained_abilities(&tg.state, mari), 0,
             "an empty area fails the condition: nothing is gained");
}

static void test_duplicate_names_fails_condition(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int mari = test_id(&tg, MARI);
    int mari2 = test_new_id(&tg, MARI);
    int chika = test_id(&tg, CHIKA_SD);
    CHECK(rb_card_no_eq(mari, MARI), "setup: the host resolves to its own print");
    /* the different-names condition is under test, so prove the two instances
     * really share one name and are two separate cards. */
    CHECK(!strcmp(test_card_name(mari), test_card_name(mari2)),
          "setup: the two instances really share one name");
    CHECK(mari != mari2, "setup: they are two separate card instances");

    test_add_to_stage(&tg, 0, mari);
    test_add_to_stage(&tg, 1, mari2);
    test_add_to_stage(&tg, 2, chika);
    test_recalc(&tg);

    CHECK(delayed_index_for(&tg.state, mari) < 0,
          "duplicate names fail the different-names condition: the left Mari gains nothing");
    CHECK(delayed_index_for(&tg.state, mari2) < 0,
          "duplicate names fail the different-names condition: the centre Mari gains nothing");
    CHECK_EQ(tg.state.n_delayed_gained_effects, 0,
             "duplicate names fail the different-names condition: nothing is registered at all");
    CHECK_EQ(rb_card_num_gained_abilities(&tg.state, mari), 0,
             "duplicate names fail the different-names condition: nothing is gained");
}

static void test_non_aqours_member_fails_condition(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int mari = test_id(&tg, MARI);
    int chika = test_id(&tg, CHIKA_SD);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int aqours_ref = test_id(&tg, RIKO);
    CHECK(rb_card_no_eq(mari, MARI), "setup: the host resolves to its own print");
    CHECK(rb_card_no_eq(filler, "PL!-sd1-010-SD"), "setup: the non-Aqours fixture resolves");
    /* Group matching is SERIES-based, so pin the contrast by card name. */
    CHECK(strcmp(test_card_name(filler), test_card_name(aqours_ref)) != 0,
          "setup: the non-Aqours fixture is a different card from an Aqours member");

    test_add_to_stage(&tg, 0, mari);
    test_add_to_stage(&tg, 1, chika);
    test_add_to_stage(&tg, 2, filler);
    test_recalc(&tg);

    CHECK_EQ(tg.state.n_delayed_gained_effects, 0, "a non-Aqours member on stage fails the condition");
    CHECK_EQ(rb_card_num_gained_abilities(&tg.state, mari), 0,
             "a non-Aqours member on stage fails the condition: nothing is gained");
}

/* -- the delayed effect, applied against a chosen reveal set ---------
 * The Rust harness helper evaluate_delayed_mari pulls Mari's delayed gained
 * effect out of the table, evaluates its condition against
 * state.revealed_cards and runs the matching branch. The C equivalent
 * detaches the same struct and hands it to the real executor. Returns 1 when
 * a branch was executed, 0 when nothing was stowed, -1 when the host owns no
 * delayed effect (a broken precondition, asserted by every caller). */
static int evaluate_delayed_host(TestGame *tg, int card_id)
{
    int idx = delayed_index_for(&tg->state, card_id);
    if (idx < 0) return -1;
    AbilityEffect *eff = tg->state.delayed_gained_effects[idx].effect;
    for (int i = idx; i < tg->state.n_delayed_gained_effects - 1; i++)
        tg->state.delayed_gained_effects[i] = tg->state.delayed_gained_effects[i + 1];
    tg->state.n_delayed_gained_effects--;
    if (!eff) return 0;
    int owner = rb_owner_of_card(&tg->state, card_id);
    if (owner < 0) { rb_effect_free(eff); return 0; }
    tg->state.activating_card = card_id;
    rb_execute_effect_ex(&tg->state, owner, eff, card_id);
    rb_drain_ability_queue(&tg->state);
    rb_effect_free(eff);
    return 1;
}

static void test_zero_live_cards_no_bonus(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int live_card = test_id(&tg, "PL!SP-sd1-023-SD");
    int mari = 0;
    setup_stage_with_3_aqours(&tg, &mari, NULL);
    test_add_to_live(&tg, live_card);
    fill_decks(&tg, filler, 20);
    test_recalc(&tg);

    test_add_to_revealed(&tg, filler);   /* 0 live cards among the reveals */
    CHECK_EQ(evaluate_delayed_host(&tg, mari), 1,
             "precondition: the host owns a delayed gained effect to evaluate");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 0,
             "no live-total bonus without live cards among the reveals");
    CHECK_EQ(test_get_score_modifier(&tg, live_card), 0,
             "no per-card modifier without live cards among the reveals");
    CHECK_EQ(test_get_score_modifier(&tg, mari), 0,
             "no per-card modifier on the host without live cards among the reveals");
}

static void test_one_live_card_plus_one(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int live_card = test_id(&tg, "PL!SP-sd1-023-SD");
    int mari = 0;
    setup_stage_with_3_aqours(&tg, &mari, NULL);
    test_add_to_live(&tg, live_card);
    fill_decks(&tg, filler, 20);
    test_recalc(&tg);

    test_add_to_revealed(&tg, live_card);
    CHECK_EQ(evaluate_delayed_host(&tg, mari), 1,
             "precondition: the host owns a delayed gained effect to evaluate");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 1,
             "1 live card among the reveals -> live total score +1");
    CHECK_EQ(test_get_score_modifier(&tg, live_card), 0,
             "the printed target is the live total score, not a per-card modifier");
    CHECK_EQ(test_get_score_modifier(&tg, mari), 0,
             "the printed target is the live total score, not a per-card modifier");
}

static void test_three_live_cards_plus_two(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int live_card = test_id(&tg, "PL!SP-sd1-023-SD");
    int mari = 0;
    setup_stage_with_3_aqours(&tg, &mari, NULL);
    test_add_to_live(&tg, live_card);
    fill_decks(&tg, filler, 20);
    test_recalc(&tg);

    test_add_to_revealed(&tg, live_card);
    test_add_to_revealed(&tg, live_card);
    test_add_to_revealed(&tg, live_card);
    CHECK_EQ(evaluate_delayed_host(&tg, mari), 1,
             "precondition: the host owns a delayed gained effect to evaluate");
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 2,
             "3 live cards among the reveals -> the alternative +2 applies once");
    CHECK_EQ(test_get_score_modifier(&tg, live_card), 0,
             "the printed target is the live total score, not a per-card modifier");
    CHECK_EQ(test_get_score_modifier(&tg, mari), 0,
             "the printed target is the live total score, not a per-card modifier");
}

/* =====================================================================
 * 9. genki_zenkai_invalidates_own_live_success_test.rs -- the CONTROL:
 *    below the threshold the LiveSuccess is not invalidated and it really
 *    resolves, giving the opponent an energy card.
 * ===================================================================== */

#define GENKI "PL!S-pb1-019-L"

static void test_genki_live_success_fires_when_condition_not_met(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int genki = test_id(&tg, GENKI);
    int chika = test_id(&tg, "PL!S-sd1-010-SD");
    int ruby = test_id(&tg, "PL!S-pb1-018-N");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int energy = test_id(&tg, "LL-E-001-SD");

    test_add_to_stage(&tg, 0, chika);
    test_add_to_stage(&tg, 1, ruby);
    test_add_to_stage(&tg, 2, filler);

    /* Opponent energy deck -- the effect moves a real energy card. */
    tg.state.p[1].energy_deck.n = 0;
    for (int i = 0; i < 10; i++) tg.state.p[1].energy_deck.cards[tg.state.p[1].energy_deck.n++] = energy;
    fill_decks(&tg, filler, 30);

    CHECK(test_advance_to_phase(&tg, RB_PHASE_LIVE_SET), "the turn reaches the live-card set phase");
    test_add_to_hand(&tg, genki);
    test_set_live_card(&tg, 0, genki);
    /* The opponent's energy zone is already fed by the energy phase, so the
     * Rust's bare ">= 1" would pass even with the ability dead. Pin the
     * observable as a DELTA across the performance instead. */
    int energy_before = tg.state.p[1].energy.n;
    CHECK(test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE), "the turn reaches the performance phase");
    decline_choices(&tg);

    CHECK(!rb_ability_is_invalidated(&tg.state, genki, RB_TSTR_LIVE_SUCCESS),
          "the heart02 total is below 6, so LiveSuccess is NOT invalidated");

    for (int i = 0; i < 3; i++) { test_pass(&tg); decline_choices(&tg); }

    CHECK(tg.state.p[1].energy.n > energy_before,
          "an un-invalidated LiveSuccess must resolve and give the opponent an energy card");
}

/* =====================================================================
 * 10. live_success_invalidation_and_live_start_suppression_test.rs -- the
 *     Butterfly half that is NOT in test_parity_ability_mod.c: suppression
 *     must not stop the LiveSuccess from scoring off the SAME board.
 * ===================================================================== */

#define BUTTERFLY "PL!SP-pb2-046-L"

static void test_butterfly_suppresses_live_start_but_live_success_still_scores(void)
{
    static TestGame tg;
    test_game_new(&tg);
    int butterfly = test_id(&tg, BUTTERFLY);
    int mei_r = test_id(&tg, "PL!SP-pb1-007-R");
    int mei_p = test_id(&tg, "PL!SP-pb1-007-P＋");
    int keke = test_id(&tg, "PL!SP-pb1-013-N");

    test_add_to_stage(&tg, 0, mei_r);
    test_add_to_stage(&tg, 1, mei_p);
    test_add_to_stage(&tg, 2, keke);
    test_give_energy(&tg, 3);
    test_set_energy_active(&tg, 0, 0);   /* all three rest */

    fill_decks(&tg, keke, 30);

    CHECK(test_advance_to_phase(&tg, RB_PHASE_LIVE_SET), "the turn reaches the live-card set phase");
    test_add_to_hand(&tg, butterfly);
    test_set_live_card(&tg, 0, butterfly);
    CHECK(test_advance_to_phase(&tg, RB_PHASE_PERFORMANCE), "the turn reaches the performance phase");
    decline_choices(&tg);

    CHECK_EQ(tg.state.p[0].energy_active, 0,
             "the suppressed LiveStart must not activate energy");

    for (int i = 0; i < 3; i++) { test_pass(&tg); decline_choices(&tg); }

    const RbLiveSnapshot *snapshot = snapshot_for(&tg, 0);
    if (!snapshot) {
        CHECK(0, "a performance snapshot must exist for the owner");
        return;
    }
    int printed = 0;
    for (int j = 0; j < snapshot->n_lives; j++) printed += card_printed_score(snapshot->lives[j]);
    CHECK_EQ(snapshot->n_lives, 1, "the performance holds exactly the Butterfly Wing live");
    CHECK_EQ(snapshot->total_score - printed, 1,
             "the LiveSuccess must still add +1 for the LiveStart member on stage, even though "
             "the LiveStart itself was suppressed");
}

/* ===================================================================== */

int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    if (getenv("ABMODX_DEBUG")) rb_ability_debug_set(1);
    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: card database load\n");
        return 1;
    }
#define RUN(fn) do { printf("-- %s\n", #fn); fn(); } while (0)
    RUN(test_chisato_bp5_cost_10_needs_10_energy);
    RUN(test_chisato_bp5_cost_10_fails_with_9);
    RUN(test_chisato_bp5_cross_card_reduction_8_works);
    RUN(test_chisato_bp5_cross_card_reduction_7_fails);
    RUN(test_chisato_bp5_self_cost_not_reduced);
    RUN(test_chisato_promo_ab0_cross_card_reduction);
    RUN(test_chisato_promo_ab1_live_start);
    RUN(test_kinako_baton_only_debut_fires_on_a_plain_play);
    RUN(test_bottom_yell_source_yell_from_bottom);
    RUN(test_no_yell_source_modifier_reveals_from_top);
    RUN(test_opponent_yell_source_modifier_does_not_change_own_yell);
    RUN(test_bottom_yell_source_short_deck);
    RUN(test_bottom_yell_source_two_yells);
    RUN(test_bottom_yell_source_leaves_zone_reverts_to_top);
    RUN(test_bottom_yell_source_in_success_zone_applies);
    RUN(test_kinako_waitroom_sumire_debut_center_gate_fails);
    RUN(test_kinako_waitroom_debut_gate_all_rarities);
    RUN(test_kinako_non_center_cost_card_reaches_waitroom);
    RUN(test_hanamaru_q_energy_cost_distinct_aqours_identities);
    RUN(test_chisato_turn1_blocks_second_activation);
    RUN(test_nozomi_p_rarity_same_behaviour);
    RUN(test_chika_center_activation_records_the_use);
    RUN(test_all_areas_aqours_diff_names_gains_ability);
    RUN(test_repeated_recalculation_registers_delayed_ability_once);
    RUN(test_condition_loss_removes_delayed_registration);
    RUN(test_empty_area_fails_condition);
    RUN(test_duplicate_names_fails_condition);
    RUN(test_non_aqours_member_fails_condition);
    RUN(test_zero_live_cards_no_bonus);
    RUN(test_one_live_card_plus_one);
    RUN(test_three_live_cards_plus_two);
    RUN(test_genki_live_success_fires_when_condition_not_met);
    RUN(test_butterfly_suppresses_live_start_but_live_success_still_scores);
    rb_unload();
    if (failures) {
        fprintf(stderr, "\n%d / %d ability_mod EXTRA parity checks FAILED\n", failures, checks);
        return 1;
    }
    printf("\nALL %d ABILITY_MOD EXTRA PARITY CHECKS PASSED\n", checks);
    return 0;
}
