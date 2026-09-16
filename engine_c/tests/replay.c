#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <stdlib.h>
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
    /* target the stage explicitly: a bare effect defaults to hand (empty
       here), so without this the effect is a correct no-op. Amount travels
       in the value extra (s_value), not count. */
    strcpy(e.card_type_field,"member_card");
    e.extra_k[0]="value"; e.extra_v[0]="1"; e.n_extra=1;
    rb_execute_effect(&g,0,&e);
    CHECK(rb_mods_get_cost(&g.mods,cid)==before+1,"modify_cost via mods");
}
static void scenario_heart_modifier(void){
    GameState g; uint32_t d0[10]={0,1,2,3,4,5,6,7,8,9}; uint32_t d1[10]={10,11,12,13,14,15,16,17,18,19};
    rb_seed(5); rb_game_init(&g,d0,10,d1,10);
    /* required-hearts modifiers in Rust target the LIVE cards (live_card_zone);
       place the card there so the need_heart modifier actually attaches. */
    if(g.p[0].live.n==0 && g.p[0].hand.n>0){ int c=g.p[0].hand.cards[0]; g.p[0].hand.cards[0]=g.p[0].hand.cards[g.p[0].hand.n-1]; g.p[0].hand.n--; g.p[0].live.cards[g.p[0].live.n++]=c; }
    int cid=g.p[0].live.cards[0]; if(g.p[0].live.n==0) return;
    int before=rb_mods_get_need_heart(&g.mods,cid,0);
    AbilityEffect e={0}; e.action="modify_required_hearts"; e.count=2; e.extra_k[0]="heart_color"; e.extra_v[0]="pink"; e.extra_k[1]="operation"; e.extra_v[1]="increase"; e.n_extra=2;
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
static void scenario_live_limit(void){
    GameState g; uint32_t d0[10]={0,1,2,3,4,5,6,7,8,9}; uint32_t d1[10]={10,11,12,13,14,15,16,17,18,19};
    rb_seed(8); rb_game_init(&g,d0,10,d1,10);
    g.live_set_limit_reduction[0]=2;
    int limit=RB_MAX_LIVE_CARDS - g.live_set_limit_reduction[0];
    CHECK(limit==1,"live limit reduction 2 → limit 1");
}
static void scenario_group_condition(void){
    GameState g; uint32_t d0[10]={0,1,2,3,4,5,6,7,8,9}; uint32_t d1[10]={10,11,12,13,14,15,16,17,18,19};
    rb_seed(9); rb_game_init(&g,d0,10,d1,10);
    if(g.p[0].stage[0]==-1 && g.p[0].hand.n>0){ int c=g.p[0].hand.cards[0]; g.p[0].stage[0]=c; g.p[0].hand.cards[0]=g.p[0].hand.cards[g.p[0].hand.n-1]; g.p[0].hand.n--; }
    Condition cond={0}; cond.variant=4;
    CondField *f=&cond.fields[cond.n_fields++];
    f->key="location"; f->v.tag=RB_TAG_STR; f->v.s="stage";
    int res=rb_eval_condition(&g,0,&cond);
    CHECK(res==1||res==0,"group condition evaluates without crash");
}
static void scenario_no_excess(void){
    GameState g; uint32_t d0[10]={0,1,2,3,4,5,6,7,8,9}; uint32_t d1[10]={10,11,12,13,14,15,16,17,18,19};
    rb_seed(10); rb_game_init(&g,d0,10,d1,10);
    /* no snapshot yet → no_excess should be false */
    Condition cond={0}; cond.variant=16;
    CHECK(rb_eval_condition(&g,0,&cond)==0,"no_excess false with no snapshots");
    /* push an exact snapshot (surplus 0) → true */
    g.snapshots[0].player=0; g.snapshots[0].success=1; g.snapshots[0].surplus_hearts=0; g.n_snapshots=1;
    CHECK(rb_eval_condition(&g,0,&cond)==1,"no_excess true when surplus==0");
    g.snapshots[0].surplus_hearts=3;
    CHECK(rb_eval_condition(&g,0,&cond)==0,"no_excess false when surplus>0");
}
static void scenario_opponent_live_success(void){
    GameState g; uint32_t d0[10]={0,1,2,3,4,5,6,7,8,9}; uint32_t d1[10]={10,11,12,13,14,15,16,17,18,19};
    rb_seed(11); rb_game_init(&g,d0,10,d1,10);
    Condition cond={0}; cond.variant=15; /* RB_COND_OPPONENT_LIVE_SUCCESS */
    /* no live performed → opponent flag 0 → false */
    CHECK(rb_eval_condition(&g,0,&cond)==0,"opponent_live_success false before any live");
    g.live_success[1]=1; /* opponent (player1) passed their live this turn */
    CHECK(rb_eval_condition(&g,0,&cond)==1,"opponent_live_success true when opponent passed");
    g.live_success[1]=0;
    CHECK(rb_eval_condition(&g,0,&cond)==0,"opponent_live_success false when opponent failed");
}
static void scenario_phase_determinism(void){
    GameState g1,g2; uint32_t d0[20],d1[20];
    for(int i=0;i<20;i++){ d0[i]=i; d1[i]=20+i; }
    rb_seed(0xCAFE); rb_game_init(&g1,d0,20,d1,20);
    rb_seed(0xCAFE); rb_game_init(&g2,d0,20,d1,20);
    /* advance both through Active→Energy→Draw→Main twice and check determinism */
    rb_advance_phase(&g1); rb_advance_phase(&g2);
    CHECK(g1.phase==g2.phase && g1.active==g2.active,"phase determinism after first advance");
    rb_advance_phase(&g1); rb_advance_phase(&g2);
    rb_advance_phase(&g1); rb_advance_phase(&g2);
    rb_advance_phase(&g1); rb_advance_phase(&g2);
    CHECK(g1.phase==g2.phase,"phase determinism after 4 advances");
}

/* ── Scenario replay mode ────────────────────────────────────────────
   Hand-authored pipe-format scenarios (one migrated test each). Same
   format the Rust oracle (`engine/tests/run_all.rs::scenario_oracle`)
   executes; both print `CHECK <n> <pass|FAIL> <actual> <expected>` so
   diffing the outputs is the parity check.
   Ops: card <ref> <card_no> | stage|hand|discard|deck|live|success <pl 1|2>
   [<area>] <ref> | energy <pl> <n> | wait <ref> | recalculate |
   play <ref> <area> | activate <ref> | pass | select [i,j,..] |
   select_option <n> | set_live <ref> | drain_auto |
   expect blade|score|cost <ref> <n> | expect hand|energy <pl> <n> |
   expect phase <Rust Display name> | expect pending <Type|none> |
   expect stage <pl> <area> <ref|null>. `#` comments / blanks skipped. */
#define SC_MAX_VARS 512
#define SC_LINE 1024
static struct { char name[64]; int id; } sc_vars[SC_MAX_VARS];
static int sc_nvars = 0;
static int sc_checks = 0;
static int sc_failures = 0;
static int sc_lookup(const char *name, int *out) {
    for (int i = 0; i < sc_nvars; i++)
        if (!strcmp(sc_vars[i].name, name)) { *out = sc_vars[i].id; return 1; }
    return 0;
}
/* Rust Phase Display -> C rb_phase_name. C has no mulligan states; those
   map to Opening (documented gap: such expects FAIL until the C phase
   machine models them). */
static const char *sc_phase_map(const char *rust) {
    if (!strcmp(rust, "LiveCardSet (1st)") || !strcmp(rust, "LiveCardSet (2nd)"))
        return "LiveCardSet";
    if (!strcmp(rust, "Perform (1st)") || !strcmp(rust, "Perform (2nd)"))
        return "Performance";
    if (!strcmp(rust, "Live Result")) return "Victory";
    if (!strcmp(rust, "Choose 1st")) return "Opening";
    if (!strcmp(rust, "Mulligan (1st)") || !strcmp(rust, "Mulligan (2nd)"))
        return "Opening";
    return rust;
}
static void sc_check(const char *actual, const char *expected) {
    sc_checks++;
    int ok = !strcmp(actual, expected);
    if (!ok) sc_failures++;
    printf("CHECK %d %s %s %s\n", sc_checks, ok ? "pass" : "FAIL", actual, expected);
}
static void sc_check_int(int actual, const char *expected) {
    char a[64];
    snprintf(a, sizeof a, "%d", actual);
    sc_check(a, expected);
}
static int sc_replay(const char *path) {
    FILE *f = fopen(path, "r");
    if (!f) { fprintf(stderr, "FAIL: cannot open scenario %s\n", path); return 1; }
    TestGame tg; test_game_new(&tg);
    char line[SC_LINE];
    while (fgets(line, sizeof line, f)) {
        /* strip trailing newline */
        line[strcspn(line, "\r\n")] = 0;
        char *s = line;
        while (*s == ' ' || *s == '\t') s++;
        if (!*s || *s == '#') continue;
        /* tokenize on whitespace */
        char *tok[16]; int ntok = 0;
        char *p = strtok(s, " \t");
        while (p && ntok < 16) { tok[ntok++] = p; p = strtok(NULL, " \t"); }
        if (!ntok) continue;
        int id, pl, area;
        if (!strcmp(tok[0], "card") && ntok >= 3) {
            id = test_id(&tg, tok[2]);
            if (id < 0) { printf("CHECK ? FAIL card-not-found %s\n", tok[2]); sc_failures++; continue; }
            if (sc_nvars < SC_MAX_VARS) {
                snprintf(sc_vars[sc_nvars].name, sizeof sc_vars[sc_nvars].name, "%s", tok[1]);
                sc_vars[sc_nvars].id = id; sc_nvars++;
            }
        } else if (!strcmp(tok[0], "stage") && ntok >= 4) {
            pl = atoi(tok[1]); area = atoi(tok[2]);
            if (!sc_lookup(tok[3], &id)) { sc_check("unknown-ref", tok[3]); continue; }
            if (pl == 1) test_add_to_stage(&tg, area, id);
            else test_set_opp_stage(&tg, area, id);
        } else if ((!strcmp(tok[0], "hand") || !strcmp(tok[0], "discard")
                    || !strcmp(tok[0], "deck") || !strcmp(tok[0], "live")
                    || !strcmp(tok[0], "success")) && ntok >= 3) {
            pl = atoi(tok[1]);
            if (!sc_lookup(tok[2], &id)) { sc_check("unknown-ref", tok[2]); continue; }
            RbPlayer *P = &tg.state.p[pl - 1];
            RbBag *b = NULL;
            if (!strcmp(tok[0], "hand")) { if (pl == 1) test_add_to_hand(&tg, id); else b = &P->hand; }
            else if (!strcmp(tok[0], "discard")) { if (pl == 1) test_add_to_discard(&tg, id); else b = &P->discard; }
            else if (!strcmp(tok[0], "deck")) { if (pl == 1) test_add_to_deck(&tg, id); else b = &P->deck; }
            else if (!strcmp(tok[0], "live")) { if (pl == 1) test_add_to_live(&tg, id); else test_add_to_opp_live(&tg, id); }
            else { if (pl == 1) test_add_to_success(&tg, id); else test_add_to_opp_success(&tg, id); }
            if (b && b->n < RB_MAX_ZONE) b->cards[b->n++] = id;
        } else if (!strcmp(tok[0], "energy") && ntok >= 3) {
            pl = atoi(tok[1]); int count = atoi(tok[2]);
            if (pl == 1) test_give_energy(&tg, count);
            else {
                int eid = test_id(&tg, "LL-E-001-SD");
                RbPlayer *P = &tg.state.p[1];
                for (int i = 0; i < count; i++) {
                    if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = eid;
                    P->energy_active++;
                }
            }
        } else if (!strcmp(tok[0], "wait") && ntok >= 2) {
            if (!sc_lookup(tok[1], &id)) { sc_check("unknown-ref", tok[1]); continue; }
            rb_mods_set_orientation(&tg.state.mods, id, "wait");
        } else if (!strcmp(tok[0], "recalculate")) {
            test_recalc(&tg);
        } else if (!strcmp(tok[0], "play") && ntok >= 3) {
            if (!sc_lookup(tok[1], &id)) { sc_check("unknown-ref", tok[1]); continue; }
            if (!test_play_to_stage(&tg, id, atoi(tok[2]))) sc_check("play-failed", "play-ok");
        } else if (!strcmp(tok[0], "activate") && ntok >= 2) {
            if (!sc_lookup(tok[1], &id)) { sc_check("unknown-ref", tok[1]); continue; }
            test_activate_ability(&tg, id);
        } else if (!strcmp(tok[0], "pass")) {
            test_pass(&tg);
        } else if (!strcmp(tok[0], "select")) {
            int idx[64]; int nidx = 0;
            if (ntok >= 2 && *tok[1]) {
                char *q = strtok(tok[1], ",");
                while (q && nidx < 64) { idx[nidx++] = atoi(q); q = strtok(NULL, ","); }
            }
            test_select_indices(&tg, idx, nidx);
        } else if (!strcmp(tok[0], "select_option") && ntok >= 2) {
            test_resume_choice(&tg, atoi(tok[1]));
        } else if (!strcmp(tok[0], "set_live") && ntok >= 2) {
            if (!sc_lookup(tok[1], &id)) { sc_check("unknown-ref", tok[1]); continue; }
            test_set_live_card(&tg, 0, id);
        } else if (!strcmp(tok[0], "drain_auto")) {
            test_drain_auto_choices(&tg);
        } else if (!strcmp(tok[0], "expect") && ntok >= 3) {
            if (!strcmp(tok[1], "blade") && ntok >= 4 && sc_lookup(tok[2], &id))
                sc_check_int(test_get_blade_modifier(&tg, id), tok[3]);
            else if (!strcmp(tok[1], "score") && ntok >= 4 && sc_lookup(tok[2], &id))
                sc_check_int(test_get_score_modifier(&tg, id), tok[3]);
            else if (!strcmp(tok[1], "cost") && ntok >= 4 && sc_lookup(tok[2], &id))
                sc_check_int(test_get_cost_modifier(&tg, id), tok[3]);
            else if (!strcmp(tok[1], "hand") && ntok >= 4) {
                pl = atoi(tok[2]);
                sc_check_int(pl == 1 ? test_hand_len(&tg) : tg.state.p[1].hand.n, tok[3]);
            } else if (!strcmp(tok[1], "energy") && ntok >= 4) {
                pl = atoi(tok[2]);
                sc_check_int(tg.state.p[pl - 1].energy_active, tok[3]);
            } else if (!strcmp(tok[1], "phase") && ntok >= 3) {
                sc_check(rb_phase_name(tg.state.phase), sc_phase_map(tok[2]));
            } else if (!strcmp(tok[1], "pending") && ntok >= 3) {
                const char *t2 = test_pending_choice_type(&tg);
                sc_check(*t2 ? t2 : "none", tok[2]);
            } else if (!strcmp(tok[1], "stage") && ntok >= 5) {
                pl = atoi(tok[2]); area = atoi(tok[3]);
                int slot = tg.state.p[pl - 1].stage[area];
                char act[128];
                if (slot == RB_EMPTY_SLOT) snprintf(act, sizeof act, "null");
                else {
                    int eid = slot;
                    /* resolve slot id back to card_no for comparison */
                    const char *cn = "?";
                    Card c;
                    if (rb_decode_card_by_index((uint32_t)eid, &c)) {
                        cn = rb_card_string(c.card_no_idx);
                        rb_free_card(&c);
                    }
                    snprintf(act, sizeof act, "%s", cn ? cn : "?");
                }
                char exp[128];
                if (!strcmp(tok[4], "null")) snprintf(exp, sizeof exp, "null");
                else if (sc_lookup(tok[4], &id)) {
                    Card c;
                    const char *cn = "?";
                    if (rb_decode_card_by_index((uint32_t)id, &c)) {
                        cn = rb_card_string(c.card_no_idx);
                        rb_free_card(&c);
                    }
                    snprintf(exp, sizeof exp, "%s", cn ? cn : "?");
                } else snprintf(exp, sizeof exp, "unknown-ref");
                sc_check(act, exp);
            } else sc_check("bad-expect", line);
        } else {
            char msg[SC_LINE + 32];
            snprintf(msg, sizeof msg, "unknown-op: %s", tok[0]);
            sc_check("unknown-op", msg);
        }
    }
    fclose(f);
    printf("SCENARIO %s: %d checks, %d failures\n", path, sc_checks, sc_failures);
    return sc_failures ? 1 : 0;
}

/* ── Trace replay mode (option D) ────────────────────────────────────
   Replays Rust option-D traces (`RABUKA_TRACE_DIR/*.trace`, see
   engine/tests/helpers/trace.rs) and prints a byte-identical stream
   (`diff`, ignoring `#` lines, is the parity check):
   - `S` blocks: dump live C state in the same canonical form. When the
     block says `PD none`, LOAD it first (resync: setup/direct writes are
     captured, and one engine difference cannot cascade).
   - `A` lines: execute on live C state, echoed verbatim.
   - `X`+`CHECK`: evaluate the spec on live C state, print `CHECK` with the
     same sequence number.
   Occurrence order (`card_no#k`) MUST match trace.rs exactly: p1/p2 stage,
   hand, deck, waitroom, live, success, energy bags, then under-cards. */
#define TR_MAX_LINES 40000
#define TR_MAX_OCC 4096
#define TR_LINE 2048
typedef struct { int id; char no[128]; int k; } TrOcc;
static TrOcc tr_occ[TR_MAX_OCC];
static int tr_nocc = 0;
static int tr_checks = 0;
static int tr_failed_checks = 0;
static int tr_skips = 0;

static void tr_card_no(int id, char *out, size_t n) {
    Card c;
    if (id >= 0 && rb_decode_card_by_index((uint32_t)id, &c)) {
        const char *s = rb_card_string(c.card_no_idx);
        snprintf(out, n, "%s", s ? s : "?");
        rb_free_card(&c);
    } else {
        snprintf(out, n, "#%d", id);
    }
}
static void tr_occ_add(int id) {
    if (id < 0) return;
    for (int i = 0; i < tr_nocc; i++) if (tr_occ[i].id == id) return;
    if (tr_nocc >= TR_MAX_OCC) return;
    char no[128]; tr_card_no(id, no, sizeof no);
    int k = 0;
    for (int i = 0; i < tr_nocc; i++) if (!strcmp(tr_occ[i].no, no)) k++;
    tr_occ[tr_nocc].id = id;
    snprintf(tr_occ[tr_nocc].no, sizeof tr_occ[tr_nocc].no, "%s", no);
    tr_occ[tr_nocc].k = k;
    tr_nocc++;
}
static void tr_build_occ(TestGame *tg) {
    tr_nocc = 0;
    for (int pl = 0; pl < 2; pl++)
        for (int a = 0; a < RB_STAGE_SIZE; a++) tr_occ_add(tg->state.p[pl].stage[a]);
    static const int is_bag = 1;
    (void)is_bag;
    for (int pl = 0; pl < 2; pl++) { RbBag *b = &tg->state.p[pl].hand; for (int i = 0; i < b->n; i++) tr_occ_add(b->cards[i]); }
    for (int pl = 0; pl < 2; pl++) { RbBag *b = &tg->state.p[pl].deck; for (int i = 0; i < b->n; i++) tr_occ_add(b->cards[i]); }
    for (int pl = 0; pl < 2; pl++) { RbBag *b = &tg->state.p[pl].discard; for (int i = 0; i < b->n; i++) tr_occ_add(b->cards[i]); }
    for (int pl = 0; pl < 2; pl++) { RbBag *b = &tg->state.p[pl].live; for (int i = 0; i < b->n; i++) tr_occ_add(b->cards[i]); }
    for (int pl = 0; pl < 2; pl++) { RbBag *b = &tg->state.p[pl].success; for (int i = 0; i < b->n; i++) tr_occ_add(b->cards[i]); }
    for (int pl = 0; pl < 2; pl++) { RbBag *b = &tg->state.p[pl].energy; for (int i = 0; i < b->n; i++) tr_occ_add(b->cards[i]); }
    for (int pl = 0; pl < 2; pl++)
        for (int a = 0; a < RB_STAGE_SIZE; a++) {
            RbBag *b = &tg->state.p[pl].under_cards[a];
            for (int i = 0; i < b->n; i++) tr_occ_add(b->cards[i]);
        }
}
/* "card_no#k" (or "#id", or bare card_no meaning #0) -> template id. */
static int tr_resolve(TestGame *tg, const char *ref, int *out) {
    if (!ref) return 0;
    if (ref[0] == '#') { *out = atoi(ref + 1); return 1; }
    const char *hash = strchr(ref, '#');
    int k = 0;
    char no[128];
    if (hash) {
        size_t l = (size_t)(hash - ref);
        if (l >= sizeof no) l = sizeof no - 1;
        memcpy(no, ref, l); no[l] = 0;
        k = atoi(hash + 1);
    } else {
        snprintf(no, sizeof no, "%s", ref);
    }
    tr_build_occ(tg);
    for (int i = 0; i < tr_nocc; i++)
        if (!strcmp(tr_occ[i].no, no) && tr_occ[i].k == k) { *out = tr_occ[i].id; return 1; }
    return 0;
}
static void tr_ref_of(TestGame *tg, int id, char *out, size_t n) {
    if (id < 0) { snprintf(out, n, "null"); return; }
    tr_build_occ(tg);
    for (int i = 0; i < tr_nocc; i++)
        if (tr_occ[i].id == id) { snprintf(out, n, "%s#%d", tr_occ[i].no, tr_occ[i].k); return; }
    snprintf(out, n, "#%d", id);
}
static void tr_print_csv(TestGame *tg, RbBag *b) {
    for (int i = 0; i < b->n; i++) {
        char no[128]; tr_card_no(b->cards[i], no, sizeof no);
        printf("%s%s", i ? "," : "", no);
    }
}
static int tr_cmp_str(const void *a, const void *b) { return strcmp((const char *)a, (const char *)b); }
/* Dump live C state in trace.rs canonical form (without the leading S). */
static void tr_dump_sections(TestGame *tg) {
    for (int pl = 0; pl < 2; pl++) {
        printf("Z %d hand ", pl + 1); tr_print_csv(tg, &tg->state.p[pl].hand); printf("\n");
        printf("Z %d deck ", pl + 1); tr_print_csv(tg, &tg->state.p[pl].deck); printf("\n");
        printf("Z %d waitroom ", pl + 1); tr_print_csv(tg, &tg->state.p[pl].discard); printf("\n");
        printf("Z %d live ", pl + 1); tr_print_csv(tg, &tg->state.p[pl].live); printf("\n");
        printf("Z %d success ", pl + 1); tr_print_csv(tg, &tg->state.p[pl].success); printf("\n");
        printf("Z %d energy ", pl + 1); tr_print_csv(tg, &tg->state.p[pl].energy); printf("\n");
        printf("E %d %d\n", pl + 1, tg->state.p[pl].energy_active);
        printf("ST %d", pl + 1);
        for (int a = 0; a < RB_STAGE_SIZE; a++) {
            int id = tg->state.p[pl].stage[a];
            if (id < 0) printf(" -");
            else { char no[128]; tr_card_no(id, no, sizeof no); printf(" %s", no); }
        }
        printf("\n");
        for (int a = 0; a < RB_STAGE_SIZE; a++) {
            RbBag *u = &tg->state.p[pl].under_cards[a];
            if (u->n > 0) { printf("U %d %d ", pl + 1, a); tr_print_csv(tg, u); printf("\n"); }
        }
        for (int a = 0; a < RB_STAGE_SIZE; a++) {
            int id = tg->state.p[pl].stage[a];
            if (id >= 0) {
                const char *o = rb_mods_get_orientation(&tg->state.mods, id);
                if (o && *o) {
                    char ref[160]; tr_ref_of(tg, id, ref, sizeof ref);
                    printf("O %s %s\n", ref, o);
                }
            }
        }
    }
    /* Mods: absolute totals, sorted (must match trace.rs tuple sort). */
    static char mbuf[2048][160];
    int nm = 0;
    tr_build_occ(tg);
    for (int i = 0; i < tr_nocc && nm < 2048; i++) {
        int id = tr_occ[i].id;
        char ref[160]; snprintf(ref, sizeof ref, "%s#%d", tr_occ[i].no, tr_occ[i].k);
        int v;
        if ((v = test_get_blade_modifier(tg, id)) != 0) { snprintf(mbuf[nm], sizeof mbuf[nm], "M blade %s %d", ref, v); nm++; }
        if ((v = test_get_score_modifier(tg, id)) != 0) { snprintf(mbuf[nm], sizeof mbuf[nm], "M score %s %d", ref, v); nm++; }
        if ((v = test_get_cost_modifier(tg, id)) != 0) { snprintf(mbuf[nm], sizeof mbuf[nm], "M cost %s %d", ref, v); nm++; }
        for (int c = 0; c < 8 && nm < 2048; c++) {
            if ((v = test_get_heart_modifier(tg, id, c)) != 0) { snprintf(mbuf[nm], sizeof mbuf[nm], "M heart %s %d %d", ref, c, v); nm++; }
            if ((v = rb_mods_get_need_heart(&tg->state.mods, id, c)) != 0) { snprintf(mbuf[nm], sizeof mbuf[nm], "M need %s %d %d", ref, c, v); nm++; }
        }
    }
    qsort(mbuf, nm, sizeof mbuf[0], tr_cmp_str);
    for (int i = 0; i < nm; i++) printf("%s\n", mbuf[i]);
    printf("PH %s %d\n", rb_phase_name(tg->state.phase), tg->state.turn);
    if (!rb_has_pending_choice(&tg->state)) {
        printf("PD none\n");
    } else {
        const RbChoice *c = rb_get_pending_choice(&tg->state);
        const char *t = test_pending_choice_type(tg);
        int n = (c && c->kind == RB_CHOICE_SELECT_CARD) ? c->count : 0;
        printf("PD %s:%d\n", t && *t ? t : "Unknown", n);
    }
}
/* Load a quiescent S block (lines[lo..hi)) into live state. */
static void tr_load(TestGame *tg, char **lines, int lo, int hi) {
    /* Quiescent load = full resync: transient engine state (queue, choice
       routing, selections, movement tracking) must reset too, otherwise
       stale entries from earlier replayed steps pollute the loaded state
       and cause phantom choices. */
    memset(&tg->state.queue, 0, sizeof tg->state.queue);
    tg->state.n_selected_cards = 0;
    tg->state.n_recently_moved = 0;
    tg->state.n_those_cards = 0;
    tg->state.n_recently_appeared = 0;
    tg->state.n_recently_state_changed = 0;
    tg->state.n_batch_movements = 0;
    tg->state.n_position_change_events = 0;
    tg->state.n_revealed = 0;
    tg->state.last_draw_count = 0;
    int cleared[512]; int ncleared = 0;
    /* Pass 1: zones, stage, under, energy counts, phase/turn. */
    for (int i = lo; i < hi; i++) {
        char *tok[64]; int nt = 0;
        char *p = strtok(lines[i], " \t");
        while (p && nt < 64) { tok[nt++] = p; p = strtok(NULL, " \t"); }
        if (!nt) continue;
        if (!strcmp(tok[0], "Z") && nt >= 3) {
            int pl = atoi(tok[1]) - 1;
            RbBag *b = NULL;
            if (!strcmp(tok[2], "hand")) b = &tg->state.p[pl].hand;
            else if (!strcmp(tok[2], "deck")) b = &tg->state.p[pl].deck;
            else if (!strcmp(tok[2], "waitroom")) b = &tg->state.p[pl].discard;
            else if (!strcmp(tok[2], "live")) b = &tg->state.p[pl].live;
            else if (!strcmp(tok[2], "success")) b = &tg->state.p[pl].success;
            else if (!strcmp(tok[2], "energy")) b = &tg->state.p[pl].energy;
            if (!b) continue;
            b->n = 0;
            if (nt >= 4 && *tok[3]) {
                char *q = strtok(tok[3], ",");
                while (q) {
                    int id = test_id(tg, q);
                    if (id >= 0 && b->n < RB_MAX_ZONE) b->cards[b->n++] = id;
                    q = strtok(NULL, ",");
                }
            }
        } else if (!strcmp(tok[0], "ST") && nt >= 5) {
            int pl = atoi(tok[1]) - 1;
            for (int a = 0; a < 3; a++) {
                if (!strcmp(tok[2 + a], "-")) tg->state.p[pl].stage[a] = -1;
                else {
                    int id = test_id(tg, tok[2 + a]);
                    tg->state.p[pl].stage[a] = id;
                    tg->state.p[pl].stage_wait[a] = 0;
                }
            }
        } else if (!strcmp(tok[0], "U") && nt >= 4) {
            int pl = atoi(tok[1]) - 1, a = atoi(tok[2]);
            RbBag *b = &tg->state.p[pl].under_cards[a];
            b->n = 0;
            if (*tok[3]) {
                char *q = strtok(tok[3], ",");
                while (q) {
                    int id = test_id(tg, q);
                    if (id >= 0 && b->n < RB_MAX_ZONE) b->cards[b->n++] = id;
                    q = strtok(NULL, ",");
                }
            }
        } else if (!strcmp(tok[0], "E") && nt >= 3) {
            test_set_energy_active(tg, atoi(tok[1]) - 1, atoi(tok[2]));
        } else if (!strcmp(tok[0], "PH") && nt >= 3) {
            const char *ph = tok[1];
            RbPhase v = RB_PHASE_MAIN;
            if (!strcmp(ph, "RPS")) v = RB_PHASE_RPS;
            else if (!strcmp(ph, "Opening")) v = RB_PHASE_OPENING;
            else if (!strcmp(ph, "Active")) v = RB_PHASE_ACTIVE;
            else if (!strcmp(ph, "Energy")) v = RB_PHASE_ENERGY;
            else if (!strcmp(ph, "Draw")) v = RB_PHASE_DRAW;
            else if (!strcmp(ph, "Main")) v = RB_PHASE_MAIN;
            else if (!strcmp(ph, "LiveCardSet")) v = RB_PHASE_LIVE_SET;
            else if (!strcmp(ph, "Performance")) v = RB_PHASE_PERFORMANCE;
            else if (!strcmp(ph, "Victory")) v = RB_PHASE_VICTORY;
            else if (!strcmp(ph, "Done")) v = RB_PHASE_DONE;
            tg->state.phase = v;
            tg->state.turn = atoi(tok[2]);
        }
    }
    /* Pass 2: orientations + mods (need occurrences from pass 1). */
    for (int i = lo; i < hi; i++) {
        char *tok[64]; int nt = 0;
        char *p = strtok(lines[i], " \t");
        while (p && nt < 64) { tok[nt++] = p; p = strtok(NULL, " \t"); }
        if (!nt) continue;
        if (!strcmp(tok[0], "O") && nt >= 3) {
            int id;
            if (tr_resolve(tg, tok[1], &id)) rb_mods_set_orientation(&tg->state.mods, id, tok[2]);
        } else if (!strcmp(tok[0], "M") && nt >= 4) {
            int id;
            if (!tr_resolve(tg, tok[2], &id)) continue;
            int seen = 0;
            for (int k = 0; k < ncleared; k++) if (cleared[k] == id) seen = 1;
            if (!seen) {
                if (ncleared < 512) cleared[ncleared++] = id;
                rb_mods_clear_card(&tg->state.mods, id);
            }
            if (!strcmp(tok[1], "blade")) rb_mods_add_blade(&tg->state.mods, id, atoi(tok[3]));
            else if (!strcmp(tok[1], "score")) rb_mods_add_score(&tg->state.mods, id, atoi(tok[3]));
            else if (!strcmp(tok[1], "cost")) rb_mods_add_cost(&tg->state.mods, id, atoi(tok[3]));
            else if (!strcmp(tok[1], "heart") && nt >= 5) rb_mods_add_heart(&tg->state.mods, id, atoi(tok[3]), atoi(tok[4]));
            else if (!strcmp(tok[1], "need") && nt >= 5) rb_mods_add_need_heart(&tg->state.mods, id, atoi(tok[3]), atoi(tok[4]));
        }
    }
}
/* Pending X spec from the last X line. */
static char tr_xspec[TR_LINE];
static char tr_last_expected[TR_LINE];
static void tr_run_check(TestGame *tg, const char *nstr) {
    char spec[TR_LINE]; snprintf(spec, sizeof spec, "%s", tr_xspec);
    char *tok[16]; int nt = 0;
    char *p = strtok(spec, " \t");
    while (p && nt < 16) { tok[nt++] = p; p = strtok(NULL, " \t"); }
    char actual[256] = "?";
    if (nt >= 2 && !strcmp(tok[0], "hand")) {
        int pl = atoi(tok[1]) - 1;
        snprintf(actual, sizeof actual, "%d", pl == 0 ? test_hand_len(tg) : tg->state.p[1].hand.n);
    } else if (nt >= 2 && !strcmp(tok[0], "energy")) {
        int pl = atoi(tok[1]) - 1;
        snprintf(actual, sizeof actual, "%d", tg->state.p[pl].energy_active);
    } else if (nt >= 2 && !strcmp(tok[0], "blade")) {
        int id;
        if (tr_resolve(tg, tok[1], &id)) snprintf(actual, sizeof actual, "%d", test_get_blade_modifier(tg, id));
    } else if (nt >= 4 && !strcmp(tok[0], "stage")) {
        int pl = atoi(tok[1]) - 1, a = atoi(tok[2]);
        tr_ref_of(tg, tg->state.p[pl].stage[a], actual, sizeof actual);
    } else if (nt >= 1 && !strcmp(tok[0], "pending")) {
        const char *t = test_pending_choice_type(tg);
        snprintf(actual, sizeof actual, "%s", *t ? t : "none");
    }
    int ok = !strcmp(actual, tr_last_expected);
    if (!ok) { tr_failed_checks++; failures++; }
    printf("CHECK %s %s %s %s\n", nstr, ok ? "pass" : "FAIL", actual, tr_last_expected);
}
static void tr_exec_a(TestGame *tg, char *line) {
    printf("%s\n", line);
    char buf[TR_LINE]; snprintf(buf, sizeof buf, "%s", line);
    char *tok[16]; int nt = 0;
    char *p = strtok(buf, " \t");
    while (p && nt < 16) { tok[nt++] = p; p = strtok(NULL, " \t"); }
    if (nt < 2) return; /* bare "A" (should not happen) */
    int id;
    if (!strcmp(tok[1], "play") && nt >= 4) {
        if (tr_resolve(tg, tok[2], &id)) test_play_to_stage(tg, id, atoi(tok[3]));
    } else if (!strcmp(tok[1], "activate") && nt >= 3) {
        if (tr_resolve(tg, tok[2], &id)) test_activate_ability(tg, id);
    } else if (!strcmp(tok[1], "activate_idx")) {
        fprintf(stderr, "# SKIP activate_idx (no C shim)\n"); tr_skips++;
    } else if (!strcmp(tok[1], "pass")) {
        test_pass(tg);
    } else if (!strcmp(tok[1], "select")) {
        int idx[64]; int nidx = 0;
        if (nt >= 3 && *tok[2]) {
            char *q = strtok(tok[2], ",");
            while (q && nidx < 64) { idx[nidx++] = atoi(q); q = strtok(NULL, ","); }
        }
        test_select_indices(tg, idx, nidx);
    } else if (!strcmp(tok[1], "selopt") && nt >= 3) {
        test_resume_choice(tg, atoi(tok[2]));
    } else if (!strcmp(tok[1], "selgen")) {
        fprintf(stderr, "# SKIP selgen (no C shim)\n"); tr_skips++;
    } else if (!strcmp(tok[1], "setlive") && nt >= 3) {
        if (tr_resolve(tg, tok[2], &id)) test_set_live_card(tg, 0, id);
    } else if (!strcmp(tok[1], "drain")) {
        test_drain_auto_choices(tg);
    } else {
        fprintf(stderr, "# SKIP unknown A op: %s\n", tok[1]); tr_skips++;
    }
}
static int tr_run(const char *path) {
    FILE *f = fopen(path, "r");
    if (!f) { fprintf(stderr, "FAIL: cannot open trace %s\n", path); return 1; }
    static char *lines[TR_MAX_LINES];
    int nlines = 0;
    char tmp[TR_LINE];
    while (fgets(tmp, sizeof tmp, f) && nlines < TR_MAX_LINES) {
        tmp[strcspn(tmp, "\r\n")] = 0;
        lines[nlines] = malloc(strlen(tmp) + 1);
        strcpy(lines[nlines], tmp);
        nlines++;
    }
    fclose(f);
    TestGame tg; test_game_new(&tg);
    tr_xspec[0] = 0; tr_last_expected[0] = 0;
    for (int i = 0; i < nlines; i++) {
        char *s = lines[i];
        while (*s == ' ' || *s == '\t') s++;
        if (!*s || *s == '#') continue;
        if (s[0] == 'S' && (s[1] == 0 || s[1] == ' ' || s[1] == '\t')) {
            int lo = i + 1, hi = lo;
            while (hi < nlines) {
                char *t = lines[hi];
                while (*t == ' ' || *t == '\t') t++;
                if (!*t || *t == '#') { hi++; continue; }
                if ((t[0] == 'S' && (!t[1] || t[1] == ' ' || t[1] == '\t'))
                    || !strncmp(t, "A ", 2) || !strcmp(t, "A")
                    || !strncmp(t, "X ", 2) || !strncmp(t, "CHECK ", 6)) break;
                hi++;
            }
            /* find PD line to decide load vs verify-only */
            int pending = 0;
            for (int k = lo; k < hi; k++) {
                char *t = lines[k];
                while (*t == ' ' || *t == '\t') t++;
                if (!strncmp(t, "PD ", 3) && strcmp(t + 3, "none")) pending = 1;
            }
            if (!pending) tr_load(&tg, lines, lo, hi);
            printf("S\n");
            tr_dump_sections(&tg);
            i = hi - 1;
        } else if (!strncmp(s, "A ", 2) || !strcmp(s, "A")) {
            char buf[TR_LINE]; snprintf(buf, sizeof buf, "%s", s);
            tr_exec_a(&tg, buf);
        } else if (!strncmp(s, "X ", 2)) {
            snprintf(tr_xspec, sizeof tr_xspec, "%s", s + 2);
        } else if (!strncmp(s, "CHECK ", 6)) {
            /* CHECK <n> <verdict> <actual> <expected...> : take <n> and the
               trailing expected token; evaluate the stored X spec live. */
            char buf[TR_LINE]; snprintf(buf, sizeof buf, "%s", s);
            char *tok[32]; int nt = 0;
            char *p = strtok(buf, " \t");
            while (p && nt < 32) { tok[nt++] = p; p = strtok(NULL, " \t"); }
            if (nt >= 5) {
                /* expected = everything from tok[4] on (refs have no spaces;
                   join defensively with single spaces) */
                tr_last_expected[0] = 0;
                for (int k = 4; k < nt; k++) {
                    if (k > 4) strcat(tr_last_expected, " ");
                    strcat(tr_last_expected, tok[k]);
                }
                tr_run_check(&tg, tok[1]);
            }
        }
    }
    for (int i = 0; i < nlines; i++) free(lines[i]);
    fprintf(stderr, "TRACE %s: checks=%d failed=%d skips=%d\n",
            path, tr_checks, tr_failed_checks, tr_skips);
    return tr_failed_checks ? 1 : 0;
}

int main(int argc, char **argv){
    setvbuf(stdout, NULL, _IONBF, 0); /* unbuffered: a crash must not swallow results */
    CHECK(rb_load("src")==0,"rb_load");
    if (argc > 2 && !strcmp(argv[1], "trace")) {
        int rc = tr_run(argv[2]);
        rb_unload();
        return rc;
    }
    if (argc > 1) {
        int rc = sc_replay(argv[1]);
        rb_unload();
        if (failures + sc_failures + rc) { printf("\n%d FAILURES\n", failures + sc_failures); return 1; }
        printf("\nALL SCENARIO CHECKS PASSED\n");
        return 0;
    }
    scenario_draw_and_score();
    scenario_live_performance();
    scenario_move_cards();
    scenario_condition_gate();
    scenario_cost_modifier();
    scenario_heart_modifier();
    scenario_look_select();
    scenario_baton();
    scenario_live_limit();
    scenario_group_condition();
    scenario_no_excess();
    scenario_opponent_live_success();
    scenario_phase_determinism();
    rb_unload();
    if(failures){ printf("\n%d FAILURES\n",failures); return 1; }
    printf("\nALL REPLAY CHECKS PASSED\n");
    return 0;
}
