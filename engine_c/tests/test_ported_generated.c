#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
static int failures=0;
#define CHECK(c,msg) do{ if(!(c)){ fprintf(stderr,"FAIL %s:%d: %s\n",__FILE__,__LINE__,msg); failures++; } else printf("ok: %s\n",msg);} while(0)
#define CHECK_EQ(a,b,msg) do{ if((a)!=(b)){ fprintf(stderr,"FAIL %s:%d: %s (got %d expected %d)\n",__FILE__,__LINE__,msg,(int)(a),(int)(b)); failures++; } else printf("ok: %s\n",msg);} while(0)

/* generated — mass-port of simple constant tests (recalculate_constants) */
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_only_two_members heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_center_low_with_two_members_no_heart heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_only_sumire_at_center heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_no_gain_when_center_is_lowest heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire_p, 3), 0, "sp_bp2_004_p_variant_tie_no_heart heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_two_empty_sides heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_no_center_no_heart heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_all_three_same_cost_no_heart heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_low_left_right heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_center_highest_with_high_left_low_right heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_center_lowest_no_heart heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_only_center heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_no_stage_no_heart heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire_p, 3), 1, "sp_bp2_004_p_variant_center_highest heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire_p, 3), 0, "sp_bp2_004_center_tie_no_heart_p_variant heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_highest_with_high_right_low_left heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_no_gain_when_center_empty_and_others_present heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire_id, 3), 0, "sp_bp2_004_center_highest_with_tie_left_right_no_heart heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_sumire_at_right_center_highest_gains heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 0, "sp_bp2_004_all_empty_no_heart heart");
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
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, sumire, 3), 1, "sp_bp2_004_center_only_one_member_gains heart");
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
    gen_sp_bp2_004_center_highest_with_only_two_members();
    gen_sp_bp2_004_center_low_with_two_members_no_heart();
    gen_sp_bp2_004_center_highest_with_only_sumire_at_center();
    gen_sp_bp2_004_no_gain_when_center_is_lowest();
    gen_sp_bp2_004_p_variant_tie_no_heart();
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
    gen_sp_bp2_004_all_empty_no_heart();
    gen_sp_bp2_004_center_only_one_member_gains();
    generated_zone_conversion();
    rb_unload();
    if(failures){ printf("\\n%d FAILURES\\n",failures); return 1; }
    printf("\\nALL GENERATED CHECKS PASSED\\n");
    printf("generated: 21 fns\\n");
    return 0;
}
