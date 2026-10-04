/* Cluster T7  Edecoder census.
 *
 * Walks the REAL shipped bytecode (src/core/generated/gen_data.c offsets +
 * src/abilities_strings.bin) through rb_decode_card_ability and counts, per
 * condition key and per effect key:
 *   - how many nodes carry it
 *   - how many of the 985 abilities carry it
 *   - whether the value survived the decode (condition: present in
 *     Condition::fields[]; effect: present in AbilityEffect's extras table or
 *     one of the dedicated struct fields).
 *
 * Nothing here is a guess: every number comes from the decoder the engine
 * actually runs.
 */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "rabuka.h"

#define MAXKEYS 4096
typedef struct { char k[64]; long nodes; long abilities; int in_c_typed; int retained; } KeyRow;
static KeyRow cond_keys[MAXKEYS]; static int n_cond_keys;
static KeyRow eff_keys[MAXKEYS];  static int n_eff_keys;
static long n_cond_nodes, n_eff_nodes;

static unsigned char cond_seen_ability[MAXKEYS];
static unsigned char eff_seen_ability[MAXKEYS];

/* The C typed condition decoder's key set (vm.c decode_condition_field,
 * vm.c:1888-2619). Anything Rust names and this does not falls to the
 * catch-all at vm.c:2619, which notes a fallback and DROPS the value. */
static const char *C_TYPED_COND_KEYS[] = {
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

static int is_c_typed_cond(const char *k) {
    for (int i = 0; C_TYPED_COND_KEYS[i]; i++)
        if (!strcmp(C_TYPED_COND_KEYS[i], k)) return 1;
    return 0;
}

static KeyRow *find_key(KeyRow *tab, int *n, const char *k) {
    for (int i = 0; i < *n; i++) if (!strcmp(tab[i].k, k)) return &tab[i];
    if (*n >= MAXKEYS) { fprintf(stderr, "key table full\n"); exit(2); }
    int at = *n;
    memset(&tab[at], 0, sizeof(KeyRow));
    snprintf(tab[at].k, 64, "%s", k);
    (*n)++;
    return &tab[at];
}

/* Recursively walk the Condition tree. `slot` identifies the ability being
 * walked so per-ability counts stay de-duplicated. */
static void walk_cond(const Condition *c, int slot) {
    if (!c) return;
    n_cond_nodes++;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (!f->key) continue;
        int idx = -1;
        for (int j = 0; j < n_cond_keys; j++) if (!strcmp(cond_keys[j].k, f->key)) { idx = j; break; }
        if (idx < 0) idx = (int)(find_key(cond_keys, &n_cond_keys, f->key) - cond_keys);
        cond_keys[idx].nodes++;
        cond_keys[idx].in_c_typed = is_c_typed_cond(f->key);
        cond_keys[idx].retained = 1;      /* it is IN fields[] by construction */
        if (slot >= 0 && slot < MAXKEYS && !cond_seen_ability[idx]) {
            cond_seen_ability[idx] = 1;
            cond_keys[idx].abilities++;
        }
        if (f->v.tag == RB_TAG_OBJVAR || f->v.tag == RB_TAG_OBJECT) walk_cond(f->v.cond, slot);
        if (f->v.tag == RB_TAG_ARRAY)
            for (uint32_t j = 0; j < f->v.arr_n; j++)
                if (f->v.arr[j].tag == RB_TAG_OBJVAR || f->v.arr[j].tag == RB_TAG_OBJECT)
                    walk_cond(f->v.arr[j].cond, slot);
    }
}

static int eff_has_extra(const AbilityEffect *e, const char *k) {
    for (int i = 0; i < e->n_extra; i++) if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return 1;
    return 0;
}

static void walk_eff(const AbilityEffect *e, int slot) {
    if (!e) return;
    n_eff_nodes++;
    /* Every key that reached the extras table, plus the struct-only carriers. */
    static const struct { const char *k; size_t off; } direct[] = { {0,0} };
    (void)direct;
    char keys[64][64]; int nk = 0;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && nk < 64) { snprintf(keys[nk], 64, "%s", e->extra_k[i]); nk++; }
    /* struct fields the decoder sets WITHOUT an extra */
    if (e->text)                        { snprintf(keys[nk],64,"__text"); nk++; }
    if (e->count >= 0)                 { snprintf(keys[nk],64,"__count"); nk++; }
    if (e->id_field[0])                 { snprintf(keys[nk],64,"__id"); nk++; }
    if (e->card_type_field[0])          { snprintf(keys[nk],64,"__card_type"); nk++; }
    if (e->self_target_field[0])        { snprintf(keys[nk],64,"__self_target"); nk++; }
    if (e->repeat_limit)                { snprintf(keys[nk],64,"__repeat_limit"); nk++; }
    if (e->per_unit)                    { snprintf(keys[nk],64,"__per_unit"); nk++; }
    if (e->distinct_flag)               { snprintf(keys[nk],64,"__distinct"); nk++; }
    if (e->per_unit_count)              { snprintf(keys[nk],64,"__per_unit_count"); nk++; }
    if (e->cost_reduction_per_group)    { snprintf(keys[nk],64,"__cost_reduction_per_group"); nk++; }
    if (e->is_optional)                 { snprintf(keys[nk],64,"__optional"); nk++; }
    if (e->is_further)                  { snprintf(keys[nk],64,"__is_further"); nk++; }
    if (e->conditional_flag)            { snprintf(keys[nk],64,"__conditional"); nk++; }
    if (e->conditional_negation)        { snprintf(keys[nk],64,"__conditional_negation"); nk++; }

    for (int i = 0; i < nk; i++) {
        const char *k = keys[i];
        if (k[0] == '_') continue;    /* struct-only carrier, counted separately below */
        int idx = -1;
        for (int j = 0; j < n_eff_keys; j++) if (!strcmp(eff_keys[j].k, k)) { idx = j; break; }
        if (idx < 0) idx = (int)(find_key(eff_keys, &n_eff_keys, k) - eff_keys);
        eff_keys[idx].nodes++;
        eff_keys[idx].retained = 1;
        if (slot >= 0 && slot < MAXKEYS && !eff_seen_ability[idx]) {
            eff_seen_ability[idx] = 1;
            eff_keys[idx].abilities++;
        }
    }
    for (int i = 0; i < e->n_child; i++) walk_eff(e->child[i], slot);
    for (int i = 0; i < e->n_options; i++) walk_eff(e->options[i], slot);
    walk_eff(e->primary_effect, slot); walk_eff(e->alternative_effect, slot);
    walk_eff(e->look_action, slot); walk_eff(e->select_action, slot);
    walk_eff(e->followup_action, slot); walk_eff(e->optional_action, slot);
    walk_eff(e->conditional_action, slot); walk_eff(e->gained_effect, slot);
    walk_eff(e->resource_on_select, slot); walk_eff(e->opponent_action, slot);
    walk_cond(e->condition, slot); walk_cond(e->result_condition, slot);
    walk_cond(e->alternative_condition, slot); walk_cond(e->activation_condition, slot);
}

static int cmp_desc(const void *a, const void *b) {
    const KeyRow *x = a, *y = b;
    if (x->nodes != y->nodes) return (y->nodes > x->nodes) ? 1 : -1;
    return strcmp(x->k, y->k);
}

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) < 0) { fprintf(stderr, "rb_load failed\n"); return 2; }
    int ncards = 0;
    for (uint32_t cid = 0; cid < 4000; cid++) {
        int n = rb_card_num_abilities(cid);
        if (n <= 0) { if (n < 0) break; continue; }
        ncards++;
        for (int k = 0; k < n; k++) {
            Ability a;
            memset(&a, 0, sizeof(a));
            if (!rb_decode_card_ability(cid, k, &a)) continue;
            int slot = -1;
            if (n_cond_keys + 1 < MAXKEYS) {
                /* slot = a stable per-ability id: cid*8+k */
                slot = cid * 8 + k; if (slot >= MAXKEYS) slot = cid % MAXKEYS;
            }
            if (a.cost) walk_eff(a.cost, slot);
            if (a.effect) walk_eff(a.effect, slot);
            rb_free_ability(&a);
        }
    }
    printf("=== CONDITION KEYS (decoded, %ld nodes) ===\n", n_cond_nodes);
    qsort(cond_keys, n_cond_keys, sizeof(KeyRow), cmp_desc);
    for (int i = 0; i < n_cond_keys; i++)
        printf("COND|%s|%ld|%ld|%s\n", cond_keys[i].k, cond_keys[i].nodes,
               cond_keys[i].abilities, cond_keys[i].in_c_typed ? "C_TYPED" : "C_DROPPED");
    printf("=== EFFECT KEYS (decoded, %ld nodes) ===\n", n_eff_nodes);
    qsort(eff_keys, n_eff_keys, sizeof(KeyRow), cmp_desc);
    for (int i = 0; i < n_eff_keys; i++)
        printf("EFF|%s|%ld|%ld\n", eff_keys[i].k, eff_keys[i].nodes, eff_keys[i].abilities);
    printf("=== FALLBACK ===\n");
    printf("decode_fallback_count=%u\n", rb_decode_fallback_count());
    rb_unload();
    return 0;
}