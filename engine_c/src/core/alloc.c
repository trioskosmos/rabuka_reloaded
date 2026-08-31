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

