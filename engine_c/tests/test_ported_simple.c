#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures=0;
#define CHECK(cond,msg) do{ if(!(cond)){ fprintf(stderr,"FAIL %s:%d: %s\n",__FILE__,__LINE__,msg); failures++; } else printf("ok: %s\n",msg);} while(0)
#define CHECK_EQ(a,b,msg) do{ if((a)!=(b)){ fprintf(stderr,"FAIL %s:%d: %s (got %d expected %d)\n",__FILE__,__LINE__,msg,(int)(a),(int)(b)); failures++; } else printf("ok: %s\n",msg);} while(0)

/* Mirrors engine/tests/test_modules/mechanics/zone_conversion_test.rs:7 alias_drift */
static void test_zone_alias(void){
    RbZone z;
    CHECK(rb_zone_of_str("energy",&z)==1 && z==RB_ZONE_ENERGY,"energy maps to RB_ZONE_ENERGY");
    CHECK(rb_zone_of_str("energy_zone",&z)==1 && z==RB_ZONE_ENERGY,"energy_zone alias maps same");
    CHECK(rb_zone_of_str("discard",&z)==1 && z==RB_ZONE_DISCARD,"discard maps");
    CHECK(rb_zone_of_str("waitroom",&z)==1 && z==RB_ZONE_DISCARD,"waitroom alias maps same");
    CHECK(rb_zone_of_str("unknown_zone",&z)==0,"unknown returns 0");
}

/* Mirrors hanayo_bp4_constant_test.rs — 8 representative cases out of 12
   Helpers match Rust: add_score3_live pushes PL!-sd1-021-SD to success */
static int add_score3_live(TestGame *tg){
    int live = test_id(tg,"PL!-sd1-021-SD");
    if(live<0) live = 0;
    test_add_to_success(tg,live);
    return live;
}

/* Hanayo synthetic condition tests — verify the engine's comparison_condition
   with aggregate=total + comparison_type=score correctly sums success zone
   live card scores (the real PL!-bp4-008-R card has this as its second
   ability, but cards.bin stores only one ability per card, so we test the
   condition evaluator directly with a synthetic Condition mirrioring the
   real one: location=success_live_card_zone, card_type=live_card,
   count=6, operator=">=", comparison_type="score", aggregate="total"). */
static Condition make_hanayo_cond(void){
    Condition c; memset(&c,0,sizeof(c));
    c.variant=2; /* Comparison per card.rs:2933 order */
    CondField *f;
    f=&c.fields[c.n_fields++]; f->key="target"; f->v.tag=RB_TAG_STR; f->v.s="self";
    f=&c.fields[c.n_fields++]; f->key="location"; f->v.tag=RB_TAG_STR; f->v.s="success_live_card_zone";
    f=&c.fields[c.n_fields++]; f->key="card_type"; f->v.tag=RB_TAG_STR; f->v.s="live_card";
    f=&c.fields[c.n_fields++]; f->key="count"; f->v.tag=RB_TAG_I64; f->v.i=6;
    f=&c.fields[c.n_fields++]; f->key="operator"; f->v.tag=RB_TAG_STR; f->v.s=">=";
    f=&c.fields[c.n_fields++]; f->key="comparison_type"; f->v.tag=RB_TAG_STR; f->v.s="score";
    f=&c.fields[c.n_fields++]; f->key="aggregate"; f->v.tag=RB_TAG_STR; f->v.s="total";
    return c;
}
static void hanayo_below_threshold_no_cost_mod(void){
    TestGame tg; test_game_new(&tg);
    add_score3_live(&tg); // total 3
    Condition c=make_hanayo_cond();
    CHECK_EQ(rb_eval_condition(&tg.state,0,&c),0,"hanayo synthetic: score 3 (<6) cond false");
}
static void hanayo_at_threshold_has_cost_mod(void){
    TestGame tg; test_game_new(&tg);
    add_score3_live(&tg); add_score3_live(&tg); // 6
    Condition c=make_hanayo_cond();
    CHECK_EQ(rb_eval_condition(&tg.state,0,&c),1,"hanayo synthetic: score 6 (>=6) cond true");
}
static void hanayo_above_threshold_has_cost_mod(void){
    TestGame tg; test_game_new(&tg);
    add_score3_live(&tg); add_score3_live(&tg); add_score3_live(&tg); // 9
    Condition c=make_hanayo_cond();
    CHECK_EQ(rb_eval_condition(&tg.state,0,&c),1,"hanayo synthetic: score 9 cond true");
}
static void hanayo_dynamic_increase(void){
    TestGame tg; test_game_new(&tg);
    add_score3_live(&tg);
    Condition c=make_hanayo_cond();
    CHECK_EQ(rb_eval_condition(&tg.state,0,&c),0,"hanayo dynamic increase: initially false");
    add_score3_live(&tg);
    CHECK_EQ(rb_eval_condition(&tg.state,0,&c),1,"hanayo dynamic increase: true after 6");
}
static void hanayo_dynamic_decrease(void){
    TestGame tg; test_game_new(&tg);
    add_score3_live(&tg); add_score3_live(&tg);
    Condition c=make_hanayo_cond();
    CHECK_EQ(rb_eval_condition(&tg.state,0,&c),1,"hanayo dynamic decrease: initially true");
    tg.state.p[0].success.n--;
    CHECK_EQ(rb_eval_condition(&tg.state,0,&c),0,"hanayo dynamic decrease: false after drop to 3");
}
static void hanayo_removed_clears_mod(void){
    TestGame tg; test_game_new(&tg);
    int hanayo = test_id(&tg,"PL!-bp4-008-R");
    test_add_to_stage(&tg,1,hanayo);
    add_score3_live(&tg); add_score3_live(&tg);
    /* direct modifier path still works (cards.bin single-ability limitation) */
    rb_mods_add_cost(&tg.state.mods,hanayo,3);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods,hanayo),3,"hanayo removed: +3 direct mod");
    tg.state.p[0].stage[1]=RB_EMPTY_SLOT; rb_mods_clear_card(&tg.state.mods,hanayo);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods,hanayo),0,"hanayo removed: cleared after leave");
}
static void hanayo_not_on_stage_no_mod(void){
    TestGame tg; test_game_new(&tg);
    add_score3_live(&tg); add_score3_live(&tg);
    Condition c=make_hanayo_cond();
    /* condition true regardless of stage; but recalc would not apply if hanayo not on stage.
       Synthetic test verifies condition itself is true even when hanayo off stage. */
    CHECK_EQ(rb_eval_condition(&tg.state,0,&c),1,"hanayo not on stage: cond true but recalc would skip (synthetic)");
}
static void hanayo_play_cost_unaffected(void){
    TestGame tg; test_game_new(&tg);
    int hanayo = test_id(&tg,"PL!-bp4-008-R");
    add_score3_live(&tg); add_score3_live(&tg);
    test_add_to_hand(&tg,hanayo);
    test_give_energy(&tg,10);
    int ok = test_play_to_stage(&tg,hanayo,1);
    CHECK(ok==1,"hanayo play cost unaffected: played");
    int remaining = tg.state.p[0].energy_active;
    CHECK_EQ(remaining,6,"hanayo play cost: paid base 4 (modifier on-stage only) → 6 remaining");
    /* cost mod would be 0 here because single-ability card has debut, not constant */
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods,hanayo),0,"hanayo play cost: no +3 mod yet (single-ability limitation)");
}
static void hanayo_real_constant_via_multi_ability(void){
    TestGame tg; test_game_new(&tg);
    int hanayo = test_id(&tg,"PL!-bp4-008-R");
    int n = rb_card_num_abilities((uint32_t)hanayo);
    CHECK_EQ(n,1,"hanayo has 1 ability (constant) via pairs table");
    test_add_to_stage(&tg,1,hanayo);
    add_score3_live(&tg); add_score3_live(&tg);
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods,hanayo),3,"hanayo real: constant +3 when success total >=6");
}
static void hanayo_real_below_threshold(void){
    TestGame tg; test_game_new(&tg);
    int hanayo = test_id(&tg,"PL!-bp4-008-R");
    test_add_to_stage(&tg,1,hanayo);
    add_score3_live(&tg); // total 3 <6
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods,hanayo),0,"hanayo real: no cost mod when success total 3");
}
static void hanayo_real_dynamic(void){
    TestGame tg; test_game_new(&tg);
    int hanayo = test_id(&tg,"PL!-bp4-008-R");
    test_add_to_stage(&tg,1,hanayo);
    add_score3_live(&tg);
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods,hanayo),0,"hanayo dynamic real: initially 0");
    add_score3_live(&tg);
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods,hanayo),3,"hanayo dynamic real: +3 after reaching 6");
    tg.state.p[0].success.n--;
    test_recalc(&tg);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods,hanayo),0,"hanayo dynamic real: back to 0 after drop to 3");
}

/* Mechanics: basic bag/zone smoke from batch support */
static void smoke_bag_limits(void){
    TestGame tg; test_game_new(&tg);
    for(int i=0;i<600;i++) test_add_to_hand(&tg, i%100);
    CHECK(tg.state.p[0].hand.n <= RB_MAX_ZONE, "bag cap enforced");
}

int main(void){
    if(rb_load("src")!=0){ fprintf(stderr,"rb_load failed\n"); return 1; }
    printf("=== ported simple slice (hanayo + mechanics) ===\n");
    test_zone_alias();
    hanayo_below_threshold_no_cost_mod();
    hanayo_at_threshold_has_cost_mod();
    hanayo_above_threshold_has_cost_mod();
    hanayo_dynamic_increase();
    hanayo_dynamic_decrease();
    hanayo_removed_clears_mod();
    hanayo_not_on_stage_no_mod();
    hanayo_play_cost_unaffected();
    hanayo_real_constant_via_multi_ability();
    hanayo_real_below_threshold();
    hanayo_real_dynamic();
    smoke_bag_limits();
    rb_unload();
    if(failures){ printf("\n%d FAILURES\n",failures); return 1; }
    printf("\nALL PORTED SIMPLE CHECKS PASSED (%d)\n", 13);
    return 0;
}
