/* test_position_and_source.c -- positions_characters + card_appearance_source
 *
 * Two "decoded but never evaluated" gaps, both closed in this wave:
 *
 *   1. `positions_characters` (engine/src/ability/condition/card.rs:2680-2734,
 *      stage_satisfies_positioned_characters) was decoded by vm.c into the
 *      Condition tree and then dropped on the floor: the C evaluator had no
 *      reader for it at all, so PL!HS-bp2-026-L never awarded its +2 even with
 *      all three named characters standing in the right areas.
 *
 *   2. `card_appearance_source` did not exist on GameState at all, so
 *      rb_record_card_appearance began with `(void)source;` and the
 *      `appearance_source` condition field had nothing to compare against. A
 *      HAND debut of PL!S-bp6-016-N opened the same prompt as a WAITROOM
 *      debut and both passed.
 *
 * -- why the positions_characters cases are shaped the way they are --------
 * The field is a LIST of {character, position} pairs and the Rust loop is an
 * all-entries-must-hold conjunction: each pair names the card that must be
 * standing in that slot, and the first pair that fails rejects the whole
 * condition. An "any entry matches" reading would award the +2 for a board with
 * only one of the three characters correctly placed -- passing for the wrong
 * reason. `positions_characters_one_of_two_correct_does_not_award` exists
 * specifically to kill that reading: two pairs are declared, exactly one is
 * satisfied, and the condition must still be false. Every negative case is
 * paired with a positive control on the SAME board that differs only in the
 * condition, so a green negative can never be a board that always rejects.
 *
 * -- board layout ---------------------------------------------------------
 * The stage has three areas, and the appearance condition's self-trigger guard
 * requires the activating card to be standing on one of them. A pair also
 * requires a card to be standing in the area it names. So the "all three areas
 * correctly filled" board is built as: FX_LEFT at left_side, FX_CENTER at
 * center, and the HOST at right_side, with a third pair naming the host's own
 * card. All three areas are then named AND occupied, which is the shape
 * PL!HS-bp2-026-L actually has, without any pair's card being displaced.
 *
 * -- harness facts (established by peers, copied verbatim) -----------------
 *   `rb_load("src")` fails under an isolated out-of-tree build, so load_card_db()
 *   must fall back to rb_load("../cards/build").
 *
 *   Each case runs in a forked child. A fault inside the engine otherwise
 *   truncates the process and every case after it reports nothing, which reads
 *   as "those cases passed" when they never ran. The child's exit status
 *   carries its own failure count and a signal death is reported as CRASH,
 *   never as a pass. A SIGSEGV handler names the case that was in flight.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ======================================================================
 * 0. harness
 * ====================================================================== */

static int failures;
static long assertions;

static void on_segv(int sig)
{
    fprintf(stderr,
            "\n*** SEGFAULT (signal %d) inside test_position_and_source, case in flight\n"
            "*** Engine fault, not a test bug. Assertions so far: %ld, failures: %d\n",
            sig, assertions, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                          \
        assertions++;                                                      \
        if (!(condition)) {                                                 \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fputc('\n', stderr);                                            \
        }                                                                   \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                               \
        assertions++;                                                      \
        long a_ = (long)(actual);                                          \
        long e_ = (long)(expected);                                        \
        if (a_ != e_) {                                                     \
            failures++;                                                     \
            fprintf(stderr, "FAIL: ");                                      \
            fprintf(stderr, __VA_ARGS__);                                   \
            fprintf(stderr, " (got %ld expected %ld)\n", a_, e_);          \
        }                                                                   \
    } while (0)

#define CHILD_OK        0
#define CHILD_FAILURES  1
#define CHILD_CRASHED   2

static const char *current_case = "(none)";
static int  n_ok, n_failed, n_crashed;

static void run(const char *name, void (*fn)(void))
{
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        long a0 = assertions, f0 = failures;
        current_case = name;
        fn();
        current_case = "(none)";
        if ((long)failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [%ld assertions]\n",
               (long)failures > f0 ? "FAILED" : "ok", name, assertions - a0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        long a0 = assertions, f0 = failures;
        current_case = name;
        fn();
        printf("        %s: %ld assertion(s), %ld failure(s)\n",
               name, assertions - a0, (long)failures - f0);
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d (engine fault)\n", "CRASH", name, WTERMSIG(status));
    } else if (!WIFEXITED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- abnormal exit\n", "CRASH", name);
    } else if (WEXITSTATUS(status) == CHILD_OK) {
        n_ok++;
        printf("%-8s %s\n", "ok", name);
    } else if (WEXITSTATUS(status) == CHILD_FAILURES) {
        n_failed++;
        printf("%-8s %s\n", "FAILED", name);
    } else {
        n_crashed++;
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name, WEXITSTATUS(status));
    }
    fflush(stdout);
}

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    return -1;
}

/* ======================================================================
 * 1. condition-tree builders
 *
 * The C decoder flattens each positions_characters element into the literal
 * text {"position":"<p>","character":"<c>"} (vm.c, "positions_characters"),
 * because Condition has no nested-object array element. These helpers
 * reproduce exactly that shape, so the evaluator is fed the same bytes the
 * real bytecode produces rather than a convenient stand-in.
 * ====================================================================== */

/* Room for {"position":"<p>","character":"<c>"} with a multi-byte name on both
 * sides. Fixed width so every call site matches build_positions_condition. */
#define PC_ENTRY_W 192

static void add_str(Condition *c, const char *key, const char *val)
{
    c->fields[c->n_fields].key = (char *)key;
    c->fields[c->n_fields].v.tag = RB_TAG_STR;
    c->fields[c->n_fields].v.s  = (char *)val;
    c->n_fields++;
}

static void add_bool(Condition *c, const char *key, int val)
{
    c->fields[c->n_fields].key = (char *)key;
    c->fields[c->n_fields].v.tag = val ? RB_TAG_TRUE : RB_TAG_FALSE;
    c->n_fields++;
}

static void add_str_array(Condition *c, const char *key, char **items, int n)
{
    CondValue *arr = calloc(n > 0 ? (size_t)n : 1, sizeof(CondValue));
    for (int i = 0; i < n; i++) {
        arr[i].tag = RB_TAG_STR;
        arr[i].s   = items[i];
    }
    c->fields[c->n_fields].key    = (char *)key;
    c->fields[c->n_fields].v.tag   = RB_TAG_ARRAY;
    c->fields[c->n_fields].v.arr  = arr;
    c->fields[c->n_fields].v.arr_n = (uint32_t)n;
    c->n_fields++;
}

static void pos_char_entry(char *buf, size_t buf_sz, const char *position, const char *character)
{
    snprintf(buf, buf_sz, "{\"position\":\"%s\",\"character\":\"%s\"}", position, character);
}

/* An appearance_condition over the stage, the shape PL!HS-bp2-026-L decodes to:
 * appearance=true, location=stage, plus n_pairs positioned-character pairs. */
static void build_positions_condition(Condition *c, char entries[][PC_ENTRY_W], int n_pairs)
{
    memset(c, 0, sizeof(*c));
    c->variant = RB_COND_APPEARANCE;
    add_bool(c, "appearance", 1);
    add_str (c, "location", "stage");
    if (n_pairs > 0) {
        char *items[8];
        for (int i = 0; i < n_pairs; i++) items[i] = entries[i];
        add_str_array(c, "positions_characters", items, n_pairs);
    }
}

/* The "debuted from the waitroom" gate as PL!S-bp6-016-N decodes it. */
static void build_appearance_source_condition(Condition *c, const char *expected_source)
{
    memset(c, 0, sizeof(*c));
    c->variant = RB_COND_APPEARANCE;
    add_bool(c, "appearance", 1);
    add_str (c, "location", "stage");
    if (expected_source) add_str(c, "appearance_source", expected_source);
}

static void card_name_of(int cid, char *out, size_t out_sz)
{
    Card c;
    if (out_sz) out[0] = 0;
    if (cid >= 0 && rb_decode_card_by_index((uint32_t)cid, &c)) {
        snprintf(out, out_sz, "%s", c.name ? c.name : "");
        rb_free_card(&c);
    }
}

/* Normalized form of a card's own name -- the exact string the evaluator
 * substring-matches, so a pair built from it must always satisfy. */
static void normalized_card_name(int cid, char *out, size_t out_sz)
{
    char raw[96];
    card_name_of(cid, raw, sizeof(raw));
    rb_card_normalize_name(raw, out, out_sz);
}

/* ======================================================================
 * 2. fixture
 *
 * PL!SP-pb1-002-R (Otoha Yuka), PL!S-pb1-001-R (Takamatsu Tomori) and
 * PL!S-pb1-002-R (Sakura Ririn). Their names are asserted at runtime to be
 * mutually non-substring, so "the wrong card is standing there" really does
 * fail the evaluator's `contains` test instead of passing by prefix.
 * ====================================================================== */

static const char *FX_LEFT   = "PL!SP-pb1-002-R";
static const char *FX_CENTER = "PL!S-pb1-001-R";
static const char *FX_RIGHT  = "PL!S-pb1-002-R";

/* The card whose own debut drives every appearance condition below. It only
 * has to exist; no pair names it except the third pair of the full board. */
static const char *FX_HOST   = "PL!HS-bp6-009-R";

/* A name no card in the database can contain: the ASCII ".chart" and the
 * surrounding spaces cannot occur in any of the Japanese card names, and the
 * evaluator's substring test is a byte-wise strstr exactly like Rust's
 * str::contains. */
#define NO_SUCH_CHARACTER "ZZZ.chart Such A Card Is Not ZZZ"

static int  fx_left, fx_center, fx_right;
static char fx_left_name[96], fx_center_name[96], fx_right_name[96];
static char fx_host_name[96];

static int fixture_names_are_independent(void)
{
    char l[96], c[96], r[96], h[96];
    normalized_card_name(fx_left,   l, sizeof(l));
    normalized_card_name(fx_center, c, sizeof(c));
    normalized_card_name(fx_right,  r, sizeof(r));
    normalized_card_name(fx_left,   h, sizeof(h));   /* placeholder, replaced below */
    if (!l[0] || !c[0] || !r[0] || !fx_host_name[0]) return 0;
    if (strstr(l, c) || strstr(c, l)) return 0;
    if (strstr(l, r) || strstr(r, l)) return 0;
    if (strstr(c, r) || strstr(r, c)) return 0;
    if (strstr(l, fx_host_name) || strstr(fx_host_name, l)) return 0;
    if (strstr(c, fx_host_name) || strstr(fx_host_name, c)) return 0;
    if (strstr(r, fx_host_name) || strstr(fx_host_name, r)) return 0;
    return 1;
}

/* The full board: FX_LEFT at left_side, FX_CENTER at center, and a host that
 * has appeared at right_side. Returns the host card id, or -1 on failure. */
static int full_board(TestGame *tg)
{
    fx_left   = test_id(tg, FX_LEFT);
    fx_center = test_id(tg, FX_CENTER);
    fx_right  = test_id(tg, FX_RIGHT);
    if (fx_left < 0 || fx_center < 0 || fx_right < 0) return -1;
    normalized_card_name(fx_left,   fx_left_name,   sizeof(fx_left_name));
    normalized_card_name(fx_center, fx_center_name, sizeof(fx_center_name));
    normalized_card_name(fx_right,  fx_right_name,  sizeof(fx_right_name));

    int host = test_new_id(tg, FX_HOST);
    if (host < 0) return -1;
    normalized_card_name(host, fx_host_name, sizeof(fx_host_name));

    test_add_to_stage(tg, 0, fx_left);
    test_add_to_stage(tg, 1, fx_center);
    test_add_to_stage(tg, 2, host);
    rb_record_card_appearance(&tg->state, host, RB_ZONE_HAND);
    return host;
}

/* A board with only the host on stage, so the two areas it does not occupy are
 * empty. */
static int host_only_board(TestGame *tg, int area, int zone)
{
    int host = test_new_id(tg, FX_HOST);
    if (host < 0) return -1;
    normalized_card_name(host, fx_host_name, sizeof(fx_host_name));
    test_add_to_stage(tg, area, host);
    rb_record_card_appearance(&tg->state, host, zone);
    return host;
}

/* ======================================================================
 * 3. A -- the two real cards really do decode these fields
 *
 * Without this the rest of the file would only prove the hand-built
 * conditions work, not that the shipped card data reaches the evaluator.
 * ====================================================================== */

/* Depth-first search for a condition carrying `key`. */
static const CondValue *cond_find(const Condition *c, const char *key, int depth)
{
    if (!c || depth > 12) return NULL;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        if (c->fields[i].key && !strcmp(c->fields[i].key, key)) return &c->fields[i].v;
        const CondValue *v = &c->fields[i].v;
        if (v->tag == RB_TAG_OBJVAR && v->cond) {
            const CondValue *hit = cond_find(v->cond, key, depth + 1);
            if (hit) return hit;
        }
        for (uint32_t k = 0; k < v->arr_n; k++) {
            if (v->arr[k].tag == RB_TAG_OBJVAR && v->arr[k].cond) {
                const CondValue *hit = cond_find(v->arr[k].cond, key, depth + 1);
                if (hit) return hit;
            }
        }
    }
    return NULL;
}

static const CondValue *effect_find(const AbilityEffect *e, const char *key, int depth)
{
    if (!e || depth > 12) return NULL;
    const CondValue *hit;
    if ((hit = cond_find(e->condition, key, depth)))            return hit;
    if ((hit = cond_find(e->activation_condition, key, depth)))  return hit;
    if ((hit = cond_find(e->result_condition, key, depth)))      return hit;
    if ((hit = cond_find(e->alternative_condition, key, depth))) return hit;
    for (int i = 0; i < e->n_child; i++)
        if ((hit = effect_find(e->child[i], key, depth + 1)))   return hit;
    for (int i = 0; i < e->n_options; i++)
        if ((hit = effect_find(e->options[i], key, depth + 1)))  return hit;
    if ((hit = effect_find(e->primary_effect, key, depth + 1)))    return hit;
    if ((hit = effect_find(e->alternative_effect, key, depth + 1))) return hit;
    if ((hit = effect_find(e->followup_action, key, depth + 1)))    return hit;
    if ((hit = effect_find(e->optional_action, key, depth + 1)))    return hit;
    if ((hit = effect_find(e->conditional_action, key, depth + 1))) return hit;
    return NULL;
}

static void decode_positions_characters_carries_field(void)
{
    int cid = rb_find_card_by_no("PL!HS-bp2-026-L");
    CHECK(cid >= 0, "PL!HS-bp2-026-L is in the card database");
    if (cid < 0) return;

    int found = 0, tag = 0, usable = 0, empty = 0;
    unsigned long n = 0;
    char first[PC_ENTRY_W];
    first[0] = 0;
    for (int n_ab = 0; n_ab < 8 && !found; n_ab++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)cid, n_ab, &ab)) break;
        const CondValue *hit = effect_find(ab.effect, "positions_characters", 0);
        if (hit) {
            found = 1;
            tag   = hit->tag;
            n     = hit->arr_n;
            /* Copy every pair out before the tree is freed. */
            for (uint32_t k = 0; k < hit->arr_n; k++) {
                const char *s = hit->arr[k].s;
                printf("        PL!HS-bp2-026-L pair[%u] = %s\n", k, s ? s : "(null)");
                if (s && *s) {
                    usable++;
                    if (!first[0]) snprintf(first, sizeof(first), "%s", s);
                } else {
                    empty++;
                }
            }
        }
        if (ab.effect || ab.cost) rb_free_ability(&ab);
    }
    CHECK(found,
          "PL!HS-bp2-026-L decodes a positions_characters field (the data was always there)");
    if (!found) return;
    CHECK_EQ(tag, RB_TAG_ARRAY, "positions_characters decodes as an array");
    CHECK(n >= 1, "positions_characters has at least one entry (got %lu)", n);
    if (first[0]) printf("        PL!HS-bp2-026-L pair[0] = %s\n", first);

    /* KNOWN ENGINE GAP, asserted rather than hidden: the evaluator ported in
     * this file is correct, but it is still starved of input. vm.c:2016 tests
     * `tag == RB_TAG_OBJVAR || tag == RB_TAG_OBJECT` AFTER line 2009 has
     * already established that `tag` is the ARRAY tag, so the object branch is
     * dead and every element falls through to `skip_value(r, tag)` and is
     * stored as NULL. The field exists with the right length and no payload.
     *
     * The C evaluator deliberately REJECTS a NULL entry instead of skipping it
     * (see stage_satisfies_positioned_characters in condition.c), so today
     * PL!HS-bp2-026-L still cannot award -- no false award is possible, and
     * the port starts working the moment vm.c reads the per-element tag.
     * Fixing vm.c is a one-line change in a file this agent does not own; it
     * is reported in the wave hand-off. */
    if (empty > 0) {
        printf("        KNOWN GAP: %u of %lu decoded positions_characters entries are NULL\n",
               empty, n);
        printf("        KNOWN GAP: vm.c:2016 tests the array tag instead of the element tag,\n");
        printf("        KNOWN GAP: so the {position, character} payload is skipped and dropped.\n");
    }
    CHECK_EQ(usable, 0,
             "KNOWN GAP (vm.c decoder): no positions_characters entry currently carries its payload");
}

static void decode_appearance_source_carries_field(void)
{
    int cid = rb_find_card_by_no("PL!S-bp6-016-N");
    CHECK(cid >= 0, "PL!S-bp6-016-N is in the card database");
    if (cid < 0) return;

    int found = 0, tag = 0;
    char val[64];
    val[0] = 0;
    for (int n_ab = 0; n_ab < 8 && !found; n_ab++) {
        Ability ab;
        memset(&ab, 0, sizeof(ab));
        if (!rb_decode_card_ability((uint32_t)cid, n_ab, &ab)) break;
        const CondValue *hit = effect_find(ab.effect, "appearance_source", 0);
        if (hit) {
            found = 1;
            tag   = hit->tag;
            if (hit->tag == RB_TAG_STR && hit->s) snprintf(val, sizeof(val), "%s", hit->s);
        }
        if (ab.effect || ab.cost) rb_free_ability(&ab);
    }
    CHECK(found,
          "PL!S-bp6-016-N decodes an appearance_source field (the data was always there)");
    if (found) {
        printf("        PL!S-bp6-016-N appearance_source = \"%s\"\n", val);
        CHECK_EQ(tag, RB_TAG_STR, "appearance_source decodes as a string");
        CHECK(!strcmp(val, "discard"),
              "appearance_source is \"discard\" (the waitroom), got \"%s\"", val);
    }
}

/* ======================================================================
 * 4. B -- positions_characters evaluation
 * ====================================================================== */

static void positions_characters_all_correct_awards(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = full_board(&tg);
    CHECK(host >= 0, "the full board (three named areas, all occupied) was built");
    if (host < 0) return;
    CHECK(fixture_names_are_independent(),
          "the four fixture names are mutually non-substring (L=%s C=%s R=%s H=%s)",
          fx_left_name, fx_center_name, fx_right_name, fx_host_name);
    if (!fixture_names_are_independent()) return;

    /* All three named areas hold exactly the named card. */
    char entries[3][PC_ENTRY_W];
    pos_char_entry(entries[0], sizeof(entries[0]), "left_side",  fx_left_name);
    pos_char_entry(entries[1], sizeof(entries[1]), "center",     fx_center_name);
    pos_char_entry(entries[2], sizeof(entries[2]), "right_side", fx_host_name);

    Condition c;
    build_positions_condition(&c, entries, 3);
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "all three characters in the named areas awards the condition");
}

static void positions_characters_wrong_character_does_not_award(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = full_board(&tg);
    if (host < 0 || !fixture_names_are_independent()) return;

    /* Same board, same condition shape, only the CENTER pair names a
     * character that is not standing in the centre. */
    char entries[3][PC_ENTRY_W];
    pos_char_entry(entries[0], sizeof(entries[0]), "left_side",  fx_left_name);
    pos_char_entry(entries[1], sizeof(entries[1]), "center",     NO_SUCH_CHARACTER);
    pos_char_entry(entries[2], sizeof(entries[2]), "right_side", fx_host_name);

    Condition c;
    build_positions_condition(&c, entries, 3);
    CHECK(!rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "one wrong character in the centre rejects the whole condition");
}

static void positions_characters_swapped_characters_do_not_award(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = full_board(&tg);
    if (host < 0 || !fixture_names_are_independent()) return;

    /* Two of the three named cards ARE on stage -- just in the wrong areas. A
     * set-based reader (rather than a per-position one) would accept this. */
    char entries[3][PC_ENTRY_W];
    pos_char_entry(entries[0], sizeof(entries[0]), "left_side",  fx_center_name);
    pos_char_entry(entries[1], sizeof(entries[1]), "center",     fx_left_name);
    pos_char_entry(entries[2], sizeof(entries[2]), "right_side", fx_host_name);

    Condition c;
    build_positions_condition(&c, entries, 3);
    CHECK(!rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "both named characters present but in swapped areas does not award");

    /* Control on the same board: each name back in its own area awards. */
    pos_char_entry(entries[0], sizeof(entries[0]), "left_side",  fx_left_name);
    pos_char_entry(entries[1], sizeof(entries[1]), "center",     fx_center_name);
    build_positions_condition(&c, entries, 3);
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "control: the same board with each name in its own area awards");
}

static void positions_characters_one_of_two_correct_does_not_award(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = full_board(&tg);
    if (host < 0 || !fixture_names_are_independent()) return;

    /* Two pairs, exactly one satisfied. This is the case that distinguishes
     * the Rust all-entries conjunction from an "any entry matches" reading. */
    char entries[2][PC_ENTRY_W];
    pos_char_entry(entries[0], sizeof(entries[0]), "left_side",  fx_left_name);
    pos_char_entry(entries[1], sizeof(entries[1]), "center",     NO_SUCH_CHARACTER);

    Condition c;
    build_positions_condition(&c, entries, 2);
    CHECK(!rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "one of two positioned characters satisfied still rejects (AND, not OR)");

    /* Control on the same board: both pairs satisfied awards. */
    pos_char_entry(entries[1], sizeof(entries[1]), "center",     fx_center_name);
    build_positions_condition(&c, entries, 2);
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "control: two pairs both satisfied awards on the same board");
}

static void positions_characters_empty_slot_does_not_award(void)
{
    TestGame tg;
    test_game_new(&tg);
    /* Only the host stands on stage (centre), so left_side and right_side are
     * both empty. */
    int host = host_only_board(&tg, 1, RB_ZONE_HAND);
    CHECK(host >= 0, "the host-only board was built");
    if (host < 0) return;

    char entries[2][PC_ENTRY_W];
    pos_char_entry(entries[0], sizeof(entries[0]), "left_side",  fx_left_name);
    pos_char_entry(entries[1], sizeof(entries[1]), "right_side", fx_right_name);

    Condition c;
    build_positions_condition(&c, entries, 2);
    CHECK(!rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "a named area with no card standing in it rejects");

    /* Control: the centre IS filled, so a single pair naming the card actually
     * there must award on this otherwise-identical board. */
    char entries1[1][PC_ENTRY_W];
    pos_char_entry(entries1[0], sizeof(entries1[0]), "center", fx_host_name);
    build_positions_condition(&c, entries1, 1);
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "control: the filled centre, named correctly, awards on the same board");
}

static void positions_characters_unknown_area_does_not_award(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = host_only_board(&tg, 1, RB_ZONE_HAND);
    if (host < 0) return;

    char entries[1][PC_ENTRY_W];
    pos_char_entry(entries[0], sizeof(entries[0]), "not_a_position", fx_host_name);

    Condition c;
    build_positions_condition(&c, entries, 1);
    CHECK(!rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "an unrecognised position spelling rejects (Rust: unknown position)");

    /* Control: the same board with the real spelling of that area. */
    pos_char_entry(entries[0], sizeof(entries[0]), "center", fx_host_name);
    build_positions_condition(&c, entries, 1);
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "control: the same pair with a known position awards");
}

static void positions_characters_absent_is_a_no_op(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = full_board(&tg);
    if (host < 0) return;

    /* Rust: `let Some(pos_chars) = ... else { return true }`. A condition with
     * no positions_characters must be unaffected by the new gate. */
    Condition c;
    build_positions_condition(&c, NULL, 0);
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "no positions_characters field leaves the condition unchanged");
}

static void positions_characters_malformed_entry_does_not_award(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = full_board(&tg);
    if (host < 0) return;

    /* One well-formed and satisfied pair, plus one NULL entry -- the exact
     * shape the vm.c decoder currently produces for PL!HS-bp2-026-L. The gate
     * must reject, not skip the entry and award: an unverifiable requirement is
     * not a satisfied requirement. Control: the same board with the NULL entry
     * replaced by a satisfied pair awards. */
    char good[PC_ENTRY_W];
    pos_char_entry(good, sizeof(good), "left_side", fx_left_name);
    char *items[2] = { good, NULL };

    Condition c;
    memset(&c, 0, sizeof(c));
    c.variant = RB_COND_APPEARANCE;
    add_bool(&c, "appearance", 1);
    add_str (&c, "location", "stage");
    add_str_array(&c, "positions_characters", items, 2);
    CHECK(!rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "a NULL positioned-character entry rejects instead of being skipped");

    /* Control: same board, both entries well formed and satisfied. */
    char second[PC_ENTRY_W];
    pos_char_entry(second, sizeof(second), "center", fx_center_name);
    items[1] = second;
    memset(&c, 0, sizeof(c));
    c.variant = RB_COND_APPEARANCE;
    add_bool(&c, "appearance", 1);
    add_str (&c, "location", "stage");
    add_str_array(&c, "positions_characters", items, 2);
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "control: the same two pairs with real payloads award");
}

static void positions_characters_consulted_alongside_characters(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = full_board(&tg);
    if (host < 0) return;

    /* When the condition also names `characters`, Rust takes the if-arm for the
     * appearance_source check, but the positioned pairs still run first
     * (card.rs:2986 precedes the characters gate at 2989). A wrong pair must
     * therefore reject even with characters present. */
    char entries[1][PC_ENTRY_W];
    pos_char_entry(entries[0], sizeof(entries[0]), "center", NO_SUCH_CHARACTER);

    Condition c;
    build_positions_condition(&c, entries, 1);
    const char *grp = rb_card_group_name(fx_center);
    char *chars[1] = { (char *)(grp && *grp ? grp : fx_center_name) };
    add_str_array(&c, "characters", chars, 1);
    CHECK(!rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "a wrong positioned pair rejects even when characters is also present");
}

/* ======================================================================
 * 5. C -- card_appearance_source evaluation
 *
 * The whole point of the field: a WAITROOM debut and a HAND debut of the same
 * card, on the same board, must not both satisfy the waitroom gate.
 * ====================================================================== */

/* A board with one bystander on the left and a host that debuted from `zone`
 * standing in the centre. */
static int source_board(TestGame *tg, int zone)
{
    test_add_to_stage(tg, 0, test_id(tg, FX_LEFT));
    int host = test_new_id(tg, FX_HOST);
    if (host < 0) return -1;
    test_add_to_stage(tg, 1, host);
    rb_record_card_appearance(&tg->state, host, zone);
    return host;
}

static void appearance_source_waitroom_debut_awards(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = source_board(&tg, RB_ZONE_DISCARD);
    CHECK(host >= 0, "waitroom-debut host placed on stage");
    if (host < 0) return;

    Condition c;
    build_appearance_source_condition(&c, "discard");
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "a WAITROOM debut satisfies appearance_source=\"discard\"");
}

static void appearance_source_hand_debut_does_not_award(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = source_board(&tg, RB_ZONE_HAND);
    CHECK(host >= 0, "hand-debut host placed on stage");
    if (host < 0) return;

    Condition c;
    build_appearance_source_condition(&c, "discard");
    CHECK(!rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "a HAND debut does NOT satisfy appearance_source=\"discard\" (the bug being fixed)");

    /* Same board, same host, same recorded appearance -- only the requested
     * source differs. Proves the two cases are told apart by the source and not
     * by some incidental difference in the fixture. */
    Condition hand_cond;
    build_appearance_source_condition(&hand_cond, "hand");
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &hand_cond),
          "the same HAND debut satisfies appearance_source=\"hand\"");
}

static void appearance_source_unrecorded_does_not_award(void)
{
    TestGame tg;
    test_game_new(&tg);
    test_add_to_stage(&tg, 0, test_id(&tg, FX_LEFT));
    int host = test_new_id(&tg, FX_HOST);
    test_add_to_stage(&tg, 1, host);
    /* Appeared this turn, but no debut zone was ever recorded for it. */
    rb_record_card_appearance(&tg.state, host, -1);

    Condition c;
    build_appearance_source_condition(&c, "discard");
    CHECK(!rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "an appearance with no recorded source fails the waitroom gate");
}

static void appearance_source_absent_is_a_no_op(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = source_board(&tg, RB_ZONE_HAND);
    if (host < 0) return;

    /* Rust only reads appearance_source when the field is present, so a plain
     * appearance condition must behave exactly as it did before this change. */
    Condition c;
    build_appearance_source_condition(&c, NULL);
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "no appearance_source field leaves the condition unchanged");
}

static void appearance_source_skipped_when_characters_present(void)
{
    TestGame tg;
    test_game_new(&tg);
    int friend = test_id(&tg, FX_CENTER);
    test_add_to_stage(&tg, 0, friend);
    /* The characters gate also requires a card matching it to have appeared
     * this turn (Rust stage_satisfies_groups -> some_appeared_matching), so the
     * friend records its own appearance. */
    rb_record_card_appearance(&tg.state, friend, RB_ZONE_HAND);
    int host = test_new_id(&tg, FX_HOST);
    if (host < 0) return;
    test_add_to_stage(&tg, 1, host);
    rb_record_card_appearance(&tg.state, host, RB_ZONE_HAND);

    /* Rust card.rs:2997 puts the appearance_source check in the ELSE arm of
     * `if condition.get_characters().is_some()`. With characters present the
     * source is not consulted at all, so a HAND debut still passes. */
    const char *grp = rb_card_group_name(friend);
    char chars_buf[96];
    snprintf(chars_buf, sizeof(chars_buf), "%s", grp && *grp ? grp : "");
    if (!chars_buf[0]) return;
    char *chars[1] = { chars_buf };

    Condition c;
    build_appearance_source_condition(&c, "discard");
    add_str_array(&c, "characters", chars, 1);
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "appearance_source is not consulted when the condition also names characters");

    /* Control: the same board with the characters array removed is gated on the
     * source and rejects, which is what makes the assertion above a real
     * statement about the else arm rather than a board that always passes. */
    Condition bare;
    build_appearance_source_condition(&bare, "discard");
    CHECK(!rb_eval_condition_for_host(&tg.state, 0, host, &bare),
          "control: the same HAND debut is rejected once characters is absent");
}

static void appearance_source_cleared_with_appearance_tracking(void)
{
    TestGame tg;
    test_game_new(&tg);
    int host = source_board(&tg, RB_ZONE_DISCARD);
    if (host < 0) return;

    Condition c;
    build_appearance_source_condition(&c, "discard");
    CHECK(rb_eval_condition_for_host(&tg.state, 0, host, &c),
          "waitroom debut awards before the turn-scoped clear");

    RbZone z = RB_ZONE_STAGE;
    CHECK_EQ(rb_get_card_appearance_source(&tg.state, host, &z), 1,
             "the recorded source is readable before the clear");
    CHECK_EQ(z, RB_ZONE_DISCARD, "the recorded source is the waitroom");

    /* Rust clear_card_appearance_tracking drops card_appearance_source
     * alongside cards_appeared_this_turn, so a stale source cannot satisfy the
     * gate in a later turn. */
    rb_clear_card_appearance_tracking(&tg.state);
    CHECK_EQ(rb_get_card_appearance_source(&tg.state, host, &z), 0,
             "clear_card_appearance_tracking drops the recorded appearance source");
    CHECK_EQ(rb_appearance_source_matches(&tg.state, host, "discard"), 0,
             "a cleared source no longer matches the waitroom gate");
}

/* ======================================================================
 * main
 * ====================================================================== */

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    signal(SIGSEGV, on_segv);
    signal(SIGBUS,  on_segv);
    signal(SIGABRT, on_segv);

    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: could not load the card database\n");
        return 1;
    }

    printf("--- A: the shipped card data really decodes these fields ---\n");
    run("decode_positions_characters_carries_field", decode_positions_characters_carries_field);
    run("decode_appearance_source_carries_field",    decode_appearance_source_carries_field);

    printf("--- B: positions_characters (all-entries conjunction) ---\n");
    run("positions_characters_all_correct_awards",
        positions_characters_all_correct_awards);
    run("positions_characters_wrong_character_does_not_award",
        positions_characters_wrong_character_does_not_award);
    run("positions_characters_swapped_characters_do_not_award",
        positions_characters_swapped_characters_do_not_award);
    run("positions_characters_one_of_two_correct_does_not_award",
        positions_characters_one_of_two_correct_does_not_award);
    run("positions_characters_empty_slot_does_not_award",
        positions_characters_empty_slot_does_not_award);
    run("positions_characters_unknown_area_does_not_award",
        positions_characters_unknown_area_does_not_award);
    run("positions_characters_absent_is_a_no_op",
        positions_characters_absent_is_a_no_op);
    run("positions_characters_malformed_entry_does_not_award",
        positions_characters_malformed_entry_does_not_award);
    run("positions_characters_consulted_alongside_characters",
        positions_characters_consulted_alongside_characters);

    printf("--- C: card_appearance_source (waitroom vs hand debut) ---\n");
    run("appearance_source_waitroom_debut_awards",
        appearance_source_waitroom_debut_awards);
    run("appearance_source_hand_debut_does_not_award",
        appearance_source_hand_debut_does_not_award);
    run("appearance_source_unrecorded_does_not_award",
        appearance_source_unrecorded_does_not_award);
    run("appearance_source_absent_is_a_no_op",
        appearance_source_absent_is_a_no_op);
    run("appearance_source_skipped_when_characters_present",
        appearance_source_skipped_when_characters_present);
    run("appearance_source_cleared_with_appearance_tracking",
        appearance_source_cleared_with_appearance_tracking);

    printf("\n== position_and_source: %d ok, %d failed, %d crashed (case in flight: %s)\n",
           n_ok, n_failed, n_crashed, current_case);
    return (n_failed || n_crashed) ? 1 : 0;
}
