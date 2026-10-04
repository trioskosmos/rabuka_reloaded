/* Cluster T7  ERAW WIRE census for the two GENERATED decoders.
 *
 * audit_t7_decoders.c walks the C's decoded trees. That is the wrong level for
 * a per-variant comparison of a GENERATED decoder, because the C mirror folds
 * nested objects (trigger_event, sub_checks, position, quoted_text,
 * dynamic_count) into Condition::fields[], so their keys get attributed to the
 * condition key table. This tool walks the BYTECODE itself, with the same key ->
 * node-class routing the Rust generator's readers use, so every key is counted
 * in the class Rust assigns it.
 *
 * Output:
 *   COND|<key>|<nodes>|<abilities>|C_TYPED|C_DROPPED|CONST_ONLY
 *   EFF|<key>|<nodes>|<abilities>|C_STRUCT|SCALAR_TAIL
 *   CVAR|<variant>|<nodes>
 *   EVAR|<variant>|<nodes>
 *   TKEY|<nodeclass>|<key>|<nodes>
 */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "rabuka.h"

#define MAXKEYS 2048
typedef struct { char k[64]; long nodes; long abilities; long last_slot; } Row;
static Row cond_keys[MAXKEYS]; static int n_cond_keys;
static Row eff_keys[MAXKEYS];  static int n_eff_keys;
static Row other_keys[MAXKEYS]; static int n_other_keys;
static Row vals[MAXKEYS];       static int n_vals;
static long cond_var[32], eff_var[32];
static long n_cond_nodes, n_eff_nodes;
static int  cur_slot;

/* Keys whose VALUE decides a Rust/C divergence, so the value census is
 * recorded alongside the key census. */
static int wants_value(const char *k) {
    return !strcmp(k,"phase") || !strcmp(k,"phase_target") ||
           !strcmp(k,"source") || !strcmp(k,"resource_type") ||
           !strcmp(k,"card_type") || !strcmp(k,"comparison_type") ||
           !strcmp(k,"operator") || !strcmp(k,"state_change") ||
           !strcmp(k,"action") || !strcmp(k,"temporal") ||
           !strcmp(k,"energy_state") || !strcmp(k,"movement") ||
           !strcmp(k,"resource") || !strcmp(k,"count") ||
           !strcmp(k,"restriction_type") || !strcmp(k,"duration");
}

static const unsigned char *P, *E;

static int ru8(unsigned char *o){ if (P+1>E) return 0; *o=*P++; return 1; }
static int ru16(unsigned short *o){ if (P+2>E) return 0; *o=(unsigned short)(P[0]|(P[1]<<8)); P+=2; return 1; }
static int ru32(uint32_t *o){ if (P+4>E) return 0; *o=(uint32_t)P[0]|((uint32_t)P[1]<<8)|((uint32_t)P[2]<<16)|((uint32_t)P[3]<<24); P+=4; return 1; }
static int rlen(uint32_t *o){ unsigned char b; if(!ru8(&b)) return 0; if(b<0xFE){*o=b;return 1;} unsigned short v; if(!ru16(&v)) return 0; *o=v; return 1; }
static int ridx(uint32_t *o){ unsigned char b; if(!ru8(&b)) return 0; if(b==0xFE){unsigned short v; if(!ru16(&v)) return 0; *o=v; return 1;} *o=b; return 1; }
static int rival(int64_t *o){
    unsigned char b; if(!ru8(&b)) return 0;
    if(b<=0xFD){*o=b;return 1;}
    if(b==0xFE){unsigned short v; if(!ru16(&v)) return 0; *o=v;return 1;}
    if(b==0xFF){uint32_t v; if(!ru32(&v)) return 0; *o=(int32_t)v;return 1;}
    if(P+8>E) return 0;
    uint32_t lo, hi;
    if(!ru32(&lo) || !ru32(&hi)) return 0;
    *o = (int64_t)(((uint64_t)hi << 32) | lo);
    return 1;
}

static int skipv(uint8_t tag);
static int skip_one(void){ uint8_t t; if(!ru8(&t)) return 0; return skipv(t); }
static int skipv(uint8_t tag){
    uint32_t len,i,k; unsigned char b;
    switch(tag){
    case 0x00: case 0x01: case 0x02: return 1;
    case 0x03: { int64_t v; return rival(&v); }
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

static Row *find(Row *tab, int *n, const char *k) {
    for (int i = 0; i < *n; i++) if (!strcmp(tab[i].k, k)) return &tab[i];
    int at = *n; memset(&tab[at], 0, sizeof(Row));
    snprintf(tab[at].k, 64, "%s", k); (*n)++;
    return &tab[at];
}
static void bump(Row *r) {
    r->nodes++;
    if (r->last_slot != cur_slot) { r->last_slot = cur_slot; r->abilities++; }
}
static void note(Row *tab, int *n, const char *k){ bump(find(tab, n, k)); }
static void note_val(const char *cls, const char *key, const char *val) {
    char buf[160];
    snprintf(buf, sizeof buf, "%s|%s=%s", cls, key, val ? val : "(null)");
    bump(find(vals, &n_vals, buf));
}
/* Read a TAG_STR value's interned string (caller has already read `t`). */
static const char *peek_str(uint8_t t, const unsigned char **save) {
    *save = P;
    if (t == 0x06) { uint32_t i; if (!ridx(&i)) return NULL; return rb_get_string(i); }
    return NULL;
}

/* The C typed condition decoder key set (vm.c decode_condition_field). */
static const char *C_TYPED_COND[] = {
 "ability_filter","ability_filter_triggers","action_reference","activation_position",
 "aggregate","all","all_areas","all_members","any_of","appearance","appearance_source",
 "area_direction","baton_touch_source","baton_touch_trigger","blade_greater_than_all",
 "blade_limit","blade_limit_operator","cache","card_names","card_property","card_type",
 "cause","characters","check_self","comparison_source","comparison_target","comparison_type",
 "condition","conditions","cost_limit","cost_limit_operator","cost_reference_character",
 "cost_reference_operator","cost_total","cost_total_operator","count","delta","destination",
 "distinct","effect","energy_placed","energy_state","exclude_characters","exclude_group_names",
 "exclude_self","from_state","group_names","group_reference","heart_colors","heart_source",
 "heart_type","location","locations","min_baton_touch_count","movement","negation",
 "no_excess_heart","operator","options","original_value","phase","phase_target","position",
 "position_compare","positions_characters","reference_card","require_position_cards",
 "resource_type","same_name","scope","self_effect_only","self_target","shuffle","source",
 "state","sub_checks","target","temporal","temporal_scope","to_state","turn_number","unit",
 "values","yell_trigger","type", NULL
};
static int in_list(const char *const *l, const char *k){
    for (int i = 0; l[i]; i++) if (!strcmp(l[i], k)) return 1;
    return 0;
}

/* C decode_effect_body's STRUCTURAL dispatch (vm.c:699-1167). Everything not
 * listed here lands in the scalar tail (vm.c:1169-1190), which retains the
 * value in the extras table. */
static const char *C_EFF_STRUCT[] = {
 "text","action","type","cost_type","source","destination","target","zone","count",
 "condition","activation_condition_parsed","dynamic_count","quoted_text","placement_order",
 "optional","non_stackable","conditional","conditional_negation","is_further","max",
 "result_condition","alternative_condition","actions","effect_steps","costs","options",
 "look_action","select_action","primary_effect","alternative_effect","followup_action",
 "optional_action","conditional_action","resource_on_select","opponent_action",
 "gained_effect","repeat_limit","max_repeats","energy","energy_count","per_unit_count",
 "cost_reduction_per_group","id","card_type","self_target","per_unit","distinct", NULL
};

static long te_probe[4];      /* te has phase / te has phase_target / own lacks phase / own lacks phase_target */
static int  te_cur[4];
static Row *te_rows; static int n_te_rows;
static void walk_effect(uint8_t tag);
static void walk_cond(uint8_t tag);
static void walk_named_obj(const char *cls, uint8_t tag);
#define TE_MAX 64
static Row te_tab[TE_MAX];
static Row *te_rows_storage(void) { return te_tab; }

static void walk_cond(uint8_t tag) {
    if (tag != 0x09) { skipv(tag); return; }
    unsigned char v; uint32_t n, i, k;
    if (!ru8(&v)) return;
    if (v < 32) cond_var[v]++;
    if (!rlen(&n)) return;
    n_cond_nodes++;
    te_cur[0] = te_cur[1] = 0; te_cur[2] = te_cur[3] = 1;
    for (i = 0; i < n; i++) {
        if (!ridx(&k)) return;
        const char *ks = rb_get_string(k);
        uint8_t t; if (!ru8(&t)) return;
        const char *key = ks ? ks : "";
        note(cond_keys, &n_cond_keys, key);
        if (!strcmp(key, "phase")) te_cur[2] = 0;
        if (!strcmp(key, "phase_target")) te_cur[3] = 0;
        if (wants_value(key)) {
            const unsigned char *save; const char *sv = peek_str(t, &save);
            if (sv) note_val("cond", key, sv);
            P = save; /* the real path below re-reads the value with its own tag */
        }
        if (!strcmp(key,"condition")||!strcmp(key,"cause")) walk_cond(t);
        else if (!strcmp(key,"conditions")) {
            if (t==0x07){ uint32_t len,j; if(!rlen(&len)) return;
                for(j=0;j<len;j++){ uint8_t st; if(!ru8(&st)) return; walk_cond(st); } }
            else skipv(t);
        }
        else if (!strcmp(key,"effect")) walk_effect(t);
        else if (!strcmp(key,"options")) {
            if (t==0x07){ uint32_t len,j; if(!rlen(&len)) return;
                for(j=0;j<len;j++){ uint8_t st; if(!ru8(&st)) return; walk_effect(st); } }
            else skipv(t);
        }
        else if (!strcmp(key,"trigger_event")) walk_named_obj("trigger_event", t);
        else if (!strcmp(key,"sub_checks")) walk_named_obj("sub_checks", t);
        else if (!strcmp(key,"position") && (t==0x08||t==0x09)) walk_named_obj("position", t);
        else if (!strcmp(key,"distinct") && (t==0x08||t==0x09)) walk_named_obj("distinct", t);
        else skipv(t);
    }
    /* Rust's check_phase_gate (condition.rs:398-422) falls back to
     * trigger_event.phase / .phase_target when the condition's own key is
     * absent; C's rb_check_phase_gate (condition.c:2547-2574) has no such
     * fallback. Cases 2 and 3 are where the two engines disagree. */
    te_probe[0] += te_cur[0];
    te_probe[1] += te_cur[1];
    if (te_cur[0] && te_cur[2]) { te_probe[2]++; bump(find(te_rows, &n_te_rows, "te_phase_without_own_phase")); }
    if (te_cur[1] && te_cur[3]) { te_probe[3]++; bump(find(te_rows, &n_te_rows, "te_phase_target_without_own")); }
}

static void walk_effect(uint8_t tag) {
    if (tag != 0x09) { skipv(tag); return; }
    unsigned char v; uint32_t n, i, k;
    if (!ru8(&v)) return;
    if (v < 32) eff_var[v]++;
    if (!rlen(&n)) return;
    n_eff_nodes++;
    for (i = 0; i < n; i++) {
        if (!ridx(&k)) return;
        const char *ks = rb_get_string(k);
        uint8_t t; if (!ru8(&t)) return;
        const char *key = ks ? ks : "";
        note(eff_keys, &n_eff_keys, key);
        if (wants_value(key)) {
            const unsigned char *save; const char *sv = peek_str(t, &save);
            if (sv) note_val("eff", key, sv);
            P = save;
        }
        if (!strcmp(key,"condition")||!strcmp(key,"result_condition")||!strcmp(key,"alternative_condition")
            ||!strcmp(key,"activation_condition_parsed")||!strcmp(key,"choice_condition")) walk_cond(t);
        else if (!strcmp(key,"dynamic_count")) walk_named_obj("dynamic_count", t);
        else if (!strcmp(key,"quoted_text")) walk_named_obj("quoted_text", t);
        else if (!strcmp(key,"or_ability_filters")) {
            if (t==0x07){ uint32_t len,j; if(!rlen(&len)) return;
                for(j=0;j<len;j++){ uint8_t st; if(!ru8(&st)) return; walk_named_obj("or_ability_filter", st); } }
            else skipv(t);
        }
        else if (!strcmp(key,"actions")||!strcmp(key,"effect_steps")||!strcmp(key,"costs")) {
            if (t==0x07){ uint32_t len,j; if(!rlen(&len)) return;
                for(j=0;j<len;j++){ uint8_t st; if(!ru8(&st)) return;
                    if (st==0x09) walk_effect(st); else skipv(st); } }
            else skipv(t);
        }
        else if (!strcmp(key,"options")) {
            if (t==0x07){ uint32_t len,j; if(!rlen(&len)) return;
                for(j=0;j<len;j++){ uint8_t st; if(!ru8(&st)) return;
                    if (st==0x09) walk_effect(st); else skipv(st); } }
            else skipv(t);
        }
        else if (!strcmp(key,"look_action")||!strcmp(key,"select_action")||
                 !strcmp(key,"primary_effect")||!strcmp(key,"alternative_effect")||
                 !strcmp(key,"followup_action")||!strcmp(key,"optional_action")||
                 !strcmp(key,"conditional_action")||!strcmp(key,"resource_on_select")||
                 !strcmp(key,"opponent_action")||!strcmp(key,"gained_effect")) walk_effect(t);
        else skipv(t);
    }
}

/* A dict whose keys belong to a named node class (TriggerEvent,
 * LocationSubChecks, PositionInfo, QuotedText, DynamicCount,
 * AbilityFilterBranch). Tagged into one flat table prefixed by class. */
static void walk_named_obj(const char *cls, uint8_t tag) {
    if (tag != 0x08 && tag != 0x09) { skipv(tag); return; }
    unsigned char v; uint32_t n, i, k;
    if (tag == 0x09) { if (!ru8(&v)) return; }
    if (!rlen(&n)) return;
    for (i = 0; i < n; i++) {
        if (!ridx(&k)) return;
        const char *ks = rb_get_string(k);
        uint8_t t; if (!ru8(&t)) return;
        char buf[96];
        snprintf(buf, sizeof buf, "%s:%s", cls, ks ? ks : "");
        note(other_keys, &n_other_keys, buf);
        if (!strncmp(cls, "trigger_event", 13) &&
            (!strcmp(ks, "phase") || !strcmp(ks, "phase_target"))) {
            const unsigned char *save; const char *sv = peek_str(t, &save);
            if (sv) note_val("trigger_event", ks, sv);
            if (!strcmp(ks, "phase")) te_cur[0] = 1;
            if (!strcmp(ks, "phase_target")) te_cur[1] = 1;
            P = save;
        }
        /* Recurse so nothing hides behind an unmodelled nested dict: a key the
         * top-level reader never sees is exactly the class of bug this audit
         * is looking for. */
        if (t == 0x08 || t == 0x09) walk_named_obj("deep", t);
        else if (t == 0x07) {
            uint32_t len, j;
            if (!rlen(&len)) return;
            for (j = 0; j < len; j++) { uint8_t st; if (!ru8(&st)) return;
                if (st == 0x08 || st == 0x09) walk_named_obj("deep", st); else skipv(st); }
        } else skipv(t);
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
    int n_abilities = 0, n_skipped = 0;
    n_te_rows = 0; te_rows = te_rows_storage();
    for (uint32_t a = 0; a < 2000; a++) {
        uint32_t len = 0;
        const unsigned char *slice = rb_bc_slice(a, &len);
        if (!slice || len == 0) continue;
        n_abilities++;
        cur_slot = (int)a;
        P = slice; E = slice + len;
        uint8_t tag; uint32_t n, i, k;
        if (!ru8(&tag) || tag != 0x08) { n_skipped++; continue; }
        if (!rlen(&n)) { n_skipped++; continue; }
        for (i = 0; i < n; i++) {
            if (!ridx(&k)) break;
            const char *ks = rb_get_string(k);
            uint8_t t; if (!ru8(&t)) break;
            const char *key = ks ? ks : "";
            if (!strcmp(key,"cost") || !strcmp(key,"effect")) walk_effect(t);
            else skipv(t);
        }
    }
    printf("abilities_walked=%d skipped=%d cond_nodes=%ld eff_nodes=%ld\n",
           n_abilities, n_skipped, n_cond_nodes, n_eff_nodes);
    qsort(cond_keys, n_cond_keys, sizeof(Row), cmp_desc);
    for (int i = 0; i < n_cond_keys; i++) {
        const char *verdict = in_list(C_TYPED_COND, cond_keys[i].k) ? "C_TYPED"
                            : (cond_keys[i].nodes ? "C_DROPPED" : "C_DROPPED");
        printf("COND|%s|%ld|%ld|%s\n", cond_keys[i].k, cond_keys[i].nodes,
               cond_keys[i].abilities, verdict);
    }
    qsort(eff_keys, n_eff_keys, sizeof(Row), cmp_desc);
    for (int i = 0; i < n_eff_keys; i++)
        printf("EFF|%s|%ld|%ld|%s\n", eff_keys[i].k, eff_keys[i].nodes,
               eff_keys[i].abilities, in_list(C_EFF_STRUCT, eff_keys[i].k) ? "C_STRUCT" : "SCALAR_TAIL");
    qsort(other_keys, n_other_keys, sizeof(Row), cmp_desc);
    for (int i = 0; i < n_other_keys; i++)
        printf("NEST|%s|%ld\n", other_keys[i].k, other_keys[i].nodes);
    qsort(vals, n_vals, sizeof(Row), cmp_desc);
    for (int i = 0; i < n_vals; i++)
        printf("VAL|%s|%ld\n", vals[i].k, vals[i].nodes);
    for (int i = 0; i < 32; i++) if (cond_var[i]) printf("CVAR|%d|%ld\n", i, cond_var[i]);
    for (int i = 0; i < 32; i++) if (eff_var[i])  printf("EVAR|%d|%ld\n", i, eff_var[i]);
    printf("TE|te_has_phase=%ld|te_has_phase_target=%ld|DIVERGE_phase=%ld|DIVERGE_phase_target=%ld\n",
           te_probe[0], te_probe[1], te_probe[2], te_probe[3]);
    for (int i = 0; i < n_te_rows; i++)
        printf("TEDIV|%s|nodes=%ld|abilities=%ld\n", te_rows[i].k, te_rows[i].nodes, te_rows[i].abilities);
    rb_unload();
    return 0;
}