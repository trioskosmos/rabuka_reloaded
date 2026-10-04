/* Cluster E corpus census. Decodes all 985 abilities with the ENGINE'S OWN
 * decoder (rb_decode_card_ability) and tallies the keys/actions E1..E17 depend
 * on. Nothing here re-implements the bytecode reader. */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "rabuka.h"

#define MAXK 4096
static char keystore[MAXK][64];
static long n_pairs, n_eff, n_abilities, n_constant_abilities;
static long n_e1;
static long has[MAXK];
static int n_keys;

/* keys[] must own its strings: the decoded Ability is freed right after the
 * walk, so a stored char* would dangle. */
static void bump(const char *k, long n){
    const char *kk = k ? k : "?";
    for (int i=0;i<n_keys;i++) if(!strcmp(keystore[i],kk)){ has[i]+=n; return; }
    if(n_keys>=MAXK) return;
    size_t l = strlen(kk); if(l>63) l=63;
    memcpy(keystore[n_keys],kk,l); keystore[n_keys][l]='\0';
    has[n_keys]=n; n_keys++;
}
static long cnt(const char *k){
    for (int i=0;i<n_keys;i++) if(!strcmp(keystore[i],k)) return has[i];
    return 0;
}

/* actions seen on abilities whose trigger list contains the constant marker */
#define MAXC 256
static char cact_keys[MAXC][64];
static long cact_has[MAXC];
static int n_cact;
static int in_constant_ability = 0;
static int n_actions_counted = 0;

static void bump_cact(const char *k){
    const char *kk = k?k:"?";
    for(int i=0;i<n_cact;i++) if(!strcmp(cact_keys[i],kk)){ cact_has[i]++; return; }
    if(n_cact>=MAXC) return;
    size_t l=strlen(kk); if(l>63) l=63;
    memcpy(cact_keys[n_cact],kk,l); cact_keys[n_cact][l]='\0';
    cact_has[n_cact]=1; n_cact++;
}

static char cur_card[64];
static char cur_card_no[64];
static int  cur_cid;
static char cur_ability[400];

/* action dump for the three E1 action types on a constant ability */
static void note_action(const AbilityEffect *e, const char *cid_no, const char *trig, int abidx, int cid);

static void walk_condition(const Condition *c, int depth);
static void walk_effect(const AbilityEffect *e, int depth);

static void walk_condition(const Condition *c, int depth){
    if(!c || depth > 8) return;
    for(uint32_t i=0;i<c->n_fields;i++){
        const char *k = c->fields[i].key ? c->fields[i].key : "?";
        bump(k,1);
        if(!strcmp(k,"trigger_event"))
            printf("CENSUS-E cond trigger_event = %s\n", c->fields[i].v.s?c->fields[i].v.s:"");
    }
}

static void walk_effect(const AbilityEffect *e, int depth){
    if(!e || depth > 12) return;
    n_eff++;
    if(e->action){
        bump(e->action,1);
        if(in_constant_ability){ bump_cact(e->action); n_actions_counted++; }
        note_action(e, cur_card, cur_ability, 0, 0);
        if(!strcmp(e->action,"count_as_multiple") || !strcmp(e->action,"replacement_appear_wait")){
            printf("CENSUS-E %-24s cardid=%d card_no=%s self_target_field=%s card_type_field=%s n_child=%d n_extra=%d has_cond=%d\n",
                   e->action, cur_cid, cur_card_no, e->self_target_field, e->card_type_field, e->n_child, e->n_extra, e->has_condition);
            for(int i=0;i<e->n_extra;i++)
                printf("        extra %s = %s\n", e->extra_k[i], e->extra_v[i]);
            if(e->condition){
                printf("        condition variant=%u n_fields=%u\n", e->condition->variant, e->condition->n_fields);
                for(uint32_t q=0;q<e->condition->n_fields;q++)
                    printf("          cond %s = %s (tag=%d)\n", e->condition->fields[q].key?e->condition->fields[q].key:"?",
                           e->condition->fields[q].v.s?e->condition->fields[q].v.s:"", e->condition->fields[q].v.tag);
                walk_condition(e->condition,1);
            }
        }
    }
    for(int i=0;i<e->n_extra;i++) bump(e->extra_k[i]?e->extra_k[i]:"?",1);
    for(int i=0;i<e->n_child;i++)   walk_effect(e->child[i],depth+1);
    for(int i=0;i<e->n_options;i++) walk_effect(e->options[i],depth+1);
    walk_effect(e->primary_effect,depth+1);
    walk_effect(e->alternative_effect,depth+1);
    walk_effect(e->look_action,depth+1);
    walk_effect(e->select_action,depth+1);
    walk_effect(e->followup_action,depth+1);
    walk_effect(e->optional_action,depth+1);
    walk_effect(e->conditional_action,depth+1);
    walk_effect(e->gained_effect,depth+1);
    walk_effect(e->resource_on_select,depth+1);
    walk_effect(e->opponent_action,depth+1);
}

static void note_action(const AbilityEffect *e, const char *cid_no, const char *trig, int abidx, int cid){
    (void)abidx; (void)cid;
    if(!in_constant_ability) return;
    if(!e->action) return;
    if(strcmp(e->action,"gain_resource") && strcmp(e->action,"modify_score") &&
       strcmp(e->action,"modify_required_hearts")) return;
    printf("E1CARD %-22s %-20s action=%-24s text=%.40s", cid_no, trig, e->action, e->text?e->text:"");
    for(int i=0;i<e->n_extra;i++){
        const char *k = e->extra_k[i]?e->extra_k[i]:"";
        if(!strcmp(k,"resource")||!strcmp(k,"target")||!strcmp(k,"self_target")||
           !strcmp(k,"value")||!strcmp(k,"operation")||!strcmp(k,"heart_colors")||
           !strcmp(k,"position")||!strcmp(k,"group_names")||!strcmp(k,"non_stackable")||
           !strcmp(k,"target_name")||!strcmp(k,"card_type")||!strcmp(k,"count"))
            printf(" %s=%s", k, e->extra_v[i]?e->extra_v[i]:"");
    }
    printf(" has_cond=%d n_child=%d\n", e->has_condition, e->n_child);
}

/* E1 census: where does `non_stackable` actually sit? evaluate_success_zone_
 * modifiers (modifiers.rs:2221/2280-2287) dedupes on "{action}:{text}" ONLY for
 * effects whose non_stackable flag is set, across BOTH live zones of BOTH seats.
 * So the tally that matters is: non_stackable on an effect that
 * gain_resource/modify_score/modify_required_hearts/sequential, reached from a
 * card that can sit in a live or success live-card zone. */
static void report_nonstack(const AbilityEffect *e, const char *who){
    if(!e) return;
    const char *ns = NULL;
    for(int i=0;i<e->n_extra;i++)
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"non_stackable")) ns = e->extra_v[i];
    if(ns) printf("CENSUS-NS %-16s action=%-24s value=%-6s text=%.50s\n",
                  who, e->action?e->action:"-", ns, e->text?e->text:"");
    for(int i=0;i<e->n_child;i++) report_nonstack(e->child[i],who);
    report_nonstack(e->primary_effect,who);
    report_nonstack(e->alternative_effect,who);
    report_nonstack(e->followup_action,who);
    report_nonstack(e->optional_action,who);
    report_nonstack(e->conditional_action,who);
    report_nonstack(e->gained_effect,who);
}

/* E1 reachability: Rust's recalculate_constants main loop scans STAGE ONLY
 * (collect_reusable_constant_stage_effect_ids, modifiers.rs:453-455 chains
 * player1.stage.stage + player2.stage.stage); evaluate_success_zone_constant_
 * modifiers (modifiers.rs:2232-2237) then scans the four LIVE/SUCCESS bags.
 * So E1's delta is exactly: constant effects on a card that can sit in a live or
 * success live-card zone -- i.e. a MEMBER card -- with one of E1's four actions. */
static int card_is_member(int cid){
    return rb_card_is_member(cid);
}

static void e1_walk(const AbilityEffect *e, const char *who, int member){
    if(!e) return;
    if(member && e->action &&
       (!strcmp(e->action,"gain_resource")||!strcmp(e->action,"modify_score")||
        !strcmp(e->action,"modify_required_hearts")||!strcmp(e->action,"sequential"))){
        printf("CENSUS-E1 %-16s action=%-24s text=%.44s", who, e->action, e->text?e->text:"");
        for(int i=0;i<e->n_extra;i++){
            const char *k=e->extra_k[i]?e->extra_k[i]:"";
            if(!strcmp(k,"resource")||!strcmp(k,"target")||!strcmp(k,"self_target")||
               !strcmp(k,"value")||!strcmp(k,"operation")||!strcmp(k,"heart_colors")||
               !strcmp(k,"position")||!strcmp(k,"per_unit")||!strcmp(k,"heart_type")||
               !strcmp(k,"non_stackable")||!strcmp(k,"count")||!strcmp(k,"dynamic_count"))
                printf(" %s=%s",k,e->extra_v[i]?e->extra_v[i]:"");
        }
        printf("\n");
        n_e1++;
    }
    for(int i=0;i<e->n_child;i++) e1_walk(e->child[i],who,member);
    e1_walk(e->primary_effect,who,member);
    e1_walk(e->alternative_effect,who,member);
    e1_walk(e->followup_action,who,member);
    e1_walk(e->optional_action,who,member);
    e1_walk(e->conditional_action,who,member);
}

int main(int argc, char **argv){
    const char *dir = argc>1?argv[1]:"src";
    if(rb_load(dir)!=0){ fprintf(stderr,"rb_load failed\n"); return 2; }
    int n_cards = rb_num_cards();
    printf("CARDS %d\n", n_cards);
    for(int cid=0; cid<n_cards; cid++){
        int n = rb_card_num_abilities((uint32_t)cid);
        for(int ai=0; ai<n; ai++){
            Ability ab; memset(&ab,0,sizeof(ab));
            if(!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            n_abilities++; n_pairs++;
            in_constant_ability = (ab.triggers && strstr(ab.triggers,"\xe5\xb8\xb8\xe6\x99\x82")) ? 1 : 0;
            if(in_constant_ability){
                n_constant_abilities++;
                snprintf(cur_card,sizeof cur_card,"cardid=%d",cid);
                cur_cid = cid;
                { Card cc;
                  if (rb_decode_card_by_index((uint32_t)cid,&cc)) {
                      const char *no = rb_card_string(cc.card_no_idx);
                      snprintf(cur_card_no,sizeof cur_card_no,"%s", no?no:"-");
                      rb_free_card(&cc);
                  } else snprintf(cur_card_no,sizeof cur_card_no,"-");
                }
                snprintf(cur_ability,sizeof cur_ability,"ab#%d trig=%.80s",ai,ab.triggers?ab.triggers:"");
            }
            if(ab.effect) walk_effect(ab.effect,0);
            if(ab.cost) walk_effect(ab.cost,0);
            if(ab.effect) report_nonstack(ab.effect, cur_card);
            if(ab.effect && ab.triggers && strstr(ab.triggers,"\xe5\xb8\xb8\xe6\x99\x82") &&
               card_is_member(cid))
                e1_walk(ab.effect, cur_card, 1);
            in_constant_ability = 0;
            rb_free_ability(&ab);
        }
    }
    printf("ABILITIES %ld  CONSTANT_ABILITIES %ld  EFFECT_NODES %ld\n", n_abilities, n_constant_abilities, n_eff);
    printf("--- CONSTANT-TRIGGER ACTIONS ---\n");
    for(int i=0;i<n_cact;i++) printf("CACT %-30s %ld\n", cact_keys[i], cact_has[i]);
    const char *want[] = {"count_as_multiple","counted_by","count_as",
        "replacement_appear_wait","heart_total_limit","heart_total_operator","blade_limit",
        "card_type","position","gain_resource","modify_score","modify_required_hearts",
        "sequential","group_names","non_stackable","resource","target","self_target",
        "operation","value","count","text","heart_colors","heart_total_limit",NULL};
    for(int i=0;want[i];i++) printf("TALLY %-26s %ld\n", want[i], cnt(want[i]));
    printf("--- KEYS (alphabetical by insertion) ---\n");
    for(int i=0;i<n_keys;i++) printf("KEY %-34s %ld\n", keystore[i], has[i]);
    /* PROBE: locate fixtures by predicate, not by hard-coded card number, so a
     * corpus regeneration cannot silently turn a CHECK vacuous. */
    for(int id=0; id<2639; id++){
        Card c;
        if(!rb_decode_card_by_index((uint32_t)id,&c)) continue;
        int base=0; for(int i=0;i<c.num_base && i<c.n_hearts;i++) base+=c.heart_count[i];
        const char *no = rb_card_string(c.card_no_idx);
        const char *grp = rb_card_string(c.group_idx);
        int grp_lw_done=0, big_done=0, grp_aq_done=0;
        if(rb_card_matches_group_str(id,"lily white") && !grp_lw_done){
            printf("PROBE lilywhite id=%d no=%s member=%d\n", id, no?no:"-", rb_card_is_member(id));
            grp_lw_done=1;
        }
        if(rb_card_matches_group_str(id,"Aqours") && !grp_aq_done){
            printf("PROBE aqours id=%d no=%s member=%d\n", id, no?no:"-", rb_card_is_member(id));
            grp_aq_done=1;
        }
        if(rb_card_is_member(id) && base<=4 && !grp_lw_done){
            /* prefer a card with a NON-empty group so the group filters are not vacuous */
            if(grp && *grp){ printf("PROBE lw_or_grouped id=%d no=%s base=%d grp=%s\n", id,no,base,grp); grp_lw_done=1; }
        }
        if(rb_card_is_member(id) && base>4 && !big_done){
            printf("PROBE member_gt4 id=%d no=%s base=%d grp=%s\n", id,no,base,grp?grp:"-"); big_done=1;
        }
        if(id==469||id==413||id==414)
            printf("PROBE target id=%d no=%s grp=%s base=%d member=%d\n", id, no?no:"-",
                   grp?grp:"-", base, rb_card_is_member(id));
        rb_free_card(&c);
    }
    rb_unload();
    return 0;
}