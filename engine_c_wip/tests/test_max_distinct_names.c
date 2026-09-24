#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_CARDS 6
#define MAX_NAMES 3
#define MAX_UNIQUE 128

typedef struct {
    int distinct;
    bool collision;
} DistinctNamesResult;

typedef struct {
    const char *names[MAX_NAMES];
    int count;
} NameSet;

static bool name_equals(const char *a, const char *b) {
    return strcmp(a, b) == 0;
}

static bool seen_contains(const char *seen[], int count, const char *name) {
    for (int i = 0; i < count; i++) {
        if (name_equals(seen[i], name)) return true;
    }
    return false;
}

static void seen_insert(const char *seen[], int *count, const char *name) {
    if (!seen_contains(seen, *count, name)) seen[(*count)++] = name;
}

static DistinctNamesResult brute_force(const NameSet name_sets[], int name_set_count) {
    DistinctNamesResult result = {0, false};
    typedef struct {
        int idx;
        const char *seen[MAX_UNIQUE];
        int seen_count;
        bool collided;
    } StackEntry;

    StackEntry *stack = malloc(sizeof(StackEntry) * 4096);
    if (!stack) {
        fprintf(stderr, "FAIL: brute-force stack allocation failed\n");
        result.distinct = -1;
        result.collision = true;
        return result;
    }
    int top = 0;
    stack[top++] = (StackEntry){0, {0}, 0, false};
    bool found_no_collision = false;

    while (top > 0) {
        StackEntry current = stack[--top];
        if (current.idx == name_set_count) {
            if (current.seen_count > result.distinct) result.distinct = current.seen_count;
            if (!current.collided) found_no_collision = true;
            continue;
        }
        for (int i = 0; i < name_sets[current.idx].count; i++) {
            const char *name = name_sets[current.idx].names[i];
            StackEntry next = current;
            next.idx++;
            next.collided = current.collided || seen_contains(current.seen, current.seen_count, name);
            next.seen_count = current.seen_count;
            seen_insert(next.seen, &next.seen_count, name);
            if (top >= 4096) {
                fprintf(stderr, "FAIL: brute-force stack overflow\n");
                result.distinct = -1;
                result.collision = true;
                free(stack);
                return result;
            }
            stack[top++] = next;
        }
    }

    result.collision = !found_no_collision;
    free(stack);
    return result;
}

static DistinctNamesResult max_distinct_names(const NameSet name_sets[], int name_set_count) {
    DistinctNamesResult result = {0, false};
    if (name_set_count == 0) return result;
    for (int i = 0; i < name_set_count; i++) {
        if (name_sets[i].count == 0) {
            result.collision = true;
            return result;
        }
    }

    const char *unique[MAX_UNIQUE];
    int unique_count = 0;
    uint64_t option_masks[MAX_CARDS];
    for (int i = 0; i < name_set_count; i++) {
        option_masks[i] = 0;
        for (int j = 0; j < name_sets[i].count; j++) {
            const char *name = name_sets[i].names[j];
            int id = -1;
            for (int u = 0; u < unique_count; u++) {
                if (name_equals(unique[u], name)) {
                    id = u;
                    break;
                }
            }
            if (id < 0) {
                if (unique_count >= MAX_UNIQUE) {
                    fprintf(stderr, "FAIL: name universe exceeds bitmask size\n");
                    result.distinct = -1;
                    result.collision = true;
                    return result;
                }
                id = unique_count++;
                unique[id] = name;
            }
            option_masks[i] |= UINT64_C(1) << id;
        }
    }

    uint64_t frontier[MAX_CARDS * 4];
    int frontier_count = 1;
    frontier[0] = 0;
    for (int card = 0; card < name_set_count; card++) {
        uint64_t next_frontier[MAX_CARDS * 4];
        int next_count = 0;
        for (int i = 0; i < frontier_count; i++) {
            uint64_t rest = option_masks[card];
            while (rest != 0) {
                uint64_t bit = rest & (~rest + 1);
                rest &= ~bit;
                next_frontier[next_count++] = frontier[i] | bit;
            }
        }
        int kept = 0;
        for (int i = 0; i < next_count; i++) {
            bool dominated = false;
            for (int j = 0; j < kept; j++) {
                if ((next_frontier[j] & next_frontier[i]) == next_frontier[i]) {
                    dominated = true;
                    break;
                }
            }
            if (!dominated) next_frontier[kept++] = next_frontier[i];
        }
        for (int i = 0; i < kept; i++) frontier[i] = next_frontier[i];
        frontier_count = kept;
    }

    int best = 0;
    bool collision_free = false;
    for (int i = 0; i < frontier_count; i++) {
        int count = 0;
        for (int bit = 0; bit < unique_count; bit++) {
            if (frontier[i] & (UINT64_C(1) << bit)) count++;
        }
        if (count > best) best = count;
        if (count == name_set_count) collision_free = true;
    }
    result.distinct = best;
    result.collision = !collision_free;
    return result;
}

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
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

static uint64_t next_random(uint64_t *seed) {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    return *seed;
}

static void dp_matches_brute_force(void) {
    const char *names[] = {"a", "b", "c", "d"};
    uint64_t seed = UINT64_C(0x2545F4914F6CDD1D);
    for (uint32_t test_case = 0; test_case < 2000; test_case++) {
        int name_set_count = (int)(next_random(&seed) % 6) + 1;
        NameSet name_sets[MAX_CARDS] = {0};
        for (int i = 0; i < name_set_count; i++) {
            name_sets[i].count = (int)(next_random(&seed) % 3) + 1;
            for (int j = 0; j < name_sets[i].count; j++) {
                name_sets[i].names[j] = names[next_random(&seed) % 4];
            }
        }
        DistinctNamesResult expect = brute_force(name_sets, name_set_count);
        DistinctNamesResult got = max_distinct_names(name_sets, name_set_count);
        char distinct_message[128];
        char collision_message[128];
        snprintf(distinct_message, sizeof distinct_message, "distinct mismatch in case %u", test_case);
        snprintf(collision_message, sizeof collision_message, "collision mismatch in case %u", test_case);
        CHECK_EQ(got.distinct, expect.distinct, distinct_message);
        if (got.collision != expect.collision) {
            fprintf(stderr, "FAIL: %s (got %s expected %s)\n", collision_message,
                    got.collision ? "true" : "false", expect.collision ? "true" : "false");
            failures++;
        } else {
            printf("ok: %s\n", collision_message);
        }
    }
}

static void degenerate_inputs(void) {
    DistinctNamesResult r = max_distinct_names(NULL, 0);
    CHECK_EQ(r.distinct, 0, "empty input has zero distinct names");
    CHECK_EQ(r.collision, 0, "empty input has no collision");

    NameSet sets[2] = {{{"a", NULL}, 1}, {{NULL, NULL}, 0}};
    r = max_distinct_names(sets, 2);
    CHECK_EQ(r.distinct, 0, "a card with zero names has zero distinct names");
    CHECK_EQ(r.collision, 1, "a card with zero names has a collision");
}

static void cyclic_overlap_is_solved_exactly(void) {
    NameSet sets[3] = {{{"x", "y"}, 2}, {{"y", "z"}, 2}, {{"z", "x"}, 2}};
    DistinctNamesResult r = max_distinct_names(sets, 3);
    CHECK_EQ(r.distinct, 3, "cyclic overlap has three distinct names");
    CHECK_EQ(r.collision, 0, "cyclic overlap is collision-free");

    NameSet shared[3] = {{{"a", NULL}, 1}, {{"a", NULL}, 1}, {{"a", NULL}, 1}};
    r = max_distinct_names(shared, 3);
    CHECK_EQ(r.distinct, 1, "three cards sharing one name have one distinct name");
    CHECK_EQ(r.collision, 1, "three cards sharing one name collide");

    NameSet greedy_instance[3] = {{{"c", "a"}, 2}, {{"c", "b"}, 2}, {{"a", "b"}, 2}};
    r = max_distinct_names(greedy_instance, 3);
    CHECK_EQ(r.distinct, 3, "adversarial ordering still has three distinct names");
    CHECK_EQ(r.collision, 0, "adversarial ordering is collision-free");
}

int main(void) {
    dp_matches_brute_force();
    degenerate_inputs();
    cyclic_overlap_is_solved_exactly();
    if (failures) return 1;
    printf("ALL MAX DISTINCT NAMES CHECKS PASSED\n");
    return 0;
}
