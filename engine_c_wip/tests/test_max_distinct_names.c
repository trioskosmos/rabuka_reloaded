#include "rabuka.h"
#include "test_game.h"

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

/* ── Card-number lookup parity (TEST_PORT_BACKLOG item 3) ───────────────────
   rb_find_card_by_no is the C port of CardDatabase::get_card_id
   (engine/src/core/card.rs:576). It used to stop after the exact and the
   normalized match, so every card number spelled with an ASCII "+" for a print
   the database stores with a fullwidth "＋" resolved to -1, and the
   normalizer it used aliased one cursor for both the UTF-8 read and the output
   write — folding a 3-byte fullwidth character to 1 byte left two
   UNINITIALIZED bytes in the buffer, so a lookup either missed or silently
   matched a different print depending on stack contents. These cases pin both.
   ── */

static void card_lookup_queries(TestGame *tg, const char *no) {
    (void)tg;
    int id = rb_find_card_by_no(no);
    if (id < 0) {
        fprintf(stderr, "FAIL: card_no '%s' does not resolve\n", no);
        failures++;
        return;
    }
    Card card;
    memset(&card, 0, sizeof card);
    if (!rb_decode_card_by_index((uint32_t)id, &card)) {
        fprintf(stderr, "FAIL: card_no '%s' (id %d) does not decode\n", no, id);
        failures++;
        return;
    }
    const char *stored = rb_card_string(card.card_no_idx);
    printf("ok: '%s' -> id %d (%s)\n", no, id, stored ? stored : "?");
    rb_free_card(&card);
}

static void card_lookup_resolves(const char *no, int expected_id) {
    int id = rb_find_card_by_no(no);
    if (id != expected_id) {
        fprintf(stderr, "FAIL: '%s' resolved to %d expected %d\n", no, id, expected_id);
        failures++;
    } else {
        printf("ok: '%s' -> %d\n", no, id);
    }
}

static void card_lookup_returns_minus_one(void) {
    /* A base with no print at all must still miss. Note "PL!N-bp1-999" is NOT
       a valid negative case: the database has PL!N-bp1-999-SEC＋, so the
       base-prefix fallback resolves it (and so does the Rust engine). */
    static const char *const absent[] = { "PL!ZZ-bp1-999-R+", "LL-ZZ-999-R" };
    for (size_t i = 0; i < sizeof(absent) / sizeof(absent[0]); i++) {
        int id = rb_find_card_by_no(absent[i]);
        if (id != -1) {
            fprintf(stderr, "FAIL: unknown card_no '%s' resolved to %d expected -1\n",
                    absent[i], id);
            failures++;
        } else {
            printf("ok: unknown card number '%s' is not found\n", absent[i]);
        }
    }
}

/* Every one of these is a print the database stores with a fullwidth "＋"; the
   ASCII "+" spelling must reach the same card the Rust engine reaches. */
static void card_lookup_ascii_plus_resolves(void) {
    TestGame tg;
    test_game_new(&tg);
    static const char *const queries[] = {
        "LL-bp1-001-R+", "LL-bp2-001-R+", "PL!-bp3-008-R+", "PL!-bp5-003-R+",
        "PL!-bp6-003-R+", "PL!-bp6-006-R+", "PL!-bp6-007-R+", "PL!HS-bp2-007-R+",
        "PL!N-bp4-007-R+", "PL!N-bp5-005-R+", "PL!N-bp5-012-R+", "PL!N-pb1-009-P+",
        "PL!N-pb1-022-P+", "PL!S-bp5-001-R+", "PL!S-bp5-002-R+", "PL!S-bp6-004-R+",
        "PL!S-bp7-007-R+", "PL!SP-bp5-001-R+", "PL!SP-bp5-002-R+", "PL!SP-bp5-004-R+"
    };
    for (size_t i = 0; i < sizeof(queries) / sizeof(queries[0]); i++)
        card_lookup_queries(&tg, queries[i]);
}

/* The fullwidth and ASCII spellings of one card number must resolve to the
   SAME print. Before the fix the fullwidth form lost the "＋" to the
   normalizer's uninitialized hole and returned a neighbouring print. */
static void card_lookup_plus_spellings_agree(void) {
    TestGame tg;
    test_game_new(&tg);
    int wide = rb_find_card_by_no("PL!SP-pb1-002-P\uff0b");
    int narrow = rb_find_card_by_no("PL!SP-pb1-002-P+");
    CHECK_EQ(wide, narrow, "fullwidth and ASCII plus spell the same card");
    CHECK_EQ(wide, 2259, "PL!SP-pb1-002-P+ resolves the P+ print");
    if (wide >= 0) {
        Card card;
        memset(&card, 0, sizeof card);
        if (rb_decode_card_by_index((uint32_t)wide, &card)) {
            const char *stored = rb_card_string(card.card_no_idx);
            CHECK(stored && !strcmp(stored, "PL!SP-pb1-002-P\uff0b"),
                  "resolved card is the P+ print, not a neighbouring rarity");
            rb_free_card(&card);
        }
    }
}

/* Steps 3 and 4 of get_card_id: rarity fallbacks and base-prefix lookup.
   Each expected id is the print the Rust engine resolves these to. */
static void card_lookup_fallbacks(void) {
    TestGame tg;
    test_game_new(&tg);
    card_lookup_resolves("PL!HS-pb1-003", 705);
    card_lookup_resolves("PL!S-bp6-024-R+", 1691);
    card_lookup_resolves("pl!n-bp1-027-X", 913);
    card_lookup_resolves("PL!N-bp1-027-L", 913);
    card_lookup_resolves("pl!n-bp1-027-l", 913);
}

/* Cards sharing character+number but differing only by print must NOT be
   conflated: bp2-011-R (debut only) and pb2-011-R (auto + live start) are
   different cards (engine_c_wip AGENTS.md card-identity rule). */
static void card_lookup_keeps_prints_distinct(void) {
    TestGame tg;
    test_game_new(&tg);
    int bp2 = rb_find_card_by_no("PL!SP-bp2-011-R");
    int pb2 = rb_find_card_by_no("PL!SP-pb2-011-R");
    CHECK(bp2 >= 0 && pb2 >= 0, "both bp2-011-R and pb2-011-R resolve");
    CHECK(bp2 != pb2, "bp2-011-R and pb2-011-R are different cards");
    if (bp2 >= 0 && pb2 >= 0) {
        Card a, b;
        memset(&a, 0, sizeof a);
        memset(&b, 0, sizeof b);
        if (rb_decode_card_by_index((uint32_t)bp2, &a) &&
            rb_decode_card_by_index((uint32_t)pb2, &b)) {
            printf("ok: bp2-011-R id=%d cost=%d, pb2-011-R id=%d cost=%d\n",
                   bp2, (int)a.cost, pb2, (int)b.cost);
            CHECK(a.cost != b.cost, "the two prints differ");
            rb_free_card(&a);
            rb_free_card(&b);
        }
    }
}

/* normalize_card_no must emit no uninitialized bytes: a fullwidth character
   folds to exactly one output byte, so the result is always exactly as long as
   the folded input. Poison the buffer and check every byte. */
static void card_normalize_no_writes_no_holes(void) {
    static const char *const cases[][2] = {
        { "PL!N-bp5-010-R\uff0b", "PL!N-BP5-010-R+" },
        { "LL-bp1-001-R\uff0b",  "LL-BP1-001-R+"  },
        { "PL!S-pb1-003-P+",     "PL!S-PB1-003-P+" },
        { "\uff41\uff42\uff01\uff0d\uff0a\uff03\uff0b", "AB!-*#+" }
    };
    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        char buf[64];
        memset(buf, 0xAA, sizeof buf);
        rb_card_normalize_no(cases[i][0], buf, sizeof buf);
        int hole = 0;
        size_t expect = strlen(cases[i][1]);
        for (size_t k = 0; k <= expect; k++)
            if ((unsigned char)buf[k] == 0xAA) hole = 1;
        if (hole) {
            fprintf(stderr, "FAIL: normalize left an unwritten byte for '%s'\n", cases[i][0]);
            failures++;
        } else if (strcmp(buf, cases[i][1]) != 0) {
            fprintf(stderr, "FAIL: normalize('%s') = '%s' expected '%s'\n",
                    cases[i][0], buf, cases[i][1]);
            failures++;
        } else {
            printf("ok: normalize('%s') = '%s'\n", cases[i][0], buf);
        }
    }
}

/* The card blobs are resolved relative to the build tree. tools/isolated_build.sh
   stages them at ../cards/build rather than copying them into src/, so try the
   in-tree location first and fall back to the staged one. */
static int load_card_database(void) {
    static const char *const dirs[] = { "src", "../cards/build", "cards/build" };
    for (size_t i = 0; i < sizeof(dirs) / sizeof(dirs[0]); i++) {
        if (rb_load(dirs[i]) == 0) {
            printf("ok: card database loaded from %s (%u cards)\n", dirs[i], rb_num_cards());
            return 1;
        }
    }
    return 0;
}

int main(void) {
    if (!load_card_database()) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    card_lookup_fallbacks();
    card_lookup_ascii_plus_resolves();
    card_lookup_plus_spellings_agree();
    card_lookup_keeps_prints_distinct();
    card_lookup_returns_minus_one();
    card_normalize_no_writes_no_holes();
    dp_matches_brute_force();
    degenerate_inputs();
    cyclic_overlap_is_solved_exactly();
    rb_unload();
    if (failures) return 1;
    printf("ALL MAX DISTINCT NAMES CHECKS PASSED\n");
    return 0;
}
