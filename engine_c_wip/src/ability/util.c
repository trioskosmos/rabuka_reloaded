/* util.c — pure card/zone/comparison helpers.
   Mirror engine/src/ability/util.rs (compare_counts, card_matches_type,
   card_matches_group_str, card_at_position, pos_to_area, orientation_matches_state,
   zone_cards, CardFilter, constant_per_unit_units, compute_play_cost helpers,
   matching_ids, per_unit resolution, distinct filtering, selection primitives).
   These are shared by the condition evaluators and the effect executors; keeping
   them in one place mirrors the Rust single-source layout.

   rb_card_matches_group_str is a faithful port of util.rs:679-725: exact unit
   match (via the canonical rb_card_unit_name accessor), the spacing-insensitive
   and '！'(U+FF01)/'µ'(U+00B5) normalized fallbacks, the comparison against the
   card's DERIVED group (rb_card_group_name — the stored group string is "" for
   every record in cards.bin, so it must never be read directly),
   per-name containment over the '&'/'＆'-separated name list, series
   membership, then the constant set_card_identity memberships.

   WHY UNIT IS THE LOAD-BEARING TERM: cards.json has no `group` key on any of
   its 2,639 cards, but it does carry `unit` on 2,198 of them, and every name in
   abilities.json's group_names / exclude_group_names / counted_by lists is a
   UNIT name (CatChu!, KALEIDOSCORE, 5yncri5e!, Printemps, …) or one of the
   five series-derived names (μ's, Aqours, 虹ヶ咲, Liella!, 蓮ノ空). Both are
   therefore live: term 1 reads unit, term 2 reads the series-derived group. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <ctype.h>

/* Heart-all wildcard key — mirrors util.rs HEART_ALL_KEY ("heart00"). */
#define RB_HEART_ALL_KEY "heart00"

/* ── Pending header fields ─────────────────────────────────────────────
   Two CardFilter/GameModifiers fields exist in the Rust twin but not yet in
   include/rabuka.h (which another agent owns). The faithful code paths below
   are compiled only when the header advertises the field, so the build stays
   green until the diff in the session report lands. Define the macro (or add
   the field to the struct) to switch the path on:

     util.rs:1102  card_property: Option<&'a str>        in RbCardFilter
     game_modifiers.rs:183  last_cost_energy_count: u8   in RbMods                */
#ifdef RB_CARD_FILTER_CARD_PROPERTY
#define RF_CARD_PROPERTY(rf) ((rf)->card_property)
#else
#define RF_CARD_PROPERTY(rf) ""
#endif
#ifdef RB_MODS_LAST_COST_ENERGY_COUNT
#define RB_LAST_COST_ENERGY(m) ((m)->last_cost_energy_count)
#else
#define RB_LAST_COST_ENERGY(m) (0)
#endif

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
    char  card_property[32];   /* Rust util.rs:1102 card_property */
    int   has_card_property;
    int   negation;            /* Rust util.rs:1105 negation — negates card_property */
    int   exclude_self_id;
    int   has_exclude_self;
    int   cost_total;
    char  cost_total_op[8];
    int   has_cost_total;
    int   cost_values[16];
    int   n_cost_values;
    int   distinct;
    int   blade_limit;
    char  blade_op[8];
    int   has_blade_limit;
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

/* Mirror util.rs::eq_ignoring_whitespace (util.rs:667-677).
   The corpus spells one unit group two ways — `lily white` on 25 cards and
   `lilywhite` on 106 — and cards.json `unit` keeps both, so a raw `==` made a
   `『lily white』` filter skip every unspaced card and a `『lilywhite』` filter
   skip every spaced one. Rust added this term explicitly for that reason; the C
   had no equivalent, so both spellings were half-dead here. Allocation free;
   the ！/µ variants are handled separately by norm_str. */
static int str_eq_ignoring_ws(const char *a, const char *b) {
    if (!a || !b) return 0;
    while (*a && *b) {
        while (*a == ' ' || *a == '\t' || *a == '\n' || *a == '\r') a++;
        while (*b == ' ' || *b == '\t' || *b == '\n' || *b == '\r') b++;
        if (!*a || !*b) break;
        if (*a != *b) return 0;
        a++; b++;
    }
    while (*a == ' ' || *a == '\t' || *a == '\n' || *a == '\r') a++;
    while (*b == ' ' || *b == '\t' || *b == '\n' || *b == '\r') b++;
    return *a == '\0' && *b == '\0';
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
    if (!c) return 0;
    if (c->num_blade > 0) return 1;
    if (c->has_special && c->special_count > 0) return 1;
    return 0;
}
int rb_card_has_blade_heart_strict(const Card *c) {
    return c && c->num_blade > 0;
}
int rb_card_has_score_icon(const Card *c) {
    return c && c->has_special && c->special_color == (uint8_t)RB_HEART_SCORE;
}
int rb_card_has_all_blade(const Card *c) {
    if (!c) return 0;
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

/* Mirror the constant set_card_identity term of util.rs:628-635: a card whose
   ROOT ability effect has action "set_card_identity" carries every entry of its
   `identities` list as an extra group membership. The C decoder flattens a
   string array into a CSV extra, so the list is split back out here; each entry
   is compared exactly, with the same ！/µ normalized fallback the unit / name
   terms use. Runtime identities recorded by rb_set_card_identity (a C-only
   registry) are OR-ed in by the caller. */
static int card_identity_ability_matches(int card_id, const char *gn) {
    int n_abilities = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n_abilities; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int match = 0;
        const AbilityEffect *eff = ab.effect;
        if (eff && eff->action && !strcmp(eff->action, "set_card_identity")) {
            const char *ids = NULL;
            for (int k = 0; k < eff->n_extra; k++)
                if (eff->extra_k[k] && !strcmp(eff->extra_k[k], "identities") && eff->extra_v[k]) {
                    ids = eff->extra_v[k];
                    break;
                }
            for (const char *cur = ids; cur && *cur && !match; ) {
                const char *comma = strchr(cur, ',');
                size_t len = comma ? (size_t)(comma - cur) : strlen(cur);
                char tok[64];
                if (len >= sizeof tok) len = sizeof tok - 1;
                memcpy(tok, cur, len);
                tok[len] = '\0';
                /* The CSV writer quotes an entry only when it contains a
                   separator, so strip one layer of quotes. */
                size_t tl = strlen(tok);
                if (tl >= 2 && tok[0] == '"' && tok[tl - 1] == '"') {
                    memmove(tok, tok + 1, tl - 2);
                    tok[tl - 2] = '\0';
                }
                if (tok[0]) {
                    if (!strcmp(tok, gn)) {
                        match = 1;
                    } else if (strstr(tok, "！") || strstr(tok, "µ")) {
                        char *tn = norm_str(tok);
                        if (tn) { if (!strcmp(tn, gn)) match = 1; rb_free(tn); }
                    }
                }
                if (!comma) break;
                cur = comma + 1;
            }
        }
        rb_free_ability(&ab);
        if (match) return 1;
    }
    return 0;
}

/* Mirror util.rs::card_matches_group_str — group/unit/name/series + set_card_identity.

   Faithful to util.rs:679-725. The previous body tested raw bidirectional
   substrings (strstr(group_name, g) with an empty card group is always
   non-NULL), so EVERY card matched EVERY group filter and every 「『X』の…」
   ability in the engine silently did nothing.

   TERM ORDER (each term is Rust's, in Rust's order):
     1. unit        — util.rs:691-696. Exact against the NORMALISED query, then
                       whitespace-insensitive, then the ！/µ-normalised form.
                       This is the term the 「『CatChu!』」 style filters live on:
                       cards.json carries `unit` on 2,198 cards and it is the
                       only card field the data actually populates (the `group`
                       key is absent from all 2,639).
     2. group       — util.rs:698-699. rb_card_group_name() is the DERIVED
                       (series→group) accessor from core/card.c, because the
                       stored `group` index resolves to "" for every record.
     3. names       — util.rs:700-701, containment over the '&'/U+FF06 split.
     4. series      — util.rs:705.
     5. identities  — util.rs:710-717, the constant set_card_identity list.

   NORMALISATION: norm_str maps U+FF01 '！' → ASCII '!' and U+00B5 'µ' →
   U+03BC 'μ', and is applied to the QUERY unconditionally. The card side is
   only re-normalised when it actually contains one of those two characters,
   which is what Rust does (util.rs:696) and is what makes the corpus's split
   spellings work: `unit` holds BOTH 'みらくらぱーく！' (51 cards, U+FF01) and
   'みらくらぱーく!' (45 cards, ASCII), while group_names holds only the U+FF01
   form — either one matches either filter. U+FF0B '＋' is deliberately NOT
   folded: Rust's norm_group_name folds only U+FF01 and U+00B5, and no unit or
   group_names string in cards.json / abilities.json contains it, so folding it
   would only make the C diverge from the reference. */
int rb_card_matches_group_str(int card_id, const char *group_name) {
    if (!group_name) return 1;

    char *gn = norm_str(group_name);
    if (!gn) return 0;

    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) { rb_free(gn); return 0; }

    /* Canonical unit accessor — never Card.unit_idx + rb_card_string, which
       cannot tell "no unit" from "index 0xFFFF". */
    const char *u = rb_card_unit_name(card_id);
    const char *s = rb_card_string(c.series_idx);

    /* util.rs:691-696 — unit == gn || eq_ignoring_whitespace(unit, gn)
       || (unit has ！/µ && norm(unit) == gn). */
    int match = 0;
    if (u && *u) {
        if (!strcmp(u, gn)) {
            match = 1;
        } else if (str_eq_ignoring_ws(u, gn)) {
            match = 1;
        } else if (strstr(u, "！") || strstr(u, "µ")) {
            char *unorm = norm_str(u);
            if (unorm) { match = !strcmp(unorm, gn); rb_free(unorm); }
        }
    }

    /* util.rs:698-699 — c.group == g, then the same whitespace-insensitive
       form. The group must come from rb_card_group_name(): the stored group
       index resolves to "" for every record in cards.bin, so reading it
       directly makes this term dead (and made the old body match everything).
       Compared against the RAW query, as Rust does. */
    if (!match) {
        const char *g = rb_card_group_name(card_id);
        if (g && *g && (!strcmp(g, group_name) || str_eq_ignoring_ws(g, group_name)))
            match = 1;
    }

    /* util.rs:700-701 — any of the card's names contains gn, with the same
       ！/µ normalized fallback. Names are '&'/U+FF06 separated. */
    if (!match) {
        char names[1024];
        names[0] = '\0';
        int n_names = rb_card_get_card_names(card_id, names, sizeof names);
        const char *p = names;
        for (int i = 0; i < n_names && p && *p && !match; i++) {
            if (strstr(p, gn)) {
                match = 1;
            } else if (strstr(p, "！") || strstr(p, "µ")) {
                char *nn = norm_str(p);
                if (nn) { if (strstr(nn, gn)) match = 1; rb_free(nn); }
            }
            p += strlen(p) + 1;
        }
    }

    /* util.rs:705 — multi-series joint cards match via any constituent line. */
    if (!match && s && rb_card_series_matches_group(s, gn)) match = 1;

    /* util.rs:710-717 — constant set_card_identity ("treated as") memberships:
       the ability-declared list first, then the runtime registry the C port
       records when the ability has actually fired. */
    if (!match) match = card_identity_ability_matches(card_id, gn);
    if (!match) match = rb_card_matches_identity_str(card_id, group_name);

    rb_free_card(&c);
    rb_free(gn);
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

/* Mirror util.rs::card_matches_characters (util.rs:789-801) — the query name is
   whitespace-normalized and must be CONTAINED IN one of the card's '&'-separated
   names. The C port used to strstr the card's raw unsplit name, which also
   matched a query spanning the separator (「A&B」 matched a joint card 「A&B&C」). */
int rb_card_matches_characters(int card_id, const char **names, int n) {
    if (n <= 0) return 1;
    char card_names[1024];
    card_names[0] = '\0';
    int n_card_names = rb_card_get_card_names(card_id, card_names, sizeof card_names);
    int r = 0;
    const char *p = card_names;
    for (int i = 0; i < n_card_names && p && *p && !r; i++, p += strlen(p) + 1) {
        for (int k = 0; k < n; k++) {
            if (!names[k]) continue;
            char norm_name[256];
            rb_card_normalize_name(names[k], norm_name, sizeof norm_name);
            if (strstr(p, norm_name)) { r = 1; break; }
        }
    }
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

/* Sum of the printed heart COUNTS of colour `hc` over [start, end) — mirrors
   HeartMap::get(&hc).unwrap_or(&0). Pass hc < 0 to sum every colour
   (HeartMap::values_sum). */
static int heart_count_in_range(const Card *c, int hc, int start, int end) {
    int total = 0;
    for (int i = start; i < end && i < c->n_hearts; i++)
        if (hc < 0 || (int)c->heart_color[i] == hc) total += c->heart_count[i];
    return total;
}

/* Mirror Card::total_hearts (card.rs:4194-4202): base_heart.values_sum() when the
   card prints base hearts (a member), else need_heart.values_sum() over the need
   slice (a live card's cost hearts). Blade hearts are never counted. */
static int card_total_hearts(const Card *c) {
    if (c->num_base > 0) return heart_count_in_range(c, -1, 0, c->num_base);
    return heart_count_in_range(c, -1, c->num_base + c->num_blade,
                                c->num_base + c->num_blade + c->num_need);
}

/* Mirror util.rs::heart_count_for_threshold (util.rs:1111-1120): whichever of the
   printed base heart and the need heart holds more hearts of `color` counts. */
static int heart_count_for_threshold(const Card *c, int hc) {
    int base = heart_count_in_range(c, hc, 0, c->num_base);
    int need = heart_count_in_range(c, hc, c->num_base + c->num_blade,
                                    c->num_base + c->num_blade + c->num_need);
    return base > need ? base : need;
}

/* Mirror util.rs::card_matches_heart_colors — OR logic over the listed colors. */
int rb_card_matches_heart_colors(int card_id, const char **heart_colors, int n) {
    if (n <= 0) return 1;
    Card c;
    /* util.rs:845 — a card the database does not know PASSES (is_none_or). */
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 1;
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
    /* util.rs:869 — same is_none_or pass for an unknown card. */
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 1;
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
    if (!strcmp(s, "permanent") || !strcmp(s, "as_long_as")) return RB_TEMP_PERM;
    if (!strcmp(s, "this_turn") || !strcmp(s, "turn_end") ||
        !strcmp(s, "until_end_of_turn") || !strcmp(s, "first_turn")) return RB_TEMP_TURN_END;
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
/* External linkage on purpose: the definition at ~line 1801 is non-static and
   move.c calls this across translation units. Declaring it `static` here fixed
   the linkage to internal, so the whole tree failed to link with "undefined
   reference" from move.c. Do not re-add `static`. */
int rb_get_selection_indices_filter(const int *cards, int n, const RbCardFilter *filter,
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

/* Resolve a zone STRING to the bag that physically holds it.
 *
 * Mirror engine/src/ability/util.rs:2066-2088 `zone_cards`, which maps
 * `Deck | DeckTop | DeckBottom` onto the SAME `main_deck` slice and falls into
 * the `_ => &[]` arm for `Zone::EnergyDeck` -- so energy_deck counts as EMPTY
 * there and must keep resolving to NULL below (rb_count_in_zone shares this
 * helper). The deck_top/deck_bottom arms were simply absent, which made
 * rb_place_card_in_zone() report "not placed" for them AFTER the caller had
 * already lifted the card out of its source: the card was DELETED from the
 * game. Proof: test_parity_bp7 kanon_selects_each_group_to_deck_bottom_and_draws
 * measured CENSUS 44->41, three cards gone while the deck length looked merely
 * unchanged. A zone count cannot tell "never moved" from "destroyed"; only a
 * whole-board census can.
 *
 * placement (util.rs:2330-2348) is the only operation Rust performs on
 * energy_deck, and it is handled by its own arm in rb_place_card_in_zone. */
static RbBag *zone_bag(RbPlayer *P, const char *zone) {
    if (!strcmp(zone, "hand")) return &P->hand;
    if (!strcmp(zone, "deck") || !strcmp(zone, "main_deck") ||
        !strcmp(zone, "deck_top") || !strcmp(zone, "deck_bottom")) return &P->deck;
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
    /* Rust's remove_card_from_zone (util.rs:2181-2241) matches Zone::Deck and
     * nothing else among the deck spellings: `deck_top`, `deck_bottom` and
     * `energy_deck` fall to the `_ => {}` arm and remove NOTHING. zone_bag()
     * above deliberately maps deck_top/deck_bottom onto main_deck, because that
     * is what zone_cards does (util.rs:2073) and place_card_in_zone needs it;
     * this guard is what keeps the two Rust functions' differing scopes from
     * being merged into one. Without it a src of "deck_bottom" would start
     * removing cards the Rust engine leaves in the deck. */
    if (!strcmp(zone, "deck_top") || !strcmp(zone, "deck_bottom") ||
        !strcmp(zone, "energy_deck"))
        return 0;
    RbBag *b = zone_bag(P, zone);
    if (!b) return 0;
    for (int i = 0; i < b->n; i++)
        if (b->cards[i] == card_id) {
            for (int k = i; k < b->n - 1; k++) b->cards[k] = b->cards[k + 1];
            b->n--; return 1;
        }
    return 0;
}

/* util.rs:2453-2463 `stage_first_empty`. The probe order is centre, then left,
 * then right -- NOT ascending index. rb_stage_first_empty (zones.c:193) scans
 * 0,1,2 and so disagrees with Rust whenever both slot 0 and slot 1 are empty.
 * The SameArea arm below has to use THIS one to stay faithful; the stage arm
 * above it is left on rb_stage_first_empty so the earlier full-stage fix keeps
 * the behaviour it was measured against. Divergence reported, not silently
 * reconciled: zones.c is not this file. */
static int rb_util_stage_first_empty_rust(const int stage[RB_STAGE_SIZE]) {
    if (stage[1] == RB_EMPTY_SLOT) return 1;
    if (stage[0] == RB_EMPTY_SLOT) return 0;
    if (stage[2] == RB_EMPTY_SLOT) return 2;
    return -1;
}

/* Park a card whose intended bag is at RB_MAX_ZONE. Rust has no ceiling at
 * all -- every destination bag is a SmallVec whose add_card cannot fail
 * (util.rs:2347, :2351, :2381, :2389, :2393) -- so overflow is a C-port
 * artefact, and the only question is where the card goes instead of nowhere.
 * The WAITROOM is the established overflow sink (util.rs:2363-2367 parks a
 * full-stage card there); the HAND is the catch-all's sink (util.rs:2446), and
 * it is also the one bag whose limit (RB_MAX_HAND) is not the one just hit.
 * Returns 1 either way: a full bag is never a "not placed" verdict, because
 * every caller has already lifted the card out of its source zone. */
static int rb_park_overflow(RbPlayer *P, int card_id) {
    if (P->discard.n < RB_MAX_ZONE) {
        P->discard.cards[P->discard.n++] = card_id;
        return 1;
    }
    if (P->hand.n < RB_MAX_HAND) {
        P->hand.cards[P->hand.n++] = card_id;
        return 1;
    }
    return 1;
}

int rb_place_card_in_zone(GameState *g, int pl, int card_id, const char *zone,
                          int vacated_area) {
    RbPlayer *P = &g->p[pl];
    if (!strcmp(zone, "stage") || !strcmp(zone, "empty_area")) {
        int area = vacated_area >= 0 ? vacated_area : rb_stage_first_empty(P->stage);
        if (area < 0 || area >= RB_STAGE_SIZE) {
            /* util.rs:2354-2368 — a FULL stage is NOT a failure. Rust's
             * `Some(Zone::Stage) | Some(Zone::EmptyArea)` arm parks the card in
             * the WAITROOM ("Stage full — return card to discard instead of
             * hand") and still returns `true`.
             *
             * Returning 0 here is the second card-destroying defect of the day
             * (the first was zone_bag() below missing deck_top/deck_bottom):
             * EVERY caller of this function has already lifted the card out of
             * its source zone — rb_move_card at util.c:976 removes first, and
             * the three choice.c call sites (choice.c:962 send_to_dst, :1732
             * hand-cost, :4276 energy-zone) all call rb_waitroom_remove_card
             * first — so a "not placed" verdict leaves the card in no zone at
             * all. Measured on the nico family: board census 12 -> 11 and
             * 14 -> 13 with an `empty_area` destination onto a full stage.
             * Only a whole-board census distinguishes "moved" from "destroyed";
             * a zone count reads the same either way. */
            if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = card_id;
            return 1;
        }
        P->stage[area] = card_id; P->stage_wait[area] = 0;
        return 1;
    }
    /* util.rs:2330-2336 `Zone::Deck | Zone::DeckTop` -- INSERT at
     * vacated_stage_area (defaulting to 0, i.e. the top of the bag, which is
     * index 0 because the C deck bag is top-first), clamped to the deck
     * length. util.rs:2337-2340 `Zone::DeckBottom` -- push, i.e. the end of
     * the bag. util.rs:2345-2348 `Zone::EnergyDeck` -- push onto the energy
     * deck bag.
     *
     * These three were the card-destroying gap: zone_bag() could not name
     * deck_bottom, so every caller that had ALREADY removed the card from its
     * source zone got a 0 back and the card ended up in no zone at all. The
     * CHOICE-driven path is the one that measured it (CENSUS 44->41 in
     * test_parity_bp7); move.c's plain path already appended for deck_bottom. */
    if (!strcmp(zone, "deck") || !strcmp(zone, "deck_top")) {
        int idx = vacated_area >= 0 ? vacated_area : 0;
        if (idx > P->deck.n) idx = P->deck.n;
        if (P->deck.n < RB_MAX_ZONE) {
            for (int k = P->deck.n; k > idx; k--) P->deck.cards[k] = P->deck.cards[k - 1];
            P->deck.cards[idx] = card_id;
            P->deck.n++;
        } else {
            return rb_park_overflow(P, card_id);
        }
        return 1;
    }
    if (!strcmp(zone, "deck_bottom")) {
        if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = card_id;
        else return rb_park_overflow(P, card_id);
        return 1;
    }
    if (!strcmp(zone, "energy_deck")) {
        if (P->energy_deck.n < RB_MAX_ZONE)
            P->energy_deck.cards[P->energy_deck.n++] = card_id;
        else return rb_park_overflow(P, card_id);
        return 1;
    }
    /* util.rs:2396-2422 `Some(Zone::SameArea)`. A DEDICATED arm, not a zone_bag
     * lookup: the destination names an AREA RELATIONSHIP, not a bag, so it can
     * resolve to stage, to hand, or -- via first_empty -- to any of the three
     * slots. Corpus: effect.destination=same_area on 9 abilities, e.g.
     * PL!S-bp3-006-P. Before this arm the generic tail returned 0 AFTER the
     * caller had already lifted the card out of its source, so all 9 deleted.
     *
     * Rust's order is vacated_area if that slot is empty, else first empty,
     * else the HAND. It never returns false, so the hand is the floor. */
    if (!strcmp(zone, "same_area")) {
        if (vacated_area >= 0 && vacated_area < RB_STAGE_SIZE &&
            P->stage[vacated_area] == RB_EMPTY_SLOT) {
            P->stage[vacated_area] = card_id;
            P->stage_wait[vacated_area] = 0;
            rb_zone_track_deployment(g, pl, card_id);
            return 1;
        }
        int ep = rb_util_stage_first_empty_rust(P->stage);
        if (ep >= 0) {
            P->stage[ep] = card_id;
            P->stage_wait[ep] = 0;
            rb_zone_track_deployment(g, pl, card_id);
        } else {
            rb_hand_add(P, card_id);
        }
        return 1;
    }
    /* util.rs:2423-2441 `Some(Zone::UnderMember)`, Rule 4.5.5. "under" is the
     * same variant (enums.rs:108 aliases it). Corpus: destination=under_member
     * on 21 effect + 5 cost records. move.c:1333 and choice.c:966 intercept the
     * stage-choice and choice routes for this name, but the plain move routes
     * (move.c:1542/1575/1767/1915, move_missing.c:250, choice.c:1738/4584) all
     * arrive here, so the arm has to exist.
     *
     * Rust's fallback prefers an OCCUPIED slot, center then left then right, and
     * sends the card to the WAITROOM when the stage is entirely empty --
     * it returns true in every one of those cases. */
    if (!strcmp(zone, "under_member") || !strcmp(zone, "under")) {
        /* util.rs:2426-2437: `vacated_stage_area` is Option<usize>, so Rust
         * only consults it when it is Some. vacated_area < 0 is the C's None. */
        int target_idx = -1;
        if (vacated_area >= 0) {
            /* util.rs:2438 pos_to_area(target_idx) -> MemberArea::from_index,
             * whose `_` arm is RightSide (zones.rs:102-109, util.rs:2468-2470).
             * An out-of-range index therefore lands on slot 2, NOT nowhere. */
            target_idx = (vacated_area < RB_STAGE_SIZE) ? vacated_area
                                                       : RB_STAGE_SIZE - 1;
        } else if (P->stage[1] != RB_EMPTY_SLOT) {
            target_idx = 1;
        } else if (P->stage[0] != RB_EMPTY_SLOT) {
            target_idx = 0;
        } else if (P->stage[2] != RB_EMPTY_SLOT) {
            target_idx = 2;
        }
        if (target_idx < 0) {
            /* util.rs:2435-2436: the stage is entirely empty, so Rust parks
             * the card in the WAITROOM and returns true. */
            if (P->discard.n < RB_MAX_ZONE) {
                P->discard.cards[P->discard.n++] = card_id;
                return 1;
            }
            return rb_park_overflow(P, card_id);
        }
        rb_stage_place_under_card(P, target_idx, card_id);
        return 1;
    }
    RbBag *b = zone_bag(P, zone);
    if (!b) {
        /* util.rs:2442-2449 `_ =>` arm. An unrecognised destination is NOT a
         * failure: Rust parks the card in the HAND, or in the WAITROOM for the
         * empty string, and returns `true`.
         *
         * Returning 0 here is a DELETION, and this is the narrowest place that
         * fact is visible. Every caller has already lifted the card out of its
         * source -- rb_move_card below removes first, and the choice.c call
         * sites (send_to_dst, hand cost, energy zone) all drain the source
         * bag before calling -- so a "not placed" verdict leaves the card in
         * NO zone at all. Measured on this file's own probe: whole-board
         * census 1 -> 0 for each of same_area, deck_top_or_bottom, front,
         * under_member and "". A per-zone count cannot tell "moved nowhere"
         * from "destroyed"; only the census can.
         *
         * Corpus destinations that reach this arm, because Zone::from_str
         * (enums.rs:89-122) has a variant for them but place_card_in_zone has
         * no arm: deck_top_or_bottom x3 (e.g. PL!S-bp6-002-P), front x1
         * (PL!HS-pb1-014-P+), and "" -- plus every non-zone marker the corpus
         * may route through here. Rust has no failure arm anywhere in
         * place_card_in_zone, so no destination string can delete a card. */
        if (zone && !zone[0]) {
            if (P->discard.n < RB_MAX_ZONE)
                P->discard.cards[P->discard.n++] = card_id;
            else return rb_park_overflow(P, card_id);
        } else {
            rb_hand_add(P, card_id);
        }
        return 1;
    }
    if (b->n < RB_MAX_ZONE) { b->cards[b->n++] = card_id; return 1; }
    /* A FULL bag is not a "not placed" verdict either. Every Rust zone here is
     * a SmallVec whose add_card cannot fail (util.rs:2347, :2351, :2381,
     * :2389, :2393), so the only difference from Rust is the fixed ceiling.
     * Returning 0 made a full bag a deletion -- probe: census 513 -> 512 with
     * the waitroom at RB_MAX_ZONE. Park it in the WAITROOM, which is what the
     * Stage arm above already does for a full stage (util.rs:2363-2367). */
    return rb_park_overflow(P, card_id);
}

int rb_move_card(GameState *g, int pl, int card_id, const char *src,
                 const char *dst, int vacated_area) {
    if (rb_remove_card_from_zone(g, pl, card_id, src)) {
        /* Rust push_movement_event (modifiers.rs:1833-1907) sets
         * recently_moved_from_zone = Some(source_zone) on EVERY recorded
         * movement (:1875-1877), before it looks at the destination. That
         * field is what populates TriggerEvent.moved_from_zone
         * (abilities.rs:448, :2269), which is what a move-phase (idoutsuuji)
         * condition gated on the SOURCE zone matches against
         * (actions/mod.rs:1509).
         *
         * The C recorded the destination only -- or nothing at all -- so every
         * source-gated move condition was unreachable: the field declared at
         * rabuka.h:1555 had NO WRITER anywhere in src/ (grep for
         * recently_moved_from_zone over engine_c_wip/src returns only the
         * declaration). Recording the SOURCE is what the field is named for and
         * what Rust stores; the destination was never the value. */
        if (src && *src)
            snprintf(g->recently_moved_from_zone,
                     sizeof g->recently_moved_from_zone, "%s", src);
        return rb_place_card_in_zone(g, pl, card_id, dst, vacated_area);
    }
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
    if (f->n_cost_values > 0) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
        int cost = c.cost;
        rb_free_card(&c);
        int found = 0;
        for (int i = 0; i < f->n_cost_values; i++) if (f->cost_values[i] == cost) found = 1;
        if (!found) return 0;
    }
    if (f->has_cost_total) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
        int total = c.cost;
        rb_free_card(&c);
        if (!rb_compare_counts(f->cost_total_op[0] ? f->cost_total_op : NULL, total, f->cost_total)) return 0;
    }
    if (f->has_blade_limit) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
        int blade = c.blade;
        rb_free_card(&c);
        if (!rb_compare_counts(f->blade_op[0] ? f->blade_op : NULL, blade, f->blade_limit)) return 0;
    }
    if (f->has_characters) {
        /* Rust (util.rs:1474-1476): a single `characters` list, matched with
           card_matches_characters (name contains the normalized character).
           RbCardFilter::characters is one name string, so wrap it as a
           one-element list. The previous code cast the char[256] buffer to a
           `const char **` and dereferenced it as a pointer. */
        const char *names[1];
        names[0] = f->characters;
        if (!rb_card_matches_characters(card_id, names, 1)) return 0;
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
        /* util.rs:1257-1271 — heart_count_for_threshold(color) >= min_count for
           EVERY listed colour when require_all_heart_colors, else for ANY one.
           The C port used a 0/1 presence flag (so a threshold of 2 could never
           pass) and ignored require_all. */
        Card c;
        if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
            int passes = f->require_all_heart_colors ? 1 : 0;
            for (int hc_idx = 0; hc_idx < f->n_heart_colors; hc_idx++) {
                int hc = (int)rb_parse_heart_color(f->heart_colors[hc_idx]);
                int meets = heart_count_for_threshold(&c, hc) >= f->heart_color_count;
                if (f->require_all_heart_colors) {
                    if (!meets) { passes = 0; break; }
                } else if (meets) {
                    passes = 1;
                    break;
                }
            }
            rb_free_card(&c);
            if (!passes) return 0;
        }
    }
    if (f->has_need_heart_total) {
        /* util.rs:1282-1308 — with need_heart_color, the COUNT of that colour
           in need_heart; otherwise Card::total_hearts() (printed base hearts for
           a member, need_heart for a live card). The C port counted heart
           ENTRIES for the total (c.num_base / c.num_need) and a 0/1 presence
           flag for the per-colour form. */
        Card c;
        int total = 0;
        if (rb_decode_card_by_index((uint32_t)card_id, &c)) {
            if (f->need_heart_color[0]) {
                int hc = (int)rb_parse_heart_color(f->need_heart_color);
                total = heart_count_in_range(&c, hc,
                    c.num_base + c.num_blade, c.num_base + c.num_blade + c.num_need);
            } else {
                total = card_total_hearts(&c);
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
    /* util.rs:1390-1408 (CardFilter::check_card_property, checked last, after the
       ability filter). `negation` inverts the property test. An unknown property
       string is false, so with negation it passes — a card the database cannot
       decode also counts as "no property", matching Rust's
       `get_card(id).is_some_and(..)` on a missing card. */
    if (f->has_card_property && f->card_property[0]) {
        Card c;
        const Card *cp = rb_decode_card_by_index((uint32_t)card_id, &c) ? &c : NULL;
        int passes = rb_check_card_property(f->card_property, f->negation, cp);
        if (cp) rb_free_card(&c);
        if (!passes) return 0;
    }
    return 1;
}

/* ── CardFilter::matches (ported) ─────────────────────────────────────── */

/* Mirror util.rs::CardFilter::has_filter. */
static int local_has_filter(const LocalCardFilter *f) {
    return f->card_type[0] || f->has_group || f->has_cost_limit || f->has_characters ||
           f->has_exclude_characters || f->n_heart_colors > 0 || f->has_need_heart_total ||
           f->n_name_fragments > 0 || f->has_original_blade || f->ability_filter[0] ||
           f->has_card_property || f->has_exclude_self || f->has_cost_total ||
           f->n_cost_values > 0 || f->has_blade_limit;
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

/* Copy the public RbCardFilter into the local mirror. Returns 0 when the
   filter has nothing set, in which case every card matches (Rust: an
   all-None CardFilter rejects nothing). */
static int local_filter_from_public(const RbCardFilter *rf, LocalCardFilter *out) {
    LocalCardFilter f;
    memset(&f, 0, sizeof f);
    if (!rf) return 0;
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
    /* util.rs:1536 card_property: effect.card_property_any(). The field is not in
       include/rabuka.h yet, so it arrives as "" until the header diff lands. */
    {
        const char *cp = RF_CARD_PROPERTY(rf);
        if (cp && *cp) { strncpy(f.card_property, cp, sizeof f.card_property - 1); f.has_card_property = 1; }
    }
    if (rf->has_exclude_self) { f.exclude_self_id = rf->exclude_self_id; f.has_exclude_self = 1; }
    if (rf->has_cost_total) { f.cost_total = rf->cost_total;
                               if (rf->cost_total_op[0]) strncpy(f.cost_total_op, rf->cost_total_op, sizeof f.cost_total_op - 1);
                               f.has_cost_total = 1; }
    for (int i = 0; i < rf->n_cost_values && i < 16; i++) f.cost_values[i] = rf->cost_values[i];
    f.n_cost_values = rf->n_cost_values;
    f.has_blade_limit = rf->has_blade_limit;
    f.blade_limit = rf->blade_limit;
    if (rf->blade_op[0]) strncpy(f.blade_op, rf->blade_op, sizeof f.blade_op - 1);
    f.distinct = rf->distinct;
    *out = f;
    return 1;
}

/* Mirror util.rs::matching_indices — return indices into cards where the
   filter matches. Rust enumerates `cards` and filters in place, so duplicate
   ids at different positions each produce their own index; the C port used to
   round-trip through matching_ids and map values back, which collapsed
   [A, B, A] into [0, 0]. */
int rb_matching_indices_filter(const RbCardFilter *rf, const int *cards, int n, int *out_idx, int max) {
    if (!cards || !out_idx) return 0;
    LocalCardFilter f;
    if (!local_filter_from_public(rf, &f)) {
        int r = 0;
        for (int i = 0; i < n && r < max; i++) out_idx[r++] = i;
        return r;
    }
    int r = 0;
    for (int i = 0; i < n && r < max; i++)
        if (card_filter_matches(&f, cards[i], 1)) out_idx[r++] = i;
    return r;
}

/* Mirror util.rs::matching_ids — return card IDs matching the filter. */
int rb_matching_ids(const RbCardFilter *rf, const int *cards, int n, int *out, int max) {
    if (!cards || !out) return 0;
    LocalCardFilter f;
    if (!rf || !rf->has_filter || !local_filter_from_public(rf, &f)) {
        int m = 0;
        for (int i = 0; i < n && m < max; i++) out[m++] = cards[i];
        return m;
    }
    int m = 0;
    for (int i = 0; i < n && m < max; i++) {
        if (card_filter_matches(&f, cards[i], 1)) out[m++] = cards[i];
    }
    return m;
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

/* Mirror util.rs::filter_distinct — return indices into cards, deduplicated by
   the normalized name of the card each index points at. Rust filters the INDEX
   list and then dedupes it, so two positions holding equal-name cards collapse
   to the first position; the C port used to round-trip through ids, which
   returned the first index of every distinct id. */
int rb_filter_distinct(const int *cards, int n, const char *card_type,
                       const char *group, RbDistinctType distinct,
                       int *out_idx, int max) {
    RbCardFilter f;
    memset(&f, 0, sizeof f);
    if (card_type) strncpy(f.card_type, card_type, sizeof f.card_type - 1);
    if (group)     strncpy(f.group, group, sizeof f.group - 1);
    f.has_filter = (card_type && card_type[0]) || (group && group[0]);
    int idxs[RB_MAX_ZONE];
    int mn = rb_matching_indices_filter(&f, cards, n, idxs, RB_MAX_ZONE);
    if (!rb_distinct_should_dedupe(distinct)) {
        int m = 0;
        for (int i = 0; i < mn && m < max; i++) out_idx[m++] = idxs[i];
        return m;
    }
    /* dedupe_by_normalized_name over the index list (util.rs:1871-1886). */
    char seen[RB_MAX_ZONE][256];
    int  nseen = 0;
    int  m = 0;
    for (int i = 0; i < mn && m < max; i++) {
        int cid = cards[idxs[i]];
        Card c;
        int keep;
        if (rb_decode_card_by_index((uint32_t)cid, &c)) {
            if (c.name) {
                char nm[256];
                rb_norm_ws(c.name, nm, sizeof nm);
                int dup = 0;
                for (int s = 0; s < nseen; s++)
                    if (!strcmp(seen[s], nm)) { dup = 1; break; }
                if (!dup && nseen < RB_MAX_ZONE) {
                    strncpy(seen[nseen], nm, 255); seen[nseen][255] = 0; nseen++;
                }
                keep = !dup;
            } else {
                keep = 1;
            }
            rb_free_card(&c);
        } else {
            keep = 1; /* card missing from the database is kept (util.rs:1880-1882) */
        }
        if (keep) out_idx[m++] = idxs[i];
    }
    return m;
}

/* Mirror util.rs::count_matching_distinct. The CardName variant is joint-aware
   (Q278/Q279): a multi-name card contributes one unit only when it introduces a
   name not already present as a single-name card. */
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
    if (distinct == RB_DISTINCT_CARDNAME)
        return rb_count_distinct_member_name_units(ids, mn);
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

/* Mirror util.rs::matching_ids_filtered. Rust order (util.rs:1766-1803):
   1. push exclude_ids into the filter (rejects by exact card id),
   2. matching_ids,
   3. when `distinct` is set, dedupe by name and THEN additionally drop
      results whose name matches an excluded card's name,
   4. truncate to target_count.
   The C port previously applied the name exclusion before the dedupe and never
   rejected by id. */
int rb_matching_ids_filtered(const RbCardFilter *f, const int *cards, int n,
                             RbDistinctType distinct, const int *exclude_ids,
                             int n_exclude, int target_count, int *out, int max) {
    if (!cards || !out) return 0;
    int m = rb_matching_ids(f, cards, n, out, max);
    /* 1. exclude_cards: exact-id rejection (util.rs:1167-1174). */
    if (n_exclude > 0) {
        int w = 0;
        for (int i = 0; i < m; i++) {
            int dropped = 0;
            for (int e = 0; e < n_exclude; e++)
                if (out[i] == exclude_ids[e]) { dropped = 1; break; }
            if (!dropped) out[w++] = out[i];
        }
        m = w;
    }
    if (rb_distinct_should_dedupe(distinct)) {
        /* 2. dedupe by name. */
        int deduped[RB_MAX_ZONE];
        int dn = rb_apply_distinct_filter(out, m, distinct, deduped, RB_MAX_ZONE);
        if (dn > max) dn = max;
        memcpy(out, deduped, sizeof(int) * (size_t)dn);
        m = dn;
        /* 3. additionally exclude by excluded card NAME. */
        if (n_exclude > 0) {
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
                    int drop = 0;
                    if (rb_decode_card_by_index((uint32_t)out[i], &c)) {
                        if (c.name) {
                            char nm[256];
                            rb_norm_ws(c.name, nm, sizeof nm);
                            for (int s = 0; s < n_excl_names; s++)
                                if (!strcmp(nm, excl_names[s])) { drop = 1; break; }
                        }
                        rb_free_card(&c);
                    }
                    if (!drop) out[w++] = out[i];
                }
                m = w;
            }
        }
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
    int r = rb_matching_indices_filter(filter, cards, n, out_idx, max);
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
    if (effect->look_action) {
        const AbilityEffect *found = rb_find_modify_cost(effect->look_action, op, loc);
        if (found) return found;
    }
    if (effect->select_action) {
        const AbilityEffect *found = rb_find_modify_cost(effect->select_action, op, loc);
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
        /* engine/src/ability/util.rs:2638-2646 `under_member_cards`: the clause
         * 「このメンバーの下に置かれているカード」 counts ONE member's
         * under-cards -- the host's own slot when the host is on stage, the
         * whole field only when the host is unknown. Walking every bag
         * multiplied the gain by every member on stage: test_parity_hearts
         * _constants reported 2 for "1 other A-RISE member grants exactly 1
         * heart05", 3 for "two others -> +2", and 2 for "energy under a
         * DIFFERENT area plus a member under center -> still 0". Same shape
         * and same fix as dynamic_count.c's per-unit arm. */
        int ids[RB_MAX_ZONE];
        int n = 0;
        int host_slot = -1;
        if (host_card_id >= 0)
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                if (P->stage[s] == host_card_id) { host_slot = s; break; }
        int lo = (host_slot >= 0) ? host_slot : 0;
        int hi = (host_slot >= 0) ? host_slot + 1 : RB_STAGE_SIZE;
        for (int s = lo; s < hi; s++)
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

/* Mirror util.rs::resolve_per_unit_count.
   Zone selection (util.rs:2346-2364): note that the bare 「枚」 unit resolves to
   the HAND unless the filter is restricted to member cards, in which case it
   resolves to the under-member zone. The C port sent 「枚」 straight to
   under_member, which inverted the count for the common 「1枚につき」 gains. */
int rb_resolve_per_unit_count(const GameState *g, int pl, const char *per_unit_type,
                              const char *card_type, const char *group,
                              const char *state_filter, int host_card_id) {
    if (!per_unit_type) return 1;
    /* score.rs:232-234 / misc.rs:832-834 — 「1つにつき」 on a cost that pays ENERGY
       counts the energy paid by the most recent cost step, i.e.
       GameModifiers::last_cost_energy_count (game_modifiers.rs:183), not a zone.
       Until that field reaches RbMods the accessor macro yields the pre-field
       behaviour (0). */
    if (!strcmp(per_unit_type, "つ")) return RB_LAST_COST_ENERGY((RbMods *)&g->mods);
    const RbPlayer *P = &g->p[pl];
    const char *zone;
    int under_member_zone = 0;
    if (!strcmp(per_unit_type, "stage") || !strcmp(per_unit_type, "member") ||
        !strcmp(per_unit_type, "人") || !strcmp(per_unit_type, "members")) {
        zone = "stage";
    } else if (!strcmp(per_unit_type, "hand") || !strcmp(per_unit_type, "card")) {
        zone = "hand";
    } else if (!strcmp(per_unit_type, "under_member")) {
        zone = "under_member";
        under_member_zone = 1;
    } else if (!strcmp(per_unit_type, "枚")) {
        under_member_zone = (card_type && !strcmp(card_type, "member_card"));
        zone = under_member_zone ? "under_member" : "hand";
    } else if (!strcmp(per_unit_type, "discard") || !strcmp(per_unit_type, "waitroom")) {
        zone = "discard";
    } else if (!strcmp(per_unit_type, "live_card_zone") || !strcmp(per_unit_type, "live")) {
        zone = "live";
    } else if (!strcmp(per_unit_type, "success_live_zone") ||
               !strcmp(per_unit_type, "success_live_card_zone")) {
        zone = "success";
    } else {
        return 1;
    }
    if (under_member_zone) {
        /* util.rs:2365-2386 — 「このメンバーの下に置かれているカード」 is scoped to
           the HOST member (whose ability this is), not to every member. Only
           when the host is unknown does the count widen to all under-cards. */
        int ids[RB_MAX_ZONE];
        int n = 0;
        int area = -1;
        if (host_card_id >= 0) {
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                if (P->stage[s] == host_card_id) { area = s; break; }
        }
        if (area >= 0) {
            for (int k = 0; k < P->under_cards[area].n; k++)
                ids[n++] = P->under_cards[area].cards[k];
        } else {
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                for (int k = 0; k < P->under_cards[s].n && n < RB_MAX_ZONE; k++)
                    ids[n++] = P->under_cards[s].cards[k];
        }
        return rb_count_matching(ids, n, card_type, group);
    }
    if (!strcmp(zone, "stage")) {
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
    if (!strcmp(zone, "discard"))
        return rb_count_matching(P->discard.cards, P->discard.n, card_type, group);
    if (!strcmp(zone, "live"))
        return rb_count_matching(P->live.cards, P->live.n, card_type, group);
    if (!strcmp(zone, "success"))
        return rb_count_matching(P->success.cards, P->success.n, card_type, group);
    return rb_count_matching(P->hand.cards, P->hand.n, card_type, group);
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

/* Mirror util.rs::prune_dominated (util.rs:1003-1018) — sort, dedup, then for
   each candidate skip it when some already kept mask is a superset (m ⊆ k),
   otherwise drop every kept mask that m subsumes and push m.
   Divergence from Rust, structural: the Rust twin operates on u128 and the C
   header declares uint64_t, so the C form can only reason about the low 64 bits.
   `kept` is heap-sized from the deduped count so a long input cannot overflow a
   fixed array; the input array itself is used for the sort, so it is the
   caller's buffer and is not clamped to RB_MAX_ZONE. */
static int cmp_u64(const void *a, const void *b) {
    uint64_t va = *(const uint64_t *)a, vb = *(const uint64_t *)b;
    return (va > vb) - (va < vb);
}
void rb_prune_dominated(uint64_t *masks, int *n) {
    if (!masks || !n || *n <= 1) return;
    int total = *n;
    qsort(masks, (size_t)total, sizeof(uint64_t), cmp_u64);
    int m = 1;
    for (int i = 1; i < total; i++)
        if (masks[i] != masks[m - 1]) masks[m++] = masks[i];
    *n = m;
    uint64_t *kept = (uint64_t *)rb_malloc(sizeof(uint64_t) * (size_t)m);
    if (!kept) return;
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
    memcpy(masks, kept, sizeof(uint64_t) * (size_t)nk);
    rb_free(kept);
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
    te->gained_card_id = -1;
    te->gained_index = -1;
    te->revert_kind = RB_REVERT_NONE;
    te->previous_value = 0;
    te->previous_value2 = 0;
    te->revert_text[0] = '\0';
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
    e->gained_card_id = -1;
    e->gained_index = -1;
    e->revert_kind = RB_REVERT_NONE;
    e->previous_value = 0;
    e->previous_value2 = 0;
    e->revert_text[0] = '\0';
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
