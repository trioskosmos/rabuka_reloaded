#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* ── AbilityLogItem — faithful C translation of engine/src/ability/log.rs ──
   Each verdict is a tagged union mirroring the Rust AbilityLogItem enum, and
   every string field is heap-owned because log.rs:10-35 declares every one of
   them `String`. Condition children are a heap-allocated array too; release
   drained items with rb_log_free_item.

   THE GATE. Rust has ONE switch. `ABILITY_DEBUG` is read by log.rs at all four
   buffer entry points (push_verdict / drain_verdicts / buffer_len /
   clear_verdicts — log.rs:88, :95, :102, :122) and nowhere else: it is the
   complete definition of "are verdicts being recorded". This file had a SECOND,
   private static (`g_log_enabled`) set only by rb_log_set_enabled, while the
   header comment called the two interchangeable. They are not wired to the same
   call sites: debug.c's rb_debug_buf_drain_to_rule_log and
   rb_debug_buf_drain_to_structured_log both funnel through rb_log_push_verdict,
   so switching tracing on with rb_ability_debug_set(1) — the switch rabuka.h
   documents as "the Rust ABILITY_DEBUG atomic gate" — produced NO rule-log
   output at all, and rb_log_set_enabled(1) produced no stderr trace. One Rust
   flag, two halves, two disjoint call sets.

   `rb_log_recording()` makes ABILITY_DEBUG sufficient, as in Rust, and keeps
   rb_log_set_enabled as the C-only hook the suites drive (test_log_drain,
   test_target_selection, test_parity_char2, test_audit_actions and three more
   call it and never touch rb_ability_debug_set). */

#define RB_LOG_BUF_MAX 32

static RbAbilityLogItem g_log_buf[RB_LOG_BUF_MAX];
static int g_log_n = 0;
static int g_log_enabled = 0;

/* Dropped-push accounting. See include/rabuka.h for why this is not simply
   "raise RB_LOG_BUF_MAX": Rust's buffer is an unbounded Vec, so no fixed C
   bound is faithful, and turning the overflow from silent to announced is the
   only honest thing a bounded ring can do about it. Drop-newest is UNCHANGED —
   the oldest entries are still the ones that survive. */
static unsigned long g_log_dropped = 0;
static int g_log_overflow_reported = 0;

int  rb_log_dropped_count(void) { return (int)g_log_dropped; }
void rb_log_dropped_reset(void) { g_log_dropped = 0; g_log_overflow_reported = 0; }

static int rb_log_recording(void) {
    return g_log_enabled || rb_ability_debug_enabled();
}

void rb_log_set_enabled(int enabled) { g_log_enabled = enabled; }

/* Announce, on the FIRST drop of an episode and every 64th after it, that the
   ring is full and what it is throwing away. Deliberately NOT gated on
   rb_log_recording(): the pushes that overflow arrive from already-enabled
   tracing, and a report of lost evidence that obeys the same switch as the
   evidence is invisible exactly when it matters. */
static void log_report_overflow(const char *text) {
    g_log_dropped++;
    if (g_log_overflow_reported && (g_log_dropped % 64) != 0) return;
    g_log_overflow_reported = 1;
    fprintf(stderr,
            "[ABILITY_LOG_OVERFLOW] verdict ring full (%d slots); dropped=%lu "
            "dropping NEWEST (oldest kept) — last push len=%d head=\"%.120s\"\n",
            RB_LOG_BUF_MAX, g_log_dropped,
            text ? (int)strlen(text) : 0, text ? text : "");
}

static int log_alloc_slot(void) {
    if (g_log_n >= RB_LOG_BUF_MAX) return -1;
    return g_log_n++;
}

/* Release the strings this variant owns. The payload arrays are gone — every
   string field of RbAbilityLogItem is heap-owned (see include/rabuka.h), which
   is what makes the 635-byte ability text arrive whole. */
static void log_free_strings(RbAbilityLogItem *item) {
    if (!item) return;
    if (item->kind == RB_LOG_KIND_CONDITION) {
        rb_free(item->as.condition.text);          item->as.condition.text = NULL;
        rb_free(item->as.condition.condition_type); item->as.condition.condition_type = NULL;
        rb_free(item->as.condition.expectation);    item->as.condition.expectation = NULL;
        rb_free(item->as.condition.actual);         item->as.condition.actual = NULL;
    } else if (item->kind == RB_LOG_KIND_COST) {
        rb_free(item->as.cost.text);       item->as.cost.text = NULL;
        rb_free(item->as.cost.expectation); item->as.cost.expectation = NULL;
        rb_free(item->as.cost.actual);     item->as.cost.actual = NULL;
    } else if (item->kind == RB_LOG_KIND_EFFECT) {
        rb_free(item->as.effect.text);    item->as.effect.text = NULL;
        rb_free(item->as.effect.action);  item->as.effect.action = NULL;
        rb_free(item->as.effect.details); item->as.effect.details = NULL;
    }
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
    log_free_strings(item);
}

/* Release a slot's payload and zero it, so a reused slot can never hand a
   stale alias to a later double-free. Replaces the old `memset` of a fixed
   array payload, which was safe only because the payload had no owned memory. */
static void log_reset_slot(int idx) {
    memset(&g_log_buf[idx], 0, sizeof(g_log_buf[idx]));
}

static RbAbilityLogItem *log_deep_copy(const RbAbilityLogItem *src) {
    if (!src) return NULL;
    RbAbilityLogItem *dst = rb_malloc(sizeof(RbAbilityLogItem));
    if (!dst) return NULL;
    memset(dst, 0, sizeof(*dst));
    dst->kind = src->kind;
    if (src->kind == RB_LOG_KIND_CONDITION) {
        dst->as.condition.text          = rb_strdup2(src->as.condition.text);
        dst->as.condition.condition_type = rb_strdup2(src->as.condition.condition_type);
        dst->as.condition.expectation    = rb_strdup2(src->as.condition.expectation);
        dst->as.condition.actual         = rb_strdup2(src->as.condition.actual);
        dst->as.condition.passed         = src->as.condition.passed;
    } else if (src->kind == RB_LOG_KIND_COST) {
        dst->as.cost.text       = rb_strdup2(src->as.cost.text);
        dst->as.cost.expectation = rb_strdup2(src->as.cost.expectation);
        dst->as.cost.actual     = rb_strdup2(src->as.cost.actual);
        dst->as.cost.passed     = src->as.cost.passed;
        dst->as.cost.optional   = src->as.cost.optional;
    } else if (src->kind == RB_LOG_KIND_EFFECT) {
        dst->as.effect.text    = rb_strdup2(src->as.effect.text);
        dst->as.effect.action  = rb_strdup2(src->as.effect.action);
        dst->as.effect.details = rb_strdup2(src->as.effect.details);
    }
    if (dst->kind == RB_LOG_KIND_CONDITION && src->as.condition.n_children > 0) {
        int n = src->as.condition.n_children;
        dst->as.condition.children = rb_malloc((size_t)n * sizeof(RbAbilityLogItem));
        if (!dst->as.condition.children) {
            log_free_strings(dst);
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
                log_free_strings(dst);
                rb_free(dst);
                return NULL;
            }
            dst->as.condition.children[i] = *child_copy;
            rb_free(child_copy);
        }
        dst->as.condition.n_children = n;
    }
    return dst;
}

void rb_log_free_item(RbAbilityLogItem *item) {
    if (!item) return;
    log_free_tree(item);
}

void rb_log_push_verdict(const char *text, const char *kind, int passed) {
    if (!rb_log_recording()) return;
    int idx = log_alloc_slot();
    if (idx < 0) { log_report_overflow(text); return; }
    log_reset_slot(idx);
    g_log_buf[idx].kind = RB_LOG_KIND_CONDITION;
    g_log_buf[idx].as.condition.text = rb_strdup2(text ? text : "");
    g_log_buf[idx].as.condition.condition_type = rb_strdup2(kind ? kind : "");
    g_log_buf[idx].as.condition.passed = passed;
}

void rb_log_push_verdict_condition(const char *text, const char *condition_type,
                                   const char *expectation, const char *actual,
                                   int passed) {
    if (!rb_log_recording()) return;
    int idx = log_alloc_slot();
    if (idx < 0) { log_report_overflow(text); return; }
    log_reset_slot(idx);
    g_log_buf[idx].kind = RB_LOG_KIND_CONDITION;
    g_log_buf[idx].as.condition.text = rb_strdup2(text ? text : "");
    g_log_buf[idx].as.condition.condition_type = rb_strdup2(condition_type ? condition_type : "");
    g_log_buf[idx].as.condition.expectation = rb_strdup2(expectation ? expectation : "");
    g_log_buf[idx].as.condition.actual = rb_strdup2(actual ? actual : "");
    g_log_buf[idx].as.condition.passed = passed;
}

void rb_log_push_verdict_cost(const char *text, const char *expectation,
                              const char *actual, int passed, int optional) {
    if (!rb_log_recording()) return;
    int idx = log_alloc_slot();
    if (idx < 0) { log_report_overflow(text); return; }
    log_reset_slot(idx);
    g_log_buf[idx].kind = RB_LOG_KIND_COST;
    g_log_buf[idx].as.cost.text = rb_strdup2(text ? text : "");
    g_log_buf[idx].as.cost.expectation = rb_strdup2(expectation ? expectation : "");
    g_log_buf[idx].as.cost.actual = rb_strdup2(actual ? actual : "");
    g_log_buf[idx].as.cost.passed = passed;
    g_log_buf[idx].as.cost.optional = optional;
}

void rb_log_push_verdict_effect(const char *text, const char *action, const char *details) {
    if (!rb_log_recording()) return;
    int idx = log_alloc_slot();
    if (idx < 0) { log_report_overflow(text); return; }
    log_reset_slot(idx);
    g_log_buf[idx].kind = RB_LOG_KIND_EFFECT;
    g_log_buf[idx].as.effect.text = rb_strdup2(text ? text : "");
    g_log_buf[idx].as.effect.action = rb_strdup2(action ? action : "");
    g_log_buf[idx].as.effect.details = rb_strdup2(details ? details : "");
}

void rb_log_push_verdict_child(int parent_index, const RbAbilityLogItem *child) {
    if (!rb_log_recording()) return;
    if (parent_index < 0 || parent_index >= g_log_n) return;
    if (g_log_buf[parent_index].kind != RB_LOG_KIND_CONDITION) return;
    RbAbilityLogItem *parent = &g_log_buf[parent_index];
    if (parent->as.condition.n_children >= RB_LOG_MAX_CHILDREN) {
        /* RB_LOG_MAX_CHILDREN is 8 against Rust's unbounded `children: Vec`.
           Same defect as the ring bound above: a bounded field that drops
           evidence quietly. Counted and announced, not silently swallowed. */
        log_report_overflow(child && child->kind == RB_LOG_KIND_CONDITION
                            ? child->as.condition.text : NULL);
        return;
    }
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
    if (!rb_log_recording()) return;
    if (!item) return;
    int idx = log_alloc_slot();
    if (idx < 0) {
        log_report_overflow(item->kind == RB_LOG_KIND_CONDITION ? item->as.condition.text : NULL);
        return;
    }
    log_reset_slot(idx);
    RbAbilityLogItem *copy = log_deep_copy(item);
    if (!copy) return;
    g_log_buf[idx] = *copy;
    rb_free(copy);
}

int rb_log_buffer_len(void) {
    if (!rb_log_recording()) return 0;
    return g_log_n;
}

void rb_log_clear_verdicts(void) {
    if (!rb_log_recording()) return;
    for (int i = 0; i < g_log_n; i++) {
        log_free_tree(&g_log_buf[i]);
    }
    g_log_n = 0;
    g_log_overflow_reported = 0;
}

int rb_log_drain_verdicts(RbAbilityLogItem *out, int max) {
    if (!rb_log_recording()) return 0;
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
    g_log_overflow_reported = 0;
    return n;
}

/* log.rs:97-108 drain_verdicts_since: `buf.drain(start_index..)` removes and
 * returns [start_index, end) and KEEPS [0, start_index) — the pre-snapshot head
 * survives, the post-snapshot tail is what the caller receives.
 *
 * Ported: the C is capped, so up to `max` post-snapshot entries are handed to
 * the caller. Survivors are therefore the head [0, start_index) PLUS any tail
 * the cap left behind ([start_index+n, end)). Everything else is discarded and
 * its tree freed here (the caller owns the deep copies in `out`).
 *
 * Order matters: the drained ORIGINALS are freed first, and they are disjoint
 * from the tail SOURCE region, so the compaction memmove never reads a freed
 * tree. The head is never freed or moved. The vacated high slots are zeroed so
 * a later push's log_reset_slot() cannot free a stale alias of a tail entry
 * that now lives at a lower index. */
int rb_log_drain_verdicts_since(int start_index, RbAbilityLogItem *out, int max) {
    if (!rb_log_recording()) return 0;
    if (start_index < 0) start_index = 0;
    if (start_index >= g_log_n) return 0;
    if (max < 0) max = 0;
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
    /* Discarded originals [start_index, start_index+n): caller now owns copies. */
    for (int i = start_index; i < start_index + n; i++) {
        log_free_tree(&g_log_buf[i]);
    }
    /* Keep head [0, start_index) then undrained tail [start_index+n, end). */
    int tail = g_log_n - (start_index + n);
    if (tail > 0) {
        memmove(&g_log_buf[start_index], &g_log_buf[start_index + n],
                (size_t)tail * sizeof(RbAbilityLogItem));
        /* Clear vacated high slots so no stale alias of a moved tail entry is
         * left for a later push's log_dispose_slot() to double free. */
        memset(&g_log_buf[start_index + tail], 0,
               (size_t)(g_log_n - (start_index + tail)) * sizeof(RbAbilityLogItem));
    }
    g_log_n = start_index + tail; /* == g_log_n - n : everything not drained */
    return n;
}
