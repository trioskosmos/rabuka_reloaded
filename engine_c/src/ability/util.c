/* util.c — pure card/zone/comparison helpers.
   Mirror engine/src/ability/util.rs (compare_counts, card_matches_type,
   card_matches_group_str, card_at_position, pos_to_area, orientation_matches_state,
   zone_cards). These are shared by the condition evaluators and the effect
   executors; keeping them in one place mirrors the Rust single-source layout.

   Group matching is an approximation of the Rust card_matches_group_str: the C
   Card exposes group/unit/name strings (via rb_card_string) but not the series
   or set_card_identity-derived memberships, so only group/unit/name substring
   matches are performed. Fullwidth '！'(U+FF01) and micro 'µ'(U+00B5) are
   normalized to '!'/'μ' exactly like the Rust norm_group_name helper. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

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

/* card_property predicates — mirror card.rs::has_blade_heart /
   has_score_icon / has_all_blade. The C Card flattens base+blade+need hearts
   into heart_color[]/heart_count[] in that order, so blade hearts live at
   indices [num_base, num_base+num_blade). */
int rb_card_has_blade_heart(const Card *c) {
    if (c->num_blade > 0) return 1;                       /* blade_heart.is_some() */
    if (c->has_special && c->special_count > 0) return 1; /* special_heart non-empty */
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

/* Mirror util::card_matches_type. card_id is a card_no index. Uses the faithful
    type_flags low-2-bit encoding (0=Member, 1=Live, 2=Energy) exactly like
    rb_card_is_live/rb_card_is_energy in core/card.c. */
int rb_card_matches_type(int card_id, const char *filter) {
    if (!filter) return 1;
    int is_live  = rb_card_is_live(card_id);
    int is_energy = rb_card_is_energy(card_id);
    int is_member = !is_live && !is_energy;
    int r;
    if (!strcmp(filter, "live_card"))        r = is_live;
    else if (!strcmp(filter, "member_card")) r = is_member;
    else if (!strcmp(filter, "energy_card")) r = is_energy;
    else r = 1;
    return r;
}

/* Mirror util::orientation_matches_state. */
int rb_orientation_matches_state(const char *orientation, const char *state) {
    if (!orientation) return state && !strcmp(state, "active");
    return !strcmp(orientation, state);
}

/* Mirror util::card_series_matches_group — does `series` belong to `group`?
   Matches the canonical KNOWN_GROUPS taxonomy (μ's/Aqours/虹ヶ咲/Liella!/蓮ノ空).
   For μ's, each series line is checked individually to handle multi-series joint
   cards (e.g. a "ラブライブ！" line among other group lines). */
static int rb_card_series_matches_group(const char *series, const char *group) {
    if (!series || !group) return 0;
    if (!strcmp(group, "μ's")) {
        /* split on '\n' and test each line */
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
    if (!strcmp(group, "Aqours"))  return strstr(series, "サンシャイン") != NULL;
    if (!strcmp(group, "虹ヶ咲")) return strstr(series, "虹ヶ咲") != NULL;
    if (!strcmp(group, "Liella!")) return strstr(series, "スーパースター") != NULL;
    if (!strcmp(group, "蓮ノ空"))  return strstr(series, "蓮ノ空") != NULL;
    return 0;
}

/* Mirror util::card_matches_group_str (group/unit/name/series substring + exact
    unit/group equality, plus set_card_identity overrides). */
int rb_card_matches_group_str(int card_id, const char *group_name) {
    if (!group_name) return 1;
    Card c; if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;

    char *gn = norm_str(group_name);
    const char *g  = rb_card_string(c.group_idx);
    const char *u  = rb_card_string(c.unit_idx);
    const char *s  = rb_card_string(c.series_idx);
    char *gnorm = g  ? norm_str(g)  : NULL;
    char *unorm = u  ? norm_str(u)  : NULL;
    char *nnorm = c.name ? norm_str(c.name) : NULL;

    int match = 0;
    if (gnorm) {
        if ((g && (!strcmp(g, group_name) || strstr(g, group_name) || strstr(group_name, g))) ||
            (u && (!strcmp(u, group_name) || strstr(u, group_name) || strstr(group_name, u))) ||
            (c.name && (strstr(c.name, group_name) || strstr(group_name, c.name))))
            match = 1;
        /* normalized comparisons catch fullwidth-bang / micro mismatches */
        if (!match && (strstr(gnorm, gn) || strstr(gn, gnorm) ||
                       (unorm && (strstr(unorm, gn) || strstr(gn, unorm))) ||
                       (nnorm && (strstr(nnorm, gn) || strstr(gn, nnorm)))))
            match = 1;
        /* series membership (multi-series joint cards match via any line) */
        if (!match && s && rb_card_series_matches_group(s, group_name))
            match = 1;
    }
    /* set_card_identity overrides: a rewritten member counts as its new identity */
    if (!match) match = rb_card_matches_identity_str(card_id, group_name);
    rb_free_card(&c);
    if (gn) rb_free(gn);
    if (gnorm) rb_free(gnorm);
    if (unorm) rb_free(unorm);
    if (nnorm) rb_free(nnorm);
    return match;
}

/* set_card_identity override table — mirrors engine/src/ability/util.rs
   card_matches_identity_str. When a member's identity is rewritten (e.g. to a
   unit/group name), group/name matching must also accept those identities.
   Stored as raw group/unit name strings keyed by card_id. */
#define RB_MAX_IDENT 8
static char g_ident[RB_MAX_CARD_IDS][RB_MAX_IDENT][28];
static int  g_ident_n[RB_MAX_CARD_IDS];

void rb_set_card_identity(int cid, const char *name) {
    if (cid < 0 || cid >= RB_MAX_CARD_IDS || !name || !*name) return;
    for (int i = 0; i < g_ident_n[cid]; i++)
        if (!strcmp(g_ident[cid][i], name)) return;
    if (g_ident_n[cid] < RB_MAX_IDENT) {
        strncpy(g_ident[cid][g_ident_n[cid]], name, 27);
        g_ident[cid][g_ident_n[cid]][27] = 0;
        g_ident_n[cid]++;
    }
}

int rb_card_matches_identity_str(int card_id, const char *group_name) {
    if (card_id < 0 || card_id >= RB_MAX_CARD_IDS || !group_name) return 0;
    char *gn = norm_str(group_name);
    int match = 0;
    for (int i = 0; i < g_ident_n[card_id]; i++) {
        char *io = norm_str(g_ident[card_id][i]);
        if ((io && (strstr(io, gn) || strstr(gn, io))) ||
            (strstr(g_ident[card_id][i], group_name) ||
             strstr(group_name, g_ident[card_id][i])))
            match = 1;
        if (io) rb_free(io);
        if (match) break;
    }
    if (gn) rb_free(gn);
    return match;
}

/* Mirror util::card_at_position. pos in {"left_side","center","right_side"}.
   Returns card_no index or -1. */
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

/* Mirror util::pos_to_area → stage index (0 left, 1 center, 2 right). */
int rb_pos_to_area(const char *pos) {
    if (!pos) return 1;
    if (!strcmp(pos, "left_side"))  return 0;
    if (!strcmp(pos, "center"))     return 1;
    if (!strcmp(pos, "right_side")) return 2;
    return 1;
}

/* Mirror util::zone_cards. Fills out_ids (capacity max) with card_no indices
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
    } else if (!strcmp(zone, "success") || !strcmp(zone, "success_live_zone") || !strcmp(zone, "success_live_card_zone")) {
        for (int i = 0; i < P->success.n; i++) PUSH(P->success.cards[i]);
    }
    #undef PUSH
    return n;
}

/* Mirror card.rs parse_heart_color — string → RbHeartColor. */
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
    /* b_heart07 / heart07 → colorless (heart00 / pool index 0). */
    if (!strcmp(s, "heart07") || !strcmp(s, "b_heart07")) return RB_HEART_PINK;
    /* Blade hearts: strip the "b_" prefix and re-parse (e.g. b_heart03 → heart03). */
    if (s[0] == 'b' && s[1] == '_') return rb_parse_heart_color(s + 2);
    return RB_HEART_PINK; /* unknown → colorless, mirrors Rust default */
}

/* Mirror HeartColor::index() — colored hearts 0..6, All = 7, the colorless
   (Draw/Score/Any/BAll) buckets collapse to 0 to match Rust's index(). */
int rb_heart_index(RbHeartColor c) {
    switch (c) {
        case RB_HEART_PINK:
        case RB_HEART_RED:
        case RB_HEART_YELLOW:
        case RB_HEART_GREEN:
        case RB_HEART_BLUE:
        case RB_HEART_PURPLE:
        case RB_HEART_ORANGE:   return (int)c;          /* 0..6 */
        case RB_HEART_ALL:      return 7;
        default:                return 0;               /* DRAW/SCORE/ANY */
    }
}

/* ── Decoded-effect field reader (mirrors util.rs `*_any()` accessors) ──
   The VM decodes every effect field verbatim into extra_k/extra_v, so the
   Rust typed getters (operation_any, location_any, heart_type_any,
   original_count_any, original_operator_any, …) map directly to these keys. */
static const char *eff_extra(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
/* case-insensitive equality (strcasecmp is not portable here) */
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

/* Mirror util.rs::heart_gain_per_entry — split a total heart gain evenly across
   the listed colors (min 1 to avoid div-by-zero). */
int rb_heart_gain_per_entry(int total, int n_colors) {
    int len = n_colors > 0 ? n_colors : 1;
    return total / len;
}

/* Mirror util.rs::is_all_heart_type — "all" heart_type is the wildcard. */
int rb_is_all_heart_type(const AbilityEffect *e) {
    const char *ht = eff_extra(e, "heart_type");
    return ht && !strcmp(ht, "all");
}

/* Mirror util.rs::constant_per_unit_zone — loc.or(per_unit_type).unwrap_or("hand"). */
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

/* Mirror util.rs::activation_position_index — stage position → area index. */
int rb_activation_position_index(const char *p) {
    if (!p) return -1;
    /* trim leading/trailing whitespace */
    while (*p == ' ' || *p == '\t') p++;
    if (!strcmp(p, "left") || !strcmp(p, "left_side")) return 0;
    if (!strcmp(p, "center")) return 1;
    if (!strcmp(p, "right") || !strcmp(p, "right_side")) return 2;
    return -1;
}

/* Mirror util.rs::cost_threshold_met — original_count/operator gate on the
   card's cost (used by cost-reduction / play-cost auras). No threshold ⇒ pass. */
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

/* ── Card predicate helpers (mirror util.rs card_matches_*) ── */

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
    for (int k = 0; k < n; k++) {
        if (!c.name || !fragments[k] || !strstr(c.name, fragments[k])) { r = 0; break; }
    }
    rb_free_card(&c);
    return r;
}

/* Mirror util.rs::card_matches_characters — card name contains any of the listed
   character names (normalized). C tracks a single name, so the multi-name
   `get_card_names` universe collapses to `c.name`. */
int rb_card_matches_characters(int card_id, const char **names, int n) {
    if (n <= 0) return 1;
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int r = 0;
    if (c.name) {
        for (int k = 0; k < n; k++) {
            if (names[k] && strstr(c.name, names[k])) { r = 1; break; }
        }
    }
    rb_free_card(&c);
    return r;
}

/* Mirror util.rs::stage_position_index — stage-position string → area index. */
int rb_stage_position_index(const char *pos) {
    if (!pos) return -1;
    if (!strcmp(pos, "center") || !strcmp(pos, "センターエリア")) return 1;
    if (!strcmp(pos, "left_side") || !strcmp(pos, "左サイドエリア") || !strcmp(pos, "left")) return 0;
    if (!strcmp(pos, "right_side") || !strcmp(pos, "右サイドエリア") || !strcmp(pos, "right")) return 2;
    return -1;
}

/* ── Zone move/place helpers (mirror util.rs remove_card_from_zone /
//    place_card_in_zone / move_card / move_cards / count_in_zone /
//    resolve_indices_to_ids). Rust operates on Player with typed zones; C maps
//    waitroom→discard and uses under_cards[] for under_member. ── */
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

/* Mirror util.rs::card_matches_any_group — pass if `groups` is empty (no
    filter) or the card matches ANY entry. Replaces the repeated
    `group_names.first().map(|g| card_matches_group_str(...))` pattern. */
int rb_card_matches_any_group(int card_id, const char **groups, int n) {
    if (!groups || n <= 0) return 1;
    for (int i = 0; i < n; i++)
        if (groups[i] && rb_card_matches_group_str(card_id, groups[i])) return 1;
    return 0;
}

/* Strip ASCII whitespace, mirroring CardDatabase::normalize_name (used by
    util.rs::card_matches_name_constraint). Caller provides a big-enough buf. */
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

/* Mirror util.rs::card_matches_name_constraint — exact normalized match of the
    card name (or any '&' / '＆'-separated constituent) against the constraint.
    C cards carry a single name string, so multi-name cards are split here. */
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
            memcpy(tok, p, len);
            tok[len] = '\0';
            if (!strcmp(tok, cons)) { r = 1; break; }
            p = sep ? sep + (sep[0] == '&' ? 1 : 3) : p + strlen(p);
        }
    }
    rb_free_card(&c);
    return r;
}

/* ── Duration / distinct-name helpers (mirror engine/src/ability/util.rs) ── */

/* Mirror util.rs::parse_duration — "this_turn" reverts at turn rollover
    (RB_TEMP_TURN_END); "live_end"/"this_live" at live phase end
    (RB_TEMP_LIVE_END); "permanent"/"as_long_as" never expire (RB_TEMP_PERM).
    Unknown ⇒ live-end (matches Rust's ThisLive default). */
int rb_parse_duration(const char *s) {
    if (!s) return RB_TEMP_LIVE_END;
    if (!strcmp(s, "this_turn"))    return RB_TEMP_TURN_END;
    if (!strcmp(s, "live_end") || !strcmp(s, "this_live")) return RB_TEMP_LIVE_END;
    if (!strcmp(s, "as_long_as") || !strcmp(s, "permanent")) return RB_TEMP_PERM;
    return RB_TEMP_LIVE_END;
}

/* Canonical group taxonomy — exactly the groups recognized by
    rb_card_series_matches_group. ONE definition, mirrored from util.rs KNOWN_GROUPS. */
const char *RB_KNOWN_GROUPS[5] = { "μ's", "Aqours", "虹ヶ咲", "Liella!", "蓮ノ空" };

/* Mirror util.rs::distinct_should_dedupe — CardName/True/Distinct all dedupe. */
int rb_distinct_should_dedupe(RbDistinctType d) {
    return d == RB_DISTINCT_CARDNAME || d == RB_DISTINCT_TRUE || d == RB_DISTINCT_DISTINCT;
}

/* Mirror util.rs::count_distinct_member_name_units — joint-aware distinct-name
    count for "名前の異なるメンバーカード1枚につき" (Q278/Q279). Single-name cards
    dedup by name; a joint ("A&B&C") card adds one unit only when it introduces
    at least one name not already present as a single-name card. */
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

/* Mirror util.rs::apply_distinct_filter — when `d` is a dedupe variant, drop
    cards whose normalized name was already seen (mirrors dedupe_by_normalized_name,
    which keeps the full "&"-joined name for joint cards). Otherwise copy through. */
int rb_apply_distinct_filter(const int *cards, int n, RbDistinctType d,
                             int *out, int max) {
    if (!rb_distinct_should_dedupe(d)) {
        int m = 0;
        for (int i = 0; i < n && m < max; i++) out[m++] = cards[i];
        return m;
    }
    char seen[RB_MAX_ZONE][256];
    int  nseen = 0;
    int  m = 0;
    for (int i = 0; i < n; i++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cards[i], &c)) {
            if (m < max) out[m++] = cards[i];
            continue;
        }
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
                if (m < max) out[m++] = cards[i];
            }
        } else {
            if (m < max) out[m++] = cards[i];
        }
        rb_free_card(&c);
    }
    return m;
}

/* Mirror util.rs::CardFilter::check_card_property — single card-property
    predicate with optional negation ("does NOT have blade heart"). */
int rb_check_card_property(const char *prop, int negation, const Card *c) {
    int has = 0;
    if (!prop) has = 1;
    else if (!strcmp(prop, "has_blade_heart")) has = rb_card_has_blade_heart(c);
    else if (!strcmp(prop, "has_score_icon"))  has = rb_card_has_score_icon(c);
    else if (!strcmp(prop, "has_all_blade"))   has = rb_card_has_all_blade(c);
    return negation ? !has : has;
}

/* Mirror util.rs::filter_current_blade — post-filter `cands` by CURRENT blade
    total (printed base, or set + additive modifiers). Matches "ブレードをNつ以上
    持つ" (no 元々) semantics; returns the surviving ids written into `out`. */
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
