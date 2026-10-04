/* Cluster D - condition census over the REAL shipped bytecode.
 *
 * Walks src/core/generated/gen_data.c + src/abilities_strings.bin through the
 * engine's own decoder (rb_decode_card_ability) and answers, per question, with
 * counts over the 985 abilities / 1589 condition nodes. No number here is
 * derived by reading Rust and guessing.
 *
 * Sections:
 *   A  variant census
 *   B  Comparison + negation (D4 double-negation reach)
 *   C  heart_type "all" (D2/D3)
 *   D  comparison_type / location shapes that reach get_count_for_condition (D6)
 *   E  highest-cost-on-stage (D5)
 *   F  no_excess_heart (D7)
 *   G  per-key census for the D8 list, with the variant it lands on
 */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "rabuka.h"

/* ── the 24 keys D8 lists, plus the D4/D5/D6/D7 discriminators ── */
static const char *D8_KEYS[] = {
    "blade_limit","blade_limit_operator","cost_limit_operator","self_effect_only",
    "watches_area_move","trigger_event","action_reference","cost_total",
    "cost_total_operator","original_value","reference_card","require_position_cards",
    "same_name","area_direction","blade_greater_than_all","activated_this_turn",
    "paid_cost_this_turn","cost_reference_character","cost_reference_operator",
    "baton_touch_source","energy_placed","any_location","original_heart_total",
    "cost_reference", NULL
};

static long n_nodes, n_abilities;
static int cur_slot, cur_cid, cur_abi;
static unsigned char ability_hit[65536];
static unsigned char kslot_seen[64][2200];
static int n_slots;

/* per key: nodes, slots(=abilities), variant bitmask (variants 0..20) */
typedef struct { const char *k; long nodes, slots; unsigned long variants; int present; int shown; } KRow;
static KRow krows[64]; static int n_krows;

static KRow *kfind(const char *k) {
    for (int i = 0; i < n_krows; i++) if (!strcmp(krows[i].k, k)) return &krows[i];
    krows[n_krows].k = k; krows[n_krows].present = 1; return &krows[n_krows++];
}

static const CondValue *fval(const Condition *c, const char *key) {
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, key)) return &c->fields[i].v;
    return NULL;
}
static const char *fstr(const Condition *c, const char *key) {
    const CondValue *v = fval(c, key);
    return (v && v->tag == RB_TAG_STR) ? v->s : NULL;
}
static int ftrue(const Condition *c, const char *key) {
    const CondValue *v = fval(c, key);
    if (!v) return 0;
    switch (v->tag) {
        case RB_TAG_TRUE:  return 1;
        case RB_TAG_FALSE: return 0;
        case RB_TAG_I64:   return v->i != 0;
        case RB_TAG_STR:   return v->s && (!strcmp(v->s,"true") || !strcmp(v->s,"1"));
        default: return 0;
    }
}

/* ── section counters ── */
static long b_cmp_total, b_cmp_neg, b_cmp_neg_generic, b_cmp_neg_values,
            b_cmp_neg_costtotal, b_cmp_neg_both, b_cmp_neg_highest, b_cmp_neg_allcost,
            b_cmp_neg_plainloc_heartall, b_cmp_early_return;
static long c_ht_all, c_ht_all_neg, c_ht_all_plainloc, c_ht_all_plainloc_neg,
            c_ht_all_notplainloc, c_ht_all_locstage;
static long e_highest, e_highest_centerstage;
static long f_noexcess, f_noexcess_via_subchecks;
static long d_score_moved, d_cost_noloc, d_cost_revealed, d_cost_self_max, d_cost_loc_sum,
            d_res_hand, d_res_heart, d_res_surplus, d_res_energy, d_fallback_moved,
            d_fallback_revealed, d_characters, d_plain_zonelen;
static long g_ht_any, g_ht_other;
static long g_pct_nodes, g_pct_activating, g_pct_selfmax, g_pct_revealed, g_pct_costtotal;
static long g_cmp_values_loc, g_cmp_loc, g_cmp_revealed_ct, g_energy_rel,
            g_plainloc_neg_revealed, g_loc_equality;

static void dump_ids(const Condition *c, const char *tag) {
    printf("ID|%s|card=%d|ab=%d|variant=%d|", tag, cur_cid, cur_abi, c->variant);
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (!f->key) continue;
        printf("%s", f->key);
        if (f->v.tag == RB_TAG_STR && f->v.s) printf("=%s", f->v.s);
        else if (f->v.tag == RB_TAG_TRUE) printf("=true");
        else if (f->v.tag == RB_TAG_FALSE) printf("=false");
        else if (f->v.tag == RB_TAG_I64) printf("=%lld", (long long)f->v.i);
        else if (f->v.tag == RB_TAG_ARRAY) printf("=[%u]", f->v.arr_n);
        printf(" ");
    }
    printf("\n");
}

static void walk_cond(const Condition *c);

static int loc_eq(const Condition *c, const char *s) {
    const char *l = fstr(c, "location");
    return l && !strcmp(l, s);
}

static void note_shape(const Condition *c) {
    const char *cty = fstr(c, "comparison_type");
    const char *loc  = fstr(c, "location");
    const char *rt   = fstr(c, "resource_type");
    const char *src  = fstr(c, "source");
    int has_loc = fval(c, "location") != NULL;
    int moved_src = src && (!strcmp(src, "preceding_moved") || !strcmp(src, "previous_moved_cards"));

    if (cty && !strcmp(cty, "score") && !has_loc && moved_src) d_score_moved++;
    if (cty && !strcmp(cty, "cost")) {
        if (!has_loc) d_cost_noloc++;
        else if (loc && !strcmp(loc, "revealed_cards")) d_cost_revealed++;
        else if (fstr(c, "comparison_target") && !strcmp(fstr(c, "comparison_target"), "self")) d_cost_self_max++;
        else if (loc && *loc) d_cost_loc_sum++;
    }
    if (rt) {
        if (!strcmp(rt, "hand_count")) d_res_hand++;
        else if (rt[0] == 'h' && rt[1] == 'e' && rt[2] == 'a' && rt[3] == 'r' && rt[4] == 't') d_res_heart++;
        else if (!strcmp(rt, "surplus_heart")) d_res_surplus++;
        else if (!strcmp(rt, "energy")) d_res_energy++;
    }
    if (!has_loc && moved_src) d_fallback_moved++;
    if (!has_loc) d_fallback_revealed++;
    if (fval(c, "characters")) d_characters++;
    if (!cty && !rt) d_plain_zonelen++;
}

/* D4: does this Comparison condition reach Rust's DOUBLE-negating numeric tail? */
static void note_comparison(const Condition *c) {
    const char *cty = fstr(c, "comparison_type");
    const char *pos = fstr(c, "position");
    b_cmp_total++;
    if (!ftrue(c, "negation")) return;
    b_cmp_neg++;

    /* route_comparison_condition (condition.rs:427-437) */
    int is_both    = fval(c, "values") != NULL && fval(c, "operator") == NULL;
    int is_highest = pos != NULL && fval(c, "count") == NULL
                     && fval(c, "position_compare") == NULL
                     && fval(c, "comparison_target") == NULL;
    int is_allcost = cty && !strcmp(cty, "cost") && pos == NULL && fval(c, "count") == NULL
                     && fval(c, "values") == NULL
                     && (!fstr(c, "comparison_target") || !strcmp(fstr(c,"comparison_target"), "self"))
                     && ftrue(c, "all");
    if (is_both)    { b_cmp_neg_both++;    return; }
    if (is_highest) { b_cmp_neg_highest++; return; }
    if (is_allcost) { b_cmp_neg_allcost++; return; }

    /* evaluate_comparison_condition early returns (no inner negation) */
    if (ftrue(c, "check_self"))       { b_cmp_early_return++; return; }
    if (ftrue(c, "paid_cost_this_turn")) { b_cmp_early_return++; return; }
    if (cty && !strcmp(cty, "energy_relative")) { b_cmp_early_return++; return; }
    if (cty && !strcmp(cty, "cost") && loc_eq(c, "activating_card")) { b_cmp_early_return++; return; }
    if (ftrue(c, "original_value") && cty && !strcmp(cty, "score")) { b_cmp_early_return++; return; }
    if (pos && !strcmp(pos, "front")) { b_cmp_early_return++; return; }
    if (pos && cty && !strcmp(cty, "cost")) { b_cmp_early_return++; return; }
    if (fval(c, "values")) { b_cmp_neg_values++; return; }        /* values tail, single negation */
    if (fval(c, "cost_total")) { b_cmp_neg_costtotal++; return; } /* cost_total tail, single */

    b_cmp_neg_generic++;   /* reaches comparison.rs:142-157 - DOUBLE negation */
    dump_ids(c, "D4_NEG_COMPARISON");
}

static void walk_cond(const Condition *c) {
    if (!c) return;
    n_nodes++;
    /* D8 key census */
    for (int i = 0; i < n_krows; i++) {
        if (fval(c, krows[i].k)) {
            krows[i].nodes++;
            krows[i].variants |= (1UL << (c->variant > 31 ? 31 : c->variant));
            if (cur_slot < 2200 && !kslot_seen[i][cur_slot]) {
                kslot_seen[i][cur_slot] = 1; krows[i].slots++;
            }
            if (krows[i].shown < 4) { krows[i].shown++; dump_ids(c, krows[i].k); }
        }
    }
    /* structural probes */
    if (c->variant == RB_COND_COMPARISON) {
        if (fval(c, "values") && fval(c, "location")) {
            g_cmp_values_loc++;
            if (g_cmp_values_loc <= 4) dump_ids(c, "X_CMP_VALUES_AND_LOCATION");
        }
        if (fval(c, "location")) g_cmp_loc++;
        if (fval(c, "card_type") && loc_eq(c, "revealed_cards")) {
            g_cmp_revealed_ct++;
            if (g_cmp_revealed_ct <= 4) dump_ids(c, "X_CMP_REVEALED_CARDTYPE");
        }
        const char *cty3 = fstr(c, "comparison_type");
        if (cty3 && !strcmp(cty3, "energy_relative")) g_energy_rel++;
    }
    if (c->variant == RB_COND_LOCATION) {
        const char *cty4 = fstr(c, "comparison_type");
        if (ftrue(c, "negation") && loc_eq(c, "revealed_cards")) {
            g_plainloc_neg_revealed++;
            dump_ids(c, "X_PLAINLOC_NEG_REVEALED");
        }
        if (cty4 && !strcmp(cty4, "equality")) {
            g_loc_equality++;
            if (g_loc_equality <= 4) dump_ids(c, "X_LOC_EQUALITY");
        }
    }

    /* D2/D3: heart_type "all" */
    const char *ht = fstr(c, "heart_type");
    if (ht && !strcmp(ht, "all")) {
        int is_plain_loc = (c->variant == RB_COND_LOCATION) && fval(c, "count") == NULL;
        int neg = ftrue(c, "negation");
        c_ht_all++;
        if (neg) c_ht_all_neg++;
        if (is_plain_loc) { c_ht_all_plainloc++; if (neg) c_ht_all_plainloc_neg++; }
        else c_ht_all_notplainloc++;
        if (loc_eq(c, "stage")) c_ht_all_locstage++;
        dump_ids(c, "D23_HEART_TYPE_ALL");
    }
    /* any heart_type at all, so "is 'all' common?" can be answered with a number */
    if (ht) { g_ht_any++; if (strcmp(ht, "all")) g_ht_other++; }

    if (c->variant == RB_COND_COMPARISON) {
        const char *cty2 = fstr(c, "comparison_type");
        if (cty2 && !strcmp(cty2, "cost") && loc_eq(c, "activating_card")) {
            g_pct_activating++;
            dump_ids(c, "D4_COST_ACTIVATING_CARD");
        }
        if (fval(c, "paid_cost_this_turn")) dump_ids(c, "D4_PAID_COST_THIS_TURN");
        if (cty2 && !strcmp(cty2, "energy_relative")) dump_ids(c, "D4_ENERGY_RELATIVE");
        if (fval(c, "cost_total")) {
            g_pct_costtotal++;
            dump_ids(c, "D4_COST_TOTAL");
        }
        const char *pos2 = fstr(c, "position");
        if (pos2 && !strcmp(pos2, "front")) dump_ids(c, "D4_POSITION_FRONT");
        if (ftrue(c, "original_value") && cty2 && !strcmp(cty2, "score"))
            dump_ids(c, "D4_ORIGINAL_VALUE_SCORE");
        note_comparison(c);
    }
    if (ftrue(c, "original_value")) g_pct_nodes++;

    /* D5: highest-cost-on-stage (is_highest_cost_on_stage, condition.rs:447-452) */
    if (c->variant == RB_COND_COMPARISON && fstr(c, "position") &&
        fval(c, "count") == NULL && fval(c, "position_compare") == NULL &&
        fval(c, "comparison_target") == NULL) {
        e_highest++;
        dump_ids(c, "D5_HIGHEST_COST");
        if (loc_eq(c, "stage") && !strcmp(fstr(c, "position"), "center")) e_highest_centerstage++;
    }

    /* D7 */
    if (c->variant == RB_COND_NO_EXCESS_HEART) {
        f_noexcess++;
        if (fval(c, "no_excess_heart") == NULL) f_noexcess_via_subchecks++;
        dump_ids(c, "D7_NO_EXCESS");
    }

    note_shape(c);

    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (!f->key) continue;
        if (f->v.tag == RB_TAG_OBJVAR || f->v.tag == RB_TAG_OBJECT) walk_cond(f->v.cond);
        if (f->v.tag == RB_TAG_ARRAY)
            for (uint32_t j = 0; j < f->v.arr_n; j++)
                if (f->v.arr[j].tag == RB_TAG_OBJVAR || f->v.arr[j].tag == RB_TAG_OBJECT)
                    walk_cond(f->v.arr[j].cond);
    }
}

static void walk_eff(const AbilityEffect *e) {
    if (!e) return;
    walk_cond(e->condition); walk_cond(e->result_condition);
    walk_cond(e->alternative_condition); walk_cond(e->activation_condition);
    for (int i = 0; i < e->n_child; i++) walk_eff(e->child[i]);
    for (int i = 0; i < e->n_options; i++) walk_eff(e->options[i]);
    walk_eff(e->primary_effect); walk_eff(e->alternative_effect);
    walk_eff(e->look_action); walk_eff(e->select_action);
    walk_eff(e->followup_action); walk_eff(e->optional_action);
    walk_eff(e->conditional_action); walk_eff(e->gained_effect);
    walk_eff(e->resource_on_select); walk_eff(e->opponent_action);
}

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) < 0) { fprintf(stderr, "rb_load failed\n"); return 2; }
    for (int i = 0; D8_KEYS[i]; i++) kfind(D8_KEYS[i]);
    n_slots = 0;
    for (uint32_t cid = 0; cid < 4000; cid++) {
        int n = rb_card_num_abilities(cid);
        if (n <= 0) { if (n < 0) break; continue; }
        for (int k = 0; k < n; k++) {
            Ability a; memset(&a, 0, sizeof(a));
            if (!rb_decode_card_ability(cid, k, &a)) continue;
            cur_slot = n_slots < 65535 ? n_slots : 65535;
            n_slots++;
            cur_cid = (int)cid; cur_abi = k;
            walk_eff(a.cost); walk_eff(a.effect);
            rb_free_ability(&a);
        }
    }
    n_abilities = n_slots;
    printf("abilities_walked=%ld condition_nodes=%ld\n", n_abilities, n_nodes);
    printf("=== A VARIANTS ===\n");
    printf("(variant census is covered by audit_t7_wire's CVAR rows)\n");
    printf("=== B D4 COMPARISON + NEGATION ===\n");
    printf("B_cmp_total=%ld\n", b_cmp_total);
    printf("B_cmp_negation=%ld\n", b_cmp_neg);
    printf("B_neg_routed_both=%ld\n", b_cmp_neg_both);
    printf("B_neg_routed_highest_cost=%ld\n", b_cmp_neg_highest);
    printf("B_neg_routed_all_cost=%ld\n", b_cmp_neg_allcost);
    printf("B_neg_early_return=%ld\n", b_cmp_early_return);
    printf("B_neg_values_tail=%ld\n", b_cmp_neg_values);
    printf("B_neg_cost_total_tail=%ld\n", b_cmp_neg_costtotal);
    printf("B_neg_GENERIC_DOUBLE_NEGATED=%ld\n", b_cmp_neg_generic);
    printf("=== C D2/D3 heart_type all ===\n");
    printf("C_heart_type_all=%ld  with_negation=%ld\n", c_ht_all, c_ht_all_neg);
    printf("C_plain_location=%ld  plain_loc_with_negation=%ld  not_plain_loc=%ld  loc_stage=%ld\n",
           c_ht_all_plainloc, c_ht_all_plainloc_neg, c_ht_all_notplainloc, c_ht_all_locstage);
    printf("=== D D6 get_count_for_condition shapes ===\n");
    printf("D_score_preceding_moved=%ld\n", d_score_moved);
    printf("D_cost_location_absent=%ld\n", d_cost_noloc);
    printf("D_cost_revealed_cards=%ld\n", d_cost_revealed);
    printf("D_cost_comparison_target_self_MAX=%ld\n", d_cost_self_max);
    printf("D_cost_named_location_sum=%ld\n", d_cost_loc_sum);
    printf("D_resource_hand_count=%ld\n", d_res_hand);
    printf("D_resource_heartNN=%ld\n", d_res_heart);
    printf("D_resource_surplus_heart=%ld\n", d_res_surplus);
    printf("D_resource_energy=%ld\n", d_res_energy);
    printf("D_moved_source_fallback=%ld\n", d_fallback_moved);
    printf("D_location_absent_total=%ld\n", d_fallback_revealed);
    printf("D_characters_arm=%ld\n", d_characters);
    printf("D_plain_zone_len=%ld\n", d_plain_zonelen);
    printf("=== E D5 highest cost on stage ===\n");
    printf("E_highest_cost_on_stage=%ld  center+stage=%ld\n", e_highest, e_highest_centerstage);
    printf("=== F D7 no excess heart ===\n");
    printf("F_no_excess_variant=%ld  without_no_excess_heart_key=%ld\n", f_noexcess, f_noexcess_via_subchecks);
    printf("=== G D8 per-key census (decoded condition tree) ===\n");
    printf("KEY|nodes|abilities|variants\n");
    for (int i = 0; i < n_krows; i++) {
        printf("G|%s|%ld|%ld|", krows[i].k, krows[i].nodes, krows[i].slots);
        if (!krows[i].nodes) printf("-\n");
        else { for (int v = 0; v <= 20; v++) if (krows[i].variants & (1UL << v)) printf("%d,", v); printf("\n"); }
    }
    printf("=== H extras ===\n");
    printf("H_heart_type_any=%ld heart_type_not_all=%ld\n", g_ht_any, g_ht_other);
    printf("H_original_value_nodes=%ld\n", g_pct_nodes);
    printf("H_cost_activating_card=%ld position_front=%ld cost_total=%ld\n",
           g_pct_activating, 0L, g_pct_costtotal);
    printf("=== I structural probes ===\n");
    printf("I_cmp_with_location=%ld\n", g_cmp_loc);
    printf("I_cmp_values_AND_location=%ld\n", g_cmp_values_loc);
    printf("I_cmp_cardtype_and_revealed_cards=%ld\n", g_cmp_revealed_ct);
    printf("I_cmp_energy_relative=%ld\n", g_energy_rel);
    printf("I_plainloc_negation_revealed_cards=%ld\n", g_plainloc_neg_revealed);
    printf("I_location_comparison_type_equality=%ld\n", g_loc_equality);
    printf("decode_fallback_count=%u\n", rb_decode_fallback_count());
    rb_unload();
    return 0;
}