#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* ── AbilityLogItem — faithful C translation of engine/src/ability/log.rs ──
   Each verdict is a tagged union mirroring the Rust AbilityLogItem enum.
   Gated by ABILITY_DEBUG (use rb_log_set_enabled / rb_ability_debug_set).
   Condition children are heap-allocated; release drained items with rb_log_free_item. */

#define RB_LOG_BUF_MAX 32

static RbAbilityLogItem g_log_buf[RB_LOG_BUF_MAX];
static int g_log_n = 0;
static int g_log_enabled = 0;

void rb_log_set_enabled(int enabled) { g_log_enabled = enabled; }

static int log_alloc_slot(void) {
    if (g_log_n >= RB_LOG_BUF_MAX) return -1;
    return g_log_n++;
}

static void log_free_tree(RbAbilityLogItem *item) {
    if (!item) return;
    if (item->kind == RB_LOG_KIND_CONDITION) {
        for (int i = 0; i < item->as.condition.n_children; i++) {
            log_free_tree(&item->as.condition.children[i]);
        }
        if (item->as.condition.children) {
            rb_free(item->as.condition.children);
            item->as.condition.children = NULL;
        }
        item->as.condition.n_children = 0;
    }
}

static void log_dispose_slot(int idx) {
    if (idx >= 0 && idx < g_log_n) {
        log_free_tree(&g_log_buf[idx]);
    }
}

static RbAbilityLogItem *log_deep_copy(const RbAbilityLogItem *src) {
    if (!src) return NULL;
    RbAbilityLogItem *dst = rb_malloc(sizeof(RbAbilityLogItem));
    if (!dst) return NULL;
    *dst = *src;
    if (dst->kind == RB_LOG_KIND_CONDITION && dst->as.condition.n_children > 0) {
        int n = dst->as.condition.n_children;
        dst->as.condition.children = rb_malloc(n * sizeof(RbAbilityLogItem));
        if (!dst->as.condition.children) {
            rb_free(dst);
            return NULL;
        }
        for (int i = 0; i < n; i++) {
            RbAbilityLogItem *child_copy = log_deep_copy(&src->as.condition.children[i]);
            if (!child_copy) {
                for (int j = 0; j < i; j++) {
                    log_free_tree(&dst->as.condition.children[j]);
                }
                rb_free(dst->as.condition.children);
                rb_free(dst);
                return NULL;
            }
            dst->as.condition.children[i] = *child_copy;
            rb_free(child_copy);
        }
    } else {
        dst->as.condition.children = NULL;
    }
    return dst;
}

void rb_log_free_item(RbAbilityLogItem *item) {
    if (!item) return;
    log_free_tree(item);
}

void rb_log_push_verdict(const char *text, const char *kind, int passed) {
    if (!g_log_enabled) return;
    int idx = log_alloc_slot();
    if (idx < 0) return;
    log_dispose_slot(idx);
    g_log_buf[idx].kind = RB_LOG_KIND_CONDITION;
    memset(&g_log_buf[idx].as.condition, 0, sizeof(g_log_buf[idx].as.condition));
    strncpy(g_log_buf[idx].as.condition.text, text ? text : "", 255);
    g_log_buf[idx].as.condition.text[255] = '\0';
    strncpy(g_log_buf[idx].as.condition.condition_type, kind ? kind : "", 63);
    g_log_buf[idx].as.condition.condition_type[63] = '\0';
    g_log_buf[idx].as.condition.passed = passed;
    g_log_buf[idx].as.condition.n_children = 0;
    g_log_buf[idx].as.condition.children = NULL;
}

void rb_log_push_verdict_condition(const char *text, const char *condition_type,
                                   const char *expectation, const char *actual,
                                   int passed) {
    if (!g_log_enabled) return;
    int idx = log_alloc_slot();
    if (idx < 0) return;
    log_dispose_slot(idx);
    g_log_buf[idx].kind = RB_LOG_KIND_CONDITION;
    memset(&g_log_buf[idx].as.condition, 0, sizeof(g_log_buf[idx].as.condition));
    strncpy(g_log_buf[idx].as.condition.text, text ? text : "", 255);
    g_log_buf[idx].as.condition.text[255] = '\0';
    strncpy(g_log_buf[idx].as.condition.condition_type, condition_type ? condition_type : "", 63);
    g_log_buf[idx].as.condition.condition_type[63] = '\0';
    strncpy(g_log_buf[idx].as.condition.expectation, expectation ? expectation : "", 255);
    g_log_buf[idx].as.condition.expectation[255] = '\0';
    strncpy(g_log_buf[idx].as.condition.actual, actual ? actual : "", 255);
    g_log_buf[idx].as.condition.actual[255] = '\0';
    g_log_buf[idx].as.condition.passed = passed;
    g_log_buf[idx].as.condition.n_children = 0;
    g_log_buf[idx].as.condition.children = NULL;
}

void rb_log_push_verdict_cost(const char *text, const char *expectation,
                              const char *actual, int passed, int optional) {
    if (!g_log_enabled) return;
    int idx = log_alloc_slot();
    if (idx < 0) return;
    log_dispose_slot(idx);
    g_log_buf[idx].kind = RB_LOG_KIND_COST;
    memset(&g_log_buf[idx].as.cost, 0, sizeof(g_log_buf[idx].as.cost));
    strncpy(g_log_buf[idx].as.cost.text, text ? text : "", 255);
    g_log_buf[idx].as.cost.text[255] = '\0';
    strncpy(g_log_buf[idx].as.cost.expectation, expectation ? expectation : "", 255);
    g_log_buf[idx].as.cost.expectation[255] = '\0';
    strncpy(g_log_buf[idx].as.cost.actual, actual ? actual : "", 255);
    g_log_buf[idx].as.cost.actual[255] = '\0';
    g_log_buf[idx].as.cost.passed = passed;
    g_log_buf[idx].as.cost.optional = optional;
}

void rb_log_push_verdict_effect(const char *text, const char *action, const char *details) {
    if (!g_log_enabled) return;
    int idx = log_alloc_slot();
    if (idx < 0) return;
    log_dispose_slot(idx);
    g_log_buf[idx].kind = RB_LOG_KIND_EFFECT;
    memset(&g_log_buf[idx].as.effect, 0, sizeof(g_log_buf[idx].as.effect));
    strncpy(g_log_buf[idx].as.effect.text, text ? text : "", 255);
    g_log_buf[idx].as.effect.text[255] = '\0';
    strncpy(g_log_buf[idx].as.effect.action, action ? action : "", 63);
    g_log_buf[idx].as.effect.action[63] = '\0';
    strncpy(g_log_buf[idx].as.effect.details, details ? details : "", 255);
    g_log_buf[idx].as.effect.details[255] = '\0';
}

void rb_log_push_verdict_child(int parent_index, const RbAbilityLogItem *child) {
    if (!g_log_enabled) return;
    if (parent_index < 0 || parent_index >= g_log_n) return;
    if (g_log_buf[parent_index].kind != RB_LOG_KIND_CONDITION) return;
    RbAbilityLogItem *parent = &g_log_buf[parent_index];
    if (parent->as.condition.n_children >= RB_LOG_MAX_CHILDREN) return;
    if (!child) return;
    if (!parent->as.condition.children) {
        parent->as.condition.children = rb_malloc(RB_LOG_MAX_CHILDREN * sizeof(RbAbilityLogItem));
        if (!parent->as.condition.children) return;
        memset(parent->as.condition.children, 0, RB_LOG_MAX_CHILDREN * sizeof(RbAbilityLogItem));
    }
    RbAbilityLogItem *copy = log_deep_copy(child);
    if (!copy) return;
    parent->as.condition.children[parent->as.condition.n_children] = *copy;
    parent->as.condition.n_children++;
    rb_free(copy);
}

void rb_log_push_verdict_item(const RbAbilityLogItem *item) {
    if (!g_log_enabled) return;
    if (!item) return;
    int idx = log_alloc_slot();
    if (idx < 0) return;
    log_dispose_slot(idx);
    RbAbilityLogItem *copy = log_deep_copy(item);
    if (!copy) return;
    g_log_buf[idx] = *copy;
    rb_free(copy);
}

int rb_log_buffer_len(void) {
    if (!g_log_enabled) return 0;
    return g_log_n;
}

void rb_log_clear_verdicts(void) {
    if (!g_log_enabled) return;
    for (int i = 0; i < g_log_n; i++) {
        log_free_tree(&g_log_buf[i]);
    }
    g_log_n = 0;
}

int rb_log_drain_verdicts(RbAbilityLogItem *out, int max) {
    if (!g_log_enabled) return 0;
    int n = g_log_n < max ? g_log_n : max;
    for (int i = 0; i < n; i++) {
        RbAbilityLogItem *copy = log_deep_copy(&g_log_buf[i]);
        if (copy) {
            out[i] = *copy;
            rb_free(copy);
        } else {
            memset(&out[i], 0, sizeof(RbAbilityLogItem));
        }
    }
    for (int i = 0; i < g_log_n; i++) {
        log_free_tree(&g_log_buf[i]);
    }
    g_log_n = 0;
    return n;
}

int rb_log_drain_verdicts_since(int start_index, RbAbilityLogItem *out, int max) {
    if (!g_log_enabled) return 0;
    if (start_index < 0) start_index = 0;
    if (start_index >= g_log_n) return 0;
    int avail = g_log_n - start_index;
    int n = avail < max ? avail : max;
    for (int i = 0; i < n; i++) {
        RbAbilityLogItem *copy = log_deep_copy(&g_log_buf[start_index + i]);
        if (copy) {
            out[i] = *copy;
            rb_free(copy);
        } else {
            memset(&out[i], 0, sizeof(RbAbilityLogItem));
        }
    }
    int remaining = g_log_n - (start_index + n);
    for (int i = 0; i < remaining; i++) {
        log_dispose_slot(i);
        g_log_buf[i] = g_log_buf[start_index + n + i];
    }
    for (int i = remaining; i < g_log_n; i++) {
        log_free_tree(&g_log_buf[i]);
    }
    g_log_n = remaining;
    return n;
}
