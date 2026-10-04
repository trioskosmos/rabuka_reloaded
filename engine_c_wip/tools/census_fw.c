/* Final-wave corpus census. Decodes every card ability with the ENGINE'S OWN
 * decoder (rb_decode_card_ability) and tallies exactly what the cluster C / F /
 * T2 items depend on. Nothing here re-implements the bytecode reader.
 *
 * Questions answered:
 *   CARDTYPE  - how many COST nodes carry `card_type` in extras vs
 *               card_type_field, per cost kind (the ten cost.c sites).
 *   DISTINCT  - how many costs carry `distinct` (C3).
 *   CHANGEST  - state_change literals on change_state costs (C4).
 *   YELLICONS - how many effects carry the yell-revealed-icon shape (F5).
 *   GAINHEART - gain_heart nodes and their per-colour shape (F2).
 *   CUSTOM    - `custom` actions (F1).
 *   RESTRICT  - `restriction` nodes and changed_state_members presence (T2-10).
 *   POSITION  - position_change nodes (T2-15).
 *   EXCLSELF  - change_state nodes carrying exclude_self (T2-16).
 */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "rabuka.h"

/* ---- cost-kind discriminators, copied verbatim from cost.c so the census
   classifies a node the same way the dispatcher does. ---- */
static int c_is_energy(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "pay_energy")) return 1;
    const char *t = e->target;
    return t && strstr(t, "energy") != NULL;
}
static int c_is_change_state(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "change_state")) return 1;
    const char *t = e->target;
    return t && strstr(t, "wait") != NULL;
}
static int c_is_move_cards(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "move_cards")) return 1;
    const char *t = e->target;
    return t && strstr(t, "move_cards") != NULL;
}
static int c_is_reveal(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "reveal")) return 1;
    const char *t = e->target;
    return t && strstr(t, "reveal") != NULL;
}
static const char *xget(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

static long n_pairs, n_ability, n_cost_nodes, n_eff_nodes;

/* COST-site tallies, indexed by a site id so the report can name the site. */
enum { SITE_REVEAL = 0, SITE_MOVE, SITE_CHANGEST, SITE_MAX };
static const char *SITE_NAME[SITE_MAX] = { "pay_cost_inner/Reveal", "pay_cost_move_cards",
                                            "pay_cost_change_state" };
static long ct_field[SITE_MAX], ct_extra[SITE_MAX], cost_n[SITE_MAX];

static long n_distinct_cost;
static char dist_shape[32][150];
static long n_changestate, cs_wait, cs_rest, cs_rested, cs_other;
static char cs_other_v[64][48]; static int n_cs_other;
static long n_yellicons, n_custom, n_restriction, n_restrict_csm, n_position, n_exclself;
static long n_gainheart, n_gainheart_hearts;
static long n_custom_has_text;

/* every distinct (action,extra-key) pair, so a literal I guessed wrong is visible
   instead of silently counting zero. */
#define MAXX 512
static char ax_act[MAXX][64], ax_key[MAXX][64];
static long ax_has[MAXX];
static int n_ax;
static void bump_ax(const char *a, const char *k) {
    char buf[64]; snprintf(buf, 64, "%s", a ? a : "(none)");
    char bk[64];  snprintf(bk,  64, "%s", k ? k : "(none)");
    for (int i = 0; i < n_ax; i++)
        if (!strcmp(ax_act[i], buf) && !strcmp(ax_key[i], bk)) { ax_has[i]++; return; }
    if (n_ax >= MAXX) return;
    memcpy(ax_act[n_ax], buf, 64); memcpy(ax_key[n_ax], bk, 64);
    ax_has[n_ax] = 1; n_ax++;
}
/* keys seen on nodes whose action contains a needle, printed verbatim */
#define MAYY 256
static char yn_act[MAYY][64], yn_key[MAYY][64], yn_val[MAYY][64];
static long yn_has[MAYY];
static int n_yn;
static void note_key(const char *a, const char *k, const char *v) {
    if (n_yn >= MAYY) return;
    snprintf(yn_act[n_yn], 64, "%s", a ? a : "(none)");
    snprintf(yn_key[n_yn], 64, "%s", k ? k : "(none)");
    snprintf(yn_val[n_yn], 64, "%s", v ? v : "(null)");
    yn_has[n_yn]++; n_yn++;
}

/* C1: top-level costs on a WHOLE-ability basis where validate_mandatory_cost and
 * validate_cost provably disagree, i.e. validate_mandatory_cost returns 1 without
 * consulting the leaf (handlers.rs:286-306): optional / any_number / max, a
 * sequential_cost, a choice_condition, or a pay_energy. */
static int mandatory_cost_exempt(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->is_optional) return 1;
    if (xget(e, "any_number") || xget(e, "max")) return 1;
    if (!e->action) return 0;
    return !strcmp(e->action, "sequential_cost") ||
           !strcmp(e->action, "sequential")     ||
           !strcmp(e->action, "choice_condition") ||
           !strcmp(e->action, "pay_energy");
}
#define MAXCAND 64
static char cand_no[MAXCAND][64], cand_shape[MAXCAND][80];
static int n_cand;
static char act_shape[512][150];
static int n_act;
static int n_act_seq_exempt;
/* C1 divergence, precisely: a mandatory_cost-exempt node reachable ONLY through
 * a sequential_cost descent, i.e. validate_cost descends into it and
 * validate_leaf_cost_for can refuse, while validate_mandatory_cost skips it
 * (handlers.rs:287-292) or passes it (:303 ChoiceCondition / :306 PayEnergy). */
static int exempt_leaf(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->is_optional || xget(e, "any_number") || xget(e, "max")) return 1;
    if (!e->action) return 0;
    return !strcmp(e->action, "choice_condition") || !strcmp(e->action, "pay_energy");
}
static int seq_has_exempt(const AbilityEffect *e, int depth) {
    if (!e || depth > 6) return 0;
    for (int i = 0; i < e->n_child; i++)
        if (exempt_leaf(e->child[i]) || seq_has_exempt(e->child[i], depth + 1)) return 1;
    return 0;
}
static int seq_count_exempt(const AbilityEffect *e, int depth) {
    if (!e || depth > 6) return 0;
    int n = 0;
    for (int i = 0; i < e->n_child; i++)
        n += exempt_leaf(e->child[i]) + seq_count_exempt(e->child[i], depth + 1);
    return n;
}
static void note_activate_candidate(const char *no, int cid, int ai,
                                    const char *trig, const AbilityEffect *cost) {
    if (n_act >= 512) return;
    /* 起動 = U+8D77 U+52D5 */
    if (!trig || !strstr(trig, "\xe8\xb5\xb7\xe5\x8b\x95")) return;
    if (cost && seq_has_exempt(cost, 0)) n_act_seq_exempt++;
    snprintf(act_shape[n_act], 150, "%-20s cid=%-5d ab#%d trig=%.22s cost=%-16s opt=%d src=%-8s cnt=%-3d ctype=%s seq_exempt=%d",
             no, cid, ai, trig, cost && cost->action ? cost->action : "-",
             cost ? cost->is_optional : 0,
             cost && cost->source ? cost->source : "-",
             cost ? cost->count : -1,
             cost && cost->card_type_field[0] ? cost->card_type_field : "-",
             cost ? seq_count_exempt(cost, 0) : 0);
    n_act++;
}
/* C4: every distinct `state_change` literal, over COST nodes and EFFECT nodes
 * alike, so "0 rest / 0 rested" is a measurement and not an assumption. */
static char sc_val[64][48];
static long sc_has[64];
static int n_sc;
static void bump_sc(const char *v) {
    char b[48]; snprintf(b, 48, "%s", v ? v : "(null)");
    for (int i = 0; i < n_sc; i++) if (!strcmp(sc_val[i], b)) { sc_has[i]++; return; }
    if (n_sc >= 64) return;
    memcpy(sc_val[n_sc], b, 48); sc_has[n_sc] = 1; n_sc++;
}
/* T2-13: `operation: "set"` on a live_total-targeting modify_score. Rust
 * score.rs:274-287 returns Ok(()) for it -- applies NOTHING. Counted by the
 * reachability rule the item is written under: the corpus total, and the subset
 * that actually targets live_total. */
static long n_ms_set, n_ms_live, n_ms_live_set;
static char ms_set_shape[32][120];
static int n_ms_set_shape;
static void note_ms(const AbilityEffect *e) {
    if (!e || !e->action || strcmp(e->action, "modify_score")) return;
    const char *op = xget(e, "operation");
    int live = (e->target && strstr(e->target, "live_total")) ? 1 : 0;
    if (live) n_ms_live++;
    if (op && !strcmp(op, "set")) {
        n_ms_set++;
        if (live) n_ms_live_set++;
        if (n_ms_set_shape < 32)
            snprintf(ms_set_shape[n_ms_set_shape++], 120,
                     "op=set target=%s count=%d live=%d card_type=%s text=%.40s",
                     e->target ? e->target : "(none)", e->count, live,
                     e->card_type_field, e->text ? e->text : "-");
    }
}
static int want_keys(const char *a) {
    if (!a) return 0;
    if (strstr(a, "custom")) return 1;
    if (strstr(a, "heart"))   return 1;
    if (strstr(a, "yell"))    return 1;
    if (strstr(a, "icon"))    return 1;
    if (strstr(a, "restrict"))return 1;
    if (strstr(a, "position"))return 1;
    if (strstr(a, "draw"))    return 1;
    return 0;
}

static void bump_cs_other(const char *v) {
    const char *s = v ? v : "(null)";
    for (int i = 0; i < n_cs_other; i++) if (!strcmp(cs_other_v[i], s)) return;
    if (n_cs_other < 64) { snprintf(cs_other_v[n_cs_other], 48, "%s", s); n_cs_other++; }
}

static void scan_cost(const AbilityEffect *e) {
    if (!e) return;
    n_cost_nodes++;
    for (int i = 0; i < e->n_extra; i++) {
        bump_ax(e->action, e->extra_k[i]);
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "state_change"))
            bump_sc(e->extra_v[i]);
    }
    if (want_keys(e->action))
        for (int i = 0; i < e->n_extra; i++)
            note_key(e->action, e->extra_k[i], e->extra_v[i]);
    /* C3: distinct on a cost */
    if (xget(e, "distinct")) {
        n_distinct_cost++;
        snprintf(dist_shape[n_distinct_cost - 1 < 32 ? n_distinct_cost - 1 : 31], 150,
                 "action=%-16s source=%-8s count=%-3d opt=%d distinct='%s' "
                 "any_number='%s' group_reference='%s' dest=%s",
                 e->action ? e->action : "-", e->source ? e->source : "-", e->count,
                 e->is_optional, xget(e, "distinct"),
                 xget(e, "any_number") ? xget(e, "any_number") : "-",
                 xget(e, "group_reference") ? xget(e, "group_reference") : "-",
                 e->destination ? e->destination : "-");
    }
    /* card_type: extras vs struct field, per kind */
    int site = -1;
    if (c_is_reveal(e))            site = SITE_REVEAL;
    else if (c_is_move_cards(e))   site = SITE_MOVE;
    else if (c_is_change_state(e)) site = SITE_CHANGEST;
    if (site >= 0) {
        cost_n[site]++;
        if (e->card_type_field[0]) ct_field[site]++;
        if (xget(e, "card_type"))  ct_extra[site]++;
    }
    /* C4: change_state state_change literal */
    if (c_is_change_state(e)) {
        n_changestate++;
        const char *sc = xget(e, "state_change");
        if (e->target && strstr(e->target, "state_change")) {
            const char *t = e->target;
            if      (strstr(t, "wait"))   cs_wait++;
            else if (strstr(t, "rested")) cs_rested++;
            else if (strstr(t, "rest"))   cs_rest++;
            else { cs_other++; bump_cs_other(t); }
        } else if (!sc)              { cs_other++; bump_cs_other(e->action ? e->action : "(none)"); }
        else if (!strcmp(sc,"wait"))   cs_wait++;
        else if (!strcmp(sc,"rested")) cs_rested++;
        else if (!strcmp(sc,"rest"))   cs_rest++;
        else { cs_other++; bump_cs_other(sc); }
        if (xget(e, "exclude_self")) n_exclself++;
    }
    /* F5 / F1 / T2-10 / T2-15 / F2 shape tallies (effects, walked below too) */
    for (int i = 0; i < e->n_child; i++) scan_cost(e->child[i]);
    scan_cost(e->primary_effect);
    scan_cost(e->alternative_effect);
    scan_cost(e->followup_action);
    scan_cost(e->optional_action);
    scan_cost(e->conditional_action);
}

static void scan_eff(const AbilityEffect *e) {
    if (!e) return;
    n_eff_nodes++;
    for (int i = 0; i < e->n_extra; i++)
        bump_ax(e->action, e->extra_k[i]);
    if (want_keys(e->action))
        for (int i = 0; i < e->n_extra; i++)
            note_key(e->action, e->extra_k[i], e->extra_v[i]);
    const char *a = e->action;
    note_ms(e);
    if (a) {
        if (!strcmp(a, "custom"))            { n_custom++; if (e->text) n_custom_has_text++; }
        if (!strcmp(a, "restriction"))      { n_restriction++; if (xget(e, "changed_state_members")) n_restrict_csm++; }
        if (!strcmp(a, "position_change"))   n_position++;
        if (!strcmp(a, "gain_heart")) {
            n_gainheart++;
            if (xget(e, "hearts") || xget(e, "heart_colors") || xget(e, "count"))
                n_gainheart_hearts++;
        }
        /* F5: the yell-revealed-card-icon decode keys. r_yell_icons reads these. */
        if (!strcmp(a, "yell") || strstr(a, "yell"))
            if (xget(e, "card_icons") || xget(e, "icons") ||
                xget(e, "revealed_cards") || xget(e, "card_type"))
                n_yellicons++;
    }
    for (int i = 0; i < e->n_child; i++) scan_eff(e->child[i]);
    scan_eff(e->primary_effect);
    scan_eff(e->alternative_effect);
    scan_eff(e->followup_action);
    scan_eff(e->optional_action);
    scan_eff(e->conditional_action);
}

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) != 0) { fprintf(stderr, "rb_load failed\n"); return 2; }
    int n_cards = rb_num_cards();
    printf("CARDS %d\n", n_cards);
    for (int cid = 0; cid < n_cards; cid++) {
        int n = rb_card_num_abilities((uint32_t)cid);
        for (int ai = 0; ai < n; ai++) {
            Ability ab; memset(&ab, 0, sizeof(ab));
            if (!rb_decode_card_ability((uint32_t)cid, ai, &ab)) continue;
            n_pairs++; n_ability++;
            /* C1: the TOP-LEVEL cost is what setup_ability_cost_offered gates. */
            {
                Card cc; char no[64] = "-";
                if (rb_decode_card_by_index((uint32_t)cid, &cc)) {
                    const char *s = rb_card_string(cc.card_no_idx);
                    if (s) snprintf(no, 64, "%s", s);
                    rb_free_card(&cc);
                }
                if (mandatory_cost_exempt(ab.cost)) {
                    if (n_cand < MAXCAND) {
                        snprintf(cand_no[n_cand], 64, "%s", no);
                        snprintf(cand_shape[n_cand], 80, "action=%s opt=%d any=%s max=%s",
                                 ab.cost->action ? ab.cost->action : "(none)", ab.cost->is_optional,
                                 xget(ab.cost, "any_number") ? xget(ab.cost, "any_number") : "-",
                                 xget(ab.cost, "max") ? xget(ab.cost, "max") : "-");
                        n_cand++;
                    }
                }
                note_activate_candidate(no, cid, ai, ab.triggers, ab.cost);
            }
            scan_cost(ab.cost);
            scan_eff(ab.effect);
            rb_free_ability(&ab);
        }
    }
    printf("ABILITIES %ld PAIRS %ld COST_NODES %ld EFFECT_NODES %ld\n",
           n_ability, n_pairs, n_cost_nodes, n_eff_nodes);
    printf("--- CARDTYPE (cost.c ten sites) ---\n");
    for (int i = 0; i < SITE_MAX; i++)
        printf("SITE %-28s nodes=%ld field_nonempty=%ld extra_present=%ld\n",
               SITE_NAME[i], cost_n[i], ct_field[i], ct_extra[i]);
    printf("DISTINCT_ON_COST %ld\n", n_distinct_cost);
    for (int i = 0; i < n_distinct_cost && i < 32; i++)
        printf("DISTINCT_COST %s\n", dist_shape[i]);
    printf("CHANGESTATE_NODES %ld wait=%ld rest=%ld rested=%ld other=%ld\n",
           n_changestate, cs_wait, cs_rest, cs_rested, cs_other);
    for (int i = 0; i < n_cs_other; i++) printf("CHANGESTATE_OTHER %s\n", cs_other_v[i]);
    printf("EXCLUDE_SELF_ON_CHANGESTATE %ld\n", n_exclself);
    printf("--- DISTINCT state_change LITERALS (all nodes, cost + effect) ---\n");
    for (int i = 0; i < n_sc; i++) printf("SC '%s' %ld\n", sc_val[i], sc_has[i]);
    printf("YELL_ICON_SHAPES %ld\n", n_yellicons);
    printf("CUSTOM_ACTIONS %ld custom_with_text=%ld\n", n_custom, n_custom_has_text);
    printf("RESTRICTION_NODES %ld restriction_with_changed_state_members=%ld\n",
           n_restriction, n_restrict_csm);
    printf("POSITION_CHANGE_NODES %ld\n", n_position);
    printf("GAIN_HEART_NODES %ld with_hearts_or_count=%ld\n", n_gainheart, n_gainheart_hearts);
    printf("T2_13 modify_score live_total=%ld op_set=%ld live_total_AND_set=%ld\n",
           n_ms_live, n_ms_set, n_ms_live_set);
    for (int i = 0; i < n_ms_set_shape; i++)
        printf("T2_13_SET %s\n", ms_set_shape[i]);
    printf("--- ACTION/KEY PAIRS ---\n");
    for (int i = 0; i < n_ax; i++) printf("AX %-32s %-34s %ld\n", ax_act[i], ax_key[i], ax_has[i]);
    printf("--- KEYS ON custom/heart/yell/icon/restrict/position/draw ---\n");
    for (int i = 0; i < n_yn; i++)
        printf("YN %-28s %-28s = %-28s %ld\n", yn_act[i], yn_key[i], yn_val[i], yn_has[i]);
    printf("--- C1 CANDIDATES (top-level cost where the two validators disagree) %d ---\n", n_cand);
    for (int i = 0; i < n_cand && i < 24; i++)
        printf("C1CAND cid=%d ab#%d %-22s %s\n", i, i, cand_no[i], cand_shape[i]);
    printf("--- 起動 ABILITIES (offer-scan candidates) ---\n");
    for (int i = 0; i < n_act; i++)
        if (strstr(act_shape[i], "seq_exempt=0") == NULL)
            printf("ACTE %s\n", act_shape[i]);
    printf("C1_SEQ_EXEMPT_ACTIVATIONS %d of %d\n", n_act_seq_exempt, n_act);
    rb_unload();
    return 0;
}