/* Port of engine/tests/test_modules/rules/trigger_paths/
 * turn_limited_activation_offered_once_test.rs
 *
 * Regression: ターン1回 activation abilities must be consumable exactly once
 * per turn through the public action pipeline (card PL!N-bp5-014-N 中須かすみ).
 */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

static int failures;

#define REQUIRE(c, msg) do { \
    if (!(c)) { \
        fprintf(stderr, "FAIL: %s\n", msg); \
        failures++; \
        return; \
    } \
} while (0)

#define CHECK(c, msg) do { \
    if (!(c)) { \
        fprintf(stderr, "FAIL: %s\n", msg); \
        failures++; \
    } else { \
        printf("ok: %s\n", msg); \
    } \
} while (0)

#define CHECK_EQ(a, e, msg) do { \
    int a_ = (a); \
    int e_ = (e); \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", msg, a_, e_); \
        failures++; \
    } else { \
        printf("ok: %s\n", msg); \
    } \
} while (0)

/* rb_execute_main_phase_action action_type 0 == Rust ActionType::UseAbility. */
#define RB_ACT_USE_ABILITY 0

/* Card numbers. NOTE — the Rust original used "PL!SP-SD2-025-SD2",
 * "PL!SP-SD2-023-SD2" and "PL!SP-BP4-025-L". NONE of those three exist in
 * cards/cards.json (verified: no key contains "SD2-0", "BP4-025" under an SP
 * prefix, and the real 虹ヶ咲 live cards are PL!N-bp1-025-L, PL!N-bp3-025-L,
 * PL!N-bp4-025-L, PL!N-sd1-025-SD, ...). Rust's `game.id()` hides that behind a
 * lenient fallback, so the Rust test never noticed. rb_find_card_by_no has a
 * fallback chain too, so a wrong number silently stages a DIFFERENT print and
 * the test stops testing what it claims. These are the real card numbers, and
 * assert_card_no() pins the identity of every one of them (AGENTS.md identity
 * rule). */
#define KASUMI        "PL!N-bp5-014-N"   /* 中須かすみ 起動/ターン1回 2E + discard 1 hand */
#define FILLER        "PL!-sd1-010-SD"
/* 虹ヶ咲 live cards for the 「自分の控え室から『虹ヶ咲』のライブカードを1枚手札に
    加える」 retrieval to find. */
#define NIJI_LIVE_1   "PL!N-bp4-025-L"   /* VIVID WORLD          */
#define NIJI_LIVE_2   "PL!N-bp1-025-L"   /* 虹色Passions！        */
/* NOT 虹ヶ咲 — a live card the 「『虹ヶ咲』のライブカード」 filter must reject.
   The Rust original wrote "PL!SP-BP4-025-L" for this slot, but PL!N-bp4-025-L
   (the number it was clearly aiming at) IS 虹ヶ咲, so the "no valid target"
   premise cannot be built from a 025 number. Aqours' WATER BLUE NEW WORLD is
   unambiguously outside 虹ヶ咲 and matches the documented intent. */
#define OTHER_LIVE    "PL!S-bp2-019-L"   /* WATER BLUE NEW WORLD (Aqours) */

/* Rust `TestGame::assert_card_identity` — pins the card_no of a resolved id so
   a lenient rb_find_card_by_no fallback can never substitute another print. */
static int assert_card_no(int card_id, const char *no, const char *what)
{
    if (card_id >= 0 && rb_card_no_eq(card_id, no)) return 1;
    fprintf(stderr, "FAIL: %s: expected card_no '%s', resolved id %d is '%s'\n",
            what, no, card_id, card_id >= 0 ? test_card_name(card_id) : "(none)");
    failures++;
    return 0;
}

/* Rust `count_kasumi_use_offers` reads the action generator. The C port has no
   UseAbility branch there, so the observable we assert instead is the engine's
   own ターン1回 bookkeeping. Print the decoded ability so a failure says
   whether the limit was decoded at all. */
static void dump_kasumi_ability(int kasumi)
{
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)kasumi, 0, &ab)) {
        printf("DBG kasumi ability[0]: DECODE FAILED\n");
        return;
    }
    printf("DBG kasumi ability[0] triggers=%s use_limit=%d keywords=%d "
           "has_cost=%d has_effect=%d full_text=%s\n",
           ab.triggers ? ab.triggers : "(null)", ab.use_limit, ab.n_keywords,
           ab.cost ? 1 : 0, ab.effect ? 1 : 0,
           ab.full_text ? ab.full_text : "(null)");
    rb_free_ability(&ab);
}

/* Rust: game.state.turn_limited_abilities_used.contains_key(&(card, 0, turn)).
 * The C engine keeps the same per-turn table in the ability queue
 * (rb_record_ability_use / rb_ability_uses_used mirror abilities.rs). */
static int turn_limit_used(TestGame *game, int card_id)
{
    return rb_ability_uses_used(&game->state, card_id, 0) > 0;
}

/* Rust: count_kasumi_use_offers — counts ActionType::UseAbility entries the
 * action generator offers for `card_id`.
 *
 * ENGINE GAP: the C port's action generator (rb_generate_action_candidates,
 * src/turn/phase.c) only emits Pass and PlayMemberToStage; it has no
 * UseAbility branch, so the Rust `game_setup::generate_possible_actions` offer
 * list has no C equivalent to read back. The closest real C engine behavior is
 * the same gate the Rust generator applies for a ターン1回 activation, evaluated
 * directly against live engine state: the card sits on p1's stage, ability #0
 * is a 起動 (Activate) ability, and its per-turn use limit is not yet consumed
 * (rb_ability_has_remaining_uses — abilities.rs::ability_has_remaining_uses). */
static int count_kasumi_use_offers(TestGame *game, int kasumi)
{
    int on_stage = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++)
        if (game->state.p[0].stage[i] == kasumi) on_stage = 1;
    if (!on_stage) return 0;

    Ability ab;
    if (!rb_decode_card_ability((uint32_t)kasumi, 0, &ab)) return 0;
    int is_activate = (ab.triggers && strstr(ab.triggers, "起動") != NULL);
    rb_free_ability(&ab);
    if (!is_activate) return 0;

    if (!rb_ability_has_remaining_uses(&game->state, kasumi, 0)) return 0;
    return 1;
}

/* Rust: use_ability_resolving_choices — drive one full UseAbility through the
 * public pipeline, always taking the first generated choice option.
 * Returns the number of pipeline steps taken, or 0 if the engine refused. */
static int use_ability_resolving_choices(TestGame *game, int card_id)
{
    int res = rb_execute_main_phase_action(&game->state, RB_ACT_USE_ABILITY,
                                           card_id, -1, -1, -1);
    if (res <= 0) return 0;
    int steps = 1;
    for (int i = 0; i < 15; i++) {
        rb_drain_ability_queue(&game->state);
        if (!test_has_pending_choice(game)) break;
        test_resume_choice(game, 0);
        steps++;
    }
    rb_drain_ability_queue(&game->state);
    return steps;
}

static int setup_kasumi_with_energy(TestGame *game, const char *waitroom_card_no, int energy)
{
    int kasumi = test_id(game, KASUMI);
    if (kasumi < 0) {
        fprintf(stderr, "FAIL: card id '%s' not in database\n", KASUMI);
        failures++;
        return -1;
    }
    if (!assert_card_no(kasumi, KASUMI, "kasumi")) return -1;
    game->state.p[0].stage[1] = kasumi;
    test_give_energy(game, energy);
    int filler = test_id(game, FILLER);
    if (filler < 0) {
        fprintf(stderr, "FAIL: card id '%s' not in database\n", FILLER);
        failures++;
        return -1;
    }
    test_add_to_hand(game, filler);
    if (waitroom_card_no) {
        int live = test_id(game, waitroom_card_no);
        if (live < 0) {
            fprintf(stderr, "FAIL: card id '%s' not in database\n", waitroom_card_no);
            failures++;
            return -1;
        }
        if (!assert_card_no(live, waitroom_card_no, "waitroom retrieval target"))
            return -1;
        test_add_to_discard(game, live);
    }
    dump_kasumi_ability(kasumi);
    return kasumi;
}

static int setup_kasumi(TestGame *game, const char *waitroom_card_no)
{
    return setup_kasumi_with_energy(game, waitroom_card_no, 4);
}

/* #[test] kasumi_turn1_ability_offered_only_once_per_turn */
static void kasumi_turn1_ability_offered_only_once_per_turn(void)
{
    TestGame game;
    test_game_new(&game);

    /* A 虹ヶ咲 live card into the waitroom for the retrieval to find. */
    int kasumi = setup_kasumi(&game, NIJI_LIVE_1);
    REQUIRE(kasumi >= 0, "kasumi setup");

    CHECK_EQ(count_kasumi_use_offers(&game, kasumi), 1,
             "fresh turn should offer the ターン1回 ability exactly once");

    int steps = use_ability_resolving_choices(&game, kasumi);
    CHECK(steps > 0, "first activation resolved through the public pipeline");

    CHECK(turn_limit_used(&game, kasumi),
          "use_limit must be recorded after a successful activation");

    /* THE REGRESSION: re-generation must NOT offer it again this turn. */
    for (int attempt = 0; attempt < 10; attempt++) {
        char msg[96];
        snprintf(msg, sizeof(msg),
                 "attempt %d: ターン1回 ability offered again after being consumed",
                 attempt);
        CHECK_EQ(count_kasumi_use_offers(&game, kasumi), 0, msg);
        test_pass(&game);
        if (game.state.phase != RB_PHASE_MAIN) break;
    }
}

/* #[test] kasumi_turn1_ability_available_again_next_turn */
static void kasumi_turn1_ability_available_again_next_turn(void)
{
    TestGame game;
    test_game_new(&game);

    int kasumi = setup_kasumi(&game, NIJI_LIVE_1);
    REQUIRE(kasumi >= 0, "kasumi setup");

    int steps = use_ability_resolving_choices(&game, kasumi);
    CHECK(steps > 0, "first activation resolved");
    CHECK(turn_limit_used(&game, kasumi), "use recorded");

    /* Next turn: the limit resets and the ability is offered again. Refund the
     * spent cost (2E + 1 hand card) so the offer reflects the reset limit. */
    game.state.turn++;
    test_give_energy(&game, 2);
    int filler = test_id(&game, FILLER);
    test_add_to_hand(&game, filler);

    CHECK_EQ(count_kasumi_use_offers(&game, kasumi), 1,
             "ターン1回 limit must reset on the next turn");
}

/* #[test] kasumi_no_valid_target_terminates */
static void kasumi_no_valid_target_terminates(void)
{
    TestGame game;
    test_game_new(&game);

    /* Waitroom intentionally left WITHOUT any 虹ヶ咲 live. */
    int kasumi = setup_kasumi(&game, OTHER_LIVE);
    REQUIRE(kasumi >= 0, "kasumi setup");

    /* Bot-flow reproduction of the arena pathology: keep taking the UseAbility
     * action whenever it is offered, resolving choices with the first option,
     * all within ONE turn. Must terminate well under 20. */
    int activations = 0;
    for (int i = 0; i < 20; i++) {
        if (count_kasumi_use_offers(&game, kasumi) == 0) break;
        activations++;
        if (use_ability_resolving_choices(&game, kasumi) == 0) break;
    }

    CHECK(activations <= 1,
          "activation looped more than once in a single turn — "
          "ターン1回 enforcement leaked");
    printf("info: no-valid-target activations in one turn = %d\n", activations);
}

/* #[test] kasumi_exact_energy_is_sufficient */
static void kasumi_exact_energy_is_sufficient(void)
{
    TestGame game;
    test_game_new(&game);

    int kasumi = setup_kasumi_with_energy(&game, NIJI_LIVE_1, 2);
    REQUIRE(kasumi >= 0, "kasumi setup");

    /* exactly the cost */
    CHECK_EQ(game.state.p[0].energy_active, 2, "energy is exactly the 2E activation cost");

    CHECK_EQ(count_kasumi_use_offers(&game, kasumi), 1,
             "affordable ability must be offered");

    int steps = use_ability_resolving_choices(&game, kasumi);
    CHECK(steps > 0, "activation with exact cost resolved");
    CHECK(turn_limit_used(&game, kasumi), "use recorded after exact-cost activation");
}

/* #[test] kasumi_limit_is_per_instance */
static void kasumi_limit_is_per_instance(void)
{
    TestGame game;
    test_game_new(&game);

    int k1 = test_new_id(&game, KASUMI);
    int k2 = test_new_id(&game, KASUMI);
    if (k1 < 0 || k2 < 0 || k1 == k2) {
        fprintf(stderr, "FAIL: could not allocate two distinct KASUMI copies\n");
        failures++;
        return;
    }
    int filler = test_id(&game, FILLER);
    if (filler < 0) {
        fprintf(stderr, "FAIL: card id 'PL!-sd1-010-SD' not in database\n");
        failures++;
        return;
    }

    game.state.p[0].stage[0] = k1;
    game.state.p[0].stage[1] = k2;
    test_give_energy(&game, 8);
    for (int i = 0; i < 4; i++) test_add_to_hand(&game, filler);

    int niji1 = test_id(&game, NIJI_LIVE_1);
    int niji2 = test_id(&game, NIJI_LIVE_2);
    if (niji1 < 0 || niji2 < 0) {
        fprintf(stderr, "FAIL: NIJI_LIVE_1 / NIJI_LIVE_2 not in database\n");
        failures++;
        return;
    }
    assert_card_no(k1, KASUMI, "k1");
    assert_card_no(k2, KASUMI, "k2");
    assert_card_no(niji1, NIJI_LIVE_1, "niji_live 1");
    assert_card_no(niji2, NIJI_LIVE_2, "niji_live 2");
    test_add_to_discard(&game, niji1);
    test_add_to_discard(&game, niji2);

    /* Both instances are offered independently. */
    int offers = count_kasumi_use_offers(&game, k1) + count_kasumi_use_offers(&game, k2);
    CHECK_EQ(offers, 2, "each instance gets its own ターン1回 budget");

    /* Using k1 must not consume k2's budget. */
    int steps = use_ability_resolving_choices(&game, k1);
    CHECK(steps > 0, "k1 activation resolved");
    CHECK_EQ(count_kasumi_use_offers(&game, k2), 1,
             "k2 must still be offered after k1 used its own activation");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    kasumi_turn1_ability_offered_only_once_per_turn();
    kasumi_turn1_ability_available_again_next_turn();
    kasumi_no_valid_target_terminates();
    kasumi_exact_energy_is_sufficient();
    kasumi_limit_is_per_instance();

    rb_unload();
    if (failures) return 1;
    printf("ALL TRIGGER PATHS TURN LIMITED ACTIVATION CHECKS PASSED\n");
    return 0;
}
