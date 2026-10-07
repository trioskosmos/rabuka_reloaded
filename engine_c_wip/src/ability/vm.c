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
/* ── nested-object field readers (mirror the BcReader::*_value family) ─────
 *
 * WHY THIS EXISTS.  The Rust readers behind the generated effect decoder —
 * BcReader::read_dynamic_count_value (vm.rs:977-1030),
 * read_quoted_text_value (vm.rs:1175-1205),
 * read_or_ability_filters_value (vm.rs:1114-1157),
 * read_position_value (vm.rs:933-975) — all match on
 *
 *     TAG_OBJECT | TAG_OBJECT_VARIANT
 *
 * and only consume a variant byte in the OBJVAR case.  That is not defensive
 * slack: it is the ONLY shape the compiler emits for these values.
 * cards/compile_abilities.py enc_val (L216-259) writes a dict value as
 *
 *     0x09 + variant   when `action` is present and maps to a known opcode
 *     0x08             otherwise — NO variant byte
 *
 * `dynamic_count`, `quoted_text` and `or_ability_filters` are nested objects
 * with no `action` key, so they are written as 0x08 and never as 0x09.  The C
 * branch that required RB_TAG_OBJVAR was therefore DEAD: the key fell through
 * to the scalar tail at the bottom of decode_effect_body, skip_value()'d the
 * whole object, and retained nothing.  47 effect objects in cards/
 * abilities.json carry `dynamic_count` and 11 carry `quoted_text`, so every
 * one of them silently lost its data — and rb_effect_count /
 * rb_resolve_dynamic_count (dynamic_count.c:250) then saw no reference, no
 * base_reference and no calculation at all.
 *
 * The rule this helper encodes: a tag test must be made against the tag of
 * the value ACTUALLY AT THAT POSITION.  Testing the tag captured before the
 * length -- or against the parent's tag -- is the dead-branch class that killed
 * `positions_characters`; do not reintroduce it here.
 *
 * `want[]` is the Rust reader's own key list. Keys outside it are skipped,
 * exactly as the `_ => self.skip_value()?` arm does.
 *
 * Returns 1 when the value was consumed (success or benign skip), 0 on a
 * structural read failure, which must abort the whole effect as elsewhere. */
static int read_named_object(Rdr *r, uint8_t tag,
                             const char *const *want, int nwant,
                             AbilityEffect *e, const char *rename_key)
{
    if (tag == RB_TAG_NULL) return 1;
    if (tag != RB_TAG_OBJECT && tag != RB_TAG_OBJVAR) return skip_value(r, tag);
    if (tag == RB_TAG_OBJVAR) { uint8_t vb; if (!rd_u8(r, &vb)) return 0; }
    uint32_t count;
    if (!rd_len(r, &count)) return 0;
    for (uint32_t i = 0; i < count; i++) {
        uint32_t kidx; uint8_t vtag;
        if (!rd_idx(r, &kidx) || !rd_u8(r, &vtag)) return 0;
        const char *k = rb_get_string(kidx);
        int keep = 0;
        for (int j = 0; j < nwant; j++)
            if (k && !strcmp(k, want[j])) { keep = 1; break; }
        if (!keep) { if (!skip_value(r, vtag)) return 0; continue; }
        /* Stringify the way the scalar tail does, so one reader serves both
         * string and u8 fields. NULL carries no value, so nothing is stored --
         * which is Rust's None for the Option fields. */
        char number[24];
        const char *value = NULL;
        if (vtag == RB_TAG_STR) {
            uint32_t sidx;
            if (!rd_idx(r, &sidx)) return 0;
            value = rb_get_string(sidx);
        } else if (vtag == RB_TAG_I64) {
            int64_t v;
            if (!rd_int(r, &v)) return 0;
            snprintf(number, sizeof(number), "%lld", (long long)v);
            value = number;
        } else if (vtag == RB_TAG_TRUE)  { value = "true";  }
        else if (vtag == RB_TAG_FALSE)   { value = "false"; }
        else { if (!skip_value(r, vtag)) return 0; continue; }
        if (!value) continue;
        effect_set_extra(e, k, value);
        /* `dynamic_count`'s wire key is "type" but the Rust struct member is
         * count_type, and dynamic_count.c:252 reads dc_extra(e, "count_type").
         * Publishing the struct name as well is what makes that lookup live;
         * without it the Option<String> field decoded in Rust is unreachable
         * in C even once the tag matches. */
        if (rename_key && !strcmp(k, rename_key))
            effect_set_extra(e, "count_type", value);
    }
    return 1;
}

/* vm.rs:977-1030 read_dynamic_count_value */
static const char *const DC_KEYS[] = {
    "type", "reference", "mode", "base_reference", "calculation",
    "calculation_value",
};
static int effect_decode_dynamic_count(AbilityEffect *e, Rdr *r, uint8_t tag)
{
    return read_named_object(r, tag, DC_KEYS,
                             (int)(sizeof(DC_KEYS) / sizeof(DC_KEYS[0])),
                             e, "type");
}

/* vm.rs:1175-1205 read_quoted_text_value */
static const char *const QT_KEYS[] = { "text", "quoted_type" };
static int effect_decode_quoted_text(AbilityEffect *e, Rdr *r, uint8_t tag)
{
    return read_named_object(r, tag, QT_KEYS,
                             (int)(sizeof(QT_KEYS) / sizeof(QT_KEYS[0])),
                             e, NULL);
}

/* vm.rs:1161-1173 read_placement_order_value -- a TAG_STR that only ever maps
 * "any_order". Kept as its own reader so the vocabulary stays in one place if
 * the enum grows. */
static int effect_decode_placement_order(AbilityEffect *e, Rdr *r, uint8_t tag)
{
    if (tag == RB_TAG_NULL) return 1;
    if (tag != RB_TAG_STR) return skip_value(r, tag);
    uint32_t idx;
    if (!rd_idx(r, &idx)) return 0;
    effect_set_extra(e, "placement_order", rb_get_string(idx));
    return 1;
}

/* ── TOMBSTONE (2026-10-04). `effect_decode_extra` REMOVED.
 *
 * A census classified this DEAD-WRONG and drew a serious consequence: "a
 * bytecode decoder arm with no caller: the `extra` tag it decodes has no C
 * consumer, so any effect `extra` the generator emits through this arm is
 * dropped rather than stored." That consequence is FALSE, and the danger of
 * believing it was that it invites someone to "fix" a decoder that is already
 * working.
 *
 * Extras are stored. Measured, not asserted: decoding all 2116 card-ability
 * pairs and walking all 3223 effects stores 6902 extras across 2352 of those
 * effects (max 12 on a single effect). A dropped-extras tree would report ~0.
 *
 * The live producer is the main decoder's generic scalar-extras arm at
 * vm.c:1168-1190, which handles every tag this function's
 * `decode_extra_value` handled and MORE:
 *     TAG_STR  (vm.c:1169-1170) -> effect_set_extra(e, key, rb_get_string(idx))
 *     TAG_I64  (vm.c:1171-1172) -> stringified into a 24-byte buffer
 *     TAG_TRUE (vm.c:1173-1174) / TAG_FALSE (vm.c:1175-1176) -> "true"/"false"
 *     TAG_ARRAY(vm.c:1177-1186) -> decode_extra_value + effect_set_extra,
 *                                 PLUS the heart_colors -> heart_color mirror
 *                                 at vm.c:1181-1183
 *     anything else (vm.c:1187-1190) -> skip_value
 * So the live arm is a strict SUPERSET of the deleted function: it had the same
 * ARRAY body and one extra rule this function never had. `effect_decode_extra`
 * was a dead duplicate of that arm, not a missing producer.
 *
 * Deadness proved by gcc -Wunused-function: "'effect_decode_extra' defined but
 * not used"; its only occurrence in the file was its own definition. */

/* decode a single effect from current cursor (assumes TAG_OBJVAR already read).
    Rust: TAG_OBJECT_VARIANT (0x09) + variant u8 + len + fields.
    The variant selects EffectKind but C stores only action string; we consume
    the byte and ignore it. Mirrors vm.rs:decode_ability_effect_direct . */
/* ---- TEMPORARY PROBE INSTRUMENTATION (removed before hand-off) ----
   Capacities are overridable so a 2 MB RAM console target
   (Jaguar) can shrink the audit scratch; the writers are
   bounds-checked against the same macros, so a smaller
   capacity only reduces audit resolution, never gameplay. */
#ifndef RB_PROBE_DROPPED_CAP
#define RB_PROBE_DROPPED_CAP 4096
#endif
#ifndef RB_PROBE_KEYS_CAP
#define RB_PROBE_KEYS_CAP 1024
#endif
uint32_t g_probe_seen = 0, g_probe_retained = 0, g_probe_effects = 0;
static char g_probe_dropped[RB_PROBE_DROPPED_CAP][48];
static uint8_t g_probe_dropped_tag[RB_PROBE_DROPPED_CAP];
static uint32_t g_probe_ndropped = 0;
static char g_probe_keys[RB_PROBE_KEYS_CAP][48];
static uint32_t g_probe_keycount[RB_PROBE_KEYS_CAP];
static uint32_t g_probe_nkeys = 0;
static void probe_seen(const char *k) {
    g_probe_seen++;
    if (k) for (uint32_t i = 0; i < g_probe_nkeys; i++)
        if (!strcmp(g_probe_keys[i], k)) { g_probe_keycount[i]++; return; }
    if (g_probe_nkeys < RB_PROBE_KEYS_CAP) {
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
    if (g_probe_ndropped < RB_PROBE_DROPPED_CAP) {
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
        /* Nested-object filter fields the generated decoder reads with a
         * dedicated BcReader method. The tag test is against `tag` -- the tag
         * of THIS value -- and both TAG_OBJECT and TAG_OBJECT_VARIANT are
         * accepted, because those are the two the compiler can emit for a
         * dict with no `action` key (see read_named_object). Requiring
         * RB_TAG_OBJVAR made all three branches unreachable; 47 effect
         * objects carry dynamic_count and 11 carry quoted_text. */
        if (key && !strcmp(key, "dynamic_count")) {
            if (!effect_decode_dynamic_count(e, r, tag)) goto fail;
            probe_keep();
            continue;
        }
        if (key && !strcmp(key, "quoted_text")) {
            if (!effect_decode_quoted_text(e, r, tag)) goto fail;
            probe_keep();
            continue;
        }
        if (key && !strcmp(key, "placement_order")) {
            if (!effect_decode_placement_order(e, r, tag)) goto fail;
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
           are also retained as extras (below) for callers that read them as strings.

           `max_repeats` is the wire ALIAS for `repeat_limit`:
           effect_decoder_gen.rs:241 `"max_repeats" => { ek.repeat_limit =
           bc.read_u8_value(); Some(true) }`. Only the literal `repeat_limit` was
           handled here, and `repeat_limit` does not occur ANYWHERE in the current
           cards/abilities.json (0 occurrences) while `max_repeats` occurs 6 times.
           So the branch was dead for every real ability: max_repeats fell to the
           scalar tail below and was stored under the key "max_repeats", which
           rb_effect_repeat_limit_any (core/card.c) never reads -- it looks at the
           struct field and at the extra "repeat_limit". Result:
             - repeat_procedure rendered "Repeat up to 1 times" where the card
               says 4 (its own count is absent, so describe.rs's
               `unwrap_or(c.unwrap_or(1))` fell back to 1);
             - perform_yell rendered "Perform 1 yell" instead of "Perform up to 4
               yells", collapsing to the amt==mx branch.
           Both are the effect losing its repeat limit at DECODE time, which is the
           defect class this decoder exists to prevent. `repeat_limit` is retained
           under BOTH names so either accessor resolves it. */
        if (key && (!strcmp(key, "repeat_limit") || !strcmp(key, "max_repeats"))) {
            char buf[24];
            if (tag == RB_TAG_I64) {
                int64_t v;
                if (!rd_int(r, &v)) goto fail;
                e->repeat_limit = (int)v;
                snprintf(buf, sizeof(buf), "%lld", (long long)v);
            } else {
                /* Rust's read_u8_value maps TAG_NULL and every non-I64 tag to
                   None, so the struct field must stay untouched. */
                if (!skip_value(r, tag)) goto fail;
                probe_drop(key, tag);
                buf[0] = '\0';
            }
            /* Retain under BOTH names. A wire object never carries the same key
               twice, so `key` is one of the two; mirroring the value onto the
               other guarantees rb_effect_repeat_limit_any -- which reads the
               struct field first and only then the extra "repeat_limit" -- sees
               the limit whichever name the card used. */
            if (buf[0]) {
                if (!effect_set_extra(e, "repeat_limit", buf)) goto fail;
                if (strcmp(key, "repeat_limit") &&
                    !effect_set_extra(e, key, buf)) goto fail;
                probe_keep();
            }
            continue;
        }
        /* `energy` is the wire ALIAS for `energy_count`:
           effect_decoder_gen.rs:240 `"energy" => { ek.energy_count =
           bc.read_u8_value(); Some(true) }` -- the SAME `ek` slot as the
           `"energy_count"` arm at :176. Rust has no second name for it: both
           spellings land on the single `energy_count: Option<u8>` field of
           AbilityEffect (ability/types.rs:315, carried into the struct literal
           at :536).

           Measured on cards/abilities.json (MD5 f459f727c852784095d33cb980034211,
           working tree): `"energy"` occurs 85 times as a key and `"energy_count"`
           only 12, i.e. the ALIAS is the dominant spelling by 7x. Every value is
           in 1..=6, so no card is lost to the u8 gate. The same shape as the
           `max_repeats` arm above: before this branch `energy` fell through to
           the scalar tail below and was retained under its own raw key only, so
           every reader that asks for the canonical `energy_count` came back None
           -- describe.c:1320 (place_energy_under_member) and :1537 (the pay_energy
           COST) both go through d_u8_extra(e, "energy_count") with no alias
           fallback, and misc.c:777 extra_int(e, "energy_count", -1) likewise.
           That is why 「{{icon_energy.png|E}}を2枚 paying 2 energy」 still described
           and executed as a single energy: the amount was dropped at DECODE time.
           cost.c / core/card.c / game_setup.c already consult both spellings via
           rb_cost_energy_count_any, and compound.c's Q92 path calls exactly that,
           so those stay correct either way -- this change is what makes the
           canonical name readable for everyone.

           read_u8_value (vm.rs:457-468 -> read_u32_value) maps TAG_NULL to None
           and every non-I64 tag to None as well, and for I64 only accepts
           `u8::try_from(int).ok()` -- 0..=255. A negative or >255 integer is
           therefore None in Rust and is retained under NEITHER name here, so the
           C never invents a value Rust dropped. */
        if (key && (!strcmp(key, "energy") || !strcmp(key, "energy_count"))) {
            char buf[24];
            buf[0] = '\0';
            if (tag == RB_TAG_I64) {
                int64_t v;
                if (!rd_int(r, &v)) goto fail;
                if (v >= 0 && v <= 255) snprintf(buf, sizeof(buf), "%lld", (long long)v);
            } else {
                /* Not TAG_I64: read_u8_value returns None without touching the
                   field. The value still has to be consumed to keep the reader
                   in sync, exactly as the max_repeats arm above does. */
                if (!skip_value(r, tag)) goto fail;
            }
            if (buf[0]) {
                /* Canonical name first: it is what every reader asks for. */
                int ok = effect_set_extra(e, "energy_count", buf);
                /* Mirror onto the wire spelling when it is a different name. The
                   mirror is best-effort: an effect already holding RB_MAX_EXTRA
                   keys must NOT be failed here — `goto fail` would drop the whole
                   ability over a key Rust stores in a struct field — so a full
                   table degrades to the canonical pair that did fit. */
                if (ok && strcmp(key, "energy_count")) effect_set_extra(e, key, buf);
                if (ok) probe_keep(); else probe_drop(key, tag);
            } else probe_drop(key, tag);
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
        /* `card_type` is a TYPED field, not a raw string, and this is the only
         * writer of AbilityEffect::card_type_field in the tree.
         *
         * Rust: effect_decoder_gen.rs:81 `"card_type" => { ek.card_type =
         * bc.read_card_type_value(); Some(true) }`, and read_card_type_value
         * (vm.rs:479-489) is `CardType::from_card_str(get_string(idx)?)`.
         * CardType::from_card_str (core/card.rs) has EXACTLY three arms and its
         * miss arm is `_ => None`, so `AbilityEffect::card_type` is
         * `Option<CardType>` -- the raw spelling cannot survive the decode in
         * Rust, by construction.
         *
         * Measured over the corpus (985 abilities, 2230 decoded effects, via
         * rb_decode_card_ability + the real decoder): 817 effects spell
         * `card_type` inside the vocabulary (member_card 442, live_card 121,
         * energy_card 86) and **168 spell it OUTSIDE it** -- `card` 167 and
         * `member` 1 -- spread over **149 of the 985 abilities**
         * (actions: move_cards 146, gain_resource 21, modify_cost 1).
         * Storing the raw string made the C disagree with Rust at five
         * independent consumers, because every one of them tests the field for
         * mere NON-EMPTINESS rather than for a vocabulary:
         *   core/card.c:959  card_filter_from_effect copies it into
         *     RbCardFilter.card_type, which core/card.c:1107 counts in
         *     `has_filter` -> an all-else-empty effect becomes "filtered",
         *     and util.c:1547 then rejects EVERY card (`card_type == "card"`
         *     matches none) -> empty candidate set, silent no-op.
         *   ability/choice.c:251 puts it straight into RbChoice.card_type;
         *     Rust can only ever place one of the three names there
         *     (`card_type_any().map(CardType::as_card_str)`).
         *   ability/compound.c:467-469 has a `!strcmp(ct,"member")` arm that
         *     exists precisely because this field held the raw spelling; Rust
         *     sees None there and keeps self_target inheritable.
         *   ability/dynamic_count.c:654-655 passes it to dc_per_unit_units.
         *   ability/describe.c:357-367 re-derives it correctly through
         *     rb_effect_card_type_from_str, i.e. it was the only one of the
         *     five that did NOT inherit the bug -- which is what made the
         *     other four look covered.
         *
         * On a miss BOTH the struct field and the `card_type` extra stay unset.
         * The extra matters: core/card.c:959 and dynamic_count.c:655 both read
         * `card_type_field[0] ? field : fx_extra(e, "card_type")`, so leaving
         * the raw spelling in the extras table would re-introduce exactly the
         * divergence the empty field removes. (Measured: today the extras
         * table never holds a `card_type` at all -- 0 of 2230 effects -- because
         * this branch `continue`s before the scalar tail, so that fallback is
         * already dead and stays dead.) Rust drops the string too, so nothing
         * that Rust can observe is lost. */
        if (key && !strcmp(key, "card_type") && tag == RB_TAG_STR) {
            uint32_t idx;
            if (!rd_idx(r, &idx)) goto fail;
            const char *s = rb_get_string(idx);
            if (s && (!strcmp(s, "member_card") || !strcmp(s, "live_card") ||
                      !strcmp(s, "energy_card"))) {
                strncpy(e->card_type_field, s, 23);
                e->card_type_field[23] = 0;
                probe_keep();
            } else {
                e->card_type_field[0] = 0;   /* Rust's `_ => None` */
                probe_drop(key, tag);
            }
            continue;
        }
        if (key && !strcmp(key, "self_target") && tag == RB_TAG_STR) {
            uint32_t idx;
            if (!rd_idx(r, &idx)) goto fail;
            const char *s = rb_get_string(idx);
            if (s) { strncpy(e->self_target_field, s, 7); e->self_target_field[7]=0; }
            probe_keep();
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
    /* T7 — the five keys Rust's ConditionCommon carries and this decoder did
     * not, so each of them used to reach the catch-all and be DROPPED.
     * Corpus counts over the 985 shipped abilities (make t7-wire):
     *   any_location        18 nodes / 18 abilities
     *   activated_this_turn  3 /  1
     *   paid_cost_this_turn  1 /  1
     *   text              680 / 504
     *   trigger_event     131 / 112
     * Rust: condition_decoder_gen.rs:137 / :25 / :84 / :234 / :25,:168 and
     * core/card.rs:3198 / :3242 / :3248 / :3218 / :3223. */
    int any_location; int has_any_location;
    int activated_this_turn; int has_activated_this_turn;
    int paid_cost_this_turn; int has_paid_cost_this_turn;
    char *text;
    Condition *trigger_event;
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
    /* T7 — keys Rust's ConditionCommon carries (card.rs:3198, :3242, :3248,
     * :3218, :3223). Copied here so every build_* variant carries them; each
     * was reaching the decode_condition_field catch-all and being dropped. */
    if (l->has_any_location) cond_add_bool(c, "any_location", l->any_location);
    if (l->has_activated_this_turn) cond_add_bool(c, "activated_this_turn", l->activated_this_turn);
    if (l->has_paid_cost_this_turn) cond_add_bool(c, "paid_cost_this_turn", l->paid_cost_this_turn);
    if (l->text) cond_add_str(c, "text", l->text);
    if (l->trigger_event) cond_add_cond(c, "trigger_event", l->trigger_event);
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
    /* ── T7: the five keys Rust's ConditionCommon carries and this decoder did
     * not name. Each of them previously fell through to the catch-all below,
     * which calls rb_note_decode_fallback and DROPS the value — a silent
     * wrong-value waiting for the moment somebody wires decode_condition_direct.
     * Rust reads them at condition_decoder_gen.rs:137 (any_location), :25
     * (activated_this_turn), :84 (paid_cost_this_turn), :234 (text) and
     * :25/:168 (trigger_event); all under core/card.rs ConditionCommon.
     *
     * `text` and `trigger_event` are `#[cfg(feature = "debug_conditions")]` in
     * Rust, and debug_conditions IS in engine/Cargo.toml's default features,
     * so an ordinary build has them.
     */
    if (strcmp(key, "any_location") == 0) {
        if (tag == RB_TAG_TRUE) { l->any_location = 1; l->has_any_location = 1; }
        else if (tag == RB_TAG_FALSE) { l->any_location = 0; l->has_any_location = 1; }
        else if (tag == RB_TAG_NULL) { l->has_any_location = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "activated_this_turn") == 0) {
        if (tag == RB_TAG_TRUE) { l->activated_this_turn = 1; l->has_activated_this_turn = 1; }
        else if (tag == RB_TAG_FALSE) { l->activated_this_turn = 0; l->has_activated_this_turn = 1; }
        else if (tag == RB_TAG_NULL) { l->has_activated_this_turn = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "paid_cost_this_turn") == 0) {
        if (tag == RB_TAG_TRUE) { l->paid_cost_this_turn = 1; l->has_paid_cost_this_turn = 1; }
        else if (tag == RB_TAG_FALSE) { l->paid_cost_this_turn = 0; l->has_paid_cost_this_turn = 1; }
        else if (tag == RB_TAG_NULL) { l->has_paid_cost_this_turn = 0; }
        else return 0;
        return 1;
    }
    if (strcmp(key, "text") == 0) {
        /* Rust: `#[cfg(debug_conditions)] text: Option<String>` read with
         * read_string_value — NULL and a non-string both yield None. */
        if (tag == RB_TAG_STR) {
            uint32_t idx; if (!rd_idx(r, &idx)) return 0;
            l->text = rb_strdup(rb_get_string(idx));
            return 1;
        }
        if (tag == RB_TAG_NULL) return 1;
        skip_value(r, tag);
        return 1;
    }
    if (strcmp(key, "trigger_event") == 0) {
        /* Rust: `#[cfg(debug_conditions)] trigger_event: Option<Box<TriggerEvent>>`.
         * The C has no TriggerEvent struct; a nested Condition under the same
         * key is the carrier the live mirror (read_condition_fields, vm.c:404)
         * already produces, so the typed path and the mirror agree on the
         * value either way. Rust's TriggerEvent reader names 20 of the 24
         * sub-keys the corpus carries and skip_values the rest, so keeping all
         * of them here is a strict superset of what Rust retains. */
        if (tag == RB_TAG_OBJVAR) { l->trigger_event = read_condition(r); return 1; }
        if (tag == RB_TAG_OBJECT) { l->trigger_event = read_condition_fields(r, 0xFF); return 1; }
        if (tag == RB_TAG_NULL) return 1;
        skip_value(r, tag);
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
/* build_unsupported: variant 20 — Condition::Unsupported.
   Rust condition_decoder_gen.rs:1738 maps variant 20 to build_unsupported, and
   the evaluator scores it FALSE at condition.rs:495-497 and again at :559.
   The C vocabulary in include/rabuka.h stops at RB_COND_ALL_REVEALED (19), so
   the value is spelled out as 20 here and matched literally in the condition.c
   dispatch; rabuka.h's owner should add `RB_COND_UNSUPPORTED` and both sites
   should switch to it. Carries no field beyond ConditionCommon — card.rs's
   `Unsupported { common }` is exactly that. */
Condition *build_unsupported(const ConditionLocals *l) {
    Condition *c = calloc(1, sizeof(Condition));
    if (!c) return NULL;
    c->variant = 20;
    cond_copy_common(c, l);
    return c;
}

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
           FALSE (condition.rs:482-484, 546). It used to be deliberately NOT
           decoded here, because the C evaluator's terminal `default: r = 1`
           (condition.c, the evaluate_condition dispatch) would score it TRUE,
           which is worse than the condition being absent. Both halves are now
           in place:
             - vm.c:     case 20: return build_unsupported(&l);   (above)
             - condition.c: `case 20: r = 0; break;` plus `default: r = 0;`
           in the variant switch, so an unmapped shape fails closed instead of
           leaving a bonus permanently on.
           STILL OUTSTANDING: include/rabuka.h needs an `RB_COND_UNSUPPORTED`
           enumerator so 20 can be named rather than spelled out. */
        case 20: return build_unsupported(&l);
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

/* ══════════════════════════════════════════════════════════════════════════
   Generated-table integrity self-check.

   WHY THIS EXISTS. The ability string pool (`abilities_strings.bin`) is a
   GITIGNORED build artifact, but the offsets that index it
   (RBKA_STRINGS_OFFSETS, in the COMMITTED src/core/generated/gen_data.c) are
   not. The two were allowed to drift, and on 2026-09-30 they did: a peer
   rebuilt the .bin from cards/abilities.json and the committed offsets were
   left 227 slots stale, up to 311 bytes wrong. The interned pool is
   UNSEPARATED, so a small drift does not fail — it splices. `cost_limit`
   decoded as `cost_limitc`, `left_side` as `eft_side`, `invalidate_ability`
   as `stiming_conditioni`, and 156 of 6032 strings carried a U+FFFD.

   Critically, the pool's TOTAL LENGTH was unchanged (726731 bytes before and
   after), so a length check alone would have stayed silent. The check that
   does catch it is the per-string FNV-1a digest emitted alongside the offsets
   by tools/gen_from_rs.py: it is read from the same commit as the offsets, so
   regenerating the .bin without regenerating gen_data.c is now detectable.

   Three layers, cheapest first:
     1. offsets monotonic, and the sentinel equals the generator-recorded table
        length  -> catches a truncated or grown table;
     2. every interned string's FNV-1a matches  -> catches ANY content drift,
        including drift too small to change the length;
     3. semantic plausibility: no U+FFFD, no C0 control, valid UTF-8, and every
        key operand reachable from the bytecode non-empty  -> catches a table
        that is internally consistent but simply not this game's data.

   Layer 3 alone would NOT have caught the 2026-09-30 incident: `cost_limitc`
   and `eft_side` are perfectly well-formed identifiers. Layer 1 alone would
   not either. Layer 2 is the load-bearing one.

   Must be called after rb_load(); tests/test_decode_skew.c is the caller.
   ══════════════════════════════════════════════════════════════════════════ */

#include "gen_data.h"
extern const uint32_t RBKA_STRINGS_OFFSETS[];
extern const uint32_t RBKA_STRINGS_FNV[];
extern const uint32_t RBKA_STRINGS_TABLE_LEN;
extern uint32_t *g_strings_offsets;

/* FNV-1a 32. MUST stay bit-identical to fnv1a() in tools/gen_from_rs.py. */
static uint32_t rb_fnv1a32(const unsigned char *p, size_t n) {
    uint32_t h = 0x811C9DC5u;
    for (size_t i = 0; i < n; i++) {
        h ^= (uint32_t)p[i];
        h *= 0x01000193u;
    }
    return h;
}

/* Same function, resumable. Lets rb_verify_generated_tables() hash the whole
 * table in one pass without allocating a copy: the pool is unseparated, so the
 * bytes are exactly the concatenation of string 0, string 1, ... */
static uint32_t rb_fnv1a32_resume(uint32_t h, const unsigned char *p, size_t n) {
    for (size_t i = 0; i < n; i++) {
        h ^= (uint32_t)p[i];
        h *= 0x01000193u;
    }
    return h;
}

/* Why a decoded string cannot be trusted. 0 = plausible.
 *
 * `allow_empty` matters: the pool legitimately contains empty strings (an
 * absent optional value interns to ""), and several AbilityEffect fields are
 * fixed char[] that stay "" when the wire key was absent. rb_get_string also
 * returns a shared "" literal for an out-of-range index, so "" is only a
 * defect when the caller is asking about a value that must be present. */
static int rb_string_defect(const char *s, size_t *bad_off, int allow_empty) {
    const unsigned char *p = (const unsigned char *)s;
    size_t n = 0;
    if (!s || !*s) return allow_empty ? 0 : 1;
    while (p[n]) {
        unsigned char c = p[n];
        if (c == 0xEF && p[n+1] == 0xBF && p[n+2] == 0xBD) { *bad_off = n; return 2; } /* U+FFFD */
        if (c < 0x20 && c != '\t' && c != '\n' && c != '\r') { *bad_off = n; return 3; }
        if (c == 0x7F) { *bad_off = n; return 3; }
        n++;
    }
    /* strict UTF-8 walk: a truncated tail is what a mid-character offset
       splice produces, and it is the mechanism behind every U+FFFD above */
    size_t i = 0;
    while (i < n) {
        unsigned char c = p[i];
        int extra;
        unsigned int cp;
        if (c < 0x80) { i++; continue; }
        else if ((c & 0xE0) == 0xC0) { extra = 1; cp = c & 0x1F; }
        else if ((c & 0xF0) == 0xE0) { extra = 2; cp = c & 0x0F; }
        else if ((c & 0xF8) == 0xF0) { extra = 3; cp = c & 0x07; }
        else { *bad_off = i; return 4; }        /* BAD_UTF8: stray continuation */
        if (i + (size_t)extra >= n) { *bad_off = i; return 4; }
        for (int k = 1; k <= extra; k++) {
            unsigned char cc = p[i + (size_t)k];
            if ((cc & 0xC0) != 0x80) { *bad_off = i; return 4; }
            cp = (cp << 6) | (cc & 0x3F);
        }
        if ((extra == 1 && cp < 0x80) || (extra == 2 && cp < 0x800) ||
            (extra == 3 && cp < 0x10000) || cp > 0x10FFFF ||
            (cp >= 0xD800 && cp <= 0xDFFF)) { *bad_off = i; return 4; }
        i += (size_t)extra + 1;
    }
    return 0;
}

/* Walk one value, reporting every string reference whose decoded text is
 * defective. `ks->n_bad` counts them; `ks->n_keys` counts the key-position
   ones. Returns 0 if the bytecode itself failed to walk. */
typedef struct { int n_bad, n_keys; } KeyScan;
static int scan_refs(Rdr *r, KeyScan *ks, int depth) {
    uint32_t len, i;
    int64_t v;
    uint8_t b, tag;
    if (depth > 24) return 0;
    if (!rd_u8(r, &tag)) return 0;
    switch (tag) {
    case RB_TAG_NULL: case RB_TAG_FALSE: case RB_TAG_TRUE: return 1;
    case RB_TAG_I64: return rd_int(r, &v);
    case RB_TAG_F64:
        if (r->p + 8 > r->end) return 0;
        r->p += 8; return 1;
    case RB_TAG_STR: {
        uint32_t idx;
        if (!rd_idx(r, &idx)) return 0;
        size_t off = 0;
        if (rb_string_defect(rb_get_string(idx), &off, 1)) ks->n_bad++;
        return 1;
    }
    case RB_TAG_ARRAY:
        if (!rd_len(r, &len)) return 0;
        for (i = 0; i < len; i++) if (!scan_refs(r, ks, depth + 1)) return 0;
        return 1;
    case RB_TAG_OBJECT: case RB_TAG_OBJVAR:
        if (tag == RB_TAG_OBJVAR) { if (!rd_u8(r, &b)) return 0; }
        if (!rd_len(r, &len)) return 0;
        for (i = 0; i < len; i++) {
            uint32_t key;
            size_t off = 0;
            if (!rd_idx(r, &key)) return 0;
            if (rb_string_defect(rb_get_string(key), &off, 0)) { ks->n_bad++; ks->n_keys++; }
            if (!scan_refs(r, ks, depth + 1)) return 0;   /* the value */
        }
        return 1;
    default: return 0;
    }
}

int rb_verify_generated_tables(int *out_bad_strings, int *out_bad_keys,
                               char *msg, unsigned long msg_sz) {
    uint32_t n = RBKA_NUM_STRING_OFFSETS ? (RBKA_NUM_STRING_OFFSETS - 1) : 0;
    int bad_len = 0, bad_order = 0, bad_digest = 0, bad_text = 0, bad_table = 0;
    size_t msz = msg ? (size_t)msg_sz : 0, mn = 0;
    if (msg && msz) msg[0] = 0;
#define VMSG(...) do { if (msg && mn < msz) { \
        int _w = snprintf(msg + mn, msz - mn, __VA_ARGS__); \
        if (_w > 0) mn += (size_t)_w; } } while (0)

    if (RBKA_NUM_STRING_OFFSETS == 0) {
        VMSG("FATAL: RBKA_NUM_STRING_OFFSETS == 0 (gen_data.h is stale)\n");
        if (out_bad_strings) *out_bad_strings = 0;
        if (out_bad_keys) *out_bad_keys = 0;
        return -1;
    }

    /* ── layer 1: offsets must be monotonic and end at the recorded length ── */
    if (g_strings_offsets[0] != 0) bad_order = 1;
    for (uint32_t i = 0; i < n; i++)
        if (g_strings_offsets[i] > g_strings_offsets[i + 1]) bad_order = 1;
    if (g_strings_offsets[n] != RBKA_STRINGS_TABLE_LEN) {
        bad_len = 1;
        VMSG("FATAL: string table length skew: offsets end at %u but the "
             "generator recorded %u.\n"
             "       cards/build/abilities_strings.bin was regenerated without "
             "regenerating src/core/generated/gen_data.c.\n"
             "       FIX: bash tools/regen_all.sh\n",
             g_strings_offsets[n], RBKA_STRINGS_TABLE_LEN);
    }
    if (bad_order)
        VMSG("FATAL: RBKA_STRINGS_OFFSETS is not monotonic / does not start at 0.\n");

    /* ── layer 1b: WHOLE-TABLE digest. One comparison that catches any
       byte-level drift, including drift that preserves every individual
       string length. A mid-pool splice (the mechanism behind every U+FFFD
       this check exists to prevent) rewrites bytes without changing any
       offset, so the length and per-string checks alone can both pass while
       the text is still wrong. Emitted by tools/gen_from_rs.py as
       RBKA_STRINGS_TABLE_FNV; 0 means "not recorded", i.e. a gen_data.c from
       before this check existed, which is not itself a defect. ── */
#ifdef RBKA_STRINGS_TABLE_FNV
    if (RBKA_STRINGS_TABLE_FNV != 0u) {
        uint32_t h = 0x811C9DC5u;
        int truncated = 0;
        for (uint32_t i = 0; i < n; i++) {
            uint32_t a = g_strings_offsets[i], b = g_strings_offsets[i + 1];
            if (b > RBKA_STRINGS_TABLE_LEN || a > b) { truncated = 1; break; }
            const char *s = rb_get_string(i);
            size_t want = (size_t)(b - a);
            if (!s || strlen(s) != want) { truncated = 1; break; }
            h = rb_fnv1a32_resume(h, (const unsigned char *)s, want);
        }
        if (truncated) {
            /* An offset points outside the table; the per-string pass below
             * reports the specifics. Do not double-report here. */
        } else if (h != RBKA_STRINGS_TABLE_FNV) {
            bad_table = 1;
            VMSG("FATAL: whole string table digest mismatch: the generator "
                 "recorded %08x, the table loaded at runtime hashes to %08x.\n"
                 "       The table has been modified in place without "
                 "regenerating gen_data.c, so some number of strings now "
                 "carry bytes the offsets never described.\n"
                 "       FIX: bash tools/regen_all.sh\n",
                 (unsigned)RBKA_STRINGS_TABLE_FNV, (unsigned)h);
        }
    }
#else
    (void)rb_fnv1a32_resume;
#endif

    /* ── layer 2: per-string FNV-1a. The load-bearing check. ──
       Counted for reporting, but the first six offenders are named so the
       failure names a card-relevant string rather than an index. */
    int named = 0;
    for (uint32_t i = 0; i < n; i++) {
        uint32_t a = g_strings_offsets[i], b = g_strings_offsets[i + 1];
        if (b > RBKA_STRINGS_TABLE_LEN || a > b) { bad_digest++; continue; }
        const char *s = rb_get_string(i);
        size_t slen = strlen(s ? s : "");
        if (slen != (size_t)(b - a) ||
            rb_fnv1a32((const unsigned char *)(s ? s : ""), slen) != RBKA_STRINGS_FNV[i]) {
            bad_digest++;
            if (named < 6) {
                named++;
                VMSG("FATAL: string #%u digest mismatch (offsets %u..%u). "
                     "Decodes to \"%s\".\n", i, a, b, s ? s : "");
            }
        }
    }
    if (bad_digest)
        VMSG("       %d of %u interned strings do not match the digest recorded "
             "in gen_data.c (showing %d).\n"
             "       cards/build/abilities_strings.bin and "
             "src/core/generated/gen_data.c are from different generator runs.\n"
             "       FIX: bash tools/regen_all.sh\n", bad_digest, n, named);

    /* ── layer 3: semantic plausibility of every string, and of every string
       reference the bytecode actually makes. ── */
    for (uint32_t i = 0; i < n; i++) {
        size_t off = 0;
        if (rb_string_defect(rb_get_string(i), &off, 1)) {
            bad_text++;
            if (bad_text <= 6) {
                const char *s = rb_get_string(i);
                VMSG("FATAL: string #%u is not decodable text (byte %zu): \"%s\"\n",
                     i, off, s ? s : "");
            }
        }
    }

    int bad_refs = 0, bad_ref_keys = 0, abil_bad = 0;
    for (uint32_t a = 0; a < RBKA_NUM_ABILITIES; a++) {
        uint32_t len;
        const unsigned char *slice = rb_bc_slice(a, &len);
        if (!slice || len == 0) continue;
        Rdr r = { slice, slice + len, (int)a };
        KeyScan ks = { 0, 0 };
        if (!scan_refs(&r, &ks, 0)) continue;
        if (ks.n_bad) { abil_bad++; bad_refs += ks.n_bad; bad_ref_keys += ks.n_keys; }
    }
    if (bad_refs)
        VMSG("       %d defective string reference(s) (%d in key position) across "
             "%d of %u abilities.\n", bad_refs, bad_ref_keys, abil_bad, RBKA_NUM_ABILITIES);
    if (bad_text)
        VMSG("       %d of %u interned strings are not decodable text.\n", bad_text, n);

    if (out_bad_strings) *out_bad_strings = bad_text > bad_digest ? bad_text : bad_digest;
    if (out_bad_keys) *out_bad_keys = bad_ref_keys;

    int rc = (bad_len || bad_order || bad_digest || bad_text || bad_refs ||
              bad_table) ? -1 : 0;
    if (rc == 0)
        VMSG("gen_data self-check OK: %u abilities, %u interned strings, table "
             "%u bytes, offsets and digests consistent.\n",
             RBKA_NUM_ABILITIES, n, RBKA_STRINGS_TABLE_LEN);
#undef VMSG
    return rc;
}