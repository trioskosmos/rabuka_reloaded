/* tests/test_log_drain.c — rb_log_drain_verdicts_since retention semantics.
 *
 * log.rs:97-108 drain_verdicts_since does `buf.drain(start_index..)`: the
 * [start_index, end) tail is removed and returned to the caller, and the
 * [0, start_index) head is KEPT. The C port previously had the compaction
 * bounds inverted, so it freed the head survivors instead of the discarded
 * tail — and, because the tail was memmoved down over the head, the trailing
 * free pass freed the tail ORIGINALS that the moved survivors aliased,
 * leaving dangling children pointers (use-after-free / double-free).
 *
 * These tests drive the drain directly and assert:
 *   1. pre-snapshot verdicts SURVIVE; the post-snapshot tail is what is drained
 *   2. a capped drain carries the undrained tail forward intact alongside the head
 *   3. the buffer stays consistent across repeated drains
 *   4. the resolver-level drain_since (the peer call site) behaves the same
 * Every pushed verdict carries a heap-allocated child so the freed/aliased
 * arrays are real heap blocks — observable under a sanitizer. */

#include "rabuka.h"
#include <stdio.h>
#include <string.h>

/* Not yet in rabuka.h; forward-declared here exactly as src/ability/resolver.c
 * does. */
int  rb_log_drain_verdicts_since(int start_index, RbAbilityLogItem *out, int max);
void rb_resolver_drain_verdicts_since(GameState *g, int snapshot);

static int failures;

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    int a_value = (actual); \
    int e_value = (expected); \
    if (a_value != e_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, a_value, e_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_STR(got, expected, message) do { \
    const char *g_s = (got); \
    const char *e_s = (expected); \
    if (strcmp(g_s, e_s) != 0) { \
        fprintf(stderr, "FAIL: %s (got \"%s\" expected \"%s\")\n", message, g_s, e_s); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* Push a CONDITION verdict owning one heap-allocated child. The child array is
 * the memory whose use-after-free / double-free the bug produced. */
static void push_cond_with_child(const char *text, const char *child_text) {
    int idx = rb_log_buffer_len();
    rb_log_push_verdict_condition(text, "hand_count", "ge:1", "1", 1);
    RbAbilityLogItem child;
    memset(&child, 0, sizeof(child));
    child.kind = RB_LOG_KIND_CONDITION;
    strncpy(child.as.condition.text, child_text, 255);
    child.as.condition.text[255] = '\0';
    child.as.condition.condition_type[0] = '\0';
    child.as.condition.passed = 1;
    child.as.condition.n_children = 0;
    child.as.condition.children = NULL;
    rb_log_push_verdict_child(idx, &child);
}

static void free_items(RbAbilityLogItem *items, int n) {
    for (int i = 0; i < n; i++) rb_log_free_item(&items[i]);
}

static const char *text_of(const RbAbilityLogItem *it) {
    if (it->kind != RB_LOG_KIND_CONDITION) return "";
    return it->as.condition.text;
}

/* 1. Uncapped drain: head survives, post-snapshot tail is returned. */
static void test_head_survives_tail_drained(void) {
    rb_log_set_enabled(1);
    rb_log_clear_verdicts();

    push_cond_with_child("H0", "H0c");
    push_cond_with_child("H1", "H1c");
    int snapshot = rb_log_buffer_len();
    CHECK_EQ(snapshot, 2, "two pre-snapshot verdicts are buffered");

    push_cond_with_child("T0", "T0c");
    push_cond_with_child("T1", "T1c");
    push_cond_with_child("T2", "T2c");
    CHECK_EQ(rb_log_buffer_len(), 5, "three post-snapshot verdicts are appended");

    RbAbilityLogItem out[64];
    int n = rb_log_drain_verdicts_since(snapshot, out, 64);
    CHECK_EQ(n, 3, "drain_since returns exactly the post-snapshot verdicts");
    CHECK_STR(text_of(&out[0]), "T0", "drained[0] is the first post-snapshot verdict");
    CHECK_STR(text_of(&out[1]), "T1", "drained[1] is the second post-snapshot verdict");
    CHECK_STR(text_of(&out[2]), "T2", "drained[2] is the third post-snapshot verdict");

    CHECK_EQ(rb_log_buffer_len(), 2, "pre-snapshot head survives the drain");

    RbAbilityLogItem all[64];
    int m = rb_log_drain_verdicts(all, 64);
    CHECK_EQ(m, 2, "surviving head holds exactly the two pre-snapshot verdicts");
    CHECK_STR(text_of(&all[0]), "H0", "survivor[0] is the original pre-snapshot H0");
    CHECK_STR(text_of(&all[1]), "H1", "survivor[1] is the original pre-snapshot H1");
    CHECK_EQ(all[0].as.condition.n_children, 1, "survivor H0 kept its heap child");
    CHECK_EQ(all[1].as.condition.n_children, 1, "survivor H1 kept its heap child");

    free_items(out, n);
    free_items(all, m);
    CHECK_EQ(rb_log_buffer_len(), 0, "buffer empty after draining everything");
    rb_log_set_enabled(0);
}

/* 2. Capped drain exercises the compaction + memmove path. This is where the
 * old code freed aliased tail originals and left dangling survivors. */
static void test_capped_drain_compaction(void) {
    rb_log_set_enabled(1);
    rb_log_clear_verdicts();

    push_cond_with_child("H0", "H0c");
    push_cond_with_child("H1", "H1c");
    int snapshot = rb_log_buffer_len(); /* 2 */

    char t[32], c[32];
    for (int i = 0; i < 6; i++) {
        sprintf(t, "T%d", i);
        sprintf(c, "T%dc", i);
        push_cond_with_child(t, c);
    }
    CHECK_EQ(rb_log_buffer_len(), 8, "two head plus six tail verdicts are buffered");

    /* Cap the drain at 3: T0..T2 are handed over, T3..T5 are the undrained
     * tail that compaction must carry forward intact next to the head. */
    RbAbilityLogItem out[8];
    int n = rb_log_drain_verdicts_since(snapshot, out, 3);
    CHECK_EQ(n, 3, "capped drain returns exactly max post-snapshot verdicts");
    CHECK_STR(text_of(&out[0]), "T0", "capped drained[0] is T0");
    CHECK_STR(text_of(&out[1]), "T1", "capped drained[1] is T1");
    CHECK_STR(text_of(&out[2]), "T2", "capped drained[2] is T2");

    /* Survivors must be head H0,H1 plus undrained tail T3,T4,T5 == 5 entries. */
    CHECK_EQ(rb_log_buffer_len(), 5, "head plus undrained tail survive a capped drain");

    RbAbilityLogItem all[16];
    int m = rb_log_drain_verdicts(all, 16);
    CHECK_EQ(m, 5, "five survivors remain after the capped drain");
    CHECK_STR(text_of(&all[0]), "H0", "survivor[0] is pre-snapshot H0");
    CHECK_STR(text_of(&all[1]), "H1", "survivor[1] is pre-snapshot H1");
    CHECK_STR(text_of(&all[2]), "T3", "survivor[2] is the first undrained tail T3");
    CHECK_STR(text_of(&all[3]), "T4", "survivor[3] is the second undrained tail T4");
    CHECK_STR(text_of(&all[4]), "T5", "survivor[4] is the third undrained tail T5");
    /* Moved tail entries must still own their heap children (not NULLed/dangling). */
    CHECK_EQ(all[2].as.condition.n_children, 1, "moved tail T3 kept its heap child");
    CHECK_EQ(all[3].as.condition.n_children, 1, "moved tail T4 kept its heap child");
    CHECK_EQ(all[4].as.condition.n_children, 1, "moved tail T5 kept its heap child");

    free_items(out, n);
    free_items(all, m);
    CHECK_EQ(rb_log_buffer_len(), 0, "buffer empty after draining everything");
    rb_log_set_enabled(0);
}

/* 3. Repeated drains keep the buffer consistent and accumulate the head. */
static void test_repeated_drains_stay_consistent(void) {
    rb_log_set_enabled(1);
    rb_log_clear_verdicts();

    for (int r = 0; r < 6; r++) {
        char h[32], hc[32];
        sprintf(h, "H%d", r);
        sprintf(hc, "H%dc", r);
        push_cond_with_child(h, hc);

        int snapshot = rb_log_buffer_len();
        CHECK_EQ(snapshot, r + 1, "head grows by one each round");

        char t0[32], t0c[32], t1[32], t1c[32];
        sprintf(t0, "T%da", r); sprintf(t0c, "T%da-c", r);
        sprintf(t1, "T%db", r); sprintf(t1c, "T%db-c", r);
        push_cond_with_child(t0, t0c);
        push_cond_with_child(t1, t1c);

        RbAbilityLogItem out[8];
        int n = rb_log_drain_verdicts_since(snapshot, out, 8);
        CHECK_EQ(n, 2, "each round drains the two post-snapshot verdicts");
        free_items(out, n);
        CHECK_EQ(rb_log_buffer_len(), snapshot, "head length is stable across the drain");
    }

    CHECK_EQ(rb_log_buffer_len(), 6, "six head verdicts accumulated across rounds");
    RbAbilityLogItem all[16];
    int m = rb_log_drain_verdicts(all, 16);
    CHECK_EQ(m, 6, "final drain returns the six accumulated heads");
    for (int r = 0; r < 6 && r < m; r++) {
        char h[32];
        sprintf(h, "H%d", r);
        CHECK_STR(text_of(&all[r]), h, "accumulated head order is preserved");
    }
    free_items(all, m);
    CHECK_EQ(rb_log_buffer_len(), 0, "buffer empty at end of repeated drains");
    rb_log_set_enabled(0);
}

/* 4. The peer call site: rb_resolver_drain_verdicts_since on the condition-gate
 * success path must keep the pre-snapshot head, not free it. */
static void test_resolver_drain_preserves_head(void) {
    GameState g;
    memset(&g, 0, sizeof(g));
    rb_log_set_enabled(1);
    rb_log_clear_verdicts();

    push_cond_with_child("H0", "H0c");
    int snapshot = rb_log_buffer_len();
    push_cond_with_child("T0", "T0c");

    rb_resolver_drain_verdicts_since(&g, snapshot);
    CHECK_EQ(rb_log_buffer_len(), 1, "resolver drain_since keeps the pre-snapshot head");

    RbAbilityLogItem all[8];
    int m = rb_log_drain_verdicts(all, 8);
    CHECK_EQ(m, 1, "resolver drain_since left exactly the head");
    CHECK_STR(text_of(&all[0]), "H0", "resolver drain_since kept H0");
    free_items(all, m);
    CHECK_EQ(rb_log_buffer_len(), 0, "buffer empty after the resolver drain path");
    rb_log_set_enabled(0);
}

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    test_head_survives_tail_drained();
    test_capped_drain_compaction();
    test_repeated_drains_stay_consistent();
    test_resolver_drain_preserves_head();
    if (failures) return 1;
    printf("ALL LOG DRAIN CHECKS PASSED\n");
    return 0;
}
