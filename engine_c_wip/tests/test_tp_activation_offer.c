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

/// Rust: `sayaka_activation_charges_energy_and_terminates`.
///
/// Rust asks `game_setup::generate_possible_actions` how many UseAbility offers
/// exist for the card; the C engine has NO action generator, so the offer is
/// measured by pressing the card and letting the engine's own cost/affordability
/// path answer. A press the engine refuses stands in for "no longer offered",
/// and a press it accepts is one activation that must have been charged.
static void sayaka_activation_charges_energy_and_terminates(void)
{
    TestGame game;
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
    for (int step = 0; step < 20; step++) {
        int en_before = game.state.p[0].energy_active;
        int stage_before[3];
        for (int i = 0; i < 3; i++) stage_before[i] = game.state.p[0].stage[i];

        int res = rb_execute_main_phase_action(&game.state, RB_ACT_USE_ABILITY,
                                              sayaka, -1, -1, -1);
        if (res <= 0) {
            printf("step %d: no longer offered (en=%d)\n", step, en_before);
            break;
        }
        activations++;
        int en_after = game.state.p[0].energy_active;
        int charged = en_before - en_after;
        charged_samples++;
        if (charged != 2) all_charged_two = 0;
        printf("step %d: ok=%d en %d->%d (charged %d) stage %d,%d,%d -> %d,%d,%d\n",
               step, res, en_before, en_after, charged,
               stage_before[0], stage_before[1], stage_before[2],
               game.state.p[0].stage[0], game.state.p[0].stage[1],
               game.state.p[0].stage[2]);
        print_stage(&game);

        /* resolve any pending choices with the first option */
        drain_first_option(&game);
        /* Rust: game_setup::settle_single_player_state — no C equivalent entry
           point; draining the ability queue + recalculating constants is the
           closest real engine settle. */
        rb_drain_ability_queue(&game.state);
        test_recalc(&game);
    }

    CHECK(activations > 0,
          "sayaka's {{E}}{{E}} activation is accepted at least once with 9 energy");
    if (charged_samples > 0) {
        CHECK(all_charged_two,
              "every accepted activation charged exactly the {{E}}{{E}} = 2 cost");
    } else {
        printf("note: no accepted activation, so the 2-energy charge was never exercised\n");
    }
    CHECK(activations <= 5,
          "activation run is bounded by energy (9/2 ~ 4)");
    printf("info: total accepted activations = %d\n", activations);
}

/// Rust: `sayaka_unaffordable_not_offered_and_press_rejected`.
///
/// The `offers == 0` half of the Rust assertion has no C equivalent (no
/// generate_possible_actions). The directly checkable half is the affordability
/// pre-check: pressing an unpayable {{E}}{{E}} activation must be refused
/// cleanly — no partial resolution, no energy spent, nothing left pending.
static void sayaka_unaffordable_not_offered_and_press_rejected(void)
{
    TestGame game;
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

    int en_before = game.state.p[0].energy_active;
    int res = test_activate_ability(&game, sayaka);
    int en_after = game.state.p[0].energy_active;

    CHECK_EQ(res, 0,
             "unpayable {{E}}{{E}} activation is refused by the affordability pre-check");
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
