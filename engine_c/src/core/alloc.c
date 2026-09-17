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

void rb_effect_free(AbilityEffect *e) {
    if (!e) return;
    rb_free(e->text); rb_free(e->action); rb_free(e->source);
    rb_free(e->destination); rb_free(e->target);
    rb_free_condition(e->condition);
    for (int i = 0; i < e->n_child; i++) rb_effect_free(e->child[i]);
    for (int i = 0; i < e->n_extra; i++) { rb_free(e->extra_k[i]); rb_free(e->extra_v[i]); }
    rb_effect_free(e->primary_effect);
    rb_effect_free(e->alternative_effect);
    rb_effect_free(e->followup_action);
    rb_effect_free(e->optional_action);
    rb_effect_free(e->conditional_action);
    rb_free_condition(e->result_condition);
    rb_free_condition(e->alternative_condition);
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
static char *rb_strdup_heap(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s) + 1;
    char *p = (char *)rb_malloc(n);
    if (p) memcpy(p, s, n);
    return p;
}

static Condition *rb_condition_clone(const Condition *c) {
    return rb_condition_clone(c);
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
    e->primary_effect = src->primary_effect ? rb_effect_deep_clone(src->primary_effect) : NULL;
    e->alternative_effect = src->alternative_effect ? rb_effect_deep_clone(src->alternative_effect) : NULL;
    e->followup_action = src->followup_action ? rb_effect_deep_clone(src->followup_action) : NULL;
    e->optional_action = src->optional_action ? rb_effect_deep_clone(src->optional_action) : NULL;
    e->conditional_action = src->conditional_action ? rb_effect_deep_clone(src->conditional_action) : NULL;
    return e;
}

