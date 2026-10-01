/* describe.c — faithful port of engine/src/ability/describe.rs
 *
 * This file renders an ability's parsed structure back to human-readable text.
 * It is a CHARACTERIZATION ORACLE, not decoration: tests/test_parity_char2.c
 * section B (describe_parity_test.rs) requires that for every effect node
 * carrying text, describe_effect_en(node) != node.text AND
 * describe_effect_ja(node) != node.text. A template that merely echoes the raw
 * text passes vacuously and hides exactly the decode divergence the Rust test
 * exists to catch, so every arm below is reproduced verbatim -- spacing,
 * punctuation, the Japanese particles and the " then " / "、その後" joins
 * included. test_output/describe_dump.json (engine/src/bin/describe_dump.rs)
 * is the byte-level oracle: 936 (describe_en, describe_ja) pairs.
 *
 * Enums are NOT duplicated here. The wire-string tables live in
 * src/ability/enums.c (rb_action_type_*, rb_ability_zone_*, and
 * rb_effect_card_type_*) and are called through, so a new action or zone name
 * cannot drift between the decoder and the description. The debug line buffer
 * in src/ability/debug.c is deliberately NOT used: describe.rs never logs.
 *
 * MODEL NOTES (where the C AbilityEffect differs from the Rust one)
 * --------------------------------------------------------------
 * Rust reads its filter fields out of `EffectFilter` (core/card.rs), which only
 * exists when `EffectKind::from_action(action)` recognises the action AND
 * `build_filter` saw at least one non-default field; every `*_any()` getter then
 * returns None. The C decoder has no such gate: it retains every wire key in
 * `extra_k`/`extra_v` unconditionally. The getters below therefore re-apply the
 * type-level gates Rust has, so the two agree:
 *
 *   - u8 fields (`count`, `value`, `cost_limit`, `energy_count`,
 *     `target_count`, `count_as`, `heart_total_limit`, `repeat_limit`) go
 *     through d_u8_extra(), which reproduces `u8::try_from(int).ok()`: a
 *     negative or >255 value is None, exactly as Rust's read_u8_value().
 *   - `operation` goes through d_operation(), reproducing parse_operation():
 *     a string outside the seven-name Operation vocabulary is None.
 *   - `card_type` goes through d_card_type(), reproducing CardType::
 *     from_card_str(): only member_card / live_card / energy_card survive.
 *   - `state_change` is read as the RAW extra, NOT through
 *     rb_effect_state_as_str(). Rust types it as EffectState, whose
 *     `Other(ArcStr)` arm preserves an unrecognised value verbatim; the C
 *     rb_effect_state_as_str() maps RB_ES_OTHER to "" and would lose it.
 *   - `source` / `destination` are normalised through the Zone tables so
 *     "energy_zone" -> "energy" and "under" -> "under_member", matching Rust's
 *     Option<Zone>::as_str(). The PUBLIC label helpers (rb_desc_zone_label)
 *     deliberately do NOT normalise, because Rust's public `zone_label` takes a
 *     raw &str -- "energy_zone" is its own arm there.
 *   - Vec<String> filter fields (`group_names`, `counted_by`, `identities`) are
 *     stored by the C decoder as a CSV blob (vm.c decode_extra_value, with
 *     `"`-quoting for empty/','-bearing elements and doubled inner quotes).
 *     d_group_label / d_join_csv re-split that blob.
 *
 * `compound.actions` is wire key "actions" only in Rust. The C decoder merges
 * "actions" / "effect_steps" / "costs" into AbilityEffect::child[], so the
 * sequential arm sees the union. That is a pre-existing C decoder divergence
 * (it affects execution too), not one introduced here; it is recorded in the
 * handoff report.
 */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* ====================================================================
 * Growable string — the C stand-in for Rust's `String`.
 * Allocated through the engine allocator so a no_std build keeps working.
 * ==================================================================== */

typedef struct RbDescBuf {
    char  *s;
    size_t len;
    size_t cap;
} RbDescBuf;

static void dbuf_init(RbDescBuf *b) {
    b->s = NULL;
    b->len = 0;
    b->cap = 0;
}

static void dbuf_free(RbDescBuf *b) {
    if (!b) return;
    rb_free(b->s);
    b->s = NULL;
    b->len = 0;
    b->cap = 0;
}

static void dbuf_reserve(RbDescBuf *b, size_t extra) {
    size_t need = b->len + extra + 1;
    size_t cap;
    char *p;
    if (need <= b->cap) return;
    cap = b->cap ? b->cap : 64;
    while (cap < need) cap *= 2;
    p = (char *)rb_malloc(cap);
    if (!p) return;
    if (b->s) {
        memcpy(p, b->s, b->len + 1);
        rb_free(b->s);
    } else {
        p[0] = 0;
    }
    b->s = p;
    b->cap = cap;
}

static void dbuf_putn(RbDescBuf *b, const char *s, size_t n) {
    if (!b || !s || !n) return;
    dbuf_reserve(b, n);
    if (!b->s) return;
    memcpy(b->s + b->len, s, n);
    b->len += n;
    b->s[b->len] = 0;
}

static void dbuf_puts(RbDescBuf *b, const char *s) {
    if (!s) return;
    dbuf_putn(b, s, strlen(s));
}

static void dbuf_putc(RbDescBuf *b, char c) {
    dbuf_putn(b, &c, 1);
}

static void dbuf_puti(RbDescBuf *b, int v) {
    char t[24];
    snprintf(t, sizeof(t), "%d", v);
    dbuf_puts(b, t);
}

/* Reuse an existing RbDescBuf as a scratch field buffer. */
static void dbuf_reset(RbDescBuf *b) {
    b->len = 0;
    if (b->s) b->s[0] = 0;
}

static char *dbuf_take(RbDescBuf *b) {
    char *out;
    if (!b->s) {
        b->s = (char *)rb_malloc(1);
        if (b->s) b->s[0] = 0;
    }
    out = b->s;
    b->s = NULL;
    b->len = 0;
    b->cap = 0;
    return out;
}

/* Rust `str::trim` (the ASCII whitespace set; every byte this file builds
 * around the trim is ASCII, so the Unicode-only tail of the Rust definition
 * can never be reached here). */
static void dbuf_trim(RbDescBuf *b) {
    size_t start = 0;
    char *p;
    if (!b->s) return;
    p = b->s;
    while (p[start] == ' ' || p[start] == '\t' || p[start] == '\n' ||
           p[start] == '\r' || p[start] == '\f' || p[start] == '\v')
        start++;
    if (start) {
        memmove(p, p + start, b->len - start + 1);
        b->len -= start;
    }
    while (b->len > 0) {
        char c = p[b->len - 1];
        if (c == ' ' || c == '\t' || c == '\n' || c == '\r' ||
            c == '\f' || c == '\v')
            b->len--;
        else
            break;
    }
    p[b->len] = 0;
}

/* ====================================================================
 * Effect-field readers — the C image of the typed `*_any()` getters.
 * `-1` means None throughout, matching Rust's Option<T>.
 * ==================================================================== */

static const char *d_extra(const AbilityEffect *e, const char *key) {
    int i;
    if (!e || !key) return NULL;
    for (i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}

/* Option<u8> off an extra. Reproduces vm.rs read_u8_value ->
   u8::try_from(int).ok(): TAG_NULL is None, and so is any integer that does
   not fit in 0..=255. */
static int d_u8_extra(const AbilityEffect *e, const char *key) {
    const char *v = d_extra(e, key);
    char *end = NULL;
    long n;
    if (!v || !*v) return -1;
    n = strtol(v, &end, 10);
    if (end == v || *end) return -1;
    if (n < 0 || n > 255) return -1;
    return (int)n;
}

/* Option<bool> off an extra. read_bool_value only accepts TAG_TRUE /
   TAG_FALSE, so a numeric extra is None, not a truthiness test. */
static int d_bool_extra(const AbilityEffect *e, const char *key, int *out) {
    const char *v = d_extra(e, key);
    if (!v) return 0;
    if (!strcmp(v, "true"))  { *out = 1; return 1; }
    if (!strcmp(v, "false")) { *out = 0; return 1; }
    return 0;
}

/* AbilityEffect::action is an ActionType VALUE, not an Option: the decoder
   maps an empty or unrecognised action string to ActionType::default()
   (Custom), and every lookup below dispatches on `to_str`. */
static const char *d_action(const AbilityEffect *e) {
    const char *a = e ? e->action : NULL;
    const char *custom = rb_action_type_to_str(rb_action_type_default());
    if (!a || !*a) return custom;
    if (rb_action_type_from_str(a) < 0) return custom;
    return a;
}

static const char *d_text(const AbilityEffect *e) {
    return (e && e->text) ? e->text : "";
}

/* count_any(): the top-level Option<u8> first, then the filter-level copy. */
static int d_count(const AbilityEffect *e) {
    if (!e) return -1;
    if (e->count >= 0) return e->count > 255 ? -1 : e->count;
    return d_u8_extra(e, "count");
}

/* Option<bool> for `optional`: struct flag first (the decoder only raises it
   on TRUE), then the retained extra. */
static int d_optional_some(const AbilityEffect *e, int *out) {
    if (!e) return 0;
    if (e->is_optional) { *out = 1; return 1; }
    return d_bool_extra(e, "optional", out);
}
static int d_optional(const AbilityEffect *e) {
    int v = 0;
    return d_optional_some(e, &v) ? v : 0;
}

static const char *d_zone(const char *raw) {
    if (!raw) return NULL;
    return rb_ability_zone_as_str(rb_ability_zone_from_source_str(raw));
}

/* source_any(): Option<Zone> on the effect, else the filter-level copy. */
static const char *d_source(const AbilityEffect *e) {
    if (!e) return NULL;
    if (e->source) return d_zone(e->source);
    return d_zone(d_extra(e, "source"));
}

/* destination_any(): same shape, and this is the one describe.rs reads for
   the `d` binding (describe.rs:187 uses `destination`, not
   `destination_any`). */
static const char *d_destination(const AbilityEffect *e) {
    if (!e) return NULL;
    if (e->destination) return d_zone(e->destination);
    return d_zone(d_extra(e, "destination"));
}

static const char *d_target(const AbilityEffect *e) {
    if (!e) return NULL;
    if (e->target) return e->target;
    return d_extra(e, "target");
}

/* card_type_any() -> CardType::as_card_str(). CardType::from_card_str only
   knows three names, so anything else is None -- which renders as the "card"
   default label rather than being echoed verbatim. */
static const char *d_card_type(const AbilityEffect *e) {
    const char *raw = e ? e->card_type_field : NULL;
    int ect;
    if (!raw || !*raw) raw = d_extra(e, "card_type");
    if (!raw || !*raw) return NULL;
    ect = rb_effect_card_type_from_str(raw);
    if (ect != RB_ECT_MEMBER_CARD && ect != RB_ECT_LIVE_CARD &&
        ect != RB_ECT_ENERGY_CARD)
        return NULL;
    return rb_effect_card_type_as_str(ect);
}

/* operation_any(): Operation::as_str(), and parse_operation() rejects every
   string outside the seven-name vocabulary. */
static const char *d_operation(const AbilityEffect *e) {
    const char *v = d_extra(e, "operation");
    if (!v || !*v) return NULL;
    if (!strcmp(v, "add") || !strcmp(v, "decrease") || !strcmp(v, "increase") ||
        !strcmp(v, "remove") || !strcmp(v, "set") || !strcmp(v, "subtract") ||
        !strcmp(v, "set_from_reference"))
        return v;
    return NULL;
}

/* state_change_any(): EffectState::as_str(), which preserves an unrecognised
   value verbatim. Read raw; rb_effect_state_as_str() would fold it to "". */
static const char *d_state_change(const AbilityEffect *e) {
    return d_extra(e, "state_change");
}

/* repeat_limit_any() — already implemented on the C side (core/card.c), which
   consults both the struct field and the extra. Reused, not duplicated. */
static int d_repeat_limit(const AbilityEffect *e) {
    return rb_effect_repeat_limit_any(e);
}

static int d_self_cost(const AbilityEffect *e) {
    int v = 0;
    if (!d_bool_extra(e, "self_cost", &v)) return 0;
    return v == 1;
}

/* alternative_effect_any(): the filter-level sub-effect, which the C decoder
   stores in the dedicated alternative_effect field. */
static const AbilityEffect *d_alternative_effect(const AbilityEffect *e) {
    return e ? e->alternative_effect : NULL;
}

/* ====================================================================
 * CSV re-splitter for the Vec<String> filter fields.
 * vm.c decode_extra_value renders a string array as "a,b,c", quoting an
 * element that is empty or contains , " CR or LF, and doubling an inner ".
 * ==================================================================== */

typedef struct RbDescCsv {
    const char *p;
} RbDescCsv;

static void dcsv_init(RbDescCsv *it, const char *s) {
    it->p = s ? s : "";
}

/* Writes the next field into `out` (reset first). 0 at end of input. */
static int dcsv_next(RbDescCsv *it, RbDescBuf *out) {
    if (!it->p || !*it->p) return 0;
    dbuf_reset(out);
    if (*it->p == '"') {
        it->p++;
        while (*it->p) {
            if (*it->p == '"') {
                if (it->p[1] == '"') { dbuf_putc(out, '"'); it->p += 2; continue; }
                it->p++;
                break;
            }
            dbuf_putc(out, *it->p++);
        }
    } else {
        while (*it->p && *it->p != ',') dbuf_putc(out, *it->p++);
    }
    if (*it->p == ',') it->p++;
    return 1;
}

/* ====================================================================
 * plural / maybe_plural (describe.rs:10-20)
 * ==================================================================== */

static void d_plural(RbDescBuf *b, int n, const char *word) {
    dbuf_puti(b, n);
    dbuf_putc(b, ' ');
    dbuf_puts(b, word);
    if (n != 1) dbuf_putc(b, 's');
}

static void d_maybe_plural(RbDescBuf *b, int c, const char *word) {
    d_plural(b, c < 0 ? 1 : c, word);
}

/* ====================================================================
 * zone_label_inner (describe.rs:22-65) — NOT normalised; Rust's public
 * `zone_label` takes a raw &str and has its own "energy_zone" arm.
 * A NULL zone is Rust's None.
 * ==================================================================== */

static const char *d_zone_label(const char *zone, int ja) {
    if (ja) {
        if (!zone) return "不明";
        if (!strcmp(zone, "hand"))             return "手札";
        if (!strcmp(zone, "discard") || !strcmp(zone, "waitroom")) return "控え室";
        if (!strcmp(zone, "deck"))             return "デッキ";
        if (!strcmp(zone, "deck_top"))         return "デッキの上";
        if (!strcmp(zone, "deck_bottom"))      return "デッキの下";
        if (!strcmp(zone, "stage"))            return "ステージ";
        if (!strcmp(zone, "energy"))           return "エネルギー";
        if (!strcmp(zone, "energy_deck"))      return "エネルギーデッキ";
        if (!strcmp(zone, "energy_zone"))      return "エネルギーゾーン";
        if (!strcmp(zone, "success_zone"))     return "成功ライブカード置き場";
        if (!strcmp(zone, "live_card_zone"))   return "ライブカードゾーン";
        if (!strcmp(zone, "under_member"))     return "このメンバーの下";
        if (!strcmp(zone, "revealed_cards"))   return "公開されたカード";
        if (!strcmp(zone, "those_cards"))      return "それらのカード";
        if (!strcmp(zone, "all_selected"))     return "選択したカード";
        return zone;
    }
    if (!zone) return "unknown";
    if (!strcmp(zone, "hand"))           return "hand";
    if (!strcmp(zone, "discard"))         return "the waiting room";
    if (!strcmp(zone, "deck"))            return "deck";
    if (!strcmp(zone, "deck_top"))        return "top of deck";
    if (!strcmp(zone, "deck_bottom"))     return "bottom of deck";
    if (!strcmp(zone, "stage"))           return "stage";
    if (!strcmp(zone, "energy"))          return "energy";
    if (!strcmp(zone, "energy_deck"))     return "energy deck";
    if (!strcmp(zone, "energy_zone"))     return "energy zone";
    if (!strcmp(zone, "waitroom"))        return "wait room";
    if (!strcmp(zone, "success_zone"))    return "success zone";
    if (!strcmp(zone, "live_card_zone"))  return "live card zone";
    if (!strcmp(zone, "under_member"))    return "under this member";
    if (!strcmp(zone, "revealed_cards"))  return "revealed cards";
    if (!strcmp(zone, "those_cards"))     return "those cards";
    if (!strcmp(zone, "all_selected"))    return "selected cards";
    return zone;
}

/* ====================================================================
 * card_type_label_inner (describe.rs:71-91)
 * ==================================================================== */

static const char *d_card_type_label(const char *ct, int ja) {
    if (ja) {
        if (!ct) return "カード";
        if (!strcmp(ct, "member_card")) return "メンバー";
        if (!strcmp(ct, "live_card"))   return "ライブカード";
        if (!strcmp(ct, "energy_card")) return "エネルギー";
        if (!strcmp(ct, "card"))        return "カード";
        return ct;
    }
    if (!ct) return "card";
    if (!strcmp(ct, "member_card")) return "member";
    if (!strcmp(ct, "live_card"))   return "live card";
    if (!strcmp(ct, "energy_card")) return "energy";
    if (!strcmp(ct, "card"))        return "card";
    return ct;
}

/* ====================================================================
 * state_verb_inner (describe.rs:97-113)
 * ==================================================================== */

static const char *d_state_verb(const char *state, int ja) {
    if (ja) {
        if (!state) return "状態変更";
        if (!strcmp(state, "wait"))   return "ウェイト";
        if (!strcmp(state, "active")) return "アクティブ";
        return state;
    }
    if (!state) return "Change state of";
    if (!strcmp(state, "wait"))   return "Rest";
    if (!strcmp(state, "active")) return "Activate";
    return state;
}

/* ====================================================================
 * resource_label_inner (describe.rs:115-131)
 * ==================================================================== */

static const char *d_resource_label(const char *r, int ja) {
    if (ja) {
        if (!r) return "リソース";
        if (!strcmp(r, "blade")) return "ブレード";
        if (!strcmp(r, "heart")) return "ハート";
        return r;
    }
    if (!r) return "resource";
    if (!strcmp(r, "blade")) return "blade";
    if (!strcmp(r, "heart")) return "heart";
    return r;
}

/* ====================================================================
 * duration_label_inner (describe.rs:133-153)
 * ==================================================================== */

static const char *d_duration_label(const char *d, int ja) {
    if (ja) {
        if (!d) return "";
        if (!strcmp(d, "live_end"))     return "ライブ終了時まで";
        if (!strcmp(d, "live_start"))   return "このライブの間";
        if (!strcmp(d, "live_success")) return "ライブ成功時";
        if (!strcmp(d, "turn_end") || !strcmp(d, "turn")) return "ターン終了時まで";
        return d;
    }
    if (!d) return "";
    if (!strcmp(d, "live_end"))     return "until end of live";
    if (!strcmp(d, "live_start"))   return "for this live";
    if (!strcmp(d, "live_success")) return "on live success";
    if (!strcmp(d, "turn_end") || !strcmp(d, "turn")) return "until end of turn";
    return d;
}

/* ====================================================================
 * group_label (describe.rs:155-160) — " a/b group", or "" for None/empty.
 * ==================================================================== */

static void d_group_label(RbDescBuf *b, const AbilityEffect *e, const char *key) {
    const char *v = d_extra(e, key);
    RbDescBuf joined, field;
    RbDescCsv it;
    int n = 0;
    if (!v) return;                       /* Option::None -> String::new() */
    dbuf_init(&joined);
    dbuf_init(&field);
    dcsv_init(&it, v);
    while (dcsv_next(&it, &field)) {
        if (n++) dbuf_putc(&joined, '/');
        dbuf_puts(&joined, field.s ? field.s : "");
    }
    dbuf_free(&field);
    if (n > 0) {
        dbuf_putc(b, ' ');
        dbuf_puts(b, joined.s ? joined.s : "");
        dbuf_puts(b, " group");
    }
    dbuf_free(&joined);
}

/* Join a Vec<String> extra with ", " (the `identities_any()` shape). */
static void d_join_csv(RbDescBuf *b, const AbilityEffect *e, const char *key,
                       const char *sep) {
    const char *v = d_extra(e, key);
    RbDescBuf field;
    RbDescCsv it;
    int n = 0;
    if (!v) return;
    dbuf_init(&field);
    dcsv_init(&it, v);
    while (dcsv_next(&it, &field)) {
        if (n++) dbuf_puts(b, sep);
        dbuf_puts(b, field.s ? field.s : "");
    }
    dbuf_free(&field);
}

/* ====================================================================
 * describe_effect (describe.rs:180-953)
 * ==================================================================== */

static void d_effect_into(RbDescBuf *b, const AbilityEffect *e, int ja);

/* Public entry points, declared here because the compound arms recurse into
 * them before their definitions further down the file. */
char *rb_desc_effect(const AbilityEffect *effect, int ja);
char *rb_desc_cost(const AbilityEffect *cost, int ja);
void  rb_desc_free(char *s);

static void d_effect_into(RbDescBuf *b, const AbilityEffect *e, int ja)
{
    const char *action;
    const char *ct, *t, *s, *d, *s_change;
    RbDescBuf gn;
    int c;

    if (!e) {
        /* Rust takes &AbilityEffect, so a reference is always present. The C
           entry points accept NULL defensively; the decoder's own default is
           an empty text, which is what the fallback arm renders. */
        return;
    }

    action  = d_action(e);
    ct      = d_card_type_label(d_card_type(e), ja);
    c       = d_count(e);
    t       = d_target(e);
    s       = d_source(e);
    d       = d_destination(e);
    s_change = d_state_change(e);
    dbuf_init(&gn);
    d_group_label(&gn, e, "group_names");

    /* ---- replacement_appear_wait -------------------------------- */
    if (!strcmp(action, "replacement_appear_wait")) {
        int limit = d_u8_extra(e, "heart_total_limit");
        if (limit < 0) limit = 0;
        if (ja) {
            dbuf_puts(b, "置き換え：{{center.png|センター}}エリアに、元々持つハートが");
            dbuf_puti(b, limit);
            dbuf_puts(b, "つ以下のメンバーは、{{icon_blade.png|ウェイト}}状態で登場する");
        } else {
            dbuf_puts(b, "Replacement: members with ");
            dbuf_puti(b, limit);
            dbuf_puts(b, " or fewer printed hearts appear in wait in "
                         "{{center.png|the centre}} area");
        }
        goto done;
    }

    /* ---- count_as_multiple -------------------------------------- */
    if (!strcmp(action, "count_as_multiple")) {
        int weight = d_u8_extra(e, "count_as");
        RbDescBuf by;
        if (weight < 0) weight = 2;
        dbuf_init(&by);
        d_group_label(&by, e, "counted_by");
        if (ja) {
            if (!by.s || !by.len) {
                dbuf_puts(b, "このカードは");
                dbuf_puti(b, weight);
                dbuf_puts(b, "枚として扱う");
            } else {
                dbuf_puts(b, by.s);
                dbuf_puts(b, "のカードの効果による枚数の数え方では、このカードは");
                dbuf_puti(b, weight);
                dbuf_puts(b, "枚として扱う");
            }
        } else {
            if (!by.s || !by.len) {
                dbuf_puts(b, "This card counts as ");
                dbuf_puti(b, weight);
            } else {
                dbuf_puts(b, "When counted by ");
                dbuf_puts(b, by.s);
                dbuf_puts(b, " card effects, this card counts as ");
                dbuf_puti(b, weight);
            }
        }
        dbuf_free(&by);
        goto done;
    }

    /* ---- move_cards --------------------------------------------- */
    if (!strcmp(action, "move_cards")) {
        const char *dest = d_zone_label(d, ja);
        if ((s && (!strcmp(s, "those_cards") || !strcmp(s, "all_selected")))) {
            if (ja) {
                dbuf_puts(b, "選択したカードを");
                dbuf_puts(b, dest);
                dbuf_puts(b, "に置く");
            } else {
                dbuf_puts(b, "Move the selected card(s) to ");
                dbuf_puts(b, dest);
            }
        } else {
            const char *src = d_zone_label(s, ja);
            if (ja) {
                if (c == 1) {
                    dbuf_puts(b, "1の");
                } else {
                    dbuf_puti(b, c < 0 ? 1 : c);
                    dbuf_puts(b, "枚の");
                }
                dbuf_puts(b, ct);
                dbuf_puts(b, "を");
                dbuf_puts(b, src);
                dbuf_puts(b, "から");
                dbuf_puts(b, dest);
                dbuf_puts(b, "に置く");
            } else {
                dbuf_puts(b, "Place ");
                d_maybe_plural(b, c, ct);
                dbuf_puts(b, " from ");
                dbuf_puts(b, src);
                dbuf_puts(b, " to ");
                dbuf_puts(b, dest);
            }
            if (s_change && !strcmp(s_change, "wait"))
                dbuf_puts(b, ja ? "（レスト）" : " (rest)");
        }
        goto done;
    }

    /* ---- draw_card ---------------------------------------------- */
    if (!strcmp(action, "draw_card")) {
        if (ja) {
            if (s) {
                dbuf_puts(b, d_zone_label(s, ja));
                dbuf_puts(b, "から");
            }
            if (c == 1) dbuf_puts(b, "1枚");
            else { dbuf_puti(b, c < 0 ? 1 : c); dbuf_puts(b, "枚"); }
            dbuf_puts(b, "引く");
        } else if (s) {
            dbuf_puts(b, "Draw ");
            d_maybe_plural(b, c, "card");
            dbuf_puts(b, " from ");
            dbuf_puts(b, d_zone_label(s, ja));
        } else {
            d_maybe_plural(b, c, "card");
        }
        goto done;
    }

    /* ---- gain_resource ------------------------------------------ */
    if (!strcmp(action, "gain_resource")) {
        const char *r = d_resource_label(d_extra(e, "resource"), ja);
        const char *dur_raw = d_extra(e, "duration");
        RbDescBuf dur_lbl_buf, dur_str, count_str;
        const char *dur_str_p;

        /* duration_label_inner returns "" for an unrecognised value, which
           Rust maps back to None; the empty label is therefore suppressed. */
        dbuf_init(&dur_lbl_buf);
        if (dur_raw) {
            const char *lbl = d_duration_label(dur_raw, ja);
            if (lbl && *lbl) dbuf_puts(&dur_lbl_buf, lbl);
        }
        dbuf_init(&dur_str);
        if (dur_lbl_buf.len > 0) {
            if (ja) {
                dbuf_puts(&dur_str, "（");
                dbuf_puts(&dur_str, dur_lbl_buf.s);
                dbuf_puts(&dur_str, "）");
            } else {
                dbuf_putc(&dur_str, ' ');
                dbuf_puts(&dur_str, dur_lbl_buf.s);
            }
        }
        dur_str_p = dur_str.s ? dur_str.s : "";

        dbuf_init(&count_str);
        if (ja) {
            if (c == 1) dbuf_puts(&count_str, r);
            else {
                dbuf_puti(&count_str, c < 0 ? 1 : c);
                dbuf_putc(&count_str, ' ');
                dbuf_puts(&count_str, r);
            }
            if (t && !strcmp(t, "opponent")) {
                dbuf_puts(b, "相手に");
                dbuf_puts(b, count_str.s ? count_str.s : "");
                dbuf_puts(b, "を与える");
                dbuf_puts(b, dur_str_p);
            } else {
                dbuf_puts(b, count_str.s ? count_str.s : "");
                dbuf_puts(b, gn.s ? gn.s : "");
                dbuf_puts(b, "を得る");
                dbuf_puts(b, dur_str_p);
            }
        } else {
            d_maybe_plural(&count_str, c, r);
            if (t && !strcmp(t, "opponent")) {
                dbuf_puts(b, "Give ");
                dbuf_puts(b, count_str.s ? count_str.s : "");
                dbuf_puts(b, " to opponent");
                dbuf_puts(b, dur_str_p);
            } else {
                dbuf_puts(b, "Gain ");
                dbuf_puts(b, count_str.s ? count_str.s : "");
                dbuf_puts(b, gn.s ? gn.s : "");
                dbuf_puts(b, dur_str_p);
            }
        }
        dbuf_free(&count_str);
        dbuf_free(&dur_str);
        dbuf_free(&dur_lbl_buf);
        goto done;
    }

    /* ---- change_state ------------------------------------------- */
    if (!strcmp(action, "change_state")) {
        const char *verb = d_state_verb(s_change, ja);
        int cnt = c < 0 ? 1 : c;
        int cl = d_u8_extra(e, "cost_limit");
        if (ja) {
            const char *who = (t && !strcmp(t, "opponent")) ? "相手の" : "";
            const char *loc = s ? d_zone_label(s, ja) : "";
            dbuf_puts(b, who);
            if (!loc || !*loc) {
                /* "{who}を{cnt}体{gn}{verb}にする{lim}" -- there is NO second
                 * を on this branch; only the loc-bearing form carries one. */
                dbuf_puts(b, "を");
                dbuf_puti(b, cnt);
                dbuf_puts(b, "体");
                dbuf_puts(b, gn.s ? gn.s : "");
                dbuf_puts(b, verb);
                dbuf_puts(b, "にする");
                if (cl >= 0) {
                    dbuf_puts(b, "（コスト");
                    dbuf_puti(b, cl);
                    dbuf_puts(b, "以下）");
                }
            } else {
                dbuf_puts(b, "の");
                dbuf_puts(b, loc);
                dbuf_puti(b, cnt);
                dbuf_puts(b, "体");
                dbuf_puts(b, gn.s ? gn.s : "");
                dbuf_puts(b, "を");
                dbuf_puts(b, verb);
                dbuf_puts(b, "にする");
                if (cl >= 0) {
                    dbuf_puts(b, "（コスト");
                    dbuf_puti(b, cl);
                    dbuf_puts(b, "以下）");
                }
            }
        } else {
            RbDescBuf tmp;
            dbuf_init(&tmp);
            dbuf_puts(&tmp, verb);
            dbuf_putc(&tmp, ' ');
            dbuf_puti(&tmp, cnt);
            dbuf_puts(&tmp, gn.s ? gn.s : "");
            if (s) {
                dbuf_puts(&tmp, " on ");
                dbuf_puts(&tmp, d_zone_label(s, ja));
            }
            dbuf_putc(&tmp, ' ');
            if (t && !strcmp(t, "opponent")) dbuf_puts(&tmp, "opponent ");
            if (cl >= 0) {
                dbuf_puts(&tmp, " (cost ≤ ");
                dbuf_puti(&tmp, cl);
                dbuf_puts(&tmp, ")");
            }
            dbuf_trim(&tmp);
            dbuf_puts(b, tmp.s ? tmp.s : "");
            dbuf_free(&tmp);
        }
        goto done;
    }

    /* ---- modify_score ------------------------------------------- */
    if (!strcmp(action, "modify_score")) {
        int val = d_u8_extra(e, "value");
        const char *op;
        int sub;
        if (val < 0) val = 1;
        op = d_operation(e);
        if (!op) op = "add";
        sub = !strcmp(op, "subtract");
        if (ja) {
            dbuf_puts(b, sub ? "スコアを" : "スコアを");
            dbuf_puti(b, val);
            dbuf_puts(b, sub ? "減らす" : "増やす");
        } else {
            dbuf_puts(b, sub ? "Subtract " : "Add ");
            dbuf_puti(b, val);
            dbuf_puts(b, sub ? " from score" : " to score");
        }
        goto done;
    }

    /* ---- position_change ---------------------------------------- */
    if (!strcmp(action, "position_change")) {
        const char *ep = d_extra(e, "exclude_position");
        if (ep) {
            if (ja) {
                dbuf_puts(b, ep);
                dbuf_puts(b, "を避けてポジションチェンジ");
            } else {
                dbuf_puts(b, "Move a");
                dbuf_puts(b, gn.s ? gn.s : "");
                dbuf_puts(b, " member away from ");
                dbuf_puts(b, ep);
            }
        } else if (c == 1 || c < 0) {
            if (ja) {
                dbuf_puts(b, "ポジションチェンジ");
                dbuf_puts(b, gn.s ? gn.s : "");
            } else {
                dbuf_puts(b, "Change position of a");
                dbuf_puts(b, gn.s ? gn.s : "");
                dbuf_puts(b, " member");
            }
        } else if (ja) {
            dbuf_puti(b, c);
            dbuf_puts(b, "体ポジションチェンジ");
            dbuf_puts(b, gn.s ? gn.s : "");
        } else {
            dbuf_puts(b, "Change position of ");
            dbuf_puti(b, c);
            dbuf_puts(b, gn.s ? gn.s : "");
            dbuf_puts(b, " members");
        }
        goto done;
    }

    /* ---- select | select_cards ----------------------------------- */
    if (!strcmp(action, "select") || !strcmp(action, "select_cards")) {
        const char *src = d_zone_label(s, ja);
        const char *opt = d_optional(e) ? (ja ? "（任意）" : " (optional)") : "";
        if (ja) {
            dbuf_puts(b, src);
            dbuf_puts(b, "から");
            dbuf_puti(b, c < 0 ? 1 : c);
            dbuf_puts(b, "枚の");
            dbuf_puts(b, ct);
            dbuf_puts(b, "を選ぶ");
            dbuf_puts(b, opt);
        } else {
            dbuf_puts(b, "Select ");
            d_maybe_plural(b, c, ct);
            dbuf_puts(b, " from ");
            dbuf_puts(b, src);
            dbuf_puts(b, gn.s ? gn.s : "");
            dbuf_puts(b, opt);
        }
        goto done;
    }

    /* ---- look_at / reveal --------------------------------------- */
    if (!strcmp(action, "look_at") || !strcmp(action, "reveal")) {
        int look = !strcmp(action, "look_at");
        if (ja) {
            dbuf_puts(b, d_zone_label(s, ja));
            dbuf_puts(b, "のカードを");
            if (c == 1) dbuf_puts(b, "1枚");
            else { dbuf_puti(b, c < 0 ? 1 : c); dbuf_puts(b, "枚"); }
            dbuf_puts(b, look ? "見る" : "公開する");
        } else {
            dbuf_puts(b, look ? "Look at " : "Reveal ");
            d_maybe_plural(b, c, "card");
            dbuf_puts(b, " from ");
            dbuf_puts(b, d_zone_label(s, ja));
        }
        goto done;
    }

    /* ---- pay_energy --------------------------------------------- */
    if (!strcmp(action, "pay_energy")) {
        const char *opt = d_optional(e) ? (ja ? "（任意）" : " (optional)") : "";
        if (ja) {
            dbuf_puts(b, "エネルギーを");
            dbuf_puti(b, c < 0 ? 1 : c);
            dbuf_puts(b, "払う");
            dbuf_puts(b, opt);
        } else {
            dbuf_puts(b, "Pay ");
            dbuf_puti(b, c < 0 ? 1 : c);
            dbuf_puts(b, " energy");
            dbuf_puts(b, opt);
        }
        goto done;
    }

    /* ---- look_and_select ---------------------------------------- */
    if (!strcmp(action, "look_and_select")) {
        const AbilityEffect *la = e->look_action;
        const AbilityEffect *sa = e->select_action;
        int lc = la ? (la->count >= 0 ? la->count : -1) : -1;
        int sc = sa ? (sa->count >= 0 ? sa->count : -1) : -1;
        const char *sdest = sa ? d_zone_label(d_zone(sa->destination), ja) : NULL;
        if (lc >= 0 && sc >= 0) {
            if (ja) {
                dbuf_puti(b, lc);
                dbuf_puts(b, "枚見て、");
                dbuf_puti(b, sc);
                dbuf_puts(b, "枚を");
                dbuf_puts(b, sdest ? sdest : "手札");
                dbuf_puts(b, "に選ぶ");
            } else {
                dbuf_puts(b, "Look at ");
                d_plural(b, lc, "card");
                dbuf_puts(b, "; pick ");
                dbuf_puti(b, sc);
                dbuf_puts(b, " to ");
                dbuf_puts(b, sdest ? sdest : "hand");
            }
        } else if (ja) {
            dbuf_puts(b, "カードを見て選ぶ");
        } else {
            dbuf_puts(b, "Look at cards \xe2\x86\x92 pick");
        }
        goto done;
    }

    /* ---- restriction -------------------------------------------- */
    if (!strcmp(action, "restriction")) {
        const char *rt = d_extra(e, "restriction_type");
        if (!rt) rt = ja ? "制限" : "restriction";
        if (ja) {
            if (!strcmp(rt, "cannot_baton_touch"))
                dbuf_puts(b, "バトンタッチでは控え室に置かれない");
            else if (!strcmp(rt, "cannot_activate"))
                dbuf_puts(b, "起動できない");
            else if (!strcmp(rt, "cannot_activate_by_effect"))
                dbuf_puts(b, "効果によってはアクティブにならない");
            else if (!strcmp(rt, "cannot_live"))
                dbuf_puts(b, "ライブできない");
            else if (!strcmp(rt, "cannot_place"))
                dbuf_puts(b, "成功ライブカード置き場に置くことができない");
            else if (!strcmp(rt, "cannot_wait_by_effect"))
                dbuf_puts(b, "相手の効果によってはウェイトしない");
            else {
                dbuf_puts(b, rt);
                dbuf_puts(b, "制限を適用");
            }
        } else {
            if (!strcmp(rt, "cannot_baton_touch"))
                dbuf_puts(b, "Cannot be replaced by a baton touch (except from excluded groups)");
            else if (!strcmp(rt, "cannot_activate"))
                dbuf_puts(b, "Cannot be activated");
            else if (!strcmp(rt, "cannot_activate_by_effect"))
                dbuf_puts(b, "Cannot be activated by card effects");
            else if (!strcmp(rt, "cannot_live"))
                dbuf_puts(b, "Cannot perform a live");
            else if (!strcmp(rt, "cannot_place"))
                dbuf_puts(b, "Cannot be placed in the success live card zone");
            else if (!strcmp(rt, "cannot_wait_by_effect"))
                dbuf_puts(b, "Do not go to Wait by opponent's effects");
            else {
                dbuf_puts(b, "Restriction: ");
                dbuf_puts(b, rt);
            }
        }
        goto done;
    }

    /* ---- gain_ability ------------------------------------------- */
    if (!strcmp(action, "gain_ability")) {
        const char *ga = d_extra(e, "ability_gain");
        if (ja) {
            dbuf_puts(b, "アビリティを得る");
            if (ga) { dbuf_puts(b, "："); dbuf_puts(b, ga); }
        } else {
            dbuf_puts(b, "Gain ability");
            if (ga) { dbuf_puts(b, ": "); dbuf_puts(b, ga); }
        }
        goto done;
    }

    /* ---- do_nothing --------------------------------------------- */
    if (!strcmp(action, "do_nothing")) goto done;

    /* ---- sequential --------------------------------------------- */
    if (!strcmp(action, "sequential")) {
        if (e->n_child > 0) {
            if (e->n_child == 1) {
                d_effect_into(b, e->child[0], ja);
            } else if (ja) {
                int i;
                for (i = 0; i < e->n_child - 1; i++) {
                    if (i) dbuf_puts(b, "、");
                    d_effect_into(b, e->child[i], ja);
                }
                dbuf_puts(b, "、その後");
                d_effect_into(b, e->child[e->n_child - 1], ja);
            } else {
                int i;
                for (i = 0; i < e->n_child - 1; i++) {
                    if (i) dbuf_puts(b, "; ");
                    d_effect_into(b, e->child[i], ja);
                }
                dbuf_puts(b, "; then ");
                d_effect_into(b, e->child[e->n_child - 1], ja);
            }
        } else {
            dbuf_puts(b, d_text(e));
        }
        goto done;
    }

    /* ---- choice ------------------------------------------------- */
    if (!strcmp(action, "choice")) {
        dbuf_puts(b, ja ? "1つを選ぶ" : "Choose 1");
        goto done;
    }

    /* ---- play_baton_touch --------------------------------------- */
    if (!strcmp(action, "play_baton_touch")) {
        if (ja) {
            dbuf_puts(b, "バトンタッチ");
            if (c != 1) {
                dbuf_puts(b, "（");
                dbuf_puti(b, c < 0 ? 1 : c);
                dbuf_puts(b, "体）");
            }
        } else {
            dbuf_puts(b, "Baton touch ");
            dbuf_puti(b, c < 0 ? 1 : c);
            dbuf_puts(b, c == 1 ? " member" : " members");
        }
        goto done;
    }

    /* ---- modify_required_hearts [_global] ----------------------- */
    if (!strcmp(action, "modify_required_hearts") ||
        !strcmp(action, "modify_required_hearts_global")) {
        int val = d_u8_extra(e, "value");
        if (val < 0) val = 1;
        if (val == 0) {
            dbuf_puts(b, ja ? "必要ハートをクリア" : "Clear required hearts");
        } else if (val > 0) {
            if (ja) { dbuf_puts(b, "必要ハートを"); dbuf_puti(b, val); dbuf_puts(b, "増やす"); }
            else    { dbuf_puts(b, "Increase required hearts by "); dbuf_puti(b, val); }
        } else if (ja) {
            dbuf_puts(b, "必要ハートを"); dbuf_puti(b, val); dbuf_puts(b, "減らす");
        } else {
            dbuf_puts(b, "Decrease required hearts by "); dbuf_puti(b, val);
        }
        goto done;
    }

    /* ---- activate_ability --------------------------------------- */
    if (!strcmp(action, "activate_ability")) {
        /* The trigger leads the sentence; the noun is a literal suffix. This is
         * NOT the gain_ability "Ability: {x}" shape. */
        const char *at = d_extra(e, "target_trigger");
        if (ja) {
            if (at) dbuf_puts(b, at);
            dbuf_puts(b, "アビリティを発動");
        } else {
            if (at) {
                dbuf_puts(b, "Activate ");
                dbuf_puts(b, at);
                dbuf_puts(b, " ability");
            } else {
                dbuf_puts(b, "Activate ability");
            }
        }
        goto done;
    }

    /* ---- set_card_identity --------------------------------------- */
    if (!strcmp(action, "set_card_identity")) {
        const char *ids = d_extra(e, "identities");
        if (ids) {
            if (ja) dbuf_puts(b, "扱い：");
            else    dbuf_puts(b, "Treat as: ");
            d_join_csv(b, e, "identities", ", ");
        } else {
            dbuf_puts(b, ja ? "カードの扱いを設定" : "Set card identity");
        }
        goto done;
    }

    /* ---- reduce_live_card_set_limit ------------------------------ */
    if (!strcmp(action, "reduce_live_card_set_limit")) {
        if (ja) {
            dbuf_puts(b, "次のライブカードセットフェイズの上限を");
            dbuf_puti(b, c < 0 ? 1 : c);
            dbuf_puts(b, "減らす");
        } else {
            dbuf_puts(b, "Reduce the live card placement limit next Live Set phase by ");
            dbuf_puti(b, c < 0 ? 1 : c);
        }
        goto done;
    }

    /* ---- set_blade_count ---------------------------------------- */
    if (!strcmp(action, "set_blade_count")) {
        if (ja) { dbuf_puts(b, "ブレードを"); dbuf_puti(b, c < 0 ? 0 : c); dbuf_puts(b, "に設定"); }
        else    { dbuf_puts(b, "Set blade to "); dbuf_puti(b, c < 0 ? 0 : c); }
        goto done;
    }

    /* ---- set_heart_type ----------------------------------------- */
    if (!strcmp(action, "set_heart_type")) {
        const char *ht = d_extra(e, "heart_type");
        if (ht) {
            if (ja) {
                dbuf_puts(b, "ハートタイプを");
                dbuf_puts(b, ht);
                dbuf_puts(b, "に設定");
            } else {
                dbuf_puts(b, "Set heart type to ");
                dbuf_puts(b, ht);
            }
        } else {
            dbuf_puts(b, ja ? "ハートタイプを設定" : "Set heart type");
        }
        goto done;
    }

    /* ---- set_blade_type ----------------------------------------- */
    if (!strcmp(action, "set_blade_type")) {
        const char *bt = d_extra(e, "blade_type");
        if (bt) {
            if (ja) {
                dbuf_puts(b, "ブレードタイプを");
                dbuf_puts(b, bt);
                dbuf_puts(b, "に設定");
            } else {
                dbuf_puts(b, "Set blade type to ");
                dbuf_puts(b, bt);
            }
        } else {
            dbuf_puts(b, ja ? "ブレードタイプを設定" : "Set blade type");
        }
        goto done;
    }

    /* ---- discard_until_count ------------------------------------ */
    if (!strcmp(action, "discard_until_count")) {
        int n = d_u8_extra(e, "target_count");
        int tc1 = (d_u8_extra(e, "target_count") == 1);
        if (n < 0) n = 1;
        if (ja) {
            dbuf_puts(b, "手札が");
            dbuf_puti(b, n);
            dbuf_puts(b, "枚になるまで捨てる");
        } else {
            dbuf_puts(b, "Discard down to ");
            dbuf_puti(b, n);
            dbuf_puts(b, " card");
            if (!tc1) dbuf_putc(b, 's');
            dbuf_puts(b, " in hand");
        }
        goto done;
    }

    /* ---- draw_until_count --------------------------------------- */
    if (!strcmp(action, "draw_until_count")) {
        int n = d_u8_extra(e, "target_count");
        int tc1 = (d_u8_extra(e, "target_count") == 1);
        if (n < 0) n = 1;
        if (ja) {
            dbuf_puts(b, "手札が");
            dbuf_puti(b, n);
            dbuf_puts(b, "枚になるまで");
            if (s) { dbuf_puts(b, d_zone_label(s, ja)); dbuf_puts(b, "から"); }
            dbuf_puts(b, "引く");
        } else {
            dbuf_puts(b, "Draw until ");
            dbuf_puti(b, n);
            dbuf_puts(b, " card");
            if (!tc1) dbuf_putc(b, 's');
            dbuf_puts(b, " in hand");
            if (s) {
                dbuf_puts(b, " from ");
                dbuf_puts(b, d_zone_label(s, ja));
            }
        }
        goto done;
    }

    /* ---- modify_cost -------------------------------------------- */
    if (!strcmp(action, "modify_cost")) {
        const char *op = d_operation(e);
        if (!op) op = "subtract";
        if (!strcmp(op, "set")) {
            int v = d_u8_extra(e, "value");
            if (v < 0) {
                dbuf_puts(b, ja ? "コストを設定" : "Set cost");
            } else if (ja) {
                dbuf_puts(b, "コストは");
                dbuf_puti(b, v);
                dbuf_puts(b, "になる");
            } else {
                dbuf_puts(b, "Cost becomes ");
                dbuf_puti(b, v);
            }
        } else {
            int amt = c < 0 ? 1 : c;
            int sub = !strcmp(op, "subtract");
            if (ja) {
                dbuf_puts(b, sub ? "{{icon_energy.png|E}}を" : "{{icon_energy.png|E}}を");
                dbuf_puti(b, amt);
                dbuf_puts(b, sub ? "減らす" : "増やす");
            } else {
                dbuf_puts(b, sub ? "Reduce cost by " : "Increase cost by ");
                dbuf_puti(b, amt);
            }
        }
        goto done;
    }

    /* ---- modify_yell_count -------------------------------------- */
    if (!strcmp(action, "modify_yell_count")) {
        const char *op = d_operation(e);
        int amt = c < 0 ? 1 : c;
        int sub;
        if (!op) op = "add";
        sub = !strcmp(op, "subtract");
        if (ja) {
            dbuf_puts(b, "エール回数を");
            dbuf_puti(b, amt);
            dbuf_puts(b, sub ? "減らす" : "増やす");
        } else {
            dbuf_puts(b, sub ? "Reduce yell count by " : "Increase yell count by ");
            dbuf_puti(b, amt);
        }
        goto done;
    }

    /* ---- perform_yell ------------------------------------------- */
    if (!strcmp(action, "perform_yell")) {
        int amt = c < 0 ? 1 : c;
        int mx = d_repeat_limit(e);
        if (mx < 0) mx = amt;
        if (ja) {
            if (amt == mx) {
                dbuf_puts(b, "エールを");
                dbuf_puti(b, amt);
                dbuf_puts(b, "回行う");
            } else {
                dbuf_puts(b, "最大");
                dbuf_puti(b, mx);
                dbuf_puts(b, "回エールを行う");
            }
        } else if (amt == mx) {
            dbuf_puts(b, "Perform ");
            dbuf_puti(b, amt);
            dbuf_puts(b, amt == 1 ? " yell" : " yells");
        } else {
            dbuf_puts(b, "Perform up to ");
            dbuf_puti(b, mx);
            dbuf_puts(b, mx == 1 ? " yell" : " yells");
        }
        goto done;
    }

    /* ---- re_yell ------------------------------------------------ */
    if (!strcmp(action, "re_yell")) {
        int lose = 0;
        d_bool_extra(e, "lose_blade_hearts", &lose);
        if (ja) {
            dbuf_puts(b, "再エール");
            if (lose) dbuf_puts(b, "（ブレード/ハートを失う）");
        } else {
            dbuf_puts(b, "Re-yell");
            if (lose) dbuf_puts(b, " (lose blade/hearts)");
        }
        goto done;
    }

    /* ---- specify_heart_color ------------------------------------ */
    if (!strcmp(action, "specify_heart_color")) {
        dbuf_puts(b, ja ? "ハートの色を指定する" : "Choose a heart color");
        goto done;
    }

    /* ---- place_energy_under_member ------------------------------ */
    if (!strcmp(action, "place_energy_under_member")) {
        int en = d_u8_extra(e, "energy_count");
        if (en < 0) en = c;
        if (en < 0) en = 1;
        if (ja) {
            dbuf_puts(b, "このメンバーの下にエネルギーを");
            dbuf_puti(b, en);
            dbuf_puts(b, "枚置く");
        } else {
            dbuf_puts(b, "Place ");
            dbuf_puti(b, en);
            dbuf_puts(b, " energy under this member");
        }
        goto done;
    }

    /* ---- invalidate_ability ------------------------------------- */
    if (!strcmp(action, "invalidate_ability")) {
        dbuf_puts(b, ja ? "アビリティを無効にする" : "Cancel an ability");
        goto done;
    }

    /* ---- choose_target_player ----------------------------------- */
    if (!strcmp(action, "choose_target_player")) {
        dbuf_puts(b, ja ? "自分か相手を選ぶ" : "Choose self or opponent");
        goto done;
    }

    /* ---- repeat_procedure --------------------------------------- */
    if (!strcmp(action, "repeat_procedure")) {
        int mx = d_repeat_limit(e);
        if (mx < 0) mx = c < 0 ? 1 : c;
        if (ja) {
            dbuf_puts(b, "最大");
            dbuf_puti(b, mx);
            dbuf_puts(b, "回繰り返す");
        } else {
            dbuf_puts(b, "Repeat up to ");
            dbuf_puti(b, mx);
            dbuf_puts(b, " times");
        }
        goto done;
    }

    /* ---- sequential_cost ---------------------------------------- */
    if (!strcmp(action, "sequential_cost")) {
        if (e->n_child > 0) {
            int i;
            for (i = 0; i < e->n_child; i++) {
                char *part = rb_desc_cost(e->child[i], ja);
                if (i) dbuf_putc(b, '\n');
                dbuf_puts(b, part ? part : "");
                rb_desc_free(part);
            }
            if (d_optional(e->child[e->n_child - 1])) {
                if (ja) dbuf_puts(b, "\n（またはスキップ）");
                else    dbuf_puts(b, "\n(or skip)");
            }
        } else {
            dbuf_puts(b, d_text(e));
        }
        goto done;
    }

    /* ---- reveal_until_live_card --------------------------------- */
    if (!strcmp(action, "reveal_until_live_card")) {
        dbuf_puts(b, ja ? "ライブカードが出るまでデッキを公開する"
                        : "Reveal from deck until a live card appears");
        goto done;
    }

    /* ---- gain_ability_from_source ------------------------------ */
    if (!strcmp(action, "gain_ability_from_source")) {
        dbuf_puts(b, ja ? "アビリティをコピーする" : "Copy an ability");
        goto done;
    }

    /* ---- conditional_on_optional ------------------------------- */
    if (!strcmp(action, "conditional_on_optional")) {
        const AbilityEffect *ca = e->conditional_action;
        if (!ca) {
            dbuf_puts(b, ja ? "コストを払うかスキップ" : "Pay cost or skip");
        } else {
            char *part = rb_desc_effect(ca, ja);
            if (ja) {
                dbuf_puts(b, "コストを払うかスキップ：");
                dbuf_puts(b, part ? part : "");
            } else {
                dbuf_puts(b, "Pay cost or skip: ");
                dbuf_puts(b, part ? part : "");
            }
            rb_desc_free(part);
        }
        goto done;
    }

    /* ---- conditional_on_result --------------------------------- */
    if (!strcmp(action, "conditional_on_result")) {
        const AbilityEffect *p = e->primary_effect;
        const AbilityEffect *f = e->followup_action;
        if (!p && !f) {
            dbuf_puts(b, d_text(e));
        } else if (ja) {
            char *ps = p ? rb_desc_effect(p, ja) : NULL;
            char *fs = f ? rb_desc_effect(f, ja) : NULL;
            dbuf_puts(b, ps ? ps : "");
            if (ps && fs) dbuf_puts(b, "、その後");
            dbuf_puts(b, fs ? fs : "");
            rb_desc_free(ps);
            rb_desc_free(fs);
        } else {
            char *ps = p ? rb_desc_effect(p, 0) : NULL;
            char *fs = f ? rb_desc_effect(f, 0) : NULL;
            dbuf_puts(b, ps ? ps : "");
            if (ps && fs) dbuf_puts(b, "; then ");
            dbuf_puts(b, fs ? fs : "");
            rb_desc_free(ps);
            rb_desc_free(fs);
        }
        goto done;
    }

    /* ---- conditional_alternative -------------------------------- */
    if (!strcmp(action, "conditional_alternative")) {
        const AbilityEffect *p = e->primary_effect;
        const AbilityEffect *a = d_alternative_effect(e);
        if (p && a) {
            char *ps = rb_desc_effect(p, ja);
            char *as = rb_desc_effect(a, ja);
            if (ja) {
                dbuf_puts(b, "どちらか：");
                dbuf_puts(b, ps ? ps : "");
                dbuf_puts(b, " / または：");
                dbuf_puts(b, as ? as : "");
            } else {
                dbuf_puts(b, "Either: ");
                dbuf_puts(b, ps ? ps : "");
                dbuf_puts(b, " / Or: ");
                dbuf_puts(b, as ? as : "");
            }
            rb_desc_free(ps);
            rb_desc_free(as);
        } else if (p) {
            char *ps = rb_desc_effect(p, ja);
            dbuf_puts(b, ps ? ps : "");
            rb_desc_free(ps);
        } else if (a) {
            char *as = rb_desc_effect(a, ja);
            dbuf_puts(b, as ? as : "");
            rb_desc_free(as);
        } else {
            dbuf_puts(b, d_text(e));
        }
        goto done;
    }

    /* ---- select_number ------------------------------------------ */
    if (!strcmp(action, "select_number")) {
        if (ja) {
            dbuf_puts(b, "数を");
            dbuf_puti(b, c < 0 ? 1 : c);
            dbuf_puts(b, "つ選ぶ");
        } else {
            dbuf_puts(b, "Choose a number (");
            dbuf_puti(b, c < 0 ? 1 : c);
            dbuf_putc(b, ')');
        }
        goto done;
    }

    /* ---- modify_yell_source ------------------------------------- */
    if (!strcmp(action, "modify_yell_source")) {
        const char *ys = d_extra(e, "yell_source");
        int bottom = ys && !strcmp(ys, "deck_bottom");
        if (ja) {
            dbuf_puts(b, bottom ? "エールはデッキの下から行う"
                               : "エールの行う場所を変更");
        } else {
            dbuf_puts(b, bottom ? "Perform yells from the bottom of the deck instead of the top"
                               : "Modify the yell source");
        }
        goto done;
    }

    /* ---- suppress_ability_trigger ------------------------------ */
    if (!strcmp(action, "suppress_ability_trigger")) {
        const char *st = d_extra(e, "suppressed_trigger");
        int live_start = st && !strcmp(st, "live_start");
        if (ja)
            dbuf_puts(b, live_start ? "ライブ開始時能力は発動しない"
                                    : "能力は発動しない");
        else
            dbuf_puts(b, live_start ? "Live Start abilities do not activate"
                                    : "Abilities do not activate");
        goto done;
    }

    /* ---- fallback: the node's own text -------------------------- */
    dbuf_puts(b, d_text(e));

done:
    dbuf_free(&gn);
}

/* ====================================================================
 * describe_cost (describe.rs:956-1023)
 * ==================================================================== */

static void d_cost_into(RbDescBuf *b, const AbilityEffect *cost, int ja);

static void d_cost_into(RbDescBuf *b, const AbilityEffect *cost, int ja)
{
    const char *action;

    if (!cost) return;
    action = d_action(cost);

    if (!strcmp(action, "pay_energy")) {
        int count = d_u8_extra(cost, "energy_count");
        if (count < 0) count = 1;
        if (ja) {
            dbuf_puts(b, "{{icon_energy.png|E}}を");
            dbuf_puti(b, count);
            dbuf_puts(b, "払う");
        } else {
            dbuf_puts(b, "Pay ");
            dbuf_puti(b, count);
            dbuf_puts(b, " energy");
        }
        return;
    }

    if (!strcmp(action, "change_state")) {
        if (d_self_cost(cost)) {
            const char *state = d_state_change(cost);
            if (!state) state = "wait";
            if (ja) {
                if (!strcmp(state, "wait")) dbuf_puts(b, "このメンバーをウェイト");
                else {
                    dbuf_puts(b, "このメンバーを");
                    dbuf_puts(b, state);
                    dbuf_puts(b, "にする");
                }
            } else {
                if (!strcmp(state, "wait")) dbuf_puts(b, "Rest this member");
                else {
                    dbuf_puts(b, "Change this member to ");
                    dbuf_puts(b, state);
                }
            }
        } else {
            d_effect_into(b, cost, ja);
        }
        return;
    }

    if (!strcmp(action, "move_cards")) {
        const char *src  = d_zone_label(d_zone(cost->source), ja);
        const char *dest = d_zone_label(d_zone(cost->destination), ja);
        const char *ct   = d_card_type_label(d_card_type(cost), ja);
        int count = cost->count >= 0 ? cost->count : 1;
        if (count > 255) count = 255;
        if (d_self_cost(cost) && cost->source &&
            !strcmp(d_zone(cost->source), "those_cards")) {
            if (ja) {
                dbuf_puts(b, "そのカードを");
                dbuf_puts(b, dest);
                dbuf_puts(b, "に置く");
            } else {
                dbuf_puts(b, "Move that card to ");
                dbuf_puts(b, dest);
            }
        } else if (ja) {
            if (count == 1) dbuf_puts(b, "1の");
            else { dbuf_puti(b, count); dbuf_puts(b, "枚の"); }
            dbuf_puts(b, ct);
            dbuf_puts(b, "を");
            dbuf_puts(b, src);
            dbuf_puts(b, "から");
            dbuf_puts(b, dest);
            dbuf_puts(b, "に置く");
        } else {
            dbuf_puts(b, "Place ");
            dbuf_puti(b, count);
            dbuf_putc(b, ' ');
            dbuf_puts(b, ct);
            dbuf_puts(b, " from ");
            dbuf_puts(b, src);
            dbuf_puts(b, " to ");
            dbuf_puts(b, dest);
        }
        return;
    }

    if (!strcmp(action, "reveal")) {
        int count = cost->count >= 0 ? cost->count : 1;
        const char *source = cost->source ? d_zone(cost->source) : NULL;
        if (!source) source = "hand";
        if (ja) {
            if (count == 1) dbuf_puts(b, "1枚");
            else { dbuf_puti(b, count); dbuf_puts(b, "枚"); }
            dbuf_puts(b, "を");
            dbuf_puts(b, d_zone_label(source, ja));
            dbuf_puts(b, "から公開する");
        } else {
            dbuf_puts(b, "Reveal ");
            dbuf_puti(b, count);
            dbuf_puts(b, " card(s) from ");
            dbuf_puts(b, d_zone_label(source, ja));
        }
        return;
    }

    d_effect_into(b, cost, ja);
}

/* ====================================================================
 * Public surface. Every symbol is rb_desc_* prefixed so it cannot collide
 * with an existing engine symbol; the orchestrator declares them in
 * include/rabuka.h.
 *
 * All string-returning functions hand back an engine-allocator block that the
 * caller releases with rb_desc_free().
 * ==================================================================== */

void rb_desc_free(char *s) {
    if (s) rb_free(s);
}

char *rb_desc_effect(const AbilityEffect *effect, int ja) {
    RbDescBuf b;
    dbuf_init(&b);
    d_effect_into(&b, effect, ja);
    return dbuf_take(&b);
}

char *rb_desc_effect_en(const AbilityEffect *effect) {
    return rb_desc_effect(effect, 0);
}

char *rb_desc_effect_ja(const AbilityEffect *effect) {
    return rb_desc_effect(effect, 1);
}

char *rb_desc_cost(const AbilityEffect *cost, int ja) {
    RbDescBuf b;
    dbuf_init(&b);
    d_cost_into(&b, cost, ja);
    return dbuf_take(&b);
}

char *rb_desc_cost_en(const AbilityEffect *cost) {
    return rb_desc_cost(cost, 0);
}

char *rb_desc_cost_ja(const AbilityEffect *cost) {
    return rb_desc_cost(cost, 1);
}

/* describe_sequential_cost (describe.rs:1028-1042): every cost from 0 up to and
   including choice_index, newline-joined; when the choice cost is optional the
   "(or skip)" tail is appended. `n_costs` bounds the slice; Rust panics on an
   out-of-range choice_index, so the C clamps instead. */
static char *d_sequential_cost(const AbilityEffect *const *costs, int n_costs,
                               int choice_index, int ja) {
    RbDescBuf b;
    int i, last;
    if (!costs || n_costs <= 0) {
        dbuf_init(&b);
        return dbuf_take(&b);
    }
    last = choice_index;
    if (last < 0) last = 0;
    if (last >= n_costs) last = n_costs - 1;
    dbuf_init(&b);
    for (i = 0; i <= last; i++) {
        char *part = rb_desc_cost(costs[i], ja);
        if (i) dbuf_putc(&b, '\n');
        dbuf_puts(&b, part ? part : "");
        rb_desc_free(part);
    }
    if (d_optional(costs[last])) {
        if (ja) dbuf_puts(&b, "（またはスキップ）");
        else    dbuf_puts(&b, "\n(or skip)");
    }
    return dbuf_take(&b);
}

char *rb_desc_sequential_cost(const AbilityEffect *const *costs, int n_costs,
                              int choice_index, int ja) {
    return d_sequential_cost(costs, n_costs, choice_index, ja);
}

char *rb_desc_sequential_cost_en(const AbilityEffect *const *costs, int n_costs,
                                 int choice_index) {
    return d_sequential_cost(costs, n_costs, choice_index, 0);
}

char *rb_desc_sequential_cost_ja(const AbilityEffect *const *costs, int n_costs,
                                 int choice_index) {
    return d_sequential_cost(costs, n_costs, choice_index, 1);
}

/* ── Label helpers (describe.rs:67-155) ──────────────────────────────
 * Rust returns &'static str / &str, so these return a borrowed pointer:
 * either one of the literals below or the caller's own string. Never free. */

const char *rb_desc_zone_label(const char *zone) {
    return d_zone_label(zone, 0);
}

const char *rb_desc_zone_label_ja(const char *zone) {
    return d_zone_label(zone, 1);
}

const char *rb_desc_card_type_label(const char *ct) {
    return d_card_type_label(ct, 0);
}

const char *rb_desc_card_type_label_ja(const char *ct) {
    return d_card_type_label(ct, 1);
}

const char *rb_desc_state_verb(const char *state, int ja) {
    return d_state_verb(state, ja);
}

const char *rb_desc_state_verb_ja(const char *state) {
    return d_state_verb(state, 1);
}

const char *rb_desc_resource_label(const char *resource, int ja) {
    return d_resource_label(resource, ja);
}

const char *rb_desc_resource_label_ja(const char *resource) {
    return d_resource_label(resource, 1);
}

const char *rb_desc_duration_label(const char *duration, int ja) {
    return d_duration_label(duration, ja);
}

/* ====================================================================
 * Bilingual wiring for the C choice model.
 *
 * describe.rs has no such function -- Rust builds a single String per call
 * site. The C RbChoice carries description / description_en / description_ja
 * separately (include/rabuka.h:1108) and rb_full_choice_to_header forwards all
 * three, so this is the adapter that fills them from one effect node. It does
 * no rendering of its own: it just calls describe_effect_en/ja.
 *
 * NOTE for the orchestrator: description[] is 128 bytes and
 * rb_full_choice_set_* uses strncpy, so a long description (the gain_ability
 * arms copy an entire ability text verbatim) is TRUNCATED here. Widening
 * RbFullChoice::description{,_en,_ja} is a rabuka.h change and is therefore a
 * cross-file request, not something this file may do.
 * ==================================================================== */

void rb_desc_apply_bilingual(RbFullChoice *ch, const AbilityEffect *effect, int ja_first) {
    char *en, *ja;
    if (!ch) return;
    en = rb_desc_effect_en(effect);
    ja = rb_desc_effect_ja(effect);
    rb_full_choice_set_bilingual(ch, en, ja);
    if (ja_first) rb_full_choice_set_description(ch, ja);
    else          rb_full_choice_set_description(ch, en);
    rb_desc_free(en);
    rb_desc_free(ja);
}

/* ====================================================================
 * Choice-prompt translation (describe.rs:1087-1158)
 * ==================================================================== */

const char *const rb_desc_choice_prompt_templates_en[] = {
    "Select up to 3 card(s) to keep",
    "Select up to 3 more card(s) from hand to keep",
    "Select up to 1 member(s) to change state",
    "Select up to 3 card(s) from the 5 looked-at cards (or skip)",
    "Select up to 2 more card(s) from the 4 remaining looked-at cards",
    "Repeat effect?",
    "Pay optional cost or skip"
};

const int rb_desc_choice_prompt_templates_en_count =
    (int)(sizeof(rb_desc_choice_prompt_templates_en) /
          sizeof(rb_desc_choice_prompt_templates_en[0]));

static int d_contains(const char *hay, const char *needle) {
    return hay && needle && strstr(hay, needle) != NULL;
}

/* Leading ASCII digits of `p`, as a NUL-terminated copy. */
static void d_leading_digits(const char *p, char *out, size_t out_sz) {
    size_t i = 0;
    while (p[i] >= '0' && p[i] <= '9' && i + 1 < out_sz) {
        out[i] = p[i];
        i++;
    }
    out[i] = 0;
}

/* Rust: translate_choice_prompt_en_to_ja(&str) -> Option<String>.
 * Returns a freshly allocated Japanese string, or NULL for Rust's None.
 * The four `strip_prefix` blocks fall through when their inner test fails, so
 * a later block can still match; that control flow is preserved.
 */
char *rb_desc_translate_choice_prompt_en_to_ja(const char *en) {
    size_t start = 0, end = 0, len;
    char *s = NULL;
    const char *rest;
    char n[32];
    RbDescBuf b;

    if (!en) return NULL;
    len = strlen(en);
    while (start < len && (en[start] == ' ' || en[start] == '\t' ||
                           en[start] == '\n' || en[start] == '\r' ||
                           en[start] == '\f' || en[start] == '\v'))
        start++;
    end = len;
    while (end > start && (en[end - 1] == ' ' || en[end - 1] == '\t' ||
                           en[end - 1] == '\n' || en[end - 1] == '\r' ||
                           en[end - 1] == '\f' || en[end - 1] == '\v'))
        end--;
    s = (char *)rb_malloc(end - start + 1);
    if (!s) return NULL;
    memcpy(s, en + start, end - start);
    s[end - start] = 0;
    dbuf_init(&b);

    if (!strncmp(s, "Select up to ", 13)) {
        rest = s + 13;
        d_leading_digits(rest, n, sizeof(n));
        rest += strlen(n);
        if (d_contains(rest, "to keep")) {
            dbuf_puts(&b, "最大"); dbuf_puts(&b, n);
            dbuf_puts(&b, "枚まで手札に残すカードを選択");
            goto out;
        }
        if (d_contains(rest, "member(s) to change state")) {
            dbuf_puts(&b, "状態を変更するメンバーを最大"); dbuf_puts(&b, n);
            dbuf_puts(&b, "体選択");
            goto out;
        }
        if (d_contains(rest, "looked-at cards")) {
            dbuf_puts(&b, "見たカードから最大"); dbuf_puts(&b, n);
            dbuf_puts(&b, "枚を選択（スキップ可）");
            goto out;
        }
    }
    if (!strncmp(s, "Place up to ", 12)) {
        rest = s + 12;
        d_leading_digits(rest, n, sizeof(n));
        if (d_contains(s, "bottom of deck")) {
            if (!strcmp(n, "1"))
                dbuf_puts(&b, "見たカードをデッキの下に置きますか？");
            else {
                dbuf_puts(&b, "見たカードを最大"); dbuf_puts(&b, n);
                dbuf_puts(&b, "枚までデッキの下に置きますか？");
            }
            goto out;
        }
        if (d_contains(s, "top of deck")) {
            if (!strcmp(n, "1"))
                dbuf_puts(&b, "見たカードをデッキの上に置きますか？");
            else {
                dbuf_puts(&b, "見たカードを最大"); dbuf_puts(&b, n);
                dbuf_puts(&b, "枚までデッキの上に置きますか？");
            }
            goto out;
        }
    }
    if (!strncmp(s, "Add up to ", 10)) {
        rest = s + 10;
        d_leading_digits(rest, n, sizeof(n));
        if (d_contains(s, "to hand")) {
            if (!strcmp(n, "1"))
                dbuf_puts(&b, "見たカードを手札に加えますか？");
            else {
                dbuf_puts(&b, "見たカードを最大"); dbuf_puts(&b, n);
                dbuf_puts(&b, "枚まで手札に加えますか？");
            }
            goto out;
        }
    }
    if (!strncmp(s, "Discard up to ", 14)) {
        rest = s + 14;
        d_leading_digits(rest, n, sizeof(n));
        if (d_contains(s, "looked-at")) {
            if (!strcmp(n, "1"))
                dbuf_puts(&b, "見たカードを控え室に置きますか？");
            else {
                dbuf_puts(&b, "見たカードを最大"); dbuf_puts(&b, n);
                dbuf_puts(&b, "枚まで控え室に置きますか？");
            }
            goto out;
        }
    }
    if (!strcmp(s, "Repeat effect?")) {
        dbuf_puts(&b, "効果を繰り返しますか？");
        goto out;
    }
    if (!strcmp(s, "Pay optional cost or skip")) {
        dbuf_puts(&b, "オプションコストを支払うかスキップ");
        goto out;
    }

out:
    rb_free(s);
    if (!b.s) return NULL;              /* Rust's None */
    return dbuf_take(&b);
}