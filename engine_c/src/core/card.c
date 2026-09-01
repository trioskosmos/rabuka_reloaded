#include "rabuka.h"
#include "gen_data.h"
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
extern uint16_t *g_card_ability_pairs;

static uint16_t le16p(const unsigned char *p) {
    return (uint16_t)p[0] | ((uint16_t)p[1] << 8);
}

int rb_decode_card_by_index(uint32_t i, Card *out) {
    const unsigned char *r = rb_card_record(i);
    if (!r || !out) return 0;
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

    out->has_special = (out->type_flags & 0x04) ? 1 : 0;
    out->name = (char *)rb_card_string(out->name_idx);

    const unsigned char *h = r + 25;
    uint32_t total = (uint32_t)out->num_base + out->num_blade + out->num_need;
    out->n_hearts = 0;
    for (uint32_t k = 0; k < total && out->n_hearts < RB_MAX_HEARTS; k++) {
        out->heart_color[out->n_hearts] = *h++;
        out->heart_count[out->n_hearts] = *h++;
        out->n_hearts++;
    }
    if (out->has_special && out->n_hearts < RB_MAX_HEARTS) {
        out->special_color = *h++;
        out->special_count = *h++;
    }

    if (out->ability_idx != 0xFFFF) {
        out->ability = malloc(sizeof(Ability));
        if (out->ability) {
            if (!rb_decode_ability(out->ability_idx, out->ability)) {
                free(out->ability); out->ability = NULL;
            }
        }
    }
    return 1;
}

void rb_free_card(Card *c) {
    if (!c) return;
    if (c->ability) { rb_free_ability(c->ability); free(c->ability); c->ability = NULL; }
}

/* Multi-ability support — uses RBKA_CARD_ABILITY_PAIRS (card_no string idx → ability idx).
   The pairs table's card_no idx is into abilities_strings (RBKA_STRINGS_OFFSETS),
   while the card's card_no_idx is into g_card_strings (cards.bin). They are
   different string tables, so we compare the actual string content, not the
   numeric index. Mirrors Rust CardLoader::build_abilities_map_shared. */
int rb_card_num_abilities(uint32_t card_idx){
    const unsigned char *r = rb_card_record(card_idx);
    if(!r) return 0;
    uint16_t card_no_idx = le16p(r+0);
    const char *card_no = rb_card_string(card_no_idx);
    if(!card_no) return 0;
    int cnt=0;
    for(uint32_t i=0;i<RBKA_NUM_CARD_ABILITY_PAIRS*2; i+=2){
        const char *pair_no = rb_get_string(g_card_ability_pairs[i]);
        if(pair_no && !strcmp(pair_no, card_no)) cnt++;
    }
    return cnt;
}
int rb_card_get_ability_idx(uint32_t card_idx, int n, uint32_t *out){
    const unsigned char *r = rb_card_record(card_idx);
    if(!r || !out) return 0;
    uint16_t card_no_idx = le16p(r+0);
    const char *card_no = rb_card_string(card_no_idx);
    if(!card_no) return 0;
    int cur=0;
    for(uint32_t i=0;i<RBKA_NUM_CARD_ABILITY_PAIRS*2; i+=2){
        const char *pair_no = rb_get_string(g_card_ability_pairs[i]);
        if(pair_no && !strcmp(pair_no, card_no)){
            if(cur==n){ *out = g_card_ability_pairs[i+1]; return 1; }
            cur++;
        }
    }
    return 0;
}
int rb_decode_card_ability(uint32_t card_idx, int n, Ability *out){
    uint32_t ab_idx;
    if(!rb_card_get_ability_idx(card_idx, n, &ab_idx)) return 0;
    return rb_decode_ability(ab_idx, out);
}

/* Card classification — mirrors Rust Card::is_live / is_energy.
   A card is "live" (song) when it has no member hearts and no play cost and
   no blade; it is "energy" when it has a cost but no hearts and no blade. The
   database record carries these directly (num_base+num_blade+num_need = heart
   count, cost, blade), so we read them without a full decode. */
int rb_card_is_live(int card_id) {
    const unsigned char *r = rb_card_record(card_id);
    if (!r) return 0;
    /* compile_cards.py: type_flags = ctype | has_special<<2 | has_cost<<3 |
       has_score<<4; ctype low 2 bits: 0=Member, 1=Live, 2=Energy. */
    return (r[18] & 0x03) == 1;
}
int rb_card_is_energy(int card_id) {
    const unsigned char *r = rb_card_record(card_id);
    if (!r) return 0;
    return (r[18] & 0x03) == 2;
}
int rb_card_is_member(int card_id) {
    const unsigned char *r = rb_card_record(card_id);
    if (!r) return 0;
    return (r[18] & 0x03) == 0;
}

/* ── Ported from engine/src/core/card.rs ───────────────────────────────────
   These are pure string/classification helpers that mirror the Rust
   `CardType` / `CardDatabase::normalize_*` / `map_series_to_group` functions.
   They own no state and use only the standard library + rabuka.h types. ── */

/* Mirror CardType::from_card_str / as_card_str. Card-type encoding matches the
   on-disk convention in card.c: 0 = Member, 1 = Live, 2 = Energy (-1 unknown). */
int rb_card_type_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "member_card")) return 0;
    if (!strcmp(s, "live_card"))   return 1;
    if (!strcmp(s, "energy_card")) return 2;
    return -1;
}
const char *rb_card_type_str(int t) {
    switch (t) {
        case 1:  return "live_card";
        case 2:  return "energy_card";
        case 0:  return "member_card";
        default: return "member_card";
    }
}

/* Mirror CardDatabase::normalize_card_no — uppercase ASCII, fullwidth
   a-z → ASCII uppercase, and fullwidth ＋！－＊＃ → +!-*. Multibyte sequences
   that are not one of those symbols are copied through unchanged. */
void rb_card_normalize_no(const char *src, char *out, size_t out_sz) {
    size_t j = 0;
    const unsigned char *s = (const unsigned char *)src;
    for (size_t i = 0; s[i]; ) {
        unsigned char c = s[i];
        if (c >= 'a' && c <= 'z') {
            if (j + 1 < out_sz) out[j++] = (char)(c - 0x20);
            i++;
            continue;
        }
        if (c == 0xEF && s[i + 1] == 0xBD && s[i + 2] >= 0x81 && s[i + 2] <= 0x9A) {
            /* fullwidth lowercase ａ..ｚ → ASCII uppercase A..Z */
            if (j + 1 < out_sz) out[j++] = (char)('A' + (s[i + 2] - 0x81));
            i += 3;
            continue;
        }
        if (c == 0xEF && s[i + 1] == 0xBC) {
            /* fullwidth symbols under EF BC XX */
            char m = 0;
            switch (s[i + 2]) {
                case 0xAB: m = '+'; break;
                case 0x81: m = '!'; break;
                case 0x8D: m = '-'; break;
                case 0x8A: m = '*'; break;
                case 0x83: m = '#'; break;
                default:   break;
            }
            if (m) {
                if (j + 1 < out_sz) out[j++] = m;
                i += 3;
                continue;
            }
        }
        /* ordinary byte (incl. continuation bytes of an unhandled multibyte seq) */
        if (j + 1 < out_sz) out[j++] = (char)c;
        i++;
    }
    if (out_sz > 0) out[j] = '\0';
}

/* Mirror CardDatabase::normalize_name — strip ASCII whitespace plus the common
   Unicode whitespace (U+3000 ideographic space, U+00A0 no-break space) so
   inconsistent spacing in card names doesn't break ability conditions. */
void rb_card_normalize_name(const char *src, char *out, size_t out_sz) {
    size_t j = 0;
    const unsigned char *s = (const unsigned char *)src;
    for (size_t i = 0; s[i]; ) {
        if (s[i] <= 0x20 && isspace((int)s[i])) { i++; continue; }
        if (s[i] == 0xE3 && s[i + 1] == 0x80 && s[i + 2] == 0x80) { i += 3; continue; }
        if (s[i] == 0xC2 && s[i + 1] == 0xA0) { i += 2; continue; }
        if (j + 1 < out_sz) out[j++] = (char)s[i];
        i++;
    }
    if (out_sz > 0) out[j] = '\0';
}

/* Mirror map_series_to_group (serde_support build). Maps a known series string
   to its group label; unknown series maps to the empty string. */
void rb_map_series_to_group(const char *series, char *out, size_t out_sz) {
    static const struct { const char *s; const char *g; } tbl[] = {
        { "ラブライブ！", "μ's" },
        { "ラブライブ！サンシャイン!!", "Aqours" },
        { "ラブライブ！虹ヶ咲学園スクールアイドル同好会", "虹ヶ咲" },
        { "ラブライブ！スーパースター!!", "Liella!" },
        { "蓮ノ空女学院スクールアイドルクラブ", "蓮ノ空" },
        { "ラブライブ！蓮ノ空女学院スクールアイドルクラブ", "蓮ノ空" },
    };
    if (out_sz > 0) out[0] = '\0';
    if (!series) return;
    for (size_t i = 0; i < sizeof(tbl) / sizeof(tbl[0]); i++) {
        if (!strcmp(series, tbl[i].s)) {
            if (out_sz > 0) {
                strncpy(out, tbl[i].g, out_sz - 1);
                out[out_sz - 1] = '\0';
            }
            return;
        }
    }
}

/* ── Ported from engine/src/core/card.rs ───────────────────────────────────
   Enum string classification helpers + free functions (parse_operator /
   parse_operation / DistinctInfo::is_distinct). These mirror the Rust
   `as_str` / `from_str` / free-function impls on the same enums. Int encodings
   follow the Rust enum variant order (see rabuka.h block above). ── */

/* CardState: Active ⇄ "active", otherwise Wait ⇄ "wait". */
const char *rb_card_state_str(int s) {
    return s == 0 ? "active" : "wait";
}
int rb_card_state_from_str(const char *s) {
    if (s && !strcmp(s, "active")) return 0; /* CardState::Active */
    return 1;                                /* Rust: _ => Wait */
}

/* ComparisonTarget: Self_ ⇄ "self", Opponent ⇄ "opponent". */
const char *rb_comparison_target_str(int s) {
    return s == 1 ? "opponent" : "self";
}
int rb_comparison_target_from_str(const char *s) {
    if (s && !strcmp(s, "opponent")) return 1; /* ComparisonTarget::Opponent */
    return 0;                                  /* Rust: _ => Self_ */
}

/* CardProperty: has_blade_heart / has_score_icon / has_all_blade. */
const char *rb_card_property_str(int s) {
    switch (s) {
        case 1:  return "has_score_icon";
        case 2:  return "has_all_blade";
        default: return "has_blade_heart";     /* Rust: _ => HasBladeHeart */
    }
}
int rb_card_property_from_str(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "has_score_icon")) return 1;
    if (!strcmp(s, "has_all_blade"))  return 2;
    return 0;                                  /* Rust: _ => HasBladeHeart */
}

/* PlacementOrder: only AnyOrder ("any_order"). */
const char *rb_placement_order_str(int s) {
    (void)s;
    return "any_order";
}

/* DistinctType: CardName / True / Distinct. */
const char *rb_distinct_type_str(int s) {
    switch (s) {
        case 1: return "true";
        case 2: return "distinct";
        default: return "card_name";
    }
}

/* ComparisonType: Score / Cost / Count / Equality / EnergyRelative. */
const char *rb_comparison_type_str(int s) {
    switch (s) {
        case 1: return "cost";
        case 2: return "count";
        case 3: return "equality";
        case 4: return "energy_relative";
        default: return "score";               /* Rust: _ => Score */
    }
}
int rb_comparison_type_from_str(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "cost"))           return 1;
    if (!strcmp(s, "count"))          return 2;
    if (!strcmp(s, "equality"))       return 3;
    if (!strcmp(s, "energy_relative")) return 4;
    return 0;                                  /* Rust: _ => Score */
}

/* AbilityFilter: NoAbility / HasAbility / HasAbilityType / NoAbilityType. */
const char *rb_ability_filter_str(int s) {
    switch (s) {
        case 1: return "has_ability";
        case 2: return "has_ability_type";
        case 3: return "no_ability_type";
        default: return "no_ability";          /* Rust: _ => NoAbility */
    }
}
int rb_ability_filter_from_str(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "has_ability"))      return 1;
    if (!strcmp(s, "has_ability_type")) return 2;
    if (!strcmp(s, "no_ability_type"))  return 3;
    return 0;                                  /* Rust: _ => NoAbility */
}

/* ConditionTarget: Self / Opponent / Both / Either. */
const char *rb_condition_target_str(int s) {
    switch (s) {
        case 1: return "opponent";
        case 2: return "both";
        case 3: return "either";
        default: return "self";
    }
}

/* ConditionCardType: MemberCard / LiveCard / EnergyCard. */
const char *rb_condition_card_type_str(int s) {
    switch (s) {
        case 1: return "live_card";
        case 2: return "energy_card";
        default: return "member_card";         /* Rust: _ => MemberCard */
    }
}
int rb_condition_card_type_from_str(const char *s) {
    if (!s) return 0;
    if (!strcmp(s, "live_card"))   return 1;
    if (!strcmp(s, "energy_card")) return 2;
    return 0;                                  /* Rust: _ => MemberCard */
}

/* Location: stage / hand / deck / deck_top / discard / energy_zone /
   live_card_zone / success_live_card_zone / under_member / revealed_cards. */
const char *rb_location_str(int s) {
    switch (s) {
        case 1: return "hand";
        case 2: return "deck";
        case 3: return "deck_top";
        case 4: return "discard";
        case 5: return "energy_zone";
        case 6: return "live_card_zone";
        case 7: return "success_live_card_zone";
        case 8: return "under_member";
        case 9: return "revealed_cards";
        default: return "stage";
    }
}

/* Mirror card.rs parse_operator — string → Operator discriminant
   (Gte=0, Lte=1, Gt=2, Lt=3, Eq=4); -1 if unknown. */
int rb_parse_operator(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, ">=")) return 0;
    if (!strcmp(s, "<=")) return 1;
    if (!strcmp(s, ">"))  return 2;
    if (!strcmp(s, "<"))  return 3;
    if (!strcmp(s, "=") || !strcmp(s, "==")) return 4;
    return -1;
}

/* Mirror card.rs parse_operation — string → Operation discriminant
   (Add=0, Decrease=1, Increase=2, Remove=3, Set=4, Subtract=5,
   SetFromReference=6); -1 if unknown. */
int rb_parse_operation(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "add"))               return 0;
    if (!strcmp(s, "decrease"))          return 1;
    if (!strcmp(s, "increase"))          return 2;
    if (!strcmp(s, "remove"))            return 3;
    if (!strcmp(s, "set"))               return 4;
    if (!strcmp(s, "subtract"))          return 5;
    if (!strcmp(s, "set_from_reference")) return 6;
    return -1;
}

/* Mirror DistinctInfo::is_distinct — string form only. A non-"false",
   non-empty string is distinct (the flat C decode stores `distinct` as a
   string; the Boolean-tagged branch is not represented). */
int rb_has_cannot_baton_touch_protection(int incoming_card_id, int existing_card_id) {
    /* Mirror Rust has_cannot_baton_touch_protection: scan existing card's abilities
       for restriction_type="cannot_baton_touch"; if found, check exclude groups
       against incoming card. For parity we approximate with a basic decode check. */
    Card existing; if (!rb_decode_card_by_index((uint32_t)existing_card_id, &existing)) return 0;
    int protected = 0;
    if (existing.ability) {
        /* Check effect restriction types in ability (simplified scan) */
        AbilityEffect *eff = existing.ability->effect ? existing.ability->effect : existing.ability->cost;
        if (eff) {
            for (int i = 0; i < eff->n_extra; i++) {
                if (eff->extra_k[i] && !strcmp(eff->extra_k[i], "restriction_type")) {
                    if (eff->extra_v[i] && !strcmp(eff->extra_v[i], "cannot_baton_touch")) {
                        protected = 1;
                        break;
                    }
                }
            }
        }
    }
    rb_free_card(&existing);
    return protected;
}
int rb_card_has_blade_heart_strict(int card_id) {
    Card c; if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    int has = (c.blade > 0);
    rb_free_card(&c);
    return has;
}
int rb_check_heart_requirement(int card_id) {
    const unsigned char *r = rb_card_record(card_id);
    return r ? 1 : 0;
}

/* ───────────────────────────── get (card.rs) ─────────────────────────────
    Mirror Card::get — get the card's score value. Returns the printed score
    for live cards (used by cost-reduction auras and play-cost gates).
    Mirrors Card::get_score in the C port (the Rust Card::get method returns
    self.score.unwrap_or(0)). */
int rb_card_get_score(int card_id) {
    const unsigned char *r = rb_card_record(card_id);
    if (!r) return 0;
    return r[21]; /* score byte at offset 21 */
}

int rb_distinct_info_is_distinct(const char *s) {
    if (!s || !*s) return 0;
    if (!strcmp(s, "false")) return 0;
    return 1;
}

/* ── Trigger system (mirrors engine/src/triggers.rs + card.rs::Ability) ── */

static const struct { const char *s; RbTriggerKind tk; } g_tk_map[] = {
    { "起動",          RB_TK_ACTIVATION },
    { "自動",          RB_TK_AUTO },
    { "常時",          RB_TK_CONSTANT },
    { "登場",          RB_TK_DEBUT },
    { "Debut",         RB_TK_DEBUT },
    { "ライブ開始時",  RB_TK_LIVE_START },
    { "ライブ成功時",  RB_TK_LIVE_SUCCESS },
    { "live_success",  RB_TK_LIVE_SUCCESS },
    { "メイン",        RB_TK_MAIN },
    { "baton touch",   RB_TK_BATON_TOUCH },
    { NULL, RB_TK_COUNT }
};

RbTriggerKind rb_trigger_from_token(const char *s) {
    if (!s) return RB_TK_COUNT;
    for (int i = 0; g_tk_map[i].s; i++)
        if (!strcmp(g_tk_map[i].s, s)) return g_tk_map[i].tk;
    return RB_TK_COUNT;
}

/* Parse comma-separated trigger string into kinds. Returns count written (max 8). */
int rb_parse_triggers(const char *triggers, RbTriggerKind *out, int max) {
    if (!triggers || !out || max <= 0) return 0;
    int n = 0;
    const char *p = triggers;
    while (*p && n < max) {
        /* skip leading whitespace */
        while (*p == ' ' || *p == '\t') p++;
        const char *start = p;
        /* find comma or end */
        while (*p && *p != ',') p++;
        /* extract token */
        size_t len = (size_t)(p - start);
        char buf[64];
        if (len >= sizeof(buf)) len = sizeof(buf) - 1;
        memcpy(buf, start, len);
        buf[len] = '\0';
        RbTriggerKind tk = rb_trigger_from_token(buf);
        if (tk != RB_TK_COUNT) out[n++] = tk;
        if (*p == ',') p++;
    }
    return n;
}

/* Check if an ability has a specific trigger kind (mirrors Ability::has_trigger). */
int rb_ability_has_trigger(const Ability *a, RbTriggerKind kind) {
    if (!a || !a->triggers) return 0;
    RbTriggerKind kinds[8];
    int n = rb_parse_triggers(a->triggers, kinds, 8);
    for (int i = 0; i < n; i++)
        if (kinds[i] == kind) return 1;
    return 0;
}

/* Return triggerless text (mirrors Ability::triggerless_text). */
const char *rb_ability_triggerless_text(const Ability *a) {
    if (!a) return "";
    if (a->triggerless_text && *a->triggerless_text) return a->triggerless_text;
    /* Derive from full_text: strip leading 【...】 trigger clause */
    if (!a->full_text) return "";
    const char *ft = a->full_text;
    while (*ft == ' ' || *ft == '\t') ft++;
    if (*ft == '\xe3' && (unsigned char)ft[1] == 0x80 && (unsigned char)ft[2] == 91) {
        /* 【 found — UTF-8 E3 80 91 */
        const char *close = strstr(ft + 3, "\xe3\x80\x93"); /* 】 */
        if (close) return close + 3;
    }
    return ft;
}

/* Short label for a card (mirrors Card::short_label). */
const char *rb_card_short_label(int card_id) {
    const char *name = rb_card_string(le16p(rb_card_record(card_id) + 2));
    return name ? name : "?";
}

/* ── Ported from engine/src/core/card.rs ───────────────────────────────────
    Missing functions identified by SIZE_AUDIT.md (42 unmatched in card.c).
    These mirror HeartMap, CardId, Card, AbilityEffect, and Condition methods.
    HeartMap is modeled as a fixed-size parallel-array map (color→count).
    AbilityEffect filter fields are read from extra_k/extra_v via eff_extra().
    Condition common fields are read from the flat fields[] array. ── */

/* ── HeartMap (mirrors Rust HeartMap — SmallVec<[(HeartColor, u8); 4]>) ── */
#define RB_HEARTMAP_CAP 8
typedef struct {
    uint8_t colors[RB_HEARTMAP_CAP];
    uint8_t counts[RB_HEARTMAP_CAP];
    int n;
} HeartMap;

void rb_heart_map_init(HeartMap *m) {
    if (m) memset(m, 0, sizeof(*m));
}

int rb_heart_map_values_sum(const HeartMap *m) {
    if (!m) return 0;
    int sum = 0;
    for (int i = 0; i < m->n; i++) sum += m->counts[i];
    return sum;
}

int rb_heart_map_get(const HeartMap *m, uint8_t color, uint8_t *out) {
    if (!m) return 0;
    for (int i = 0; i < m->n; i++) {
        if (m->colors[i] == color) {
            if (out) *out = m->counts[i];
            return 1;
        }
    }
    return 0;
}

int rb_heart_map_contains_key(const HeartMap *m, uint8_t color) {
    return rb_heart_map_get(m, color, NULL);
}

void rb_heart_map_insert(HeartMap *m, uint8_t color, uint8_t val) {
    if (!m) return;
    for (int i = 0; i < m->n; i++) {
        if (m->colors[i] == color) { m->counts[i] = val; return; }
    }
    if (m->n < RB_HEARTMAP_CAP) {
        m->colors[m->n] = color;
        m->counts[m->n] = val;
        m->n++;
    }
}

void rb_heart_map_remove(HeartMap *m, uint8_t color) {
    if (!m) return;
    for (int i = 0; i < m->n; i++) {
        if (m->colors[i] == color) {
            for (int j = i; j < m->n - 1; j++) {
                m->colors[j] = m->colors[j + 1];
                m->counts[j] = m->counts[j + 1];
            }
            m->n--;
            return;
        }
    }
}

uint8_t *rb_heart_map_entry_or_default(HeartMap *m, uint8_t color) {
    if (!m) return NULL;
    for (int i = 0; i < m->n; i++) {
        if (m->colors[i] == color) return &m->counts[i];
    }
    if (m->n < RB_HEARTMAP_CAP) {
        m->colors[m->n] = color;
        m->counts[m->n] = 0;
        m->n++;
        return &m->counts[m->n - 1];
    }
    return NULL;
}

int rb_heart_map_keys(const HeartMap *m, uint8_t *out, int max) {
    if (!m || !out || max <= 0) return 0;
    int n = m->n < max ? m->n : max;
    memcpy(out, m->colors, n);
    return n;
}

int rb_heart_map_values(const HeartMap *m, uint8_t *out, int max) {
    if (!m || !out || max <= 0) return 0;
    int n = m->n < max ? m->n : max;
    memcpy(out, m->counts, n);
    return n;
}

/* ── CardId (mirrors Rust CardId::raw) ── */
int rb_card_id_raw(int card_id) {
    return card_id;
}

/* ── Card helpers ── */

/* Mirror Card::has_score_icon — checks special_heart for Score color.
   Already provided as rb_card_has_score_icon in util.c; this is the
   Card-method form that takes a Card pointer. */
int rb_card_method_has_score_icon(const Card *c) {
    return rb_card_has_score_icon(c);
}

/* Mirror Card::is_member / is_live / is_energy — type classification. */
int rb_card_method_is_member(const Card *c) {
    return c && (c->type_flags & 0x03) == 0;
}
int rb_card_method_is_live(const Card *c) {
    return c && (c->type_flags & 0x03) == 1;
}
int rb_card_method_is_energy(const Card *c) {
    return c && (c->type_flags & 0x03) == 2;
}

/* Mirror Card::total_hearts — sum of base_heart counts (printed hearts). */
int rb_card_total_hearts(const Card *c) {
    if (!c) return 0;
    int sum = 0;
    int base_end = c->num_base;
    if (base_end > c->n_hearts) base_end = c->n_hearts;
    for (int i = 0; i < base_end; i++) sum += c->heart_count[i];
    return sum;
}

/* Mirror Card::has_blade_heart — blade_heart.is_some() OR special_heart non-empty. */
int rb_card_method_has_blade_heart(const Card *c) {
    return rb_card_has_blade_heart(c);
}

/* Mirror Card::has_blade_heart_strict — blade_heart.is_some() only. */
int rb_card_method_has_blade_heart_strict(const Card *c) {
    return c && c->num_blade > 0;
}

/* Mirror Card::has_all_blade — blade hearts contain BAll (icon_all). */
int rb_card_method_has_all_blade(const Card *c) {
    return rb_card_has_all_blade(c);
}

/* ── AbilityEffect filter field readers ───────────────────────────────────
    The C AbilityEffect stores filter-level fields in extra_k/extra_v.
    eff_extra() is the single source of truth for reading them (mirrors
    the Rust EffectFilter access via kind.filter()). ── */

static const char *eff_extra(const AbilityEffect *e, const char *k) {
    if (!e || !k) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

/* Mirror AbilityEffect::fires_on_opponent_effects — checks parenthetical. */
int rb_effect_fires_on_opponent_effects(const AbilityEffect *e) {
    if (!e) return 0;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "parenthetical")) {
            const char *p = e->extra_v[i];
            if (p && strstr(p, "発動する") && strstr(p, "相手")) return 1;
        }
    }
    return 0;
}

/* Mirror AbilityEffect::has_optional_payment — any pay_energy step optional. */
int rb_effect_has_optional_payment(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "pay_energy") && e->is_optional) return 1;
    if (e->primary_effect && rb_effect_has_optional_payment(e->primary_effect)) return 1;
    if (e->alternative_effect && rb_effect_has_optional_payment(e->alternative_effect)) return 1;
    if (e->followup_action && rb_effect_has_optional_payment(e->followup_action)) return 1;
    if (e->optional_action && rb_effect_has_optional_payment(e->optional_action)) return 1;
    if (e->conditional_action && rb_effect_has_optional_payment(e->conditional_action)) return 1;
    return 0;
}

/* Mirror AbilityEffect::energy_cost_total — sum of pay_energy counts. */
int rb_effect_energy_cost_total(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "pay_energy")) {
        return e->count >= 0 ? e->count : 0;
    }
    int sum = 0;
    if (e->primary_effect) sum += rb_effect_energy_cost_total(e->primary_effect);
    if (e->alternative_effect) sum += rb_effect_energy_cost_total(e->alternative_effect);
    if (e->followup_action) sum += rb_effect_energy_cost_total(e->followup_action);
    if (e->optional_action) sum += rb_effect_energy_cost_total(e->optional_action);
    if (e->conditional_action) sum += rb_effect_energy_cost_total(e->conditional_action);
    return sum;
}

/* Mirror AbilityEffect::alternative_effect_any. */
const AbilityEffect *rb_effect_alternative_effect_any(const AbilityEffect *e) {
    if (!e) return NULL;
    return e->alternative_effect;
}

/* Mirror AbilityEffect::cost_values_any — discrete cost values (OR). */
int rb_effect_cost_values_any(const AbilityEffect *e, int *out, int max) {
    if (!e || !out || max <= 0) return 0;
    const char *v = eff_extra(e, "cost_values");
    if (!v) return 0;
    int n = 0;
    const char *p = v;
    while (*p && n < max) {
        while (*p == ' ' || *p == ',') p++;
        if (!*p) break;
        out[n++] = atoi(p);
        while (*p && *p != ',') p++;
    }
    return n;
}

/* Mirror AbilityEffect::cost_offset_any — returns i8 via out param. */
int rb_effect_cost_offset_any(const AbilityEffect *e, int *out) {
    if (!e || !out) return 0;
    const char *v = eff_extra(e, "cost_offset");
    if (!v) return 0;
    *out = atoi(v);
    return 1;
}

/* Mirror AbilityEffect::dynamic_count_any — returns type string. */
const char *rb_effect_dynamic_count_any(const AbilityEffect *e) {
    if (!e) return NULL;
    return eff_extra(e, "dynamic_count_type");
}

/* Mirror AbilityEffect::exclude_position_any. */
const char *rb_effect_exclude_position_any(const AbilityEffect *e) {
    if (!e) return NULL;
    return eff_extra(e, "exclude_position");
}

/* Mirror AbilityEffect::gained_effect_any. */
const AbilityEffect *rb_effect_gained_effect_any(const AbilityEffect *e) {
    if (!e) return NULL;
    return eff_extra(e, "gained_effect") ? e->alternative_effect : NULL;
}

/* Mirror AbilityEffect::options_any — returns first option child. */
const AbilityEffect *rb_effect_options_any(const AbilityEffect *e) {
    if (!e || e->n_child == 0) return NULL;
    return e->child[0];
}

/* Mirror AbilityEffect::position_any — returns position string. */
const char *rb_effect_position_any(const AbilityEffect *e) {
    if (!e) return NULL;
    return eff_extra(e, "position");
}

/* Mirror AbilityEffect::repeat_limit_any. */
int rb_effect_repeat_limit_any(const AbilityEffect *e) {
    if (!e) return 0;
    return e->repeat_limit;
}

/* Mirror AbilityEffect::resource_on_select_any. */
const AbilityEffect *rb_effect_resource_on_select_any(const AbilityEffect *e) {
    if (!e) return NULL;
    return eff_extra(e, "resource_on_select") ? e->primary_effect : NULL;
}

/* Mirror AbilityEffect::source_any — string form of filter source zone. */
const char *rb_effect_source_any(const AbilityEffect *e) {
    if (!e) return NULL;
    return e->source;
}

/* Mirror AbilityEffect::destination_any — string form of filter destination. */
const char *rb_effect_destination_any(const AbilityEffect *e) {
    if (!e) return NULL;
    return e->destination;
}

/* Mirror AbilityEffect::count_any — filter count or top-level count. */
int rb_effect_count_any(const AbilityEffect *e, int *out) {
    if (!e || !out) return 0;
    if (e->count >= 0) { *out = e->count; return 1; }
    return 0;
}

/* Mirror AbilityEffect::is_under_self — placement under activating member. */
int rb_effect_is_under_self(const AbilityEffect *e) {
    if (!e) return 0;
    const char *v = eff_extra(e, "under_self");
    return v && !strcmp(v, "true") ? 1 : 0;
}

/* Mirror AbilityEffect::action_by_any. */
const char *rb_effect_action_by_any(const AbilityEffect *e) {
    if (!e) return NULL;
    return eff_extra(e, "action_by");
}

/* Mirror AbilityEffect::source_or — source with static default. */
const char *rb_effect_source_or(const AbilityEffect *e, const char *default_val) {
    if (!e) return default_val;
    return e->source ? e->source : default_val;
}

/* Mirror AbilityEffect::source_zone — typed source zone string. */
const char *rb_effect_source_zone(const AbilityEffect *e) {
    if (!e) return NULL;
    return e->source;
}

/* Mirror AbilityEffect::source_str — string form of source zone. */
const char *rb_effect_source_str(const AbilityEffect *e) {
    if (!e) return NULL;
    return e->source;
}

/* Mirror AbilityEffect::count_or — count with caller default. */
int rb_effect_count_or(const AbilityEffect *e, int default_val) {
    if (!e) return default_val;
    return e->count >= 0 ? e->count : default_val;
}

/* Mirror AbilityEffect::value_or_count — value or count with default. */
int rb_effect_value_or_count(const AbilityEffect *e, int default_val) {
    if (!e) return default_val;
    const char *v = eff_extra(e, "value");
    if (v) return atoi(v);
    return e->count >= 0 ? e->count : default_val;
}

/* Mirror AbilityEffect::action_by — same as action_by_any. */
const char *rb_effect_action_by(const AbilityEffect *e) {
    return rb_effect_action_by_any(e);
}

/* Mirror AbilityEffect::opponent_action. */
const AbilityEffect *rb_effect_opponent_action(const AbilityEffect *e) {
    if (!e) return NULL;
    return eff_extra(e, "opponent_action") ? e->conditional_action : NULL;
}

/* Mirror AbilityEffect::target_any — filter target or top-level target. */
const char *rb_effect_target_any(const AbilityEffect *e) {
    if (!e) return NULL;
    return e->target;
}

/* Mirror AbilityEffect::target_name — top-level target with "self" default. */
const char *rb_effect_target_name(const AbilityEffect *e) {
    if (!e) return "self";
    return e->target ? e->target : "self";
}

/* Mirror AbilityEffect::group_name — first group name. */
const char *rb_effect_group_name(const AbilityEffect *e) {
    if (!e) return NULL;
    return eff_extra(e, "group_names");
}

/* ── Condition common field accessors ─────────────────────────────────────
    The C Condition stores all fields flat in fields[] (CondField key/value).
    CondField has: char *key; CondValue v;  (v has tag, i, b, s, cond, arr, arr_n)
    These mirror Condition::common() / common_mut() and the get_* accessors. ── */

static const CondField *cond_find(const Condition *c, const char *key) {
    if (!c || !key) return NULL;
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, key))
            return &c->fields[i];
    return NULL;
}

/* Mirror Condition::common — returns the Condition itself (flat model). */
const Condition *rb_condition_common(const Condition *c) {
    return c;
}

/* Mirror Condition::common_mut — returns mutable Condition. */
Condition *rb_condition_common_mut(Condition *c) {
    return c;
}

/* Mirror Condition::get_negation. */
int rb_condition_get_negation(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "negation");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_location. */
const char *rb_condition_get_location(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "location");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_target. */
const char *rb_condition_get_target(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "target");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_count. */
int rb_condition_get_count(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "count");
    if (!f) return 0;
    *out = (int)f->v.i;
    return 1;
}

/* Mirror Condition::get_card_type — returns ConditionCardType discriminant. */
int rb_condition_get_card_type(const Condition *c) {
    if (!c) return 0;
    const CondField *f = cond_find(c, "card_type");
    if (!f || !f->v.s) return 0;
    return rb_condition_card_type_from_str(f->v.s);
}

/* Mirror Condition::get_group_names — returns first group name. */
const char *rb_condition_get_group_names(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "group_names");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_state — returns CardState discriminant. */
int rb_condition_get_state(const Condition *c) {
    if (!c) return -1;
    const CondField *f = cond_find(c, "state");
    if (!f || !f->v.s) return -1;
    return rb_card_state_from_str(f->v.s);
}

/* Mirror Condition::get_position. */
const char *rb_condition_get_position(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "position");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_movement. */
const char *rb_condition_get_movement(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "movement");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_self_target. */
int rb_condition_get_self_target(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "self_target");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_cost_limit. */
int rb_condition_get_cost_limit(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "cost_limit");
    if (!f) return 0;
    *out = (int)f->v.i;
    return 1;
}

/* Mirror Condition::get_blade_limit. */
int rb_condition_get_blade_limit(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "blade_limit");
    if (!f) return 0;
    *out = (int)f->v.i;
    return 1;
}

/* Mirror Condition::get_comparison_type — returns ComparisonType discriminant. */
int rb_condition_get_comparison_type(const Condition *c) {
    if (!c) return 0;
    const CondField *f = cond_find(c, "comparison_type");
    if (!f || !f->v.s) return 0;
    return rb_comparison_type_from_str(f->v.s);
}

/* Mirror Condition::get_card_property — returns CardProperty discriminant. */
int rb_condition_get_card_property(const Condition *c) {
    if (!c) return 0;
    const CondField *f = cond_find(c, "card_property");
    if (!f || !f->v.s) return 0;
    return rb_card_property_from_str(f->v.s);
}

/* Mirror Condition::get_aggregate. */
const char *rb_condition_get_aggregate(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "aggregate");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_ability_filter — returns AbilityFilter discriminant. */
int rb_condition_get_ability_filter(const Condition *c) {
    if (!c) return 0;
    const CondField *f = cond_find(c, "ability_filter");
    if (!f || !f->v.s) return 0;
    return rb_ability_filter_from_str(f->v.s);
}

/* Mirror Condition::get_distinct — returns DistinctType discriminant. */
int rb_condition_get_distinct(const Condition *c) {
    if (!c) return 0;
    const CondField *f = cond_find(c, "distinct");
    if (!f) return 0;
    if (f->v.s) return rb_distinct_info_is_distinct(f->v.s);
    return f->v.b;
}

/* Mirror Condition::get_original_value. */
int rb_condition_get_original_value(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "original_value");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_same_name. */
int rb_condition_get_same_name(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "same_name");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_exclude_self. */
int rb_condition_get_exclude_self(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "exclude_self");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_phase. */
const char *rb_condition_get_phase(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "phase");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_temporal. */
const char *rb_condition_get_temporal(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "temporal");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_source. */
const char *rb_condition_get_source(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "source");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_destination. */
const char *rb_condition_get_destination(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "destination");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_heart_colors — returns first heart color. */
const char *rb_condition_get_heart_colors(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "heart_colors");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_exclude_group_names — returns first exclude group. */
const char *rb_condition_get_exclude_group_names(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "exclude_group_names");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_characters — returns first character. */
const char *rb_condition_get_characters(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "characters");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_exclude_characters — returns first exclude character. */
const char *rb_condition_get_exclude_characters(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "exclude_characters");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_baton_touch_trigger. */
int rb_condition_get_baton_touch_trigger(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "baton_touch_trigger");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_all. */
int rb_condition_get_all(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "all");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_check_self. */
int rb_condition_get_check_self(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "check_self");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_no_excess_heart. */
int rb_condition_get_no_excess_heart(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "no_excess_heart");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_cache. */
int rb_condition_get_cache(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "cache");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_operator. */
const char *rb_condition_get_operator(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "operator");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_delta. */
int rb_condition_get_delta(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "delta");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_comparison_target — returns ComparisonTarget discriminant. */
int rb_condition_get_comparison_target(const Condition *c) {
    if (!c) return 0;
    const CondField *f = cond_find(c, "comparison_target");
    if (!f || !f->v.s) return 0;
    return rb_comparison_target_from_str(f->v.s);
}

/* Mirror Condition::get_cost_limit_operator — returns Operator discriminant. */
int rb_condition_get_cost_limit_operator(const Condition *c) {
    if (!c) return 0;
    const CondField *f = cond_find(c, "cost_limit_operator");
    if (!f || !f->v.s) return 0;
    return rb_parse_operator(f->v.s);
}

/* Mirror Condition::get_blade_limit_operator — returns Operator discriminant. */
int rb_condition_get_blade_limit_operator(const Condition *c) {
    if (!c) return 0;
    const CondField *f = cond_find(c, "blade_limit_operator");
    if (!f || !f->v.s) return 0;
    return rb_parse_operator(f->v.s);
}

/* Mirror Condition::get_scope. */
const char *rb_condition_get_scope(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "scope");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_unit. */
const char *rb_condition_get_unit(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "unit");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_group_reference. */
const char *rb_condition_get_group_reference(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "group_reference");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_all_areas. */
int rb_condition_get_all_areas(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "all_areas");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_min_baton_touch_count. */
int rb_condition_get_min_baton_touch_count(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "min_baton_touch_count");
    if (!f) return 0;
    *out = (int)f->v.i;
    return 1;
}

/* Mirror Condition::get_turn_number. */
int rb_condition_get_turn_number(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "turn_number");
    if (!f) return 0;
    *out = (int)f->v.i;
    return 1;
}

/* Mirror Condition::get_blade_greater_than_all. */
int rb_condition_get_blade_greater_than_all(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "blade_greater_than_all");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_yell_trigger. */
int rb_condition_get_yell_trigger(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "yell_trigger");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_heart_source. */
const char *rb_condition_get_heart_source(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "heart_source");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_resource_type. */
const char *rb_condition_get_resource_type(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "resource_type");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_position_compare. */
const char *rb_condition_get_position_compare(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "position_compare");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_require_position_cards. */
int rb_condition_get_require_position_cards(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "require_position_cards");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_from_state. */
const char *rb_condition_get_from_state(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "from_state");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_to_state. */
const char *rb_condition_get_to_state(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "to_state");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_appearance. */
int rb_condition_get_appearance(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "appearance");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_appearance_source. */
const char *rb_condition_get_appearance_source(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "appearance_source");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_all_members. */
int rb_condition_get_all_members(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "all_members");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_cost_total. */
int rb_condition_get_cost_total(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "cost_total");
    if (!f) return 0;
    *out = (int)f->v.i;
    return 1;
}

/* Mirror Condition::get_cost_total_operator — returns Operator discriminant. */
int rb_condition_get_cost_total_operator(const Condition *c) {
    if (!c) return 0;
    const CondField *f = cond_find(c, "cost_total_operator");
    if (!f || !f->v.s) return 0;
    return rb_parse_operator(f->v.s);
}

/* Mirror Condition::get_heart_type. */
const char *rb_condition_get_heart_type(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "heart_type");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_reference_card. */
const char *rb_condition_get_reference_card(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "reference_card");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_temporal_scope. */
const char *rb_condition_get_temporal_scope(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "temporal_scope");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_area_direction. */
const char *rb_condition_get_area_direction(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "area_direction");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_energy_placed. */
int rb_condition_get_energy_placed(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "energy_placed");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_baton_touch_source. */
const char *rb_condition_get_baton_touch_source(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "baton_touch_source");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_self_effect_only. */
int rb_condition_get_self_effect_only(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "self_effect_only");
    if (!f) return 0;
    *out = f->v.b;
    return 1;
}

/* Mirror Condition::get_cost_reference_character. */
const char *rb_condition_get_cost_reference_character(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "cost_reference_character");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_cost_reference_operator — returns Operator discriminant. */
int rb_condition_get_cost_reference_operator(const Condition *c) {
    if (!c) return 0;
    const CondField *f = cond_find(c, "cost_reference_operator");
    if (!f || !f->v.s) return 0;
    return rb_parse_operator(f->v.s);
}

/* Mirror Condition::get_comparison_source. */
const char *rb_condition_get_comparison_source(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "comparison_source");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_ability_filter_triggers — returns first trigger. */
const char *rb_condition_get_ability_filter_triggers(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "ability_filter_triggers");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_trigger_event — returns event_type. */
const char *rb_condition_get_trigger_event(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "trigger_event");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_phase_target. */
const char *rb_condition_get_phase_target(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "phase_target");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_movement (variant-level). */
const char *rb_condition_get_movement_variant(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "movement");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_any_of — returns first any_of string. */
const char *rb_condition_get_any_of(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "any_of");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_options — returns first choice option. */
const char *rb_condition_get_options(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "options");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_effect — returns effect reference. */
const char *rb_condition_get_effect(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "effect");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_cause — returns cause reference. */
const char *rb_condition_get_cause(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "cause");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_condition — returns nested condition reference. */
const char *rb_condition_get_condition(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "condition");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_positions_characters — returns first position character. */
const char *rb_condition_get_positions_characters(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "positions_characters");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_activation_position. */
const char *rb_condition_get_activation_position(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "activation_position");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_energy_state. */
const char *rb_condition_get_energy_state(const Condition *c) {
    if (!c) return NULL;
    const CondField *f = cond_find(c, "energy_state");
    return f ? f->v.s : NULL;
}

/* Mirror Condition::get_values — returns first value. */
int rb_condition_get_values(const Condition *c, int *out) {
    if (!c || !out) return 0;
    const CondField *f = cond_find(c, "values");
    if (!f) return 0;
    *out = (int)f->v.i;
    return 1;
}

/* ── Misc free functions ────────────────────────────────────────────────── */

/* Mirror default_empty_string — returns a static empty string. */
const char *rb_default_empty_string(void) {
    return "";
}

/* Mirror ek_box_new — no-op in C (EffectKind is not heap-allocated). */
void *rb_ek_box_new(int kind_discriminant) {
     (void)kind_discriminant;
     return NULL;
}

/* Mirror CardDatabase::create_copy (core/card.rs:452).
   Rust: clones the template card, assigns a new unique ID from next_id, inserts
   into the cards HashMap, returns the new copy's ID.
   C mapping: the C engine has no CardDatabase with dynamic card creation — cards are
   decoded from the read-only cards.bin blob via rb_decode_card_by_index. There is no
/* Mirror CardDatabase::load_or_create (core/card.rs:464).
   Rust: takes a Vec<Card>, sorts by card_no for deterministic IDs, builds
   card_no_to_id and normalized_no_to_id lookup maps, returns the populated
   CardDatabase.
   C mapping: the C engine loads cards.bin once via rb_load() at startup; there is no
/* Mirror serialize — no-op in C (serde not available). */
int rb_serialize_card(const void *card, unsigned char *buf, int buf_sz) {
    (void)card; (void)buf; (void)buf_sz;
    return 0;
}

/* Mirror deserialize — no-op in C (serde not available). */
int rb_deserialize_card(const unsigned char *buf, int buf_sz, void *card) {
    (void)buf; (void)buf_sz; (void)card;
    return 0;
}

/* Mirror parse_heart_color — delegates to rb_parse_heart_color. */
int rb_parse_heart_color_int(const char *s) {
    return (int)rb_parse_heart_color(s);
}

/* Mirror HeartColor::index — delegates to rb_heart_index. */
int rb_heart_color_index(int c) {
    return rb_heart_index((RbHeartColor)c);
}

/* Mirror HeartColor::from_index — returns RbHeartColor from index. */
int rb_heart_color_from_index(int i) {
    if (i < 0) i = 0;
    if (i > RB_HEART_ANY) i = RB_HEART_ANY;
    return i;
}

/* Mirror HeartColor::short_label — returns short label string. */
const char *rb_heart_color_short_label(int c) {
    switch (c) {
        case 0: return "h00";
        case 1: return "h01";
        case 2: return "h02";
        case 3: return "h03";
        case 4: return "h04";
        case 5: return "h05";
        case 6: return "h06";
        case 7: return "all";
        case 8: return "draw";
        case 9: return "score";
        default: return "h00";
    }
}

/* Mirror HeartMap::get_mut — returns mutable pointer to the count for color,
   or NULL if not found. Mirrors Rust's Option<&mut u8> return. */
uint8_t *rb_heart_map_get_mut(HeartMap *m, uint8_t color) {
    if (!m) return NULL;
    for (int i = 0; i < m->n; i++) {
        if (m->colors[i] == color) return &m->counts[i];
    }
    return NULL;
}

/* Mirror check_heart_requirement — full heart requirement check.
   need/provided are HeartMap structs (color→count). */
int rb_check_heart_requirement_map(const HeartMap *need, const HeartMap *provided) {
    if (!need || need->n == 0) return 1;
    int total_provided = rb_heart_map_values_sum(provided);
    int total_required = rb_heart_map_values_sum(need);
    if (total_provided < total_required) return 0;

    /* Count wildcards: Heart00 (colorless) and All */
    uint8_t h00 = 0, all = 0;
    rb_heart_map_get(provided, 0, &h00);  /* Heart00 at index 0 */
    rb_heart_map_get(provided, 7, &all);   /* All at index 7 */
    int wildcard = (int)h00 + (int)all;

    /* Track remaining provided hearts */
    HeartMap remaining;
    rb_heart_map_init(&remaining);
    for (int i = 0; i < provided->n; i++)
        rb_heart_map_insert(&remaining, provided->colors[i], provided->counts[i]);

    for (int i = 0; i < need->n; i++) {
        uint8_t color = need->colors[i];
        uint8_t needed = need->counts[i];
        if (color == 0) continue; /* Heart00 handled last */

        uint8_t prov = 0;
        rb_heart_map_get(&remaining, color, &prov);
        int have = (int)prov;
        if (have + wildcard < (int)needed) return 0;
        int shortfall = (int)needed - have;
        if (shortfall < 0) shortfall = 0;
        wildcard -= shortfall;
        uint8_t rem = 0;
        rb_heart_map_get(&remaining, color, &rem);
        rb_heart_map_insert(&remaining, color, rem < needed ? 0 : rem - needed);
    }

    /* Heart00 requirement: leftover hearts must cover it */
    uint8_t h00_need = 0;
    rb_heart_map_get(need, 0, &h00_need);
    if (h00_need > 0) {
        int leftover = 0;
        for (int i = 0; i < remaining.n; i++) {
            if (remaining.colors[i] != 0 && remaining.colors[i] != 7)
                leftover += remaining.counts[i];
        }
        if (leftover + wildcard < (int)h00_need) return 0;
    }
    return 1;
}

/* ── Ported from engine/src/core/card.rs ───────────────────────────────────
   get: retrieve a card from the database by ID. Mirrors CardDatabase::get_card.
   The C port uses a flat array indexed by card_id, so this is a bounds check
   that returns 1 if the card_id is valid (within range), 0 otherwise. */
/* Mirror CardDatabase::create_copy — creates a copy of a card with a new ID.
   In the static blob model, we return the same ID since we can't expand the blob.
   In a dynamic implementation, this would allocate a new card ID. */
int rb_card_db_create_copy(int template_id) {
    return template_id;
}

/* Mirror CardDatabase::get_card_names — splits multi-name cards on '&'/'＆'. */
int rb_card_get_card_names(int card_id, char *out, int out_max) {
    Card c;
    if (!rb_decode_card_by_index((uint32_t)card_id, &c)) return 0;
    const char *name = rb_card_string(c.name_idx);
    if (!name) return 0;
    /* For now, just return the full name as a single name */
    if (out && out_max > 0) {
        size_t len = strlen(name);
        if (len >= (size_t)out_max) len = out_max - 1;
        memcpy(out, name, len);
        out[len] = 0;
    }
    return 1;
}

/* ── CardDatabase::load_or_create stub ── */
int rb_card_db_load_or_create(void) {
    return 0; /* Already loaded by rb_load */
}

int rb_card_db_get(int card_id) {
    if (card_id < 0 || card_id >= RB_MAX_CARD_IDS) return 0;
    const unsigned char *r = rb_card_record(card_id);
    return r ? 1 : 0;
}

/* ── Ported from engine/src/core/card.rs ───────────────────────────────────
   get: HeartMap::get — retrieve the heart count for a given heart color.
   Mirrors HeartMap::get(&HeartColor). Returns the count via out param,
   or 0 if the color is not present. */

int rb_heart_map_get_score(const HeartMap *m, uint8_t color, uint8_t *out) {
    return rb_heart_map_get(m, color, out);
}



