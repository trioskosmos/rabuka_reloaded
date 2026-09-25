/* CardDatabase methods — mirrors engine/src/core/card.rs CardDatabase impl. */
#include "rabuka.h"
#include "gen_data.h"
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

/* ── Record layout (mirrors engine/src/core/card_binary.rs
   decode_card_from_record): u16 card_no/name/series/group/unit/img/product/
   rare/ability @0..17, type_flags@18, cost@19, blade@20, score@21,
   num_base@22, num_blade@23, num_need@24, hearts @25 as (color,count)
   pairs, optional 2-byte special heart if bit2 of flags. ── */
static uint16_t le16p(const unsigned char *p) {
    return (uint16_t)(p[0] | ((uint16_t)p[1] << 8));
}
static int card_type_of(const unsigned char *r) {
    int type = r[18] & 0x03;
    return type == 3 ? 0 : type;
}

int rb_card_type_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "member_card")) return 0;
    if (!strcmp(s, "live_card")) return 1;
    if (!strcmp(s, "energy_card")) return 2;
    return -1;
}

const char *rb_card_type_str(int t) {
    switch (t) {
        case 0: return "member_card";
        case 1: return "live_card";
        case 2: return "energy_card";
        default: return "";
    }
}

int rb_card_is_live(int card_id) {
    const unsigned char *r = rb_card_record((uint32_t)card_id);
    return r != NULL && rb_card_record_len((uint32_t)card_id) >= 25 && card_type_of(r) == 1;
}
int rb_card_is_energy(int card_id) {
    const unsigned char *r = rb_card_record((uint32_t)card_id);
    return r != NULL && rb_card_record_len((uint32_t)card_id) >= 25 && card_type_of(r) == 2;
}
int rb_card_is_member(int card_id) {
    const unsigned char *r = rb_card_record((uint32_t)card_id);
    return r != NULL && rb_card_record_len((uint32_t)card_id) >= 25 && card_type_of(r) == 0;
}

int rb_decode_card_by_index(uint32_t i, Card *out) {
    const unsigned char *r = rb_card_record(i);
    uint32_t len = rb_card_record_len(i);
    if (!r || !out || len < 25) return 0;
    uint32_t total = (uint32_t)r[22] + r[23] + r[24];
    uint32_t required = 25 + total * 2 + ((r[18] & 4) ? 2 : 0);
    if (total > RB_MAX_HEARTS || len < required) return 0;
    memset(out, 0, sizeof(*out));
    out->card_no_idx = le16p(r + 0);
    out->name_idx    = le16p(r + 2);
    out->series_idx  = le16p(r + 4);
    out->group_idx   = le16p(r + 6);
    out->unit_idx    = le16p(r + 8);
    out->img_idx     = le16p(r + 10);
    out->product_idx = le16p(r + 12);
    out->rare_idx    = le16p(r + 14);
    out->ability_idx = le16p(r + 16);
    out->type_flags  = (uint8_t)((r[18] & ~3u) | (unsigned)card_type_of(r));
    out->cost        = (r[18] & 8) ? r[19] : 0;
    out->blade       = r[20];
    out->score       = (r[18] & 16) ? r[21] : 0;
    out->num_base    = r[22];
    out->num_blade   = r[23];
    out->num_need    = r[24];
    /* hearts: (color,count) byte pairs from offset 25 */
    uint32_t pos = 25;
    for (uint32_t k = 0; k < total; k++) {
        if (out->n_hearts < RB_MAX_HEARTS) {
            out->heart_color[out->n_hearts] = r[pos];
            out->heart_count[out->n_hearts] = r[pos + 1];
            out->n_hearts++;
        }
        pos += 2;
    }
    out->has_special = (r[18] >> 2) & 0x01;
    if (out->has_special && pos + 1 < len) {
        out->special_color = r[pos];
        out->special_count = r[pos + 1];
        out->has_special = out->special_count != 0;
    }
    const char *nm = rb_card_string(out->name_idx);
    out->name = rb_strdup2(nm ? nm : "");
    out->ability = NULL; /* decoded lazily via rb_decode_card_ability */
    return 1;
}

void rb_free_card(Card *c) {
    if (!c) return;
    if (c->name) { rb_free(c->name); c->name = NULL; }
    if (c->ability) { rb_free_ability(c->ability); rb_free(c->ability); c->ability = NULL; }
}

/* ── Ability slices (mirrors card_loader.rs::attach_abilities:
   CARD_ABILITY_PAIRS holds (ability-string-idx, ability-idx) pairs;
   a pair belongs to the card whose card_no equals get_string(str_idx). ── */
static int card_no_matches(uint32_t card_idx, uint32_t str_idx) {
    const unsigned char *r = rb_card_record(card_idx);
    if (!r) return 0;
    const char *no = rb_card_string(le16p(r + 0));
    const char *s = get_string(str_idx);
    return no && s && !strcmp(no, s);
}
int rb_card_num_abilities(uint32_t card_idx) {
    int n = 0;
    for (uint32_t k = 0; k + 1 < 2u * (uint32_t)RBKA_NUM_CARD_ABILITY_PAIRS; k += 2)
        if (card_no_matches(card_idx, RBKA_CARD_ABILITY_PAIRS[k])) n++;
    return n;
}
int rb_card_get_ability_idx(uint32_t card_idx, int n, uint32_t *out_ability_idx) {
    int seen = 0;
    for (uint32_t k = 0; k + 1 < 2u * (uint32_t)RBKA_NUM_CARD_ABILITY_PAIRS; k += 2) {
        if (!card_no_matches(card_idx, RBKA_CARD_ABILITY_PAIRS[k])) continue;
        if (seen == n) {
            if (out_ability_idx) *out_ability_idx = RBKA_CARD_ABILITY_PAIRS[k + 1];
            return 1;
        }
        seen++;
    }
    return 0;
}
int rb_decode_card_ability(uint32_t card_idx, int n, Ability *out) {
    uint32_t aidx = 0;
    if (!rb_card_get_ability_idx(card_idx, n, &aidx)) return 0;
    return rb_decode_ability(aidx, out);
}

/* ── CardDatabase::normalize_name (card.rs:675): strip all whitespace.
   Covers ASCII whitespace plus U+3000 (ideographic space, E3 80 80). ── */
static size_t card_utf8_char(const char *s, uint32_t *cp) {
    const unsigned char *p = (const unsigned char *)s;
    size_t n = 1;
    uint32_t v = p[0];
    if (p[0] >= 0xC2 && p[0] <= 0xDF) { n = 2; v &= 0x1F; }
    else if (p[0] >= 0xE0 && p[0] <= 0xEF) { n = 3; v &= 0x0F; }
    else if (p[0] >= 0xF0 && p[0] <= 0xF4) { n = 4; v &= 0x07; }
    for (size_t i = 1; i < n; i++) {
        if ((p[i] & 0xC0) != 0x80) { *cp = p[0]; return 1; }
        v = (v << 6) | (p[i] & 0x3F);
    }
    *cp = v;
    return n;
}

static int card_whitespace(uint32_t cp) {
    return (cp >= 0x09 && cp <= 0x0D) || cp == 0x20 || cp == 0x85
        || cp == 0xA0 || cp == 0x1680 || (cp >= 0x2000 && cp <= 0x200A)
        || cp == 0x2028 || cp == 0x2029 || cp == 0x202F || cp == 0x205F
        || cp == 0x3000;
}

static uint32_t card_no_codepoint(uint32_t cp) {
    if (cp >= 'a' && cp <= 'z') return cp - 'a' + 'A';
    if (cp >= 0xFF41 && cp <= 0xFF5A) return cp - 0xFF41 + 'A';
    switch (cp) {
        case 0xFF0B: return '+';
        case 0xFF01: return '!';
        case 0xFF0D: return '-';
        case 0xFF0A: return '*';
        case 0xFF03: return '#';
        default: return cp;
    }
}

void rb_card_normalize_no(const char *src, char *out, size_t out_sz) {
    if (!out || !out_sz) return;
    size_t w = 0;
    if (src) {
        while (*src) {
            uint32_t cp;
            size_t n = card_utf8_char(src, &cp);
            uint32_t normalized = card_no_codepoint(cp);
            size_t written = normalized == cp ? n : 1;
            if (written >= out_sz - w) break;
            if (normalized == cp) memmove(out + w, src, n);
            else out[w] = (char)normalized;
            w += written;
            src += n;
        }
    }
    out[w] = 0;
}

int rb_card_equivalent_rarity(const char *rarity, char *out, size_t out_sz) {
    const char *canonical = NULL;
    if (rarity) {
        if (!strcmp(rarity, "P+") || !strcmp(rarity, "P2") || !strcmp(rarity, "P＋")) canonical = "P＋";
        else if (!strcmp(rarity, "R+") || !strcmp(rarity, "R2") || !strcmp(rarity, "R＋")) canonical = "R＋";
        else if (!strcmp(rarity, "L+") || !strcmp(rarity, "L2") || !strcmp(rarity, "L＋")) canonical = "L＋";
        else if (!strcmp(rarity, "N+") || !strcmp(rarity, "N2") || !strcmp(rarity, "N＋")) canonical = "N＋";
        else if (!strcmp(rarity, "PR+") || !strcmp(rarity, "PR＋")) canonical = "PR＋";
    }
    if (!canonical) return 0;
    size_t len = strlen(canonical);
    if (!out || len >= out_sz) return 0;
    memcpy(out, canonical, len + 1);
    return 1;
}

void rb_map_series_to_group(const char *series, char *out, size_t out_sz) {
    if (!out || !out_sz) return;
    const char *group = "";
    if (series) {
        if (!strcmp(series, "ラブライブ！")) group = "μ's";
        else if (!strcmp(series, "ラブライブ！サンシャイン!!")) group = "Aqours";
        else if (!strcmp(series, "ラブライブ！虹ヶ咲学園スクールアイドル同好会")) group = "虹ヶ咲";
        else if (!strcmp(series, "ラブライブ！スーパースター!!")) group = "Liella!";
        else if (!strcmp(series, "蓮ノ空女学院スクールアイドルクラブ")
              || !strcmp(series, "ラブライブ！蓮ノ空女学院スクールアイドルクラブ")) group = "蓮ノ空";
    }
    size_t len = strlen(group);
    if (len >= out_sz) { out[0] = 0; return; }
    memcpy(out, group, len + 1);
}

void rb_card_normalize_name(const char *src, char *out, size_t out_sz) {
    if (!out || out_sz == 0) return;
    size_t w = 0;
    if (src) {
        while (*src) {
            uint32_t cp;
            size_t n = card_utf8_char(src, &cp);
            if (!card_whitespace(cp)) {
                if (n >= out_sz - w) break;
                memmove(out + w, src, n);
                w += n;
            }
            src += n;
        }
    }
    out[w] = 0;
}

/* ── parse_operator (card.rs:2694): >=0 / <=1 / >2 / <3 / =|==4, else -1 ── */
int rb_parse_operator(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, ">=")) return 0;
    if (!strcmp(s, "<=")) return 1;
    if (!strcmp(s, ">")) return 2;
    if (!strcmp(s, "<")) return 3;
    if (!strcmp(s, "=") || !strcmp(s, "==")) return 4;
    return -1;
}

int rb_parse_operation(const char *s) {
    static const char *const names[] = {
        "add", "decrease", "increase", "remove", "set", "subtract", "set_from_reference"
    };
    if (!s) return -1;
    for (size_t i = 0; i < sizeof(names) / sizeof(names[0]); i++)
        if (!strcmp(s, names[i])) return (int)i;
    return -1;
}

int rb_distinct_info_is_distinct(const char *s) {
    return s && *s && strcmp(s, "false") != 0;
}

/* ── card.rs enum string tables (card.rs:2544-2933). Int encodings follow the
   Rust discriminant order documented in rabuka.h (Active=0/Wait=1; Self=0/
   Opponent=1; MemberCard=0/LiveCard=1/EnergyCard=2; Stage=0/Hand=1/Deck=2/
   DeckTop=3/Discard=4/EnergyZone=5/LiveCardZone=6/SuccessLiveZone=7/
   UnderMember=8/RevealedCards=9). ── */

const char *rb_card_state_str(int s) {
    switch (s) {
        case 0: return "active";
        case 1: return "wait";
        default: return "";
    }
}

int rb_card_state_from_str(const char *s) {
    if (s && !strcmp(s, "active")) return 0;
    return 1;
}

const char *rb_comparison_target_str(int s) {
    switch (s) {
        case 0: return "self";
        case 1: return "opponent";
        default: return "";
    }
}

int rb_comparison_target_from_str(const char *s) {
    if (s && !strcmp(s, "opponent")) return 1;
    return 0;
}

const char *rb_card_property_str(int s) {
    switch (s) {
        case 0: return "has_blade_heart";
        case 1: return "has_score_icon";
        case 2: return "has_all_blade";
        default: return "";
    }
}

int rb_card_property_from_str(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "has_score_icon")) return 1;
    if (!strcmp(s, "has_all_blade")) return 2;
    return 0;
}

const char *rb_placement_order_str(int s) {
    return s == 0 ? "any_order" : "";
}

const char *rb_distinct_type_str(int s) {
    switch (s) {
        case 0: return "card_name";
        case 1: return "true";
        case 2: return "distinct";
        default: return "";
    }
}

const char *rb_comparison_type_str(int s) {
    switch (s) {
        case 0: return "score";
        case 1: return "cost";
        case 2: return "count";
        case 3: return "equality";
        case 4: return "energy_relative";
        default: return "";
    }
}

int rb_comparison_type_from_str(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "cost")) return 1;
    if (!strcmp(s, "count")) return 2;
    if (!strcmp(s, "equality")) return 3;
    if (!strcmp(s, "energy_relative")) return 4;
    return 0;
}

const char *rb_ability_filter_str(int s) {
    switch (s) {
        case 0: return "no_ability";
        case 1: return "has_ability";
        case 2: return "has_ability_type";
        case 3: return "no_ability_type";
        default: return "";
    }
}

int rb_ability_filter_from_str(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "has_ability")) return 1;
    if (!strcmp(s, "has_ability_type")) return 2;
    if (!strcmp(s, "no_ability_type")) return 3;
    return 0;
}

const char *rb_condition_target_str(int s) {
    switch (s) {
        case 0: return "self";
        case 1: return "opponent";
        case 2: return "both";
        case 3: return "either";
        default: return "";
    }
}

const char *rb_condition_card_type_str(int s) {
    switch (s) {
        case 0: return "member_card";
        case 1: return "live_card";
        case 2: return "energy_card";
        default: return "";
    }
}

int rb_condition_card_type_from_str(const char *s) {
    if (s && !strcmp(s, "live_card")) return 1;
    if (s && !strcmp(s, "energy_card")) return 2;
    return 0;
}

const char *rb_location_str(int s) {
    switch (s) {
        case 0: return "stage";
        case 1: return "hand";
        case 2: return "deck";
        case 3: return "deck_top";
        case 4: return "discard";
        case 5: return "energy_zone";
        case 6: return "live_card_zone";
        case 7: return "success_live_card_zone";
        case 8: return "under_member";
        case 9: return "revealed_cards";
        default: return "";
    }
}

/* ── Card::short_label — normalized card name as the short display label. ── */
const char *rb_card_short_label(int card_id) {
    static char label[64];
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return "";
    const char *name = rb_card_string(c.name_idx);
    rb_card_normalize_name(name ? name : "", label, sizeof(label));
    rb_free_card(&c);
    return label;
}

/* ── Ability::has_trigger (card.rs:834): parse triggers text, match kind ── */
int rb_ability_has_trigger(const Ability *a, RbTriggerKind kind) {
    if (!a || !a->triggers) return 0;
    const char *p = a->triggers;
    while (*p) {
        const char *start = p;
        const char *end = p;
        int leading = 1;
        while (*p && *p != ',') {
            uint32_t cp;
            size_t n = card_utf8_char(p, &cp);
            if (!card_whitespace(cp)) {
                if (leading) start = p;
                leading = 0;
                end = p + n;
            }
            p += n;
        }
        if (!leading && (size_t)(end - start) < 64) {
            char token[64];
            size_t len = (size_t)(end - start);
            memcpy(token, start, len);
            token[len] = 0;
            RbTriggerKind parsed;
            if (rb_parse_triggers(token, &parsed, 1) == 1 && parsed == kind) return 1;
        }
        if (*p) p++;
    }
    return 0;
}

/* ── util.rs::has_cannot_baton_touch_protection: any effect of the existing
   card's abilities with restriction_type == cannot_baton_touch, unless the
   incoming card matches exclude_group_names. Recurses into sub-effects
   (same walk as engine.c effect_has_restriction). ── */
static const char *fx_extra(const AbilityEffect *e, const char *key) {
    if (!e || !key) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}
static int fx_blocks_baton(const AbilityEffect *e, int incoming) {
    if (!e) return 0;
    const char *rt = fx_extra(e, "restriction_type");
    if (rt && !strcmp(rt, "cannot_baton_touch")) {
        const char *ex = fx_extra(e, "exclude_group_names");
        if (ex && *ex && rb_card_matches_group_str((uint32_t)incoming, ex)) return 0;
        return 1;
    }
    for (int i = 0; i < e->n_child; i++)
        if (fx_blocks_baton(e->child[i], incoming)) return 1;
    if (fx_blocks_baton(e->primary_effect, incoming)) return 1;
    if (fx_blocks_baton(e->alternative_effect, incoming)) return 1;
    if (fx_blocks_baton(e->look_action, incoming)) return 1;
    if (fx_blocks_baton(e->select_action, incoming)) return 1;
    if (fx_blocks_baton(e->followup_action, incoming)) return 1;
    if (fx_blocks_baton(e->optional_action, incoming)) return 1;
    if (fx_blocks_baton(e->conditional_action, incoming)) return 1;
    return 0;
}
int rb_has_cannot_baton_touch_protection(int incoming_card_id, int existing_card_id) {
    int n = rb_card_num_abilities((uint32_t)existing_card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)existing_card_id, i, &ab)) continue;
        int hit = fx_blocks_baton(ab.effect, incoming_card_id)
               || fx_blocks_baton(ab.cost, incoming_card_id);
        rb_free_ability(&ab);
        if (hit) return 1;
    }
    return 0;
}

/* ── Condition/AbilityEffect field getters (card.rs get_cache/get_group_names/
   get_position/get_distinct, position_any). C conditions are generic key/value
   nodes; each getter scans for its key. ── */
static const CondValue *cond_find(const Condition *c, const char *key) {
    if (!c || !key) return NULL;
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, key)) return &c->fields[i].v;
    return NULL;
}
int rb_condition_get_cache(const Condition *c, int *out) {
    const CondValue *v = cond_find(c, "cache");
    if (!v) return 0;
    int b = 0;
    if (v->tag == RB_TAG_TRUE) b = 1;
    else if (v->tag == RB_TAG_FALSE) b = 0;
    else if (v->tag == RB_TAG_I64) b = (v->i != 0);
    else return 0;
    if (out) *out = b;
    return 1;
}
const char *rb_condition_get_group_names(const Condition *c) {
    const CondValue *v = cond_find(c, "group_names");
    if (v && v->tag == RB_TAG_STR && v->s) return v->s;
    if (v && v->tag == RB_TAG_ARRAY && v->arr && v->arr_n
        && v->arr[0].tag == RB_TAG_STR) return v->arr[0].s;
    return NULL;
}
const char *rb_condition_get_position(const Condition *c) {
    const CondValue *v = cond_find(c, "position");
    if (v && v->tag == RB_TAG_STR && v->s) return v->s;
    /* nested PositionInfo struct shape: { position: "..." } */
    if (v && (v->tag == RB_TAG_OBJECT || v->tag == RB_TAG_OBJVAR) && v->cond) {
        const CondValue *inner = cond_find(v->cond, "position");
        if (inner && inner->tag == RB_TAG_STR && inner->s) return inner->s;
    }
    return NULL;
}
int rb_condition_get_distinct(const Condition *c) {
    const CondValue *v = cond_find(c, "distinct");
    if (!v) return 0;
    if (v->tag == RB_TAG_TRUE) return 1;
    if (v->tag == RB_TAG_FALSE) return 0;
    if (v->tag == RB_TAG_I64) return (int)v->i;
    if (v->tag == RB_TAG_STR && v->s) return *v->s && strcmp(v->s, "false") != 0;
    if (v->tag == RB_TAG_OBJECT || v->tag == RB_TAG_OBJVAR) return 1;
    return 0;
}
const char *rb_effect_position_any(const AbilityEffect *e) {
    return fx_extra(e, "position");
}

/* ── (original CardDatabase-method ports follow) ── */
static int card_no_equal(const char *stored, const char *query, int prefix) {
    while (*stored && *query) {
        uint32_t a, b;
        size_t an = card_utf8_char(stored, &a);
        size_t bn = card_utf8_char(query, &b);
        if (card_no_codepoint(a) != b) return 0;
        stored += an;
        query += bn;
    }
    return !*query && (prefix || !*stored);
}

static int card_no_lookup(const char *query, int prefix) {
    for (uint32_t i = 0; i < rb_num_cards(); i++) {
        const unsigned char *r = rb_card_record(i);
        if (!r || rb_card_record_len(i) < 25) continue;
        const char *no = rb_card_string(le16p(r));
        if (no && card_no_equal(no, query, prefix)) return (int)i;
    }
    return -1;
}

int rb_card_get_card_id(const char *card_no) {
    if (!card_no) return -1;
    int id = rb_find_card_by_no(card_no);
    if (id >= 0) return id;
    size_t len = strlen(card_no);
    if (len > (SIZE_MAX - 1) / 3) return -1;
    char *normalized = rb_malloc(len + 1);
    if (!normalized) return -1;
    rb_card_normalize_no(card_no, normalized, len + 1);
    id = card_no_lookup(normalized, 0);
    char *dash = strrchr(normalized, '-');
    if (id < 0 && dash) {
        char requested[64];
        size_t rarity_len = strlen(dash + 1);
        if (rarity_len < sizeof(requested)) {
            memcpy(requested, dash + 1, rarity_len + 1);
            char equivalent[32];
            if (rb_card_equivalent_rarity(requested, equivalent, sizeof(equivalent))) {
                *dash = 0;
                size_t base_len = strlen(normalized);
                size_t eq_len = strlen(equivalent);
                if (base_len + eq_len + 2 < len + 1) {
                    normalized[base_len] = '-';
                    memcpy(normalized + base_len + 1, equivalent, eq_len + 1);
                    id = card_no_lookup(normalized, 0);
                }
                *dash = '-';
            }
        }
        if (id < 0) {
            *dash = 0;
            id = card_no_lookup(normalized, 1);
            *dash = '-';
        }
        if (id < 0) {
            *dash = 0;
            id = card_no_lookup(normalized, 0);
            *dash = '-';
        }
    }
    if (id < 0) {
        char *wide = rb_malloc(strlen(normalized) * 3 + 1);
        if (wide) {
            char *w = wide;
            for (const char *p = normalized; *p; p++) {
                if (*p == '+') {
                    memcpy(w, "＋", 3);
                    w += 3;
                } else *w++ = *p;
            }
            *w = 0;
            for (uint32_t i = 0; i < rb_num_cards(); i++) {
                const unsigned char *r = rb_card_record(i);
                if (!r || rb_card_record_len(i) < 25) continue;
                const char *no = rb_card_string(le16p(r));
                if (no && (strstr(no, normalized) || strstr(no, wide))) {
                    id = (int)i;
                    break;
                }
            }
            rb_free(wide);
        }
    }
    rb_free(normalized);
    return id;
}
int rb_card_get_card_names(int card_id, char *out, size_t out_sz) {
    if (!out || !out_sz) return 0;
    out[0] = 0;
    const unsigned char *r = rb_card_record((uint32_t)card_id);
    if (!r || rb_card_record_len((uint32_t)card_id) < 4) return 0;
    const char *name = rb_card_string(le16p(r + 2));
    if (!name) return 0;
    size_t w = 0;
    int count = 1;
    while (*name) {
        uint32_t cp;
        size_t n = card_utf8_char(name, &cp);
        if (cp == '&' || cp == 0xFF06) {
            if (w + 1 >= out_sz) { out[0] = 0; return 0; }
            out[w++] = 0;
            count++;
        } else if (!card_whitespace(cp)) {
            if (n >= out_sz - w) { out[0] = 0; return 0; }
            memcpy(out + w, name, n);
            w += n;
        }
        name += n;
    }
    out[w] = 0;
    return count;
}
int rb_card_get_card(const char *card_no) {
    if (!card_no) return 0;
    return rb_find_card_by_no(card_no) >= 0 ? 1 : 0;
}
int rb_card_has_trigger(int card_id, int kind) {
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int r = rb_ability_has_trigger(&ab, (RbTriggerKind)kind);
        rb_free_ability(&ab);
        if (r) return 1;
    }
    return 0;
}
const char *rb_ability_triggerless_text(const Ability *a) {
    if (!a) return "";
    if (a->triggerless_text) return a->triggerless_text;
    const char *text = a->full_text ? a->full_text : "";
    while (*text) {
        uint32_t cp;
        size_t n = card_utf8_char(text, &cp);
        if (!card_whitespace(cp)) break;
        text += n;
    }
    if (!strncmp(text, "【", strlen("【"))) {
        const char *rest = text + strlen("【");
        const char *end = strstr(rest, "】");
        if (end) return text + (end - rest) + strlen("】");
    }
    return text;
}

int rb_card_triggerless_text(int card_id, char *out, size_t out_sz) {
    if (!out || !out_sz) return 0;
    out[0] = 0;
    Ability ab = {0};
    if (!rb_decode_card_ability((uint32_t)card_id, 0, &ab)) return 0;
    const char *text = rb_ability_triggerless_text(&ab);
    size_t len = strlen(text);
    if (len >= out_sz) { rb_free_ability(&ab); return 0; }
    memcpy(out, text, len + 1);
    rb_free_ability(&ab);
    return 1;
}
const char *rb_effect_target_name(const AbilityEffect *e) {
    return e && e->target ? e->target : "self";
}

const char *rb_effect_source_or(const AbilityEffect *e, const char *default_source) {
    if (e && e->source && *e->source) return e->source;
    if (e) {
        const char *source = fx_extra(e, "source");
        if (source && *source) return source;
    }
    return default_source;
}

int rb_effect_count_or(const AbilityEffect *e, int default_count) {
    return e && e->count >= 0 ? e->count : default_count;
}

static int card_extra_int(const AbilityEffect *e, const char *key, int *out) {
    const char *v = fx_extra(e, key);
    if (!v || !*v) return 0;
    char *end;
    long parsed = strtol(v, &end, 10);
    if (*end || parsed < 0 || parsed > 255) return 0;
    *out = (int)parsed;
    return 1;
}

static void card_filter_set_text(char *dst, size_t dst_sz, const char *src) {
    if (!src || !*src) return;
    size_t len = strlen(src);
    if (len < dst_sz) memcpy(dst, src, len + 1);
}

static void card_filter_list(const char *src, char dst[][64], int max, int *count) {
    *count = 0;
    if (!src || !*src) return;
    while (*src && *count < max) {
        while (*src == ',' || *src == ' ' || *src == '[' || *src == ']' || *src == '"') src++;
        const char *start = src;
        while (*src && *src != ',' && *src != ']' && *src != ' ') src++;
        size_t len = (size_t)(src - start);
        if (len && len < 64) {
            memcpy(dst[*count], start, len);
            dst[*count][len] = 0;
            (*count)++;
        }
    }
}

static void card_filter_cost_values(const char *src, int dst[16], int *count) {
    *count = 0;
    if (!src || !*src) return;
    while (*src && *count < 16) {
        while (*src == ',' || *src == ' ' || *src == '[' || *src == ']' || *src == '"') src++;
        if (!*src) break;
        char *end;
        long value = strtol(src, &end, 10);
        if (end == src || value < 0 || value > 255) break;
        dst[(*count)++] = (int)value;
        src = end;
    }
}

static void card_filter_from_effect(const AbilityEffect *e, RbCardFilter *out) {
    if (!e || !out) return;
    const char *card_type = e->card_type_field[0] ? e->card_type_field : fx_extra(e, "card_type");
    card_filter_set_text(out->card_type, sizeof(out->card_type), card_type);
    const char *group = fx_extra(e, "group_names");
    if (!group) group = fx_extra(e, "group");
    if (group && *group) {
        card_filter_set_text(out->group, sizeof(out->group), group);
        out->has_group = 1;
    }
    if (card_extra_int(e, "cost_limit", &out->cost_limit)) {
        out->has_cost_limit = 1;
        card_filter_set_text(out->cost_op, sizeof(out->cost_op), fx_extra(e, "cost_limit_operator"));
        if (!out->cost_op[0]) card_filter_set_text(out->cost_op, sizeof(out->cost_op), fx_extra(e, "operator"));
    }
    const char *characters = fx_extra(e, "characters");
    if (characters && *characters) {
        card_filter_set_text(out->characters, sizeof(out->characters), characters);
        out->has_characters = 1;
    }
    const char *exclude_characters = fx_extra(e, "exclude_characters");
    if (exclude_characters && *exclude_characters) {
        card_filter_set_text(out->exclude_characters, sizeof(out->exclude_characters), exclude_characters);
        out->has_exclude_characters = 1;
    }
    const char *exclude_self = fx_extra(e, "exclude_self");
    if (exclude_self && (!strcmp(exclude_self, "true") || strtol(exclude_self, NULL, 10) != 0)) {
        out->exclude_self_id = -1;
        out->has_exclude_self = 1;
    }
    const char *ability_filter = fx_extra(e, "ability_filter");
    if (ability_filter && *ability_filter) card_filter_set_text(out->ability_filter, sizeof(out->ability_filter), ability_filter);
    const char *negation = fx_extra(e, "negation");
    out->negation = negation && (!strcmp(negation, "true") || strtol(negation, NULL, 10) != 0);
    const char *heart_gate = fx_extra(e, "filter_targets_by_heart_colors");
    const char *heart_colors = fx_extra(e, "heart_colors");
    if (heart_gate && (!strcmp(heart_gate, "true") || strtol(heart_gate, NULL, 10) != 0) && heart_colors && *heart_colors) {
        const char *cursor = heart_colors;
        while (*cursor && out->n_heart_colors < 8) {
            while (*cursor == ',' || *cursor == ' ' || *cursor == '[' || *cursor == ']' || *cursor == '"') cursor++;
            const char *start = cursor;
            while (*cursor && *cursor != ',' && *cursor != ']' && *cursor != ' ') cursor++;
            size_t len = (size_t)(cursor - start);
            if (len && len < sizeof(out->heart_colors[0])) {
                memcpy(out->heart_colors[out->n_heart_colors], start, len);
                out->heart_colors[out->n_heart_colors][len] = 0;
                out->n_heart_colors++;
            }
        }
    }
    if (card_extra_int(e, "original_blade_limit", &out->original_blade_limit)) {
        out->has_original_blade = 1;
        card_filter_set_text(out->original_blade_op, sizeof(out->original_blade_op), fx_extra(e, "original_blade_operator"));
    } else if (card_extra_int(e, "blade_limit", &out->original_blade_limit)) {
        const char *original = fx_extra(e, "original_value");
        if (original && (!strcmp(original, "true") || strtol(original, NULL, 10) != 0)) {
            out->has_original_blade = 1;
            card_filter_set_text(out->original_blade_op, sizeof(out->original_blade_op), fx_extra(e, "blade_limit_operator"));
        }
    }
    if (card_extra_int(e, "cost_total", &out->cost_total)) {
        out->has_cost_total = 1;
        card_filter_set_text(out->cost_total_op, sizeof(out->cost_total_op), fx_extra(e, "cost_total_operator"));
    }
    const char *cost_values = fx_extra(e, "cost_values");
    if (cost_values) card_filter_cost_values(cost_values, out->cost_values, &out->n_cost_values);
    const char *card_names = fx_extra(e, "card_names");
    if (card_names) card_filter_list(card_names, out->name_fragments, 8, &out->n_name_fragments);
    const char *name_constraint = fx_extra(e, "name_constraint");
    if (name_constraint && *name_constraint && !out->n_name_fragments) {
        card_filter_set_text(out->name_fragments[0], sizeof(out->name_fragments[0]), name_constraint);
        out->n_name_fragments = 1;
    }
    out->distinct = rb_distinct_info_is_distinct(fx_extra(e, "distinct"));
    if (card_extra_int(e, "blade_limit", &out->blade_limit)) {
        out->has_blade_limit = 1;
        card_filter_set_text(out->blade_op, sizeof(out->blade_op), fx_extra(e, "blade_limit_operator"));
    }
    out->has_filter = !!(out->card_type[0] || out->has_group || out->has_cost_limit ||
                         out->has_characters || out->has_exclude_characters || out->has_exclude_self ||
                         out->ability_filter[0] || out->negation || out->n_heart_colors || out->has_original_blade ||
                         out->has_cost_total || out->n_cost_values || out->n_name_fragments ||
                         out->distinct || out->has_blade_limit);
}

int rb_effect_filter_subset(const AbilityEffect *e, RbCardFilter *out) {
    if (!out) return 0;
    memset(out, 0, sizeof(*out));
    if (!e) return 0;
    card_filter_from_effect(e, out);
    return out->has_filter;
}

int rb_condition_filter_subset(const Condition *c, RbCardFilter *out) {
    if (!out) return 0;
    memset(out, 0, sizeof(*out));
    if (!c) return 0;
    const CondValue *card_type = cond_find(c, "card_type");
    const CondValue *group = cond_find(c, "group_names");
    const CondValue *cost_limit = cond_find(c, "cost_limit");
    const CondValue *operator_value = cond_find(c, "cost_limit_operator");
    if (!operator_value) operator_value = cond_find(c, "operator");
    const CondValue *characters = cond_find(c, "characters");
    const CondValue *exclude_characters = cond_find(c, "exclude_characters");
    const CondValue *cost_values = cond_find(c, "cost_values");
    const CondValue *card_names = cond_find(c, "card_names");
    const CondValue *distinct = cond_find(c, "distinct");
    if (card_type && card_type->tag == RB_TAG_STR) card_filter_set_text(out->card_type, sizeof(out->card_type), card_type->s);
    if (group && group->tag == RB_TAG_ARRAY && group->arr_n && group->arr[0].tag == RB_TAG_STR) {
        card_filter_set_text(out->group, sizeof(out->group), group->arr[0].s);
        out->has_group = 1;
    }
    if (cost_limit && cost_limit->tag == RB_TAG_I64 && cost_limit->i >= 0 && cost_limit->i <= 255) {
        out->cost_limit = (int)cost_limit->i;
        out->has_cost_limit = 1;
        if (operator_value && operator_value->tag == RB_TAG_STR) card_filter_set_text(out->cost_op, sizeof(out->cost_op), operator_value->s);
    }
    if (characters && characters->tag == RB_TAG_ARRAY && characters->arr_n) {
        size_t w = 0;
        for (uint32_t i = 0; i < characters->arr_n && w < sizeof(out->characters); i++) {
            if (characters->arr[i].tag != RB_TAG_STR || !characters->arr[i].s) continue;
            size_t len = strlen(characters->arr[i].s);
            if (w && w + 1 < sizeof(out->characters)) out->characters[w++] = ',';
            size_t copy = len;
            if (copy >= sizeof(out->characters) - w) copy = sizeof(out->characters) - w - 1;
            memcpy(out->characters + w, characters->arr[i].s, copy);
            w += copy;
            out->characters[w] = 0;
        }
        out->has_characters = w != 0;
    }
    if (exclude_characters && exclude_characters->tag == RB_TAG_ARRAY && exclude_characters->arr_n) {
        size_t w = 0;
        for (uint32_t i = 0; i < exclude_characters->arr_n && w < sizeof(out->exclude_characters); i++) {
            if (exclude_characters->arr[i].tag != RB_TAG_STR || !exclude_characters->arr[i].s) continue;
            size_t len = strlen(exclude_characters->arr[i].s);
            if (w && w + 1 < sizeof(out->exclude_characters)) out->exclude_characters[w++] = ',';
            size_t copy = len;
            if (copy >= sizeof(out->exclude_characters) - w) copy = sizeof(out->exclude_characters) - w - 1;
            memcpy(out->exclude_characters + w, exclude_characters->arr[i].s, copy);
            w += copy;
            out->exclude_characters[w] = 0;
        }
        out->has_exclude_characters = w != 0;
    }
    if (cost_values && cost_values->tag == RB_TAG_ARRAY) {
        for (uint32_t i = 0; i < cost_values->arr_n && out->n_cost_values < 16; i++)
            if (cost_values->arr[i].tag == RB_TAG_I64 && cost_values->arr[i].i >= 0 && cost_values->arr[i].i <= 255)
                out->cost_values[out->n_cost_values++] = (int)cost_values->arr[i].i;
    }
    if (card_names && card_names->tag == RB_TAG_ARRAY) {
        for (uint32_t i = 0; i < card_names->arr_n && out->n_name_fragments < 8; i++) {
            if (card_names->arr[i].tag != RB_TAG_STR || !card_names->arr[i].s) continue;
            card_filter_set_text(out->name_fragments[out->n_name_fragments], sizeof(out->name_fragments[0]), card_names->arr[i].s);
            out->n_name_fragments++;
        }
    }
    if (distinct) {
        if (distinct->tag == RB_TAG_STR) out->distinct = rb_distinct_info_is_distinct(distinct->s);
        else if (distinct->tag == RB_TAG_TRUE) out->distinct = 1;
        else if (distinct->tag == RB_TAG_I64) out->distinct = distinct->i != 0;
    }
    out->has_filter = !!(out->card_type[0] || out->has_group || out->has_cost_limit || out->has_characters ||
                         out->has_exclude_characters || out->n_cost_values || out->n_name_fragments || out->distinct);
    return out->has_filter;
}

int rb_effect_value_or_count(const AbilityEffect *e, int default_value) {
    if (e) {
        int value;
        if (card_extra_int(e, "value", &value)) return value;
        if (e->count >= 0) return e->count;
    }
    return default_value;
}
int rb_card_fires_on_opponent_effects(int card_id) {
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab = {0};
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int fires = 0;
        for (int e = 0; e < 2 && !fires; e++) {
            const AbilityEffect *fx = e == 0 ? ab.effect : ab.cost;
            if (!fx) continue;
            for (int k = 0; k < fx->n_extra && !fires; k++) {
                if (fx->extra_k[k] && !strcmp(fx->extra_k[k], "parenthetical")
                    && fx->extra_v[k]) {
                    const char *p = fx->extra_v[k];
                    if (strstr(p, "発動する") && strstr(p, "相手")) fires = 1;
                }
            }
        }
        rb_free_ability(&ab);
        if (fires) return 1;
    }
    return 0;
}
static int effect_energy_total(const AbilityEffect *e, int groups_on_stage) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "pay_energy")) {
        int printed = e->count < 0 ? 0 : rb_saturate_u8(e->count);
        const char *energy = fx_extra(e, "energy_count");
        if (!energy || !*energy) energy = fx_extra(e, "energy");
        if (energy && *energy) {
            char *end;
            long value = strtol(energy, &end, 10);
            if (*energy && !*end && value >= 0 && value <= 255) printed = (int)value;
        }
        int reduction = rb_saturate_u8(e->cost_reduction_per_group)
                      * rb_saturate_u8(groups_on_stage);
        return rb_saturate_u8(printed - rb_saturate_u8(reduction));
    }
    int total = 0;
    for (int i = 0; i < e->n_child; i++)
        total = rb_saturate_u8(total + effect_energy_total(e->child[i], groups_on_stage));
    return total;
}
static int effect_optional_payment(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "pay_energy") && e->is_optional) return 1;
    for (int i = 0; i < e->n_child; i++)
        if (effect_optional_payment(e->child[i])) return 1;
    return 0;
}
int rb_card_energy_cost_total(int card_id) {
    Ability ab = {0};
    if (!rb_decode_card_ability((uint32_t)card_id, 0, &ab)) return 0;
    int total = effect_energy_total(ab.cost, 0);
    rb_free_ability(&ab);
    return total;
}
int rb_card_has_optional_payment(int card_id) {
    Ability ab = {0};
    if (!rb_decode_card_ability((uint32_t)card_id, 0, &ab)) return 0;
    int optional = effect_optional_payment(ab.cost);
    rb_free_ability(&ab);
    return optional;
}
int rb_card_effective_energy_cost_total(int card_id, int groups_on_stage) {
    Ability ab = {0};
    if (!rb_decode_card_ability((uint32_t)card_id, 0, &ab)) return 0;
    int total = effect_energy_total(ab.cost, groups_on_stage);
    rb_free_ability(&ab);
    return total;
}