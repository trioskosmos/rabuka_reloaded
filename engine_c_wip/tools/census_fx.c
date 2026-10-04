/* FIXER-WAVE census (2026-10-04) -- condition-node reachability for four items.
 *
 * Walks the SHIPPED bytecode through the engine's OWN decoder (rb_decode_card_ability),
 * the same way tools/audit_cluster_d.c does. No bytecode reader is reimplemented and
 * no number here comes from reading JSON.
 *
 *   A  item 3  card_property: distinct values (mapped vs UNMAPPED), variant/location
 *             split, negation split  -> is the empty-set early return reachable?
 *   B  item 1  eval_group temporal this_turn arm: the four Rust guards
 *             (temporal/self_target/group_names-nonempty/card_type==member_card),
 *             the C's looser superset, and the variant each lands on.
 *   C  item 2  movement values, and which position_change_events readers the corpus
 *             can reach (position_change / moves / area_direction / has_moved).
 *   D  item 5  top-level cost distinct values (for the r_distinct_type casing report).
 *
 * ASCII ONLY: the PowerShell -replace round-trip destroys non-ASCII bytes in this
 * repo (PARITY_CHECKLIST trap 4), so every comment here is deliberately plain.
 */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "rabuka.h"

/* -- A: card_property --------------------------------------------------- */
#define MAXPROP 32
#define MAXMV 256
#define MAXSC 16
typedef struct { const char *v; long nodes; long unmapped; } PropRow;
static PropRow props[MAXPROP];
static int n_props;
static long a_nodes, a_neg, a_mapped, a_unmapped, a_variant[32];
static long a_slot;
static long a_loc[24]; static const char *a_locname[24]; static int n_locs;

/* The decoded tree is freed by rb_free_ability at the end of each ability, so
   every string kept for reporting must be COPIED. Storing the char* itself
   leaves a dangling pointer and prints garbage (measured: 23 bogus
   card_property "values"). Fixed by copying into these fixed buffers. */
static char a_locbuf[24][48];
static char a_propbuf[MAXPROP][48];
static char a_mvbuf[MAXMV][64];
static char a_scbuf[MAXSC][48];
static void copy_fixed(char *dst, size_t cap, const char *s) {
    size_t i = 0; if (!s) { dst[0] = 0; return; }
    while (s[i] && i + 1 < cap) { dst[i] = s[i]; i++; }
    dst[i] = 0;
}

static PropRow *pfind(const char *k) {
    for (int i = 0; i < n_props; i++) if (!strcmp(props[i].v, k)) return &props[i];
    if (n_props >= MAXPROP) return &props[MAXPROP - 1];
    copy_fixed(a_propbuf[n_props], sizeof a_propbuf[0], k);
    props[n_props].v = a_propbuf[n_props];
    return &props[n_props++];
}
static int prop_mapped(const char *p) {
    return p && (!strcmp(p,"has_blade_heart") || !strcmp(p,"has_score_icon") ||
                 !strcmp(p,"has_all_blade")  || !strcmp(p,"has_draw_icon"));
}
static void note_loc(const char *l) {
    for (int i = 0; i < n_locs; i++) if (!strcmp(a_locname[i], l)) { a_loc[i]++; return; }
    if (n_locs >= 24) { a_loc[23]++; return; }
    copy_fixed(a_locbuf[n_locs], sizeof a_locbuf[0], l ? l : "(none)");
    a_locname[n_locs] = a_locbuf[n_locs];
    a_loc[n_locs] = 1; n_locs++;
}

/* -- B: eval_group temporal arm ----------------------------------------- */
static long b_group_total, b_group_this_turn_no_groups, b_arm_strict;
static long b_arm_no_cardtype, b_arm_with_cardtype_member, b_arm_with_cardtype_other;
static long b_arm_arr0_notstr, b_arm_c_loose, b_slot;

/* -- C: movement -------------------------------------------------------- */
typedef struct { const char *v; long nodes; } MvRow;
static MvRow mvs[MAXMV]; static int n_mvs;
static MvRow *mfind(const char *k) {
    for (int i = 0; i < n_mvs; i++) if (!strcmp(mvs[i].v, k)) return &mvs[i];
    if (n_mvs >= MAXMV) return &mvs[MAXMV - 1];   /* bounded, never overruns */
    copy_fixed(a_mvbuf[n_mvs], sizeof a_mvbuf[0], k);
    mvs[n_mvs].v = a_mvbuf[n_mvs];
    return &mvs[n_mvs++];
}
static long c_pc, c_moves, c_has_moved, c_not_moved, c_ad_from, c_ad_any, c_self_target_mv;
static long c_pc_self_target, c_pc_groups, c_pc_pos, c_pc_no_pos, c_pc_neg;
static long c_has_moved_group_names, c_pc_slot;

/* -- E: key presence over ALL condition nodes, and over the node classes the
      reader ports key on. Printed so "is this reader reachable" is a number. -- */
#define MAXKEYS 16
static const char *EKEYS[MAXKEYS] = {
    "cost_limit", "cost_limit_operator", "area_direction", "position",
    "self_effect_only", "energy_placed", "watches_area_move", "card_property",
    "group_names", "self_target", "target", "location", "negation", NULL
};
static long e_all[MAXKEYS];
static long e_pc[MAXKEYS];      /* on movement:position_change nodes */
static long e_moves[MAXKEYS];   /* on movement:moves nodes */
static int  e_pc_nodes, e_moves_nodes;
static long e_pc_posname;       /* position_change nodes naming a position */

static int cur_cid, cur_abi, cur_slot;
static int slot_seen[8][2200];

/* -- D: cost extras ----------------------------------------------------- */
static struct { const char *k; long n; } scs[MAXSC]; static int n_scs;
static void scfind(const char *k) {
    for (int i = 0; i < n_scs; i++) if (!strcmp(scs[i].k, k)) return;
    if (n_scs >= MAXSC) return;
    copy_fixed(a_scbuf[n_scs], sizeof a_scbuf[0], k);
    scs[n_scs].k = a_scbuf[n_scs];
    scs[n_scs].n = 0; n_scs++;
}
static long d_distinct_nodes;

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
static void dump(const char *tag, const Condition *c) {
    printf("ID|%s|card=%d|ab=%d|var=%d|", tag, cur_cid, cur_abi, c->variant);
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (!f->key) continue;
        printf("%s", f->key);
        if (f->v.tag == RB_TAG_STR && f->v.s) printf("=%s", f->v.s);
        else if (f->v.tag == RB_TAG_TRUE) printf("=true");
        else if (f->v.tag == RB_TAG_FALSE) printf("=false");
        else if (f->v.tag == RB_TAG_I64) printf("=%lld", (long long)f->v.i);
        else if (f->v.tag == RB_TAG_ARRAY) {
            printf("=[");
            for (uint32_t j = 0; j < f->v.arr_n; j++)
                if (f->v.arr[j].tag == RB_TAG_STR && f->v.arr[j].s) printf("%s,", f->v.arr[j].s);
            printf("]");
        }
        printf(" ");
    }
    printf("\n");
}

static void walk_cond(const Condition *c) {
    if (!c) return;
    int slot = cur_slot < 2200 ? cur_slot : 2199;

    /* -- A -- */
    const char *prop = fstr(c, "card_property");
    if (prop) {
        PropRow *r = pfind(prop);
        r->nodes++;
        a_nodes++;
        if (prop_mapped(prop)) a_mapped++; else { a_unmapped++; r->unmapped++; }
        if (ftrue(c, "negation")) a_neg++;
        a_variant[c->variant > 31 ? 31 : c->variant]++;
        const char *l = fstr(c, "location"); note_loc(l ? l : "(none)");
        if (!slot_seen[0][slot]) { slot_seen[0][slot] = 1; a_slot++; }
        if (a_nodes <= 6) dump("A_CARD_PROPERTY", c);
    }

    /* -- B -- */
    const char *temporal = fstr(c, "temporal");
    const CondValue *gn = fval(c, "group_names");
    int has_groups = gn && gn->tag == RB_TAG_ARRAY && gn->arr_n > 0;
    if (temporal && !strcmp(temporal, "this_turn") && ftrue(c, "self_target") && has_groups) {
        b_arm_strict++;
        if (!slot_seen[1][slot]) { slot_seen[1][slot] = 1; b_slot++; }
        const char *ct = fstr(c, "card_type");
        if (!ct) b_arm_no_cardtype++;
        else if (!strcmp(ct, "member_card")) b_arm_with_cardtype_member++;
        else b_arm_with_cardtype_other++;
        b_arm_arr0_notstr += (gn->arr[0].tag != RB_TAG_STR);
        dump("B_GROUP_THIS_TURN", c);
    }
    if (c->variant == RB_COND_GROUP) {
        b_group_total++;
        if (temporal && !strcmp(temporal, "this_turn") && ftrue(c, "self_target") && has_groups)
            b_arm_c_loose++;
        else if (temporal && !strcmp(temporal, "this_turn") && ftrue(c, "self_target")) {
            b_group_this_turn_no_groups++;
            dump("B_GROUP_THIS_TURN_NO_GROUPS", c);
        }
    }

    /* -- C -- */
    const char *mv = fstr(c, "movement");
    if (!mv) mv = fstr(c, "source");
    if (mv) mfind(mv)->nodes++;
    if (mv && !strcmp(mv, "position_change")) {
        c_pc++;
        if (!slot_seen[2][slot]) { slot_seen[2][slot] = 1; c_pc_slot++; }
        if (ftrue(c, "self_target")) c_pc_self_target++;
        if (has_groups) c_pc_groups++;
        if (fval(c, "position")) c_pc_pos++; else c_pc_no_pos++;
        if (ftrue(c, "negation")) c_pc_neg++;
        if (c_pc <= 40) dump("C_POSITION_CHANGE", c);
    }
    if (mv && !strcmp(mv, "moves")) c_moves++;
    if (mv && !strcmp(mv, "has_moved")) { c_has_moved++; if (has_groups) c_has_moved_group_names++; }
    if (mv && !strcmp(mv, "not_moved")) c_not_moved++;
    const char *ad = fstr(c, "area_direction");
    if (ad) { if (!strcmp(ad, "from")) c_ad_from++; c_ad_any++; }
    if (ftrue(c, "self_target") && (mv || ad)) c_self_target_mv++;

    /* -- E -- */
    { int is_pc = (mv && !strcmp(mv, "position_change"));
      int is_mv = (mv && !strcmp(mv, "moves"));
      if (is_pc) e_pc_nodes++;
      if (is_mv) e_moves_nodes++;
      for (int q = 0; EKEYS[q]; q++) {
          if (!fval(c, EKEYS[q])) continue;
          e_all[q]++;
          if (is_pc) e_pc[q]++;
          if (is_mv) e_moves[q]++;
      }
      if (is_pc && fstr(c, "position")) e_pc_posname++;
    }

    /* nested temporal this_turn -> has_moved / not_moved (state.rs:91-181) */
    if (temporal && !strcmp(temporal, "this_turn")) {
        const CondValue *nc = fval(c, "condition");
        if (nc && (nc->tag == RB_TAG_OBJVAR || nc->tag == RB_TAG_OBJECT) && nc->cond) {
            const char *nm = fstr(nc->cond, "movement");
            if (nm && !strcmp(nm, "has_moved")) c_has_moved++;
            if (nm && !strcmp(nm, "not_moved")) c_not_moved++;
        }
    }

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

/* -- D: cost extras -- */
static const char *cost_extra(const AbilityEffect *e, const char *key) {
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}
static int ftrue_cost(const AbilityEffect *e, const char *key) {
    const char *s = cost_extra(e, key);
    if (!s) return 0;
    return !strcmp(s, "true") || !strcmp(s, "1");
}

/* `depth` distinguishes a TOP-LEVEL cost from a cost buried in an effect tree:
   the project has been burned three times by counting only the top level and
   reporting "0 of N" when the truth was "2 of M". */
/* -- G: B3 SECOND method, at the ABILITY level (not the cost level).
      choice_cost_opens_hand_prompt is reached ONLY from choice_debut_cost_action,
      which requires ab.triggers to contain 登場 or バトンタッチ. So the honest
      denominator is "debut-triggered hand costs", not "hand costs". -- */
static long g_debut_abilities, g_debut_hand_costs, g_debut_same_unit,
            g_debut_all, g_debut_any_number, g_debut_cost_actions[8];
static long d_distinct_nodes, d_distinct_top;

/* -- H: the triggers.c:1189-1197 SECOND DISPATCHER. Measured here rather than
      trusted: the brief's "0 of 275 constant abilities" is cross-checked against
      the WHOLE corpus effect tree, which is a superset of the constant-ability
      set and so cannot understate it. -- */
static long h_look_at, h_reveal, h_reveal_per_group, h_reveal_until_live,
            h_reveal_until_chosen, h_reveal_until_target;
static long h_constant_abilities, h_constant_effect_nodes, h_constant_look_at,
            h_constant_reveal, h_constant_reveal_per_group, h_constant_reveal_until_target;
static int  in_constant;

/* -- F: B3 -- the hand-cost shape choice_cost_opens_hand_prompt discriminates.
      Counts cost nodes with source=="hand" that carry each of the keys the
      function's three early-exit tests read. -- */
static long f_hand_costs, f_hand_same_unit, f_hand_all, f_hand_any_number,
            f_hand_optional, f_hand_cost_counts[8];

static void walk_eff_d(const AbilityEffect *e, int top) {
    if (!e) return;
    walk_cond(e->condition); walk_cond(e->result_condition);
    walk_cond(e->alternative_condition); walk_cond(e->activation_condition);
    { const char *d = cost_extra(e, "distinct");
      if (d) { scfind(d); for (int i = 0; i < n_scs; i++)
                  if (!strcmp(scs[i].k, d)) { scs[i].n++; break; }
               d_distinct_nodes++; if (top) d_distinct_top++; }
    /* -- F -- */
    { const char *a = e->action;
      int is_look = a && !strcmp(a, "look_at");
      int is_rev  = a && !strcmp(a, "reveal");
      int is_rpg  = a && !strcmp(a, "reveal_per_group");
      int is_rut  = a && !strcmp(a, "reveal_until_target");
      int is_rul  = a && !strcmp(a, "reveal_until_live_card");
      int is_ruc  = a && !strcmp(a, "reveal_until_chosen_card");
      if (is_look) h_look_at++;
      if (is_rev)  h_reveal++;
      if (is_rpg)  h_reveal_per_group++;
      if (is_rut)  h_reveal_until_target++;
      if (is_rul)  h_reveal_until_live++;
      if (is_ruc)  h_reveal_until_chosen++;
      if (in_constant) {
          h_constant_effect_nodes++;
          if (is_look) h_constant_look_at++;
          if (is_rev)  h_constant_reveal++;
          if (is_rpg)  h_constant_reveal_per_group++;
          if (is_rut)  h_constant_reveal_until_target++;
      }
    }
        if (e->source && !strcmp(e->source, "hand")) {
        f_hand_costs++;
        if (ftrue_cost(e, "same_unit_name")) {
            f_hand_same_unit++;
            { Card cc; memset(&cc, 0, sizeof cc);
              const char *cno = "-";
              if (rb_decode_card_by_index((uint32_t)cur_cid, &cc)) {
                  const char *s = rb_card_string(cc.card_no_idx);
                  if (s) cno = s;
                  rb_free_card(&cc);
              }
              printf("F_SAME_UNIT|card=%d|no=%s|ab=%d|action=%s|count=%d|optional=%d\n",
                     cur_cid, cno, cur_abi, e->action ? e->action : "-", e->count, e->is_optional);
            }
        }
        if (ftrue_cost(e, "all"))           f_hand_all++;
        if (ftrue_cost(e, "any_number"))    f_hand_any_number++;
        if (e->is_optional)                 f_hand_optional++;
        int cc = e->count > 0 ? e->count : 1;
        if (cc < 8) f_hand_cost_counts[cc]++;
    } }
    for (int i = 0; i < e->n_child; i++) walk_eff_d(e->child[i], 0);
    for (int i = 0; i < e->n_options; i++) walk_eff_d(e->options[i], 0);
    walk_eff_d(e->primary_effect, 0); walk_eff_d(e->alternative_effect, 0);
    walk_eff_d(e->look_action, 0); walk_eff_d(e->select_action, 0);
    walk_eff_d(e->followup_action, 0); walk_eff_d(e->optional_action, 0);
    walk_eff_d(e->conditional_action, 0); walk_eff_d(e->gained_effect, 0);
    walk_eff_d(e->resource_on_select, 0); walk_eff_d(e->opponent_action, 0);
}
static void walk_eff(const AbilityEffect *e) { walk_eff_d(e, 0); }

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) < 0) { fprintf(stderr, "rb_load failed\n"); return 2; }
    long n_slots = 0;
    memset(slot_seen, 0, sizeof(slot_seen));
    for (uint32_t cid = 0; cid < 4000; cid++) {
        int n = rb_card_num_abilities(cid);
        if (n <= 0) { if (n < 0) break; continue; }
        for (int k = 0; k < n; k++) {
            Ability a; memset(&a, 0, sizeof(a));
            if (!rb_decode_card_ability(cid, k, &a)) continue;
            cur_cid = (int)cid; cur_abi = k;
            cur_slot = n_slots < 2200 ? (int)n_slots : 2199;
            n_slots++;
            walk_eff_d(a.cost, 1);
            { int is_const = a.triggers && strstr(a.triggers, "常時");
              if (is_const) { h_constant_abilities++; in_constant = 1; }
              walk_eff_d(a.effect, 0);
              in_constant = 0; }
            /* -- G: the only caller requires a debut trigger -- */
            if (a.triggers && (strstr(a.triggers, "登場") || strstr(a.triggers, "バトンタッチ"))) {
                g_debut_abilities++;
                if (a.cost && a.cost->source && !strcmp(a.cost->source, "hand")) {
                    g_debut_hand_costs++;
                    int cc = a.cost->count > 0 ? a.cost->count : 1;
                    if (cc < 8) g_debut_cost_actions[cc]++;
                    if (ftrue_cost(a.cost, "same_unit_name")) g_debut_same_unit++;
                    if (ftrue_cost(a.cost, "all"))           g_debut_all++;
                    if (ftrue_cost(a.cost, "any_number"))    g_debut_any_number++;
                    printf("G_DEBUT_HAND_COST|card=%d|ab=%d|action=%s|count=%d|opt=%d|same_unit=%d|all=%d|any_number=%d\n",
                           (int)cid, k, a.cost->action ? a.cost->action : "-", a.cost->count,
                           a.cost->is_optional, ftrue_cost(a.cost, "same_unit_name"),
                           ftrue_cost(a.cost, "all"), ftrue_cost(a.cost, "any_number"));
                }
            }
            rb_free_ability(&a);
        }
    }
    printf("abilities_walked=%ld\n", n_slots);
    printf("=== A card_property (item 3) ===\n");
    printf("A_nodes=%ld mapped=%ld UNMAPPED=%ld with_negation=%ld slots=%ld\n",
           a_nodes, a_mapped, a_unmapped, a_neg, a_slot);
    printf("A_by_variant:"); for (int v = 0; v < 32; v++) if (a_variant[v]) printf(" %d=%ld", v, a_variant[v]); printf("\n");
    printf("A_by_location:"); for (int i = 0; i < n_locs; i++) printf(" %s=%ld", a_locname[i], a_loc[i]); printf("\n");
    printf("A_distinct_values=%d\n", n_props);
    for (int i = 0; i < n_props; i++)
        printf("A_prop|%s|nodes=%ld|unmapped=%ld\n", props[i].v, props[i].nodes, props[i].unmapped);
    printf("=== B eval_group temporal this_turn (item 1) ===\n");
    printf("B_group_variant_nodes=%ld  this_turn_self_target_NO_groups=%ld\n",
           b_group_total, b_group_this_turn_no_groups);
    printf("B_RUST_ARM_REACHABLE=%ld (slots=%ld)\n", b_arm_strict, b_slot);
    printf("B_no_cardtype=%ld cardtype_member_card=%ld cardtype_other=%ld arr0_not_str=%ld\n",
           b_arm_no_cardtype, b_arm_with_cardtype_member, b_arm_with_cardtype_other, b_arm_arr0_notstr);
    printf("B_C_LOOSE_REACHABLE=%ld\n", b_arm_c_loose);
    printf("=== C movement / position_change_events (item 2) ===\n");
    printf("C_position_change=%ld (slots=%ld) self_target=%ld groups=%ld pos=%ld no_pos=%ld neg=%ld\n",
           c_pc, c_pc_slot, c_pc_self_target, c_pc_groups, c_pc_pos, c_pc_no_pos, c_pc_neg);
    printf("C_moves=%ld has_moved=%ld has_moved_with_groups=%ld not_moved=%ld\n",
           c_moves, c_has_moved, c_has_moved_group_names, c_not_moved);
    printf("C_area_direction=%ld (from=%ld) self_target_with_movement=%ld\n",
           c_ad_any, c_ad_from, c_self_target_mv);
    printf("C_movement_literals=%d\n", n_mvs);
    for (int i = 0; i < n_mvs; i++) printf("C_mv|%s|%ld\n", mvs[i].v, mvs[i].nodes);
    printf("=== D cost distinct (item 5 casing) ===\n");
    printf("D_distinct_nodes=%ld TOP_LEVEL_COST=%ld literals=%d\n",
           d_distinct_nodes, d_distinct_top, n_scs);
    for (int i = 0; i < n_scs; i++) printf("D_distinct|%s|%ld\n", scs[i].k, scs[i].n);
    printf("=== F B3 hand-cost shape (choice_cost_opens_hand_prompt) ===\n");
    printf("F_hand_sourced_cost_nodes=%ld same_unit_name=%ld all=%ld any_number=%ld is_optional=%ld\n",
           f_hand_costs, f_hand_same_unit, f_hand_all, f_hand_any_number, f_hand_optional);
    printf("F_count_distribution:");
    for (int i = 0; i < 8; i++) if (f_hand_cost_counts[i]) printf(" c%d=%ld", i, f_hand_cost_counts[i]);
    printf("\n");
    printf("=== G B3 second method: debut-triggered costs ===\n");
    printf("G_debut_abilities=%ld debut_hand_costs=%ld debut_same_unit_name=%ld debut_all=%ld debut_any_number=%ld\n",
           g_debut_abilities, g_debut_hand_costs, g_debut_same_unit, g_debut_all, g_debut_any_number);
    printf("G_debut_hand_cost_count_distribution:");
    for (int i = 0; i < 8; i++) if (g_debut_cost_actions[i]) printf(" c%d=%ld", i, g_debut_cost_actions[i]);
    printf("\n");
    printf("=== H triggers.c second dispatcher ===\n");
    printf("H_whole_corpus_effect_nodes: look_at=%ld reveal=%ld reveal_per_group=%ld reveal_until_target=%ld reveal_until_live_card=%ld reveal_until_chosen_card=%ld\n",
           h_look_at, h_reveal, h_reveal_per_group, h_reveal_until_target,
           h_reveal_until_live, h_reveal_until_chosen);
    printf("H_constant_abilities=%ld constant_effect_nodes=%ld | within them: look_at=%ld reveal=%ld reveal_per_group=%ld reveal_until_target=%ld\n",
           h_constant_abilities, h_constant_effect_nodes, h_constant_look_at,
           h_constant_reveal, h_constant_reveal_per_group, h_constant_reveal_until_target);
    printf("decode_fallback_count=%u\n", rb_decode_fallback_count());
    printf("=== E key presence ===\n");
    printf("E_position_change_nodes=%ld  E_moves_nodes=%ld  E_pc_naming_position=%ld\n",
           e_pc_nodes, e_moves_nodes, e_pc_posname);
    printf("E_key|all_nodes|on_position_change|on_moves\n");
    for (int q = 0; EKEYS[q]; q++)
        printf("E|%s|%ld|%ld|%ld\n", EKEYS[q], e_all[q], e_pc[q], e_moves[q]);
    rb_unload();
    return 0;
}