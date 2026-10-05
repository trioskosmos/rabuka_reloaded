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
        FLAT_VERDICT(); \
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
        FLAT_VERDICT(); \
    } else { \
        printf("ok: %s\n", msg); \
    } \
} while (0)

/* Rust: ActionType::UseAbility (rb_execute_main_phase_action case 0). */
#define RB_ACT_USE_ABILITY 0

/* Rust: game.select_generated(0) / first option, bounded to 10 rounds. */
static void drain_first_option(TestGame *game)
{
    for (int i = 0; i < 10; i++) {
        if (!test_has_pending_choice(game)) break;
        test_resume_choice(game, 0);
    }
}

/* Rust `TestGame::assert_card_identity`. rb_find_card_by_no has a lenient
   fallback chain, so a mistyped card number silently stages a different print
   (AGENTS.md identity rule). Pin the number of every card this file stages. */
static int assert_card_no(int card_id, const char *no, const char *what)
{
    if (card_id >= 0 && rb_card_no_eq(card_id, no)) return 1;
    fprintf(stderr, "FAIL: %s: expected card_no '%s', resolved id %d is '%s'\n",
            what, no, card_id, card_id >= 0 ? test_card_name(card_id) : "(none)");
    failures++;
    return 0;
}

/* Print the decoded 起動 ability so a failure says whether the {{E}}{{E}} cost
   is decoded at all, or decoded-but-never-charged. */
static void dump_ability(int card_id, const char *what)
{
    int n = rb_card_num_abilities((uint32_t)card_id);
    printf("DBG %s id=%d name='%s' n_abilities=%d\n", what, card_id,
           test_card_name(card_id), n);
    for (int a = 0; a < n; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, a, &ab)) {
            printf("DBG   ability[%d]: DECODE FAILED\n", a);
            continue;
        }
        printf("DBG   ability[%d] triggers=%s use_limit=%d has_cost=%d has_effect=%d\n"
               "DBG     cost:   %s\n"
               "DBG     effect: %s\n"
               "DBG     text:   %s\n",
               a, ab.triggers ? ab.triggers : "(null)", ab.use_limit,
               ab.cost ? 1 : 0, ab.effect ? 1 : 0,
               ab.cost ? (ab.cost->action ? ab.cost->action : "(no action)") : "(none)",
               ab.effect ? (ab.effect->action ? ab.effect->action : "(no action)") : "(none)",
               ab.full_text ? ab.full_text : "(null)");
        rb_free_ability(&ab);
    }
}

static void print_stage(const TestGame *game)
{
    printf("stage [");
    for (int i = 0; i < 3; i++) {
        printf("%s%d", i == 0 ? "" : ",", game->state.p[0].stage[i]);
    }
    printf("] waitroom(");
    const RbBag *b = &game->state.p[0].discard;
    for (int i = 0; i < b->n; i++) {
        printf("%s%d", i == 0 ? "" : ",", b->cards[i]);
    }
    printf(")\n");
}

/* Rust `count_sayaka_offers` (activation_offer_bounded_and_terminates_test.rs:14-23),
 * verbatim:
 *     game_setup::generate_possible_actions(&game.state).into_iter()
 *       .filter(|a| a.action_type == ActionType::UseAbility
 *                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(sayaka))
 *       .count()
 *
 * C TWINS, all present in this tree:
 *   - `rb_generate_action_candidates` (include/rabuka.h:2061, src/turn/phase.c) is
 *     the C twin of `game_setup::generate_possible_actions`;
 *   - `RB_SETUP_ACT_USE_ABILITY` (include/rabuka.h:3719) is Rust's
 *     `ActionType::UseAbility`;
 *   - `RbGeneratedAction.parameters.card_id` is `parameters.card_id`;
 *   - the ability half of the Main-phase generator is
 *     `rb_setup_for_each_ability_candidate` (src/game/game_setup.c:1339), which
 *     game_setup.c:1320-1324 records as the bridge phase.c calls, added because
 *     "before this bridge existed it had no caller in the tree, so an activated
 *     ability could never be offered".
 *
 * WHY THIS HELPER EXISTS AT ALL: the previous revision of this file measured
 * the offer by PRESSING the card and treating a refusal as "no longer offered".
 * That was written when the C had no action generator to read.  It is not the
 * same observable -- Rust counts rows in the generated menu, this counted press
 * successes -- and it is what made `activations` overshoot Rust's bound.
 * `rb_generate_action_candidates` returns an EMPTY list unless the current phase
 * is Main (src/turn/phase.c:2156-2160), so the caller must park there; see the
 * `RB_PHASE_MAIN` assignments below.
 *
 * NOTE the returned `RbGeneratedActionList.actions` is a malloc'd block and the
 * tree exposes no free for it; these are process-lifetime test allocations. */
static int count_ability_offers(const GameState *st, int card_id)
{
    RbGeneratedActionList list = rb_generate_action_candidates(st);
    int n = 0;
    for (int i = 0; i < list.count; i++) {
        if (list.actions[i].action_type != RB_SETUP_ACT_USE_ABILITY) continue;
        if (!list.actions[i].has_parameters) continue;
        if (list.actions[i].parameters.card_id != card_id) continue;
        n++;
    }
    return n;
}

/// Rust: `sayaka_activation_charges_energy_and_terminates`
/// (activation_offer_bounded_and_terminates_test.rs:25-80).
///
/// Rust loops on `count_sayaka_offers`: `offers == 0` BREAKS, and every step with
/// `offers != 0` increments `activations`.  Its single assertion is
/// `activations <= 5` (:75-79).  So the bound is a bound on how many times the
/// card was OFFERED, i.e. on affordability, and the C now reads that directly
/// instead of counting presses.
static void sayaka_activation_charges_energy_and_terminates(void)
{
static    TestGame game;
    test_game_new(&game);

    int sayaka = test_id(&game, "PL!HS-bp1-002-R");
    if (sayaka < 0) {
        fprintf(stderr, "FAIL: card id 'PL!HS-bp1-002-R' not in database\n");
        failures++;
        return;
    }
    if (!assert_card_no(sayaka, "PL!HS-bp1-002-R", "sayaka")) return;
    dump_ability(sayaka, "sayaka");

    test_add_to_stage(&game, 1, sayaka);
    test_give_energy(&game, 9);

    /* Waitroom full of legal retrieval targets (蓮ノ空 members <=15 cost).
       Rust wraps each game.id() in catch_unwind and skips unknown card numbers;
       test_id() returns -1 for those, which is the same skip. */
    const char *target_nos[3] = {
        "PL!HS-bp1-002-R", "PL!HS-bp1-005-R", "PL!HS-bp1-006-P"
    };
    for (int i = 0; i < 3; i++) {
        int cid = test_id(&game, target_nos[i]);
        if (cid >= 0) {
            if (!assert_card_no(cid, target_nos[i], "sayaka retrieval target")) continue;
            test_add_to_discard(&game, cid);
        } else {
            printf("note: card no '%s' not in database, skipped\n", target_nos[i]);
        }
    }

    /* Rust dumps umi's cost structure for comparison (Q228 group-reduction shape). */
    int umi = test_id(&game, "PL!S-bp1-002-R＋");
    if (umi >= 0) {
        int n_abilities = rb_card_num_abilities((uint32_t)umi);
        for (int a = 0; a < n_abilities; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)umi, a, &ab)) continue;
            printf("DBG umi ability[%d] cost=%s\n", a, ab.cost ? "present" : "none");
            rb_free_ability(&ab);
        }
    } else {
        printf("note: card no 'PL!S-bp1-002-R＋' not in database, skipped\n");
    }

    printf("dbg sayaka id=%d member=%d n_abilities=%d\n", sayaka,
           rb_card_is_member((uint32_t)sayaka),
           rb_card_num_abilities((uint32_t)sayaka));

    int activations = 0;
    int all_charged_two = 1;
    int charged_samples = 0;
    /* Main is where the generator emits anything at all
     * (src/turn/phase.c:2156-2160: Rust matches on current_phase and returns
     * Vec::new() for Active/Energy/Draw/Performance phases). */
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;
    for (int step = 0; step < 20; step++) {
        int en_before = game.state.p[0].energy_active;
        int stage_before[3];
        for (int i = 0; i < 3; i++) stage_before[i] = game.state.p[0].stage[i];

        /* THE RUST LOOP CONDITION.  Rust :44-48 reads the generated menu and
         * breaks when the card is no longer offered; `activations` counts the
         * steps where it WAS offered.  This replaces the retired press proxy. */
        int offers = count_ability_offers(&game.state, sayaka);
        if (offers == 0) {
            printf("step %d: no longer offered (en=%d)\n", step, en_before);
            break;
        }
        activations++;
        int res = rb_execute_main_phase_action(&game.state, RB_ACT_USE_ABILITY,
                                              sayaka, -1, -1, -1);
        int en_after = game.state.p[0].energy_active;
        int charged = en_before - en_after;
        if (charged_samples == 0 || res > 0) {
            charged_samples++;
            if (charged != 2) all_charged_two = 0;
        }
        printf("step %d: offers=%d ok=%d en %d->%d (charged %d) stage %d,%d,%d -> %d,%d,%d\n",
               step, offers, res, en_before, en_after, charged,
               stage_before[0], stage_before[1], stage_before[2],
               game.state.p[0].stage[0], game.state.p[0].stage[1],
               game.state.p[0].stage[2]);
        print_stage(&game);

        /* resolve any pending choices with the first option */
        drain_first_option(&game);
        /* Rust: game_setup::settle_single_player_state -- no C equivalent entry
           point; draining the ability queue + recalculating constants is the
           closest real engine settle.  LEFT AS A DOCUMENTED LIMITATION: unlike
           the offer count above, this entry point really is absent. */
        rb_drain_ability_queue(&game.state);
        test_recalc(&game);
        /* The offer list is read at the top of the next step; re-entering Main
         * keeps the generator's phase precondition satisfied across the
         * settle, which does not itself advance the phase. */
        game.state.phase = RB_PHASE_MAIN;
    }

    CHECK(activations > 0,
          "sayaka's {{E}}{{E}} activation is OFFERED at least once with 9 energy "
          "(RUST: count_sayaka_offers > 0, "
          "activation_offer_bounded_and_terminates_test.rs:44-49)");
    if (charged_samples > 0) {
        CHECK(all_charged_two,
              "every accepted activation charged exactly the {{E}}{{E}} = 2 cost");
    } else {
        printf("note: no accepted activation, so the 2-energy charge was never exercised\n");
    }
    /* RUST'S ACTUAL ASSERTION, verbatim:
     * activation_offer_bounded_and_terminates_test.rs:75-79,
     * `assert!(activations <= 5, ...)`.  `activations` counts OFFERS here, as it
     * does there; the retired press proxy counted something else and overshot. */
    printf("info: offered activation count = %d (Rust asserts <= 5)\n", activations);
    /* NOT written as a bare `CHECK(activations <= 5, ...)`.  With the real
     * generator a zero offer count satisfies that bound VACUOUSLY -- 0 <= 5 --
     * and a vacuously green bound is exactly how this file's premise went
     * stale in the first place.  The bound is therefore only evaluated once
     * its premise holds, and a missing premise is a FAILURE rather than a
     * silent pass.  The retired press proxy could not go vacuous this way,
     * which is the only thing it had going for it. */
    if (activations > 0) {
        CHECK(activations <= 5,
              "RUST (:75-79): activations <= 5 -- bounded by affordable energy "
              "(9/2 = 4); the count on the preceding info line is what is bounded");
    } else {
        CHECK(0,
              "RUST (:75-79) bound is UNEVALUABLE: rb_generate_action_candidates "
              "emitted no RB_SETUP_ACT_USE_ABILITY row for PL!HS-bp1-002-R at all, "
              "with the card on p1's centre and 9 active energy, so 'activations "
              "<= 5' would hold at 0 for free. Rust's count_sayaka_offers "
              "(game_setup.rs:2078 activation scan) is non-zero here. This is a C "
              "GENERATOR gap, not a bound that was met.");
    }
    {
        RbGeneratedActionList dbg = rb_generate_action_candidates(&game.state);
        int use_ability_rows = 0;
        for (int i = 0; i < dbg.count; i++)
            if (dbg.actions[i].action_type == RB_SETUP_ACT_USE_ABILITY) use_ability_rows++;
        printf("info: generated rows=%d of which USE_ABILITY=%d "
               "(phase=%d active=%d energy_active=%d)\n",
               dbg.count, use_ability_rows, game.state.phase, game.state.active,
               game.state.p[0].energy_active);
    }
    printf("info: total OFFERED activations = %d\n", activations);
}

/// Rust: `sayaka_unaffordable_not_offered_and_press_rejected`.
///
/// Rust, activation_offer_bounded_and_terminates_test.rs:94-120, has THREE
/// assertions and the C previously had only the last two:
///   :111  `assert_eq!(offers, 0, "cost 2 with 1 active energy must not be offered")`
///   :115  the press is rejected with an error mentioning cost/energy
///   :120  `assert!(!game.has_pending_choice())`
/// The missing first one was written off with "no C equivalent (no
/// generate_possible_actions)".  `rb_generate_action_candidates` EXISTS
/// (include/rabuka.h:2061) and `count_ability_offers` above reads it, so the
/// `offers == 0` half is asserted here from the real generator, exactly as
/// Rust does.  The press half is kept unchanged: it is Rust's :115 and it is
/// still the direct check of the affordability pre-check.
static void sayaka_unaffordable_not_offered_and_press_rejected(void)
{
static    TestGame game;
    test_game_new(&game);

    int sayaka = test_id(&game, "PL!HS-bp1-002-R");
    if (sayaka < 0) {
        fprintf(stderr, "FAIL: card id 'PL!HS-bp1-002-R' not in database\n");
        failures++;
        return;
    }
    int cid = test_id(&game, "PL!HS-bp1-005-R");
    if (cid < 0) {
        fprintf(stderr, "FAIL: card id 'PL!HS-bp1-005-R' not in database\n");
        failures++;
        return;
    }

    test_add_to_stage(&game, 1, sayaka);
    test_give_energy(&game, 1); /* cost is {{E}}{{E}} = 2 */
    test_add_to_discard(&game, cid);

    /* Main is the generator's only productive phase (src/turn/phase.c:2156-2160). */
    game.state.phase = RB_PHASE_MAIN;
    game.state.active = 0;

    /* RUST :110-111 -- generation WITHHOLDS the unpayable activation.  This is
     * the half the old revision dropped as "no C equivalent"; it is read from
     * the real generator now. */
    int offers = count_ability_offers(&game.state, sayaka);
    CHECK_EQ(offers, 0,
             "RUST (:111): cost 2 with 1 active energy must NOT be offered -- "
             "game_setup::generate_possible_actions withholds it; C: "
             "rb_generate_action_candidates");

    int en_before = game.state.p[0].energy_active;
    int res = test_activate_ability(&game, sayaka);
    int en_after = game.state.p[0].energy_active;

    CHECK_EQ(res, 0,
             "RUST (:113-119): a direct press of an unpayable {{E}}{{E}} "
             "activation is rejected by the affordability pre-check");
    CHECK_EQ(en_after, en_before,
             "a refused activation charges no energy (no partial resolution)");
    CHECK(!test_has_pending_choice(&game),
          "nothing may be left pending after a refused press");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    sayaka_activation_charges_energy_and_terminates();
    sayaka_unaffordable_not_offered_and_press_rejected();

    rb_unload();
    if (failures) return 1;
    printf("ALL TRIGGER PATHS ACTIVATION OFFER CHECKS PASSED\n");
    return 0;
}
