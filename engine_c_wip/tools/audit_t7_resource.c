/* Cluster T7  Eper-branch reachability census for the RESOURCE family
 * (Rust: engine/src/ability/effects/resource.rs, energy.rs, deck.rs).
 *
 * Walks the shipped bytecode, collects every key/value pair that appears on a
 * `gain_resource` / `pay_energy` / `place_energy_under_member` / `draw_card` /
 * `draw_until_count` effect, and evaluates the exact guard expressions Rust
 * uses, so every branch in the Rust handler gets a measured 0-of-985 or a
 * measured node/ability count. Nothing is inferred from reading the code.
 */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "rabuka.h"

#define MAXKEYS 2048
typedef struct { char k[80]; long nodes; long abilities; long last_slot; } Row;
static Row rows[MAXKEYS]; static int n_rows;
static long n_eff_nodes;
static int  cur_slot;

static const unsigned char *P, *E;
static int ru8(unsigned char *o){ if (P+1>E) return 0; *o=*P++; return 1; }
static int ru16(unsigned short *o){ if (P+2>E) return 0; *o=(unsigned short)(P[0]|(P[1]<<8)); P+=2; return 1; }
static int ru32(uint32_t *o){ if (P+4>E) return 0; *o=(uint32_t)P[0]|((uint32_t)P[1]<<8)|((uint32_t)P[2]<<16)|((uint32_t)P[3]<<24); P+=4; return 1; }
static int rlen(uint32_t *o){ unsigned char b; if(!ru8(&b)) return 0; if(b<0xFE){*o=b;return 1;} unsigned short v; if(!ru16(&v)) return 0; *o=v; return 1; }
static int ridx(uint32_t *o){ unsigned char b; if(!ru8(&b)) return 0; if(b==0xFE){unsigned short v; if(!ru16(&v)) return 0; *o=v; return 1;} *o=b; return 1; }
static int rint(int64_t *o){
    unsigned char b; if(!ru8(&b)) return 0;
    if(b<=0xFD){*o=b;return 1;}
    if(b==0xFE){unsigned short v; if(!ru16(&v)) return 0; *o=v;return 1;}
    if(b==0xFF){uint32_t v; if(!ru32(&v)) return 0; *o=(int32_t)v;return 1;}
    if(P+8>E) return 0;
    uint32_t lo,hi; if(!ru32(&lo)||!ru32(&hi)) return 0;
    *o=(int64_t)(((uint64_t)hi<<32)|lo); return 1;
}
static int skipv(uint8_t tag);
static int skip_one(void){ uint8_t t; if(!ru8(&t)) return 0; return skipv(t); }
static int skipv(uint8_t tag){
    uint32_t len,i,k; unsigned char b;
    switch(tag){
    case 0x00: case 0x01: case 0x02: return 1;
    case 0x03: { int64_t v; return rint(&v); }
    case 0x04: if(P+8>E) return 0; P+=8; return 1;
    case 0x06: return ridx(&k);
    case 0x07: if(!rlen(&len)) return 0; for(i=0;i<len;i++) if(!skip_one()) return 0; return 1;
    case 0x08: case 0x09:
        if(tag==0x09 && !ru8(&b)) return 0;
        if(!rlen(&len)) return 0;
        for(i=0;i<len;i++){ if(!ridx(&k)) return 0; if(!skip_one()) return 0; }
        return 1;
    default: return 0;
    }
}

/* One effect's collected key/value picture. */
#define MAXKV 96
typedef struct { char k[64]; char v[48]; int is_str; long long num; int present; } KV;
typedef struct { KV kv[MAXKV]; int n; char action[48]; } Eff;

static const char *eff_get_str(const Eff *e, const char *k) {
    for (int i = 0; i < e->n; i++)
        if (e->kv[i].present && e->kv[i].is_str && !strcmp(e->kv[i].k, k)) return e->kv[i].v;
    return NULL;
}
static int eff_has(const Eff *e, const char *k) {
    for (int i = 0; i < e->n; i++) if (e->kv[i].present && !strcmp(e->kv[i].k, k)) return 1;
    return 0;
}
static long long eff_num(const Eff *e, const char *k, long long dflt) {
    for (int i = 0; i < e->n; i++)
        if (e->kv[i].present && !strcmp(e->kv[i].k, k) && !e->kv[i].is_str) return e->kv[i].num;
    return dflt;
}

static void note(const char *k) {
    for (int i = 0; i < n_rows; i++) if (!strcmp(rows[i].k, k)) {
        rows[i].nodes++; if (rows[i].last_slot != cur_slot) { rows[i].last_slot = cur_slot; rows[i].abilities++; }
        return;
    }
    int at = n_rows; memset(&rows[at], 0, sizeof(Row));
    snprintf(rows[at].k, 80, "%s", k); n_rows++;
    rows[at].nodes++; if (rows[at].last_slot != cur_slot) { rows[at].last_slot = cur_slot; rows[at].abilities++; }
}
/* note a branch predicate: "<family>|<guard>" */
static void branch(const char *fam, const char *guard, int on) {
    char buf[320];
    snprintf(buf, sizeof buf, "%s :: %s", fam, guard);
    if (on) note(buf); else { char b2[320]; snprintf(b2,sizeof b2,"%s :: %s [FALSE]", fam, guard); note(b2); }
    if (on) printf("HIT|ability=%d|%s :: %s\n", cur_slot, fam, guard);
}

static void walk_effect(uint8_t tag);

static void walk_effect(uint8_t tag) {
    if (tag != 0x09) { skipv(tag); return; }
    unsigned char v; uint32_t n, i, k;
    if (!ru8(&v)) return;
    if (!rlen(&n)) return;
    /* First pass: read every field into an Eff while remembering the total
     * number of entries so the object is fully consumed. */
    Eff e; memset(&e, 0, sizeof e);
    unsigned char tags[128];
    for (i = 0; i < n; i++) {
        if (!ridx(&k)) return;
        const char *ks = rb_get_string(k);
        uint8_t t; if (!ru8(&t)) return;
        if (i < 128) tags[i] = t;
        KV *slot = (e.n < MAXKV) ? &e.kv[e.n] : NULL;
        if (slot) {
            slot->present = 1;
            snprintf(slot->k, 64, "%s", ks ? ks : "");
            if (t == 0x06) { uint32_t i2; if (!ridx(&i2)) return;
                slot->is_str = 1; snprintf(slot->v, 48, "%s", rb_get_string(i2) ? rb_get_string(i2) : ""); }
            else if (t == 0x03) { int64_t x; if (!rint(&x)) return; slot->is_str = 0; slot->num = x; }
            else if (t == 0x02) { slot->is_str = 0; slot->num = 1; }
            else if (t == 0x01) { slot->is_str = 0; slot->num = 0; }
            else { slot->is_str = 0; slot->num = -999; }
            e.n++;
        }
        if (t != 0x06) {
                /* non-scalar: record presence, then consume */
                if (slot) slot->present = 1;
                if (t == 0x07 || t == 0x08 || t == 0x09 || t == 0x04) {
                /* recurse so nested effects/conditions are walked */
                if (t == 0x09) {
                    unsigned char vv; const unsigned char *save = P;
                    if (ru8(&vv)) {
                        uint32_t ln; if (rlen(&ln)) {
                            if (!strcmp(slot->k, "options") || !strcmp(slot->k, "effect_steps") ||
                                !strcmp(slot->k, "actions")) {
                                P = save; walk_effect(0x09); continue;
                            }
                            P = save; skipv(0x09); continue;
                        }
                    }
                    P = save; skipv(0x09);
                } else skipv(t);
            } else if (t == 0x03 || t == 0x02 || t == 0x01 || t == 0x00) { /* already consumed */ }
        }
    }
    snprintf(e.action, 48, "%s", eff_get_str(&e, "action") ? eff_get_str(&e, "action") : "(none)");
    n_eff_nodes++;
    {
        const char *fam = NULL;
        if      (!strcmp(e.action, "gain_resource"))              fam = "gain_resource";
        else if (!strcmp(e.action, "pay_energy"))                fam = "pay_energy";
        else if (!strcmp(e.action, "place_energy_under_member")) fam = "place_energy";
        else if (!strcmp(e.action, "draw_card"))                 fam = "draw_card";
        else if (!strcmp(e.action, "draw_until_count"))          fam = "draw_until";
        if (!fam) return;
        char g[320];
        const char *res = eff_get_str(&e, "resource");
        const char *ct  = eff_get_str(&e, "card_type");
        const char *put = eff_get_str(&e, "per_unit_type");
        const char *tgt = eff_get_str(&e, "target");
        const char *loc = eff_get_str(&e, "location");
        const char *src = eff_get_str(&e, "source");
        snprintf(g,sizeof g,"%s|resource=%s", fam, res?res:"(none)"); note(g);
        snprintf(g,sizeof g,"%s|card_type=%s", fam, ct?ct:"(none)");   note(g);
        snprintf(g,sizeof g,"%s|target=%s", fam, tgt?tgt:"(none)");   note(g);
        snprintf(g,sizeof g,"%s|location=%s", fam, loc?loc:"(none)"); note(g);
        snprintf(g,sizeof g,"%s|source=%s", fam, src?src:"(none)");   note(g);
        snprintf(g,sizeof g,"%s|per_unit=%s", fam, eff_has(&e,"per_unit")?(eff_num(&e,"per_unit",0)?"true":"false"):"(absent)"); note(g);
        snprintf(g,sizeof g,"%s|per_unit_type=%s", fam, put?put:"(none)"); note(g);
        snprintf(g,sizeof g,"%s|per_unit_count=%lld", fam, eff_num(&e,"per_unit_count",-1)); note(g);
        snprintf(g,sizeof g,"%s|multiple_targets=%s", fam, eff_has(&e,"multiple_targets")?(eff_num(&e,"multiple_targets",0)?"true":"false"):"(absent)"); note(g);
        snprintf(g,sizeof g,"%s|duration=%s", fam, eff_get_str(&e,"duration")?eff_get_str(&e,"duration"):"(none)"); note(g);
        snprintf(g,sizeof g,"%s|all=%s", fam, eff_has(&e,"all")?(eff_num(&e,"all",0)?"true":"false"):"(absent)"); note(g);
        snprintf(g,sizeof g,"%s|self_target=%s", fam, eff_has(&e,"self_target")?(eff_num(&e,"self_target",0)?"true":"false"):"(absent)"); note(g);
        snprintf(g,sizeof g,"%s|count=%lld", fam, eff_num(&e,"count",-1)); note(g);
        snprintf(g,sizeof g,"%s|energy=%lld", fam, eff_num(&e,"energy",-1)); note(g);
        snprintf(g,sizeof g,"%s|energy_count=%lld", fam, eff_num(&e,"energy_count",-1)); note(g);
        snprintf(g,sizeof g,"%s|heart_colors=%s", fam, eff_has(&e,"heart_colors")?"yes":"no"); note(g);
        snprintf(g,sizeof g,"%s|heart_colors_from_selected_card=%s", fam, eff_has(&e,"heart_colors_from_selected_card")?(eff_num(&e,"heart_colors_from_selected_card",0)?"true":"false"):"(absent)"); note(g);
        snprintf(g,sizeof g,"%s|require_all_heart_colors=%s", fam, eff_has(&e,"require_all_heart_colors")?(eff_num(&e,"require_all_heart_colors",0)?"true":"false"):"(absent)"); note(g);
        snprintf(g,sizeof g,"%s|per_group=%s", fam, eff_has(&e,"per_group")?(eff_num(&e,"per_group",0)?"true":"false"):"(absent)"); note(g);
        snprintf(g,sizeof g,"%s|heart_type=%s", fam, eff_get_str(&e,"heart_type")?eff_get_str(&e,"heart_type"):"(none)"); note(g);
        snprintf(g,sizeof g,"%s|per_unit_aggregate=%s", fam, eff_get_str(&e,"per_unit_aggregate")?eff_get_str(&e,"per_unit_aggregate"):"(none)"); note(g);
        snprintf(g,sizeof g,"%s|per_unit_source=%s", fam, eff_get_str(&e,"per_unit_source")?eff_get_str(&e,"per_unit_source"):"(none)"); note(g);
        snprintf(g,sizeof g,"%s|target_count=%lld", fam, eff_num(&e,"target_count",-1)); note(g);
        snprintf(g,sizeof g,"%s|shuffle=%s", fam, eff_has(&e,"shuffle")?(eff_num(&e,"shuffle",0)?"true":"false"):"(absent)"); note(g);
        snprintf(g,sizeof g,"%s|dynamic_count=%s", fam, eff_has(&e,"dynamic_count")?"yes":"no"); note(g);
        snprintf(g,sizeof g,"%s|exclude_self=%s", fam, eff_has(&e,"exclude_self")?(eff_num(&e,"exclude_self",0)?"true":"false"):"(absent)"); note(g);
        snprintf(g,sizeof g,"%s|distinct=%s", fam, eff_has(&e,"distinct")?(eff_num(&e,"distinct",0)?"true":"false"):"(absent)"); note(g);
        snprintf(g,sizeof g,"%s|heart_color=%s", fam, eff_get_str(&e,"heart_color")?eff_get_str(&e,"heart_color"):"(none)"); note(g);
        snprintf(g,sizeof g,"%s|group_names=%s", fam, eff_has(&e,"group_names")?"yes":"no"); note(g);
        snprintf(g,sizeof g,"%s|is_all_HEURISTIC_RUST_TERM1", fam,
                 (!src && ct && !strcmp(ct,"member_card") && tgt && (!strcmp(tgt,"self")||!strcmp(tgt,"opponent"))
                  && eff_num(&e,"self_target",0)==0 && !eff_has(&e,"exclude_self")
                  && eff_num(&e,"target_count",-1)<0) ? "TRUE" : "false");
        note(g);
        snprintf(g,sizeof g,"%s|is_all_HEURISTIC_RUST_TERM3", fam,
                 (ct && !strcmp(ct,"member_card") && tgt && (!strcmp(tgt,"self")||!strcmp(tgt,"opponent"))
                  && eff_num(&e,"target_count",-1)<0 && !eff_has(&e,"distinct")) ? "TRUE" : "false");
        note(g);
        snprintf(g,sizeof g,"%s|is_all_DIVERGES_RUST_TERM1(exclude_self_present_but_false)", fam,
                 (!src && ct && !strcmp(ct,"member_card") && tgt && (!strcmp(tgt,"self")||!strcmp(tgt,"opponent"))
                  && eff_num(&e,"self_target",0)==0 && eff_has(&e,"exclude_self") && eff_num(&e,"exclude_self",0)==0
                  && eff_num(&e,"target_count",-1)<0) ? "DIVERGE" : "-");
        note(g);
        snprintf(g,sizeof g,"%s|is_all_DIVERGES_RUST_TERM3(distinct_present_but_false)", fam,
                 (ct && !strcmp(ct,"member_card") && tgt && (!strcmp(tgt,"self")||!strcmp(tgt,"opponent"))
                  && eff_num(&e,"target_count",-1)<0 && eff_has(&e,"distinct") && eff_num(&e,"distinct",0)==0) ? "DIVERGE" : "-");
        note(g);
        if (!strcmp(fam,"gain_resource")) {
            branch("resource.rs:305-310 handle_bp6_pattern","FULL GUARD TRUE",
                res&&!strcmp(res,"heart")&&eff_num(&e,"per_unit",0)!=0&&put&&!strcmp(put,"discard")&&eff_num(&e,"multiple_targets",0)!=0);
            branch("resource.rs:40 ResourceKind::from_str","_ => Other", res&&strcmp(res,"blade")&&strcmp(res,"heart"));
            branch("resource.rs:40 ResourceKind::from_str","\"blade\"", res&&!strcmp(res,"blade"));
            branch("resource.rs:40 ResourceKind::from_str","\"heart\"", res&&!strcmp(res,"heart"));
            branch("resource.rs:1453 apply_blade_resource","card_type absent", !ct);
            branch("resource.rs:1453 apply_blade_resource","card_type==member_card", ct&&!strcmp(ct,"member_card"));
            branch("resource.rs:1453 apply_blade_resource","card_type==live_card", ct&&!strcmp(ct,"live_card"));
            branch("resource.rs:1453 apply_blade_resource","card_type==energy_card", ct&&!strcmp(ct,"energy_card"));
            branch("resource.rs:1329 apply_heart_resource","heart_colors_from_selected_card", eff_num(&e,"heart_colors_from_selected_card",0)!=0);
            branch("resource.rs:1329 apply_heart_resource","require_all_heart_colors", eff_num(&e,"require_all_heart_colors",0)!=0);
            branch("resource.rs:1329 apply_heart_resource","per_group", eff_num(&e,"per_group",0)!=0);
            branch("resource.rs:1329 apply_heart_resource","heart_type present", !!eff_get_str(&e,"heart_type"));
            branch("resource.rs:314 is_temporary","duration!=permanent", eff_get_str(&e,"duration")&&strcmp(eff_get_str(&e,"duration"),"permanent"));
        }
        if (!strcmp(fam,"pay_energy")) {
            branch("energy.rs:546 execute_pay_energy","card_type=energy_card", ct&&!strcmp(ct,"energy_card"));
            branch("energy.rs:546 execute_pay_energy","target=self", tgt&&!strcmp(tgt,"self"));
            branch("energy.rs:546 execute_pay_energy","energy key", eff_has(&e,"energy"));
            branch("energy.rs:546 execute_pay_energy","energy_count key", eff_has(&e,"energy_count"));
            branch("energy.rs:546 execute_pay_energy","source=energy_zone", src&&!strcmp(src,"energy_zone"));
            branch("energy.rs:546 execute_pay_energy","source=energy_deck", src&&!strcmp(src,"energy_deck"));
        }
        if (!strcmp(fam,"place_energy")) {
            branch("energy.rs:311 place_energy_impl","card_type=member_card", ct&&!strcmp(ct,"member_card"));
            branch("energy.rs:311 place_energy_impl","energy key", eff_has(&e,"energy"));
            branch("energy.rs:311 place_energy_impl","energy_count key", eff_has(&e,"energy_count"));
            branch("energy.rs:311 place_energy_impl","location present", !!loc);
            branch("energy.rs:311 place_energy_impl","self_cost", eff_num(&e,"self_cost",0)!=0);
            branch("energy.rs:311 place_energy_impl","allow_occupied_stage", eff_num(&e,"allow_occupied_stage",0)!=0);
            branch("energy.rs:311 place_energy_impl","target_member present", !!eff_get_str(&e,"target_member"));
            branch("energy.rs:311 place_energy_impl","source=under_member", src&&!strcmp(src,"under_member"));
        }
        if (!strcmp(fam,"draw_until")) {
            branch("deck.rs:483 execute_draw_until_count","target_count present", eff_has(&e,"target_count"));
            branch("deck.rs:483 execute_draw_until_count","location present", !!loc);
            branch("deck.rs:483 execute_draw_until_count","count present", eff_has(&e,"count"));
        }
        if (!strcmp(fam,"draw_card")) {
            static const char *S[] = {"deck","deck_top","deck_bottom","energy_deck","discard","hand",
                                      "energy_zone","stage","looked_at","revealed_cards","looked_at_remaining",
                                      "success_live_zone","live_card_zone","selected_cards","those_cards",
                                      "recently_moved","under_member",NULL};
            for (int q=0;S[q];q++) {
                char b2[320];
                snprintf(b2,sizeof b2,"draw_card :: source==\"%s\"%s", S[q],
                         (src && !strcmp(src,S[q])) ? "" : " [FALSE]");
                note(b2);
            }
            branch("deck.rs:178 execute_draw","shuffle flag", eff_num(&e,"shuffle",0)!=0);
            branch("deck.rs:178 execute_draw","count present", eff_has(&e,"count"));
            branch("deck.rs:178 execute_draw","per_unit", eff_num(&e,"per_unit",0)!=0);
            branch("deck.rs:178 execute_draw","dynamic_count present", eff_has(&e,"dynamic_count"));
        }
    }
}

static int cmp_desc(const void *a, const void *b) {
    const Row *x = a, *y = b;
    if (x->nodes != y->nodes) return (y->nodes > x->nodes) ? 1 : -1;
    return strcmp(x->k, y->k);
}

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) < 0) { fprintf(stderr, "rb_load failed\n"); return 2; }
    int n_abilities = 0;
    for (uint32_t a = 0; a < 2000; a++) {
        uint32_t len = 0;
        const unsigned char *slice = rb_bc_slice(a, &len);
        if (!slice || len == 0) continue;
        n_abilities++;
        cur_slot = (int)a;
        P = slice; E = slice + len;
        uint8_t tag; uint32_t n, i, k;
        if (!ru8(&tag) || tag != 0x08) continue;
        if (!rlen(&n)) continue;
        for (i = 0; i < n; i++) {
            if (!ridx(&k)) break;
            const char *ks = rb_get_string(k);
            uint8_t t; if (!ru8(&t)) break;
            const char *key = ks ? ks : "";
            if (!strcmp(key,"cost") || !strcmp(key,"effect")) walk_effect(t);
            else skipv(t);
        }
    }
    printf("abilities=%d effects_walked=%ld rows=%d\n", n_abilities, n_eff_nodes, n_rows);
    qsort(rows, n_rows, sizeof(Row), cmp_desc);
    for (int i = 0; i < n_rows; i++) printf("R|%s|nodes=%ld|abilities=%ld\n", rows[i].k, rows[i].nodes, rows[i].abilities);
    rb_unload();
    return 0;
}