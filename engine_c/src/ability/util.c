/* util.c — pure card/zone/comparison helpers.
   Mirror engine/src/ability/util.rs (compare_counts, card_matches_type,
   card_matches_group_str, card_at_position, pos_to_area, orientation_matches_state,
   zone_cards, CardFilter, constant_per_unit_units, compute_play_cost helpers,
   matching_ids, per_unit resolution, distinct filtering, selection primitives).
   These are shared by the condition evaluators and the effect executors; keeping
   them in one place mirrors the Rust single-source layout.

   Group matching is an approximation of the Rust card_matches_group_str: the C
   Card exposes group/unit/name strings (via rb_card_string) but not the series
   or set_card_identity-derived memberships, so only group/unit/name substring
   matches are performed. Fullwidth '！'(U+FF01) and micro 'µ'(U+00B5) are
   normalized to '!'/'μ' exactly like the Rust norm_group_name helper. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <ctype.h>

/* Heart-all wildcard key — mirrors util.rs HEART_ALL_KEY ("heart00"). */
#define RB_HEART_ALL_KEY "heart00"

/* ── Internal filter struct (mirrors CardFilter) ─────────────────────── */

/* See RbCardFilter in rabuka.h for the public version of this struct. */
/* The local copy below is used inside this TU; the public one in the header
   is used by external callers. They must stay in sync. */
typedef struct {
    char  card_type[32];
    char  group[64];
    int   has_group;
    int   cost_limit;
    char  cost_op[8];
    int   has_cost_limit;
    char  characters[256];
    int   has_characters;
    char  exclude_characters[256];
    int   has_exclude_characters;
    char  heart_colors[8][24];
    int   n_heart_colors;
    int   require_all_heart_colors;
    int   heart_color_count;
    int   need_heart_total;
    char  need_heart_operator[8];
    char  need_heart_color[24];
    int   has_need_heart_total;
    char  name_fragments[8][64];
    int   n_name_fragments;
    int   original_blade_limit;
    char  original_blade_op[8];
    int   has_original_blade;
    char  ability_filter[32];
    char  ability_filter_triggers[8][32];
    int   n_ability_filter_triggers;
    int   negation;
    int   exclude_self_id;
    int   has_exclude_self;
    int   cost_total;
    char  cost_total_op[8];
    int   has_cost_total;
    int   has_filter;
} LocalCardFilter;

/* ── UTF-8 / normalization helpers ───────────────────────────────────── */

/* Normalize a UTF-8 copy: '！'(EF BC 81)→'!', 'µ'(C2 B5)→'μ'(CE BC).
   Caller must rb_free the result. Returns NULL on alloc failure. */
static char *norm_str(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s);
    char *out = rb_malloc(n + 1);
    if (!out) return NULL;
    size_t j = 0;
    for (size_t i = 0; i < n; ) {
        if ((unsigned char)s[i] == 0xEF && (unsigned char)s[i+1] == 0xBC && (unsigned char)s[i+2] == 0x81) {
            out[j++] = '!'; i += 3; continue; /* fullwidth ! */
        }
        if ((unsigned char)s[i] == 0xC2 && (unsigned char)s[i+1] == 0xB5) {
            out[j++] = (char)0xCE; out[j++] = (char)0xBC; i += 2; continue; /* µ → μ */
        }
        out[j++] = s[i++];
    }
    out[j] = '\0';
    return out;
}

/* Mirror util.rs::norm_group_name — normalize fullwidth '！'→'!' and 'µ'→'μ'.
   Caller must rb_free the result. */
static char *rb_norm_group_name(const char *s) {
    return norm_str(s);
}

/* Strip ASCII whitespace, mirroring CardDatabase::normalize_name. */
static void rb_norm_ws(const char *s, char *out, size_t outsz) {
    size_t j = 0;
    if (!s) { if (outsz) out[0] = '\0'; return; }
    for (size_t i = 0; s[i]; i++) {
        char c = s[i];
        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') continue;
        if (j + 1 < outsz) out[j++] = c;
    }
    out[j] = '\0';
}

/* case-insensitive equality (portable, no strcasecmp dependency). */
static int str_ieq(const char *a, const char *b) {
    if (!a || !b) return 0;
    while (*a && *b) {
        char ca = *a, cb = *b;
        if (ca >= 'A' && ca <= 'Z') ca += 32;
        if (cb >= 'A' && cb <= 'Z') cb += 32;
        if (ca != cb) return 0;
        a++; b++;
    }
    return *a == *b;
}

/* ── Effect-field reader ─────────────────────────────────────────────── */

/* Mirror the Rust typed getters (operation_any, location_any, heart_type_any,
   original_count_any, original_operator_any, …) mapped to extra_k/extra_v. */
static const char *eff_extra(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

/* ── Comparison / threshold ──────────────────────────────────────────── */

/* Mirror util::compare_counts. operator NULL defaults to ">=". */
int rb_compare_counts(const char *operator, int actual, int expected) {
    const char *op = operator ? operator : ">=";
    if (!strcmp(op, ">=")) return actual >= expected;
    if (!strcmp(op, ">"))  return actual >  expected;
    if (!strcmp(op, "<=")) return actual <= expected;
    if (!strcmp(op, "<"))  return actual <  expected;
    if (!strcmp(op, "==") || !strcmp(op, "="))  return actual == expected;
    if (!strcmp(op, "!=")) return actual != expected;
    return 1;
}

/* Mirror util.rs::cost_threshold_met — original_count/operator gate on the
   card's cost (used by cost-reduction / play-cost auras). */
int rb_cost_threshold_met(const Card *card, const AbilityEffect *e) {
    const char *th = eff_extra(e, "original_count");
    if (!th || !*th) return 1;
    int threshold = atoi(th);
    int cost = card ? card->cost : 0;
    const char *op = eff_extra(e, "original_operator");
    int met = 1;
    if (op) {
        if      (!strcmp(op, ">=")) met = cost >= threshold;
        else if (!strcmp(op, "<=")) met = cost <= threshold;
        else if (!strcmp(op, ">"))  met = cost >  threshold;
        else if (!strcmp(op, "<"))  met = cost <  threshold;
        else if (!strcmp(op, "==")) met = cost == threshold;
        else if (!strcmp(op, "!=")) met = cost != threshold;
        else met = 1;
    } else {
        met = (cost == threshold);
    }
    return met;
}

/* ── Card property predicates ────────────────────────────────────────── */

/* Mirror card.rs::has_blade_heart / has_score_icon / has_all_blade. */
int rb_card_has_blade_heart(const Card *c) {
    if (c->num_blade > 0) return 1;
    if (c->has_special && c->special_count > 0) return 1;
    return 0;
}
int rb_card_has_score_icon(const Card *c) {
    return c->has_special && c->special_color == (uint8_t)RB_HEART_SCORE;
}
int rb_card_has_all_blade(const Card *c) {
    int base = c->num_base;
    int end = base + c->num_blade;
    if (end > c->n_hearts) end = c->n_hearts;
    for (int h = base; h < end; h++)
        if (c->heart_color[h] == (uint8_t)RB_HEART_ALL) return 1;
    return 0;
}

/* ── Card-type predicate ──────────────────────────────────────────────── */

/* Mirror util::card_matches_type. card_id is a card_no index. */
int rb_card_matches_type(int card_id, const char *filter) {
    if (!filter) return 1;
    int is_live   = rb_card_is_live(card_id);
    int is_energy = rb_card_is_energy(card_id);
    int is_member = !is_live && !is_energy;
    if (!strcmp(filter, "live_card"))        return is_live;
    if (!strcmp(filter, "member_card"))      return is_member;
    if (!strcmp(filter, "energy_card"))      return is_energy;
    return 1;
}

/* ── Orientation predicate ────────────────────────────────────────────── */

/* Mirror util::orientation_matches_state. */
int rb_orientation_matches_state(const char *orientation, const char *state) {
    if (!orientation) return state && !strcmp(state, "active");
    return !strcmp(orientation, state);
}

/* ── Group matching ───────────────────────────────────────────────────── */

/* Mirror util.rs::card_series_matches_group. */
static int rb_card_series_matches_group(const char *series, const char *group) {
    if (!series || !group) return 0;
    if (!strcmp(group, "μ's")) {
        const char *p = series;
        while (*p) {
            const char *nl = strchr(p, '\n');
            size_t len = nl ? (size_t)(nl - p) : strlen(p);
            char line[1024];
            if (len >= sizeof(line)) len = sizeof(line) - 1;
            memcpy(line, p, len);
            line[len] = '\0';
            if (strstr(line, "ラブライブ！") &&
                !strstr(line, "サンシャイン") &&
                !strstr(line, "虹ヶ咲") &&
                !strstr(line, "スーパースター") &&
                !strstr(line, "蓮ノ空"))
                return 1;
            if (!nl) break;
            p = nl + 1;
        }
        return 0;
    }
    if (!strcmp(group, "Aqours"))   return strstr(series, "サンシャイン") != NULL;
    if (!strcmp(group, "虹ヶ咲"))  return strstr(series, "虹ヶ咲") != NULL;
    if (!strcmp(group, "Liella!")) return strstr(series, "スーパースター") != NULL;
    if (!strcmp(group, "蓮ノ空"))  return strstr(series, "蓮ノ空") != NULL;
    return 0;
}

/* Mirror util.rs::debug_group_match — no-op in the C port. */
static void rb_debug_group_match(int card_id, const char *group_name, int result) {
    (void)card_id; (void)group_name; (void)result;
}

/* Mirror util.rs::card_matches_group_str — group/unit/name/series + set_card_identity. */
int rb_card_matches_group_str(int card_id, const char *group_name) {
    if (!group_name) return 1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;

    char *gn = norm_str(group_name);
    const char *g  = rb_card_string(c.group_idx);
    const char *u  = rb_card_string(c.unit_idx);
    const char *s  = rb_card_string(c.series_idx);
    char *gnorm = g  ? norm_str(g)  : NULL;
    char *unorm = u  ? norm_str(u)  : NULL;
    char *nnorm = c.name ? norm_str(c.name) : NULL;

    int match = 0;
    if (gnorm) {
        /* Exact and substring match on raw strings */
        if ((g && (!strcmp(g, group_name) || strstr(g, group_name) || strstr(group_name, g))) ||
            (u && (!strcmp(u, group_name) || strstr(u, group_name) || strstr(group_name, u))) ||
            (c.name && (strstr(c.name, group_name) || strstr(group_name, c.name))))
            match = 1;
        /* Normalized comparisons catch fullwidth-bang / micro mismatches */
        if (!match && (strstr(gnorm, gn) || strstr(gn, gnorm) ||
                       (unorm && (strstr(unorm, gn) || strstr(gn, unorm))) ||
                       (nnorm && (strstr(nnorm, gn) || strstr(gn, nnorm)))))
            match = 1;
        /* Series membership (multi-series joint cards match via any line) */
        if (!match && s && rb_card_series_matches_group(s, group_name))
            match = 1;
    }
    /* set_card_identity overrides */
    if (!match) match = rb_card_matches_identity_str(card_id, group_name);
    rb_free_card(&c);
    if (gn)    rb_free(gn);
    if (gnorm) rb_free(gnorm);
    if (unorm) rb_free(unorm);
    if (nnorm) rb_free(nnorm);
    return match;
}

/* Mirror util.rs::card_matches_any_group — pass if groups is empty or card
   matches ANY entry. */
int rb_card_matches_any_group(int card_id, const char **groups, int n) {
    if (!groups || n <= 0) return 1;
    for (int i = 0; i < n; i++)
        if (groups[i] && rb_card_matches_group_str(card_id, groups[i])) return 1;
    return 0;
}

/* ── Card predicate helpers ───────────────────────────────────────────── */

/* Mirror util.rs::card_matches_characters — name must contain any listed name
   (after normalization). */
int rb_card_matches_characters(int card_id, const char **names, int n) {
    if (n <= 0) return 1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = 0;
    if (c.name) {
        char norm_card[512];
        rb_card_normalize_name(c.name, norm_card, sizeof norm_card);
        for (int k = 0; k < n; k++) {
            if (!names[k]) continue;
            char norm_name[256];
            rb_card_normalize_name(names[k], norm_name, sizeof norm_name);
            if (strstr(norm_card, norm_name)) { r = 1; break; }
        }
    }
    rb_free_card(&c);
    return r;
}

/* Mirror util.rs::card_matches_cost_limit_op — live cards match on score,
   members on cost; comparison defaults to "<=". */
int rb_card_matches_cost_limit(int card_id, int cost_limit, const char *comparison) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int value = rb_card_is_live(card_id) ? (int)c.score : (int)c.cost;
    int r;
    if (!comparison || !*comparison)            r = value <= cost_limit;
    else if (!strcmp(comparison, "min") || !strcmp(comparison, ">=")) r = value >= cost_limit;
    else if (!strcmp(comparison, "exact") || !strcmp(comparison, "=")) r = value == cost_limit;
    else if (!strcmp(comparison, ">"))  r = value >  cost_limit;
    else if (!strcmp(comparison, "<"))  r = value <  cost_limit;
    else                                 r = value <= cost_limit;
    rb_free_card(&c);
    return r;
}
int rb_card_matches_cost_limit_op(int card_id, int cost_limit, const char *comparison) {
    return rb_card_matches_cost_limit(card_id, cost_limit, comparison);
}

static int card_has_heart_in_range(const Card *c, int hc, int start, int end) {
    for (int i = start; i < end && i < c->n_hearts; i++)
        if ((int)c->heart_color[i] == hc) return 1;
    return 0;
}

/* Mirror util.rs::card_matches_heart_colors — OR logic over the listed colors. */
int rb_card_matches_heart_colors(int card_id, const char **heart_colors, int n) {
    if (n <= 0) return 1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = 0;
    for (int k = 0; k < n; k++) {
        int hc = (int)rb_parse_heart_color(heart_colors[k]);
        int found = (c.num_base > 0)
            ? card_has_heart_in_range(&c, hc, 0, c.num_base)
            : card_has_heart_in_range(&c, hc, c.num_base + c.num_blade,
                                      c.num_base + c.num_blade + c.num_need);
        if (found) { r = 1; break; }
    }
    rb_free_card(&c);
    return r;
}

/* Mirror util.rs::card_matches_all_heart_colors — AND logic over the listed colors. */
int rb_card_matches_all_heart_colors(int card_id, const char **heart_colors, int n) {
    if (n <= 0) return 1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = 1;
    for (int k = 0; k < n; k++) {
        int hc = (int)rb_parse_heart_color(heart_colors[k]);
        int found = (c.num_base > 0)
            ? card_has_heart_in_range(&c, hc, 0, c.num_base)
            : card_has_heart_in_range(&c, hc, c.num_base + c.num_blade,
                                      c.num_base + c.num_blade + c.num_need);
        if (!found) { r = 0; break; }
    }
    rb_free_card(&c);
    return r;
}

/* Mirror util.rs::card_matches_name_fragments — name must contain every fragment. */
int rb_card_matches_name_fragments(int card_id, const char **fragments, int n) {
    if (n <= 0) return 1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = 1;
    if (c.name) {
        char norm_card[512];
        rb_card_normalize_name(c.name, norm_card, sizeof norm_card);
        for (int k = 0; k < n; k++) {
            if (!fragments[k]) continue;
            char norm_frag[256];
            rb_card_normalize_name(fragments[k], norm_frag, sizeof norm_frag);
            if (!strstr(norm_card, norm_frag)) { r = 0; break; }
        }
    } else {
        r = 0;
    }
    rb_free_card(&c);
    return r;
}

/* Mirror util.rs::card_matches_name_constraint — exact normalized match. */
int rb_card_matches_name_constraint(int card_id, const char *name_constraint) {
    if (!name_constraint) return 1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    char cons[256];
    rb_norm_ws(name_constraint, cons, sizeof cons);
    int r = 0;
    if (c.name) {
        char nm[256];
        rb_norm_ws(c.name, nm, sizeof nm);
        const char *p = nm;
        while (*p) {
            const char *sep = NULL;
            for (const char *q = p; *q; q++) {
                if (*q == '&') { sep = q; break; }
                if ((unsigned char)q[0] == 0xEF && (unsigned char)q[1] == 0xBC &&
                    (unsigned char)q[2] == 0x86) { sep = q; break; }
            }
            size_t len = sep ? (size_t)(sep - p) : strlen(p);
            char tok[256];
            if (len >= sizeof tok) len = sizeof tok - 1;
            memcpy(tok, p, len); tok[len] = '\0';
            if (!strcmp(tok, cons)) { r = 1; break; }
            p = sep ? sep + (sep[0] == '&' ? 1 : 3) : p + strlen(p);
        }
    }
    rb_free_card(&c);
    return r;
}

/* ── HeartColor / duration / target helpers ───────────────────────────── */

/* Mirror util.rs::heart_gain_per_entry. */
int rb_heart_gain_per_entry(int total, int n_colors) {
    int len = n_colors > 0 ? n_colors : 1;
    return total / len;
}

/* Mirror util.rs::is_all_heart_type. */
int rb_is_all_heart_type(const AbilityEffect *e) {
    const char *ht = eff_extra(e, "heart_type");
    return ht && (!strcmp(ht, "all") || !strcmp(ht, RB_HEART_ALL_KEY));
}

/* Mirror util.rs::constant_per_unit_zone. */
const char *rb_constant_per_unit_zone(const AbilityEffect *e) {
    const char *loc = eff_extra(e, "location");
    if (loc) return loc;
    const char *per = eff_extra(e, "per_unit_type");
    if (per) return per;
    return "hand";
}

/* Mirror util.rs::target_player_index — returns -1 when unresolvable. */
int rb_target_player_index(const char *target, const char *master) {
    if (!target) return -1;
    int master_p2 = master && (!strcmp(master, "player2") || !strcmp(master, "p2"));
    if (!strcmp(target, "self"))     return master_p2 ? 1 : 0;
    if (!strcmp(target, "opponent")) return master_p2 ? 0 : 1;
    if (str_ieq(target, "player1") || !strcmp(target, "p1")) return 0;
    if (str_ieq(target, "player2") || !strcmp(target, "p2")) return 1;
    return -1;
}

/* Mirror util.rs::target_player_label. */
const char *rb_target_player_label(const char *target, const char *master) {
    if (target) {
        int master_p2 = master && (!strcmp(master, "player2") || !strcmp(master, "p2"));
        if (!strcmp(target, "self"))     return master_p2 ? "P2" : "P1";
        if (!strcmp(target, "opponent")) return master_p2 ? "P1" : "P2";
    }
    return "P1";
}

/* Mirror util.rs::parse_duration. */
int rb_parse_duration(const char *s) {
    if (!s) return RB_TEMP_LIVE_END;
    if (!strcmp(s, "this_turn"))    return RB_TEMP_TURN_END;
    if (!strcmp(s, "live_end") || !strcmp(s, "this_live")) return RB_TEMP_LIVE_END;
    if (!strcmp(s, "as_long_as") || !strcmp(s, "permanent")) return RB_TEMP_PERM;
    return RB_TEMP_LIVE_END;
}

/* ── Position / stage helpers ─────────────────────────────────────────── */

/* Mirror util.rs::activation_position_index. */
int rb_activation_position_index(const char *p) {
    if (!p) return -1;
    while (*p == ' ' || *p == '\t') p++;
    if (!strcmp(p, "left") || !strcmp(p, "left_side")) return 0;
    if (!strcmp(p, "center")) return 1;
    if (!strcmp(p, "right") || !strcmp(p, "right_side")) return 2;
    return -1;
}

/* Mirror util.rs::stage_position_index. */
int rb_stage_position_index(const char *pos) {
    if (!pos) return -1;
    if (!strcmp(pos, "center") || !strcmp(pos, "センターエリア")) return 1;
    if (!strcmp(pos, "left_side") || !strcmp(pos, "左サイドエリア") || !strcmp(pos, "left")) return 0;
    if (!strcmp(pos, "right_side") || !strcmp(pos, "右サイドエリア") || !strcmp(pos, "right")) return 2;
    return -1;
}

/* Mirror util.rs::pos_to_area — delegates to stage index, RightSide fallback. */
int rb_pos_to_area(const char *pos) {
    if (!pos) return 1;
    if (!strcmp(pos, "left_side"))  return 0;
    if (!strcmp(pos, "center"))     return 1;
    if (!strcmp(pos, "right_side")) return 2;
    return 1;
}

/* Mirror util.rs::card_at_position. */
int rb_card_at_position(const struct GameState *g, int pl, const char *pos) {
    int idx = -1;
    if (!pos) return -1;
    if (!strcmp(pos, "left_side")) idx = 0;
    else if (!strcmp(pos, "center")) idx = 1;
    else if (!strcmp(pos, "right_side")) idx = 2;
    else return -1;
    if (idx < 0 || idx >= RB_STAGE_SIZE) return -1;
    int cid = g->p[pl].stage[idx];
    return cid == RB_EMPTY_SLOT ? -1 : cid;
}

/* Per-card identity override slots (mirrors Rust set_card_identity). */
#define RB_MAX_IDENT 8
static char *g_ident[RB_MAX_CARD_IDS];
static int  g_ident_n[RB_MAX_CARD_IDS];

/* Forward declarations for mutually-recursive helpers. */
static RbBag *zone_bag(RbPlayer *P, const char *zone);
static int rb_get_selection_indices_filter(const int *cards, int n, const RbCardFilter *filter,
                                            int self_target_only, int activating_card,
                                            int *out_idx, int max);
static int rb_resolve_selection_filter(const int *cards, int n, const RbCardFilter *filter,
                                        int count, int is_all, int self_target_only,
                                        int activating_card);

/* ── Zone card access ──────────────────────────────────────────────────── */

/* Mirror util.rs::zone_cards — fill out_ids (capacity max) with card_no indices
   for the named zone of player pl; returns the count written. */
int rb_zone_cards(const struct GameState *g, int pl, const char *zone, int *out_ids, int max) {
    const RbPlayer *P = &g->p[pl];
    int n = 0;
    #define PUSH(cid) do { if (n < max) out_ids[n++] = (cid); } while(0)
    if (!zone) return 0;
    if (!strcmp(zone, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) PUSH(P->stage[i]);
    } else if (!strcmp(zone, "hand")) {
        for (int i = 0; i < P->hand.n; i++) PUSH(P->hand.cards[i]);
    } else if (!strcmp(zone, "deck") || !strcmp(zone, "deck_top") || !strcmp(zone, "deck_bottom")) {
        for (int i = 0; i < P->deck.n; i++) PUSH(P->deck.cards[i]);
    } else if (!strcmp(zone, "discard") || !strcmp(zone, "waitroom")) {
        for (int i = 0; i < P->discard.n; i++) PUSH(P->discard.cards[i]);
    } else if (!strcmp(zone, "energy") || !strcmp(zone, "energy_zone")) {
        for (int i = 0; i < P->energy.n; i++) PUSH(P->energy.cards[i]);
    } else if (!strcmp(zone, "live") || !strcmp(zone, "live_card_zone")) {
        for (int i = 0; i < P->live.n; i++) PUSH(P->live.cards[i]);
    } else if (!strcmp(zone, "success") || !strcmp(zone, "success_live_zone") ||
               !strcmp(zone, "success_live_card_zone")) {
        for (int i = 0; i < P->success.n; i++) PUSH(P->success.cards[i]);
    }
    #undef PUSH
    return n;
}

/* Mirror util.rs::zone_card_ids. */
int rb_zone_card_ids(const GameState *g, int pl, const char *zone, int *out_ids, int max) {
    return rb_zone_cards(g, pl, zone, out_ids, max);
}

/* Mirror util.rs::get_zone_card_count. */
int rb_get_zone_card_count(const GameState *g, int pl, const char *zone) {
    if (!zone) return 0;
    const RbPlayer *P = &g->p[pl];
    if (!strcmp(zone, "stage")) {
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) n++;
        return n;
    }
    if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) n += P->under_cards[s].n;
        return n;
    }
    return rb_count_in_zone(g, pl, zone);
}

/* Mirror util.rs::count_in_zone (with optional card_type/group filter). */
int rb_count_in_zone(const GameState *g, int pl, const char *zone) {
    RbPlayer *P = (RbPlayer *)&g->p[pl];
    if (!strcmp(zone, "stage")) {
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) n++;
        return n;
    }
    if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) n += P->under_cards[s].n;
        return n;
    }
    RbBag *b = zone_bag(P, zone);
    return b ? b->n : 0;
}
int rb_count_in_zone_filtered(const GameState *g, int pl, const char *zone,
                              const char *card_type, const char *group) {
    if (!zone) return 0;
    const RbPlayer *P = &g->p[pl];
    if (!strcmp(zone, "stage")) {
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            if (card_type && !rb_card_matches_type(cid, card_type)) continue;
            if (group && !rb_card_matches_group_str(cid, group)) continue;
            n++;
        }
        return n;
    }
    if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            const RbBag *u = &P->under_cards[s];
            for (int i = 0; i < u->n; i++) {
                int cid = u->cards[i];
                if (card_type && !rb_card_matches_type(cid, card_type)) continue;
                if (group && !rb_card_matches_group_str(cid, group)) continue;
                n++;
            }
        }
        return n;
    }
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, zone, ids, RB_MAX_ZONE);
    int count = 0;
    for (int i = 0; i < n; i++) {
        if (card_type && !rb_card_matches_type(ids[i], card_type)) continue;
        if (group && !rb_card_matches_group_str(ids[i], group)) continue;
        count++;
    }
    return count;
}

/* ── Zone move / place helpers ─────────────────────────────────────────── */

static RbBag *zone_bag(RbPlayer *P, const char *zone) {
    if (!strcmp(zone, "hand")) return &P->hand;
    if (!strcmp(zone, "deck") || !strcmp(zone, "main_deck")) return &P->deck;
    if (!strcmp(zone, "waitroom") || !strcmp(zone, "discard")) return &P->discard;
    if (!strcmp(zone, "energy") || !strcmp(zone, "energy_zone")) return &P->energy;
    if (!strcmp(zone, "live") || !strcmp(zone, "live_card_zone")) return &P->live;
    if (!strcmp(zone, "success") || !strcmp(zone, "success_live_zone") ||
        !strcmp(zone, "success_live_card_zone")) return &P->success;
    return NULL;
}

int rb_remove_card_from_zone(GameState *g, int pl, int card_id, const char *zone) {
    RbPlayer *P = &g->p[pl];
    if (!strcmp(zone, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == card_id) { P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0; return 1; }
        return 0;
    }
    if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            RbBag *u = &P->under_cards[s];
            for (int i = 0; i < u->n; i++)
                if (u->cards[i] == card_id) {
                    for (int k = i; k < u->n - 1; k++) u->cards[k] = u->cards[k + 1];
                    u->n--; return 1;
                }
        }
        return 0;
    }
    RbBag *b = zone_bag(P, zone);
    if (!b) return 0;
    for (int i = 0; i < b->n; i++)
        if (b->cards[i] == card_id) {
            for (int k = i; k < b->n - 1; k++) b->cards[k] = b->cards[k + 1];
            b->n--; return 1;
        }
    return 0;
}

int rb_place_card_in_zone(GameState *g, int pl, int card_id, const char *zone,
                          int vacated_area) {
    RbPlayer *P = &g->p[pl];
    if (!strcmp(zone, "stage") || !strcmp(zone, "empty_area")) {
        int area = vacated_area >= 0 ? vacated_area : rb_stage_first_empty(P->stage);
        if (area < 0 || area >= RB_STAGE_SIZE) return 0;
        P->stage[area] = card_id; P->stage_wait[area] = 0;
        return 1;
    }
    RbBag *b = zone_bag(P, zone);
    if (!b) return 0;
    if (b->n < RB_MAX_ZONE) { b->cards[b->n++] = card_id; return 1; }
    return 0;
}

int rb_move_card(GameState *g, int pl, int card_id, const char *src,
                 const char *dst, int vacated_area) {
    if (rb_remove_card_from_zone(g, pl, card_id, src))
        return rb_place_card_in_zone(g, pl, card_id, dst, vacated_area);
    return 0;
}

int rb_move_cards(GameState *g, int pl, const int *card_ids, int n,
                  const char *src, const char *dst, int vacated_area) {
    int c = 0;
    for (int i = 0; i < n; i++)
        if (rb_move_card(g, pl, card_ids[i], src, dst, vacated_area)) c++;
    return c;
}

int rb_resolve_indices_to_ids(const GameState *g, int pl, const char *zone,
                               const int *indices, int n_idx, int *out) {
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, zone, ids, RB_MAX_ZONE);
    int m = 0;
    for (int i = 0; i < n_idx; i++) {
        int idx = indices[i];
        if (idx >= 0 && idx < n) out[m++] = ids[idx];
    }
    return m;
}

/* ── Card-filter matching (local helpers) ──────────────────────────────── */

/* Check if card_id matches the LocalCardFilter fields. */
static int local_filter_matches(const LocalCardFilter *f, int card_id) {
    if (f->has_exclude_self && card_id == f->exclude_self_id) return 0;
    if (f->card_type[0] && !rb_card_matches_type(card_id, f->card_type)) return 0;
    if (f->has_group && !rb_card_matches_group_str(card_id, f->group)) return 0;
    if (f->has_cost_limit && !rb_card_matches_cost_limit(card_id, f->cost_limit, f->cost_op[0] ? f->cost_op : NULL))
        return 0;
    if (f->has_characters) {
        if (!rb_card_matches_characters(card_id, (const char **)(intptr_t)(intptr_t)(const char **)f->characters, 1))
            return 0;
    }
    if (f->has_exclude_characters) {
        /* Passes only if it does NOT match the excluded characters */
        Card c;
        if (rb_decode_card_by_index((uint32_t)card_id, &c) && c.name) {
            char nc[512], ec[256];
            rb_card_normalize_name(c.name, nc, sizeof nc);
            rb_card_normalize_name(f->exclude_characters, ec, sizeof ec);
            if (strstr(nc, ec)) { rb_free_card(&c); return 0; }
            rb_free_card(&c);
        }
    }
    if (f->n_heart_colors > 0) {
        const char *colors[8];
        for (int i = 0; i < f->n_heart_colors; i++) colors[i] = f->heart_colors[i];
        int ok = f->require_all_heart_colors
            ? rb_card_matches_all_heart_colors(card_id, colors, f->n_heart_colors)
            : rb_card_matches_heart_colors(card_id, colors, f->n_heart_colors);
        if (!ok) return 0;
    }
    if (f->heart_color_count > 0) {
        /* Per-color count threshold check */
        Card c;
        if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
            int passes = 0;
            for (int hc_idx = 0; hc_idx < f->n_heart_colors; hc_idx++) {
                int hc = (int)rb_parse_heart_color(f->heart_colors[hc_idx]);
                int base_amount = 0, need_amount = 0;
                if (c.num_base > 0)
                    base_amount = card_has_heart_in_range(&c, hc, 0, c.num_base) ? 1 : 0;
                else
                    need_amount = card_has_heart_in_range(&c, hc,
                        c.num_base + c.num_blade, c.num_base + c.num_blade + c.num_need) ? 1 : 0;
                if (base_amount >= f->heart_color_count || need_amount >= f->heart_color_count)
                    passes = 1;
            }
            rb_free_card(&c);
            if (!passes) return 0;
        }
    }
    if (f->has_need_heart_total) {
        Card c;
        int total = 0;
        if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
            if (f->need_heart_color[0]) {
                int hc = (int)rb_parse_heart_color(f->need_heart_color);
                total = card_has_heart_in_range(&c, hc,
                    c.num_base + c.num_blade, c.num_base + c.num_blade + c.num_need) ? 1 : 0;
            } else {
                total = c.num_base > 0 ? c.num_base : c.num_need;
            }
            rb_free_card(&c);
        }
        const char *op = f->need_heart_operator[0] ? f->need_heart_operator : ">=";
        if (!rb_compare_counts(op, total, f->need_heart_total)) return 0;
    }
    if (f->n_name_fragments > 0) {
        const char *frags[8];
        for (int i = 0; i < f->n_name_fragments; i++) frags[i] = f->name_fragments[i];
        if (!rb_card_matches_name_fragments(card_id, frags, f->n_name_fragments)) return 0;
    }
    if (f->has_original_blade) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
        int blade = c.blade;
        rb_free_card(&c);
        if (!rb_compare_counts(f->original_blade_op[0] ? f->original_blade_op : NULL,
                               blade, f->original_blade_limit)) return 0;
    }
    if (f->ability_filter[0]) {
        Card c;
        if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
            int has_ability = c.ability != NULL;
            if (!strcmp(f->ability_filter, "no_ability")) {
                if (has_ability) { rb_free_card(&c); return 0; }
            } else if (!strcmp(f->ability_filter, "has_ability")) {
                if (!has_ability) { rb_free_card(&c); return 0; }
            } else if (!strcmp(f->ability_filter, "no_ability_type")) {
                if (has_ability && c.ability) {
                    int n = rb_card_num_abilities(card_id);
                    for (int ai = 0; ai < n; ai++) {
                        Ability ab;
                        if (!rb_decode_card_ability(card_id, ai, &ab)) continue;
                        if (ab.triggers) {
                            for (int ti = 0; ti < f->n_ability_filter_triggers; ti++) {
                                if (strstr(ab.triggers, f->ability_filter_triggers[ti])) {
                                    rb_free_ability(&ab);
                                    rb_free_card(&c);
                                    return 0;
                                }
                            }
                        }
                        rb_free_ability(&ab);
                    }
                }
            } else if (!strcmp(f->ability_filter, "has_ability_type")) {
                if (has_ability && c.ability) {
                    int found = 0;
                    int n = rb_card_num_abilities(card_id);
                    for (int ai = 0; ai < n; ai++) {
                        Ability ab;
                        if (!rb_decode_card_ability(card_id, ai, &ab)) continue;
                        if (ab.triggers) {
                            for (int ti = 0; ti < f->n_ability_filter_triggers; ti++) {
                                if (strstr(ab.triggers, f->ability_filter_triggers[ti])) {
                                    found = 1; rb_free_ability(&ab); break;
                                }
                            }
                        }
                        rb_free_ability(&ab);
                        if (found) break;
                    }
                    if (!found) { rb_free_card(&c); return 0; }
                }
            }
            rb_free_card(&c);
        }
    }
    return 1;
}

/* ── CardFilter::matches (ported) ─────────────────────────────────────── */

/* Mirror util.rs::CardFilter::has_filter. */
static int local_has_filter(const LocalCardFilter *f) {
    return f->card_type[0] || f->has_group || f->has_cost_limit || f->has_characters ||
           f->has_exclude_characters || f->n_heart_colors > 0 || f->has_need_heart_total ||
           f->n_name_fragments > 0 || f->has_original_blade || f->ability_filter[0] ||
           f->has_exclude_self || f->has_cost_total;
}

/* Mirror util.rs::CardFilter::matches — check all present filter fields. */
static int card_filter_matches(const LocalCardFilter *f, int card_id, int skip_empty) {
    if (skip_empty && card_id < 0) return 0;
    return local_filter_matches(f, card_id);
}

/* Public: matches() with skip_empty. */
static int card_filter_matches_skip(const LocalCardFilter *f, int card_id, int skip_empty) {
    return card_filter_matches(f, card_id, skip_empty);
}

/* Public: matches_card (skip_empty=0). */
static int card_filter_matches_card(const LocalCardFilter *f, int card_id) {
    return card_filter_matches(f, card_id, 0);
}

/* ── matching_ids / matching_indices (ported) ──────────────────────────── */

/* Mirror util.rs::matching_ids — return card IDs matching the filter. */
int rb_matching_ids(const RbCardFilter *rf, const int *cards, int n, int *out, int max) {
    if (!cards || !rf || !rf->has_filter) {
        int m = 0;
        for (int i = 0; i < n && m < max; i++) out[m++] = cards[i];
        return m;
    }
    LocalCardFilter f;
    memset(&f, 0, sizeof f);
    if (rf->card_type[0])    strncpy(f.card_type, rf->card_type, sizeof f.card_type - 1);
    if (rf->group[0])        { strncpy(f.group, rf->group, sizeof f.group - 1); f.has_group = 1; }
    if (rf->has_cost_limit)  { f.cost_limit = rf->cost_limit;
                                if (rf->cost_op[0]) strncpy(f.cost_op, rf->cost_op, sizeof f.cost_op - 1);
                                f.has_cost_limit = 1; }
    if (rf->characters[0])   { strncpy(f.characters, rf->characters, sizeof f.characters - 1); f.has_characters = 1; }
    if (rf->exclude_characters[0]) { strncpy(f.exclude_characters, rf->exclude_characters, sizeof f.exclude_characters - 1); f.has_exclude_characters = 1; }
    for (int i = 0; i < rf->n_heart_colors && i < 8; i++)
        strncpy(f.heart_colors[i], rf->heart_colors[i], sizeof f.heart_colors[i] - 1);
    f.n_heart_colors = rf->n_heart_colors;
    f.require_all_heart_colors = rf->require_all_heart_colors;
    f.heart_color_count = rf->heart_color_count;
    if (rf->has_need_heart_total) {
        f.need_heart_total = rf->need_heart_total;
        if (rf->need_heart_operator[0]) strncpy(f.need_heart_operator, rf->need_heart_operator, sizeof f.need_heart_operator - 1);
        if (rf->need_heart_color[0]) strncpy(f.need_heart_color, rf->need_heart_color, sizeof f.need_heart_color - 1);
        f.has_need_heart_total = 1;
    }
    for (int i = 0; i < rf->n_name_fragments && i < 8; i++)
        strncpy(f.name_fragments[i], rf->name_fragments[i], sizeof f.name_fragments[i] - 1);
    f.n_name_fragments = rf->n_name_fragments;
    if (rf->has_original_blade) {
        f.original_blade_limit = rf->original_blade_limit;
        if (rf->original_blade_op[0]) strncpy(f.original_blade_op, rf->original_blade_op, sizeof f.original_blade_op - 1);
        f.has_original_blade = 1;
    }
    if (rf->ability_filter[0]) {
        strncpy(f.ability_filter, rf->ability_filter, sizeof f.ability_filter - 1);
        for (int i = 0; i < rf->n_ability_filter_triggers && i < 8; i++)
            strncpy(f.ability_filter_triggers[i], rf->ability_filter_triggers[i],
                    sizeof f.ability_filter_triggers[i] - 1);
        f.n_ability_filter_triggers = rf->n_ability_filter_triggers;
    }
    f.negation = rf->negation;
    if (rf->has_exclude_self) { f.exclude_self_id = rf->exclude_self_id; f.has_exclude_self = 1; }
    if (rf->has_cost_total) { f.cost_total = rf->cost_total;
                               if (rf->cost_total_op[0]) strncpy(f.cost_total_op, rf->cost_total_op, sizeof f.cost_total_op - 1);
                               f.has_cost_total = 1; }

    int m = 0;
    for (int i = 0; i < n && m < max; i++) {
        if (card_filter_matches(&f, cards[i], 1)) out[m++] = cards[i];
    }
    return m;
}

/* Mirror util.rs::matching_indices — return indices into cards where filter matches. */
int rb_matching_indices_filter(const RbCardFilter *f, const int *cards, int n, int *out_idx, int max) {
    if (!cards || !out_idx) return 0;
    int ids[RB_MAX_ZONE];
    int m = rb_matching_ids(f, cards, n, ids, RB_MAX_ZONE);
    int r = 0;
    /* Map surviving ids back to original indices */
    for (int d = 0; d < m && r < max; d++) {
        for (int i = 0; i < n; i++) {
            if (cards[i] == ids[d]) { out_idx[r++] = i; break; }
        }
    }
    return r;
}

/* Mirror util.rs::count_matching — count cards matching the filter. */
int rb_count_matching_filter(const RbCardFilter *f, const int *cards, int n) {
    int ids[RB_MAX_ZONE];
    int m = rb_matching_ids(f, cards, n, ids, RB_MAX_ZONE);
    return m;
}

/* Legacy matching_indices (card_type + group only, backward compatible). */
int rb_matching_indices(const int *cards, int n, const char *card_type,
                        const char *group, int *out_idx, int max) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    return rb_matching_indices_filter(&f, cards, n, out_idx, max);
}

/* Legacy count_matching (card_type + group only). */
int rb_count_matching(const int *cards, int n, const char *card_type, const char *group) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    return rb_count_matching_filter(&f, cards, n);
}

/* ── Distinct filtering ────────────────────────────────────────────────── */

/* Mirror util.rs::distinct_should_dedupe. */
int rb_distinct_should_dedupe(RbDistinctType d) {
    return d == RB_DISTINCT_CARDNAME || d == RB_DISTINCT_TRUE || d == RB_DISTINCT_DISTINCT;
}

/* Mirror util.rs::dedupe_by_normalized_name. */
int rb_dedupe_by_normalized_name(const int *items, int n, int *out, int max) {
    if (!items || !out) return 0;
    char seen[RB_MAX_ZONE][256];
    int  nseen = 0;
    int  m = 0;
    for (int i = 0; i < n; i++) {
        int cid = items[i];
        Card c;
        int found = 0;
        if (rb_decode_card_by_index((uint32_t)cid, &c)) {
            if (c.name) {
                char nm[256];
                rb_norm_ws(c.name, nm, sizeof nm);
                int dup = 0;
                for (int s = 0; s < nseen; s++)
                    if (!strcmp(seen[s], nm)) { dup = 1; break; }
                if (!dup) {
                    if (nseen < RB_MAX_ZONE) {
                        strncpy(seen[nseen], nm, 255); seen[nseen][255] = 0; nseen++;
                    }
                    found = 1;
                }
            } else {
                found = 1;
            }
            rb_free_card(&c);
        } else {
            found = 1;
        }
        if (found && m < max) out[m++] = items[i];
    }
    return m;
}

/* Mirror util.rs::apply_distinct_filter. */
int rb_apply_distinct_filter(const int *cards, int n, RbDistinctType d,
                             int *out, int max) {
    if (!rb_distinct_should_dedupe(d)) {
        int m = 0;
        for (int i = 0; i < n && m < max; i++) out[m++] = cards[i];
        return m;
    }
    return rb_dedupe_by_normalized_name(cards, n, out, max);
}

/* Mirror util.rs::count_distinct_member_name_units — Q278/Q279. */
int rb_count_distinct_member_name_units(const int *cards, int n) {
    char seen[RB_MAX_ZONE][256];
    int  nseen = 0;
    int  joints[RB_MAX_ZONE];
    int  njoints = 0;
    for (int i = 0; i < n && i < RB_MAX_ZONE; i++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cards[i], &c)) continue;
        if (c.name) {
            char nm[256];
            rb_norm_ws(c.name, nm, sizeof nm);
            if (strchr(nm, '&')) {
                if (njoints < RB_MAX_ZONE) joints[njoints++] = cards[i];
            } else {
                int dup = 0;
                for (int s = 0; s < nseen; s++)
                    if (!strcmp(seen[s], nm)) { dup = 1; break; }
                if (!dup && nseen < RB_MAX_ZONE) {
                    strncpy(seen[nseen], nm, 255); seen[nseen][255] = 0; nseen++;
                }
            }
        }
        rb_free_card(&c);
    }
    int count = nseen;
    for (int j = 0; j < njoints; j++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)joints[j], &c)) continue;
        if (c.name) {
            char nm[256];
            rb_norm_ws(c.name, nm, sizeof nm);
            int has_new = 0;
            const char *p = nm;
            while (*p) {
                const char *sep = strchr(p, '&');
                size_t len = sep ? (size_t)(sep - p) : strlen(p);
                char tok[256];
                if (len >= sizeof tok) len = sizeof tok - 1;
                memcpy(tok, p, len); tok[len] = '\0';
                int in_set = 0;
                for (int s = 0; s < nseen; s++)
                    if (!strcmp(seen[s], tok)) { in_set = 1; break; }
                if (!in_set) {
                    if (nseen < RB_MAX_ZONE) {
                        strncpy(seen[nseen], tok, 255); seen[nseen][255] = 0; nseen++;
                    }
                    has_new = 1;
                }
                p = sep ? sep + 1 : p + strlen(p);
            }
            if (has_new) count++;
        }
        rb_free_card(&c);
    }
    return count;
}

/* Mirror util.rs::filter_distinct — return indices into cards, deduped by name. */
int rb_filter_distinct(const int *cards, int n, const char *card_type,
                       const char *group, RbDistinctType distinct,
                       int *out_idx, int max) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    if (!rb_distinct_should_dedupe(distinct))
        return rb_matching_indices_filter(&f, cards, n, out_idx, max);
    int matching[RB_MAX_ZONE];
    int mn = rb_matching_indices_filter(&f, cards, n, matching, RB_MAX_ZONE);
    int ids[RB_MAX_ZONE];
    for (int i = 0; i < mn; i++) ids[i] = cards[matching[i]];
    int deduped_ids[RB_MAX_ZONE];
    int dn = rb_apply_distinct_filter(ids, mn, distinct, deduped_ids, RB_MAX_ZONE);
    int m = 0;
    for (int i = 0; i < mn && m < max; i++) {
        for (int d = 0; d < dn; d++) {
            if (cards[matching[i]] == deduped_ids[d]) {
                out_idx[m++] = matching[i];
                break;
            }
        }
    }
    return m;
}

/* Mirror util.rs::count_matching_distinct. */
int rb_count_matching_distinct(const int *cards, int n, const char *card_type,
                               const char *group, RbDistinctType distinct) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    if (!rb_distinct_should_dedupe(distinct))
        return rb_count_matching_filter(&f, cards, n);
    int matching[RB_MAX_ZONE];
    int mn = rb_matching_indices_filter(&f, cards, n, matching, RB_MAX_ZONE);
    int ids[RB_MAX_ZONE];
    for (int i = 0; i < mn; i++) ids[i] = cards[matching[i]];
    int deduped[RB_MAX_ZONE];
    return rb_apply_distinct_filter(ids, mn, distinct, deduped, RB_MAX_ZONE);
}

/* ── Candidate pool / selection helpers ────────────────────────────────── */

/* Mirror util.rs::build_candidate_pool — filter the given card list through
   the effect's CardFilter. */
int rb_build_candidate_pool(const GameState *g, int pl, const RbCardFilter *f,
                            int *out, int max) {
    if (!g || !out || max <= 0) return 0;
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, "hand", ids, RB_MAX_ZONE);
    return rb_matching_ids(f, ids, n, out, max);
}

/* Mirror util.rs::matching_ids_filtered — matching ids with distinct,
   target_count truncation, and exclusion by id. */
int rb_matching_ids_filtered(const RbCardFilter *f, const int *cards, int n,
                             RbDistinctType distinct, const int *exclude_ids,
                             int n_exclude, int target_count, int *out, int max) {
    if (!cards || !out) return 0;
    RbCardFilter ff = *f;
    /* Apply exclude_ids as a post-filter */
    int m = rb_matching_ids(&ff, cards, n, out, max);
    if (n_exclude > 0 && distinct != RB_DISTINCT_NONE) {
        /* Also exclude cards whose names match excluded cards */
        char excl_names[RB_MAX_ZONE][256];
        int n_excl_names = 0;
        for (int e = 0; e < n_exclude && n_excl_names < RB_MAX_ZONE; e++) {
            Card c;
            if (rb_decode_card_by_index((uint32_t)exclude_ids[e], &c)) {
                if (c.name) {
                    rb_norm_ws(c.name, excl_names[n_excl_names], 256);
                    n_excl_names++;
                }
                rb_free_card(&c);
            }
        }
        if (n_excl_names > 0) {
            int w = 0;
            for (int i = 0; i < m; i++) {
                Card c;
                if (rb_decode_card_by_index((uint32_t)out[i], &c)) {
                    if (c.name) {
                        char nm[256];
                        rb_norm_ws(c.name, nm, sizeof nm);
                        int excl = 0;
                        for (int s = 0; s < n_excl_names; s++)
                            if (!strcmp(nm, excl_names[s])) { excl = 1; break; }
                        if (!excl) out[w++] = out[i];
                    } else {
                        out[w++] = out[i];
                    }
                    rb_free_card(&c);
                } else {
                    out[w++] = out[i];
                }
            }
            m = w;
        }
    }
    if (rb_distinct_should_dedupe(distinct)) {
        int deduped[RB_MAX_ZONE];
        int dn = rb_apply_distinct_filter(out, m, distinct, deduped, RB_MAX_ZONE);
        for (int i = 0; i < dn && i < max; i++) out[i] = deduped[i];
        m = dn;
    }
    if (target_count > 0 && m > target_count) m = target_count;
    return m;
}

/* ── Selection primitives ──────────────────────────────────────────────── */

/* Mirror util.rs::classify_selection — returns 0=Skip, 1=Exact, 2=Prompt. */
int rb_classify_selection(const int *idxs, int n, int count, int is_all) {
    if (is_all) return 1;
    if (n < count) return 0;
    if (n > count) return 2;
    return 1;
}

/* Mirror util.rs::get_selection_indices — backward-compatible card_type+group version. */
int rb_get_selection_indices(const int *cards, int n, const char *card_type,
                             const char *group, int self_target_only,
                             int activating_card, int *out_idx, int max) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    return rb_get_selection_indices_filter(cards, n, &f, self_target_only,
                                            activating_card, out_idx, max);
}

/* New: RbCardFilter-based get_selection_indices (full filter support). */
int rb_get_selection_indices_filter(const int *cards, int n, const RbCardFilter *filter,
                                     int self_target_only, int activating_card,
                                     int *out_idx, int max) {
    if (!cards || !out_idx) return 0;
    int ids[RB_MAX_ZONE];
    int m = rb_matching_ids(filter, cards, n, ids, RB_MAX_ZONE);
    int r = 0;
    for (int d = 0; d < m && r < max; d++) {
        for (int i = 0; i < n; i++) {
            if (cards[i] == ids[d]) { out_idx[r++] = i; break; }
        }
    }
    if (self_target_only && activating_card >= 0) {
        int w = 0;
        for (int i = 0; i < r; i++)
            if (cards[out_idx[i]] == activating_card)
                out_idx[w++] = out_idx[i];
        r = w;
    }
    return r;
}

/* Mirror util.rs::resolve_selection — backward-compatible card_type+group version. */
int rb_resolve_selection(const int *cards, int n, const char *card_type,
                         const char *group, int count, int is_all,
                         int self_target_only, int activating_card) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    return rb_resolve_selection_filter(cards, n, &f, count, is_all,
                                        self_target_only, activating_card);
}

/* New: RbCardFilter-based resolve_selection (full filter support). */
int rb_resolve_selection_filter(const int *cards, int n, const RbCardFilter *filter,
                                 int count, int is_all, int self_target_only,
                                 int activating_card) {
    int idxs[RB_MAX_ZONE];
    int mn = rb_get_selection_indices_filter(cards, n, filter, self_target_only,
                                               activating_card, idxs, RB_MAX_ZONE);
    return rb_classify_selection(idxs, mn, count, is_all);
}

/* Mirror util.rs::zone_remove_at_indices — remove from zone by indices (descending). */
int rb_zone_remove_at_indices(GameState *g, int pl, const char *zone,
                              const int *indices, int n_indices) {
    if (!g || !zone || !indices || n_indices <= 0) return 0;
    int sorted[RB_MAX_ZONE];
    int sn = 0;
    for (int i = 0; i < n_indices && sn < RB_MAX_ZONE; i++) sorted[sn++] = indices[i];
    for (int i = 0; i < sn - 1; i++)
        for (int j = i + 1; j < sn; j++)
            if (sorted[j] > sorted[i]) { int t = sorted[i]; sorted[i] = sorted[j]; sorted[j] = t; }
    int removed = 0;
    RbPlayer *P = &g->p[pl];
    for (int i = 0; i < sn; i++) {
        int idx = sorted[i];
        if (!strcmp(zone, "hand")) {
            if (idx >= 0 && idx < P->hand.n) {
                for (int k = idx; k < P->hand.n - 1; k++) P->hand.cards[k] = P->hand.cards[k + 1];
                P->hand.n--; removed++;
            }
        } else if (!strcmp(zone, "discard") || !strcmp(zone, "waitroom")) {
            if (idx >= 0 && idx < P->discard.n) {
                for (int k = idx; k < P->discard.n - 1; k++) P->discard.cards[k] = P->discard.cards[k + 1];
                P->discard.n--; removed++;
            }
        } else if (!strcmp(zone, "energy")) {
            if (idx >= 0 && idx < P->energy.n) {
                for (int k = idx; k < P->energy.n - 1; k++) P->energy.cards[k] = P->energy.cards[k + 1];
                P->energy.n--; removed++;
            }
        } else if (!strcmp(zone, "live") || !strcmp(zone, "live_card_zone")) {
            if (idx >= 0 && idx < P->live.n) {
                for (int k = idx; k < P->live.n - 1; k++) P->live.cards[k] = P->live.cards[k + 1];
                P->live.n--; removed++;
            }
        } else if (!strcmp(zone, "success") || !strcmp(zone, "success_live_zone") ||
                   !strcmp(zone, "success_live_card_zone")) {
            if (idx >= 0 && idx < P->success.n) {
                for (int k = idx; k < P->success.n - 1; k++) P->success.cards[k] = P->success.cards[k + 1];
                P->success.n--; removed++;
            }
        }
    }
    return removed;
}

/* ── Cost-reduction helpers ────────────────────────────────────────────── */

/* Mirror util.rs::find_modify_cost — search the effect tree for ModifyCost. */
const AbilityEffect *rb_find_modify_cost(const AbilityEffect *effect,
                                          const char *op, const char *loc) {
    if (!effect) return NULL;
    if (!strcmp(effect->action ? effect->action : "", "modify_cost")) {
        if (op) {
            const char *eff_op = eff_extra(effect, "operation");
            if (!eff_op || strcmp(eff_op, op)) return NULL;
        }
        if (loc) {
            const char *eff_loc = eff_extra(effect, "location");
            if (!eff_loc || strcmp(eff_loc, loc)) return NULL;
        }
        return effect;
    }
    for (int i = 0; i < effect->n_child; i++) {
        const AbilityEffect *found = rb_find_modify_cost(effect->child[i], op, loc);
        if (found) return found;
    }
    if (effect->primary_effect) {
        const AbilityEffect *found = rb_find_modify_cost(effect->primary_effect, op, loc);
        if (found) return found;
    }
    if (effect->alternative_effect) {
        const AbilityEffect *found = rb_find_modify_cost(effect->alternative_effect, op, loc);
        if (found) return found;
    }
    if (effect->followup_action) {
        const AbilityEffect *found = rb_find_modify_cost(effect->followup_action, op, loc);
        if (found) return found;
    }
    return NULL;
}

/* Mirror util.rs::play_cost_reduction_matches. */
static int rb_play_cost_reduction_matches(const AbilityEffect *effect, int card_id,
                                          const Card *card) {
    const char *gn = eff_extra(effect, "group");
    if (gn && !rb_card_matches_group_str(card_id, gn)) return 0;
    const char *cost_limit = eff_extra(effect, "cost_limit");
    if (cost_limit) {
        int limit = atoi(cost_limit);
        if (card->cost != limit) return 0;
    }
    if (!rb_cost_threshold_met(card, effect)) return 0;
    const char *ct = eff_extra(effect, "card_type");
    if (ct && strcmp(ct, "member_card")) return 0;
    const char *af = eff_extra(effect, "ability_filter");
    if (af && !strcmp(af, "no_ability")) {
        if (card->ability) return 0;
    }
    return 1;
}

/* Count stage cards matching a group name. */
static int rb_stage_count_group(const RbPlayer *P, const char *group_name) {
    if (!group_name) return 0;
    int count = 0;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = P->stage[i];
        if (cid == RB_EMPTY_SLOT) continue;
        if (rb_card_matches_group_str(cid, group_name)) count++;
    }
    return count;
}

/* Mirror util.rs::per_unit_cost_reduction. */
static int rb_per_unit_cost_reduction(const AbilityEffect *effect,
                                       const RbPlayer *P, int hand_count) {
    const char *pul  = eff_extra(effect, "per_unit_location");
    const char *loc  = eff_extra(effect, "location");
    const char *count_zone = pul ? pul : loc;
    if (!count_zone) count_zone = "hand";
    int raw_count;
    if (!strcmp(count_zone, "stage") && eff_extra(effect, "group")) {
        raw_count = rb_stage_count_group(P, eff_extra(effect, "group"));
    } else {
        raw_count = hand_count;
    }
    const char *puc = eff_extra(effect, "per_unit_count");
    int per_unit_count = puc ? atoi(puc) : 1;
    if (per_unit_count < 1) per_unit_count = 1;
    int exclude_self = 0;
    const char *es = eff_extra(effect, "exclude_self");
    if (es && !strcmp(es, "true")) exclude_self = 1;
    int effective = exclude_self ? (raw_count > 0 ? raw_count - 1 : 0) : raw_count;
    const char *val = eff_extra(effect, "value");
    int value = val ? atoi(val) : 1;
    return (effective / per_unit_count) * value;
}

/* Mirror util.rs::scan_abilities_for_cost_reduction. */
static int rb_scan_abilities_for_cost_reduction(uint32_t card_idx,
                                                 int target_id, const Card *target_card,
                                                 const GameState *g, int pl,
                                                 int hand_count, int hand_condition_guard) {
    int n_abilities = rb_card_num_abilities(card_idx);
    for (int i = 0; i < n_abilities; i++) {
        Ability ab;
        if (!rb_decode_card_ability(card_idx, i, &ab)) continue;
        int r = -1;
        if (ab.effect) {
            const AbilityEffect *mc = rb_find_modify_cost(ab.effect, "subtract", "hand");
            if (mc && rb_play_cost_reduction_matches(mc, target_id, target_card)) {
                if (hand_condition_guard) {
                    const AbilityEffect *cond_eff = mc->condition
                        ? (const AbilityEffect *)mc->condition : NULL;
                    /* Skip if the effect's condition requires location == hand
                       (the aura card is on stage, not in hand). */
                    (void)cond_eff;
                }
                if (mc->per_unit)
                    r = rb_per_unit_cost_reduction(mc, &g->p[pl], hand_count);
                else {
                    const char *val = eff_extra(mc, "value");
                    r = val ? atoi(val) : 1;
                }
            }
        }
        rb_free_ability(&ab);
        if (r >= 0) return r;
    }
    return 0;
}

/* Mirror util.rs::calculate_play_cost_reduction. */
int rb_calculate_play_cost_reduction(const GameState *g, int pl, int hand_count,
                                      int card_id) {
    Card card;
    if (!rb_decode_card_by_index((uint32_t)card_id, &card)) return 0;
    int reduction = 0;
    /* 1. Self-reduction from own abilities */
    {
        int n_abilities = rb_card_num_abilities((uint32_t)card_id);
        for (int i = 0; i < n_abilities; i++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
            if (ab.effect) {
                const AbilityEffect *mc = rb_find_modify_cost(ab.effect, "subtract", "hand");
                if (mc && rb_play_cost_reduction_matches(mc, card_id, &card)) {
                    if (mc->per_unit)
                        reduction = rb_per_unit_cost_reduction(mc, &g->p[pl], hand_count);
                    else {
                        const char *val = eff_extra(mc, "value");
                        reduction = val ? atoi(val) : 1;
                    }
                }
            }
            rb_free_ability(&ab);
        }
    }
    /* 2. Stage card auras */
    const RbPlayer *P = &g->p[pl];
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int stage_id = P->stage[s];
        if (stage_id == RB_EMPTY_SLOT) continue;
        int r = rb_scan_abilities_for_cost_reduction((uint32_t)stage_id,
                                                      card_id, &card, g, pl,
                                                      hand_count, 1);
        if (r > 0) reduction += r;
    }
    /* 3. Success live card auras */
    if (reduction == 0) {
        for (int i = 0; i < P->success.n; i++) {
            int live_id = P->success.cards[i];
            int r = rb_scan_abilities_for_cost_reduction((uint32_t)live_id,
                                                           card_id, &card, g, pl,
                                                           hand_count, 0);
            if (r > 0) { reduction = r; break; }
        }
    }
    rb_free_card(&card);
    return reduction;
}

/* Mirror util.rs::constant_per_unit_units — compute the units part of a
   constant per_unit gain (before base). Uses the effect's filter subset. */
int rb_constant_per_unit_units(const AbilityEffect *effect, const GameState *g, int pl,
                               int host_card_id) {
    const char *zone = rb_constant_per_unit_zone(effect);
    const RbPlayer *P = &g->p[pl];

    /* Build a minimal filter from the effect's fields */
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    const char *eff_ct = eff_extra(effect, "card_type");
    const char *eff_group = eff_extra(effect, "group");
    if (eff_ct)  strncpy(f.card_type, eff_ct, sizeof f.card_type - 1);
    if (eff_group) strncpy(f.group, eff_group, sizeof f.group - 1);
    f.has_filter = (eff_ct && eff_ct[0]) || (eff_group && eff_group[0]);
    f.exclude_self_id = host_card_id;
    f.has_exclude_self = (host_card_id >= 0);

    int per_count = 0;
    if (!strcmp(zone, "stage")) {
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, pl, "stage", ids, RB_MAX_ZONE);
        int matched[RB_MAX_ZONE];
        int mn = rb_matching_ids(&f, ids, n, matched, RB_MAX_ZONE);
        per_count = mn;
    } else if (!strcmp(zone, "hand")) {
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, pl, "hand", ids, RB_MAX_ZONE);
        int matched[RB_MAX_ZONE];
        per_count = rb_matching_ids(&f, ids, n, matched, RB_MAX_ZONE);
    } else if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        int ids[RB_MAX_ZONE];
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            for (int i = 0; i < P->under_cards[s].n && n < RB_MAX_ZONE; i++)
                ids[n++] = P->under_cards[s].cards[i];
        int matched[RB_MAX_ZONE];
        per_count = rb_matching_ids(&f, ids, n, matched, RB_MAX_ZONE);
    } else if (!strcmp(zone, "discard") || !strcmp(zone, "waitroom")) {
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, pl, "discard", ids, RB_MAX_ZONE);
        int matched[RB_MAX_ZONE];
        per_count = rb_matching_ids(&f, ids, n, matched, RB_MAX_ZONE);
    } else if (!strcmp(zone, "live_card_zone") || !strcmp(zone, "live")) {
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, pl, "live", ids, RB_MAX_ZONE);
        int matched[RB_MAX_ZONE];
        per_count = rb_matching_ids(&f, ids, n, matched, RB_MAX_ZONE);
    } else if (!strcmp(zone, "success_live_zone") || !strcmp(zone, "success_live_card_zone")) {
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, pl, "success", ids, RB_MAX_ZONE);
        int matched[RB_MAX_ZONE];
        per_count = rb_matching_ids(&f, ids, n, matched, RB_MAX_ZONE);
    } else if (!strcmp(zone, "energy") || !strcmp(zone, "energy_zone")) {
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, pl, "energy", ids, RB_MAX_ZONE);
        int matched[RB_MAX_ZONE];
        per_count = rb_matching_ids(&f, ids, n, matched, RB_MAX_ZONE);
    } else {
        per_count = rb_count_in_zone_filtered(g, pl, zone,
                                               eff_ct, eff_group);
    }

    const char *puc = eff_extra(effect, "per_unit_count");
    int per_unit_count = puc ? atoi(puc) : 1;
    if (per_unit_count < 1) per_unit_count = 1;
    int units = per_count / per_unit_count;
    const char *max_str = eff_extra(effect, "max");
    if (max_str && !strcmp(max_str, "true")) {
        const char *cap_str = eff_extra(effect, "count");
        if (cap_str) {
            int cap = atoi(cap_str);
            if (units > cap) units = cap;
        }
    }
    return units;
}

/* ── Per-unit calculation ──────────────────────────────────────────────── */

/* Mirror util.rs::calculate_per_unit_multiplier. */
int rb_calculate_per_unit_multiplier(const GameState *g, int pl, const char *per_unit_type,
                                      const char *state_filter) {
    if (!per_unit_type) return 1;
    const RbPlayer *P = &g->p[pl];
    if (!strcmp(per_unit_type, "member") || !strcmp(per_unit_type, "人") ||
        !strcmp(per_unit_type, "members")) {
        int count = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            if (state_filter) {
                const char *ori = rb_mods_get_orientation((RbMods *)&g->mods, cid);
                if (!rb_orientation_matches_state(ori, state_filter)) continue;
            }
            count++;
        }
        return count;
    }
    if (!strcmp(per_unit_type, "hand") || !strcmp(per_unit_type, "card") ||
        !strcmp(per_unit_type, "枚")) {
        return P->hand.n;
    }
    if (!strcmp(per_unit_type, "energy")) return P->energy.n;
    if (!strcmp(per_unit_type, "live_card_zone")) return P->live.n;
    if (!strcmp(per_unit_type, "discard")) return P->discard.n;
    if (!strcmp(per_unit_type, "under_member") || !strcmp(per_unit_type, "下")) {
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) n += P->under_cards[s].n;
        return n;
    }
    return 1;
}

/* Mirror util.rs::resolve_per_unit_count. */
int rb_resolve_per_unit_count(const GameState *g, int pl, const char *per_unit_type,
                              const char *card_type, const char *group,
                              const char *state_filter, int host_card_id) {
    if (!per_unit_type) return 1;
    const RbPlayer *P = &g->p[pl];
    if (!strcmp(per_unit_type, "stage") || !strcmp(per_unit_type, "member") ||
        !strcmp(per_unit_type, "人") || !strcmp(per_unit_type, "members")) {
        int count = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int cid = P->stage[i];
            if (cid == RB_EMPTY_SLOT) continue;
            if (card_type && !rb_card_matches_type(cid, card_type)) continue;
            if (group && !rb_card_matches_group_str(cid, group)) continue;
            if (state_filter) {
                const char *ori = rb_mods_get_orientation((RbMods *)&g->mods, cid);
                if (!rb_orientation_matches_state(ori, state_filter)) continue;
            }
            count++;
        }
        return count;
    }
    if (!strcmp(per_unit_type, "hand") || !strcmp(per_unit_type, "card")) {
        return rb_count_matching(P->hand.cards, P->hand.n, card_type, group);
    }
    if (!strcmp(per_unit_type, "under_member") || !strcmp(per_unit_type, "枚")) {
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            n += rb_count_matching(P->under_cards[s].cards, P->under_cards[s].n, card_type, group);
        return n;
    }
    if (!strcmp(per_unit_type, "discard")) {
        return rb_count_matching(P->discard.cards, P->discard.n, card_type, group);
    }
    if (!strcmp(per_unit_type, "live_card_zone")) {
        return rb_count_matching(P->live.cards, P->live.n, card_type, group);
    }
    if (!strcmp(per_unit_type, "success_live_zone") || !strcmp(per_unit_type, "success_live_card_zone")) {
        return rb_count_matching(P->success.cards, P->success.n, card_type, group);
    }
    return rb_calculate_per_unit_multiplier(g, pl, per_unit_type, state_filter);
}

/* Mirror util.rs::resolve_discard_per_unit_count. */
int rb_resolve_discard_per_unit_count(const GameState *g, int last_discard_count,
                                      const char *card_type, const char *group) {
    if (g->n_recently_moved > 0) {
        return rb_count_matching(g->recently_moved, g->n_recently_moved, card_type, group);
    }
    return last_discard_count;
}

/* ── Distinct name search (ported) ─────────────────────────────────────── */

/* Mirror util.rs::max_distinct_names — simplified greedy in C. */
int rb_max_distinct_names(const int *cards, int n) {
    if (n <= 0) return 0;
    char seen[RB_MAX_ZONE][256];
    int  nseen = 0;
    for (int i = 0; i < n && i < RB_MAX_ZONE; i++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cards[i], &c)) continue;
        if (c.name) {
            char nm[256];
            rb_norm_ws(c.name, nm, sizeof nm);
            int dup = 0;
            for (int s = 0; s < nseen; s++)
                if (!strcmp(seen[s], nm)) { dup = 1; break; }
            if (!dup) {
                if (nseen < RB_MAX_ZONE) {
                    strncpy(seen[nseen], nm, 255); seen[nseen][255] = 0; nseen++;
                }
            }
        }
        rb_free_card(&c);
    }
    return nseen;
}

/* Mirror util.rs::max_distinct_names_greedy — delegates to greedy. */
static int rb_max_distinct_names_greedy(const int *cards, int n) {
    return rb_max_distinct_names(cards, n);
}

/* Mirror util.rs::prune_dominated — remove masks that are strict subsets. */
static int cmp_u64(const void *a, const void *b) {
    uint64_t va = *(const uint64_t *)a, vb = *(const uint64_t *)b;
    return (va > vb) - (va < vb);
}
void rb_prune_dominated(uint64_t *masks, int *n) {
    if (*n <= 1) return;
    qsort(masks, *n, sizeof(uint64_t), cmp_u64);
    int m = 1;
    for (int i = 1; i < *n; i++)
        if (masks[i] != masks[m - 1]) masks[m++] = masks[i];
    *n = m;
    uint64_t kept[RB_MAX_ZONE];
    int nk = 0;
    for (int i = 0; i < *n; i++) {
        uint64_t mi = masks[i];
        int dominated = 0;
        for (int j = 0; j < nk; j++)
            if ((kept[j] & mi) == mi) { dominated = 1; break; }
        if (dominated) continue;
        int w = 0;
        for (int j = 0; j < nk; j++)
            if ((mi & kept[j]) != kept[j]) kept[w++] = kept[j];
        nk = w;
        kept[nk++] = mi;
    }
    memcpy(masks, kept, sizeof(uint64_t) * nk);
    *n = nk;
}

/* ── Temporary effects ─────────────────────────────────────────────────── */

/* Check whether a given effect_type has a revert handler. */
static int rb_is_revertable_effect_type(const char *effect_type) {
    if (!effect_type) return 0;
    if (!strcmp(effect_type, "blade_bonus")) return 1;
    if (!strcmp(effect_type, "heart_bonus")) return 1;
    if (!strcmp(effect_type, "score_bonus")) return 1;
    if (!strcmp(effect_type, "score_set")) return 1;
    if (!strcmp(effect_type, "cost_bonus")) return 1;
    if (!strcmp(effect_type, "cost_set")) return 1;
    if (!strcmp(effect_type, "heart_override")) return 1;
    if (!strcmp(effect_type, "modify_cost")) return 1;
    if (!strcmp(effect_type, "set_heart_type")) return 1;
    if (!strncmp(effect_type, "gain_blade", 10)) return 1;
    if (!strncmp(effect_type, "gain_heart", 10)) return 1;
    if (!strncmp(effect_type, "gain_ability:", 12)) return 1;
    if (!strncmp(effect_type, "set_blade_type:", 15)) return 1;
    if (!strncmp(effect_type, "modify_score_", 13)) return 1;
    return 0;
}

/* Mirror util::push_temporary_effect. */
void rb_util_push_temporary_effect(
    GameState *g,
    const char *effect_type,
    const char *duration,
    const char *target_player_id,
    const char *description)
{
    if (!g || !effect_type) return;
    if (!duration || !strcmp(duration, "permanent")) return;
    if (!rb_is_revertable_effect_type(effect_type)) {
        fprintf(stderr, "[warn] temporary effect type '%s' has no expiry revert handler\n", effect_type);
    }
    if (g->n_temp_effects >= RB_MAX_TEMP_EFFECTS) {
        fprintf(stderr, "[warn] temporary effects buffer full, dropping '%s'\n", effect_type);
        return;
    }
    RbTempEffect *te = &g->temp_effects[g->n_temp_effects++];
    te->card_id = -1;
    te->dur = rb_parse_duration(duration);
    te->blade = 0;
    te->score = 0;
    te->cost = 0;
    for (int c = 0; c < 8; c++) { te->heart[c] = 0; te->need_heart[c] = 0; }
    (void)target_player_id;
    (void)description;
}

/* Low-level: push a pre-parsed temporary effect. */
int rb_push_temporary_effect(GameState *g, int card_id, int dur, int blade,
                              int score, int cost, const int *heart,
                              const int *need_heart) {
    if (!g || g->n_temp_effects >= RB_MAX_TEMP_EFFECTS) return 0;
    RbTempEffect *e = &g->temp_effects[g->n_temp_effects];
    e->card_id = card_id;
    e->dur = dur;
    e->blade = blade;
    e->score = score;
    e->cost = cost;
    for (int i = 0; i < 8; i++) {
        e->heart[i] = heart ? heart[i] : 0;
        e->need_heart[i] = need_heart ? need_heart[i] : 0;
    }
    g->n_temp_effects++;
    return 1;
}

/* ── Card property / blade filter helpers ──────────────────────────────── */

/* Mirror util.rs::check_card_property. */
int rb_check_card_property(const char *prop, int negation, const Card *c) {
    int has = 0;
    if (!prop) has = 1;
    else if (!strcmp(prop, "has_blade_heart")) has = rb_card_has_blade_heart(c);
    else if (!strcmp(prop, "has_score_icon"))  has = rb_card_has_score_icon(c);
    else if (!strcmp(prop, "has_all_blade"))   has = rb_card_has_all_blade(c);
    return negation ? !has : has;
}

/* Mirror util.rs::filter_current_blade — post-filter by CURRENT blade total. */
int rb_filter_current_blade(const int *cands, int n, const GameState *g,
                            int blade_limit, const char *op, int *out, int max) {
    if (blade_limit < 0) {
        int m = 0;
        for (int i = 0; i < n && m < max; i++) out[m++] = cands[i];
        return m;
    }
    const char *o = op ? op : ">=";
    int m = 0;
    for (int i = 0; i < n; i++) {
        int cid = cands[i];
        int base = 0;
        Card c;
        if (rb_decode_card_by_index((uint32_t)cid, &c)) { base = c.blade; rb_free_card(&c); }
        int set       = rb_mods_get_blade_set((RbMods *)&g->mods, cid);
        int effective = set != 0 ? set : base;
        int additive  = rb_mods_get_blade((RbMods *)&g->mods, cid) - set;
        int total     = rb_saturate_u8(effective + additive);
        if (rb_compare_counts(o, total, blade_limit) && m < max) out[m++] = cid;
    }
    return m;
}

/* ── Identity / group helpers ──────────────────────────────────────────── */

void rb_set_card_identity(int cid, const char *name) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || !name || !*name) return;
    if (!g_ident[cid]) g_ident[cid] = (char *)malloc((size_t)RB_MAX_IDENT * 28);
    char *base = g_ident[cid];
    if (!base) return;
    for (int i = 0; i < g_ident_n[cid]; i++)
        if (!strcmp(base + (size_t)i * 28, name)) return;
    if (g_ident_n[cid] < RB_MAX_IDENT) {
        strncpy(base + (size_t)g_ident_n[cid] * 28, name, 27);
        base[(size_t)g_ident_n[cid] * 28 + 27] = 0;
        g_ident_n[cid]++;
    }
}

int rb_card_matches_identity_str(int card_id, const char *group_name) {
    if (card_id < 0 || card_id >= RB_MAX_CARD_IDS || !group_name) return 0;
    char *gn = norm_str(group_name);
    int match = 0;
    char *base = g_ident[card_id];
    for (int i = 0; i < g_ident_n[card_id]; i++) {
        char *slot = base ? base + (size_t)i * 28 : NULL;
        char *io = slot ? norm_str(slot) : NULL;
        if ((io && (strstr(io, gn) || strstr(gn, io))) ||
            (slot && (strstr(slot, group_name) || strstr(group_name, slot))))
            match = 1;
        if (io) rb_free(io);
        if (match) break;
    }
    if (gn) rb_free(gn);
    return match;
}

/* ── Filter construction helpers ───────────────────────────────────────── */

/* Mirror util.rs::filter_from_parts. */
int rb_filter_from_parts(const char *card_type, const char *group, int cost_limit,
                          const char *cost_op, RbCardFilter *out) {
    if (!out) return 0;
    memset(out, 0, sizeof(RbCardFilter));
    if (card_type) strncpy(out->card_type, card_type, sizeof out->card_type - 1);
    if (group) strncpy(out->group, group, sizeof out->group - 1);
    out->cost_limit = cost_limit;
    if (cost_op) strncpy(out->cost_op, cost_op, sizeof out->cost_op - 1);
    out->has_filter = (card_type && card_type[0]) || (group && group[0]) || cost_limit >= 0;
    return 1;
}

/* Mirror util.rs::filter_from_parts_full. */
int rb_filter_from_parts_full(const char *card_type, const char *group, int cost_limit,
                               const char *cost_op, int cost_total, const char *cost_total_op,
                               RbCardFilter *out) {
    if (!out) return 0;
    memset(out, 0, sizeof(RbCardFilter));
    if (card_type) strncpy(out->card_type, card_type, sizeof out->card_type - 1);
    if (group) strncpy(out->group, group, sizeof out->group - 1);
    out->cost_limit = cost_limit;
    if (cost_op) strncpy(out->cost_op, cost_op, sizeof out->cost_op - 1);
    out->cost_total = cost_total;
    if (cost_total_op) strncpy(out->cost_total_op, cost_total_op, sizeof out->cost_total_op - 1);
    out->has_filter = (card_type && card_type[0]) || (group && group[0]) || cost_limit >= 0;
    return 1;
}

/* ── Cannot-baton-touch helper ─────────────────────────────────────────── */

int rb_has_cannot_baton_touch_protection_util(int incoming, int existing) {
    return rb_has_cannot_baton_touch_protection(incoming, existing);
}

/* ── exclude-self helper ───────────────────────────────────────────────── */

int rb_exclude_self(const int *ids, int n, int self_id) {
    if (!ids) return 0;
    int kept = 0;
    for (int i = 0; i < n; i++) if (ids[i] != self_id) kept++;
    return kept;
}

/* ── HeartColor helpers ────────────────────────────────────────────────── */

RbHeartColor rb_parse_heart_color(const char *s) {
    if (!s) return RB_HEART_PINK;
    if (!strcmp(s, "heart00") || !strcmp(s, "h00")) return RB_HEART_PINK;
    if (!strcmp(s, "heart01") || !strcmp(s, "h01")) return RB_HEART_RED;
    if (!strcmp(s, "heart02") || !strcmp(s, "h02")) return RB_HEART_YELLOW;
    if (!strcmp(s, "heart03") || !strcmp(s, "h03")) return RB_HEART_GREEN;
    if (!strcmp(s, "heart04") || !strcmp(s, "h04")) return RB_HEART_BLUE;
    if (!strcmp(s, "heart05") || !strcmp(s, "h05")) return RB_HEART_PURPLE;
    if (!strcmp(s, "heart06") || !strcmp(s, "h06")) return RB_HEART_ORANGE;
    if (!strcmp(s, "b_all")) return RB_HEART_ALL;
    if (!strcmp(s, "draw"))  return RB_HEART_DRAW;
    if (!strcmp(s, "score")) return RB_HEART_SCORE;
    if (!strcmp(s, "all"))   return RB_HEART_ALL;
    if (!strcmp(s, "heart07") || !strcmp(s, "b_heart07")) return RB_HEART_PINK;
    if (s[0] == 'b' && s[1] == '_') return rb_parse_heart_color(s + 2);
    return RB_HEART_PINK;
}
