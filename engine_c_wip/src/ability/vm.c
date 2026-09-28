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
int rb_zone_from_source_str(const char *s);

static char *rb_strdup(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s) + 1;
    char *p = malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

/* ── byte reader ── */
typedef struct { const unsigned char *p; const unsigned char *end; int ability; } Rdr;

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
    uint32_t len, i, key;
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
            if (!rd_idx(r, &key)) return 0;
            if (!skip_one(r)) return 0;      /* value */
        }
        return 1;
    default: return 0;
    }
}

typedef struct { char *s; size_t n, cap; } DecodeText;

static int decode_text_add(DecodeText *t, const char *s, size_t n) {
    if (n > SIZE_MAX - t->n - 1) return 0;
    size_t need = t->n + n + 1;
    if (need > t->cap) {
        size_t cap = t->cap ? t->cap : 64;
        while (cap < need) {
            if (cap > SIZE_MAX / 2) { cap = need; break; }
            cap *= 2;
        }
        char *p = realloc(t->s, cap);
        if (!p) return 0;
        t->s = p; t->cap = cap;
    }
    memcpy(t->s + t->n, s, n);
    t->n += n; t->s[t->n] = 0;
    return 1;
}

static int decode_text_quote(DecodeText *t, const char *s) {
    if (!s || !decode_text_add(t, "\"", 1)) return 0;
    for (const unsigned char *p = (const unsigned char *)s; *p; p++) {
        char buf[7];
        if (*p == '"' || *p == '\\') {
            buf[0] = '\\'; buf[1] = (char)*p;
            if (!decode_text_add(t, buf, 2)) return 0;
        } else if (*p < 0x20) {
            snprintf(buf, sizeof(buf), "\\u%04x", (unsigned)*p);
            if (!decode_text_add(t, buf, 6)) return 0;
        } else if (!decode_text_add(t, (const char *)p, 1)) return 0;
    }
    return decode_text_add(t, "\"", 1);
}

static int decode_text_value(Rdr *r, uint8_t tag, DecodeText *t) {
    uint32_t n, idx;
    uint8_t st, variant;
    int64_t v;
    char buf[64];
    switch (tag) {
    case RB_TAG_NULL: return decode_text_add(t, "null", 4);
    case RB_TAG_TRUE: return decode_text_add(t, "true", 4);
    case RB_TAG_FALSE: return decode_text_add(t, "false", 5);
    case RB_TAG_I64:
        if (!rd_int(r, &v)) return 0;
        snprintf(buf, sizeof(buf), "%lld", (long long)v);
        return decode_text_add(t, buf, strlen(buf));
    case RB_TAG_F64:
        if ((size_t)(r->end - r->p) < 8) return 0;
        if (!decode_text_add(t, "{\"$f64\":\"", 9)) return 0;
        for (int i = 0; i < 8; i++) {
            snprintf(buf, sizeof(buf), "%02x", (unsigned)r->p[i]);
            if (!decode_text_add(t, buf, 2)) return 0;
        }
        r->p += 8;
        return decode_text_add(t, "\"}", 2);
    case RB_TAG_STR:
        return rd_idx(r, &idx) && decode_text_quote(t, rb_get_string(idx));
    case RB_TAG_ARRAY:
        if (!rd_len(r, &n) || !decode_text_add(t, "[", 1)) return 0;
        for (uint32_t i = 0; i < n; i++) {
            if (i && !decode_text_add(t, ",", 1)) return 0;
            if (!rd_u8(r, &st) || !decode_text_value(r, st, t)) return 0;
        }
        return decode_text_add(t, "]", 1);
    case RB_TAG_OBJECT: case RB_TAG_OBJVAR:
        if (tag == RB_TAG_OBJVAR) {
            if (!rd_u8(r, &variant)) return 0;
            snprintf(buf, sizeof(buf), "{\"$variant\":%u,\"$fields\":", (unsigned)variant);
            if (!decode_text_add(t, buf, strlen(buf))) return 0;
        }
        if (!rd_len(r, &n) || !decode_text_add(t, "{", 1)) return 0;
        for (uint32_t i = 0; i < n; i++) {
            if (i && !decode_text_add(t, ",", 1)) return 0;
            if (!rd_idx(r, &idx) || !decode_text_quote(t, rb_get_string(idx)) ||
                !decode_text_add(t, ":", 1) || !rd_u8(r, &st) ||
                !decode_text_value(r, st, t)) return 0;
        }
        return decode_text_add(t, tag == RB_TAG_OBJVAR ? "}}" : "}", tag == RB_TAG_OBJVAR ? 2 : 1);
    default: return 0;
    }
}

static char *decode_extra_value(Rdr *r, uint8_t tag) {
    DecodeText t = {0};
    Rdr scan = *r;
    if (tag == RB_TAG_STR) {
        uint32_t idx;
        if (!rd_idx(r, &idx)) return NULL;
        return rb_strdup(rb_get_string(idx));
    }
    if (tag == RB_TAG_ARRAY) {
        uint32_t n;
        int csv = 1;
        if (!rd_len(&scan, &n)) return NULL;
        for (uint32_t i = 0; i < n; i++) {
            uint8_t st;
            if (!rd_u8(&scan, &st)) { free(t.s); return NULL; }
            if (st != RB_TAG_STR && st != RB_TAG_I64) { csv = 0; break; }
            char *s = decode_extra_value(&scan, st);
            if (!s) { free(t.s); return NULL; }
            int ok = !i || decode_text_add(&t, ",", 1);
            int quote = !*s || strpbrk(s, ",\"\r\n") != NULL;
            if (ok && quote) ok = decode_text_add(&t, "\"", 1);
            for (const char *p = s; ok && *p; p++) {
                if (*p == '"') ok = decode_text_add(&t, "\"", 1);
                if (ok) ok = decode_text_add(&t, p, 1);
            }
            if (ok && quote) ok = decode_text_add(&t, "\"", 1);
            free(s);
            if (!ok) { free(t.s); return NULL; }
        }
        if (csv) {
            *r = scan;
            return t.s ? t.s : rb_strdup("");
        }
        free(t.s); memset(&t, 0, sizeof(t));
    }
    if (!decode_text_value(r, tag, &t)) { free(t.s); return NULL; }
    return t.s;
}

/* ── condition tree ── */
static Condition *read_condition(Rdr *r);          /* fwd */
static Condition *read_condition_fields(Rdr *r, uint8_t variant);
static CondValue read_cond_value(Rdr *r, uint8_t tag); /* fwd */
static void cond_value_free(CondValue *v);          /* fwd */
static CondValue cond_value_null(void) {
    CondValue v; memset(&v, 0, sizeof(v));
    v.tag = RB_TAG_NULL; return v;
}
static void cond_value_free(CondValue *v) {
    if (!v) return;
    if (v->tag == RB_TAG_STR || v->tag == RB_TAG_F64) free(v->s);
    else if (v->tag == RB_TAG_OBJVAR || v->tag == RB_TAG_OBJECT) rb_free_condition(v->cond);
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
    case RB_TAG_I64:
        if (!rd_int(r, &v.i)) goto fail;
        return v;
    case RB_TAG_F64:
        v.s = decode_extra_value(r, tag);
        if (!v.s) goto fail;
        return v;
    case RB_TAG_STR: {
        uint32_t idx;
        if (!rd_idx(r, &idx) || !(v.s = rb_strdup(rb_get_string(idx)))) goto fail;
        return v;
    }
    case RB_TAG_ARRAY: {
        uint32_t n;
        if (!rd_len(r, &n) || n > (size_t)(r->end - r->p)) goto fail;
        if (n && !(v.arr = calloc(n, sizeof(*v.arr)))) goto fail;
        for (uint32_t j = 0; j < n; j++) {
            uint8_t st;
            if (!rd_u8(r, &st)) goto fail;
            v.arr[j] = read_cond_value(r, st);
            v.arr_n++;
            if (v.arr[j].tag == 0xFF) goto fail;
        }
        return v;
    }
    case RB_TAG_OBJECT:
        v.cond = read_condition_fields(r, 0xFF);
        if (!v.cond) goto fail;
        return v;
    case RB_TAG_OBJVAR:
        v.cond = read_condition(r);
        if (!v.cond) goto fail;
        return v;
    default: break;
    }
fail:
    cond_value_free(&v);
    v = cond_value_null();
    v.tag = 0xFF;
    return v;
}

/* ── positions_characters (engine/src/ability/vm.rs:621-661,
      BcReader::read_positions_characters_value) ──
   A LIST of {character, position} pairs, flattened to one string per entry,
   {"position":"<p>","character":"<c>"}.

   This is NOT the generic read_cond_value() shape. That arm decodes each
   element as a nested object into CondValue.cond, while
   stage_satisfies_positioned_characters (condition.c) pulls the two fields
   back out of a FLAT STRING and rejects anything whose tag is not
   RB_TAG_STR. So the generic shape made every entry unreadable and the gate
   rejected PL!HS-bp2-026-L no matter how the board was arranged. Flattening
   happens here, at decode time, exactly as the direct condition decoder's
   "positions_characters" arm does. */
static int read_positions_characters_value(Rdr *r, CondValue *out) {
    uint32_t n, i;
    if (!rd_len(r, &n)) return 0;
    out->tag = RB_TAG_ARRAY;
    out->arr_n = n;
    out->arr = calloc(n ? n : 1, sizeof(CondValue));
    if (!out->arr) return 0;
    for (i = 0; i < n; i++) {
        uint8_t etag;
        if (!rd_u8(r, &etag)) return 0;
        if (etag != RB_TAG_OBJECT && etag != RB_TAG_OBJVAR) {
            /* Rust drops the element (vm.rs:633-634). The slot is kept and left
               NULL instead: dropping would shrink the array, and a half-decoded
               list must not satisfy the all-entries conjunction by having
               fewer entries left to fail. The evaluator rejects a NULL entry,
               so this is the fail-closed direction. */
            if (!skip_value(r, etag)) return 0;
            continue;
        }
        if (etag == RB_TAG_OBJVAR) { uint8_t vb; if (!rd_u8(r, &vb)) return 0; }
        uint32_t ocount;
        if (!rd_len(r, &ocount)) return 0;
        char buf[512];
        size_t blen = 0;
        int emitted = 0, overflow = 0;
        buf[0] = 0;
        for (uint32_t j = 0; j < ocount; j++) {
            uint32_t kidx;
            uint8_t vtag;
            if (!rd_idx(r, &kidx) || !rd_u8(r, &vtag)) return 0;
            const char *kstr = rb_get_string(kidx);
            int is_pos = kstr && !strcmp(kstr, "position");
            int is_chr = kstr && !strcmp(kstr, "character");
            if (!is_pos && !is_chr) { if (!skip_value(r, vtag)) return 0; continue; }
            const char *val = NULL;
            if (vtag == RB_TAG_STR) {
                uint32_t sidx;
                if (!rd_idx(r, &sidx)) return 0;
                val = rb_get_string(sidx);
            } else if (vtag == RB_TAG_NULL) {
                val = "";    /* Rust: read_string_value().unwrap_or_default() */
            } else {
                /* Rust's read_string_value (vm.rs:398-408) returns None for any
                   tag other than NULL/STR WITHOUT consuming the payload, which
                   would leave this reader mid-value and corrupt every field after
                   this object. Consume it here so the stream stays aligned. */
                if (!skip_value(r, vtag)) return 0;
            }
            if (!val) val = "";
            if (!overflow) {
                int w = snprintf(buf + blen, sizeof(buf) - blen,
                                 "%s\"%s\":\"%s\"", emitted ? "," : "",
                                 is_pos ? "position" : "character", val);
                if (w < 0 || (size_t)w >= sizeof(buf) - blen) { buf[blen] = 0; overflow = 1; }
                else blen += (size_t)w;
            }
            emitted++;
        }
        /* The separator counts the pairs actually written, not the raw field
           index, so an unknown key ahead of "position" cannot push a leading
           comma into the flattened text. A pair too large to flatten is stored
           NULL rather than truncated: a truncated pair would still parse as a
           real, wrong requirement, and the evaluator rejects NULL instead. */
        if (!overflow) {
            char *s = malloc(strlen(buf) + 3);
            if (!s) return 0;
            s[0] = '{';
            strcpy(s + 1, buf);
            strcat(s, "}");
            out->arr[i].tag = RB_TAG_STR;
            out->arr[i].s = s;
        }
    }
    return 1;
}

static Condition *read_condition_fields(Rdr *r, uint8_t variant) {
    uint32_t count;
    if (!rd_len(r, &count) || count > RB_MAX_COND_FIELD) return NULL;
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = variant;
    for (uint32_t i = 0; i < count; i++) {
        uint32_t kidx;
        uint8_t tag;
        if (!rd_idx(r, &kidx) || !rd_u8(r, &tag)) goto fail;
        CondField *f = &c->fields[c->n_fields++];
        f->key = rb_strdup(rb_get_string(kidx));
        if (!f->key) goto fail;
        if (!strcmp(f->key, "positions_characters") && tag == RB_TAG_ARRAY) {
            if (!read_positions_characters_value(r, &f->v)) goto fail;
        } else {
            f->v = read_cond_value(r, tag);
            if (f->v.tag == 0xFF) goto fail;
        }
    }
    return c;
fail:
    rb_free_condition(c);
    return NULL;
}

/* Mirror Condition's recursive values without interpreting nested effect variants. */
static Condition *read_condition(Rdr *r) {
    uint8_t variant;
    if (!rd_u8(r, &variant)) return NULL;
    return read_condition_fields(r, variant);
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
    for (int i = 0; i < e->n_options; i++) effect_free(e->options[i]);
    for (int i = 0; i < e->n_extra; i++) { free(e->extra_k[i]); free(e->extra_v[i]); }
    effect_free(e->primary_effect);
    effect_free(e->alternative_effect);
    effect_free(e->look_action);
    effect_free(e->select_action);
    effect_free(e->followup_action);
    effect_free(e->optional_action);
    effect_free(e->conditional_action);
    effect_free(e->gained_effect);
    effect_free(e->resource_on_select);
    effect_free(e->opponent_action);
    rb_free_condition(e->result_condition);
    rb_free_condition(e->alternative_condition);
    rb_free_condition(e->activation_condition);
    free(e);
}
static int effect_add_child(AbilityEffect *e, AbilityEffect *c) {
    if (!e || !c || e->n_child >= RB_MAX_CHILD) { effect_free(c); return 0; }
    e->child[e->n_child++] = c; return 1;
}
static int effect_set_extra(AbilityEffect *e, const char *k, const char *v) {
    if (!e || !k) return 0;
    int i;
    for (i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) break;
    if (i == RB_MAX_EXTRA) return 0;
    char *value = v ? rb_strdup(v) : NULL;
    if (v && !value) return 0;
    if (i == e->n_extra) {
        char *key = rb_strdup(k);
        if (!key) { free(value); return 0; }
        e->extra_k[i] = key;
        e->n_extra++;
    } else free(e->extra_v[i]);
    e->extra_v[i] = value;
    return 1;
}
static void effect_decode_dynamic_count(AbilityEffect *e, Rdr *r)
{
    Condition *dynamic = read_condition(r);
    if (!dynamic) return;
    for (uint32_t i = 0; i < dynamic->n_fields; i++) {
        const CondField *field = &dynamic->fields[i];
        char number[32];
        const char *value = NULL;
        if (field->v.tag == RB_TAG_STR || field->v.tag == RB_TAG_F64)
            value = field->v.s;
        else if (field->v.tag == RB_TAG_I64) {
            snprintf(number, sizeof(number), "%lld", (long long)field->v.i);
            value = number;
        } else if (field->v.tag == RB_TAG_TRUE)
            value = "true";
        else if (field->v.tag == RB_TAG_FALSE)
            value = "false";
        if (field->key && value) effect_set_extra(e, field->key, value);
    }
    rb_free_condition(dynamic);
}

static int effect_decode_extra(AbilityEffect *e, const char *key, Rdr *r, uint8_t tag) {
    char *value = decode_extra_value(r, tag);
    if (!value) return 0;
    int ok = effect_set_extra(e, key, value);
    free(value);
    return ok;
}

/* decode a single effect from current cursor (assumes TAG_OBJVAR already read).
    Rust: TAG_OBJECT_VARIANT (0x09) + variant u8 + len + fields.
    The variant selects EffectKind but C stores only action string; we consume
    the byte and ignore it. Mirrors vm.rs:decode_ability_effect_direct . */
/* ---- TEMPORARY PROBE INSTRUMENTATION (removed before hand-off) ---- */
uint32_t g_probe_seen = 0, g_probe_retained = 0, g_probe_effects = 0;
static char g_probe_dropped[4096][48];
static uint8_t g_probe_dropped_tag[4096];
static uint32_t g_probe_ndropped = 0;
static char g_probe_keys[1024][48];
static uint32_t g_probe_keycount[1024];
static uint32_t g_probe_nkeys = 0;
static void probe_seen(const char *k) {
    g_probe_seen++;
    if (k) for (uint32_t i = 0; i < g_probe_nkeys; i++)
        if (!strcmp(g_probe_keys[i], k)) { g_probe_keycount[i]++; return; }
    if (g_probe_nkeys < 1024) {
        snprintf(g_probe_keys[g_probe_nkeys], 48, "%s", k);
        g_probe_keycount[g_probe_nkeys] = 1;
        g_probe_nkeys++;
    }
}
uint32_t rb_probe_nkeys(void) { return g_probe_nkeys; }
const char *rb_probe_key(uint32_t i) { return i < g_probe_nkeys ? g_probe_keys[i] : NULL; }
uint32_t rb_probe_keycount(uint32_t i) { return i < g_probe_nkeys ? g_probe_keycount[i] : 0; }
static void probe_keep(void) { g_probe_retained++; }
static void probe_drop(const char *k, uint8_t t) {
    if (g_probe_ndropped < 4096) {
        snprintf(g_probe_dropped[g_probe_ndropped], 48, "%s", k ? k : "<null>");
        g_probe_dropped_tag[g_probe_ndropped] = t;
        g_probe_ndropped++;
    }
}
void rb_probe_reset(void) { g_probe_seen = g_probe_retained = g_probe_effects =
    g_probe_ndropped = 0; g_probe_nkeys = 0; }
uint32_t rb_probe_ndropped_keys(void) { return g_probe_ndropped; }
const char *rb_probe_dropped_key(uint32_t i) { return i < g_probe_ndropped ? g_probe_dropped[i] : NULL; }
uint8_t rb_probe_dropped_tag(uint32_t i) { return i < g_probe_ndropped ? g_probe_dropped_tag[i] : 0; }
/* ---- END TEMPORARY PROBE ---- */

/* Name of a decoded Condition variant (RB_COND_*). Used to label the
   activation-condition marker below; Condition has no wire `type` string of its
   own in the C tree, only the variant byte. */
static const char *cond_variant_name(int v) {
    static const char *names[] = {
        "compound_condition",   "location_condition",   "comparison_condition",
        "movement_condition",   "group_condition",      "appearance_condition",
        "temporal_condition",   "state_condition",      "resource_condition",
        "ability_filter_condition", "score_threshold_condition",
        "choice_condition",     "complex_condition",    "position_condition",
        "opponent_choice_condition", "opponent_live_success",
        "no_excess_heart",      "always_true",          "any_of_condition",
        "all_revealed_match_heart_color"
    };
    int n = (int)(sizeof(names) / sizeof(names[0]));
    return (v >= 0 && v < n) ? names[v] : "unsupported_condition";
}

/* One string field off a decoded Condition tree. condition.c has a static
   get_str() for the same purpose but it is not visible from this file. */
static const char *cond_field_str(const Condition *c, const char *key) {
    if (!c || !key) return NULL;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondField *f = &c->fields[i];
        if (f->key && !strcmp(f->key, key) &&
            (f->v.tag == RB_TAG_STR || f->v.tag == RB_TAG_F64))
            return f->v.s;
    }
    return NULL;
}

static AbilityEffect *decode_effect_value(Rdr *r, uint8_t tag);
static AbilityEffect *decode_effect_body(Rdr *r) {
    uint8_t variant;
    if (!rd_u8(r, &variant)) return NULL;
    uint32_t count, i;
    if (!rd_len(r, &count)) return NULL;
    AbilityEffect *e = effect_new();
    if (!e) return NULL;
    g_probe_effects++;
    for (i = 0; i < count; i++) {
        uint32_t kidx; if (!rd_idx(r, &kidx)) goto fail;
        const char *key = rb_get_string(kidx);
        uint8_t tag; if (!rd_u8(r, &tag)) goto fail;
        probe_seen(key);

        if (key && strcmp(key, "text") == 0) {
            free(e->text); e->text = rd_string_val(r, tag); probe_keep(); continue;
        }
        if (key && (!strcmp(key, "action") || !strcmp(key, "type") ||
                    !strcmp(key, "cost_type"))) {
            free(e->action); e->action = rd_string_val(r, tag); probe_keep(); continue;
        }
        if (key && (!strcmp(key, "source") || !strcmp(key, "destination") ||
                    !strcmp(key, "target") || !strcmp(key, "zone"))) {
            char *s = rd_string_val(r, tag);
            if (s && (strcmp(key, "target") != 0) &&
                s && rb_zone_from_source_str(s) == RB_ZONEID_UNKNOWN) {
                rb_note_decode_fallback(r->ability, key, s);
            }
            if (!strcmp(key, "source") || !strcmp(key, "zone")) { free(e->source); e->source = s; }
            else if (!strcmp(key, "destination")) { free(e->destination); e->destination = s; }
            else { free(e->target); e->target = s; }
            probe_keep();
            continue;
        }
        if (key && strcmp(key, "count") == 0) {
            if (tag == RB_TAG_I64) { int64_t v; if (rd_int(r, &v)) { e->count = (int)v; probe_keep(); } }
            else { skip_value(r, tag); probe_drop(key, tag); }
            continue;
        }
        if (key && (strcmp(key, "condition") == 0)) {
            e->has_condition = 1;
            if (tag == RB_TAG_OBJVAR) { e->condition = read_condition(r); if (e->condition) probe_keep(); else probe_drop(key, tag); }
            else { skip_value(r, tag); probe_drop(key, tag); }
            continue;
        }
        /* activation_condition_parsed — effect_decoder_gen.rs:159,
           "activation_condition_parsed" => ek.activation_condition_parsed =
           bc.read_condition_value(), i.e. Option<Box<Condition>>: TAG_NULL ->
           None, TAG_OBJECT_VARIANT -> the tree, anything else -> None
           (vm.rs:555-565). It is a *separate* gate from `condition`, checked on
           its own in resolver.rs:456-494 and required in ADDITION to `condition`.

           13 abilities in cards/abilities.json carry it, and it is the only
           gate on all of them: a location_condition pinning the host to
           left_side/right_side/center, an appearance_condition with
           activation_position "left_side,right_side", 「このカードが控え室にある場合」
           (discard), 「このカードが手札にある場合」 (hand), and two
           success_live_card_zone score >= 6 conditions. Before this branch the
           key fell through to the scalar tail at the bottom of the loop, which
           skip_value()d the whole nested object and retained nothing — so
           rb_can_activate_effect's activation-condition branch (resolver.c:596)
           and rb_resolver_needs_gate's activation term (resolver.c:891) were
           dead: `needs_gate` was always `condition != NULL`, and an ability
           with only an activation condition was never gated at all.

           The tree is decoded for real (read_condition, so a malformed
           activation condition is caught here exactly as a malformed `condition`
           is) and RETAINED in AbilityEffect::activation_condition (rabuka.h,
           beside result_condition), so rb_can_activate_effect can evaluate the
           actual gate instead of a presence marker. The "true:<variant>:..."
           extra is still recorded under the SAME wire key: it is the accessor
           needs_gate in rb_resolve_ability looks up, and it folds
           location/position into the value so a debug dump identifies the gate.

           effect_free (above) releases the retained tree, so it is not leaked. */
        if (key && strcmp(key, "activation_condition_parsed") == 0) {
            if (tag == RB_TAG_OBJVAR) {
                Condition *c = read_condition(r);
                if (c) {
                    /* Extras in this decoder are "true"/"false" strings, so the
                       marker follows that convention; the variant is folded in
                       so the gate is identifiable in a debug dump. */
                    char marker[80];
                    const char *loc = cond_field_str(c, "location");
                    const char *pos = cond_field_str(c, "position");
                    if (loc || pos)
                        snprintf(marker, sizeof(marker), "true:%s:%s%s",
                                 cond_variant_name(c->variant),
                                 loc ? loc : "-", pos ? pos : "");
                    else
                        snprintf(marker, sizeof(marker), "true:%s",
                                 cond_variant_name(c->variant));
                    effect_set_extra(e, key, marker);
                    /* Retain the decoded tree itself; the marker above is a
                       convenience, not the gate. Guard the (theoretically
                       impossible) duplicate key so nothing leaks. */
                    if (e->activation_condition) rb_free_condition(e->activation_condition);
                    e->activation_condition = c;
                    probe_keep();
                } else probe_drop(key, tag);
            } else { skip_value(r, tag); probe_drop(key, tag); }
            continue;
        }
        if (key && !strcmp(key, "dynamic_count") && tag == RB_TAG_OBJVAR) {
            effect_decode_dynamic_count(e, r);
            probe_keep();
            continue;
        }
        /* Boolean wire fields. The wire encodes all six as BOOL
           (engine/src/ability/effect_decoder_gen.rs:55-59,63,73 read_bool_value:
           TAG_NULL -> None, TAG_TRUE -> Some(true), TAG_FALSE -> Some(false)).
           Previously this branch only ever *set* a struct flag and called
           skip_value(), so the key/value pair was thrown away: `non_stackable` was
           lost outright (core/card.c:626 rb_effect_non_stackable_any reads the extra
           and therefore always returned "no"), `optional` was lost outright
           (core/modifiers.c:463 cost_extra_flag(e,"optional") was always 0), and
           `max`/`conditional`/`conditional_negation`/`is_further` were lost whenever
           the value was false. The pair is now retained as an extra as well as
           driving the struct flag, so both the struct-field readers and the
           extras-table readers see the same truth. */
        if (key && (!strcmp(key, "optional") || !strcmp(key, "non_stackable") ||
                    !strcmp(key, "conditional") || !strcmp(key, "conditional_negation") ||
                    !strcmp(key, "is_further") || !strcmp(key, "max"))) {
            const char *sv = NULL;
            char nbuf[24];
            int b = 0;
            if (tag == RB_TAG_TRUE) { b = 1; sv = "true"; }
            else if (tag == RB_TAG_FALSE) { b = 0; sv = "false"; }
            else if (tag == RB_TAG_I64) {
                int64_t v;
                if (!rd_int(r, &v)) goto fail;
                b = (v != 0);
                snprintf(nbuf, sizeof(nbuf), "%lld", (long long)v);
                sv = nbuf;
            } else { skip_value(r, tag); probe_drop(key, tag); continue; }
            if (!strcmp(key, "optional") && b) e->is_optional = 1;
            if (!strcmp(key, "conditional") && b) e->conditional_flag = 1;
            if (!strcmp(key, "conditional_negation") && b) e->conditional_negation = 1;
            if (!strcmp(key, "is_further") && b) e->is_further = 1;
            effect_set_extra(e, key, sv);
            probe_keep();
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
                if (c) probe_keep(); else probe_drop(key, tag);
            } else { skip_value(r, tag); probe_drop(key, tag); }
            continue;
        }
        /* nested effect(s) */
        if (key && (!strcmp(key, "actions") || !strcmp(key, "effect_steps") ||
                    !strcmp(key, "costs"))) {
            if (tag == RB_TAG_ARRAY) {
                uint32_t n; if (rd_len(r, &n)) {
                    for (uint32_t j = 0; j < n; j++) {
                        uint8_t st; if (!rd_u8(r, &st)) goto fail;
                        if (st == RB_TAG_OBJVAR) {
                            AbilityEffect *c = decode_effect_body(r);
                            if (!c || !effect_add_child(e, c)) goto fail;
                        } else skip_value(r, st);
                    }
                    probe_keep();
                }
            } else { skip_value(r, tag); probe_drop(key, tag); }
            continue;
        }
        if (key && strcmp(key, "options") == 0) {
            if (tag == RB_TAG_ARRAY) {
                uint32_t n; if (rd_len(r, &n)) {
                    for (uint32_t j = 0; j < n && e->n_options < RB_MAX_CHILD; j++) {
                        uint8_t st; if (!rd_u8(r, &st)) break;
                        if (st == RB_TAG_OBJVAR) {
                            AbilityEffect *c = decode_effect_body(r);
                            if (c) e->options[e->n_options++] = c;
                        } else skip_value(r, st);
                    }
                    probe_keep();
                }
            } else { skip_value(r, tag); probe_drop(key, tag); }
            continue;
        }
        if (key && (strcmp(key, "look_action") == 0 || strcmp(key, "select_action") == 0)) {
            if (tag == RB_TAG_OBJVAR) {
                AbilityEffect *c = decode_effect_body(r);
                if (c) {
                    if (!strcmp(key, "look_action")) {
                        effect_free(e->look_action);
                        e->look_action = c;
                    } else {
                        effect_free(e->select_action);
                        e->select_action = c;
                    }
                }
                if (e->look_action || e->select_action) probe_keep(); else probe_drop(key, tag);
            } else { skip_value(r, tag); probe_drop(key, tag); }
            continue;
        }
        /* compound sub-effects (mirror AbilityEffect::compound primary/alternative/
            followup/optional/conditional). Decoded into dedicated fields (NOT child[])
            so branch ordering is unambiguous and the pre-order walk in rb_execute_effect_ex
            does not double-execute them. */
        if (key && (!strcmp(key, "primary_effect") ||
                    !strcmp(key, "alternative_effect") ||
                    !strcmp(key, "followup_action") ||
                    !strcmp(key, "optional_action") ||
                    !strcmp(key, "conditional_action") ||
                    !strcmp(key, "resource_on_select") ||
                    !strcmp(key, "opponent_action"))) {
            if (tag == RB_TAG_OBJVAR) {
                AbilityEffect *c = decode_effect_body(r);
                if (c) {
                    if (!strcmp(key, "primary_effect")) { effect_free(e->primary_effect); e->primary_effect = c; }
                    else if (!strcmp(key, "alternative_effect")) { effect_free(e->alternative_effect); e->alternative_effect = c; }
                    else if (!strcmp(key, "followup_action")) { effect_free(e->followup_action); e->followup_action = c; }
                    else if (!strcmp(key, "optional_action")) { effect_free(e->optional_action); e->optional_action = c; }
                    else if (!strcmp(key, "conditional_action")) { effect_free(e->conditional_action); e->conditional_action = c; }
                    else if (!strcmp(key, "resource_on_select")) { effect_free(e->resource_on_select); e->resource_on_select = c; }
                    else { effect_free(e->opponent_action); e->opponent_action = c; }
                }
                if (c) probe_keep(); else probe_drop(key, tag);
            } else { skip_value(r, tag); probe_drop(key, tag); }
            continue;
        }
        if (key && strcmp(key, "gained_effect") == 0) {
            e->gained_effect = decode_effect_value(r, tag);
            if (e->gained_effect || tag == RB_TAG_NULL) probe_keep(); else probe_drop(key, tag);
            continue;
        }
        /* compound scalar fields mirrored from AbilityEffect::compound / root. These
           are also retained as extras (below) for callers that read them as strings. */
        if (key && !strcmp(key, "repeat_limit") && tag == RB_TAG_I64) {
            int64_t v; if (rd_int(r, &v)) { e->repeat_limit = (int)v; probe_keep(); }
            continue;
        }
        if (key && !strcmp(key, "per_unit_count") && tag == RB_TAG_I64) {
            int64_t v; if (rd_int(r, &v)) { e->per_unit_count = (int)v; probe_keep(); }
            continue;
        }
        if (key && !strcmp(key, "cost_reduction_per_group") && tag == RB_TAG_I64) {
            int64_t v; if (rd_int(r, &v)) { e->cost_reduction_per_group = (int)v; probe_keep(); }
            continue;
        }
        if (key && !strcmp(key, "id") && tag == RB_TAG_STR) {
            uint32_t idx; if (rd_idx(r, &idx)) { const char *s = rb_get_string(idx); if (s) { strncpy(e->id_field, s, 31); e->id_field[31]=0; probe_keep(); } }
            continue;
        }
        if (key && (!strcmp(key, "self_target") || !strcmp(key, "card_type")) && tag == RB_TAG_STR) {
            uint32_t idx; if (rd_idx(r, &idx)) { const char *s = rb_get_string(idx); if (s) {
                if (!strcmp(key,"self_target")) { strncpy(e->self_target_field, s, 7); e->self_target_field[7]=0; }
                else { strncpy(e->card_type_field, s, 23); e->card_type_field[23]=0; }
                probe_keep();
            } }
            continue;
        }
        if (key && !strcmp(key, "self_target") &&
            (tag == RB_TAG_TRUE || tag == RB_TAG_FALSE)) {
            /* effect_decoder_gen.rs:79 reads self_target with read_bool_value(), so
               the wire carries TRUE/FALSE, not a string. The struct field used to be
               filled only for TAG_STR and was therefore dead for all card data. */
            const char *sv = tag == RB_TAG_TRUE ? "true" : "false";
            strncpy(e->self_target_field, sv, 7); e->self_target_field[7] = 0;
            effect_set_extra(e, key, sv);
            probe_keep();
            continue;
        }
        /* `per_unit` is BOOL (effect_decoder_gen.rs:89 read_bool_value) and `distinct`
           is BOOL-or-STR (vm.rs:1051 read_distinct_value: NULL -> None,
           FALSE -> DistinctType::CardName, TRUE -> DistinctType::True,
           STR -> "card_name" | "true" | "distinct" enable the filter).
           The old code required RB_TAG_I64, so AbilityEffect.per_unit and
           AbilityEffect.distinct_flag were 0 for every effect in the corpus. The raw
           value is still retained as an extra so the extras-table readers
           (cost.c:cr_per_unit, compound.c:eff_distinct_any, dynamic_count.c,
           resolver.c, core/modifiers.c) keep seeing it unchanged. */
        if (key && (!strcmp(key, "per_unit") || !strcmp(key, "distinct"))) {
            int is_per_unit = !strcmp(key, "per_unit");
            const char *sv = NULL;
            char nbuf[24];
            int flag = 0, have = 1;
            if (tag == RB_TAG_TRUE) { flag = 1; sv = "true"; }
            else if (tag == RB_TAG_FALSE) { flag = 0; sv = "false"; }
            else if (tag == RB_TAG_NULL) { have = 0; }
            else if (tag == RB_TAG_I64) {
                int64_t v;
                if (!rd_int(r, &v)) goto fail;
                flag = (int)v;
                snprintf(nbuf, sizeof(nbuf), "%lld", (long long)v);
                sv = nbuf;
            } else if (tag == RB_TAG_STR) {
                uint32_t idx;
                if (!rd_idx(r, &idx)) goto fail;
                const char *s = rb_get_string(idx);
                sv = s ? s : "";
                flag = (!strcmp(sv, "card_name") || !strcmp(sv, "true") ||
                        !strcmp(sv, "distinct"));
            } else { skip_value(r, tag); probe_drop(key, tag); continue; }
            if (have) { if (is_per_unit) e->per_unit = flag; else e->distinct_flag = flag; }
            if (sv) effect_set_extra(e, key, sv);
            if (sv) probe_keep(); else probe_drop(key, tag);
            continue;
        }
        /* scalar extras (stringify) — also handle heart_colors array */
        if (tag == RB_TAG_STR) {
            uint32_t idx; if (rd_idx(r, &idx)) { effect_set_extra(e, key, rb_get_string(idx)); probe_keep(); } else probe_drop(key, tag);
        } else if (tag == RB_TAG_I64) {
            int64_t v; if (rd_int(r, &v)) { char buf[24]; snprintf(buf,sizeof(buf),"%lld",(long long)v); effect_set_extra(e, key, buf); probe_keep(); } else probe_drop(key, tag);
        } else if (tag == RB_TAG_TRUE) {
            effect_set_extra(e, key, "true"); probe_keep();
        } else if (tag == RB_TAG_FALSE) {
            effect_set_extra(e, key, "false"); probe_keep();
        } else if (tag == RB_TAG_ARRAY) {
            char *value = decode_extra_value(r, tag);
            if (!value) return NULL;
            int ok = effect_set_extra(e, key, value);
            if (ok && key && !strcmp(key, "heart_colors") && !strchr(value, ',')) {
                ok = effect_set_extra(e, "heart_color", value);
            }
            free(value);
            if (ok) probe_keep(); else probe_drop(key, tag);
            if (!ok) goto fail;
        } else {
            skip_value(r, tag);
            probe_drop(key, tag);
        }
    }
    return e;

fail:
    effect_free(e);
    return NULL;
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

static int decode_keywords_value(Rdr *r, uint8_t tag, unsigned char *out,
                                 int cap, int *count, int *present) {
    if (!count) return 0;
    *count = 0;
    if (present) *present = 0;
    if (tag == RB_TAG_NULL) return 1;
    if (tag != RB_TAG_ARRAY || !out || cap <= 0) return 0;
    uint32_t n;
    if (!rd_len(r, &n)) return 0;
    if (present) *present = 1;
    for (uint32_t i = 0; i < n; i++) {
        uint8_t st;
        if (!rd_u8(r, &st) || st != RB_TAG_STR) return 0;
        uint32_t idx;
        if (!rd_idx(r, &idx)) return 0;
        const char *s = rb_get_string(idx);
        if (!s) return 0;
        RbKeyword kw = rb_keyword_from_str(s);
        if (kw == RB_KW_COUNT) {
            rb_note_decode_fallback(r->ability, "keyword", s);
        } else if (*count < cap) {
            out[(*count)++] = (unsigned char)kw;
        }
    }
    return 1;
}

int rb_decode_keywords(const unsigned char *arr, uint32_t arr_len, RbKeyword *out, int max) {
    if (!arr || arr_len == 0 || !out || max <= 0) return 0;
    Rdr r = { arr, arr + arr_len, -1 };
    uint8_t tag;
    int count, present;
    if (!rd_u8(&r, &tag) ||
        !decode_keywords_value(&r, tag, (unsigned char *)out, max, &count, &present)) {
        return 0;
    }
    return present ? count : 0;
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
    Rdr r = { slice, slice + len, (int)idx };
    uint8_t tag;
    if (!rd_u8(&r, &tag) || tag != RB_TAG_OBJECT) return 0;
    uint32_t count;
    if (!rd_len(&r, &count)) return 0;
    for (uint32_t i = 0; i < count; i++) {
        uint32_t kidx; if (!rd_idx(&r, &kidx)) return 0;
        const char *key = rb_get_string(kidx);
        if (!rd_u8(&r, &tag)) return 0;
        if (key && strcmp(key, "full_text") == 0) { out->full_text = rd_string_val(&r, tag); }
        else if (key && strcmp(key, "triggerless_text") == 0) { out->triggerless_text = rd_string_val(&r, tag); }
        else if (key && strcmp(key, "triggers") == 0) { out->triggers = rd_string_val(&r, tag); }
        else if (key && strcmp(key, "use_limit") == 0) {
            if (tag == RB_TAG_I64) { int64_t v; if (rd_int(&r, &v)) out->use_limit = (int)v; } else skip_value(&r, tag);
        }
        else if (key && strcmp(key, "is_null") == 0) {
            if (tag == RB_TAG_TRUE) out->is_null = 1; else if (tag == RB_TAG_FALSE) out->is_null = 0; else skip_value(&r, tag);
        }
        else if (key && strcmp(key, "cost") == 0) { out->cost = decode_effect_value(&r, tag); }
        else if (key && strcmp(key, "effect") == 0) { out->effect = decode_effect_value(&r, tag); }
        else if (key && strcmp(key, "keywords") == 0) {
            if (!decode_keywords_value(&r, tag, out->keywords, RB_MAX_ABILITY_KEYWORDS,
                                       &out->n_keywords, &out->has_keywords)) {
                rb_free_ability(out);
                memset(out, 0, sizeof(*out));
                out->use_limit = -1;
                return 0;
            }
        }
        else { skip_value(&r, tag); }
    }
    return 1;
}

/* ── get_ability wrapper (mirrors vm.rs get_ability) ──
   Returns 1 on success, 0 on decode failure. For out-of-range idx or empty
   slice, returns 1 with a default Ability (matching Rust's Ok(Ability::default())).
   Only returns 0 when the bytecode is present but structurally invalid. */
int rb_get_ability(uint32_t idx, Ability *out) {
    if (!out) return 0;
    if (idx >= RBKA_NUM_ABILITIES) {
        memset(out, 0, sizeof(*out));
        out->use_limit = -1;
        return 0;
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
    char *action_reference;
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
    int shuffle; int has_shuffle;
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
    if (l->action_reference) cond_add_str(c, "action_reference", l->action_reference);
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
    if (l->has_shuffle) cond_add_bool(c, "shuffle", l->shuffle);
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

/* build_abilityfilter: variant 9 — ability_filter_condition
   condition_decoder_gen.rs:1727 maps variant 9 to build_abilityfilter, and
   card.rs `AbilityFilter { common, ability_filter }` adds exactly one field
   beyond ConditionCommon — `ability_filter`, which cond_copy_common already
   carries. The evaluator `eval_ability_filter` (condition.c:1604) was already
   present and unreachable: the decoder used to return NULL here, so the whole
   condition was silently dropped and the gate became vacuously true. */
Condition *build_abilityfilter(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_ABILITY_FILTER;
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

/* build_positioncond: variant 13 — position_condition
   condition_decoder_gen.rs:1731 maps variant 13 to build_positioncond, and
   card.rs `PositionCond { common }` has no field beyond ConditionCommon.
   `eval_position` (condition.c:1608) already existed but was unreachable. */
Condition *build_positioncond(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = RB_COND_POSITION;
    cond_copy_common(c, l);
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

/* resolver.rs:403 and :516 — `cond.get_positions_characters().is_none()`.
   Returns 1 when the condition carries the field, 0 when it does not. Rust's
   `is_none()` is false for a present-but-empty list, but the decoder only
   emits the field when it holds at least one entry (cond_add_str_array is
   gated on n > 0, as it is for every other list field here), and an empty
   positions_characters states no requirement, so the two agree.

   Defined in this file rather than beside its siblings in src/core/card.c
   because that file belongs to another agent; the forward declaration sits in
   resolver.c with the other rb_condition_get_* prototypes. */
int rb_condition_get_positions_characters(const Condition *c) {
    if (!c) return 0;
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, "positions_characters"))
            return c->fields[i].v.tag != RB_TAG_NULL;
    return 0;
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
    /* action_reference (condition_decoder_gen.rs:127, read_arc_str_value) is a
       ConditionCommon field, so every variant receives it. It is not inert:
       compound/conditional_on.rs:50-54 uses it to test `resolver.last_action_result`
       instead of evaluating the condition. Decoding it into the condition is the
       decoder's half; compound.c:487-488 is the consumer half and still ignores it. */
    if (strcmp(key, "action_reference") == 0) {
        if (tag == RB_TAG_STR) { uint32_t idx; if (!rd_idx(r, &idx)) return 0; l->action_reference = rb_strdup(rb_get_string(idx)); }
        else if (tag == RB_TAG_NULL) { l->action_reference = NULL; }
        else return 0;
        return 1;
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
            l->positions_characters = calloc(n ? n : 1, sizeof(char*));
            if (!l->positions_characters) return 0;
            l->n_positions_characters = (int)n;
            for (uint32_t i = 0; i < n; i++) {
                /* Each element is an object with position + character fields.
                   We serialize it as a JSON-like string for simplicity.

                   vm.rs:630-631 reads the tag of EACH ELEMENT here, before
                   deciding whether it is an object. This branch used to test
                   `tag` instead — but `tag` was already established to be
                   RB_TAG_ARRAY by the test above, so the comparison against
                   RB_TAG_OBJVAR/RB_TAG_OBJECT was dead, every element fell
                   through to `skip_value(r, tag)`, and every entry was stored
                   NULL. The field then decoded with the right length and no
                   payload, and stage_satisfies_positioned_characters
                   (condition.c) rejects a NULL entry, so PL!HS-bp2-026-L could
                   never award its +2 no matter how the board was arranged. */
                uint8_t etag; if (!rd_u8(r, &etag)) return 0;
                if (etag != RB_TAG_OBJECT && etag != RB_TAG_OBJVAR) {
                    /* Rust drops the element (vm.rs:633-634). C keeps the slot
                       and stores NULL instead: dropping would shrink the array,
                       and a half-decoded list must not be able to satisfy the
                       all-entries conjunction by having fewer entries to fail.
                       The evaluator rejects a NULL entry, so this is the
                       fail-closed direction. */
                    if (!skip_value(r, etag)) return 0;
                    l->positions_characters[i] = NULL;
                    continue;
                }
                if (etag == RB_TAG_OBJVAR) { uint8_t vb; if (!rd_u8(r, &vb)) return 0; }
                uint32_t ocount; if (!rd_len(r, &ocount)) return 0;
                char buf[512];
                size_t blen = 0;
                int emitted = 0, overflow = 0;
                buf[0] = 0;
                for (uint32_t j = 0; j < ocount; j++) {
                    uint32_t kidx; if (!rd_idx(r, &kidx)) return 0;
                    const char *kstr = rb_get_string(kidx);
                    uint8_t vtag; if (!rd_u8(r, &vtag)) return 0;
                    int is_pos = kstr && !strcmp(kstr, "position");
                    int is_chr = kstr && !strcmp(kstr, "character");
                    if (!is_pos && !is_chr) { if (!skip_value(r, vtag)) return 0; continue; }
                    const char *val = NULL;
                    if (vtag == RB_TAG_STR) {
                        uint32_t sidx; if (!rd_idx(r, &sidx)) return 0;
                        val = rb_get_string(sidx);
                    } else if (vtag == RB_TAG_NULL) {
                        val = "";    /* Rust: read_string_value().unwrap_or_default() */
                    } else {
                        /* Rust's read_string_value (vm.rs:398-408) returns None
                           for any tag other than NULL/STR WITHOUT consuming the
                           payload. Leaving it unconsumed would desync this
                           reader and corrupt every field after this object, so
                           consume it here and keep the stream aligned. */
                        if (!skip_value(r, vtag)) return 0;
                    }
                    if (!val) val = "";
                    if (!overflow) {
                        int w = snprintf(buf + blen, sizeof(buf) - blen, "%s\"%s\":\"%s\"",
                                         emitted ? "," : "",
                                         is_pos ? "position" : "character", val);
                        if (w < 0 || (size_t)w >= sizeof(buf) - blen) { buf[blen] = 0; overflow = 1; }
                        else blen += (size_t)w;
                    }
                    emitted++;
                }
                /* The separator counts the pairs actually written, not the raw
                   field index, so an unknown key ahead of "position" cannot
                   push a leading comma into the flattened text. An entry too
                   large to flatten is stored NULL rather than truncated: a
                   truncated pair would still parse as a real, wrong
                   requirement, and the evaluator rejects NULL instead. */
                l->positions_characters[i] = overflow ? NULL : rb_strdup(buf);
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
    /* shuffle (condition_decoder_gen.rs:197, read_bool_value) is a ConditionCommon
       field (card.rs `pub shuffle: Option<bool>`) carrying the location_condition
       "shuffle all waitroom members under deck" flag. */
    if (strcmp(key, "shuffle") == 0) {
        if (tag == RB_TAG_TRUE) { l->shuffle = 1; l->has_shuffle = 1; }
        else if (tag == RB_TAG_FALSE) { l->shuffle = 0; l->has_shuffle = 1; }
        else if (tag == RB_TAG_NULL) { l->has_shuffle = 0; }
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
    rb_note_decode_fallback(r->ability, "condition_field", key ? key : "<invalid>");
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
        case 9: return build_abilityfilter(&l);
        case 10: return build_scorethreshold(&l);
        case 11: return build_choice(&l);
        case 12: return build_complex(&l);
        case 13: return build_positioncond(&l);
        case 14: return build_opponentchoice(&l);
        case 15: return build_opponentlivesuccess(&l);
        case 16: return build_noexcessheart(&l);
        case 17: return build_alwaystrue(&l);
        case 18: return build_anyof(&l);
        case 19: return build_allrevealedmatchheartcolor(&l);
        /* Variant 20 is Condition::Unsupported in Rust
           (condition_decoder_gen.rs:1738 -> build_unsupported) and evaluates to
           FALSE (condition.rs:482-484, 546). It is deliberately NOT decoded here:
           the C evaluator's terminal `default: r = 1` (condition.c, the
           evaluate_condition dispatch) would score it TRUE, which is worse than
           the condition being absent. Both halves are needed:
             - vm.c:  case 20: return build_unsupported(&l);
             - condition.c: add `case 20: r = 0; break;` to the variant switch
           (rbuka.h would also need an RB_COND_UNSUPPORTED enumerator). */
        default: {
            char value[16];
            snprintf(value, sizeof(value), "%u", variant);
            rb_note_decode_fallback(r->ability, "condition_variant", value);
            return NULL;
        }
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
    if (!ek) return NULL;
    RbCardFilter *f = calloc(1, sizeof(*f));
    if (!f) return NULL;
    if (!rb_effect_filter_subset((const AbilityEffect *)ek, f)) {
        free(f);
        return NULL;
    }
    return f;
}

/* ── offset_of (stub) ──
   Computes the byte offset of ability `idx` within the bytecode blob.
   In Rust this sums OFFSET_DELTAS[..idx]; the C engine uses rb_bc_slice
   which does the same via g_offset_deltas. This stub exists for ABI
   parity with the Rust decoder. */
uint32_t offset_of(uint32_t idx) {
    if (idx >= RBKA_NUM_ABILITIES) return 0;
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
    return rb_zone_id_as_str((RbZoneId)z);
}

/* Zone::from_source_str — always-succeed conversion. Unknown → RB_ZONEID_UNKNOWN. */
int rb_zone_from_source_str(const char *s) {
    return rb_zone_id_from_str(s);
}

/* Zone::as_str — alias for to_str. */
const char *rb_zone_as_str(int z) {
    return rb_zone_to_str(z);
}

/* Type enums moved to enums.c. */
/* -- Decode-fallback audit (mirrors vm.rs DECODE_FALLBACKS) -- */
#define RB_DECODE_AUDIT_MAX 4096
static uint32_t g_decode_fallback_count = 0;
static uint32_t g_decode_fallback_abilities[RB_DECODE_AUDIT_MAX] = {0};

void rb_note_decode_fallback(int ability, const char *field, const char *value) {
    /* vm.rs:100-111 — the counter bump is only half of the contract. Rust also
       log::warn!'s every site ("[decode_audit] fallback #n (ability Some(i)):
       field = value"), because a silent default-substitution is exactly the
       defect this audit exists to surface; the C previously dropped `field` and
       `value` on the floor with (void) casts, leaving no way to find the gap.
       ability < 0 is Rust's None (bc.idx is Option<usize>). */
    uint32_t n = ++g_decode_fallback_count;
    if (ability >= 0 && ability < RB_DECODE_AUDIT_MAX) {
        g_decode_fallback_abilities[ability]++;
    }
    fprintf(stderr, "[decode_audit] fallback #%u (ability %s%d): %s = \"%s\"\n",
            n, ability < 0 ? "None, " : "", ability,
            field ? field : "<null>", value ? value : "");
}

uint32_t rb_decode_fallback_count(void) {
    return g_decode_fallback_count;
}

/* vm.rs:119-125 collects (0..NUM_ABILITIES.min(DECODE_AUDIT_MAX)), so indices at
   or past NUM_ABILITIES are never reported even if a stale counter survived. */
int rb_decode_fallback_abilities(uint32_t *out, int max) {
    uint32_t limit = RBKA_NUM_ABILITIES;
    if (limit > RB_DECODE_AUDIT_MAX) limit = RB_DECODE_AUDIT_MAX;
    int n = 0;
    for (uint32_t i = 0; i < limit && n < max; i++) {
        if (g_decode_fallback_abilities[i] > 0) {
            out[n++] = i;
        }
    }
    return n;
}