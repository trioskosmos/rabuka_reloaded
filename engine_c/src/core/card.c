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

