#include "rabuka.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

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

int rb_decode_ability(uint32_t idx, Ability *out) {
    uint32_t len;
    const unsigned char *slice = rb_bc_slice(idx, &len);
    if (!slice || len == 0) return 0;
    memset(out, 0, sizeof(*out));
    out->use_limit = -1;
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
        else if (strcmp(key, "keywords") == 0) { skip_value(&r, tag); }
        else { skip_value(&r, tag); }
    }
    (void)b;
    return 1;
}

void rb_free_ability(Ability *a) {
    if (!a) return;
    free(a->full_text); free(a->triggerless_text); free(a->triggers);
    effect_free(a->cost); effect_free(a->effect);
    memset(a, 0, sizeof(*a));
}
