#include "rabuka.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#ifdef RB_NO_MALLOC
/* Bare-metal bump arena: 512 KB static pool, no free(). Swap in per-platform. */
#define ARENA_SIZE (512u * 1024u)
static unsigned char arena[ARENA_SIZE];
static size_t arena_off = 0;

void *rb_malloc(size_t n) {
    n = (n + 7u) & ~7u; /* 8-byte align */
    if (arena_off + n > ARENA_SIZE) return NULL;
    void *p = arena + arena_off;
    arena_off += n;
    return p;
}
void rb_free(void *p) { (void)p; /* no-op */ }
void rb_alloc_reset(void) { arena_off = 0; }
#else
void *rb_malloc(size_t n) { return malloc(n); }
void rb_free(void *p) { free(p); }
void rb_alloc_reset(void) {}
#endif

char *rb_strdup2(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s) + 1;
    char *p = (char *)rb_malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

/* ── Pool (mirrors Rust Pool<T> from core/pool.rs) ──
   Fixed-size object pool with a free list. Used by EkBox. */
#define RB_POOL_CAPACITY 128
typedef struct {
    AbilityEffect *slots[RB_POOL_CAPACITY];
    int            free_list[RB_POOL_CAPACITY];
    int            free_count;
    int            next;
} RbPool;

/* Two freers exist for AbilityEffect and they MUST stay field-for-field
   identical:
     - effect_free      (vm.c:443, static)  — releases via libc free()
     - rb_effect_free   (here, exported)     — releases via rb_free()
   vm.c's is reachable through rb_free_ability -> effect_free, so an ability
   decoded by rb_decode_card_ability releases its effect through THAT path.
   rb_effect_free is what every other caller uses (ability_queue.c pending
   actions, turn/triggers.c delayed_gained_effects, engine.c target_selection_eff,
   effects/rules.c, and rb_drop_value below).
   When this function omitted resource_on_select / opponent_action /
   activation_condition, a clone produced by rb_effect_deep_clone SHARED those
   three subtrees with its source: rb_effect_free leaked them, so the leak
   looked harmless — but rb_clear_gained_abilities_for_card
   (game_state_abilities.c:227) releases the source through rb_free_ability ->
   effect_free, which DOES free them. Freeing both halves is a double free.
   The three were not dead code: vm.c:925-926 populates resource_on_select and
   opponent_action, and vm.c:781-782 populates activation_condition for the 13
   abilities that carry activation_condition_parsed (rabuka.h:121-127).
   effect_free is static in vm.c and cannot be called from here, so the two
   bodies are kept identical by hand. Adding a heap-owning field to
   AbilityEffect (include/rabuka.h) requires adding it to BOTH, plus
   rb_effect_deep_clone below. */
void rb_effect_free(AbilityEffect *e) {
    if (!e) return;
    rb_free(e->text); rb_free(e->action); rb_free(e->source);
    rb_free(e->destination); rb_free(e->target);
    rb_free_condition(e->condition);
    for (int i = 0; i < e->n_child; i++) rb_effect_free(e->child[i]);
    for (int i = 0; i < e->n_options; i++) rb_effect_free(e->options[i]);
    for (int i = 0; i < e->n_extra; i++) { rb_free(e->extra_k[i]); rb_free(e->extra_v[i]); }
    rb_effect_free(e->primary_effect);
    rb_effect_free(e->alternative_effect);
    rb_effect_free(e->look_action);
    rb_effect_free(e->select_action);
    rb_effect_free(e->followup_action);
    rb_effect_free(e->optional_action);
    rb_effect_free(e->conditional_action);
    rb_effect_free(e->gained_effect);
    rb_effect_free(e->resource_on_select);
    rb_effect_free(e->opponent_action);
    rb_free_condition(e->result_condition);
    rb_free_condition(e->alternative_condition);
    rb_free_condition(e->activation_condition);
    rb_free(e);
}

/* put: write val into slot idx (mirrors Pool::put) */
void rb_put(RbPool *pool, int idx, AbilityEffect *val) {
    if (!pool || idx < 0 || idx >= RB_POOL_CAPACITY) return;
    pool->slots[idx] = val;
}

/* drop_value: drop the value at slot idx (mirrors Pool::drop_value) */
void rb_drop_value(RbPool *pool, int idx) {
    if (!pool || idx < 0 || idx >= RB_POOL_CAPACITY) return;
    if (pool->slots[idx]) {
        rb_effect_free(pool->slots[idx]);
        pool->slots[idx] = NULL;
    }
}

/* ── AbilityEffect deep clone (mirrors Rust #[derive(Clone)] on AbilityEffect,
    used by queue pending-action snapshots and deferred-cost parking) ── */
static Condition *rb_condition_clone(const Condition *c);
static void rb_cond_value_release(CondValue *v);
static void rb_condition_release(Condition *c);

/* Release a Condition THIS FILE allocated (rb_calloc / rb_strdup_heap), so the
   error paths below pair rb_free with rb_malloc.
   rb_free_condition (vm.c:257) is the equivalent for conditions vm.c itself
   decoded with calloc()/malloc(); it uses libc free(). Under -DRB_NO_MALLOC
   rb_free is a no-op, so calling rb_free_condition on an rb_malloc'd Condition
   would be a mismatched-free — hence this separate helper rather than reuse.
   The two cover the same fields: field keys, then each CondValue's owned
   member (string, nested Condition, or array). */
static void rb_condition_release(Condition *c) {
    if (!c) return;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        rb_free(c->fields[i].key);
        rb_cond_value_release(&c->fields[i].v);
    }
    rb_free(c);
}

static char *rb_strdup_heap(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s) + 1;
    char *p = (char *)rb_malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

/* Zeroed allocation through rb_malloc. rb_malloc has no calloc variant and does
   not zero under -DRB_NO_MALLOC (the arena hands back raw bytes), so the memset
   is required for correctness, not just tidiness. */
static void *rb_calloc(size_t count, size_t size) {
    void *p = rb_malloc(count * size);
    if (p) memset(p, 0, count * size);
    return p;
}

/* Releases an rb_cond_value_clone'd / rb_condition_clone'd subtree. Mirrors
   cond_value_free + rb_free_condition (vm.c:248-264) field for field, so a
   cloned Condition is releasable by either path without a double free.
   A nested condition comes from rb_condition_clone, so it is released with
   rb_condition_release, not rb_free_condition (which frees with libc free() and
   would mismatch an rb_malloc'd block under -DRB_NO_MALLOC). */
static void rb_cond_value_release(CondValue *v) {
    if (!v) return;
    if (v->tag == RB_TAG_STR || v->tag == RB_TAG_F64) rb_free(v->s);
    else if (v->tag == RB_TAG_OBJVAR || v->tag == RB_TAG_OBJECT) rb_condition_release(v->cond);
    else if (v->tag == RB_TAG_ARRAY) {
        for (uint32_t i = 0; i < v->arr_n; i++) rb_cond_value_release(&v->arr[i]);
        rb_free(v->arr);
    }
}

static CondValue *rb_cond_value_clone(const CondValue *src) {
    if (!src) return NULL;
    CondValue *out = (CondValue *)rb_calloc(1, sizeof(*out));
    if (!out) return NULL;
    out->tag = src->tag;
    out->i = src->i;
    out->b = src->b;
    if (src->tag == RB_TAG_STR || src->tag == RB_TAG_F64) {
        out->s = rb_strdup_heap(src->s);
        if (src->s && !out->s) { rb_free(out); return NULL; }
    } else if (src->tag == RB_TAG_OBJVAR || src->tag == RB_TAG_OBJECT) {
        out->cond = rb_condition_clone(src->cond);
        if (src->cond && !out->cond) { rb_free(out); return NULL; }
    } else if (src->tag == RB_TAG_ARRAY && src->arr_n > 0) {
        out->arr = (CondValue *)rb_calloc(src->arr_n, sizeof(*out->arr));
        if (!out->arr) { rb_free(out); return NULL; }
        for (uint32_t i = 0; i < src->arr_n; i++) {
            CondValue *copy = rb_cond_value_clone(&src->arr[i]);
            if (!copy) {
                for (uint32_t j = 0; j < i; j++) rb_cond_value_release(&out->arr[j]);
                rb_free(out->arr);
                rb_free(out);
                return NULL;
            }
            out->arr[i] = *copy;
            rb_free(copy);
        }
        out->arr_n = src->arr_n;
    }
    return out;
}

static Condition *rb_condition_clone(const Condition *c) {
    if (!c) return NULL;
    Condition *out = (Condition *)rb_calloc(1, sizeof(*out));
    if (!out) return NULL;
    out->variant = c->variant;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        /* Clamp at RB_MAX_COND_FIELD like every sibling loop below. Rust's
           Condition::conditions is a Vec and cannot overflow, but the C's
           `fields[]` is a fixed 64 array; a decoded source carrying more than
           64 fields wrote straight past the end of the clone. */
        if (out->n_fields >= RB_MAX_COND_FIELD) break;
        CondField *dst = &out->fields[out->n_fields];
        dst->key = rb_strdup_heap(c->fields[i].key);
        CondValue *value = rb_cond_value_clone(&c->fields[i].v);
        if (value) dst->v = *value;
        if (!dst->key || !value) {
            /* Only the container is released here; dst->v's owned members are
               reached through out->n_fields by rb_condition_release below, so
               nothing is double-freed on this failure path. */
            rb_free(value);
            rb_condition_release(out);
            return NULL;
        }
        rb_free(value);
        out->n_fields++;
    }
    return out;
}

AbilityEffect *rb_effect_deep_clone(const AbilityEffect *src) {
    if (!src) return NULL;
    AbilityEffect *e = (AbilityEffect *)rb_malloc(sizeof(AbilityEffect));
    if (!e) return NULL;
    memcpy(e, src, sizeof(*e));
    /* heap-owned strings */
    e->text = rb_strdup_heap(src->text);
    e->action = rb_strdup_heap(src->action);
    e->source = rb_strdup_heap(src->source);
    e->destination = rb_strdup_heap(src->destination);
    e->target = rb_strdup_heap(src->target);
    /* conditions */
    e->condition = src->condition ? rb_condition_clone(src->condition) : NULL;
    e->result_condition = src->result_condition ? rb_condition_clone(src->result_condition) : NULL;
    e->alternative_condition = src->alternative_condition ? rb_condition_clone(src->alternative_condition) : NULL;
    /* extras */
    e->n_extra = 0;
    for (int i = 0; i < src->n_extra && i < RB_MAX_EXTRA; i++) {
        e->extra_k[i] = rb_strdup_heap(src->extra_k[i]);
        e->extra_v[i] = rb_strdup_heap(src->extra_v[i]);
        if (!e->extra_k[i]) break;
        e->n_extra++;
    }
    /* children + named sub-effects (recursive) */
    e->n_child = 0;
    for (int i = 0; i < src->n_child && i < RB_MAX_CHILD; i++) {
        AbilityEffect *cc = rb_effect_deep_clone(src->child[i]);
        if (!cc) break;
        e->child[e->n_child++] = cc;
    }
    e->n_options = 0;
    for (int i = 0; i < src->n_options && i < RB_MAX_CHILD; i++) {
        AbilityEffect *opt = rb_effect_deep_clone(src->options[i]);
        if (!opt) break;
        e->options[e->n_options++] = opt;
    }
    e->primary_effect = src->primary_effect ? rb_effect_deep_clone(src->primary_effect) : NULL;
    e->alternative_effect = src->alternative_effect ? rb_effect_deep_clone(src->alternative_effect) : NULL;
    e->look_action = src->look_action ? rb_effect_deep_clone(src->look_action) : NULL;
    e->select_action = src->select_action ? rb_effect_deep_clone(src->select_action) : NULL;
    e->followup_action = src->followup_action ? rb_effect_deep_clone(src->followup_action) : NULL;
    e->optional_action = src->optional_action ? rb_effect_deep_clone(src->optional_action) : NULL;
    e->conditional_action = src->conditional_action ? rb_effect_deep_clone(src->conditional_action) : NULL;
    e->gained_effect = src->gained_effect ? rb_effect_deep_clone(src->gained_effect) : NULL;
    e->resource_on_select = src->resource_on_select ? rb_effect_deep_clone(src->resource_on_select) : NULL;
    e->opponent_action = src->opponent_action ? rb_effect_deep_clone(src->opponent_action) : NULL;
    /* activation_condition was omitted here too, so a clone shared the gate with
       its source: the source is released through effect_free (vm.c:443), which
       frees it, while rb_effect_free only leaked it — freeing both halves was a
       double free. Live data: vm.c:781-782 populates this for the 13 abilities
       carrying activation_condition_parsed. */
    e->activation_condition = src->activation_condition ? rb_condition_clone(src->activation_condition) : NULL;
    return e;
}

