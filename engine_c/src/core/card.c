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
static int card_type_of(const unsigned char *r) { return r[18] & 0x03; }

int rb_card_is_live(int card_id) {
    const unsigned char *r = rb_card_record((uint32_t)card_id);
    return r != NULL && card_type_of(r) == 1;
}
int rb_card_is_energy(int card_id) {
    const unsigned char *r = rb_card_record((uint32_t)card_id);
    return r != NULL && card_type_of(r) == 2;
}
int rb_card_is_member(int card_id) {
    const unsigned char *r = rb_card_record((uint32_t)card_id);
    return r != NULL && card_type_of(r) == 0;
}

int rb_decode_card_by_index(uint32_t i, Card *out) {
    const unsigned char *r = rb_card_record(i);
    uint32_t len = rb_card_record_len(i);
    if (!r || !out || len < 25) return 0;
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
    out->type_flags  = r[18];
    out->cost        = r[19];
    out->blade       = r[20];
    out->score       = r[21];
    out->num_base    = r[22];
    out->num_blade   = r[23];
    out->num_need    = r[24];
    /* hearts: (color,count) byte pairs from offset 25 */
    uint32_t pos = 25;
    uint32_t total = (uint32_t)out->num_base + out->num_blade + out->num_need;
    for (uint32_t k = 0; k < total && pos + 1 < len; k++) {
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
void rb_card_normalize_name(const char *src, char *out, size_t out_sz) {
    if (!out || out_sz == 0) return;
    size_t w = 0;
    if (src) {
        for (size_t i = 0; src[i]; ) {
            unsigned char c = (unsigned char)src[i];
            if (c == 0xE3 && (unsigned char)src[i+1] == 0x80 && (unsigned char)src[i+2] == 0x80) { i += 3; continue; }
            if (c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\v' || c == '\f') { i++; continue; }
            if (w + 1 < out_sz) out[w++] = src[i];
            i++;
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

/* ── Ability::has_trigger (card.rs:834): parse triggers text, match kind ── */
int rb_ability_has_trigger(const Ability *a, RbTriggerKind kind) {
    if (!a || !a->triggers) return 0;
    RbTriggerKind out[16];
    int n = rb_parse_triggers(a->triggers, out, 16);
    for (int i = 0; i < n; i++) if (out[i] == kind) return 1;
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
    return NULL;
}
const char *rb_condition_get_position(const Condition *c) {
    const CondValue *v = cond_find(c, "position");
    if (v && v->tag == RB_TAG_STR && v->s) return v->s;
    /* nested PositionInfo struct shape: { position: "..." } */
    if (v && v->tag == RB_TAG_OBJECT && v->cond) {
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
    if (v->tag == RB_TAG_OBJECT || v->tag == RB_TAG_OBJVAR) return 1;
    return 0;
}
const char *rb_effect_position_any(const AbilityEffect *e) {
    const char *p = fx_extra(e, "position");
    if (p) return p;
    /* nested filter struct shape is flattened into extras by the decoder;
       fall back to position_compare/area keys used by older bytecode. */
    p = fx_extra(e, "position_compare");
    if (p) return p;
    return fx_extra(e, "area");
}

/* ── (original CardDatabase-method ports follow) ── */
int rb_card_get_card_id(const char *card_no) {
    if (!card_no) return -1;
    return rb_find_card_by_no(card_no);
}
int rb_card_get_card_names(int card_id, char *out, size_t out_sz) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) { if (out_sz) out[0] = 0; return 0; }
    const char *n = c.name;
    if (!n) { if (out_sz) out[0] = 0; rb_free_card(&c); return 0; }
    strncpy(out, n, out_sz - 1);
    out[out_sz - 1] = 0;
    rb_free_card(&c);
    return 1;
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
        int r = ab.triggers && strstr(ab.triggers, "起動");
        rb_free_ability(&ab);
        if (r) return 1;
    }
    return 0;
}
int rb_card_triggerless_text(int card_id, char *out, size_t out_sz) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) { if (out_sz) out[0] = 0; return 0; }
    const char *t = c.ability ? c.ability->triggerless_text : NULL;
    if (!t) { if (out_sz) out[0] = 0; rb_free_card(&c); return 0; }
    strncpy(out, t, out_sz - 1);
    out[out_sz - 1] = 0;
    rb_free_card(&c);
    return 1;
}
int rb_card_filter_subset(int card_id) { (void)card_id; return 0; }
int rb_card_fires_on_opponent_effects(int card_id) { (void)card_id; return 0; }
int rb_card_energy_cost_total(int card_id) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int t = c.cost;
    rb_free_card(&c);
    return t;
}
int rb_card_has_optional_payment(int card_id) { (void)card_id; return 0; }
int rb_card_effective_energy_cost_total(int card_id, int groups_on_stage) {
    int base = rb_card_energy_cost_total(card_id);
    (void)groups_on_stage;
    return base;
}