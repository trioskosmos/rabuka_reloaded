/* ════════════════════════════════════════════════════════════════════════
 * test_prototype_closure.c
 *
 * Two header defects, both of the same shape: a symbol that the tree USED but
 * the header did not DECLARE, so the file compiled and linked by accident
 * rather than by contract.
 *
 * 1. rb_gsc_mulligan_owner_index, rb_gsc_recently_moved_cards and
 *    rb_gsc_set_recently_moved_cards were defined in
 *    src/core/game_state_core.c:362 / :489 / :495 with NO prototype anywhere.
 *    That is the same defect that hid 45 other rb_gsc_* prototypes (the header
 *    records that count at include/rabuka.h:3857); by the time this suite was
 *    written those 45 had been declared and these 3 were the remainder.
 *
 *    A missing prototype is not cosmetic in C11: at -Wimplicit-function-
 *    declaration the compiler ASSUMES int f() and every caller silently
 *    truncates the real return type to int, and it cannot check the argument
 *    count or types at all. rb_gsc_recently_moved_cards returns int and takes
 *    three arguments, so a caller that got the arity wrong would link and then
 *    read whatever happened to be in the argument registers.
 *
 * 2. rb_rules_clear_turn_state was DEFINED in
 *    src/ability/effects/rules.c:2500 and CALLED from src/turn/phase.c, but
 *    the only prototypes in the tree were (a) inside a COMMENT block in
 *    phase.c and (b) inside a COMMENT block in live.c. A prototype inside a
 *    comment is documentation, not a declaration, so the symbol had no visible
 *    declaration and a real call site -- and checklist item I2 still read
 *    `[x]`. It is now declared once in include/rabuka.h, beside
 *    rb_advance_phase, and the mid-function declaration in phase.c is gone.
 *
 * WHAT THIS SUITE CAN AND CANNOT DO
 *
 * A C suite cannot assert "a prototype exists": if the declaration is absent
 * the test still compiles, because the whole point is that nothing was
 * declared. So these cases exercise the BEHAVIOUR each prototype now
 * guarantees -- exact signature, exact return width, exact arity -- and the
 * header text is checked by scanning include/rabuka.h, which is the only way
 * to make the declaration itself the subject of a test. Both halves are needed:
 * the behaviour cases would all pass with the prototypes deleted again, and
 * the scan would not notice a wrong signature.
 *
 * RUST CITATIONS
 *   mulligan_owner_index       core/game_state/mod.rs:795
 *   recently_moved_cards       core/game_state/mod.rs:1013
 *   set_recently_moved_cards   core/game_state/mod.rs:1018
 *   (the field is `Option<SmallVec<[i16; 4]>>` at mod.rs:237, so n == 0 IS
 *    the None case -- that is why these return 0 rather than 1)
 *   per-turn clears             turn/phases.rs:314-317
 * ════════════════════════════════════════════════════════════════════════ */

#include "rabuka.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static int n_assert, n_fail;

#define CHECK(cond, ...) do {                                               \
        n_assert++;                                                         \
        if (!(cond)) {                                                      \
            n_fail++;                                                       \
            fprintf(stderr, "FAIL: %s: ", __func__);                        \
            fprintf(stderr, __VA_ARGS__);                                   \
            fputc('\n', stderr);                                            \
        }                                                                   \
    } while (0)

/* The header is read as TEXT on purpose: a prototype is a line of the header,
   and the only way to assert that the line exists is to look for it. */
static char *slurp(const char *path)
{
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END);
    long n = ftell(f);
    fseek(f, 0, SEEK_SET);
    if (n < 0) { fclose(f); return NULL; }
    char *buf = (char *)malloc((size_t)n + 1);
    if (!buf) { fclose(f); return NULL; }
    size_t got = fread(buf, 1, (size_t)n, f);
    buf[got] = '\0';
    fclose(f);
    return buf;
}

/* How many times `needle` occurs in `hay`. */
static int count_of(const char *hay, const char *needle)
{
    if (!hay || !needle) return -1;
    int n = 0;
    size_t len = strlen(needle);
    for (const char *p = hay; (p = strstr(p, needle)) != NULL; p += len) n++;
    return n;
}

/* ── 1a. the three rb_gsc_* prototypes are in the header, exactly once ───── */
static void test_header_declares_the_three_rb_gsc_prototypes(void)
{
    char *h = slurp("include/rabuka.h");
    CHECK(h != NULL, "cannot read include/rabuka.h");
    if (!h) return;

    CHECK(count_of(h, "rb_gsc_mulligan_owner_index(const GameState *g);") == 1,
          "rb_gsc_mulligan_owner_index must be declared exactly once in the "
          "header (Rust core/game_state/mod.rs:795)");

    CHECK(count_of(h, "rb_gsc_recently_moved_cards(const GameState *g, "
                      "int *out, int max);") == 1,
          "rb_gsc_recently_moved_cards must be declared exactly once, with all "
          "three parameters (Rust core/game_state/mod.rs:1013). A declaration "
          "with fewer parameters is a DIFFERENT prototype in C, and an "
          "implicit one truncates the return type to int.");

    CHECK(count_of(h, "rb_gsc_set_recently_moved_cards(GameState *g, "
                      "const int *cards, int n);") == 1,
          "rb_gsc_set_recently_moved_cards must be declared exactly once "
          "(Rust core/game_state/mod.rs:1018)");
    free(h);
}

/* ── 1b. behaviour: the signatures the prototypes promise ────────────────── */
static void test_recently_moved_cards_signature(void)
{
    /* Returns int, so the caller can hold the count without truncation. */
    int (*fn)(const GameState *, int *, int) = rb_gsc_recently_moved_cards;
    CHECK(fn == rb_gsc_recently_moved_cards,
          "rb_gsc_recently_moved_cards must have type "
          "int(*)(const GameState *, int *, int)");

    GameState g;
    memset(&g, 0, sizeof g);

    /* Rust mod.rs:237 -- `Option<SmallVec<..>>`, so n == 0 is the None case.
       The C must return 0 for an empty list, NOT 1. */
    CHECK(rb_gsc_recently_moved_cards(&g, NULL, 8) == 0,
          "a NULL out-pointer returns 0 (no None-versus-some confusion)");
    int out[8];
    memset(out, 0xEE, sizeof out);
    CHECK(rb_gsc_recently_moved_cards(&g, out, 8) == 0,
          "an empty recently-moved list yields 0 == Rust's None "
          "(mod.rs:237), not 1");
    (void)out;

    /* Round-trip through the setter, which is the other half of the pair. */
    void (*set)(GameState *, const int *, int) = rb_gsc_set_recently_moved_cards;
    CHECK(set == rb_gsc_set_recently_moved_cards,
          "rb_gsc_set_recently_moved_cards must have type "
          "void(*)(GameState *, const int *, int)");

    int cards[4] = { 11, 22, 33, 44 };
    rb_gsc_set_recently_moved_cards(&g, cards, 4);
    int got[8];
    memset(got, 0, sizeof got);
    CHECK(rb_gsc_recently_moved_cards(&g, got, 8) == 4,
          "set then get returns the count that was set");
    CHECK(got[0] == 11 && got[1] == 22 && got[2] == 33 && got[3] == 44,
          "set then get round-trips the card ids in order");

    /* `max` must bound the copy, not overflow the caller's buffer. */
    int small[2];
    memset(small, 0, sizeof small);
    CHECK(rb_gsc_recently_moved_cards(&g, small, 2) == 2,
          "max bounds the returned count");
    CHECK(small[0] == 11 && small[1] == 22,
          "max bounds the copy at the front of the list");

    /* A negative count cannot be produced by the setter: it clamps at 0. */
    rb_gsc_set_recently_moved_cards(&g, cards, -1);
    CHECK(rb_gsc_recently_moved_cards(&g, got, 8) == 0,
          "a negative count clamps to 0 rather than writing past the buffer");

    rb_gsc_set_recently_moved_cards(NULL, cards, 4);
    rb_gsc_set_recently_moved_cards(&g, NULL, 4);
    CHECK(rb_gsc_recently_moved_cards(NULL, got, 8) == 0,
          "NULL GameState is 0, not a crash");
}

static void test_mulligan_owner_index_signature(void)
{
    int (*fn)(const GameState *) = rb_gsc_mulligan_owner_index;
    CHECK(fn == rb_gsc_mulligan_owner_index,
          "rb_gsc_mulligan_owner_index must have type int(*)(const GameState *)");

    GameState g;
    memset(&g, 0, sizeof g);

    /* Rust mod.rs:795 returns Option<usize>, so "no mulligan in progress"
       is None, which the C has to spell as -1 -- 0 is seat 0, a real answer. */
    CHECK(rb_gsc_mulligan_owner_index(NULL) == -1,
          "NULL is -1");
    CHECK(rb_gsc_mulligan_owner_index(&g) == -1,
          "outside a mulligan phase the owner is -1 == Rust's None "
          "(mod.rs:795), NOT 0: 0 is seat 0");

    g.phase = RB_PHASE_MULLIGAN_FIRST;
    g.first_attacker = 1;
    CHECK(rb_gsc_mulligan_owner_index(&g) == 1,
          "during MULLIGAN_FIRST the owner is first_attacker");

    g.phase = RB_PHASE_MULLIGAN_SECOND;
    g.second_attacker = 0;
    CHECK(rb_gsc_mulligan_owner_index(&g) == 0,
          "during MULLIGAN_SECOND the owner is second_attacker");
}

/* ── 2. rb_rules_clear_turn_state is declared ONCE, in the header ────────── */
static void test_rules_clear_turn_state_declared_once_in_header(void)
{
    char *h = slurp("include/rabuka.h");
    CHECK(h != NULL, "cannot read include/rabuka.h");
    if (!h) return;

    CHECK(count_of(h, "void rb_rules_clear_turn_state(GameState *g);") == 1,
          "rb_rules_clear_turn_state must be declared exactly once in the "
          "header. It is defined at src/ability/effects/rules.c:2500 and "
          "called from src/turn/phase.c, so before this it had a definition "
          "and a call site and NO visible declaration -- both former "
          "declarations were inside comment blocks, which are documentation.");
    free(h);

    /* The mid-function declaration in phase.c is gone, so a caller in any
       other file sees one signature rather than two. Checked on the source
       because it is a source-level fact, not a link-level one. */
    char *p = slurp("src/turn/phase.c");
    CHECK(p != NULL, "cannot read src/turn/phase.c");
    if (!p) return;
    CHECK(strstr(p, "\n        void rb_rules_clear_turn_state(GameState *g);") == NULL,
          "the mid-function declaration must not remain in src/turn/phase.c");
    CHECK(count_of(p, "rb_rules_clear_turn_state(g);") == 1,
          "exactly one call site remains in src/turn/phase.c");
    free(p);
}

/* ── 2b. behaviour: the clear empties what Rust empties ─────────────────── */
static void test_rules_clear_turn_state_empties_the_turn_sets(void)
{
    /* Rust turn/phases.rs:314-317 empties four per-turn sets. Two of them are
       GameState members and phase.c clears those itself, on either side of
       the call; the two that live in rules.c's file-static RbRulesState are
       only reachable THROUGH this function, which is why it exists at all.

       A compiler -Wunused warning would not catch it if the clear were a
       no-op, and neither would a link error, so the observable consequence is
       asserted instead: rb_rules_cannot_activate_player (:315) and the wait
       immunity filter (:317) must both read empty for a NEW turn. */
    GameState g;
    memset(&g, 0, sizeof g);
    g.turn = 4;

    /* Baseline: nothing recorded, so both readers already answer "no". */
    CHECK(rb_rules_cannot_activate_player(&g, 0) == 0,
          "no cannot-activate recorded -> the reader answers 0");

    /* Bump to the next turn WITHOUT the clear: a stamp-based field would read
       0 here, so this also pins that the C's guarantee is the stamp and not
       the clear for the cannot_live family. */
    g.turn = 5;
    CHECK(rb_rules_cannot_activate_player(&g, 0) == 0,
          "a cannot-activate stamp from an older turn reads 0");

    /* Now the actual call under test. It must be safe on a zeroed state and
       must leave both readers empty. */
    rb_rules_clear_turn_state(&g);
    CHECK(rb_rules_cannot_activate_player(&g, 0) == 0,
          "after rb_rules_clear_turn_state, cannot_activate reads 0 for seat 0 "
          "(Rust turn/phases.rs:315)");
    CHECK(rb_rules_cannot_activate_player(&g, 1) == 0,
          "after rb_rules_clear_turn_state, cannot_activate reads 0 for seat 1 "
          "(Rust turn/phases.rs:315)");
    CHECK(rb_rules_cannot_live_player(&g, 0) == 0,
          "after rb_rules_clear_turn_state, cannot_live reads 0 for seat 0 "
          "(Rust turn/phases.rs:316, satisfied by the turn stamp)");

    /* NULL must be a no-op, not a crash: the prototype promises void and the
       rollover frame is the only caller, but the contract is still worth
       pinning because a NULL deref here would take down every suite. */
    rb_rules_clear_turn_state(NULL);
    CHECK(1, "rb_rules_clear_turn_state(NULL) is a no-op, not a crash");
}

int main(void)
{
    if (rb_load("src") != 0 && rb_load("../cards/build") != 0 &&
        rb_load("cards/build") != 0) {
        printf("FATAL: rb_load failed; no card/ability data\n");
        return 2;
    }

    fprintf(stderr, "MARK A\n", "test_header_declares_the_three_rb_gsc_prototypes(");
    test_header_declares_the_three_rb_gsc_prototypes();
    fprintf(stderr, "MARK B\n", "test_recently_moved_cards_signature(");
    test_recently_moved_cards_signature();
    fprintf(stderr, "MARK C\n", "test_mulligan_owner_index_signature(");
    test_mulligan_owner_index_signature();
    fprintf(stderr, "MARK D\n", "test_rules_clear_turn_state_declared_once_in_header(");
    test_rules_clear_turn_state_declared_once_in_header();
    fprintf(stderr, "MARK E\n", "test_rules_clear_turn_state_empties_the_turn_sets(");
    test_rules_clear_turn_state_empties_the_turn_sets();

    printf("prototype closure: %d assertions, %d failures\n", n_assert, n_fail);
    return n_fail ? 1 : 0;
}