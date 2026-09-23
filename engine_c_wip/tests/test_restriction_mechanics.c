#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

static int failures;

#define CHECK(c, msg) do { if (!(c)) { fprintf(stderr, "FAIL: %s\n", msg); failures++; } else printf("ok: %s\n", msg); } while (0)
#define CHECK_EQ(a, b, msg) do { int actual_ = (a); int expected_ = (b); if (actual_ != expected_) { fprintf(stderr, "FAIL: %s (got %d expected %d)\n", msg, actual_, expected_); failures++; } else printf("ok: %s\n", msg); } while (0)

static void clear_choices(TestGame *tg)
{
    for (int guard = 0; guard < 100 && test_has_pending_choice(tg); guard++) {
        rb_resume_with_choice(&tg->state, -1);
        rb_drain_ability_queue(&tg->state);
    }
}

static int trigger_auto(TestGame *tg, int cid, const char *trigger_str)
{
    int count = rb_card_num_abilities((uint32_t)cid);
    for (int ability_idx = 0; ability_idx < count; ability_idx++) {
        Ability ability;
        if (!rb_decode_card_ability((uint32_t)cid, ability_idx, &ability)) continue;
        int matches = ability.triggers && strstr(ability.triggers, trigger_str);
        rb_free_ability(&ability);
        if (matches) {
            tg->state.activating_card = cid;
            return rb_queue_push(&tg->state.queue, cid, ability_idx);
        }
    }
    return 0;
}

static void test_ll_bp2001_cannot_be_baton_touched_out(void)
{
    TestGame tg;
    test_game_new(&tg);
    int protected_card = test_id(&tg, "LL-bp2-001-R＋");
    int attacker = test_id(&tg, "PL!-pb1-021-PR");

    test_add_to_stage(&tg, 1, protected_card);
    rb_mods_set_orientation(&tg.state.mods, protected_card, "wait");
    test_add_to_hand(&tg, attacker);
    test_give_energy(&tg, 6);

    int result = test_try_play_to_stage(&tg, attacker, 1);
    CHECK(result == 0, "cannot_baton_touch protection must block the play");
    CHECK(tg.state.p[0].discard.n == 0, "protected member was not sent to the waitroom");
}

static void test_himena_bp6006_baton_only_with_murasakipark(void)
{
    TestGame tg;
    test_game_new(&tg);
    int himena = test_id(&tg, "PL!HS-bp6-006-R＋");
    int outsider = test_id(&tg, "PL!-pb1-021-PR");

    test_add_to_stage(&tg, 0, himena);
    rb_mods_set_orientation(&tg.state.mods, himena, "wait");
    test_add_to_hand(&tg, outsider);
    test_give_energy(&tg, 6);

    int result = test_try_play_to_stage(&tg, outsider, 0);
    CHECK(result == 0, "non-みらくらぱーく！ baton partner must be blocked");
}

static void test_niko_pb1009_suppresses_effect_activations_for_the_turn(void)
{
    TestGame tg;
    test_game_new(&tg);
    int nico = test_id(&tg, "PL!-pb1-009-R");
    int mass_activator = test_id(&tg, "PL!-bp3-005-R");
    int victim = test_id(&tg, "PL!-sd1-010-SD");

    tg.state.p[0].stage[0] = victim;
    rb_mods_set_orientation(&tg.state.mods, victim, "wait");
    tg.state.p[0].stage[1] = nico;
    tg.state.p[0].stage[2] = mass_activator;

    int ability_count = rb_card_num_abilities((uint32_t)nico);
    for (int ability_idx = 0; ability_idx < ability_count; ability_idx++) {
        Ability ability;
        if (!rb_decode_card_ability((uint32_t)nico, ability_idx, &ability)) continue;
        int is_debut = ability.triggers && !strcmp(ability.triggers, "登場");
        rb_free_ability(&ability);
        if (is_debut) {
            tg.state.activating_card = nico;
            rb_queue_push(&tg.state.queue, nico, ability_idx);
            rb_process_pending_auto_abilities(&tg.state);
            clear_choices(&tg);
        }
    }

    trigger_auto(&tg, mass_activator, "登場");
    rb_process_pending_auto_abilities(&tg.state);
    clear_choices(&tg);

    const char *orientation = rb_mods_get_orientation(&tg.state.mods, victim);
    CHECK(orientation && !strcmp(orientation, "wait"),
          "効果によってはアクティブにならない — effect activation must be blocked");
}

static void test_ruby_bpb7009_front_blade_loss_respects_mirror_and_cost(void)
{
    TestGame tg;
    test_game_new(&tg);
    int ruby = test_id(&tg, "PL!S-bp7-009-R");
    int cheap = test_id(&tg, "PL!-sd1-010-SD");
    int pricey = test_id(&tg, "PL!S-bp5-009-R");

    tg.state.p[0].stage[2] = ruby;
    tg.state.p[1].stage[0] = cheap;
    tg.state.p[1].stage[1] = -1;
    tg.state.p[1].stage[2] = pricey;

    test_recalc(&tg);

    CHECK_EQ(test_get_blade_modifier(&tg, cheap), -1,
             "front opponent with cost ≤4 loses 1 blade");
    CHECK_EQ(test_get_blade_modifier(&tg, ruby), 0,
             "Ruby does not lose a blade");

    tg.state.p[0].stage[2] = -1;
    tg.state.p[0].stage[0] = ruby;
    test_recalc(&tg);
    CHECK_EQ(test_get_blade_modifier(&tg, pricey), 0,
             "cost-15 opponent exceeds コスト4以下 → no loss");
    CHECK_EQ(test_get_blade_modifier(&tg, cheap), 0,
             "no longer in front → previous loss removed");
}

static void test_mijuku_dreamer_refresh_condition_scores(void)
{
    TestGame tg;
    test_game_new(&tg);
    int live = test_id(&tg, "PL!S-bp2-022-L");

    test_add_to_live(&tg, live);

    trigger_auto(&tg, live, "ライブ成功時");
    rb_process_pending_auto_abilities(&tg.state);
    clear_choices(&tg);
    CHECK_EQ(test_get_score_modifier(&tg, live), 0,
             "deck did not refresh → no +2");

    tg.state.p[0].deck_refreshed_this_turn = 1;
    trigger_auto(&tg, live, "ライブ成功時");
    rb_process_pending_auto_abilities(&tg.state);
    clear_choices(&tg);
    CHECK_EQ(test_get_score_modifier(&tg, live), 2,
             "リフレッシュしていた場合 → スコア+2");
}

static void test_himena_bp6006_murasakipark_partner_baton_succeeds(void)
{
    TestGame tg;
    test_game_new(&tg);
    int himena = test_id(&tg, "PL!HS-bp6-006-R＋");
    int partner = test_id(&tg, "PL!HS-PR-018-PR");

    test_add_to_stage(&tg, 0, himena);
    rb_mods_set_orientation(&tg.state.mods, himena, "wait");
    test_add_to_hand(&tg, partner);
    test_give_energy(&tg, 0);

    int result = test_try_play_to_stage(&tg, partner, 0);
    CHECK(result != 0, "allowed partner group must pass the restriction");
    CHECK(!test_zone_has_id(&tg, 0, "stage", himena),
          "protected member replaced by allowed partner");
    CHECK(test_zone_has_id(&tg, 0, "discard", himena),
          "baton-touched member goes to the waitroom");
    CHECK(test_zone_has_id(&tg, 0, "stage", partner),
          "partner now occupies the area");
}

static void test_sumire_bpb4004_double_baton_removes_both_and_clamps_cost(void)
{
    TestGame tg;
    test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp4-004-R＋");
    int big_a = test_id(&tg, "PL!S-bp5-009-R");
    int big_b = test_id(&tg, "PL!N-bp1-006-R＋");

    tg.state.p[0].stage[0] = big_a;
    tg.state.p[0].stage[1] = big_b;
    tg.state.p[0].stage[2] = -1;
    rb_mods_set_orientation(&tg.state.mods, big_a, "wait");
    rb_mods_set_orientation(&tg.state.mods, big_b, "wait");
    test_add_to_hand(&tg, sumire);
    test_give_energy(&tg, 3);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    for (int i = 0; i < 40; i++) test_add_to_deck(&tg, filler);

    int result = test_try_play_to_stage(&tg, sumire, 1);
    CHECK(result != 0,
          "double-baton play must be offered when two eligible occupants exist");
    for (int guard = 0; guard < 100 && test_has_pending_choice(&tg); guard++) {
        const char *choice_type = test_pending_choice_type(&tg);
        fprintf(stderr, "[SUMIRE_CHOICE] %s\n", choice_type);
        if (!strcmp(choice_type, "SelectTarget")) {
            int option = 1;
            test_select_indices(&tg, &option, 1);
        } else if (!strcmp(choice_type, "SelectCard")) {
            int index = 0;
            test_select_indices(&tg, &index, 1);
        } else break;
    }

    CHECK(!test_zone_has_id(&tg, 0, "stage", big_a) &&
              !test_zone_has_id(&tg, 0, "stage", big_b),
          "both occupants removed");
    CHECK(test_zone_has_id(&tg, 0, "discard", big_a) &&
              test_zone_has_id(&tg, 0, "discard", big_b),
          "both batoned members in the waitroom");
    CHECK_EQ(rb_energy_active_count(&tg.state.p[0]), 3,
             "combined cost 28 ≥ 22 → pair cost clamps to 0 (Q26: no refunds)");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    test_ll_bp2001_cannot_be_baton_touched_out();
    test_himena_bp6006_baton_only_with_murasakipark();
    test_niko_pb1009_suppresses_effect_activations_for_the_turn();
    test_ruby_bpb7009_front_blade_loss_respects_mirror_and_cost();
    test_mijuku_dreamer_refresh_condition_scores();
    test_himena_bp6006_murasakipark_partner_baton_succeeds();
    test_sumire_bpb4004_double_baton_removes_both_and_clamps_cost();

    rb_unload();
    if (failures) return 1;
    printf("ALL RESTRICTION MECHANICS CHECKS PASSED\n");
    return 0;
}
