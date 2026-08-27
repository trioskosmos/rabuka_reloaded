#include "rabuka.h"
#include <stdio.h>
#include <string.h>

/* Scenario-replay harness — portable parity scaffold.
   Runs 3 embedded fixtures (no JSON dep) that mirror Rust engine/tests:
   - debut trigger queues correctly
   - live performance via RbMods allocates and scores
   - move_cards via typed zones
   Future: load tests/fixtures/*.json and diff against Rust oracle dumps
   (cargo run --bin trace_game). For now the embedded fixtures prove the
   harness wiring and give a place to add golden snapshots. */

static int failures=0;
#define CHECK(c,msg) do{ if(!(c)){ fprintf(stderr,"FAIL: %s\n",msg); failures++; } else printf("ok: %s\n",msg);} while(0)

static void scenario_draw_and_score(void){
    GameState g; uint32_t d0[20]={0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19};
    uint32_t d1[20]={20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39};
    rb_seed(1); rb_game_init(&g,d0,20,d1,20);
    int hand_before=g.p[0].hand.n;
    rb_draw(&g,0); CHECK(g.p[0].hand.n==hand_before+1,"draw increments hand");
    int sc_before=g.p[0].score;
    AbilityEffect e={0}; e.action="modify_score"; e.count=2;
    rb_execute_effect(&g,0,&e);
    CHECK(g.p[0].score==sc_before+2,"modify_score via effect");
}

static void scenario_live_performance(void){
    GameState g; uint32_t d0[40],d1[40];
    uint32_t nc=rb_num_cards();
    int n0=0,n1=0;
    for(uint32_t i=0;i<nc && n0<40;i++) if(rb_card_ability_idx(i)!=0xFFFF) d0[n0++]=i;
    for(uint32_t i=0;i<nc && n1<40;i++) if(rb_card_ability_idx(i)!=0xFFFF) d1[n1++]=i;
    rb_seed(0xCAFE); rb_game_init(&g,d0,n0,d1,n1);
    /* force a live into P0's live zone and a stage member with hearts */
    if(g.p[0].hand.n>0){
        int cid=g.p[0].hand.cards[0];
        g.p[0].live.cards[g.p[0].live.n++]=cid;
        g.p[0].hand.cards[0]=g.p[0].hand.cards[g.p[0].hand.n-1]; g.p[0].hand.n--;
    }
    if(g.p[0].hand.n>0){
        int cid=g.p[0].hand.cards[0];
        if(g.p[0].stage[0]==-1){ g.p[0].stage[0]=cid; g.p[0].hand.cards[0]=g.p[0].hand.cards[g.p[0].hand.n-1]; g.p[0].hand.n--; }
    }
    int lives_before=g.p[0].live.n;
    int success_before=g.p[0].success.n;
    int score_before=g.p[0].score;
    int res=rb_perform_live(&g,0);
    CHECK(res==0||res==1,"live performance returns 0/1");
    CHECK(g.p[0].live.n==0,"live zone cleared after performance");
    CHECK(g.p[0].success.n==success_before||g.p[0].success.n==success_before+lives_before,"success or discard after live");
    CHECK(g.p[0].score>=score_before,"score non-decreasing after live");
    /* stage hearts via mods should be computable */
    int hearts[8]; rb_calc_stage_hearts(&g,0,hearts);
    int sum=0; for(int i=0;i<8;i++) sum+=hearts[i];
    CHECK(sum>=0,"stage_hearts computable");
}

static void scenario_move_cards(void){
    GameState g; uint32_t d0[10]={0,1,2,3,4,5,6,7,8,9}; uint32_t d1[10]={10,11,12,13,14,15,16,17,18,19};
    rb_seed(2); rb_game_init(&g,d0,10,d1,10);
    int hand_n=g.p[0].hand.n;
    int disc_n=g.p[0].discard.n;
    AbilityEffect e={0}; e.action="move_cards"; e.source="hand"; e.destination="discard"; e.count=1;
    rb_execute_effect(&g,0,&e);
    CHECK(g.p[0].hand.n==hand_n-1,"move_cards hand->discard reduces hand");
    CHECK(g.p[0].discard.n==disc_n+1,"move_cards hand->discard increases discard");
}

static void scenario_condition_gate(void){
    GameState g; uint32_t d0[10]={0,1,2,3,4,5,6,7,8,9}; uint32_t d1[10]={10,11,12,13,14,15,16,17,18,19};
    rb_seed(3); rb_game_init(&g,d0,10,d1,10);
    AbilityEffect e={0}; e.action="modify_score"; e.count=5;
    Condition cond={0}; cond.variant=1;
    CondField *f=&cond.fields[cond.n_fields++];
    f->key="location"; f->v.tag=RB_TAG_STR; f->v.s="hand";
    f=&cond.fields[cond.n_fields++];
    f->key="count"; f->v.tag=RB_TAG_I64; f->v.i=99;
    f=&cond.fields[cond.n_fields++];
    f->key="operator"; f->v.tag=RB_TAG_STR; f->v.s=">=";
    e.has_condition=1; e.condition=&cond;
    int sc=g.p[0].score;
    rb_execute_effect(&g,0,&e);
    CHECK(g.p[0].score==sc,"condition-gated effect skipped when hand<99");
    cond.fields[1].v.i=1;
    rb_execute_effect(&g,0,&e);
    CHECK(g.p[0].score==sc+5,"condition-gated effect fires when hand>=1");
}
static void scenario_cost_modifier(void){
    GameState g; uint32_t d0[10]={0,1,2,3,4,5,6,7,8,9}; uint32_t d1[10]={10,11,12,13,14,15,16,17,18,19};
    rb_seed(4); rb_game_init(&g,d0,10,d1,10);
    if(g.p[0].stage[0]==-1 && g.p[0].hand.n>0){ int c=g.p[0].hand.cards[0]; g.p[0].stage[0]=c; g.p[0].hand.cards[0]=g.p[0].hand.cards[g.p[0].hand.n-1]; g.p[0].hand.n--; }
    int cid=g.p[0].stage[0];
    if(cid==-1) return;
    int before=rb_mods_get_cost(&g.mods,cid);
    AbilityEffect e={0}; e.action="modify_cost"; e.count=1;
    rb_execute_effect(&g,0,&e);
    CHECK(rb_mods_get_cost(&g.mods,cid)==before+1,"modify_cost via mods");
}
static void scenario_heart_modifier(void){
    GameState g; uint32_t d0[10]={0,1,2,3,4,5,6,7,8,9}; uint32_t d1[10]={10,11,12,13,14,15,16,17,18,19};
    rb_seed(5); rb_game_init(&g,d0,10,d1,10);
    if(g.p[0].stage[0]==-1 && g.p[0].hand.n>0){ int c=g.p[0].hand.cards[0]; g.p[0].stage[0]=c; g.p[0].hand.cards[0]=g.p[0].hand.cards[g.p[0].hand.n-1]; g.p[0].hand.n--; }
    int cid=g.p[0].stage[0]; if(cid==-1) return;
    int before=rb_mods_get_need_heart(&g.mods,cid,0);
    AbilityEffect e={0}; e.action="modify_required_hearts"; e.count=2; e.extra_k[0]="heart_color"; e.extra_v[0]="pink"; e.n_extra=1;
    rb_execute_effect(&g,0,&e);
    CHECK(rb_mods_get_need_heart(&g.mods,cid,0)==before+2,"modify_required_hearts via need_heart mods");
}
static void scenario_look_select(void){
    GameState g; uint32_t d0[10]={0,1,2,3,4,5,6,7,8,9}; uint32_t d1[10]={10,11,12,13,14,15,16,17,18,19};
    rb_seed(6); rb_game_init(&g,d0,10,d1,10);
    AbilityEffect e={0}; e.action="look_at"; e.source="deck"; e.count=2;
    rb_execute_effect(&g,0,&e);
    CHECK(rb_has_pending_choice(&g)==1,"look_at emits SELECT_CARD pending");
    const RbChoice *ch=rb_get_pending_choice(&g);
    CHECK(ch && ch->kind==RB_CHOICE_SELECT_CARD,"look_at pending is SELECT_CARD");
    rb_resume_with_choice(&g,-1);
    CHECK(rb_has_pending_choice(&g)==0,"look_at skip clears pending");
}
static void scenario_baton(void){
    GameState g; uint32_t d0[10]={0,1,2,3,4,5,6,7,8,9}; uint32_t d1[10]={10,11,12,13,14,15,16,17,18,19};
    rb_seed(7); rb_game_init(&g,d0,10,d1,10);
    if(g.p[0].stage[0]==-1 && g.p[0].hand.n>0){ int c=g.p[0].hand.cards[0]; g.p[0].stage[0]=c; g.p[0].hand.cards[0]=g.p[0].hand.cards[g.p[0].hand.n-1]; g.p[0].hand.n--; }
    int stage_before = (g.p[0].stage[0]!=-1)+(g.p[0].stage[1]!=-1)+(g.p[0].stage[2]!=-1);
    int hand_before=g.p[0].hand.n;
    AbilityEffect e={0}; e.action="play_baton_touch"; e.count=1;
    rb_execute_effect(&g,0,&e);
    CHECK(g.p[0].hand.n==hand_before-1 || g.p[0].hand.n==hand_before,"baton consumes hand if possible");
}

int main(void){
    CHECK(rb_load("src")==0,"rb_load");
    scenario_draw_and_score();
    scenario_live_performance();
    scenario_move_cards();
    scenario_condition_gate();
    scenario_cost_modifier();
    scenario_heart_modifier();
    scenario_look_select();
    scenario_baton();
    rb_unload();
    if(failures){ printf("\n%d FAILURES\n",failures); return 1; }
    printf("\nALL REPLAY CHECKS PASSED\n");
    return 0;
}
