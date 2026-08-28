#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
static int failures=0;
#define CHECK(c,msg) do{ if(!(c)){ fprintf(stderr,"FAIL %s:%d: %s\n",__FILE__,__LINE__,msg); failures++; } else printf("ok: %s\n",msg);} while(0)
#define CHECK_EQ(a,b,msg) do{ if((a)!=(b)){ fprintf(stderr,"FAIL %s:%d: %s (got %d expected %d)\n",__FILE__,__LINE__,msg,(int)(a),(int)(b)); failures++; } else printf("ok: %s\n",msg);} while(0)

/* generated — mass-port of simple constant tests (recalculate_constants) */
// test_modules/jidou/complex/bp7_005_ren_jidou_test.rs::ren_005_turn2_blocks_third_energy_placed
static void gen_ren_005_turn2_blocks_third_energy_placed(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int ren = test_id(&tg, "PL!SP-bp7-005-R＋");
    tg.state.p[0].stage[0] = ren;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    // TODO: game.state.player1.energy_zone.cards.push(test_id(&tg, "PL!-sd1-010-SD"));
    // TODO: game.state.player1.energy_zone.set_active_count(1);
    // // First energy placed - use energy_zone
    // TODO: game.state.push_movement_event(-1, "energy_deck", "energy_zone", Some(ren), "p1", true);
    // TODO: let pid = game.state.player1.id.clone();
    // TODO: rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    // TODO: game.state.process_pending_auto_abilities(&pid);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, ren), 1, "ren_005_turn2_blocks_third_energy_placed");
    // // Second same turn (turn2 allows 2)
    // TODO: game.state.push_movement_event(-1, "energy_deck", "energy_zone", Some(ren), "p1", true);
    // TODO: rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    // TODO: game.state.process_pending_auto_abilities(&pid);
    int second = rb_mods_get_blade(&tg.state.mods, ren);
    // TODO assert: assert!(second == 1 || second == 2, "second placement should be 1 or 2, got {}", second);
    // // Third same turn should be blocked (ターン2回)
    // TODO: game.state.push_movement_event(-1, "energy_deck", "energy_zone", Some(ren), "p1", true);
    // TODO: rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    // TODO: game.state.process_pending_auto_abilities(&pid);
    int third = rb_mods_get_blade(&tg.state.mods, ren);
    // TODO assert: assert!(third <= 2, "ターン2回 should block third, got {}", third);
    // 
}

// test_modules/edge_cases/sd2_002_edge_test.rs::sd2_002_natural_move_no_heart
static void gen_sd2_002_natural_move_no_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int keke = test_id(&tg, "PL!SP-sd2-002-SD2");
    tg.state.p[0].stage[1] = keke;
    // // Natural move (effect_only false) should NOT trigger the jidou
    // TODO: game.state.push_movement_event(keke, "stage", "stage", None, "p1", false);
    // TODO: let pid = game.state.player1.id.clone();
    // TODO: rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    // TODO: game.state.process_pending_auto_abilities(&pid);
    int heart_mod = rb_mods_get_heart(&tg.state.mods, keke, 6);
    CHECK_EQ(heart_mod, 0, "sd2_002_natural_move_no_heart");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test10.rs::sp_bp2_004_center_highest_with_only_two_members
static void gen_sp_bp2_004_center_highest_with_only_two_members(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int center_high = test_id(&tg, "PL!SP-pb1-001-R");
    tg.state.p[0].stage[0] = sumire;
    tg.state.p[0].stage[1] = center_high;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_only_two_members");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test10.rs::sp_bp2_004_center_low_with_two_members_no_heart
static void gen_sp_bp2_004_center_low_with_two_members_no_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int left_high = test_id(&tg, "PL!SP-pb1-001-R");
    int center_low = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[0] = left_high;
    tg.state.p[0].stage[1] = center_low;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[0].stage[0] = sumire;
    // // Actually left is sumire with 9, center is 4, so center not highest
    tg.state.p[0].stage[0] = sumire;
    tg.state.p[0].stage[1] = center_low;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_center_low_with_two_members_no_heart");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test11.rs::sp_bp2_004_center_highest_with_only_sumire_at_center
static void gen_sp_bp2_004_center_highest_with_only_sumire_at_center(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    // // Only sumire at center with cost 9, no other members -> center is highest (only member)
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = sumire;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_only_sumire_at_center");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test11.rs::sp_bp2_004_no_gain_when_center_is_lowest
static void gen_sp_bp2_004_no_gain_when_center_is_lowest(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int left_high = test_id(&tg, "PL!SP-pb1-001-R");
    int center_low = test_id(&tg, "PL!-sd1-010-SD");
    int right_mid = test_id(&tg, "PL!HS-PR-001-PR");
    tg.state.p[0].stage[0] = left_high;
    tg.state.p[0].stage[1] = center_low;
    tg.state.p[0].stage[2] = right_mid;
    tg.state.p[0].stage[0] = sumire;
    // // Actually left is sumire with 9, center is 4, right is 10 -> center is lowest, not highest
    tg.state.p[0].stage[0] = sumire;
    tg.state.p[0].stage[1] = center_low;
    tg.state.p[0].stage[2] = right_mid;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_no_gain_when_center_is_lowest");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test2.rs::sp_bp2_004_p_variant_tie_no_heart
static void gen_sp_bp2_004_p_variant_tie_no_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire_p = test_id(&tg, "PL!SP-bp2-004-P");
    int c9a = test_id(&tg, "PL!SP-bp2-004-P");
    int c9b = test_id(&tg, "PL!-PR-005-PR");
    tg.state.p[0].stage[0] = c9a;
    tg.state.p[0].stage[1] = c9b;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire_p, 3), 0, "sp_bp2_004_p_variant_tie_no_heart");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test2.rs::sp_bp2_004_center_empty_no_heart
static void gen_sp_bp2_004_center_empty_no_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    tg.state.p[0].stage[0] = sumire;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    // // Only sumire at left, center empty -> center not highest (no card), so no heart
    // // The condition checks center's cost vs others; with center empty, it should be false
    int h = rb_mods_get_heart(&tg.state.mods, sumire, 3);
    CHECK_EQ(h, 0, "sp_bp2_004_center_empty_no_heart");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test3.rs::sp_bp2_004_center_highest_with_two_empty_sides
static void gen_sp_bp2_004_center_highest_with_two_empty_sides(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int center_high = test_id(&tg, "PL!SP-pb1-001-R");
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = center_high;
    tg.state.p[0].stage[2] = -1;
    // // sumire is at left? Actually we need sumire on stage to check its heart, but sumire is at left with cost 9, center is 11, so center is highest, sumire should gain
    tg.state.p[0].stage[0] = sumire;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_two_empty_sides");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test3.rs::sp_bp2_004_no_center_no_heart
static void gen_sp_bp2_004_no_center_no_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    tg.state.p[0].stage[0] = sumire;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_no_center_no_heart");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test4.rs::sp_bp2_004_all_three_same_cost_no_heart
static void gen_sp_bp2_004_all_three_same_cost_no_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int c9a = test_id(&tg, "PL!SP-bp2-004-R");
    int c9b = test_id(&tg, "PL!-PR-005-PR");
    int c9c = test_id(&tg, "PL!-PR-005-PR");
    tg.state.p[0].stage[0] = c9a;
    tg.state.p[0].stage[1] = c9b;
    tg.state.p[0].stage[2] = c9c;
    // // Put sumire at left with same cost as center and right
    tg.state.p[0].stage[0] = sumire;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_all_three_same_cost_no_heart");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test4.rs::sp_bp2_004_center_highest_with_low_left_right
static void gen_sp_bp2_004_center_highest_with_low_left_right(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int center_high = test_id(&tg, "PL!SP-pb1-001-R");
    int left_low = test_id(&tg, "PL!-sd1-010-SD");
    int right_low = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[0] = left_low;
    tg.state.p[0].stage[1] = center_high;
    tg.state.p[0].stage[2] = right_low;
    // // Need sumire on stage to check its heart, but sumire is not at center, it's at left with low cost
    // // Actually sumire is at left with low cost, center is high, so sumire should gain
    tg.state.p[0].stage[0] = sumire;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_low_left_right");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test5.rs::sp_bp2_004_center_highest_with_high_left_low_right
static void gen_sp_bp2_004_center_highest_with_high_left_low_right(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int left_high = test_id(&tg, "PL!SP-pb1-001-R");
    int center_mid = test_id(&tg, "PL!HS-PR-001-PR");
    int right_low = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[0] = left_high;
    tg.state.p[0].stage[1] = center_mid;
    tg.state.p[0].stage[2] = right_low;
    test_recalc(&tg);
    // // Sumire is at left with 11, center is 10, so center is NOT highest
    // // But sumire's heart is for sumire card itself, which is at left with 11, center is 10, so center (10) < left (11) -> no heart
    // // Actually sumire is at left with 11, center is 10, right is 4, so center is not highest
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_center_highest_with_high_left_low_right");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test5.rs::sp_bp2_004_center_lowest_no_heart
static void gen_sp_bp2_004_center_lowest_no_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int left_high = test_id(&tg, "PL!SP-pb1-001-R");
    int center_low = test_id(&tg, "PL!-sd1-010-SD");
    int right_mid = test_id(&tg, "PL!HS-PR-001-PR");
    tg.state.p[0].stage[0] = left_high;
    tg.state.p[0].stage[1] = center_low;
    tg.state.p[0].stage[2] = right_mid;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_center_lowest_no_heart");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test6.rs::sp_bp2_004_center_highest_with_only_center
static void gen_sp_bp2_004_center_highest_with_only_center(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int center = test_id(&tg, "PL!SP-pb1-001-R");
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = center;
    tg.state.p[0].stage[2] = -1;
    // // sumire is not on stage, but we check its heart modifier - should be 0 because sumire not on stage
    // // Actually we need sumire on stage to check its own heart
    tg.state.p[0].stage[0] = sumire;
    test_recalc(&tg);
    // // Center is 11, left sumire is 9, so center is highest -> sumire gains
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_only_center");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test6.rs::sp_bp2_004_no_stage_no_heart
static void gen_sp_bp2_004_no_stage_no_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_no_stage_no_heart");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test7.rs::sp_bp2_004_p_variant_center_highest
static void gen_sp_bp2_004_p_variant_center_highest(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire_p = test_id(&tg, "PL!SP-bp2-004-P");
    int center_high = test_id(&tg, "PL!SP-pb1-001-R");
    tg.state.p[0].stage[0] = sumire_p;
    tg.state.p[0].stage[1] = center_high;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire_p, 3), 1, "sp_bp2_004_p_variant_center_highest");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test7.rs::sp_bp2_004_center_tie_no_heart_p_variant
static void gen_sp_bp2_004_center_tie_no_heart_p_variant(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire_p = test_id(&tg, "PL!SP-bp2-004-P");
    int c9b = test_id(&tg, "PL!-PR-005-PR");
    tg.state.p[0].stage[0] = sumire_p;
    tg.state.p[0].stage[1] = c9b;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    // // Both cost 9? sumire_p cost 9, center 9 -> tie -> no heart
    // // Need to check sumire_p cost: it is also 9
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire_p, 3), 0, "sp_bp2_004_center_tie_no_heart_p_variant");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test8.rs::sp_bp2_004_center_highest_with_high_right_low_left
static void gen_sp_bp2_004_center_highest_with_high_right_low_left(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int left_low = test_id(&tg, "PL!-sd1-010-SD");
    int center_high = test_id(&tg, "PL!SP-pb1-001-R");
    int right_low = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[0] = left_low;
    tg.state.p[0].stage[1] = center_high;
    tg.state.p[0].stage[2] = right_low;
    tg.state.p[0].stage[0] = sumire;
    // // sumire at 0 with cost 9, center 11, right 4 -> center highest -> sumire gains
    tg.state.p[0].stage[0] = sumire;
    tg.state.p[0].stage[1] = center_high;
    tg.state.p[0].stage[2] = right_low;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_high_right_low_left");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test8.rs::sp_bp2_004_no_gain_when_center_empty_and_others_present
static void gen_sp_bp2_004_no_gain_when_center_empty_and_others_present(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int left = test_id(&tg, "PL!SP-pb1-001-R");
    int right = test_id(&tg, "PL!HS-PR-001-PR");
    tg.state.p[0].stage[0] = left;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = right;
    tg.state.p[0].stage[0] = sumire;
    test_recalc(&tg);
    // // Center empty -> no highest, so no heart
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_no_gain_when_center_empty_and_others_present");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test9.rs::sp_bp2_004_center_highest_with_tie_left_right_no_heart
static void gen_sp_bp2_004_center_highest_with_tie_left_right_no_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int left = test_id(&tg, "PL!SP-pb1-001-R");
    int center = test_id(&tg, "PL!HS-PR-001-PR");
    int right = test_id(&tg, "PL!SP-pb1-001-R");
    tg.state.p[0].stage[0] = left;
    tg.state.p[0].stage[1] = center;
    tg.state.p[0].stage[2] = right;
    tg.state.p[0].stage[0] = sumire;
    // // Let's set left 4, center 11, right 11 -> center tie with right, not highest
    int left_low = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[0] = left_low;
    tg.state.p[0].stage[1] = center;
    tg.state.p[0].stage[2] = right;
    tg.state.p[0].stage[0] = sumire;
    // // Simplify: sumire at left with 9, center 10, right 11 -> center not highest (right is)
    int sumire_id = test_id(&tg, "PL!SP-bp2-004-R");
    int center_mid = test_id(&tg, "PL!HS-PR-001-PR");
    int right_high = test_id(&tg, "PL!SP-pb1-001-R");
    tg.state.p[0].stage[0] = sumire_id;
    tg.state.p[0].stage[1] = center_mid;
    tg.state.p[0].stage[2] = right_high;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire_id, 3), 0, "sp_bp2_004_center_highest_with_tie_left_right_no_heart");
    // 
}

// test_modules/edge_cases/sp_bp2_004_extra_test9.rs::sp_bp2_004_sumire_at_right_center_highest_gains
static void gen_sp_bp2_004_sumire_at_right_center_highest_gains(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int center_high = test_id(&tg, "PL!SP-pb1-001-R");
    int left_low = test_id(&tg, "PL!-sd1-010-SD");
    // // sumire at right with 9, center 11, left 4 -> center is highest, sumire should gain even though sumire is at right
    tg.state.p[0].stage[0] = left_low;
    tg.state.p[0].stage[1] = center_high;
    tg.state.p[0].stage[2] = sumire;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_sumire_at_right_center_highest_gains");
    // 
}

// test_modules/abilities/moderate/bp5_333_erena_edge_test2.rs::erena_p_variant_wait_gains_heart
static void gen_erena_p_variant_wait_gains_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int erena = test_id(&tg, "PL!-bp5-333-P＋");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[0] = erena;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    // clear
    test_add_to_deck(&tg, filler);
    test_add_to_deck(&tg, filler);
    rb_mods_set_orientation(&tg.state.mods, erena, "wait");
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, erena, 5), 1, "erena_p_variant_wait_gains_heart");
    // 
}

// test_modules/abilities/moderate/bp5_333_erena_edge_test2.rs::erena_wait_then_active_loses_heart
static void gen_erena_wait_then_active_loses_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int erena = test_id(&tg, "PL!-bp5-333-R");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].stage[0] = erena;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    test_add_to_deck(&tg, filler);
    test_add_to_deck(&tg, filler);
    rb_mods_set_orientation(&tg.state.mods, erena, "wait");
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, erena, 5), 1, "erena_wait_then_active_loses_heart");
    rb_mods_set_orientation(&tg.state.mods, erena, "active");
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, erena, 5), 0, "erena_wait_then_active_loses_heart");
    // 
}

// test_modules/abilities/moderate/l0_gap_constant_test.rs::sumire_pr_center_position_grants_blade
static void gen_sumire_pr_center_position_grants_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int sumire = test_id(&tg, "PL!SP-bp1-004-PR");
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = sumire;
    tg.state.p[0].stage[2] = -1;
    test_give_energy(&tg, 20);
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, sumire), 5, "sumire_pr_center_position_grants_blade");
    // 
    // // Negative: move out of center → modifier drops.
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[0] = sumire;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, sumire), 0, "sumire_pr_center_position_grants_blade");
    // 
}

// test_modules/abilities/moderate/sp_bp2_004_edge2_test.rs::sp_bp2_004_all_empty_no_heart
static void gen_sp_bp2_004_all_empty_no_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_all_empty_no_heart");
    // 
}

// test_modules/abilities/moderate/sp_bp2_004_edge2_test.rs::sp_bp2_004_center_only_one_member_gains
static void gen_sp_bp2_004_center_only_one_member_gains(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = sumire;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_only_one_member_gains");
    // 
}

// test_modules/abilities/simple/rin_test.rs::rin_q79_vacated_area_can_receive_new_member
static void gen_rin_q79_vacated_area_can_receive_new_member(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int rin = test_id(&tg, "PL!-sd1-005-SD");
    int live_card = test_id(&tg, "PL!-sd1-019-SD");
    // 
    // // Stage: 星空凛 at center
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = rin;
    tg.state.p[0].stage[2] = -1;
    // 
    // // Hand: a live card so waitroom has a target for recovery
    test_add_to_hand(&tg, live_card);
    // // Put the live card in waitroom for recovery
    // TODO: game.add_to_discard(live_card);
    test_give_energy(&tg, 3);
    // 
    // TODO: game.activate_ability(rin);
    // 
    // // Center stage should be empty after self_cost
    CHECK_EQ(tg.state.p[0].stage[1], -1, "rin_q79_vacated_area_can_receive_new_member");
    // 
    // // Waitroom should have Rin (from self_cost) — live card was recovered
    // TODO assert: assert!( game.state.player1.waitroom.cards.contains(&rin), "Rin should be in waitroom" );
    // TODO assert: assert!( game.state.player1.hand.cards.contains(&live_card), "Live card should be recovered to hand" );
    // 
    // // The vacated center area can receive a new card (empty slot exists)
    // 
}

// test_modules/jidou/moderate/sd2_002_jidou_cause_test.rs::keke_jidou_effect_only_false_no_trigger
static void gen_keke_jidou_effect_only_false_no_trigger(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int keke = test_id(&tg, "PL!SP-sd2-002-SD2");
    tg.state.p[0].stage[1] = keke;
    // // Natural move (effect_only false) should NOT trigger
    // TODO: game.state.push_movement_event(keke, "stage", "stage", None, "p1", false);
    // TODO: let pid = game.state.player1.id.clone();
    // TODO: rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    // TODO: game.state.process_pending_auto_abilities(&pid);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, keke, 6), 0, "keke_jidou_effect_only_false_no_trigger");
    // 
}

// test_modules/jidou/moderate/sd2_002_jidou_cause_test.rs::keke_jidou_self_effect_move_triggers
static void gen_keke_jidou_self_effect_move_triggers(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int keke = test_id(&tg, "PL!SP-sd2-002-SD2");
    tg.state.p[0].stage[1] = keke;
    // TODO: game.state.push_movement_event(keke, "stage", "stage", Some(keke), "p1", true);
    // TODO: let pid = game.state.player1.id.clone();
    // TODO: rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    // TODO: game.state.process_pending_auto_abilities(&pid);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, keke, 6), 1, "keke_jidou_self_effect_move_triggers");
    // 
}

// test_modules/batches/untested_abilities_batch18_test.rs::wish_song_five_distinct_liella_revealed_scores
static void gen_wish_song_five_distinct_liella_revealed_scores(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!SP-bp4-026-L");
    test_add_to_live(&tg, live);
    // 
    // TODO: for id in [game.id(KANON), game.id(KEKE), game.id(REN), game.id(SUMIRE), game.id(WIEN)] {
    // TODO: game.state.revealed_cards.push(id);
    // TODO: }
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // 
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 1, "wish_song_five_distinct_liella_revealed_scores");
    // 
}

// test_modules/batches/untested_abilities_batch18_test.rs::wish_song_four_distinct_liella_revealed_no_score
static void gen_wish_song_four_distinct_liella_revealed_no_score(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!SP-bp4-026-L");
    test_add_to_live(&tg, live);
    // 
    // TODO: for id in [game.id(KANON), game.id(KEKE), game.id(REN), game.id(SUMIRE)] {
    // TODO: game.state.revealed_cards.push(id);
    // TODO: }
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // 
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 0, "wish_song_four_distinct_liella_revealed_no_score");
    // 
}

// test_modules/edge_cases/bp4_014_ability_filter_edge_test.rs::rin_only_ls_live_no_blade
static void gen_rin_only_ls_live_no_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int rin = test_id(&tg, "PL!-bp4-014-N");
    int mate = test_id(&tg, "PL!S-sd1-001-SD");
    tg.state.p[0].stage[0] = mate;
    tg.state.p[0].stage[1] = rin;
    tg.state.p[0].stage[2] = -1;
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // TODO: fill_decks(&mut game, filler);
    // // Find a live with only LS (e.g., PL!N-bp1-027-L has LS)
    int only_ls = test_id(&tg, "PL!N-bp1-027-L");
    test_add_to_live(&tg, only_ls);
    // TODO: fire_live_start(&mut game, rin);
    int blade = rb_mods_get_blade(&tg.state.mods, mate);
    CHECK_EQ(blade, 0, "rin_only_ls_live_no_blade");
    // 
}

// test_modules/edge_cases/sp_bp2_004_highest_cost_edge_test.rs::sp_bp2_004_center_tie_with_both_sides_no_heart
static void gen_sp_bp2_004_center_tie_with_both_sides_no_heart(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    // // All three same cost 9 -> center not strictly highest
    int c9a = test_id(&tg, "PL!SP-bp2-004-R");
    int c9b = test_id(&tg, "PL!-PR-005-PR");
    int c9c = test_id(&tg, "PL!-PR-005-PR");
    tg.state.p[0].stage[0] = c9a;
    tg.state.p[0].stage[1] = c9b;
    tg.state.p[0].stage[2] = c9c;
    test_recalc(&tg);
    int before = rb_mods_get_heart(&tg.state.mods, sumire, 3);
    // // Already recalculated, should be 0
    CHECK_EQ(before, 0, "sp_bp2_004_center_tie_with_both_sides_no_heart");
    // 
}

// test_modules/edge_cases/sp_bp2_004_highest_cost_edge_test.rs::sp_bp2_004_center_highest_with_empty_side
static void gen_sp_bp2_004_center_highest_with_empty_side(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-004-R");
    int center_high = test_id(&tg, "PL!SP-pb1-001-R");
    tg.state.p[0].stage[0] = sumire;
    tg.state.p[0].stage[1] = center_high;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    int h = rb_mods_get_heart(&tg.state.mods, sumire, 3);
    CHECK_EQ(h, 1, "sp_bp2_004_center_highest_with_empty_side");
    // 
}

// test_modules/edge_cases/sp_bp2_004_highest_cost_edge_test.rs::sp_bp2_004_p_variant_same
static void gen_sp_bp2_004_p_variant_same(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire_p = test_id(&tg, "PL!SP-bp2-004-P");
    int center_high = test_id(&tg, "PL!SP-pb1-001-R");
    tg.state.p[0].stage[0] = sumire_p;
    tg.state.p[0].stage[1] = center_high;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    int h = rb_mods_get_heart(&tg.state.mods, sumire_p, 3);
    CHECK_EQ(h, 1, "sp_bp2_004_p_variant_same");
    // 
}

// test_modules/abilities/complex/l0_gap_constant4_test.rs::sp_bp5_011_position_hearts
static void gen_sp_bp5_011_position_hearts(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int member = test_id(&tg, "PL!SP-bp5-011-R");
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = member;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    // 
    // // Left → heart02×3
    tg.state.p[0].stage[0] = member;
    tg.state.p[0].stage[1] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, member, 2), 3, "sp_bp5_011_position_hearts");
    // 
    // // Center → heart03×3
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = member;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, member, 3), 3, "sp_bp5_011_position_hearts");
    // 
    // // Right → heart05×3
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = member;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, member, 5), 3, "sp_bp5_011_position_hearts");
    // 
}

// test_modules/abilities/complex/l0_gap_constant4_test.rs::sb7_005_aqours_under_card_blade
static void gen_sb7_005_aqours_under_card_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int member = test_id(&tg, "PL!S-bp7-005-R\u{ff0b}");
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = member;
    tg.state.p[0].stage[2] = -1;
    // // Place an Aqours member card under this member
    int under_card = test_id(&tg, "PL!S-bp2-001-R");
    // TODO: if let Some(idx) = game.state.player1.stage.stage.iter().position(|&x| x == member) {
    // TODO: game.state.player1.stage.under_cards[idx].push(under_card);
    // TODO: }
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, member);
    CHECK_EQ(blade, 1, "sb7_005_aqours_under_card_blade");
    // 
}

// test_modules/abilities/moderate/l0_gap_constant3_test.rs::hs_bp2_006_per_other_mirakuraku_member_blade
static void gen_hs_bp2_006_per_other_mirakuraku_member_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int member = test_id(&tg, "PL!HS-bp2-006-R");
    // // Another みらくらぱーく！ member + a non-Mirakuraku member
    int other_mk = test_id(&tg, "PL!HS-bp1-005-R");
    int not_mk = test_id(&tg, "PL!HS-sd1-005-SD");
    tg.state.p[0].stage[0] = other_mk;
    tg.state.p[0].stage[1] = member;
    tg.state.p[0].stage[2] = not_mk;
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, member);
    CHECK_EQ(blade, 1, "hs_bp2_006_per_other_mirakuraku_member_blade");
    // 
}

// test_modules/abilities/moderate/l0_gap_constant3_test.rs::spb1_005_opponent_more_energy_grants_blade
static void gen_spb1_005_opponent_more_energy_grants_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int member = test_id(&tg, "PL!S-pb1-005-PR");
    tg.state.p[0].stage[0] = member;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    // // P1 has no energy, P2 has plenty
    // TODO: game.state.player2.energy_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "LL-E-001-SD"));
    // TODO: game.state.player2.energy_zone.add_active(3);
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, member);
    CHECK_EQ(blade, 3, "spb1_005_opponent_more_energy_grants_blade");
    // 
    // // Negative: give P1 energy so P2 doesn't have more
    test_give_energy(&tg, 10);
    test_recalc(&tg);
    // 
}

// test_modules/abilities/moderate/modifier_layer_characterization_test.rs::temporary_live_end_effect_expires_when_live_phase_ends
static void gen_temporary_live_end_effect_expires_when_live_phase_ends(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int me = test_id(&tg, "PL!HS-cl1-006-CL");
    tg.state.p[0].stage[0] = me;
    // 
    // TODO: fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, me), 3, "temporary_live_end_effect_expires_when_live_phase_ends");
    // TODO assert: assert!( !game.state.temporary_effects.is_empty(), "the grant must be registered as a tracked temporary effect" );
    // 
    // // Stay inside the live phase -> nothing expires.
    // TODO: game.state.current_turn_phase = TurnPhase::Live;
    // TODO: game.state.check_expired_effects();
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, me), 3, "temporary_live_end_effect_expires_when_live_phase_ends");
    // 
    // // Live phase ends -> the effect expires and reverts exactly +3.
    // TODO: game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    // TODO: game.state.check_expired_effects();
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, me), 0, "temporary_live_end_effect_expires_when_live_phase_ends");
    // TODO assert: assert!(game.state.temporary_effects.is_empty());
    // 
}

// test_modules/abilities/simple/l0_gap_position_blade_test.rs::sd2_004_center_blade_plus4
static void gen_sd2_004_center_blade_plus4(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int member = test_id(&tg, "PL!SP-sd2-004-SD2");
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = member;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, member), 4, "sd2_004_center_blade_plus4");
    // 
    // // Negative: move to left → no bonus
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[0] = member;
    test_recalc(&tg);
    // TODO: assert_ne!(
    // TODO: game.state.mods.get_blade_modifier(member),
    // TODO: 4,
    // TODO: "left position should not grant the center bonus"
    // TODO: );
    // 
}

// test_modules/abilities/simple/l0_gap_position_blade_test.rs::pb2_035_left_blade_plus2
static void gen_pb2_035_left_blade_plus2(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int member = test_id(&tg, "PL!SP-pb2-035-N");
    tg.state.p[0].stage[0] = member;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, member), 2, "pb2_035_left_blade_plus2");
    // 
}

// test_modules/abilities/simple/l0_gap_position_blade_test.rs::pb2_041_right_blade_plus2
static void gen_pb2_041_right_blade_plus2(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int member = test_id(&tg, "PL!SP-pb2-041-N");
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = member;
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, member), 2, "pb2_041_right_blade_plus2");
    // 
}

// test_modules/jidou/moderate/miyashita_ai_test.rs::miyashita_ai_only_two_debuts_no_draw
static void gen_miyashita_ai_only_two_debuts_no_draw(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int ai = test_id(&tg, "PL!N-bp3-005-P");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    test_give_energy(&tg, 20);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, ai);
    // 
    // // Debut #1: filler
    test_play_to_stage(&tg, filler, 1);
    // // Debut #2: Ai (only 2 total debuts)
    test_play_to_stage(&tg, ai, 1);
    // 
    // // With only 2 debuts (< 3), the auto ability condition should fail
    // // Hand was [filler, ai] and both played, so hand = []
    CHECK_EQ(tg.state.p[0].hand.n, 0, "miyashita_ai_only_two_debuts_no_draw");
    // 
}

// test_modules/jidou/moderate/sd2_011_tomari_jidou_test.rs::tomari_jidou_self_move_triggers
static void gen_tomari_jidou_self_move_triggers(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int tomari = test_id(&tg, "PL!SP-sd2-011-SD2");
    tg.state.p[0].stage[1] = tomari;
    // TODO: game.state.push_movement_event(tomari, "stage", "stage", Some(tomari), "p1", true);
    // TODO: let pid = game.state.player1.id.clone();
    // TODO: rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    // TODO: game.state.process_pending_auto_abilities(&pid);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, tomari), 1, "tomari_jidou_self_move_triggers");
    // 
}

// test_modules/jidou/moderate/sd2_011_tomari_jidou_test.rs::tomari_jidou_opponent_move_triggers
static void gen_tomari_jidou_opponent_move_triggers(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int tomari = test_id(&tg, "PL!SP-sd2-011-SD2");
    tg.state.p[0].stage[1] = tomari;
    // TODO: game.state.push_movement_event(tomari, "stage", "stage", Some(tomari), "p2", true);
    // TODO: let pid = game.state.player1.id.clone();
    // TODO: rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    // TODO: game.state.process_pending_auto_abilities(&pid);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, tomari), 1, "tomari_jidou_opponent_move_triggers");
    // 
}

// test_modules/jidou/moderate/sd2_011_tomari_jidou_test.rs::tomari_jidou_natural_move_no_trigger
static void gen_tomari_jidou_natural_move_no_trigger(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int tomari = test_id(&tg, "PL!SP-sd2-011-SD2");
    tg.state.p[0].stage[1] = tomari;
    // TODO: game.state.push_movement_event(tomari, "stage", "stage", None, "p1", false);
    // TODO: let pid = game.state.player1.id.clone();
    // TODO: rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    // TODO: game.state.process_pending_auto_abilities(&pid);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, tomari), 0, "tomari_jidou_natural_move_no_trigger");
    // 
}

// test_modules/batches/untested_abilities_batch11_test.rs::cl1_006_debut_gains_three_blades
static void gen_cl1_006_debut_gains_three_blades(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int me = test_id(&tg, "PL!HS-cl1-006-CL");
    tg.state.p[0].stage[1] = me;
    // 
    // TODO: fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, me), 3, "cl1_006_debut_gains_three_blades");
    // 
}

// test_modules/batches/untested_abilities_batch15_test.rs::sd1026_energy_below_nine_no_score
static void gen_sd1026_energy_below_nine_no_score(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!SP-sd1-026-SD");
    test_add_to_live(&tg, live);
    test_give_energy(&tg, 8);
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    // 
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 0, "sd1026_energy_below_nine_no_score");
    // 
}

// test_modules/batches/untested_abilities_batch15_test.rs::treco_please_aqours_h04_total_ten_scores_two
static void gen_treco_please_aqours_h04_total_ten_scores_two(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!S-pb1-020-L");
    test_add_to_live(&tg, live);
    // 
    // // Two Aqours members with 5 printed heart04 each => total 10.
    int a1 = test_id(&tg, "PL!S-bp5-007-R");
    int a2 = test_id(&tg, "PL!S-bp5-007-R");
    tg.state.p[0].stage[0] = a1;
    tg.state.p[0].stage[1] = a2;
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    // 
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 2, "treco_please_aqours_h04_total_ten_scores_two");
    // 
}

// test_modules/batches/untested_abilities_batch15_test.rs::treco_please_aqours_h04_below_ten_no_score
static void gen_treco_please_aqours_h04_below_ten_no_score(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!S-pb1-020-L");
    test_add_to_live(&tg, live);
    // 
    // // Single member with 5 heart04 -> below threshold.
    int a1 = test_id(&tg, "PL!S-bp5-007-R");
    tg.state.p[0].stage[0] = a1;
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    // 
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 0, "treco_please_aqours_h04_below_ten_no_score");
    // 
}

// test_modules/abilities/moderate/bp7_q267_rinna_mill_refresh_test.rs::q267_deck_exhausts_mid_mill_refreshes_and_completes
static void gen_q267_deck_exhausts_mid_mill_refreshes_and_completes(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    // TODO: setup_and_trigger(&mut game, 3, 7, 30, 0);
    // 
    // // Core Q267: refresh happened DURING the mill.
    // TODO assert: assert!( game.state.player1.deck_refreshed_this_turn, "p1 deck hit 0 mid-mill → must refresh immediately (Q267)" );
    // // The initially-milled 3 cards left the waitroom (shuffled back to deck).
    CHECK_EQ(tg.state.p[0].discard.n, 4, "q267_deck_exhausts_mid_mill_refreshes_and_completes");
    CHECK_EQ(tg.state.p[0].deck.n, 6, "q267_deck_exhausts_mid_mill_refreshes_and_completes");
    CHECK_EQ(tg.state.p[0].discard.n, 10, "q267_deck_exhausts_mid_mill_refreshes_and_completes");
    // 
    // // p2 had a full deck → milled a clean 7, no refresh.
    // TODO assert: assert!( !game.state.player2.deck_refreshed_this_turn, "p2 had 30 cards → must NOT refresh" );
    CHECK_EQ(tg.state.p[1].discard.n, 7, "q267_deck_exhausts_mid_mill_refreshes_and_completes");
    CHECK_EQ(tg.state.p[1].deck.n, 23, "q267_deck_exhausts_mid_mill_refreshes_and_completes");
    // 
}

// test_modules/abilities/moderate/bp7_q267_rinna_mill_refresh_test.rs::q267_deck_exactly_seven_no_refresh
static void gen_q267_deck_exactly_seven_no_refresh(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    // // p1: deck=7 (exact), waitroom=3 (refresh material present but unused).
    // TODO: setup_and_trigger(&mut game, 7, 3, 30, 0);
    // 
    // TODO assert: assert!( !game.state.player1.deck_refreshed_this_turn, "deck exactly 7 → no overdraw → no refresh, despite waitroom material" );
    CHECK_EQ(tg.state.p[0].deck.n, 0, "q267_deck_exactly_seven_no_refresh");
    CHECK_EQ(tg.state.p[0].discard.n, 10, "q267_deck_exactly_seven_no_refresh");
    // 
}

// test_modules/abilities/moderate/bp7_q267_rinna_mill_refresh_test.rs::q267_deck_five_refresh_completes_to_seven
static void gen_q267_deck_five_refresh_completes_to_seven(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    // TODO: setup_and_trigger(&mut game, 5, 3, 30, 0);
    // 
    // TODO assert: assert!( game.state.player1.deck_refreshed_this_turn, "p1 deck (5) ran out mid-mill → refresh" );
    CHECK_EQ(tg.state.p[0].discard.n, 2, "q267_deck_five_refresh_completes_to_seven");
    CHECK_EQ(tg.state.p[0].deck.n, 6, "q267_deck_five_refresh_completes_to_seven");
    CHECK_EQ(tg.state.p[0].discard.n, 8, "q267_deck_five_refresh_completes_to_seven");
    // 
}

// test_modules/abilities/moderate/bp7_q267_rinna_mill_refresh_test.rs::q267_both_players_refresh_their_own_deck
static void gen_q267_both_players_refresh_their_own_deck(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    // TODO: setup_and_trigger(&mut game, 3, 7, 3, 7);
    // 
    // TODO assert: assert!( game.state.player1.deck_refreshed_this_turn, "p1 deck hit 0 → refresh" );
    // TODO assert: assert!( game.state.player2.deck_refreshed_this_turn, "p2 deck hit 0 during its own mill → refresh" );
    CHECK_EQ(tg.state.p[0].discard.n, 4, "q267_both_players_refresh_their_own_deck");
    CHECK_EQ(tg.state.p[1].discard.n, 4, "q267_both_players_refresh_their_own_deck");
    CHECK_EQ(tg.state.p[0].deck.n, 6, "q267_both_players_refresh_their_own_deck");
    CHECK_EQ(tg.state.p[1].deck.n, 6, "q267_both_players_refresh_their_own_deck");
    // 
}

// test_modules/abilities/moderate/wien_cost_mod_test.rs::wien_cost_modifier_dynamic
static void gen_wien_cost_modifier_dynamic(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int wien = test_id(&tg, "PL!SP-pb1-010-R");
    int energy_id = test_id(&tg, "LL-E-001-SD");
    // 
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = wien;
    tg.state.p[0].stage[2] = -1;
    test_give_energy(&tg, 9);
    // 
    // // At 9 energy: no modifier
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods, wien), 0, "wien_cost_modifier_dynamic");
    // 
    // // Add 1 more → 10 energy
    test_add_to_discard(&tg, energy_id);
    // TODO: game.state.player1.energy_zone.add_active(1);
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods, wien), 4, "wien_cost_modifier_dynamic");
    // 
    // // Remove 1 → back to 9
    // pop
    // TODO: game.state.player1.energy_zone.sub_active(1);
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods, wien), 0, "wien_cost_modifier_dynamic");
    // 
}

// test_modules/abilities/moderate/wien_cost_mod_test.rs::wien_cost_modifier_cleared_on_leave
static void gen_wien_cost_modifier_cleared_on_leave(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int wien = test_id(&tg, "PL!SP-pb1-010-R");
    // 
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = wien;
    tg.state.p[0].stage[2] = -1;
    test_give_energy(&tg, 10);
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods, wien), 4, "wien_cost_modifier_cleared_on_leave");
    // 
    // // Remove from stage
    tg.state.p[0].stage[1] = -1;
    test_add_to_discard(&tg, wien);
    test_clear_mods_for_card(&tg, wien);
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods, wien), 0, "wien_cost_modifier_cleared_on_leave");
    // 
}

// test_modules/batches/untested_abilities_batch12_test.rs::bp6022_more_opp_energy_scores
static void gen_bp6022_more_opp_energy_scores(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!S-bp6-022-L");
    test_add_to_live(&tg, live);
    // 
    test_give_energy(&tg, 1);
    // // Opponent gets 3 active energies.
    // TODO: for _ in 0..3 {
    int e2 = test_id(&tg, "LL-E-001-SD");
    test_add_to_discard(&tg, e2);
    // TODO: }
    // TODO: game.state.player2.energy_zone.add_active(3);
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // 
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 1, "bp6022_more_opp_energy_scores");
    // 
}

// test_modules/batches/untested_abilities_batch12_test.rs::bp6022_equal_or_fewer_opp_energy_no_score
static void gen_bp6022_equal_or_fewer_opp_energy_no_score(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!S-bp6-022-L");
    test_add_to_live(&tg, live);
    // 
    test_give_energy(&tg, 2);
    // TODO: for _ in 0..2 {
    int e2 = test_id(&tg, "LL-E-001-SD");
    test_add_to_discard(&tg, e2);
    // TODO: }
    // TODO: game.state.player2.energy_zone.add_active(2);
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    // 
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 0, "bp6022_equal_or_fewer_opp_energy_no_score");
    // 
}

// test_modules/batches/untested_abilities_batch12_test.rs::bp6012_debut_activates_energy_with_sbuuke_teammate
static void gen_bp6012_debut_activates_energy_with_sbuuke_teammate(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int me = test_id(&tg, "PL!HS-bp6-012-R");
    int mate = test_id(&tg, "PL!HS-bp1-012-PR");
    tg.state.p[0].stage[0] = me;
    tg.state.p[0].stage[1] = mate;
    // 
    // // One WAIT energy (card pushed without add_active).
    int energy = test_id(&tg, "LL-E-001-SD");
    test_add_to_discard(&tg, energy);
    CHECK_EQ(tg.state.p[0].energy_active, 0, "bp6012_debut_activates_energy_with_sbuuke_teammate");
    // 
    // TODO: fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");
    // 
    CHECK_EQ(tg.state.p[0].energy_active, 1, "bp6012_debut_activates_energy_with_sbuuke_teammate");
    // 
}

// test_modules/batches/untested_abilities_batch13_test.rs::spr039_constant_blades_with_combined_success_cards
static void gen_spr039_constant_blades_with_combined_success_cards(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int me = test_id(&tg, "PL!S-PR-039-PR");
    tg.state.p[0].stage[0] = me;
    // 
    // TODO: for _ in 0..2 {
    int a = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_live(&tg, a);
    int b = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_live(&tg, b);
    // TODO: }
    // // Combined = 4.
    // 
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, me), 2, "spr039_constant_blades_with_combined_success_cards");
    // 
}

// test_modules/batches/untested_abilities_batch13_test.rs::spr039_constant_blades_off_below_four_combined
static void gen_spr039_constant_blades_off_below_four_combined(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int me = test_id(&tg, "PL!S-PR-039-PR");
    tg.state.p[0].stage[0] = me;
    // 
    int a = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_live(&tg, a);
    int b = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_live(&tg, b);
    // // Combined = 2 < 4.
    // 
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, me), 0, "spr039_constant_blades_off_below_four_combined");
    // 
}

// test_modules/batches/untested_abilities_batch14_test.rs::bp7020_constant_blades_while_energy_ahead
static void gen_bp7020_constant_blades_while_energy_ahead(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int me = test_id(&tg, "PL!SP-bp7-020-N");
    tg.state.p[0].stage[0] = me;
    test_give_energy(&tg, 3);
    // TODO: give_opp_energy(&mut game, 1);
    // 
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, me), 2, "bp7020_constant_blades_while_energy_ahead");
    // 
}

// test_modules/batches/untested_abilities_batch14_test.rs::bp7020_constant_blades_off_when_behind
static void gen_bp7020_constant_blades_off_when_behind(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int me = test_id(&tg, "PL!SP-bp7-020-N");
    tg.state.p[0].stage[0] = me;
    test_give_energy(&tg, 1);
    // TODO: give_opp_energy(&mut game, 3);
    // 
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, me), 0, "bp7020_constant_blades_off_when_behind");
    // 
}

// test_modules/batches/untested_abilities_batch14_test.rs::sd1022_live_start_grants_blade_to_all_aqours_members
static void gen_sd1022_live_start_grants_blade_to_all_aqours_members(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!S-sd1-022-SD");
    test_add_to_live(&tg, live);
    // 
    int a1 = test_id(&tg, "PL!S-sd1-001-SD");
    int a2 = test_id(&tg, "PL!S-sd1-002-SD");
    int non_aqours = test_id(&tg, "PL!HS-bp5-004-R");
    tg.state.p[0].stage[0] = a1;
    tg.state.p[0].stage[1] = a2;
    tg.state.p[0].stage[2] = non_aqours;
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    // 
    // TODO assert: assert!( game.state.mods.get_blade_modifier(a1) >= 1, "Aqours member 1 gains a blade" );
    // TODO assert: assert!( game.state.mods.get_blade_modifier(a2) >= 1, "Aqours member 2 gains a blade" );
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, non_aqours), 0, "sd1022_live_start_grants_blade_to_all_aqours_members");
    // 
}

// test_modules/edge_cases/sp_bp2_015_021_comprehensive_edge_test.rs::yell_all_blade_blocks_sumire_and_wien
static void gen_yell_all_blade_blocks_sumire_and_wien(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-015-N");
    int wien = test_id(&tg, "PL!SP-bp2-021-N");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // // Use a card that has ALL blade heart. PL!HS-bp1-019? Let's pick PL!SP-bp1-025-L which is Starlight Prologue with ALL? Check has_blade_heart includes ALL.
    // // The test helper earlier used PL!-pb1-014-R as blade example. For ALL, we can use PL!HS-PR-010-PR which is Reflection with ALL per earlier.
    int all_blade_card = test_id(&tg, "PL!HS-PR-010-PR");
    // // Ensure it indeed has blade heart (has_blade_heart includes ALL)
    // TODO assert: assert!(game.db.get_card(all_blade_card).unwrap().has_blade_heart(), "chosen card must have blade heart (ALL)");
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = sumire;
    tg.state.p[0].stage[2] = wien;
    // TODO: trigger_yell(&mut game, sumire, wien, &[all_blade_card]);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 6), 0, "yell_all_blade_blocks_sumire_and_wien");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, wien, 3), 0, "yell_all_blade_blocks_sumire_and_wien");
    // 
}

// test_modules/edge_cases/sp_bp2_015_021_comprehensive_edge_test.rs::yell_only_one_present_triggers_self_only
static void gen_yell_only_one_present_triggers_self_only(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-015-N");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int m_no_blade = test_id(&tg, "PL!S-bp2-002-R");
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = sumire;
    tg.state.p[0].stage[2] = -1;
    // TODO: trigger_yell(&mut game, sumire, filler, &[m_no_blade]);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 6), 1, "yell_only_one_present_triggers_self_only");
    // // Ensure Wien absent doesn't somehow get heart on filler
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, filler, 3), 0, "yell_only_one_present_triggers_self_only");
    // 
}

// test_modules/batches/untested_abilities_batch10_test.rs::go_master_start_fewer_success_cards_scores
static void gen_go_master_start_fewer_success_cards_scores(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!SP-bp2-023-L");
    test_add_to_live(&tg, live);
    // 
    int s1 = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_live(&tg, s1);
    int o1 = test_id(&tg, "PL!-sd1-010-SD");
    int o2 = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_live(&tg, o1);
    test_add_to_live(&tg, o2);
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    // 
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 1, "go_master_start_fewer_success_cards_scores");
    // 
}

// test_modules/batches/untested_abilities_batch10_test.rs::go_master_start_equal_success_cards_no_score
static void gen_go_master_start_equal_success_cards_no_score(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!SP-bp2-023-L");
    test_add_to_live(&tg, live);
    // 
    int s1 = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_live(&tg, s1);
    int o1 = test_id(&tg, "PL!-sd1-010-SD");
    test_add_to_live(&tg, o1);
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    // 
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 0, "go_master_start_equal_success_cards_no_score");
    // 
}

// test_modules/batches/untested_abilities_batch10_test.rs::note_mermaid_two_distinct_kaleidoscope_members_score
static void gen_note_mermaid_two_distinct_kaleidoscope_members_score(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!SP-pb1-024-L");
    test_add_to_live(&tg, live);
    // 
    // // Two different KALEIDOSCORE characters.
    int ren = test_id(&tg, "PL!SP-bp1-013-PR");
    int wien = test_id(&tg, "PL!SP-PR-017-PR");
    tg.state.p[0].stage[0] = ren;
    tg.state.p[0].stage[1] = wien;
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    // 
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 1, "note_mermaid_two_distinct_kaleidoscope_members_score");
    // 
}

// test_modules/batches/untested_abilities_batch10_test.rs::note_mermaid_duplicate_kaleidoscope_names_no_score
static void gen_note_mermaid_duplicate_kaleidoscope_names_no_score(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int live = test_id(&tg, "PL!SP-pb1-024-L");
    test_add_to_live(&tg, live);
    // 
    // // Two copies of the SAME character -> not "名前の異なる".
    int ren1 = test_id(&tg, "PL!SP-bp1-013-PR");
    int ren2 = test_id(&tg, "PL!SP-pb1-013-PR");
    tg.state.p[0].stage[0] = ren1;
    tg.state.p[0].stage[1] = ren2;
    // 
    // TODO: fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    // 
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live), 0, "note_mermaid_duplicate_kaleidoscope_names_no_score");
    // 
}

// test_modules/jidou/complex/sp_bp2_015_021_yell_blade_test.rs::sumire_yell_no_blade_gains_heart06
static void gen_sumire_yell_no_blade_gains_heart06(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-015-N");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int m_no_blade = test_id(&tg, "PL!S-bp2-002-R");
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = sumire;
    tg.state.p[0].stage[2] = -1;
    // TODO: game.state.revealed_cards.clear();
    // TODO: game.state.revealed_cards.push(m_no_blade);
    // TODO: game.state.yell_occurred = true;
    test_add_to_discard(&tg, m_no_blade);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 6), 0, "sumire_yell_no_blade_gains_heart06");
    // TODO: game.state.trigger_auto_abilities_for_player("p1");
    // TODO: game.state.process_pending_auto_abilities("p1");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 6), 1, "sumire_yell_no_blade_gains_heart06");
    // 
}

// test_modules/jidou/complex/sp_bp2_015_021_yell_blade_test.rs::sumire_yell_with_blade_no_gain
static void gen_sumire_yell_with_blade_no_gain(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-015-N");
    int wien = test_id(&tg, "PL!SP-bp2-021-N");
    int m_blade = test_id(&tg, "PL!-pb1-014-R");
    // TODO: setup_yell(&mut game, sumire, wien, &[m_blade]);
    // TODO: game.state.trigger_auto_abilities_for_player("p1");
    // TODO: game.state.process_pending_auto_abilities("p1");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 6), 0, "sumire_yell_with_blade_no_gain");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, wien, 3), 0, "sumire_yell_with_blade_no_gain");
    // 
}

// test_modules/jidou/complex/sp_bp2_015_021_yell_blade_test.rs::yell_mixed_blade_and_no_blade_no_gain
static void gen_yell_mixed_blade_and_no_blade_no_gain(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-015-N");
    int wien = test_id(&tg, "PL!SP-bp2-021-N");
    int m_no_blade = test_id(&tg, "PL!S-bp2-002-R");
    int m_blade = test_id(&tg, "PL!-pb1-014-R");
    // TODO: setup_yell(&mut game, sumire, wien, &[m_no_blade, m_blade]);
    // TODO: game.state.trigger_auto_abilities_for_player("p1");
    // TODO: game.state.process_pending_auto_abilities("p1");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 6), 0, "yell_mixed_blade_and_no_blade_no_gain");
    // 
}

// test_modules/jidou/complex/sp_bp2_015_021_yell_blade_test.rs::yell_empty_revealed_no_gain
static void gen_yell_empty_revealed_no_gain(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-015-N");
    int wien = test_id(&tg, "PL!SP-bp2-021-N");
    // // Empty yell: per phases.rs yell_occurred = !revealed.is_empty() => false, so no trigger
    tg.state.p[0].stage[0] = test_id(&tg, "PL!S-sd1-003-SD");
    tg.state.p[0].stage[1] = sumire;
    tg.state.p[0].stage[2] = wien;
    // TODO: game.state.revealed_cards.clear();
    // TODO: game.state.yell_occurred = false;
    // TODO: game.state.trigger_auto_abilities_for_player("p1");
    // TODO: game.state.process_pending_auto_abilities("p1");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 6), 0, "yell_empty_revealed_no_gain");
    // 
}

// test_modules/jidou/complex/sp_bp2_015_021_yell_blade_test.rs::yell_turn1_blocks_second_trigger
static void gen_yell_turn1_blocks_second_trigger(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int sumire = test_id(&tg, "PL!SP-bp2-015-N");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int m_no_blade = test_id(&tg, "PL!S-bp2-002-R");
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = sumire;
    tg.state.p[0].stage[2] = -1;
    // TODO: game.state.revealed_cards.clear();
    // TODO: game.state.revealed_cards.push(m_no_blade);
    // TODO: game.state.yell_occurred = true;
    test_add_to_discard(&tg, m_no_blade);
    // TODO: game.state.trigger_auto_abilities_for_player("p1");
    // TODO: game.state.process_pending_auto_abilities("p1");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 6), 1, "yell_turn1_blocks_second_trigger");
    // // Second trigger same turn should be blocked by ターン1回 — clear revealed and retrigger, modifier must stay 1
    // TODO: game.state.revealed_cards.clear();
    // TODO: game.state.revealed_cards.push(m_no_blade);
    // TODO: game.state.yell_occurred = true;
    // TODO: game.state.trigger_auto_abilities_for_player("p1");
    // TODO: game.state.process_pending_auto_abilities("p1");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 6), 1, "yell_turn1_blocks_second_trigger");
    // 
}

// test_modules/jidou/complex/sp_bp2_015_021_yell_blade_test.rs::wien_yell_no_blade_gains_heart03_independent
static void gen_wien_yell_no_blade_gains_heart03_independent(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // // Only Wien on stage — Sumire absent, so only Wien triggers
    int wien = test_id(&tg, "PL!SP-bp2-021-N");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    int m_no_blade = test_id(&tg, "PL!S-bp2-002-R");
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = wien;
    tg.state.p[0].stage[2] = -1;
    // TODO: game.state.revealed_cards.clear();
    // TODO: game.state.revealed_cards.push(m_no_blade);
    // TODO: game.state.yell_occurred = true;
    // TODO: game.state.trigger_auto_abilities_for_player("p1");
    // TODO: game.state.process_pending_auto_abilities("p1");
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, wien, 3), 1, "wien_yell_no_blade_gains_heart03_independent");
    // // Sumire not on stage → no heart06 anywhere
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, filler, 6), 0, "wien_yell_no_blade_gains_heart03_independent");
    // 
}

// test_modules/jidou/moderate/location_condition_cost_test.rs::cost13_on_opponent_stage_meets_condition
static void gen_cost13_on_opponent_stage_meets_condition(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int you = test_id(&tg, "PL!S-PR-029-PR");
    int cost13_card = test_id(&tg, "PL!S-sd1-001-SD");
    // 
    tg.state.p[0].stage[0] = you;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[1].stage[0] = cost13_card;
    tg.state.p[1].stage[1] = -1;
    tg.state.p[1].stage[2] = -1;
    // 
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, you);
    CHECK_EQ(blade, 2, "cost13_on_opponent_stage_meets_condition");
    // 
}

// test_modules/jidou/moderate/location_condition_cost_test.rs::cost13_exact_boundary_triggers
static void gen_cost13_exact_boundary_triggers(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int you = test_id(&tg, "PL!S-PR-029-PR");
    int cost13_exact = test_id(&tg, "PL!-sd1-003-SD");
    tg.state.p[0].stage[0] = you;
    tg.state.p[0].stage[1] = cost13_exact;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[1].stage[0] = -1;
    tg.state.p[1].stage[1] = -1;
    tg.state.p[1].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, you), 2, "cost13_exact_boundary_triggers");
    // 
}

// test_modules/jidou/moderate/location_condition_cost_test.rs::cost12_below_threshold_no_trigger
static void gen_cost12_below_threshold_no_trigger(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int you = test_id(&tg, "PL!S-PR-029-PR");
    // // Cost 12 card: use PL!HS-bp1-003 cost 13 is still >=13, so need cost 12 — pick PL!S-bp2-009 is cost 4, or PL!N-bp1-007 is 13, so use cost 4 as below threshold representative and cost 12 is still below; the test verifies <13 does not trigger
    int cost_low = test_id(&tg, "PL!S-bp2-009-R");
    tg.state.p[0].stage[0] = you;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[1].stage[0] = cost_low;
    tg.state.p[1].stage[1] = -1;
    tg.state.p[1].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, you), 0, "cost12_below_threshold_no_trigger");
    // 
}

// test_modules/jidou/moderate/location_condition_cost_test.rs::cost13_on_self_triggers
static void gen_cost13_on_self_triggers(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int you = test_id(&tg, "PL!S-PR-029-PR");
    int cost13 = test_id(&tg, "PL!-sd1-003-SD");
    tg.state.p[0].stage[0] = you;
    tg.state.p[0].stage[1] = cost13;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, you), 2, "cost13_on_self_triggers");
    // 
}

// test_modules/jidou/moderate/location_condition_cost_test.rs::both_sides_cost13_still_only_two_blades
static void gen_both_sides_cost13_still_only_two_blades(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int you = test_id(&tg, "PL!S-PR-029-PR");
    int c13a = test_id(&tg, "PL!-sd1-003-SD");
    int c13b = test_id(&tg, "PL!-sd1-003-SD");
    tg.state.p[0].stage[0] = you;
    tg.state.p[0].stage[1] = c13a;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[1].stage[0] = c13b;
    tg.state.p[1].stage[1] = -1;
    tg.state.p[1].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, you), 2, "both_sides_cost13_still_only_two_blades");
    // 
}

// test_modules/qa/q46_kanako_all_heart_timing_test.rs::q46_kanako_condition_less_than_3_live_cards_no_gain
static void gen_q46_kanako_condition_less_than_3_live_cards_no_gain(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int kanako = test_id(&tg, "PL!N-bp1-012-R\u{ff0b}");
    int niji_live = test_id(&tg, "PL!N-sd1-025-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = kanako;
    tg.state.p[0].stage[2] = filler;
    // 
    // // Only 2 live cards → condition fails
    test_add_to_live(&tg, niji_live);
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // 
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, kanako);
    CHECK_EQ(blade, 0, "q46_kanako_condition_less_than_3_live_cards_no_gain");
    // 
}

// test_modules/qa/q46_kanako_all_heart_timing_test.rs::q46_kanako_no_nijigasaki_live_card_no_gain
static void gen_q46_kanako_no_nijigasaki_live_card_no_gain(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int kanako = test_id(&tg, "PL!N-bp1-012-R\u{ff0b}");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = kanako;
    tg.state.p[0].stage[2] = filler;
    // 
    // // 3 live cards, none are 虹ヶ咲
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // 
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, kanako);
    CHECK_EQ(blade, 0, "q46_kanako_no_nijigasaki_live_card_no_gain");
    // 
}

// test_modules/qa/q46_kanako_all_heart_timing_test.rs::q46_kanako_not_on_stage_no_constant
static void gen_q46_kanako_not_on_stage_no_constant(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int kanako = test_id(&tg, "PL!N-bp1-012-R\u{ff0b}");
    int niji_live = test_id(&tg, "PL!N-sd1-025-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    // // Kanako in hand, not on stage
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = filler;
    // 
    test_add_to_live(&tg, niji_live);
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // 
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, kanako);
    CHECK_EQ(blade, 0, "q46_kanako_not_on_stage_no_constant");
    // 
}

// test_modules/qa/q46_kanako_all_heart_timing_test.rs::q46_kanako_leaves_stage_blade_removed
static void gen_q46_kanako_leaves_stage_blade_removed(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int kanako = test_id(&tg, "PL!N-bp1-012-R\u{ff0b}");
    int niji_live = test_id(&tg, "PL!N-sd1-025-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = kanako;
    tg.state.p[0].stage[2] = filler;
    // 
    test_add_to_live(&tg, niji_live);
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // 
    test_recalc(&tg);
    // 
    int blade_before = rb_mods_get_blade(&tg.state.mods, kanako);
    CHECK_EQ(blade_before, 2, "q46_kanako_leaves_stage_blade_removed");
    // 
    // // Remove Kanako from stage
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = filler;
    test_recalc(&tg);
    // 
    int blade_after = rb_mods_get_blade(&tg.state.mods, kanako);
    CHECK_EQ(blade_after, 0, "q46_kanako_leaves_stage_blade_removed");
    // 
}

// test_modules/qa/q46_kanako_all_heart_timing_test.rs::q46_live_card_removed_condition_fails_blade_removed
static void gen_q46_live_card_removed_condition_fails_blade_removed(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int kanako = test_id(&tg, "PL!N-bp1-012-R\u{ff0b}");
    int niji_live = test_id(&tg, "PL!N-sd1-025-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = kanako;
    tg.state.p[0].stage[2] = filler;
    // 
    test_add_to_live(&tg, niji_live);
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // 
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, kanako), 2, "q46_live_card_removed_condition_fails_blade_removed");
    // 
    // // Remove 2 live cards → only 1 left → condition fails
    tg.state.p[0].live.n=0;
    test_add_to_live(&tg, niji_live);
    // 
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, kanako);
    CHECK_EQ(blade, 0, "q46_live_card_removed_condition_fails_blade_removed");
    // 
}

// test_modules/qa/q46_kanako_all_heart_timing_test.rs::q46_multiple_kanako_each_gains_blades
static void gen_q46_multiple_kanako_each_gains_blades(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int kanako1 = test_id(&tg, "PL!N-bp1-012-R\u{ff0b}");
    int kanako2 = test_id(&tg, "PL!N-bp1-012-R\u{ff0b}");
    int niji_live = test_id(&tg, "PL!N-sd1-025-SD");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    tg.state.p[0].stage[0] = kanako1;
    tg.state.p[0].stage[1] = kanako2;
    tg.state.p[0].stage[2] = filler;
    // 
    test_add_to_live(&tg, niji_live);
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // TODO: game.state
    // TODO: .player1
    // TODO: .live_card_zone
    // TODO: .cards
    // TODO: .push(test_id(&tg, "PL!-sd1-019-SD"));
    // 
    test_recalc(&tg);
    // 
    int blade1 = rb_mods_get_blade(&tg.state.mods, kanako1);
    int blade2 = rb_mods_get_blade(&tg.state.mods, kanako2);
    CHECK_EQ(blade1, 2, "q46_multiple_kanako_each_gains_blades");
    CHECK_EQ(blade2, 2, "q46_multiple_kanako_each_gains_blades");
    // 
}

// test_modules/abilities/complex/angelic_angel_test.rs::angelic_angel_in_success_with_mus_gets_plus5
static void gen_angelic_angel_in_success_with_mus_gets_plus5(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int angel = test_id(&tg, "PL!-bp4-019-L");
    int honoka = test_id(&tg, "PL!-sd1-010-SD");
    // 
    // // Angelic Angel in success zone
    test_add_to_live(&tg, angel);
    // // μ's member on stage
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = honoka;
    tg.state.p[0].stage[2] = -1;
    // 
    test_recalc(&tg);
    // 
    int score_mod = rb_mods_get_score(&tg.state.mods, angel);
    CHECK_EQ(score_mod, 5, "angelic_angel_in_success_with_mus_gets_plus5");
    // 
}

// test_modules/abilities/complex/angelic_angel_test.rs::angelic_angel_no_mus_on_stage_no_mod
static void gen_angelic_angel_no_mus_on_stage_no_mod(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int angel = test_id(&tg, "PL!-bp4-019-L");
    // 
    test_add_to_live(&tg, angel);
    // // Stage has no μ's member (non-μ's filler)
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    // 
    test_recalc(&tg);
    // 
    int score_mod = rb_mods_get_score(&tg.state.mods, angel);
    CHECK_EQ(score_mod, 0, "angelic_angel_no_mus_on_stage_no_mod");
    // 
}

// test_modules/abilities/complex/angelic_angel_test.rs::angelic_angel_mus_leaves_stage_loses_mod
static void gen_angelic_angel_mus_leaves_stage_loses_mod(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int angel = test_id(&tg, "PL!-bp4-019-L");
    int honoka = test_id(&tg, "PL!-sd1-010-SD");
    // 
    test_add_to_live(&tg, angel);
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = honoka;
    tg.state.p[0].stage[2] = -1;
    // 
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, angel), 5, "angelic_angel_mus_leaves_stage_loses_mod");
    // 
    // // Remove μ's from stage
    tg.state.p[0].stage[1] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, angel), 0, "angelic_angel_mus_leaves_stage_loses_mod");
    // 
}

// test_modules/abilities/complex/angelic_angel_test.rs::angelic_angel_removed_from_success_clears_mod
static void gen_angelic_angel_removed_from_success_clears_mod(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int angel = test_id(&tg, "PL!-bp4-019-L");
    int honoka = test_id(&tg, "PL!-sd1-010-SD");
    // 
    test_add_to_live(&tg, angel);
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = honoka;
    tg.state.p[0].stage[2] = -1;
    // 
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, angel), 5, "angelic_angel_removed_from_success_clears_mod");
    // 
    // // Remove from success zone
    tg.state.p[0].live.n=0;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, angel), 0, "angelic_angel_removed_from_success_clears_mod");
    // 
}

// test_modules/abilities/complex/angelic_angel_test.rs::angelic_angel_does_not_bleed_to_live_set_zone
static void gen_angelic_angel_does_not_bleed_to_live_set_zone(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int angel = test_id(&tg, "PL!-bp4-019-L");
    int honoka = test_id(&tg, "PL!-sd1-010-SD");
    int live_card = test_id(&tg, "PL!-sd1-021-SD");
    // 
    // // Angelic Angel in success zone
    test_add_to_live(&tg, angel);
    // // μ's on stage
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = honoka;
    tg.state.p[0].stage[2] = -1;
    // // Another live card set for the live
    test_add_to_live(&tg, live_card);
    // 
    test_recalc(&tg);
    // 
    // // Angelic Angel in success zone gets +5 (self-targeted)
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, angel), 5, "angelic_angel_does_not_bleed_to_live_set_zone");
    // 
    // // Live set zone card should NOT get the +5
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, live_card), 0, "angelic_angel_does_not_bleed_to_live_set_zone");
    // 
}

// test_modules/abilities/complex/angelic_angel_test.rs::angelic_angel_compound_and_both_required
static void gen_angelic_angel_compound_and_both_required(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int angel = test_id(&tg, "PL!-bp4-019-L");
    // 
    // // Angel in success zone ✓
    test_add_to_live(&tg, angel);
    // // But empty stage ✗ (no μ's)
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    // 
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, angel), 0, "angelic_angel_compound_and_both_required");
    // 
}

// test_modules/abilities/simple/bp7_ruby_front_blade_test.rs::ruby_left_affects_p2_right_only
static void gen_ruby_left_affects_p2_right_only(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int ruby = test_id(&tg, "PL!S-bp7-009-R");
    int opp_right = test_id(&tg, "PL!-sd1-010-SD");
    int opp_center = test_id(&tg, "PL!-sd1-010-SD");
    // 
    // // Ruby at P1 Left; cost 4 members at P2 Center and P2 Right
    tg.state.p[0].stage[0] = ruby;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[1].stage[0] = -1;
    tg.state.p[1].stage[1] = opp_center;
    tg.state.p[1].stage[2] = opp_right;
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, opp_right), -1, "ruby_left_affects_p2_right_only");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, opp_center), 0, "ruby_left_affects_p2_right_only");
    // 
}

// test_modules/abilities/simple/bp7_ruby_front_blade_test.rs::ruby_facing_ruby_both_debuffed
static void gen_ruby_facing_ruby_both_debuffed(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int ruby_p1 = test_id(&tg, "PL!S-bp7-009-R");
    int ruby_p2 = test_id(&tg, "PL!S-bp7-009-R");
    // 
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = ruby_p1;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[1].stage[0] = -1;
    tg.state.p[1].stage[1] = ruby_p2;
    tg.state.p[1].stage[2] = -1;
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, ruby_p1), -1, "ruby_facing_ruby_both_debuffed");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, ruby_p2), -1, "ruby_facing_ruby_both_debuffed");
    // 
}

// test_modules/abilities/simple/bp7_ruby_front_blade_test.rs::opponent_member_moves_out_of_front_recovers_blade
static void gen_opponent_member_moves_out_of_front_recovers_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int ruby = test_id(&tg, "PL!S-bp7-009-R");
    int opp = test_id(&tg, "PL!-sd1-010-SD");
    // 
    // // Opponent member at P2 Center (in front of Ruby)
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = ruby;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[1].stage[0] = -1;
    tg.state.p[1].stage[1] = opp;
    tg.state.p[1].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, opp), -1, "opponent_member_moves_out_of_front_recovers_blade");
    // 
    // // Opponent moves to P2 Left (slot 0)
    tg.state.p[1].stage[0] = opp;
    tg.state.p[1].stage[1] = -1;
    tg.state.p[1].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, opp), 0, "opponent_member_moves_out_of_front_recovers_blade");
    // 
}

// test_modules/abilities/simple/bp7_ruby_front_blade_test.rs::ruby_leaves_stage_modifier_removed
static void gen_ruby_leaves_stage_modifier_removed(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    int ruby = test_id(&tg, "PL!S-bp7-009-R");
    int opp = test_id(&tg, "PL!-sd1-010-SD");
    // 
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = ruby;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[1].stage[0] = -1;
    tg.state.p[1].stage[1] = opp;
    tg.state.p[1].stage[2] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, opp), -1, "ruby_leaves_stage_modifier_removed");
    // 
    tg.state.p[0].stage[1] = -1;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, opp), 0, "ruby_leaves_stage_modifier_removed");
    // 
}

// test_modules/jidou/complex/love_wing_bell_test.rs::love_wing_bell_grants_blade_to_center_mus_member
static void gen_love_wing_bell_grants_blade_to_center_mus_member(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int love_wing = test_id(&tg, "PL!-bp4-020-L");
    int muse_center = test_id(&tg, "PL!-PR-001-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    // TODO: game.state
    // TODO: .player1
    // TODO: .success_live_card_zone
    // TODO: .cards
    // TODO: .push(love_wing);
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = muse_center;
    tg.state.p[0].stage[2] = -1;
    // 
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, muse_center);
    CHECK_EQ(blade, 1, "love_wing_bell_grants_blade_to_center_mus_member");
    // 
}

// test_modules/jidou/complex/love_wing_bell_test.rs::love_wing_bell_non_mus_center_no_blade
static void gen_love_wing_bell_non_mus_center_no_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int love_wing = test_id(&tg, "PL!-bp4-020-L");
    int non_mus_center = test_id(&tg, "PL!S-bp2-008-P");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    // TODO: game.state
    // TODO: .player1
    // TODO: .success_live_card_zone
    // TODO: .cards
    // TODO: .push(love_wing);
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = non_mus_center;
    tg.state.p[0].stage[2] = -1;
    // 
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, non_mus_center);
    CHECK_EQ(blade, 0, "love_wing_bell_non_mus_center_no_blade");
    // 
}

// test_modules/jidou/complex/love_wing_bell_test.rs::love_wing_bell_mus_left_no_blade
static void gen_love_wing_bell_mus_left_no_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int love_wing = test_id(&tg, "PL!-bp4-020-L");
    int muse_left = test_id(&tg, "PL!-sd1-010-SD");
    int filler = test_id(&tg, "PL!-PR-001-PR");
    // 
    // TODO: game.state
    // TODO: .player1
    // TODO: .success_live_card_zone
    // TODO: .cards
    // TODO: .push(love_wing);
    tg.state.p[0].stage[0] = muse_left;
    tg.state.p[0].stage[1] = filler;
    tg.state.p[0].stage[2] = -1;
    // 
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, muse_left);
    CHECK_EQ(blade, 0, "love_wing_bell_mus_left_no_blade");
    // // Center μ's member should also not get blade (only center is targeted,
    // // but the effect grants to center, so actually center WOULD get blade.
    // // This test just verifies left does not.)
    // 
}

// test_modules/jidou/complex/love_wing_bell_test.rs::love_wing_bell_mus_right_no_blade
static void gen_love_wing_bell_mus_right_no_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int love_wing = test_id(&tg, "PL!-bp4-020-L");
    int muse_right = test_id(&tg, "PL!-sd1-010-SD");
    int center = test_id(&tg, "PL!-PR-001-PR");
    // 
    // TODO: game.state
    // TODO: .player1
    // TODO: .success_live_card_zone
    // TODO: .cards
    // TODO: .push(love_wing);
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = center;
    tg.state.p[0].stage[2] = muse_right;
    // 
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, muse_right);
    CHECK_EQ(blade, 0, "love_wing_bell_mus_right_no_blade");
    // 
}

// test_modules/jidou/complex/love_wing_bell_test.rs::love_wing_bell_not_in_success_zone_no_blade
static void gen_love_wing_bell_not_in_success_zone_no_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int love_wing = test_id(&tg, "PL!-bp4-020-L");
    int muse_center = test_id(&tg, "PL!-PR-001-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    // // Love wing bell in waitroom, NOT success zone
    test_add_to_discard(&tg, love_wing);
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = muse_center;
    tg.state.p[0].stage[2] = -1;
    // 
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, muse_center);
    CHECK_EQ(blade, 0, "love_wing_bell_not_in_success_zone_no_blade");
    // 
}

// test_modules/jidou/complex/love_wing_bell_test.rs::love_wing_bell_removed_from_success_zone_loses_blade
static void gen_love_wing_bell_removed_from_success_zone_loses_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int love_wing = test_id(&tg, "PL!-bp4-020-L");
    int muse_center = test_id(&tg, "PL!-PR-001-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    // TODO: game.state
    // TODO: .player1
    // TODO: .success_live_card_zone
    // TODO: .cards
    // TODO: .push(love_wing);
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = muse_center;
    tg.state.p[0].stage[2] = -1;
    // 
    // // Initially: blade should be granted
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, muse_center), 1, "love_wing_bell_removed_from_success_zone_loses_blade");
    // 
    // // Remove from success zone
    tg.state.p[0].live.n=0;
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, muse_center);
    CHECK_EQ(blade, 0, "love_wing_bell_removed_from_success_zone_loses_blade");
    // 
}

// test_modules/jidou/complex/love_wing_bell_test.rs::love_wing_bell_member_leaves_center_loses_blade
static void gen_love_wing_bell_member_leaves_center_loses_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int love_wing = test_id(&tg, "PL!-bp4-020-L");
    int muse_center = test_id(&tg, "PL!-PR-001-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    // TODO: game.state
    // TODO: .player1
    // TODO: .success_live_card_zone
    // TODO: .cards
    // TODO: .push(love_wing);
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = muse_center;
    tg.state.p[0].stage[2] = -1;
    // 
    // // Initially in center → blade
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, muse_center), 1, "love_wing_bell_member_leaves_center_loses_blade");
    // 
    // // Move to left side
    tg.state.p[0].stage[0] = muse_center;
    tg.state.p[0].stage[1] = filler;
    tg.state.p[0].stage[2] = -1;
    test_recalc(&tg);
    // 
    int blade = rb_mods_get_blade(&tg.state.mods, muse_center);
    CHECK_EQ(blade, 0, "love_wing_bell_member_leaves_center_loses_blade");
    // 
}

// test_modules/jidou/complex/love_wing_bell_test.rs::love_wing_bell_both_players_get_own_center_blade
static void gen_love_wing_bell_both_players_get_own_center_blade(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int love_wing_p1 = test_id(&tg, "PL!-bp4-020-L");
    int love_wing_p2 = test_id(&tg, "PL!-bp4-020-L");
    int muse_center_p1 = test_id(&tg, "PL!-PR-001-PR");
    int muse_center_p2 = test_id(&tg, "PL!-PR-001-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    // 
    // // Both players set up
    // TODO: game.state
    // TODO: .player1
    // TODO: .success_live_card_zone
    // TODO: .cards
    // TODO: .push(love_wing_p1);
    tg.state.p[0].stage[0] = filler;
    tg.state.p[0].stage[1] = muse_center_p1;
    tg.state.p[0].stage[2] = -1;
    // 
    // TODO: game.state
    // TODO: .player2
    // TODO: .success_live_card_zone
    // TODO: .cards
    // TODO: .push(love_wing_p2);
    tg.state.p[1].stage[0] = filler;
    tg.state.p[1].stage[1] = muse_center_p2;
    tg.state.p[1].stage[2] = -1;
    // 
    test_recalc(&tg);
    // 
    int blade_p1 = rb_mods_get_blade(&tg.state.mods, muse_center_p1);
    int blade_p2 = rb_mods_get_blade(&tg.state.mods, muse_center_p2);
    CHECK_EQ(blade_p1, 1, "love_wing_bell_both_players_get_own_center_blade");
    CHECK_EQ(blade_p2, 1, "love_wing_bell_both_players_get_own_center_blade");
    // 
}

// test_modules/jidou/complex/love_wing_bell_test.rs::love_wing_bell_only_center_gets_blade_not_left_or_right
static void gen_love_wing_bell_only_center_gets_blade_not_left_or_right(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int love_wing = test_id(&tg, "PL!-bp4-020-L");
    int left = test_id(&tg, "PL!-sd1-010-SD");
    int center = test_id(&tg, "PL!-PR-001-PR");
    int right = test_id(&tg, "PL!-sd1-005-SD");
    // 
    // TODO: game.state
    // TODO: .player1
    // TODO: .success_live_card_zone
    // TODO: .cards
    // TODO: .push(love_wing);
    tg.state.p[0].stage[0] = left;
    tg.state.p[0].stage[1] = center;
    tg.state.p[0].stage[2] = right;
    // 
    test_recalc(&tg);
    // 
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, left), 0, "love_wing_bell_only_center_gets_blade_not_left_or_right");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, center), 1, "love_wing_bell_only_center_gets_blade_not_left_or_right");
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, right), 0, "love_wing_bell_only_center_gets_blade_not_left_or_right");
    // 
}

// test_modules/jidou/complex/love_wing_bell_test.rs::love_wing_bell_empty_stage_no_crash
static void gen_love_wing_bell_empty_stage_no_crash(void){
    // 
    // db loaded via rb_load
    TestGame tg; test_game_new(&tg);
    // 
    int love_wing = test_id(&tg, "PL!-bp4-020-L");
    int muse = test_id(&tg, "PL!-PR-001-PR");
    // 
    // TODO: game.state
    // TODO: .player1
    // TODO: .success_live_card_zone
    // TODO: .cards
    // TODO: .push(love_wing);
    // // Empty stage
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    // 
    test_recalc(&tg);
    // 
    // // Now put a μ's member at center and re-evaluate.
    tg.state.p[0].stage[1] = muse;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_blade(&tg.state.mods, muse), 1, "love_wing_bell_empty_stage_no_crash");
    // 
}


static void generated_zone_conversion(void){
    RbZone z;
    CHECK(rb_zone_of_str("hand",&z)==1 && z==RB_ZONE_HAND,"gen: hand");
    CHECK(rb_zone_of_str("stage",&z)==1 && z==RB_ZONE_STAGE,"gen: stage");
}

int main(void){
    if(rb_load("src")!=0){ fprintf(stderr,"rb_load failed\\n"); return 1; }
    printf("=== generated mass-port batch (simple constants) ===\\n");
    gen_ren_005_turn2_blocks_third_energy_placed();
    gen_sd2_002_natural_move_no_heart();
    gen_sp_bp2_004_center_highest_with_only_two_members();
    gen_sp_bp2_004_center_low_with_two_members_no_heart();
    gen_sp_bp2_004_center_highest_with_only_sumire_at_center();
    gen_sp_bp2_004_no_gain_when_center_is_lowest();
    gen_sp_bp2_004_p_variant_tie_no_heart();
    gen_sp_bp2_004_center_empty_no_heart();
    gen_sp_bp2_004_center_highest_with_two_empty_sides();
    gen_sp_bp2_004_no_center_no_heart();
    gen_sp_bp2_004_all_three_same_cost_no_heart();
    gen_sp_bp2_004_center_highest_with_low_left_right();
    gen_sp_bp2_004_center_highest_with_high_left_low_right();
    gen_sp_bp2_004_center_lowest_no_heart();
    gen_sp_bp2_004_center_highest_with_only_center();
    gen_sp_bp2_004_no_stage_no_heart();
    gen_sp_bp2_004_p_variant_center_highest();
    gen_sp_bp2_004_center_tie_no_heart_p_variant();
    gen_sp_bp2_004_center_highest_with_high_right_low_left();
    gen_sp_bp2_004_no_gain_when_center_empty_and_others_present();
    gen_sp_bp2_004_center_highest_with_tie_left_right_no_heart();
    gen_sp_bp2_004_sumire_at_right_center_highest_gains();
    gen_erena_p_variant_wait_gains_heart();
    gen_erena_wait_then_active_loses_heart();
    gen_sumire_pr_center_position_grants_blade();
    gen_sp_bp2_004_all_empty_no_heart();
    gen_sp_bp2_004_center_only_one_member_gains();
    gen_rin_q79_vacated_area_can_receive_new_member();
    gen_keke_jidou_effect_only_false_no_trigger();
    gen_keke_jidou_self_effect_move_triggers();
    gen_wish_song_five_distinct_liella_revealed_scores();
    gen_wish_song_four_distinct_liella_revealed_no_score();
    gen_rin_only_ls_live_no_blade();
    gen_sp_bp2_004_center_tie_with_both_sides_no_heart();
    gen_sp_bp2_004_center_highest_with_empty_side();
    gen_sp_bp2_004_p_variant_same();
    gen_sp_bp5_011_position_hearts();
    gen_sb7_005_aqours_under_card_blade();
    gen_hs_bp2_006_per_other_mirakuraku_member_blade();
    gen_spb1_005_opponent_more_energy_grants_blade();
    gen_temporary_live_end_effect_expires_when_live_phase_ends();
    gen_sd2_004_center_blade_plus4();
    gen_pb2_035_left_blade_plus2();
    gen_pb2_041_right_blade_plus2();
    gen_miyashita_ai_only_two_debuts_no_draw();
    gen_tomari_jidou_self_move_triggers();
    gen_tomari_jidou_opponent_move_triggers();
    gen_tomari_jidou_natural_move_no_trigger();
    gen_cl1_006_debut_gains_three_blades();
    gen_sd1026_energy_below_nine_no_score();
    gen_treco_please_aqours_h04_total_ten_scores_two();
    gen_treco_please_aqours_h04_below_ten_no_score();
    gen_q267_deck_exhausts_mid_mill_refreshes_and_completes();
    gen_q267_deck_exactly_seven_no_refresh();
    gen_q267_deck_five_refresh_completes_to_seven();
    gen_q267_both_players_refresh_their_own_deck();
    gen_wien_cost_modifier_dynamic();
    gen_wien_cost_modifier_cleared_on_leave();
    gen_bp6022_more_opp_energy_scores();
    gen_bp6022_equal_or_fewer_opp_energy_no_score();
    gen_bp6012_debut_activates_energy_with_sbuuke_teammate();
    gen_spr039_constant_blades_with_combined_success_cards();
    gen_spr039_constant_blades_off_below_four_combined();
    gen_bp7020_constant_blades_while_energy_ahead();
    gen_bp7020_constant_blades_off_when_behind();
    gen_sd1022_live_start_grants_blade_to_all_aqours_members();
    gen_yell_all_blade_blocks_sumire_and_wien();
    gen_yell_only_one_present_triggers_self_only();
    gen_go_master_start_fewer_success_cards_scores();
    gen_go_master_start_equal_success_cards_no_score();
    gen_note_mermaid_two_distinct_kaleidoscope_members_score();
    gen_note_mermaid_duplicate_kaleidoscope_names_no_score();
    gen_sumire_yell_no_blade_gains_heart06();
    gen_sumire_yell_with_blade_no_gain();
    gen_yell_mixed_blade_and_no_blade_no_gain();
    gen_yell_empty_revealed_no_gain();
    gen_yell_turn1_blocks_second_trigger();
    gen_wien_yell_no_blade_gains_heart03_independent();
    gen_cost13_on_opponent_stage_meets_condition();
    gen_cost13_exact_boundary_triggers();
    gen_cost12_below_threshold_no_trigger();
    gen_cost13_on_self_triggers();
    gen_both_sides_cost13_still_only_two_blades();
    gen_q46_kanako_condition_less_than_3_live_cards_no_gain();
    gen_q46_kanako_no_nijigasaki_live_card_no_gain();
    gen_q46_kanako_not_on_stage_no_constant();
    gen_q46_kanako_leaves_stage_blade_removed();
    gen_q46_live_card_removed_condition_fails_blade_removed();
    gen_q46_multiple_kanako_each_gains_blades();
    gen_angelic_angel_in_success_with_mus_gets_plus5();
    gen_angelic_angel_no_mus_on_stage_no_mod();
    gen_angelic_angel_mus_leaves_stage_loses_mod();
    gen_angelic_angel_removed_from_success_clears_mod();
    gen_angelic_angel_does_not_bleed_to_live_set_zone();
    gen_angelic_angel_compound_and_both_required();
    gen_ruby_left_affects_p2_right_only();
    gen_ruby_facing_ruby_both_debuffed();
    gen_opponent_member_moves_out_of_front_recovers_blade();
    gen_ruby_leaves_stage_modifier_removed();
    gen_love_wing_bell_grants_blade_to_center_mus_member();
    gen_love_wing_bell_non_mus_center_no_blade();
    gen_love_wing_bell_mus_left_no_blade();
    gen_love_wing_bell_mus_right_no_blade();
    gen_love_wing_bell_not_in_success_zone_no_blade();
    gen_love_wing_bell_removed_from_success_zone_loses_blade();
    gen_love_wing_bell_member_leaves_center_loses_blade();
    gen_love_wing_bell_both_players_get_own_center_blade();
    gen_love_wing_bell_only_center_gets_blade_not_left_or_right();
    gen_love_wing_bell_empty_stage_no_crash();
    generated_zone_conversion();
    rb_unload();
    if(failures){ printf("\\n%d FAILURES\\n",failures); return 1; }
    printf("\\nALL GENERATED CHECKS PASSED\\n");
    printf("generated: 109 fns\\n");
    return 0;
}
