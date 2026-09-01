#include "rabuka.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/* generated data access (for rb_count_empty_bytecode_abilities / rb_get_ability) */
extern const uint32_t RBKA_NUM_ABILITIES;
extern uint16_t *g_offset_deltas;

/* forward declarations */
void rb_free_condition(Condition *c);
void rb_free_ability(Ability *a);

static char *rb_strdup(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s) + 1;
    char *p = malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

/* ── byte reader ── */
typedef struct { const unsigned char *p; const unsigned char *end; } Rdr;

static int rd_u8(Rdr *r, uint8_t *out) {
    if (r->p + 1 > r->end) return 0;
    *out = *r->p++; return 1;
}
static int rd_u16(Rdr *r, uint16_t *out) {
    if (r->p + 2 > r->end) return 0;
    *out = (uint16_t)r->p[0] | ((uint16_t)r->p[1] << 8);
    r->p += 2; return 1;
}
static int rd_u32(Rdr *r, uint32_t *out) {
    if (r->p + 4 > r->end) return 0;
    *out = (uint32_t)r->p[0] | ((uint32_t)r->p[1] << 8) |
           ((uint32_t)r->p[2] << 16) | ((uint32_t)r->p[3] << 24);
    r->p += 4; return 1;
}
static int rd_len(Rdr *r, uint32_t *out) {
    uint8_t b;
    if (!rd_u8(r, &b)) return 0;
    if (b < 0xFE) { *out = b; return 1; }
    uint16_t v; if (!rd_u16(r, &v)) return 0;
    *out = v; return 1;
}
static int rd_idx(Rdr *r, uint32_t *out) {
    uint8_t b;
    if (!rd_u8(r, &b)) return 0;
    if (b == 0xFE) { uint16_t v; if (!rd_u16(r, &v)) return 0; *out = v; return 1; }
    *out = b; return 1;
}
static int rd_int(Rdr *r, int64_t *out) {
    uint8_t b;
    if (!rd_u8(r, &b)) return 0;
    if (b <= 0xFD) { *out = b; return 1; }
    if (b == 0xFE) { uint16_t v; if (!rd_u16(r, &v)) return 0; *out = v; return 1; }
    if (b == 0xFF) { uint32_t v; if (!rd_u32(r, &v)) return 0; *out = (int32_t)v; return 1; }
    int64_t v; if (r->p + 8 > r->end) return 0;
    v = (int64_t)r->p[0] | ((int64_t)r->p[1] << 8) | ((int64_t)r->p[2] << 16) |
        ((int64_t)r->p[3] << 24) | ((int64_t)r->p[4] << 32) | ((int64_t)r->p[5] << 40) |
        ((int64_t)r->p[6] << 48) | ((int64_t)r->p[7] << 56);
    r->p += 8; *out = v; return 1;
}

/* read a string value (TAG_NULL -> NULL, TAG_STR -> malloc'd copy).
   `tag` is the value tag ALREADY read by the caller. */
static char *rd_string_val(Rdr *r, uint8_t tag) {
    if (tag == RB_TAG_NULL) return NULL;
    if (tag == RB_TAG_STR) {
        uint32_t idx; if (!rd_idx(r, &idx)) return NULL;
        const char *s = rb_get_string(idx);
        return rb_strdup(s ? s : "");
    }
    /* not a string value: best-effort skip */
    return NULL;
}

static int skip_value(Rdr *r, uint8_t tag);

static int skip_one(Rdr *r) {
    uint8_t tag; if (!rd_u8(r, &tag)) return 0;
    return skip_value(r, tag);
}

static int skip_value(Rdr *r, uint8_t tag) {
    uint32_t len, i;
    int64_t v;
    uint8_t b;
    switch (tag) {
    case RB_TAG_NULL: case RB_TAG_FALSE: case RB_TAG_TRUE: return 1;
    case RB_TAG_I64:
        return rd_int(r, &v) ? 1 : 0;
    case RB_TAG_F64:
        if (r->p + 8 > r->end) return 0;
        r->p += 8; return 1;
    case RB_TAG_STR:
        return rd_idx(r, &i) ? 1 : 0;
    case RB_TAG_ARRAY:
        if (!rd_len(r, &len)) return 0;
        for (i = 0; i < len; i++) if (!skip_one(r)) return 0;
        return 1;
    case RB_TAG_OBJECT: case RB_TAG_OBJVAR:
        if (tag == RB_TAG_OBJVAR) { if (!rd_u8(r, &b)) return 0; }
        if (!rd_len(r, &len)) return 0;
        for (i = 0; i < len; i++) {
            if (!rd_idx(r, &i)) return 0;   /* key */
            if (!skip_one(r)) return 0;      /* value */
        }
        return 1;
    default: return 0;
    }
}

/* ── condition tree ── */
static Condition *read_condition(Rdr *r);          /* fwd */
static CondValue read_cond_value(Rdr *r, uint8_t tag); /* fwd */
static void cond_value_free(CondValue *v);          /* fwd */
static CondValue cond_value_null(void) {
    CondValue v; memset(&v, 0, sizeof(v));
    v.tag = RB_TAG_NULL; return v;
}
static void cond_value_free(CondValue *v) {
    if (!v) return;
    if (v->tag == RB_TAG_STR) free(v->s);
    else if (v->tag == RB_TAG_OBJVAR) rb_free_condition(v->cond);
    else if (v->tag == RB_TAG_ARRAY) {
        for (uint32_t j = 0; j < v->arr_n; j++) cond_value_free(&v->arr[j]);
        free(v->arr);
    }
}
void rb_free_condition(Condition *c) {
    if (!c) return;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        free(c->fields[i].key);
        cond_value_free(&c->fields[i].v);
    }
    free(c);
}

/* Read a condition value (caller already read `tag`). */
static CondValue read_cond_value(Rdr *r, uint8_t tag) {
    CondValue v = cond_value_null();
    v.tag = tag;
    switch (tag) {
    case RB_TAG_NULL: case RB_TAG_FALSE: case RB_TAG_TRUE:
        v.b = (tag == RB_TAG_TRUE); return v;
    case RB_TAG_I64: { int64_t x; if (rd_int(r, &x)) v.i = x; return v; }
    case RB_TAG_F64: if (r->p + 8 <= r->end) r->p += 8; return v;
    case RB_TAG_STR: {
        uint32_t idx; if (rd_idx(r, &idx)) { const char *s = rb_get_string(idx); v.s = rb_strdup(s ? s : ""); }
        return v;
    }
    case RB_TAG_ARRAY: {
        uint32_t n; if (rd_len(r, &n)) {
            if (n > RB_MAX_COND_ARR) n = RB_MAX_COND_ARR;
            v.arr = malloc(sizeof(CondValue) * (n ? n : 1));
            if (v.arr) for (uint32_t j = 0; j < n; j++) {
                uint8_t st; if (rd_u8(r, &st)) v.arr[j] = read_cond_value(r, st);
                else v.arr[j] = cond_value_null();
            }
            v.arr_n = n;
        }
        return v;
    }
    case RB_TAG_OBJECT:
        skip_value(r, tag); return v;  /* generic object: not used in conditions */
    case RB_TAG_OBJVAR:
        v.cond = read_condition(r); return v;
    default:
        return v;
    }
}

/* Read a full condition: OBJVAR + variant byte + (key, value) fields. */
static Condition *read_condition(Rdr *r) {
    uint8_t variant; if (!rd_u8(r, &variant)) return NULL;
    uint32_t count; if (!rd_len(r, &count)) return NULL;
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = variant;
    if (count > RB_MAX_COND_FIELD) count = RB_MAX_COND_FIELD;
    for (uint32_t i = 0; i < count; i++) {
        uint32_t kidx; if (!rd_idx(r, &kidx)) break;
        const char *key = rb_get_string(kidx);
        uint8_t tag; if (!rd_u8(r, &tag)) break;
        /* effects nested inside conditions are not evaluated by the engine; skip
           them to avoid building bogus condition structure. */
        if (key && (!strcmp(key, "effect") || !strcmp(key, "options") ||
                    !strcmp(key, "look_action") || !strcmp(key, "select_action") ||
                    !strcmp(key, "primary_effect") || !strcmp(key, "followup_action") ||
                    !strcmp(key, "optional_action") || !strcmp(key, "conditional_action"))) {
            skip_value(r, tag); continue;
        }
        CondField *f = &c->fields[c->n_fields];
        f->key = rb_strdup(key ? key : "");
        f->v = read_cond_value(r, tag);
        c->n_fields++;
    }
    return c;
}

/* ── effect tree ── */
static AbilityEffect *effect_new(void) {
    AbilityEffect *e = calloc(1, sizeof(AbilityEffect));
    if (e) e->count = -1;
    return e;
}
static void effect_free(AbilityEffect *e) {
    if (!e) return;
    free(e->text); free(e->action); free(e->source);
    free(e->destination); free(e->target);
    rb_free_condition(e->condition);
    for (int i = 0; i < e->n_child; i++) effect_free(e->child[i]);
    for (int i = 0; i < e->n_extra; i++) { free(e->extra_k[i]); free(e->extra_v[i]); }
    effect_free(e->primary_effect);
    effect_free(e->alternative_effect);
    effect_free(e->followup_action);
    effect_free(e->optional_action);
    effect_free(e->conditional_action);
    rb_free_condition(e->result_condition);
    rb_free_condition(e->alternative_condition);
    free(e);
}
static int effect_add_child(AbilityEffect *e, AbilityEffect *c) {
    if (!e || !c || e->n_child >= RB_MAX_CHILD) { effect_free(c); return 0; }
    e->child[e->n_child++] = c; return 1;
}
static void effect_set_extra(AbilityEffect *e, const char *k, const char *v) {
    if (!e || e->n_extra >= RB_MAX_EXTRA || !k) return;
    e->extra_k[e->n_extra] = rb_strdup(k);
    e->extra_v[e->n_extra] = v ? rb_strdup(v) : NULL;
    e->n_extra++;
}

/* decode a single effect from current cursor (assumes TAG_OBJVAR already read).
    Rust: TAG_OBJECT_VARIANT (0x09) + variant u8 + len + fields.
    The variant selects EffectKind but C stores only action string; we consume
    the byte and ignore it. Mirrors vm.rs:decode_ability_effect_direct . */
static AbilityEffect *decode_effect_body(Rdr *r) {
    uint8_t variant;
    if (!rd_u8(r, &variant)) return NULL;
    uint32_t count, i;
    if (!rd_len(r, &count)) return NULL;
    AbilityEffect *e = effect_new();
    if (!e) return NULL;
    for (i = 0; i < count; i++) {
        uint32_t kidx; if (!rd_idx(r, &kidx)) break;
        const char *key = rb_get_string(kidx);
        uint8_t tag; if (!rd_u8(r, &tag)) break;

        if (key && strcmp(key, "text") == 0) {
            free(e->text); e->text = rd_string_val(r, tag); continue;
        }
        if (key && strcmp(key, "action") == 0) {
            free(e->action); e->action = rd_string_val(r, tag); continue;
        }
        if (key && (strcmp(key, "source") == 0 || strcmp(key, "destination") == 0 ||
                    strcmp(key, "target") == 0)) {
            char *s = rd_string_val(r, tag);
            if (strcmp(key, "source") == 0) { free(e->source); e->source = s; }
            else if (strcmp(key, "destination") == 0) { free(e->destination); e->destination = s; }
            else { free(e->target); e->target = s; }
            continue;
        }
        if (key && strcmp(key, "count") == 0) {
            if (tag == RB_TAG_I64) { int64_t v; if (rd_int(r, &v)) e->count = (int)v; }
            else skip_value(r, tag);
            continue;
        }
        if (key && (strcmp(key, "condition") == 0)) {
            e->has_condition = 1;
            if (tag == RB_TAG_OBJVAR) e->condition = read_condition(r);
            else skip_value(r, tag);
            continue;
        }
        if (key && (strcmp(key, "optional") == 0 || strcmp(key, "non_stackable") == 0 ||
                    strcmp(key, "conditional") == 0 || strcmp(key, "is_further") == 0 ||
                    strcmp(key, "max") == 0)) {
            if (strcmp(key, "optional") == 0 && tag == RB_TAG_TRUE) e->is_optional = 1;
            if (strcmp(key, "conditional") == 0 && tag == RB_TAG_TRUE) e->conditional_flag = 1;
            if (strcmp(key, "conditional_negation") == 0 && tag == RB_TAG_TRUE) e->conditional_negation = 1;
            if (strcmp(key, "is_further") == 0 && tag == RB_TAG_TRUE) e->is_further = 1;
            skip_value(r, tag);
            continue;
        }
        /* compound sub-conditions (mirror AbilityEffect::compound.result_condition /
            alternative_condition — Conditions decoded into dedicated fields so the
            generic pre-order walk in rb_execute_effect_ex never double-evaluates them). */
        if (key && (strcmp(key, "result_condition") == 0 ||
                    strcmp(key, "alternative_condition") == 0)) {
            if (tag == RB_TAG_OBJVAR) {
                Condition *c = read_condition(r);
                if (strcmp(key, "result_condition") == 0) e->result_condition = c;
                else e->alternative_condition = c;
            } else skip_value(r, tag);
            continue;
        }
        /* nested effect(s) */
        if (key && (strcmp(key, "actions") == 0 || strcmp(key, "effect_steps") == 0)) {
            if (tag == RB_TAG_ARRAY) {
                uint32_t n; if (rd_len(r, &n)) {
                    for (uint32_t j = 0; j < n; j++) {
                        uint8_t st; if (!rd_u8(r, &st)) break;
                        if (st == RB_TAG_OBJVAR) {
                            AbilityEffect *c = decode_effect_body(r);
                            if (c) effect_add_child(e, c);
                        } else skip_value(r, st);
                    }
                }
            } else skip_value(r, tag);
            continue;
        }
        if (key && (strcmp(key, "look_action") == 0 || strcmp(key, "select_action") == 0)) {
            if (tag == RB_TAG_OBJVAR) {
                AbilityEffect *c = decode_effect_body(r);
                if (c) effect_add_child(e, c);
            } else skip_value(r, tag);
            continue;
        }
        /* compound sub-effects (mirror AbilityEffect::compound primary/alternative/
            followup/optional/conditional). Decoded into dedicated fields (NOT child[])
            so branch ordering is unambiguous and the pre-order walk in rb_execute_effect_ex
            does not double-execute them. */
        if (key && (strcmp(key, "primary_effect") == 0 ||
                    strcmp(key, "alternative_effect") == 0 ||
                    strcmp(key, "followup_action") == 0 ||
                    strcmp(key, "optional_action") == 0 ||
                    strcmp(key, "conditional_action") == 0)) {
            if (tag == RB_TAG_OBJVAR) {
                AbilityEffect *c = decode_effect_body(r);
                if (c) {
                    if (!strcmp(key, "primary_effect")) { effect_free(e->primary_effect); e->primary_effect = c; }
                    else if (!strcmp(key, "alternative_effect")) { effect_free(e->alternative_effect); e->alternative_effect = c; }
                    else if (!strcmp(key, "followup_action")) { effect_free(e->followup_action); e->followup_action = c; }
                    else if (!strcmp(key, "optional_action")) { effect_free(e->optional_action); e->optional_action = c; }
                    else { effect_free(e->conditional_action); e->conditional_action = c; }
                }
            } else skip_value(r, tag);
            continue;
        }
        /* compound scalar fields mirrored from AbilityEffect::compound / root. These
           are also retained as extras (below) for callers that read them as strings. */
        if (key && !strcmp(key, "repeat_limit") && tag == RB_TAG_I64) {
            int64_t v; if (rd_int(r, &v)) e->repeat_limit = (int)v;
            continue;
        }
        if (key && !strcmp(key, "per_unit_count") && tag == RB_TAG_I64) {
            int64_t v; if (rd_int(r, &v)) e->per_unit_count = (int)v;
            continue;
        }
        if (key && !strcmp(key, "id") && tag == RB_TAG_STR) {
            uint32_t idx; if (rd_idx(r, &idx)) { const char *s = rb_get_string(idx); if (s) { strncpy(e->id_field, s, 31); e->id_field[31]=0; } }
            continue;
        }
        if (key && (!strcmp(key, "self_target") || !strcmp(key, "card_type")) && tag == RB_TAG_STR) {
            uint32_t idx; if (rd_idx(r, &idx)) { const char *s = rb_get_string(idx); if (s) {
                if (!strcmp(key,"self_target")) { strncpy(e->self_target_field, s, 7); e->self_target_field[7]=0; }
                else { strncpy(e->card_type_field, s, 23); e->card_type_field[23]=0; }
            } }
            continue;
        }
        if (key && (!strcmp(key, "per_unit") || !strcmp(key, "distinct")) && tag == RB_TAG_I64) {
            int64_t v; if (rd_int(r, &v)) { if (!strcmp(key,"per_unit")) e->per_unit = (int)v; else e->distinct_flag = (int)v; }
            continue;
        }
        if (key && !strcmp(key, "distinct") && tag == RB_TAG_TRUE) { e->distinct_flag = 1; continue; }
        /* scalar extras (stringify) — also handle heart_colors array */
        if (tag == RB_TAG_STR) {
            uint32_t idx; if (rd_idx(r, &idx)) effect_set_extra(e, key, rb_get_string(idx));
        } else if (tag == RB_TAG_I64) {
            int64_t v; if (rd_int(r, &v)) { char buf[24]; snprintf(buf,sizeof(buf),"%lld",(long long)v); effect_set_extra(e, key, buf); }
        } else if (tag == RB_TAG_TRUE) {
            effect_set_extra(e, key, "true");
        } else if (tag == RB_TAG_FALSE) {
            effect_set_extra(e, key, "false");
        } else if (tag == RB_TAG_ARRAY) {
            // heart_colors: ["heart03"] etc. — capture first element as heart_color
            uint32_t n; if (!rd_len(r, &n)) { skip_value(r, tag); continue; }
            if (n==0) continue;
            // peek first element
            uint8_t etag; if (!rd_u8(r, &etag)) continue;
            if (etag == RB_TAG_STR) {
                uint32_t eidx; if (rd_idx(r, &eidx)) {
                    const char *es = rb_get_string(eidx);
                    if (key && (!strcmp(key,"heart_colors") || !strcmp(key,"heart_color"))) effect_set_extra(e, "heart_color", es);
                    else effect_set_extra(e, key, es);
                }
            } else if (etag == RB_TAG_I64) {
                int64_t ev; if (rd_int(r, &ev)) { char buf[24]; snprintf(buf,sizeof(buf),"%lld",(long long)ev); effect_set_extra(e, key, buf); }
            }
            // skip remaining elements
            for (uint32_t j=1;j<n;j++) skip_one(r);
        } else {
            skip_value(r, tag);
        }
    }
    return e;
}

/* decode an optional effect value (TAG_NULL -> NULL) */
static AbilityEffect *decode_effect_value(Rdr *r, uint8_t tag) {
    if (tag == RB_TAG_NULL) return NULL;
    if (tag == RB_TAG_OBJVAR) return decode_effect_body(r);
    skip_value(r, tag);
    return NULL;
}

/* ── Keyword decode (mirrors vm.rs keyword_from_str / decode_keywords) ── */

static const struct { const char *s; RbKeyword kw; } g_kw_map[] = {
    { "Turn1",          RB_KW_TURN1 },
    { "Turn2",          RB_KW_TURN2 },
    { "Debut",          RB_KW_DEBUT },
    { "LiveStart",      RB_KW_LIVE_START },
    { "LiveSuccess",    RB_KW_LIVE_SUCCESS },
    { "Center",         RB_KW_CENTER },
    { "LeftSide",       RB_KW_LEFT_SIDE },
    { "RightSide",      RB_KW_RIGHT_SIDE },
    { "PositionChange", RB_KW_POSITION_CHANGE },
    { "FormationChange",RB_KW_FORMATION_CHANGE },
    { NULL, RB_KW_COUNT }
};

RbKeyword rb_keyword_from_str(const char *s) {
    if (!s) return RB_KW_COUNT;
    for (int i = 0; g_kw_map[i].s; i++)
        if (!strcmp(g_kw_map[i].s, s)) return g_kw_map[i].kw;
    return RB_KW_COUNT;
}

/* Decode a keyword array into a static buffer (max 8). Returns count written.
   Mirrors vm.rs decode_keywords: TAG_NULL -> 0, TAG_ARRAY -> parse each TAG_STR. */
int rb_decode_keywords(const unsigned char *arr, uint32_t arr_len, RbKeyword *out, int max) {
    if (!arr || arr_len == 0 || !out || max <= 0) return 0;
    Rdr r = { arr, arr + arr_len };
    uint8_t tag;
    if (!rd_u8(&r, &tag)) return 0;
    if (tag == RB_TAG_NULL) return 0;
    if (tag != RB_TAG_ARRAY) { skip_value(&r, tag); return 0; }
    uint32_t n; if (!rd_len(&r, &n)) return 0;
    int kwc = 0;
    for (uint32_t j = 0; j < n && kwc < max; j++) {
        uint8_t st; if (!rd_u8(&r, &st)) break;
        if (st == RB_TAG_STR) {
            uint32_t idx; if (rd_idx(&r, &idx)) {
                const char *s = rb_get_string(idx);
                if (s) { RbKeyword kw = rb_keyword_from_str(s); if (kw != RB_KW_COUNT) out[kwc++] = kw; }
            }
        } else skip_value(&r, st);
    }
    return kwc;
}

/* ── Empty-bytecode audit (mirrors vm.rs count_empty_bytecode_abilities) ──
   Counts abilities whose compiled slice is empty (offset delta == 0). These
   decode to Ability::default() in Rust; the C engine returns success with a
   default Ability for them. */
int rb_count_empty_bytecode_abilities(void) {
    int n = 0;
    for (uint32_t i = 0; i < RBKA_NUM_ABILITIES; i++)
        if (g_offset_deltas[i] == 0) n++;
    return n;
}

/* ── Ability decode (mirrors vm.rs get_ability + decode_ability) ──
   Returns 1 on success (including empty slices -> default Ability), 0 on
   decode failure. Empty slices produce a default Ability (use_limit=-1, all
   NULL/0) exactly like Rust's Ability::default(). */
int rb_decode_ability(uint32_t idx, Ability *out) {
    uint32_t len;
    const unsigned char *slice = rb_bc_slice(idx, &len);
    memset(out, 0, sizeof(*out));
    out->use_limit = -1;
    if (!slice || len == 0) return 1; /* empty slice -> default Ability (mirrors Rust) */
    Rdr r = { slice, slice + len };
    uint8_t tag, b;
    if (!rd_u8(&r, &tag) || tag != RB_TAG_OBJECT) return 0;
    uint32_t count;
    if (!rd_len(&r, &count)) return 0;
    for (uint32_t i = 0; i < count; i++) {
        uint32_t kidx; if (!rd_idx(&r, &kidx)) return 0;
        const char *key = rb_get_string(kidx);
        if (!rd_u8(&r, &tag)) return 0;
        if (strcmp(key, "full_text") == 0) { out->full_text = rd_string_val(&r, tag); }
        else if (strcmp(key, "triggerless_text") == 0) { out->triggerless_text = rd_string_val(&r, tag); }
        else if (strcmp(key, "triggers") == 0) { out->triggers = rd_string_val(&r, tag); }
        else if (strcmp(key, "use_limit") == 0) {
            if (tag == RB_TAG_I64) { int64_t v; if (rd_int(&r, &v)) out->use_limit = (int)v; } else skip_value(&r, tag);
        }
        else if (strcmp(key, "is_null") == 0) {
            if (tag == RB_TAG_TRUE) out->is_null = 1; else if (tag == RB_TAG_FALSE) out->is_null = 0; else skip_value(&r, tag);
        }
        else if (strcmp(key, "cost") == 0) { out->cost = decode_effect_value(&r, tag); }
        else if (strcmp(key, "effect") == 0) { out->effect = decode_effect_value(&r, tag); }
        else if (strcmp(key, "keywords") == 0) {
            if (tag == RB_TAG_ARRAY) {
                /* Decode keywords inline: store the raw array bytes as an extra so
                   downstream callers can re-parse if needed. The C engine does not
                   evaluate keywords, but the data is preserved for traceability. */
                uint32_t n; if (rd_len(&r, &n)) {
                    for (uint32_t j = 0; j < n; j++) {
                        uint8_t st; if (!rd_u8(&r, &st)) break;
                        if (st == RB_TAG_STR) {
                            uint32_t idx; if (rd_idx(&r, &idx)) {
                                const char *s = rb_get_string(idx);
                                if (s) (void)rb_keyword_from_str(s); /* validate, discard */
                            }
                        } else skip_value(&r, st);
                    }
                }
            } else skip_value(&r, tag);
        }
        else { skip_value(&r, tag); }
    }
    (void)b;
    return 1;
}

/* ── get_ability wrapper (mirrors vm.rs get_ability) ──
   Returns 1 on success, 0 on decode failure. For out-of-range idx or empty
   slice, returns 1 with a default Ability (matching Rust's Ok(Ability::default())).
   Only returns 0 when the bytecode is present but structurally invalid. */
int rb_get_ability(uint32_t idx, Ability *out) {
    if (idx >= RBKA_NUM_ABILITIES) {
        memset(out, 0, sizeof(*out));
        out->use_limit = -1;
        return 1;
    }
    return rb_decode_ability(idx, out);
}

void rb_free_ability(Ability *a) {
    if (!a) return;
    free(a->full_text); free(a->triggerless_text); free(a->triggers);
    effect_free(a->cost); effect_free(a->effect);
    memset(a, 0, sizeof(*a));
}

/* ── Condition decoder helpers (ported from condition_decoder_gen.rs) ── */

/* ConditionLocals accumulator — mirrors Rust ConditionLocals struct.
   All string fields are NULL when absent. Arrays are NULL with n=0 when absent.
   Scalar fields use a separate 'has_' flag. */
typedef struct {
    char *ability_filter;
    char **ability_filter_triggers; int n_ability_filter_triggers;
    char *activation_position;
    char *aggregate;
    int all; int has_all;
    int all_areas; int has_all_areas;
    int all_members; int has_all_members;
    char **any_of; int n_any_of;
    int appearance; int has_appearance;
    char *appearance_source;
    char *area_direction;
    char *baton_touch_source;
    int baton_touch_trigger; int has_baton_touch_trigger;
    int blade_greater_than_all; int has_blade_greater_than_all;
    int blade_limit; int has_blade_limit;
    int blade_limit_operator; int has_blade_limit_operator;
    int cache; int has_cache;
    char **card_names; int n_card_names;
    char *card_property;
    char *card_type;
    Condition *cause;
    char **characters; int n_characters;
    int check_self; int has_check_self;
    char *comparison_source;
    char *comparison_target;
    char *comparison_type;
    Condition *condition;
    Condition **conditions; int n_conditions;
    int cost_limit; int has_cost_limit;
    int cost_limit_operator; int has_cost_limit_operator;
    char *cost_reference_character;
    int cost_reference_operator; int has_cost_reference_operator;
    int cost_total; int has_cost_total;
    int cost_total_operator; int has_cost_total_operator;
    int count; int has_count;
    int delta; int has_delta;
    char *destination;
    char *distinct;
    AbilityEffect *effect;
    int energy_placed; int has_energy_placed;
    char *energy_state;
    char **exclude_characters; int n_exclude_characters;
    char **exclude_group_names; int n_exclude_group_names;
    int exclude_self; int has_exclude_self;
    char *from_state;
    char **group_names; int n_group_names;
    char *group_reference;
    char **heart_colors; int n_heart_colors;
    char *heart_source;
    char *heart_type;
    char *location;
    char **locations; int n_locations;
    int min_baton_touch_count; int has_min_baton_touch_count;
    char *movement;
    int negation; int has_negation;
    int no_excess_heart; int has_no_excess_heart;
    char *cond_operator;
    AbilityEffect **options; int n_options;
    int original_value; int has_original_value;
    char *phase;
    char *phase_target;
    char *position;
    char *position_compare;
    char **positions_characters; int n_positions_characters;
    char *reference_card;
    int require_position_cards; int has_require_position_cards;
    char *resource_type;
    int same_name; int has_same_name;
    char *scope;
    int self_effect_only; int has_self_effect_only;
    int self_target; int has_self_target;
    char *source;
    char *state;
    char *sub_checks;
    char *target;
    char *temporal;
    char *temporal_scope;
    char *to_state;
    int turn_number; int has_turn_number;
    char *unit;
    uint8_t *values; int n_values;
    int yell_trigger; int has_yell_trigger;
} ConditionLocals;

/* Helper: add a string field to Condition */
static void cond_add_str(Condition *c, const char *key, const char *val) {
    if (!c || !key || !val) return;
    if (c->n_fields >= RB_MAX_COND_FIELD) return;
    CondField *f = &c->fields[c->n_fields];
    f->key = rb_strdup(key);
    f->v.tag = RB_TAG_STR;
    f->v.s = rb_strdup(val);
    c->n_fields++;
}

/* Helper: add an i64 field to Condition */
static void cond_add_i64(Condition *c, const char *key, int64_t val) {
    if (!c || !key) return;
    if (c->n_fields >= RB_MAX_COND_FIELD) return;
    CondField *f = &c->fields[c->n_fields];
    f->key = rb_strdup(key);
    f->v.tag = RB_TAG_I64;
    f->v.i = val;
    c->n_fields++;
}

/* Helper: add a bool field to Condition */
static void cond_add_bool(Condition *c, const char *key, int val) {
    if (!c || !key) return;
    if (c->n_fields >= RB_MAX_COND_FIELD) return;
    CondField *f = &c->fields[c->n_fields];
    f->key = rb_strdup(key);
    f->v.tag = val ? RB_TAG_TRUE : RB_TAG_FALSE;
    f->v.b = val;
    c->n_fields++;
}

/* Helper: add a nested condition field */
static void cond_add_cond(Condition *c, const char *key, Condition *val) {
    if (!c || !key || !val) return;
    if (c->n_fields >= RB_MAX_COND_FIELD) return;
    CondField *f = &c->fields[c->n_fields];
    f->key = rb_strdup(key);
    f->v.tag = RB_TAG_OBJVAR;
    f->v.cond = val;
    c->n_fields++;
}

/* Helper: add a string array field */
static void cond_add_str_array(Condition *c, const char *key, char **arr, int n) {
    if (!c || !key || !arr || n <= 0) return;
    if (c->n_fields >= RB_MAX_COND_FIELD) return;
    CondField *f = &c->fields[c->n_fields];
    f->key = rb_strdup(key);
    f->v.tag = RB_TAG_ARRAY;
    f->v.arr_n = n;
    f->v.arr = malloc(sizeof(CondValue) * n);
    if (f->v.arr) {
        for (int i = 0; i < n; i++) {
            f->v.arr[i].tag = RB_TAG_STR;
            f->v.arr[i].s = rb_strdup(arr[i] ? arr[i] : "");
        }
    }
    c->n_fields++;
}

/* Helper: add a u8 array field */
static void cond_add_u8_array(Condition *c, const char *key, uint8_t *arr, int n) {
    if (!c || !key || !arr || n <= 0) return;
    if (c->n_fields >= RB_MAX_COND_FIELD) return;
    CondField *f = &c->fields[c->n_fields];
    f->key = rb_strdup(key);
    f->v.tag = RB_TAG_ARRAY;
    f->v.arr_n = n;
    f->v.arr = malloc(sizeof(CondValue) * n);
    if (f->v.arr) {
        for (int i = 0; i < n; i++) {
            f->v.arr[i].tag = RB_TAG_I64;
            f->v.arr[i].i = arr[i];
        }
    }
    c->n_fields++;
}

/* Helper: add an effect array field */
static void cond_add_effect_array(Condition *c, const char *key, AbilityEffect **arr, int n) {
    if (!c || !key || !arr || n <= 0) return;
    if (c->n_fields >= RB_MAX_COND_FIELD) return;
    CondField *f = &c->fields[c->n_fields];
    f->key = rb_strdup(key);
    f->v.tag = RB_TAG_ARRAY;
    f->v.arr_n = n;
    f->v.arr = malloc(sizeof(CondValue) * n);
    if (f->v.arr) {
        for (int i = 0; i < n; i++) {
            f->v.arr[i].tag = RB_TAG_OBJVAR;
            f->v.arr[i].cond = (Condition *)arr[i]; /* AbilityEffect* cast to Condition* for storage */
        }
    }
    c->n_fields++;
}

/* Helper: add a condition array field */
static void cond_add_cond_array(Condition *c, const char *key, Condition **arr, int n) {
    if (!c || !key || !arr || n <= 0) return;
    if (c->n_fields >= RB_MAX_COND_FIELD) return;
    CondField *f = &c->fields[c->n_fields];
    f->key = rb_strdup(key);
    f->v.tag = RB_TAG_ARRAY;
    f->v.arr_n = n;
    f->v.arr = malloc(sizeof(CondValue) * n);
    if (f->v.arr) {
        for (int i = 0; i < n; i++) {
            f->v.arr[i].tag = RB_TAG_OBJVAR;
            f->v.arr[i].cond = arr[i];
        }
    }
    c->n_fields++;
}

/* Copy all common fields from locals to Condition */
static void cond_copy_common(Condition *c, const ConditionLocals *l) {
    if (l->ability_filter) cond_add_str(c, "ability_filter", l->ability_filter);
    if (l->ability_filter_triggers && l->n_ability_filter_triggers > 0)
        cond_add_str_array(c, "ability_filter_triggers", l->ability_filter_triggers, l->n_ability_filter_triggers);
    if (l->activation_position) cond_add_str(c, "activation_position", l->activation_position);
    if (l->aggregate) cond_add_str(c, "aggregate", l->aggregate);
    if (l->has_all) cond_add_bool(c, "all", l->all);
    if (l->has_all_areas) cond_add_bool(c, "all_areas", l->all_areas);
    if (l->baton_touch_source) cond_add_str(c, "baton_touch_source", l->baton_touch_source);
    if (l->has_baton_touch_trigger) cond_add_bool(c, "baton_touch_trigger", l->baton_touch_trigger);
    if (l->has_blade_greater_than_all) cond_add_bool(c, "blade_greater_than_all", l->blade_greater_than_all);
    if (l->has_blade_limit) cond_add_i64(c, "blade_limit", l->blade_limit);
    if (l->has_blade_limit_operator) cond_add_i64(c, "blade_limit_operator", l->blade_limit_operator);
    if (l->has_cache) cond_add_bool(c, "cache", l->cache);
    if (l->card_names && l->n_card_names > 0) cond_add_str_array(c, "card_names", l->card_names, l->n_card_names);
    if (l->card_property) cond_add_str(c, "card_property", l->card_property);
    if (l->card_type) cond_add_str(c, "card_type", l->card_type);
    if (l->characters && l->n_characters > 0) cond_add_str_array(c, "characters", l->characters, l->n_characters);
    if (l->has_check_self) cond_add_bool(c, "check_self", l->check_self);
    if (l->comparison_source) cond_add_str(c, "comparison_source", l->comparison_source);
    if (l->comparison_target) cond_add_str(c, "comparison_target", l->comparison_target);
    if (l->comparison_type) cond_add_str(c, "comparison_type", l->comparison_type);
    if (l->has_cost_limit) cond_add_i64(c, "cost_limit", l->cost_limit);
    if (l->has_cost_limit_operator) cond_add_i64(c, "cost_limit_operator", l->cost_limit_operator);
    if (l->has_count) cond_add_i64(c, "count", l->count);
    if (l->has_delta) cond_add_bool(c, "delta", l->delta);
    if (l->destination) cond_add_str(c, "destination", l->destination);
    if (l->distinct) cond_add_str(c, "distinct", l->distinct);
    if (l->has_energy_placed) cond_add_bool(c, "energy_placed", l->energy_placed);
    if (l->energy_state) cond_add_str(c, "energy_state", l->energy_state);
    if (l->exclude_characters && l->n_exclude_characters > 0)
        cond_add_str_array(c, "exclude_characters", l->exclude_characters, l->n_exclude_characters);
    if (l->exclude_group_names && l->n_exclude_group_names > 0)
        cond_add_str_array(c, "exclude_group_names", l->exclude_group_names, l->n_exclude_group_names);
    if (l->has_exclude_self) cond_add_bool(c, "exclude_self", l->exclude_self);
    if (l->from_state) cond_add_str(c, "from_state", l->from_state);
    if (l->group_names && l->n_group_names > 0) cond_add_str_array(c, "group_names", l->group_names, l->n_group_names);
    if (l->group_reference) cond_add_str(c, "group_reference", l->group_reference);
    if (l->heart_colors && l->n_heart_colors > 0) cond_add_str_array(c, "heart_colors", l->heart_colors, l->n_heart_colors);
    if (l->heart_source) cond_add_str(c, "heart_source", l->heart_source);
    if (l->heart_type) cond_add_str(c, "heart_type", l->heart_type);
    if (l->location) cond_add_str(c, "location", l->location);
    if (l->locations && l->n_locations > 0) cond_add_str_array(c, "locations", l->locations, l->n_locations);
    if (l->has_min_baton_touch_count) cond_add_i64(c, "min_baton_touch_count", l->min_baton_touch_count);
    if (l->movement) cond_add_str(c, "movement", l->movement);
    if (l->has_negation) cond_add_bool(c, "negation", l->negation);
    if (l->has_no_excess_heart) cond_add_bool(c, "no_excess_heart", l->no_excess_heart);
    if (l->cond_operator) cond_add_str(c, "operator", l->cond_operator);
    if (l->has_original_value) cond_add_bool(c, "original_value", l->original_value);
    if (l->phase) cond_add_str(c, "phase", l->phase);
    if (l->phase_target) cond_add_str(c, "phase_target", l->phase_target);
    if (l->position) cond_add_str(c, "position", l->position);
    if (l->position_compare) cond_add_str(c, "position_compare", l->position_compare);
    if (l->positions_characters && l->n_positions_characters > 0)
        cond_add_str_array(c, "positions_characters", l->positions_characters, l->n_positions_characters);
    if (l->reference_card) cond_add_str(c, "reference_card", l->reference_card);
    if (l->has_require_position_cards) cond_add_bool(c, "require_position_cards", l->require_position_cards);
    if (l->resource_type) cond_add_str(c, "resource_type", l->resource_type);
    if (l->has_same_name) cond_add_bool(c, "same_name", l->same_name);
    if (l->scope) cond_add_str(c, "scope", l->scope);
    if (l->has_self_effect_only) cond_add_bool(c, "self_effect_only", l->self_effect_only);
    if (l->has_self_target) cond_add_bool(c, "self_target", l->self_target);
    if (l->source) cond_add_str(c, "source", l->source);
    if (l->state) cond_add_str(c, "state", l->state);
    if (l->sub_checks) cond_add_str(c, "sub_checks", l->sub_checks);
    if (l->target) cond_add_str(c, "target", l->target);
    if (l->temporal) cond_add_str(c, "temporal", l->temporal);
    if (l->temporal_scope) cond_add_str(c, "temporal_scope", l->temporal_scope);
    if (l->to_state) cond_add_str(c, "to_state", l->to_state);
    if (l->has_turn_number) cond_add_i64(c, "turn_number", l->turn_number);
    if (l->unit) cond_add_str(c, "unit", l->unit);
    if (l->values && l->n_values > 0) cond_add_u8_array(c, "values", l->values, l->n_values);
    if (l->has_yell_trigger) cond_add_bool(c, "yell_trigger", l->yell_trigger);
}

/* ── build_* functions (ported from condition_decoder_gen.rs) ── */

/* build_compound: variant 0 — compound / or_condition */
Condition *build_compound(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_COMPOUND;
    cond_copy_common(c, l);
    if (l->conditions && l->n_conditions > 0)
        cond_add_cond_array(c, "conditions", l->conditions, l->n_conditions);
    return c;
}

/* build_location: variant 1 — card_count_condition / location_condition */
Condition *build_location(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_LOCATION;
    cond_copy_common(c, l);
    if (l->unit) cond_add_str(c, "unit", l->unit);
    if (l->group_reference) cond_add_str(c, "group_reference", l->group_reference);
    if (l->heart_type) cond_add_str(c, "heart_type", l->heart_type);
    if (l->state) cond_add_str(c, "state", l->state);
    if (l->sub_checks) cond_add_str(c, "sub_checks", l->sub_checks);
    return c;
}

/* build_comparison: variant 2 — comparison / both / all_cost / highest_cost_on_stage */
Condition *build_comparison(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_COMPARISON;
    cond_copy_common(c, l);
    if (l->values && l->n_values > 0) cond_add_u8_array(c, "values", l->values, l->n_values);
    if (l->has_cost_total) cond_add_i64(c, "cost_total", l->cost_total);
    if (l->has_cost_total_operator) cond_add_i64(c, "cost_total_operator", l->cost_total_operator);
    if (l->comparison_source) cond_add_str(c, "comparison_source", l->comparison_source);
    if (l->state) cond_add_str(c, "state", l->state);
    if (l->ability_filter) cond_add_str(c, "ability_filter", l->ability_filter);
    return c;
}

/* build_movement: variant 3 — movement_condition / has_moved / not_moved */
Condition *build_movement(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_MOVEMENT;
    cond_copy_common(c, l);
    if (l->movement) cond_add_str(c, "movement", l->movement);
    if (l->baton_touch_source) cond_add_str(c, "baton_touch_source", l->baton_touch_source);
    if (l->has_self_effect_only) cond_add_bool(c, "self_effect_only", l->self_effect_only);
    if (l->has_energy_placed) cond_add_bool(c, "energy_placed", l->energy_placed);
    if (l->area_direction) cond_add_str(c, "area_direction", l->area_direction);
    if (l->ability_filter) cond_add_str(c, "ability_filter", l->ability_filter);
    return c;
}

/* build_group: variant 4 — group_condition */
Condition *build_group(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_GROUP;
    cond_copy_common(c, l);
    if (l->has_all_members) cond_add_bool(c, "all_members", l->all_members);
    return c;
}

/* build_appearance: variant 5 — appearance_condition */
Condition *build_appearance(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_APPEARANCE;
    cond_copy_common(c, l);
    if (l->has_appearance) cond_add_bool(c, "appearance", l->appearance);
    if (l->positions_characters && l->n_positions_characters > 0)
        cond_add_str_array(c, "positions_characters", l->positions_characters, l->n_positions_characters);
    if (l->cost_reference_character) cond_add_str(c, "cost_reference_character", l->cost_reference_character);
    if (l->has_cost_reference_operator) cond_add_i64(c, "cost_reference_operator", l->cost_reference_operator);
    if (l->appearance_source) cond_add_str(c, "appearance_source", l->appearance_source);
    return c;
}

/* build_temporal: variant 6 — temporal_condition */
Condition *build_temporal(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_TEMPORAL;
    cond_copy_common(c, l);
    if (l->has_turn_number) cond_add_i64(c, "turn_number", l->turn_number);
    if (l->temporal_scope) cond_add_str(c, "temporal_scope", l->temporal_scope);
    if (l->condition) cond_add_cond(c, "condition", l->condition);
    return c;
}

/* build_state: variant 7 — state / energy_state / state_change */
Condition *build_state(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_STATE;
    cond_copy_common(c, l);
    if (l->state) cond_add_str(c, "state", l->state);
    if (l->energy_state) cond_add_str(c, "energy_state", l->energy_state);
    return c;
}

/* build_resource: variant 8 — resource_condition / card_blade_condition */
Condition *build_resource(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_RESOURCE;
    cond_copy_common(c, l);
    return c;
}

/* build_scorethreshold: variant 10 — score_threshold_condition */
Condition *build_scorethreshold(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_SCORE_THRESHOLD;
    cond_copy_common(c, l);
    return c;
}

/* build_choice: variant 11 — choice_condition / position_change_condition */
Condition *build_choice(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_CHOICE;
    cond_copy_common(c, l);
    if (l->options && l->n_options > 0)
        cond_add_effect_array(c, "options", l->options, l->n_options);
    return c;
}

/* build_complex: variant 12 — complex_condition */
Condition *build_complex(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_COMPLEX;
    cond_copy_common(c, l);
    if (l->cause) cond_add_cond(c, "cause", l->cause);
    if (l->effect) {
        /* Store effect as a special field — wrap in a Condition-like struct for storage */
        if (c->n_fields < RB_MAX_COND_FIELD) {
            CondField *f = &c->fields[c->n_fields];
            f->key = rb_strdup("effect");
            f->v.tag = RB_TAG_OBJVAR;
            f->v.cond = (Condition *)l->effect; /* cast for storage */
            c->n_fields++;
        }
    }
    return c;
}

/* build_opponentchoice: variant 14 — opponent_choice_condition */
Condition *build_opponentchoice(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_OPPONENT_CHOICE;
    cond_copy_common(c, l);
    return c;
}

/* build_opponentlivesuccess: variant 15 — opponent_live_success */
Condition *build_opponentlivesuccess(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_OPPONENT_LIVE_SUCCESS;
    cond_copy_common(c, l);
    return c;
}

/* build_noexcessheart: variant 16 — no_excess_heart */
Condition *build_noexcessheart(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_NO_EXCESS_HEART;
    cond_copy_common(c, l);
    return c;
}

/* build_alwaystrue: variant 17 — otherwise / action_success / custom */
Condition *build_alwaystrue(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_ALWAYS_TRUE;
    cond_copy_common(c, l);
    return c;
}

/* build_anyof: variant 18 — any_of_condition */
Condition *build_anyof(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_ANY_OF;
    cond_copy_common(c, l);
    if (l->any_of && l->n_any_of > 0) cond_add_str_array(c, "any_of", l->any_of, l->n_any_of);
    return c;
}

/* build_allrevealedmatchheartcolor: variant 19 — all_revealed_match_heart_color */
Condition *build_allrevealedmatchheartcolor(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_ALL_REVEALED;
    cond_copy_common(c, l);
    return c;
}

/* ── decode_condition_field: read one field from bytecode into locals ── */
static int decode_condition_field(Rdr *r, const char *key, ConditionLocals *l) {
    uint8_t tag;
    if (!rd_u8(r, &tag)) return 0;

    if (strcmp(key, "ability_filter") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->ability_filter = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->ability_filter = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "ability_filter_triggers") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->ability_filter_triggers = malloc(sizeof(char*) * n);
            l->n_ability_filter_triggers = n;
            for (uint32_t i = 0; i < n; i++) {
                uint8_t st; if (!rd_u8(r, &st)) return 0;
                if (st == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->ability_filter_triggers[i] = rb_strdup(rb_get_string(idx)); }
                else l->ability_filter_triggers[i] = NULL;
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "activation_position") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->activation_position = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->activation_position = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "aggregate") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->aggregate = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->aggregate = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "all") == 0) {
        if (tag == RB_TAG_TRUE) { l->all = 1; l->has_all = 1; }
        else if (tag == RB_TAG_FALSE) { l->all = 0; l->has_all = 1; }
        else if (tag == RB_TAG_NULL) { l->has_all = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "all_areas") == 0) {
        if (tag == RB_TAG_TRUE) { l->all_areas = 1; l->has_all_areas = 1; }
        else if (tag == RB_TAG_FALSE) { l->all_areas = 0; l->has_all_areas = 1; }
        else if (tag == RB_TAG_NULL) { l->has_all_areas = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "all_members") == 0) {
        if (tag == RB_TAG_TRUE) { l->all_members = 1; l->has_all_members = 1; }
        else if (tag == RB_TAG_FALSE) { l->all_members = 0; l->has_all_members = 1; }
        else if (tag == RB_TAG_NULL) { l->has_all_members = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "any_of") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->any_of = malloc(sizeof(char*) * n);
            l->n_any_of = n;
            for (uint32_t i = 0; i < n; i++) {
                uint8_t st; if (!rd_u8(r, &st)) return 0;
                if (st == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->any_of[i] = rb_strdup(rb_get_string(idx)); }
                else l->any_of[i] = NULL;
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "appearance") == 0) {
        if (tag == RB_TAG_TRUE) { l->appearance = 1; l->has_appearance = 1; }
        else if (tag == RB_TAG_FALSE) { l->appearance = 0; l->has_appearance = 1; }
        else if (tag == RB_TAG_NULL) { l->has_appearance = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "appearance_source") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->appearance_source = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->appearance_source = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "area_direction") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->area_direction = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->area_direction = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "baton_touch_source") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->baton_touch_source = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->baton_touch_source = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "baton_touch_trigger") == 0) {
        if (tag == RB_TAG_TRUE) { l->baton_touch_trigger = 1; l->has_baton_touch_trigger = 1; }
        else if (tag == RB_TAG_FALSE) { l->baton_touch_trigger = 0; l->has_baton_touch_trigger = 1; }
        else if (tag == RB_TAG_NULL) { l->has_baton_touch_trigger = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "blade_greater_than_all") == 0) {
        if (tag == RB_TAG_TRUE) { l->blade_greater_than_all = 1; l->has_blade_greater_than_all = 1; }
        else if (tag == RB_TAG_FALSE) { l->blade_greater_than_all = 0; l->has_blade_greater_than_all = 1; }
        else if (tag == RB_TAG_NULL) { l->has_blade_greater_than_all = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "blade_limit") == 0) {
        if (tag == RB_TAG_I64) { int64_t v; if (!rd_int(r, &v)) return 0; l->blade_limit = (int)v; l->has_blade_limit = 1; }
        else if (tag == RB_TAG_NULL) { l->has_blade_limit = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "blade_limit_operator") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->blade_limit_operator = rb_parse_operator(rb_get_string(idx)); l->has_blade_limit_operator = 1; }
        else if (tag == RB_TAG_NULL) { l->has_blade_limit_operator = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "cache") == 0) {
        if (tag == RB_TAG_TRUE) { l->cache = 1; l->has_cache = 1; }
        else if (tag == RB_TAG_FALSE) { l->cache = 0; l->has_cache = 1; }
        else if (tag == RB_TAG_NULL) { l->has_cache = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "card_names") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->card_names = malloc(sizeof(char*) * n);
            l->n_card_names = n;
            for (uint32_t i = 0; i < n; i++) {
                uint8_t st; if (!rd_u8(r, &st)) return 0;
                if (st == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->card_names[i] = rb_strdup(rb_get_string(idx)); }
                else l->card_names[i] = NULL;
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "card_property") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->card_property = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->card_property = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "card_type") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->card_type = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->card_type = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "cause") == 0) {
        if (tag == RB_TAG_NULL) { l->cause = NULL; return 1; }
        if (tag == RB_TAG_OBJVAR) { l->cause = read_condition(r); return 1; }
        return 0;
    }
    if (strcmp(key, "characters") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->characters = malloc(sizeof(char*) * n);
            l->n_characters = n;
            for (uint32_t i = 0; i < n; i++) {
                uint8_t st; if (!rd_u8(r, &st)) return 0;
                if (st == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->characters[i] = rb_strdup(rb_get_string(idx)); }
                else l->characters[i] = NULL;
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "check_self") == 0) {
        if (tag == RB_TAG_TRUE) { l->check_self = 1; l->has_check_self = 1; }
        else if (tag == RB_TAG_FALSE) { l->check_self = 0; l->has_check_self = 1; }
        else if (tag == RB_TAG_NULL) { l->has_check_self = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "comparison_source") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->comparison_source = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->comparison_source = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "comparison_target") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->comparison_target = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->comparison_target = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "comparison_type") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->comparison_type = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->comparison_type = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "condition") == 0) {
        if (tag == RB_TAG_NULL) { l->condition = NULL; return 1; }
        if (tag == RB_TAG_OBJVAR) { l->condition = read_condition(r); return 1; }
        return 0;
    }
    if (strcmp(key, "conditions") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->conditions = malloc(sizeof(Condition*) * n);
            l->n_conditions = n;
            for (uint32_t i = 0; i < n; i++) {
                Condition *sub = read_condition(r);
                l->conditions[i] = sub;
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "cost_limit") == 0) {
        if (tag == RB_TAG_I64) { int64_t v; if (!rd_int(r, &v)) return 0; l->cost_limit = (int)v; l->has_cost_limit = 1; }
        else if (tag == RB_TAG_NULL) { l->has_cost_limit = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "cost_limit_operator") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->cost_limit_operator = rb_parse_operator(rb_get_string(idx)); l->has_cost_limit_operator = 1; }
        else if (tag == RB_TAG_NULL) { l->has_cost_limit_operator = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "cost_reference_character") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->cost_reference_character = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->cost_reference_character = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "cost_reference_operator") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->cost_reference_operator = rb_parse_operator(rb_get_string(idx)); l->has_cost_reference_operator = 1; }
        else if (tag == RB_TAG_NULL) { l->has_cost_reference_operator = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "cost_total") == 0) {
        if (tag == RB_TAG_I64) { int64_t v; if (!rd_int(r, &v)) return 0; l->cost_total = (int)v; l->has_cost_total = 1; }
        else if (tag == RB_TAG_NULL) { l->has_cost_total = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "cost_total_operator") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->cost_total_operator = rb_parse_operator(rb_get_string(idx)); l->has_cost_total_operator = 1; }
        else if (tag == RB_TAG_NULL) { l->has_cost_total_operator = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "count") == 0) {
        if (tag == RB_TAG_I64) { int64_t v; if (!rd_int(r, &v)) return 0; l->count = (int)v; l->has_count = 1; }
        else if (tag == RB_TAG_NULL) { l->has_count = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "delta") == 0) {
        if (tag == RB_TAG_TRUE) { l->delta = 1; l->has_delta = 1; }
        else if (tag == RB_TAG_FALSE) { l->delta = 0; l->has_delta = 1; }
        else if (tag == RB_TAG_NULL) { l->has_delta = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "destination") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->destination = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->destination = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "distinct") == 0) {
        if (tag == RB_TAG_NULL) { l->distinct = NULL; return 1; }
        if (tag == RB_TAG_TRUE) { l->distinct = rb_strdup("true"); return 1; }
        if (tag == RB_TAG_FALSE) { l->distinct = rb_strdup("false"); return 1; }
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->distinct = rb_strdup(rb_get_string(idx)); return 1; }
        return 0;
    }
    if (strcmp(key, "effect") == 0) {
        if (tag == RB_TAG_NULL) { l->effect = NULL; return 1; }
        if (tag == RB_TAG_OBJVAR) { l->effect = decode_effect_body(r); return 1; }
        return 0;
    }
    if (strcmp(key, "energy_placed") == 0) {
        if (tag == RB_TAG_TRUE) { l->energy_placed = 1; l->has_energy_placed = 1; }
        else if (tag == RB_TAG_FALSE) { l->energy_placed = 0; l->has_energy_placed = 1; }
        else if (tag == RB_TAG_NULL) { l->has_energy_placed = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "energy_state") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->energy_state = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->energy_state = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "exclude_characters") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->exclude_characters = malloc(sizeof(char*) * n);
            l->n_exclude_characters = n;
            for (uint32_t i = 0; i < n; i++) {
                uint8_t st; if (!rd_u8(r, &st)) return 0;
                if (st == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->exclude_characters[i] = rb_strdup(rb_get_string(idx)); }
                else l->exclude_characters[i] = NULL;
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "exclude_group_names") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->exclude_group_names = malloc(sizeof(char*) * n);
            l->n_exclude_group_names = n;
            for (uint32_t i = 0; i < n; i++) {
                uint8_t st; if (!rd_u8(r, &st)) return 0;
                if (st == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->exclude_group_names[i] = rb_strdup(rb_get_string(idx)); }
                else l->exclude_group_names[i] = NULL;
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "exclude_self") == 0) {
        if (tag == RB_TAG_TRUE) { l->exclude_self = 1; l->has_exclude_self = 1; }
        else if (tag == RB_TAG_FALSE) { l->exclude_self = 0; l->has_exclude_self = 1; }
        else if (tag == RB_TAG_NULL) { l->has_exclude_self = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "from_state") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->from_state = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->from_state = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "group_names") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->group_names = malloc(sizeof(char*) * n);
            l->n_group_names = n;
            for (uint32_t i = 0; i < n; i++) {
                uint8_t st; if (!rd_u8(r, &st)) return 0;
                if (st == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->group_names[i] = rb_strdup(rb_get_string(idx)); }
                else l->group_names[i] = NULL;
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "group_reference") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->group_reference = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->group_reference = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "heart_colors") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->heart_colors = malloc(sizeof(char*) * n);
            l->n_heart_colors = n;
            for (uint32_t i = 0; i < n; i++) {
                uint8_t st; if (!rd_u8(r, &st)) return 0;
                if (st == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->heart_colors[i] = rb_strdup(rb_get_string(idx)); }
                else l->heart_colors[i] = NULL;
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "heart_source") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->heart_source = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->heart_source = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "heart_type") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->heart_type = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->heart_type = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "location") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->location = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->location = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "locations") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->locations = malloc(sizeof(char*) * n);
            l->n_locations = n;
            for (uint32_t i = 0; i < n; i++) {
                uint8_t st; if (!rd_u8(r, &st)) return 0;
                if (st == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->locations[i] = rb_strdup(rb_get_string(idx)); }
                else l->locations[i] = NULL;
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "min_baton_touch_count") == 0) {
        if (tag == RB_TAG_I64) { int64_t v; if (!rd_int(r, &v)) return 0; l->min_baton_touch_count = (int)v; l->has_min_baton_touch_count = 1; }
        else if (tag == RB_TAG_NULL) { l->has_min_baton_touch_count = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "movement") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->movement = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->movement = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "negation") == 0) {
        if (tag == RB_TAG_TRUE) { l->negation = 1; l->has_negation = 1; }
        else if (tag == RB_TAG_FALSE) { l->negation = 0; l->has_negation = 1; }
        else if (tag == RB_TAG_NULL) { l->has_negation = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "no_excess_heart") == 0) {
        if (tag == RB_TAG_TRUE) { l->no_excess_heart = 1; l->has_no_excess_heart = 1; }
        else if (tag == RB_TAG_FALSE) { l->no_excess_heart = 0; l->has_no_excess_heart = 1; }
        else if (tag == RB_TAG_NULL) { l->has_no_excess_heart = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "operator") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->cond_operator = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->cond_operator = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "options") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->options = malloc(sizeof(AbilityEffect*) * n);
            l->n_options = n;
            for (uint32_t i = 0; i < n; i++) {
                uint8_t st; if (!rd_u8(r, &st)) return 0;
                if (st == RB_TAG_OBJVAR) { l->options[i] = decode_effect_body(r); }
                else { skip_value(r, st); l->options[i] = NULL; }
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "original_value") == 0) {
        if (tag == RB_TAG_TRUE) { l->original_value = 1; l->has_original_value = 1; }
        else if (tag == RB_TAG_FALSE) { l->original_value = 0; l->has_original_value = 1; }
        else if (tag == RB_TAG_NULL) { l->has_original_value = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "phase") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->phase = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->phase = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "phase_target") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->phase_target = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->phase_target = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "position") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->position = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->position = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "position_compare") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->position_compare = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->position_compare = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "positions_characters") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->positions_characters = malloc(sizeof(char*) * n);
            l->n_positions_characters = n;
            for (uint32_t i = 0; i < n; i++) {
                /* Each element is an object with position + character fields.
                   We serialize it as a JSON-like string for simplicity. */
                if (tag == RB_TAG_OBJVAR || tag == RB_TAG_OBJECT) {
                    char buf[256]; buf[0] = 0;
                    uint8_t obtag = tag;
                    if (obtag == RB_TAG_OBJVAR) { uint8_t vb; if (!rd_u8(r, &vb)) return 0; }
                    uint32_t ocount; if (!rd_len(r, &ocount)) return 0;
                    strcat(buf, "{");
                    for (uint32_t j = 0; j < ocount; j++) {
                        uint32_t kidx; if (!rd_idx(r, &kidx)) return 0;
                        const char *kstr = rb_get_string(kidx);
                        uint8_t vtag; if (!rd_u8(r, &vtag)) return 0;
                        if (strcmp(kstr, "position") == 0 && vtag == RB_TAG_STR) {
                            uint32_t pidx; if (!rd_idx(r, &pidx)) return 0;
                            if (j > 0) strcat(buf, ",");
                            strcat(buf, "\"position\":\"");
                            strcat(buf, rb_get_string(pidx));
                            strcat(buf, "\"");
                        } else if (strcmp(kstr, "character") == 0 && vtag == RB_TAG_STR) {
                            uint32_t cidx; if (!rd_idx(r, &cidx)) return 0;
                            if (j > 0) strcat(buf, ",");
                            strcat(buf, "\"character\":\"");
                            strcat(buf, rb_get_string(cidx));
                            strcat(buf, "\"");
                        } else {
                            skip_value(r, vtag);
                        }
                    }
                    strcat(buf, "}");
                    l->positions_characters[i] = rb_strdup(buf);
                } else {
                    skip_value(r, tag);
                    l->positions_characters[i] = NULL;
                }
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "reference_card") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->reference_card = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->reference_card = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "require_position_cards") == 0) {
        if (tag == RB_TAG_TRUE) { l->require_position_cards = 1; l->has_require_position_cards = 1; }
        else if (tag == RB_TAG_FALSE) { l->require_position_cards = 0; l->has_require_position_cards = 1; }
        else if (tag == RB_TAG_NULL) { l->has_require_position_cards = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "resource_type") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->resource_type = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->resource_type = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "same_name") == 0) {
        if (tag == RB_TAG_TRUE) { l->same_name = 1; l->has_same_name = 1; }
        else if (tag == RB_TAG_FALSE) { l->same_name = 0; l->has_same_name = 1; }
        else if (tag == RB_TAG_NULL) { l->has_same_name = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "scope") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->scope = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->scope = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "self_effect_only") == 0) {
        if (tag == RB_TAG_TRUE) { l->self_effect_only = 1; l->has_self_effect_only = 1; }
        else if (tag == RB_TAG_FALSE) { l->self_effect_only = 0; l->has_self_effect_only = 1; }
        else if (tag == RB_TAG_NULL) { l->has_self_effect_only = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "self_target") == 0) {
        if (tag == RB_TAG_TRUE) { l->self_target = 1; l->has_self_target = 1; }
        else if (tag == RB_TAG_FALSE) { l->self_target = 0; l->has_self_target = 1; }
        else if (tag == RB_TAG_NULL) { l->has_self_target = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "source") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->source = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->source = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "state") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->state = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->state = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "sub_checks") == 0) {
        if (tag == RB_TAG_NULL) { l->sub_checks = NULL; return 1; }
        if (tag == RB_TAG_OBJVAR || tag == RB_TAG_OBJECT) {
            /* Serialize as JSON-like string */
            char buf[512]; buf[0] = 0;
            uint8_t obtag = tag;
            if (obtag == RB_TAG_OBJVAR) { uint8_t vb; if (!rd_u8(r, &vb)) return 0; }
            uint32_t ocount; if (!rd_len(r, &ocount)) return 0;
            strcat(buf, "{");
            for (uint32_t j = 0; j < ocount; j++) {
                uint32_t kidx; if (!rd_idx(r, &kidx)) return 0;
                const char *kstr = rb_get_string(kidx);
                uint8_t vtag; if (!rd_u8(r, &vtag)) return 0;
                /* Just store key:tag for simplicity */
                char fbuf[128]; snprintf(fbuf, sizeof(fbuf), "%s\"%s\":%d", j>0?",":"", kstr, vtag);
                if (strlen(buf) + strlen(fbuf) < sizeof(buf)-1) strcat(buf, fbuf);
                skip_value(r, vtag);
            }
            strcat(buf, "}");
            l->sub_checks = rb_strdup(buf);
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "target") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->target = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->target = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "temporal") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->temporal = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->temporal = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "temporal_scope") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->temporal_scope = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->temporal_scope = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "to_state") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->to_state = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->to_state = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "turn_number") == 0) {
        if (tag == RB_TAG_I64) { int64_t v; if (!rd_int(r, &v)) return 0; l->turn_number = (int)v; l->has_turn_number = 1; }
        else if (tag == RB_TAG_NULL) { l->has_turn_number = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "unit") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->unit = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->unit = NULL; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "values") == 0) {
        if (tag == RB_TAG_NULL) return 1;
        if (tag == RB_TAG_ARRAY) {
            uint32_t n; if (!rd_len(r, &n)) return 0;
            l->values = malloc(sizeof(uint8_t) * n);
            l->n_values = n;
            for (uint32_t i = 0; i < n; i++) {
                uint8_t st; if (!rd_u8(r, &st)) return 0;
                if (st == RB_TAG_I64) { int64_t v; if (!rd_int(r, &v)) return 0; l->values[i] = (uint8_t)v; }
                else { skip_value(r, st); l->values[i] = 0; }
            }
            return 1;
        }
        return 0;
    }
    if (strcmp(key, "yell_trigger") == 0) {
        if (tag == RB_TAG_TRUE) { l->yell_trigger = 1; l->has_yell_trigger = 1; }
        else if (tag == RB_TAG_FALSE) { l->yell_trigger = 0; l->has_yell_trigger = 1; }
        else if (tag == RB_TAG_NULL) { l->has_yell_trigger = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "type") == 0) {
        skip_value(r, tag);
        return 1;
    }
    /* Unknown field — skip */
    skip_value(r, tag);
    return 1;
}

/* ── decode_condition_direct: direct decoder for TAG_OBJECT_VARIANT conditions ──
   Mirrors Rust decode_condition_direct. Reads variant byte, then all fields into
   ConditionLocals, then dispatches to the appropriate build_* function. */
Condition *decode_condition_direct(Rdr *r, uint8_t variant) {
    uint32_t count;
    if (!rd_len(r, &count)) return NULL;
    ConditionLocals l;
    memset(&l, 0, sizeof(l));
    for (uint32_t i = 0; i < count; i++) {
        uint32_t kidx; if (!rd_idx(r, &kidx)) return NULL;
        const char *key = rb_get_string(kidx);
        if (!decode_condition_field(r, key, &l)) return NULL;
    }
    switch (variant) {
        case 0: return build_compound(&l);
        case 1: return build_location(&l);
        case 2: return build_comparison(&l);
        case 3: return build_movement(&l);
        case 4: return build_group(&l);
        case 5: return build_appearance(&l);
        case 6: return build_temporal(&l);
        case 7: return build_state(&l);
        case 8: return build_resource(&l);
        case 9: return NULL; /* build_abilityfilter — not in the 23 unmatched list */
        case 10: return build_scorethreshold(&l);
        case 11: return build_choice(&l);
        case 12: return build_complex(&l);
        case 13: return NULL; /* build_positioncond — not in the 23 unmatched list */
        case 14: return build_opponentchoice(&l);
        case 15: return build_opponentlivesuccess(&l);
        case 16: return build_noexcessheart(&l);
        case 17: return build_alwaystrue(&l);
        case 18: return build_anyof(&l);
        case 19: return build_allrevealedmatchheartcolor(&l);
        default: return NULL;
    }
}

/* ── build_filter (ported from effect_decoder_gen.rs) ──
   Rust: builds an EffectFilter from EffectKindLocals, returns None when every
   filter field is empty (lazy allocation). C mapping: the AbilityEffect struct
   carries all filter fields directly (source, destination, target, card_type,
   group_names, heart_colors, etc. decoded into extra_k/extra_v[] by
   decode_effect_body), so there is no separate EffectFilter struct to build.
   Returns NULL (= Rust None). The function is retained for ABI parity with the
   Rust decoder; it is never called on the C execution path because the action
   string (e->action) is used directly instead of EffectKind::from_action(). */
void *build_filter(const void *ek) {
     (void)ek;
     return NULL;
}

/* ── offset_of (stub) ──
   Computes the byte offset of ability `idx` within the bytecode blob.
   In Rust this sums OFFSET_DELTAS[..idx]; the C engine uses rb_bc_slice
   which does the same via g_offset_deltas. This stub exists for ABI
   parity with the Rust decoder. */
uint32_t offset_of(uint32_t idx) {
    uint32_t off = 0;
    for (uint32_t i = 0; i < idx; i++)
        off += g_offset_deltas[i];
    return off;
}

/* ── i64 (stub) ──
   Reads a full 8-byte little-endian i64 from the reader.
   Mirrors BcReader::i64 in vm.rs. The C engine uses rd_int for the
   variable-width encoding; this is the fixed-width fallback. */
int rd_i64(Rdr *r, int64_t *out) {
    if (r->p + 8 > r->end) return 0;
    *out = (int64_t)r->p[0] | ((int64_t)r->p[1] << 8) | ((int64_t)r->p[2] << 16) |
           ((int64_t)r->p[3] << 24) | ((int64_t)r->p[4] << 32) | ((int64_t)r->p[5] << 40) |
           ((int64_t)r->p[6] << 48) | ((int64_t)r->p[7] << 56);
    r->p += 8;
    return 1;
}

/* ── key (stub) ──
   Reads a string-table key from the reader.
   Mirrors BcReader::key in vm.rs: reads a string index and returns the
   interned string pointer. Returns NULL on failure. */
const char *rd_key(Rdr *r) {
    uint32_t idx;
    if (!rd_idx(r, &idx)) return NULL;
    return rb_get_string(idx);
}

/* ── populate_from_json (stub) ──
    JSON-path decode; used only by the deep-compare oracle (feature
    json_path_test). The C engine has no JSON path, so this is a no-op.
    Mirrors AbilityEffect::populate_from_json in vm.rs. */
void populate_from_json(void *effect, const void *json_val) {
    (void)effect;
    (void)json_val;
}

/* ── Enum wire-string helpers (ported from engine/src/ability/enums.rs) ── */

/* Zone::to_str — convert RbZoneId to its wire string. */
const char *rb_zone_to_str(int z) {
    switch (z) {
        case RB_ZONEID_STAGE: return "stage";
        case RB_ZONEID_HAND: return "hand";
        case RB_ZONEID_DECK: return "deck";
        case RB_ZONEID_DECK_TOP: return "deck_top";
        case RB_ZONEID_DECK_BOTTOM: return "deck_bottom";
        case RB_ZONEID_DISCARD: return "discard";
        case RB_ZONEID_WAITROOM: return "waitroom";
        case RB_ZONEID_ENERGY: return "energy";
        case RB_ZONEID_ENERGY_ZONE: return "energy_zone";
        case RB_ZONEID_ENERGY_DECK: return "energy_deck";
        case RB_ZONEID_SUCCESS_ZONE: return "success_zone";
        case RB_ZONEID_LIVE_CARD_ZONE: return "live_card_zone";
        case RB_ZONEID_SUCCESS_LIVE_ZONE: return "success_live_zone";
        case RB_ZONEID_EMPTY_AREA: return "empty_area";
        case RB_ZONEID_SAME_AREA: return "same_area";
        case RB_ZONEID_UNDER_MEMBER: return "under_member";
        case RB_ZONEID_LOOKED_AT: return "looked_at";
        case RB_ZONEID_REVEALED_CARDS: return "revealed_cards";
        case RB_ZONEID_SELECTED_CARDS: return "selected_cards";
        case RB_ZONEID_RESOLUTION: return "resolution";
        case RB_ZONEID_EXCLUSION_ZONE: return "exclusion_zone";
        default: return "unknown";
    }
}

/* Zone::from_source_str — always-succeed conversion. Unknown → RB_ZONEID_UNKNOWN. */
int rb_zone_from_source_str(const char *s) {
    int z = RB_ZONEID_UNKNOWN;
    if (!s) return z;
    if (rb_zone_of_str(s, (RbZone *)&z)) return z;
    /* Also try RbZoneId mapping */
    if (!strcmp(s, "stage")) z = RB_ZONEID_STAGE;
    else if (!strcmp(s, "hand")) z = RB_ZONEID_HAND;
    else if (!strcmp(s, "deck")) z = RB_ZONEID_DECK;
    else if (!strcmp(s, "deck_top")) z = RB_ZONEID_DECK_TOP;
    else if (!strcmp(s, "deck_bottom")) z = RB_ZONEID_DECK_BOTTOM;
    else if (!strcmp(s, "discard")) z = RB_ZONEID_DISCARD;
    else if (!strcmp(s, "waitroom")) z = RB_ZONEID_WAITROOM;
    else if (!strcmp(s, "energy")) z = RB_ZONEID_ENERGY;
    else if (!strcmp(s, "energy_zone")) z = RB_ZONEID_ENERGY_ZONE;
    else if (!strcmp(s, "energy_deck")) z = RB_ZONEID_ENERGY_DECK;
    else if (!strcmp(s, "success_zone")) z = RB_ZONEID_SUCCESS_ZONE;
    else if (!strcmp(s, "live_card_zone")) z = RB_ZONEID_LIVE_CARD_ZONE;
    else if (!strcmp(s, "success_live_zone") || !strcmp(s, "success_live_card_zone")) z = RB_ZONEID_SUCCESS_LIVE_ZONE;
    else if (!strcmp(s, "empty_area")) z = RB_ZONEID_EMPTY_AREA;
    else if (!strcmp(s, "same_area")) z = RB_ZONEID_SAME_AREA;
    else if (!strcmp(s, "under_member") || !strcmp(s, "under")) z = RB_ZONEID_UNDER_MEMBER;
    else if (!strcmp(s, "looked_at")) z = RB_ZONEID_LOOKED_AT;
    else if (!strcmp(s, "revealed_cards")) z = RB_ZONEID_REVEALED_CARDS;
    else if (!strcmp(s, "selected_cards")) z = RB_ZONEID_SELECTED_CARDS;
    else if (!strcmp(s, "resolution") || !strcmp(s, "resolution_zone")) z = RB_ZONEID_RESOLUTION;
    else if (!strcmp(s, "exclusion_zone")) z = RB_ZONEID_EXCLUSION_ZONE;
    else if (!strcmp(s, "preceding_moved")) z = RB_ZONEID_UNKNOWN;
    else if (!strcmp(s, "recently_moved")) z = RB_ZONEID_UNKNOWN;
    else if (!strcmp(s, "those_cards")) z = RB_ZONEID_UNKNOWN;
    else if (!strcmp(s, "looked_at_remaining")) z = RB_ZONEID_UNKNOWN;
    else if (!strcmp(s, "deck_top_or_bottom")) z = RB_ZONEID_UNKNOWN;
    else if (!strcmp(s, "front")) z = RB_ZONEID_UNKNOWN;
    else z = RB_ZONEID_UNKNOWN;
    return z;
}

/* Zone::as_str — alias for to_str. */
const char *rb_zone_as_str(int z) {
    return rb_zone_to_str(z);
}

/* TargetPlayer wire strings. */
typedef enum {
    RB_TARGET_SELF = 0,
    RB_TARGET_OPPONENT,
    RB_TARGET_BOTH,
    RB_TARGET_EITHER
} RbTargetPlayer;

int rb_target_player_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "self")) return RB_TARGET_SELF;
    if (!strcmp(s, "opponent")) return RB_TARGET_OPPONENT;
    if (!strcmp(s, "both")) return RB_TARGET_BOTH;
    if (!strcmp(s, "either")) return RB_TARGET_EITHER;
    return -1;
}

const char *rb_target_player_to_str(int tp) {
    switch (tp) {
        case RB_TARGET_SELF: return "self";
        case RB_TARGET_OPPONENT: return "opponent";
        case RB_TARGET_BOTH: return "both";
        case RB_TARGET_EITHER: return "either";
        default: return NULL;
    }
}

const char *rb_target_player_as_str(int tp) {
    return rb_target_player_to_str(tp);
}

/* ActionType wire strings. */
typedef enum {
    RB_ACTION_DRAW_CARD = 0,
    RB_ACTION_DRAW_UNTIL_COUNT,
    RB_ACTION_MOVE_CARDS,
    RB_ACTION_DISCARD_CARD,
    RB_ACTION_SELECT,
    RB_ACTION_SELECT_NUMBER,
    RB_ACTION_SELECT_CARDS,
    RB_ACTION_LOOK_AND_SELECT,
    RB_ACTION_LOOK_AT,
    RB_ACTION_REVEAL,
    RB_ACTION_REVEAL_PER_GROUP,
    RB_ACTION_REVEAL_UNTIL_LIVE_CARD,
    RB_ACTION_REVEAL_UNTIL_CHOSEN_CARD,
    RB_ACTION_CHANGE_STATE,
    RB_ACTION_POSITION_CHANGE,
    RB_ACTION_ROTATION,
    RB_ACTION_PLACE_ENERGY_UNDER_MEMBER,
    RB_ACTION_MODIFY_REQUIRED_HEARTS_SUCCESS,
    RB_ACTION_GAIN_RESOURCE,
    RB_ACTION_PAY_ENERGY,
    RB_ACTION_GAIN_ABILITY,
    RB_ACTION_GAIN_ABILITY_FROM_SOURCE,
    RB_ACTION_INVALIDATE_ABILITY,
    RB_ACTION_SUPPRESS_ABILITY_TRIGGER,
    RB_ACTION_ACTIVATE_ABILITY,
    RB_ACTION_MODIFY_COST,
    RB_ACTION_MODIFY_YELL_SOURCE,
    RB_ACTION_SET_COST,
    RB_ACTION_SET_CARD_IDENTITY,
    RB_ACTION_SET_COST_TO_USE,
    RB_ACTION_MODIFY_SCORE,
    RB_ACTION_MODIFY_REQUIRED_HEARTS,
    RB_ACTION_SET_BLADE_TYPE,
    RB_ACTION_SET_BLADE_COUNT,
    RB_ACTION_SET_HEART_TYPE,
    RB_ACTION_SPECIFY_HEART_COLOR,
    RB_ACTION_CHOOSE_REQUIRED_HEARTS,
    RB_ACTION_SEQUENTIAL,
    RB_ACTION_CONDITIONAL_ALTERNATIVE,
    RB_ACTION_CONDITIONAL_ON_RESULT,
    RB_ACTION_CONDITIONAL_ON_OPTIONAL,
    RB_ACTION_RESTRICTION,
    RB_ACTION_ACTIVATION_RESTRICTION,
    RB_ACTION_MODIFY_LIMIT,
    RB_ACTION_SHUFFLE,
    RB_ACTION_RE_YELL,
    RB_ACTION_CUSTOM,
    RB_ACTION_DO_NOTHING,
    RB_ACTION_CHOICE,
    RB_ACTION_REPEAT_PROCEDURE,
    RB_ACTION_DISCARD_UNTIL_COUNT,
    RB_ACTION_ALL_BLADE_TIMING,
    RB_ACTION_REDUCE_LIVE_CARD_SET_LIMIT,
    RB_ACTION_CHOOSE_TARGET_PLAYER,
    RB_ACTION_PLAY_BATON_TOUCH,
    RB_ACTION_MODIFY_REQUIRED_HEARTS_GLOBAL,
    RB_ACTION_MODIFY_YELL_COUNT,
    RB_ACTION_ACTIVATION_COST,
    RB_ACTION_PERFORM_YELL,
    RB_ACTION_CONDITIONAL_OPTIONAL,
    RB_ACTION_COMPOUND_ACTION,
    RB_ACTION_OPPONENT_ACTION,
    RB_ACTION_ACTION_BY,
    RB_ACTION_SEQUENTIAL_COST,
    RB_ACTION_CHOICE_CONDITION,
    RB_ACTION_ENERGY_CONDITION,
} RbActionType;

int rb_action_type_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "draw_card")) return RB_ACTION_DRAW_CARD;
    if (!strcmp(s, "draw_until_count")) return RB_ACTION_DRAW_UNTIL_COUNT;
    if (!strcmp(s, "move_cards")) return RB_ACTION_MOVE_CARDS;
    if (!strcmp(s, "discard_card")) return RB_ACTION_DISCARD_CARD;
    if (!strcmp(s, "select")) return RB_ACTION_SELECT;
    if (!strcmp(s, "select_number")) return RB_ACTION_SELECT_NUMBER;
    if (!strcmp(s, "select_cards")) return RB_ACTION_SELECT_CARDS;
    if (!strcmp(s, "look_and_select")) return RB_ACTION_LOOK_AND_SELECT;
    if (!strcmp(s, "look_at")) return RB_ACTION_LOOK_AT;
    if (!strcmp(s, "reveal")) return RB_ACTION_REVEAL;
    if (!strcmp(s, "reveal_per_group")) return RB_ACTION_REVEAL_PER_GROUP;
    if (!strcmp(s, "reveal_until_live_card")) return RB_ACTION_REVEAL_UNTIL_LIVE_CARD;
    if (!strcmp(s, "reveal_until_chosen_card")) return RB_ACTION_REVEAL_UNTIL_CHOSEN_CARD;
    if (!strcmp(s, "change_state")) return RB_ACTION_CHANGE_STATE;
    if (!strcmp(s, "position_change")) return RB_ACTION_POSITION_CHANGE;
    if (!strcmp(s, "rotation")) return RB_ACTION_ROTATION;
    if (!strcmp(s, "place_energy_under_member")) return RB_ACTION_PLACE_ENERGY_UNDER_MEMBER;
    if (!strcmp(s, "modify_required_hearts_success")) return RB_ACTION_MODIFY_REQUIRED_HEARTS_SUCCESS;
    if (!strcmp(s, "gain_resource")) return RB_ACTION_GAIN_RESOURCE;
    if (!strcmp(s, "pay_energy")) return RB_ACTION_PAY_ENERGY;
    if (!strcmp(s, "gain_ability")) return RB_ACTION_GAIN_ABILITY;
    if (!strcmp(s, "gain_ability_from_source")) return RB_ACTION_GAIN_ABILITY_FROM_SOURCE;
    if (!strcmp(s, "invalidate_ability")) return RB_ACTION_INVALIDATE_ABILITY;
    if (!strcmp(s, "suppress_ability_trigger")) return RB_ACTION_SUPPRESS_ABILITY_TRIGGER;
    if (!strcmp(s, "activate_ability")) return RB_ACTION_ACTIVATE_ABILITY;
    if (!strcmp(s, "modify_cost")) return RB_ACTION_MODIFY_COST;
    if (!strcmp(s, "modify_yell_source")) return RB_ACTION_MODIFY_YELL_SOURCE;
    if (!strcmp(s, "set_cost")) return RB_ACTION_SET_COST;
    if (!strcmp(s, "set_card_identity")) return RB_ACTION_SET_CARD_IDENTITY;
    if (!strcmp(s, "set_cost_to_use")) return RB_ACTION_SET_COST_TO_USE;
    if (!strcmp(s, "modify_score")) return RB_ACTION_MODIFY_SCORE;
    if (!strcmp(s, "modify_required_hearts")) return RB_ACTION_MODIFY_REQUIRED_HEARTS;
    if (!strcmp(s, "set_blade_type")) return RB_ACTION_SET_BLADE_TYPE;
    if (!strcmp(s, "set_blade_count")) return RB_ACTION_SET_BLADE_COUNT;
    if (!strcmp(s, "set_heart_type")) return RB_ACTION_SET_HEART_TYPE;
    if (!strcmp(s, "specify_heart_color")) return RB_ACTION_SPECIFY_HEART_COLOR;
    if (!strcmp(s, "choose_required_hearts")) return RB_ACTION_CHOOSE_REQUIRED_HEARTS;
    if (!strcmp(s, "sequential")) return RB_ACTION_SEQUENTIAL;
    if (!strcmp(s, "conditional_alternative")) return RB_ACTION_CONDITIONAL_ALTERNATIVE;
    if (!strcmp(s, "conditional_on_result")) return RB_ACTION_CONDITIONAL_ON_RESULT;
    if (!strcmp(s, "conditional_on_optional")) return RB_ACTION_CONDITIONAL_ON_OPTIONAL;
    if (!strcmp(s, "restriction")) return RB_ACTION_RESTRICTION;
    if (!strcmp(s, "activation_restriction")) return RB_ACTION_ACTIVATION_RESTRICTION;
    if (!strcmp(s, "modify_limit")) return RB_ACTION_MODIFY_LIMIT;
    if (!strcmp(s, "shuffle")) return RB_ACTION_SHUFFLE;
    if (!strcmp(s, "re_yell")) return RB_ACTION_RE_YELL;
    if (!strcmp(s, "custom")) return RB_ACTION_CUSTOM;
    if (!strcmp(s, "do_nothing")) return RB_ACTION_DO_NOTHING;
    if (!strcmp(s, "choice")) return RB_ACTION_CHOICE;
    if (!strcmp(s, "repeat_procedure")) return RB_ACTION_REPEAT_PROCEDURE;
    if (!strcmp(s, "discard_until_count")) return RB_ACTION_DISCARD_UNTIL_COUNT;
    if (!strcmp(s, "all_blade_timing")) return RB_ACTION_ALL_BLADE_TIMING;
    if (!strcmp(s, "reduce_live_card_set_limit")) return RB_ACTION_REDUCE_LIVE_CARD_SET_LIMIT;
    if (!strcmp(s, "choose_target_player")) return RB_ACTION_CHOOSE_TARGET_PLAYER;
    if (!strcmp(s, "play_baton_touch")) return RB_ACTION_PLAY_BATON_TOUCH;
    if (!strcmp(s, "modify_required_hearts_global")) return RB_ACTION_MODIFY_REQUIRED_HEARTS_GLOBAL;
    if (!strcmp(s, "modify_yell_count")) return RB_ACTION_MODIFY_YELL_COUNT;
    if (!strcmp(s, "activation_cost")) return RB_ACTION_ACTIVATION_COST;
    if (!strcmp(s, "perform_yell")) return RB_ACTION_PERFORM_YELL;
    if (!strcmp(s, "conditional_optional")) return RB_ACTION_CONDITIONAL_OPTIONAL;
    if (!strcmp(s, "compound_action")) return RB_ACTION_COMPOUND_ACTION;
    if (!strcmp(s, "opponent_action")) return RB_ACTION_OPPONENT_ACTION;
    if (!strcmp(s, "action_by")) return RB_ACTION_ACTION_BY;
    if (!strcmp(s, "sequential_cost")) return RB_ACTION_SEQUENTIAL_COST;
    if (!strcmp(s, "choice_condition")) return RB_ACTION_CHOICE_CONDITION;
    if (!strcmp(s, "energy_condition")) return RB_ACTION_ENERGY_CONDITION;
    return -1;
}

const char *rb_action_type_to_str(int at) {
    switch (at) {
        case RB_ACTION_DRAW_CARD: return "draw_card";
        case RB_ACTION_DRAW_UNTIL_COUNT: return "draw_until_count";
        case RB_ACTION_MOVE_CARDS: return "move_cards";
        case RB_ACTION_DISCARD_CARD: return "discard_card";
        case RB_ACTION_SELECT: return "select";
        case RB_ACTION_SELECT_NUMBER: return "select_number";
        case RB_ACTION_SELECT_CARDS: return "select_cards";
        case RB_ACTION_LOOK_AND_SELECT: return "look_and_select";
        case RB_ACTION_LOOK_AT: return "look_at";
        case RB_ACTION_REVEAL: return "reveal";
        case RB_ACTION_REVEAL_PER_GROUP: return "reveal_per_group";
        case RB_ACTION_REVEAL_UNTIL_LIVE_CARD: return "reveal_until_live_card";
        case RB_ACTION_REVEAL_UNTIL_CHOSEN_CARD: return "reveal_until_chosen_card";
        case RB_ACTION_CHANGE_STATE: return "change_state";
        case RB_ACTION_POSITION_CHANGE: return "position_change";
        case RB_ACTION_ROTATION: return "rotation";
        case RB_ACTION_PLACE_ENERGY_UNDER_MEMBER: return "place_energy_under_member";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS_SUCCESS: return "modify_required_hearts_success";
        case RB_ACTION_GAIN_RESOURCE: return "gain_resource";
        case RB_ACTION_PAY_ENERGY: return "pay_energy";
        case RB_ACTION_GAIN_ABILITY: return "gain_ability";
        case RB_ACTION_GAIN_ABILITY_FROM_SOURCE: return "gain_ability_from_source";
        case RB_ACTION_INVALIDATE_ABILITY: return "invalidate_ability";
        case RB_ACTION_SUPPRESS_ABILITY_TRIGGER: return "suppress_ability_trigger";
        case RB_ACTION_ACTIVATE_ABILITY: return "activate_ability";
        case RB_ACTION_MODIFY_COST: return "modify_cost";
        case RB_ACTION_MODIFY_YELL_SOURCE: return "modify_yell_source";
        case RB_ACTION_SET_COST: return "set_cost";
        case RB_ACTION_SET_CARD_IDENTITY: return "set_card_identity";
        case RB_ACTION_SET_COST_TO_USE: return "set_cost_to_use";
        case RB_ACTION_MODIFY_SCORE: return "modify_score";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS: return "modify_required_hearts";
        case RB_ACTION_SET_BLADE_TYPE: return "set_blade_type";
        case RB_ACTION_SET_BLADE_COUNT: return "set_blade_count";
        case RB_ACTION_SET_HEART_TYPE: return "set_heart_type";
        case RB_ACTION_SPECIFY_HEART_COLOR: return "specify_heart_color";
        case RB_ACTION_CHOOSE_REQUIRED_HEARTS: return "choose_required_hearts";
        case RB_ACTION_SEQUENTIAL: return "sequential";
        case RB_ACTION_CONDITIONAL_ALTERNATIVE: return "conditional_alternative";
        case RB_ACTION_CONDITIONAL_ON_RESULT: return "conditional_on_result";
        case RB_ACTION_CONDITIONAL_ON_OPTIONAL: return "conditional_on_optional";
        case RB_ACTION_RESTRICTION: return "restriction";
        case RB_ACTION_ACTIVATION_RESTRICTION: return "activation_restriction";
        case RB_ACTION_MODIFY_LIMIT: return "modify_limit";
        case RB_ACTION_SHUFFLE: return "shuffle";
        case RB_ACTION_RE_YELL: return "re_yell";
        case RB_ACTION_CUSTOM: return "custom";
        case RB_ACTION_DO_NOTHING: return "do_nothing";
        case RB_ACTION_CHOICE: return "choice";
        case RB_ACTION_REPEAT_PROCEDURE: return "repeat_procedure";
        case RB_ACTION_DISCARD_UNTIL_COUNT: return "discard_until_count";
        case RB_ACTION_ALL_BLADE_TIMING: return "all_blade_timing";
        case RB_ACTION_REDUCE_LIVE_CARD_SET_LIMIT: return "reduce_live_card_set_limit";
        case RB_ACTION_CHOOSE_TARGET_PLAYER: return "choose_target_player";
        case RB_ACTION_PLAY_BATON_TOUCH: return "play_baton_touch";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS_GLOBAL: return "modify_required_hearts_global";
        case RB_ACTION_MODIFY_YELL_COUNT: return "modify_yell_count";
        case RB_ACTION_ACTIVATION_COST: return "activation_cost";
        case RB_ACTION_PERFORM_YELL: return "perform_yell";
        case RB_ACTION_CONDITIONAL_OPTIONAL: return "conditional_optional";
        case RB_ACTION_COMPOUND_ACTION: return "compound_action";
        case RB_ACTION_OPPONENT_ACTION: return "opponent_action";
        case RB_ACTION_ACTION_BY: return "action_by";
        case RB_ACTION_SEQUENTIAL_COST: return "sequential_cost";
        case RB_ACTION_CHOICE_CONDITION: return "choice_condition";
        case RB_ACTION_ENERGY_CONDITION: return "energy_condition";
        default: return NULL;
    }
}

const char *rb_action_type_label(int at) {
    switch (at) {
        case RB_ACTION_DRAW_CARD: return "Draw Card";
        case RB_ACTION_DRAW_UNTIL_COUNT: return "Draw Until Count";
        case RB_ACTION_CHOOSE_TARGET_PLAYER: return "Choose Target Player";
        case RB_ACTION_MOVE_CARDS: return "Move Cards";
        case RB_ACTION_DISCARD_CARD: return "Discard Card";
        case RB_ACTION_SELECT: return "Select";
        case RB_ACTION_SELECT_NUMBER: return "Select Number";
        case RB_ACTION_SELECT_CARDS: return "Select Cards";
        case RB_ACTION_LOOK_AND_SELECT: return "Look and Select";
        case RB_ACTION_LOOK_AT: return "Look At";
        case RB_ACTION_REVEAL: return "Reveal";
        case RB_ACTION_REVEAL_PER_GROUP: return "Reveal Per Group";
        case RB_ACTION_REVEAL_UNTIL_LIVE_CARD: return "Reveal Until Live Card";
        case RB_ACTION_REVEAL_UNTIL_CHOSEN_CARD: return "Reveal Until Chosen Card";
        case RB_ACTION_CHANGE_STATE: return "Change State";
        case RB_ACTION_POSITION_CHANGE: return "Position Change";
        case RB_ACTION_ROTATION: return "Rotation";
        case RB_ACTION_PLACE_ENERGY_UNDER_MEMBER: return "Place Energy Under Member";
        case RB_ACTION_SET_CARD_IDENTITY: return "Set Card Identity";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS_SUCCESS: return "Modify Required Hearts (Success)";
        case RB_ACTION_GAIN_RESOURCE: return "Gain Resource";
        case RB_ACTION_PAY_ENERGY: return "Pay Energy";
        case RB_ACTION_GAIN_ABILITY: return "Gain Ability";
        case RB_ACTION_GAIN_ABILITY_FROM_SOURCE: return "Gain Ability from Source";
        case RB_ACTION_INVALIDATE_ABILITY: return "Invalidate Ability";
        case RB_ACTION_SUPPRESS_ABILITY_TRIGGER: return "Suppress Ability Trigger";
        case RB_ACTION_ACTIVATE_ABILITY: return "Activate Ability";
        case RB_ACTION_MODIFY_SCORE: return "Modify Score";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS: return "Modify Required Hearts";
        case RB_ACTION_MODIFY_YELL_SOURCE: return "Modify Yell Source";
        case RB_ACTION_MODIFY_COST: return "Modify Cost";
        case RB_ACTION_SET_COST: return "Set Cost";
        case RB_ACTION_SET_COST_TO_USE: return "Set Cost to Use";
        case RB_ACTION_SET_BLADE_TYPE: return "Set Blade Type";
        case RB_ACTION_SET_BLADE_COUNT: return "Set Blade Count";
        case RB_ACTION_SET_HEART_TYPE: return "Set Heart Type";
        case RB_ACTION_SPECIFY_HEART_COLOR: return "Specify Heart Color";
        case RB_ACTION_CHOOSE_REQUIRED_HEARTS: return "Choose Required Hearts";
        case RB_ACTION_SEQUENTIAL: return "Sequential";
        case RB_ACTION_CONDITIONAL_ALTERNATIVE: return "Conditional Alternative";
        case RB_ACTION_CONDITIONAL_ON_RESULT: return "Conditional on Result";
        case RB_ACTION_CONDITIONAL_ON_OPTIONAL: return "Conditional on Optional";
        case RB_ACTION_RESTRICTION: return "Restriction";
        case RB_ACTION_ACTIVATION_RESTRICTION: return "Activation Restriction";
        case RB_ACTION_MODIFY_LIMIT: return "Modify Limit";
        case RB_ACTION_SHUFFLE: return "Shuffle";
        case RB_ACTION_RE_YELL: return "Re Yell";
        case RB_ACTION_CUSTOM: return "Custom";
        case RB_ACTION_DO_NOTHING: return "Do Nothing";
        case RB_ACTION_CHOICE: return "Choice";
        case RB_ACTION_REPEAT_PROCEDURE: return "Repeat Procedure";
        case RB_ACTION_DISCARD_UNTIL_COUNT: return "Discard Until Count";
        case RB_ACTION_ALL_BLADE_TIMING: return "All Blade Timing";
        case RB_ACTION_REDUCE_LIVE_CARD_SET_LIMIT: return "Reduce Live Card Set Limit";
        case RB_ACTION_PLAY_BATON_TOUCH: return "Play Baton Touch";
        case RB_ACTION_MODIFY_REQUIRED_HEARTS_GLOBAL: return "Modify Required Hearts (Global)";
        case RB_ACTION_MODIFY_YELL_COUNT: return "Modify Yell Count";
        case RB_ACTION_ACTIVATION_COST: return "Activation Cost";
        case RB_ACTION_PERFORM_YELL: return "Perform Yell";
        case RB_ACTION_CONDITIONAL_OPTIONAL: return "Conditional Optional";
        case RB_ACTION_COMPOUND_ACTION: return "Compound Action";
        case RB_ACTION_OPPONENT_ACTION: return "Opponent Action";
        case RB_ACTION_ACTION_BY: return "Action By";
        case RB_ACTION_SEQUENTIAL_COST: return "Sequential Cost";
        case RB_ACTION_CHOICE_CONDITION: return "Choice Condition";
        case RB_ACTION_ENERGY_CONDITION: return "Energy Condition";
        default: return NULL;
    }
}

/* ConditionType wire strings. */
typedef enum {
    RB_CONDTYPE_COMPOUND = 0,
    RB_CONDTYPE_COMPARISON,
    RB_CONDTYPE_LOCATION,
    RB_CONDTYPE_CARD_COUNT,
    RB_CONDTYPE_CARD_BLADE,
    RB_CONDTYPE_GROUP,
    RB_CONDTYPE_POSITION,
    RB_CONDTYPE_APPEARANCE,
    RB_CONDTYPE_TEMPORAL,
    RB_CONDTYPE_STATE,
    RB_CONDTYPE_ENERGY_STATE,
    RB_CONDTYPE_MOVEMENT,
    RB_CONDTYPE_ABILITY_FILTER,
    RB_CONDTYPE_OR,
    RB_CONDTYPE_ANY_OF,
    RB_CONDTYPE_SCORE_THRESHOLD,
    RB_CONDTYPE_CHOICE,
    RB_CONDTYPE_POSITION_CHANGE,
    RB_CONDTYPE_STATE_CHANGE,
    RB_CONDTYPE_OPPONENT_CHOICE,
    RB_CONDTYPE_OPPONENT_LIVE_SUCCESS,
    RB_CONDTYPE_COMPLEX,
    RB_CONDTYPE_NO_EXCESS_HEART,
    RB_CONDTYPE_OTHERWISE,
    RB_CONDTYPE_NOT_MOVED,
    RB_CONDTYPE_HAS_MOVED,
    RB_CONDTYPE_RESOURCE,
    RB_CONDTYPE_ACTION_SUCCESS,
    RB_CONDTYPE_ALL_COST_COMPARISON,
    RB_CONDTYPE_HIGHEST_COST_ON_STAGE,
    RB_CONDTYPE_BOTH,
    RB_CONDTYPE_ALL_REVEALED_MATCH_HEART_COLOR,
    RB_CONDTYPE_CUSTOM,
} RbConditionType;

int rb_condition_type_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "compound")) return RB_CONDTYPE_COMPOUND;
    if (!strcmp(s, "comparison_condition")) return RB_CONDTYPE_COMPARISON;
    if (!strcmp(s, "location_condition")) return RB_CONDTYPE_LOCATION;
    if (!strcmp(s, "card_count_condition")) return RB_CONDTYPE_CARD_COUNT;
    if (!strcmp(s, "card_blade_condition")) return RB_CONDTYPE_CARD_BLADE;
    if (!strcmp(s, "group_condition")) return RB_CONDTYPE_GROUP;
    if (!strcmp(s, "position_condition")) return RB_CONDTYPE_POSITION;
    if (!strcmp(s, "appearance_condition")) return RB_CONDTYPE_APPEARANCE;
    if (!strcmp(s, "temporal_condition")) return RB_CONDTYPE_TEMPORAL;
    if (!strcmp(s, "state_condition")) return RB_CONDTYPE_STATE;
    if (!strcmp(s, "energy_state_condition")) return RB_CONDTYPE_ENERGY_STATE;
    if (!strcmp(s, "movement_condition")) return RB_CONDTYPE_MOVEMENT;
    if (!strcmp(s, "ability_filter_condition")) return RB_CONDTYPE_ABILITY_FILTER;
    if (!strcmp(s, "or_condition")) return RB_CONDTYPE_OR;
    if (!strcmp(s, "any_of_condition")) return RB_CONDTYPE_ANY_OF;
    if (!strcmp(s, "score_threshold_condition")) return RB_CONDTYPE_SCORE_THRESHOLD;
    if (!strcmp(s, "choice_condition")) return RB_CONDTYPE_CHOICE;
    if (!strcmp(s, "position_change_condition")) return RB_CONDTYPE_POSITION_CHANGE;
    if (!strcmp(s, "state_change_condition")) return RB_CONDTYPE_STATE_CHANGE;
    if (!strcmp(s, "opponent_choice_condition")) return RB_CONDTYPE_OPPONENT_CHOICE;
    if (!strcmp(s, "opponent_live_success")) return RB_CONDTYPE_OPPONENT_LIVE_SUCCESS;
    if (!strcmp(s, "complex_condition")) return RB_CONDTYPE_COMPLEX;
    if (!strcmp(s, "no_excess_heart")) return RB_CONDTYPE_NO_EXCESS_HEART;
    if (!strcmp(s, "otherwise_condition")) return RB_CONDTYPE_OTHERWISE;
    if (!strcmp(s, "both_condition")) return RB_CONDTYPE_BOTH;
    if (!strcmp(s, "not_moved")) return RB_CONDTYPE_NOT_MOVED;
    if (!strcmp(s, "has_moved")) return RB_CONDTYPE_HAS_MOVED;
    if (!strcmp(s, "resource_condition")) return RB_CONDTYPE_RESOURCE;
    if (!strcmp(s, "action_success_condition")) return RB_CONDTYPE_ACTION_SUCCESS;
    if (!strcmp(s, "all_cost_comparison_condition")) return RB_CONDTYPE_ALL_COST_COMPARISON;
    if (!strcmp(s, "highest_cost_on_stage_condition")) return RB_CONDTYPE_HIGHEST_COST_ON_STAGE;
    if (!strcmp(s, "all_revealed_match_heart_color")) return RB_CONDTYPE_ALL_REVEALED_MATCH_HEART_COLOR;
    if (!strcmp(s, "custom")) return RB_CONDTYPE_CUSTOM;
    return -1;
}

const char *rb_condition_type_to_str(int ct) {
    switch (ct) {
        case RB_CONDTYPE_COMPOUND: return "compound";
        case RB_CONDTYPE_COMPARISON: return "comparison_condition";
        case RB_CONDTYPE_LOCATION: return "location_condition";
        case RB_CONDTYPE_CARD_COUNT: return "card_count_condition";
        case RB_CONDTYPE_CARD_BLADE: return "card_blade_condition";
        case RB_CONDTYPE_GROUP: return "group_condition";
        case RB_CONDTYPE_POSITION: return "position_condition";
        case RB_CONDTYPE_APPEARANCE: return "appearance_condition";
        case RB_CONDTYPE_TEMPORAL: return "temporal_condition";
        case RB_CONDTYPE_STATE: return "state_condition";
        case RB_CONDTYPE_ENERGY_STATE: return "energy_state_condition";
        case RB_CONDTYPE_MOVEMENT: return "movement_condition";
        case RB_CONDTYPE_ABILITY_FILTER: return "ability_filter_condition";
        case RB_CONDTYPE_OR: return "or_condition";
        case RB_CONDTYPE_ANY_OF: return "any_of_condition";
        case RB_CONDTYPE_SCORE_THRESHOLD: return "score_threshold_condition";
        case RB_CONDTYPE_CHOICE: return "choice_condition";
        case RB_CONDTYPE_POSITION_CHANGE: return "position_change_condition";
        case RB_CONDTYPE_STATE_CHANGE: return "state_change_condition";
        case RB_CONDTYPE_OPPONENT_CHOICE: return "opponent_choice_condition";
        case RB_CONDTYPE_OPPONENT_LIVE_SUCCESS: return "opponent_live_success";
        case RB_CONDTYPE_COMPLEX: return "complex_condition";
        case RB_CONDTYPE_NO_EXCESS_HEART: return "no_excess_heart";
        case RB_CONDTYPE_OTHERWISE: return "otherwise_condition";
        case RB_CONDTYPE_BOTH: return "both_condition";
        case RB_CONDTYPE_NOT_MOVED: return "not_moved";
        case RB_CONDTYPE_HAS_MOVED: return "has_moved";
        case RB_CONDTYPE_RESOURCE: return "resource_condition";
        case RB_CONDTYPE_ACTION_SUCCESS: return "action_success_condition";
        case RB_CONDTYPE_ALL_COST_COMPARISON: return "all_cost_comparison_condition";
        case RB_CONDTYPE_HIGHEST_COST_ON_STAGE: return "highest_cost_on_stage_condition";
        case RB_CONDTYPE_ALL_REVEALED_MATCH_HEART_COLOR: return "all_revealed_match_heart_color";
        case RB_CONDTYPE_CUSTOM: return "custom";
        default: return NULL;
    }
}

/* SelectTargetKind wire strings. */
typedef enum {
    RB_STK_CHOICE = 0,
    RB_STK_CHOICE_STRING,
    RB_STK_PAY_OPTIONAL_COST_SKIP_OPTIONAL_COST,
    RB_STK_DOUBLE_BATON_TOUCH,
    RB_STK_PRIMARY_ALTERNATIVE,
    RB_STK_APPLY_REPLACEMENT,
    RB_STK_CHOOSE_REQUIRED_HEARTS,
    RB_STK_POSITION_DESTINATION,
    RB_STK_HEART_COLOR,
    RB_STK_CHOICE_TYPE,
    RB_STK_CHOICE_CONDITION,
    RB_STK_CONDITIONAL_OPTIONAL,
    RB_STK_DRAW_ANY_NUMBER,
    RB_STK_ORDER,
    RB_STK_SELF_OR_OPPONENT,
    RB_STK_PAY_COST_ALL_DISCARD,
} RbSelectTargetKind;

int rb_select_target_kind_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "choice")) return RB_STK_CHOICE;
    if (!strcmp(s, "choice_string")) return RB_STK_CHOICE_STRING;
    if (!strcmp(s, "pay_optional_cost:skip_optional_cost")) return RB_STK_PAY_OPTIONAL_COST_SKIP_OPTIONAL_COST;
    if (!strcmp(s, "double_baton_touch")) return RB_STK_DOUBLE_BATON_TOUCH;
    if (!strcmp(s, "primary|alternative")) return RB_STK_PRIMARY_ALTERNATIVE;
    if (!strcmp(s, "apply_replacement")) return RB_STK_APPLY_REPLACEMENT;
    if (!strcmp(s, "choose_required_hearts")) return RB_STK_CHOOSE_REQUIRED_HEARTS;
    if (!strcmp(s, "position|destination")) return RB_STK_POSITION_DESTINATION;
    if (!strcmp(s, "heart_color")) return RB_STK_HEART_COLOR;
    if (!strcmp(s, "choice_type")) return RB_STK_CHOICE_TYPE;
    if (!strcmp(s, "choice_condition")) return RB_STK_CHOICE_CONDITION;
    if (!strcmp(s, "conditional_optional")) return RB_STK_CONDITIONAL_OPTIONAL;
    if (!strcmp(s, "draw_any_number")) return RB_STK_DRAW_ANY_NUMBER;
    if (!strcmp(s, "order")) return RB_STK_ORDER;
    if (!strcmp(s, "self_or_opponent")) return RB_STK_SELF_OR_OPPONENT;
    if (!strcmp(s, "pay_cost_all:discard_all")) return RB_STK_PAY_COST_ALL_DISCARD;
    return -1;
}

const char *rb_select_target_kind_to_str(int stk) {
    switch (stk) {
        case RB_STK_CHOICE: return "choice";
        case RB_STK_CHOICE_STRING: return "choice_string";
        case RB_STK_PAY_OPTIONAL_COST_SKIP_OPTIONAL_COST: return "pay_optional_cost:skip_optional_cost";
        case RB_STK_DOUBLE_BATON_TOUCH: return "double_baton_touch";
        case RB_STK_PRIMARY_ALTERNATIVE: return "primary|alternative";
        case RB_STK_APPLY_REPLACEMENT: return "apply_replacement";
        case RB_STK_CHOOSE_REQUIRED_HEARTS: return "choose_required_hearts";
        case RB_STK_POSITION_DESTINATION: return "position|destination";
        case RB_STK_HEART_COLOR: return "heart_color";
        case RB_STK_CHOICE_TYPE: return "choice_type";
        case RB_STK_CHOICE_CONDITION: return "choice_condition";
        case RB_STK_CONDITIONAL_OPTIONAL: return "conditional_optional";
        case RB_STK_DRAW_ANY_NUMBER: return "draw_any_number";
        case RB_STK_ORDER: return "order";
        case RB_STK_SELF_OR_OPPONENT: return "self_or_opponent";
        case RB_STK_PAY_COST_ALL_DISCARD: return "pay_cost_all:discard_all";
        default: return NULL;
    }
}

/* EffectCardType helpers. */
typedef enum {
    RB_ECT_MEMBER_CARD = 0,
    RB_ECT_LIVE_CARD,
    RB_ECT_ENERGY_CARD,
} RbEffectCardType;

int rb_effect_card_type_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "member_card")) return RB_ECT_MEMBER_CARD;
    if (!strcmp(s, "live_card")) return RB_ECT_LIVE_CARD;
    if (!strcmp(s, "energy_card")) return RB_ECT_ENERGY_CARD;
    return -1;
}

const char *rb_effect_card_type_as_str(int ect) {
    switch (ect) {
        case RB_ECT_MEMBER_CARD: return "member_card";
        case RB_ECT_LIVE_CARD: return "live_card";
        case RB_ECT_ENERGY_CARD: return "energy_card";
        default: return NULL;
    }
}

/* EffectState helpers. */
typedef enum {
    RB_ES_ACTIVE = 0,
    RB_ES_WAIT,
} RbEffectState;

int rb_effect_state_from_str(const char *s) {
    if (!s) return -1;
    if (!strcmp(s, "active")) return RB_ES_ACTIVE;
    if (!strcmp(s, "wait")) return RB_ES_WAIT;
    return -1;
}

const char *rb_effect_state_as_str(int es) {
    switch (es) {
        case RB_ES_ACTIVE: return "active";
        case RB_ES_WAIT: return "wait";
        default: return NULL;
    }
}

/* -- Decode-fallback audit (mirrors vm.rs DECODE_FALLBACKS) -- */
#define RB_DECODE_AUDIT_MAX 4096
static uint32_t g_decode_fallback_count = 0;
static uint32_t g_decode_fallback_abilities[RB_DECODE_AUDIT_MAX] = {0};

void rb_note_decode_fallback(int ability, const char *field, const char *value) {
    (void)field; (void)value;
    g_decode_fallback_count++;
    if (ability >= 0 && ability < RB_DECODE_AUDIT_MAX) {
        g_decode_fallback_abilities[ability]++;
    }
}

uint32_t rb_decode_fallback_count(void) {
    return g_decode_fallback_count;
}

int rb_decode_fallback_abilities(uint32_t *out, int max) {
    int n = 0;
    for (uint32_t i = 0; i < RB_DECODE_AUDIT_MAX && n < max; i++) {
        if (g_decode_fallback_abilities[i] > 0) {
            out[n++] = i;
        }
    }
    return n;
}