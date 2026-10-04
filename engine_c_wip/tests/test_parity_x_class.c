/*
 * test_parity_x_class.c — cluster X: "the struct field is read out of the extras
 *   table".  See engine_c_wip/NOZOMI_ROOT_CAUSE.md §4/§6 and
 *   PARITY_CHECKLIST.md cluster X.
 *
 * THE DEFECT
 * ----------
 * rb_decode_card_ability -> rb_vm_decode_extras (vm.c) stores `text`, `action`,
 * `source`, `destination`, `target`, `count`, `condition`, `card_type`,
 * `per_unit_count`, `cost_reduction_per_group`, `result_condition`,
 * `alternative_condition`, `look_action`, `select_action` and `gained_effect`
 * into DEDICATED AbilityEffect fields and then `continue`s, skipping the scalar
 * extras tail at vm.c:1169. So NONE of them is ever put in the extras table.
 * MEASURED through the engine's own decoder over the shipped bytecode
 * (tools/audit_x_class.c, 985 abilities / 4024 effect nodes):
 *
 *     key                     nodes_FIELD  abilities  nodes_EXTRA
 *     card_type                     1221       1074            0
 *     per_unit_count                 155        136            0
 *     action                        4022       2103            0
 *     source   /  zone              1621       1283            0
 *     destination                   1378       1124            0
 *     target                        1499       1309            0
 *     count                         2636       1810            0
 *     condition                      952        880            0
 *     result_condition                53         53            0
 *     alternative_condition           13         13            0
 *     look_action                    183        183            0
 *     select_action                  185        185            0
 *     gained_effect                   52         50            0
 *
 * Rust reads the TYPED field (`effect.card_type_any()`,
 * `mod_cost.per_unit_count_any()`). A bare `*_extra(e, "card_type")` therefore
 * returns NULL permanently, and the reader silently takes its default branch.
 *
 * WHY THIS TEST IS SYNTHETIC
 * --------------------------
 * MEASURED reachability of the four readers this file pins, again through the
 * engine's own decoder:
 *
 *   look.c   rb_effect_reveal_until_chosen_card  action reveal_until_chosen_card: 0 nodes
 *   look.c   rb_effect_reveal_until_target       action reveal_until_target:      0 nodes
 *   score.c  rb_execute_modify_required_hearts_success
 *                                                   action modify_required_hearts_success: 0 nodes
 *   resolver.c apply_per_unit_cost_reduction     modify_cost + per_unit + subtract: 7 nodes,
 *                                                   and ALL 7 carry per_unit_count == 1,
 *                                                   which is what the broken reader
 *                                                   already returned.
 *
 * So no corpus-driven assertion can separate fixed from broken: the four fixes
 * are byte-identical over 41 suites. These tests therefore drive each reader
 * DIRECTLY with an effect that carries the key in the STRUCT FIELD and nothing
 * in the extras table -- the exact shape the decoder produces. Each one FAILS on
 * the pre-fix reader.
 *
 * It deliberately does NOT re-implement the bytecode reader: the values are
 * hand-built, which is the whole point. Reproducing the reader was a recorded
 * hazard (PARITY_CHECKLIST.md cluster T).
 *
 * HARNESS NOTES
 * -------------
 *  * sizeof(GameState) is ~781 KB, so every TestGame local is `static`.
 *  * test_new_id() (Rust's `game.id()` pool) for every card tracked by identity;
 *    test_id() aliases the shared template.
 *  * No Japanese/Fullwidth literal appears in this file.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <sys/wait.h>

/* ===================================================================== */
/* 0. harness                                                            */
/* ===================================================================== */

static long checks;
static int  failures;
static const char *current_case = "(none)";

static void on_fatal_signal(int sig)
{
    fprintf(stderr,
            "\n*** FATAL signal %d in test_parity_x_class\n"
            "*** case in flight: %s\n"
            "*** checks so far: %ld  failures: %d\n",
            sig, current_case, checks, failures);
    fflush(stderr);
    _Exit(128 + sig);
}

#define CHECK(condition, ...) do {                                  \
        checks++;                                                   \
        if (!(condition)) {                                         \
            failures++;                                             \
            fprintf(stderr, "FAIL: ");                              \
            fprintf(stderr, __VA_ARGS__);                           \
            fputc('\n', stderr);                                    \
        }                                                           \
    } while (0)

#define CHECK_EQ(actual, expected, ...) do {                        \
        checks++;                                                   \
        long a_ = (long)(actual);                                   \
        long e_ = (long)(expected);                                 \
        if (a_ != e_) {                                             \
            failures++;                                             \
            fprintf(stderr, "FAIL: ");                              \
            fprintf(stderr, __VA_ARGS__);                           \
            fprintf(stderr, " (got %ld expected %ld)\n", a_, e_);   \
        }                                                           \
    } while (0)

#define CHILD_OK       0
#define CHILD_FAILURES 1

static int n_ok, n_failed, n_crashed;
static const char *only_case;

static void run(const char *name, void (*fn)(void))
{
    if (only_case && !strstr(name, only_case)) return;
    fflush(stdout);
    fflush(stderr);

    pid_t pid = fork();
    if (pid < 0) {
        long c0 = checks, f0 = failures;
        current_case = name;
        fn();
        current_case = "(none)";
        if (failures > f0) n_failed++; else n_ok++;
        printf("%-8s %s  [%ld checks]\n", failures > f0 ? "FAILED" : "ok",
               name, checks - c0);
        fflush(stdout);
        return;
    }
    if (pid == 0) {
        long c0 = checks, f0 = failures;
        current_case = name;
        fn();
        printf("        %s: %ld check(s), %ld failure(s)\n",
               name, checks - c0, failures - f0);
        fflush(stdout);
        fflush(stderr);
        _Exit(failures > f0 ? CHILD_FAILURES : CHILD_OK);
    }

    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status)) {
        n_crashed++;
        printf("%-8s %s  <-- CRASHED on signal %d\n", "CRASH", name, WTERMSIG(status));
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
        printf("%-8s %s  <-- unexpected child exit %d\n", "CRASH", name,
               WEXITSTATUS(status));
    }
    fflush(stdout);
}

/* ===================================================================== */
/* 1. helpers                                                            */
/* ===================================================================== */

#define MEMBER_MUS  "PL!-sd1-010-SD"  /* a member, NOT a live card */
#define MEMBER_MUS2 "PL!S-sd1-001-SD"  /* a different member      */
#define LIVE_CARD   "PL!-sd1-001-SD"  /* a live card             */
#define LIVE_CARD2  "PL!S-sd1-001-SD"  /* a different live card   */
#define ENERGY_CARD "IKZL-PR-001-PR"  /* an energy card: NOT a member and NOT live */

/* Set one extras pair, exactly as rb_vm_decode_extras would leave it for a key
 * that is NOT in the class (i.e. one the scalar tail really does write). */
static void fx_set(AbilityEffect *e, int i, const char *k, const char *v)
{
    if (i >= RB_MAX_EXTRA) return;
    e->extra_k[i] = (char *)k;
    e->extra_v[i] = (char *)v;
    if (i + 1 > e->n_extra) e->n_extra = i + 1;
}

/* Recursive census helpers for the two corpus tests below. */
static void x_walk_card_type(const AbilityEffect *e, long *f, long *x,
                             long *reveal_chosen, long *reveal_target, long *mrhs)
{
    if (!e) return;
    if (e->card_type_field[0]) (*f)++;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "card_type")) (*x)++;
    if (e->action) {
        if (!strcmp(e->action, "reveal_until_chosen_card"))      (*reveal_chosen)++;
        if (!strcmp(e->action, "reveal_until_target"))           (*reveal_target)++;
        if (!strcmp(e->action, "modify_required_hearts_success")) (*mrhs)++;
    }
    for (int i = 0; i < e->n_child; i++)   x_walk_card_type(e->child[i], f, x, reveal_chosen, reveal_target, mrhs);
    for (int i = 0; i < e->n_options; i++) x_walk_card_type(e->options[i], f, x, reveal_chosen, reveal_target, mrhs);
    x_walk_card_type(e->primary_effect, f, x, reveal_chosen, reveal_target, mrhs);
    x_walk_card_type(e->alternative_effect, f, x, reveal_chosen, reveal_target, mrhs);
    x_walk_card_type(e->look_action, f, x, reveal_chosen, reveal_target, mrhs);
    x_walk_card_type(e->select_action, f, x, reveal_chosen, reveal_target, mrhs);
    x_walk_card_type(e->followup_action, f, x, reveal_chosen, reveal_target, mrhs);
    x_walk_card_type(e->optional_action, f, x, reveal_chosen, reveal_target, mrhs);
    x_walk_card_type(e->conditional_action, f, x, reveal_chosen, reveal_target, mrhs);
    x_walk_card_type(e->gained_effect, f, x, reveal_chosen, reveal_target, mrhs);
    x_walk_card_type(e->resource_on_select, f, x, reveal_chosen, reveal_target, mrhs);
    x_walk_card_type(e->opponent_action, f, x, reveal_chosen, reveal_target, mrhs);
}

static void x_walk_per_unit(const AbilityEffect *e, long *nodes_field, long *nodes_extra,
                            long *abilities)
{
    if (!e) return;
    if (e->action && !strcmp(e->action, "modify_cost") && e->per_unit) {
        (*abilities)++;
        if (e->per_unit_count > 0) (*nodes_field)++;
        for (int i = 0; i < e->n_extra; i++)
            if (e->extra_k[i] && !strcmp(e->extra_k[i], "per_unit_count"))
                (*nodes_extra)++;
    }
    for (int i = 0; i < e->n_child; i++)   x_walk_per_unit(e->child[i], nodes_field, nodes_extra, abilities);
    for (int i = 0; i < e->n_options; i++) x_walk_per_unit(e->options[i], nodes_field, nodes_extra, abilities);
    x_walk_per_unit(e->primary_effect, nodes_field, nodes_extra, abilities);
    x_walk_per_unit(e->alternative_effect, nodes_field, nodes_extra, abilities);
    x_walk_per_unit(e->look_action, nodes_field, nodes_extra, abilities);
    x_walk_per_unit(e->select_action, nodes_field, nodes_extra, abilities);
    x_walk_per_unit(e->followup_action, nodes_field, nodes_extra, abilities);
    x_walk_per_unit(e->optional_action, nodes_field, nodes_extra, abilities);
    x_walk_per_unit(e->conditional_action, nodes_field, nodes_extra, abilities);
    x_walk_per_unit(e->gained_effect, nodes_field, nodes_extra, abilities);
    x_walk_per_unit(e->resource_on_select, nodes_field, nodes_extra, abilities);
    x_walk_per_unit(e->opponent_action, nodes_field, nodes_extra, abilities);
}

/* Assert the premise of every test below: the key is NOT in the extras table,
 * so a reader that consults only the extras table sees NULL. This is what
 * rb_vm_decode_extras produces, and what makes the readers below discriminating. */
static void check_key_absent_from_extras(AbilityEffect *e, const char *key)
{
    const char *found = NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) found = e->extra_v[i];
    checks++;
    if (found) {
        failures++;
        fprintf(stderr,
                "FAIL: premise: '%s' must not be in the extras table "
                "(got \"%s\"); the fixture does not model vm.c\n", key, found);
    }
}

/* ===================================================================== */
/* 2. score.c:1031 -- rb_execute_modify_required_hearts_success           */
/*    Rust: score.rs:792-853, reached from dispatch.rs:97-100            */
/*    ActionType::ModifyRequiredHeartsSuccess. The key is read typed:     */
/*    `card_type` decides WHICH cards get the need-heart modifier.         */
/* ===================================================================== */

static void test_x_modify_required_hearts_success_reads_card_type(void)
{
    /* Rust (score.rs:792-853): the target set is chosen by
     * `effect.card_type_any()`. With `Some("live_card")` the modifier lands on
     * the SUCCESS-ZONE cards. The C read the key from the extras table, so it
     * ALWAYS saw NULL, `nids` stayed 0 and the whole effect was a silent
     * no-op. */
    static TestGame tg;
    test_game_new(&tg);
    int in_success = test_new_id(&tg, MEMBER_MUS);
    int not_in_it  = test_new_id(&tg, MEMBER_MUS2);
    CHECK(in_success >= 0 && not_in_it >= 0, "score fixtures resolve");
    if (in_success < 0 || not_in_it < 0) return;

    /* Only the success-zone card is a target. The other card is a card of the
     * same kind that is NOT in the success zone -- the pair makes the
     * card_type selection observable. */
    test_add_to_success(&tg, in_success);
    test_add_to_hand(&tg, not_in_it);

    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"modify_required_hearts_success";
    e.target = (char *)"self";
    e.count = 2;
    snprintf(e.card_type_field, sizeof(e.card_type_field), "live_card");
    /* `operation` is NOT a class key: vm.c:709 routes it through
     * effect_set_extra, so the extras table really does hold it. */
    fx_set(&e, 0, "operation", "increase");
    check_key_absent_from_extras(&e, "card_type");

    rb_execute_modify_required_hearts_success(&tg.state, 0, &e);

    /* Summing every colour avoids depending on the heart-index convention.
     * Positive control: the effect really ran at all (count=2, increase). */
    int got_target = 0;
    for (int col = 0; col < 8; col++)
        got_target += rb_mods_get_need_heart(&tg.state.mods, in_success, col);
    CHECK(got_target > 0,
          "score: the success-zone card received the modifier (positive control)");

    int got_other = 0;
    for (int col = 0; col < 8; col++)
        got_other += rb_mods_get_need_heart(&tg.state.mods, not_in_it, col);
    CHECK_EQ(got_other, 0,
             "score: card_type_field \"live_card\" selected ONLY the success-zone "
             "card (score.rs:792-853 reads effect.card_type_any(), not an extras read)");
}

/* ===================================================================== */
/* 3. look.c:905 -- rb_effect_reveal_until_chosen_card                    */
/*    Rust: look.rs:1371-1406, reached from dispatch.rs:82                */
/*    ActionType::RevealUntilChosenCard. Reads `effect.card_type_any()` at */
/*    :1384 and delegates to execute_reveal_until_target (:1328-1369).    */
/* ===================================================================== */

static void test_x_reveal_until_chosen_card_reads_card_type(void)
{
    /* Rust (look.rs:1377-1406): `chosen_card_type` is the queue entry's
     * conditional_choice else `effect.card_type_any()`. With Some(t) the reveal
     * terminates on the first card matching t; with None it falls to the
     * or_card_types prompt branch and reveals nothing matching. */
    static TestGame tg;
    test_game_new(&tg);
    /* deck, top first: ENERGY, MEMBER, LIVE. A member_card filter skips the
     * energy and terminates at the member (pool 2). The broken reader saw NULL
     * and fell back to card_is_live_pred, which skips BOTH the energy and the
     * member and terminates at the live card (pool 3). The pool size alone
     * separates the two readers. */
    int energy_top = test_new_id(&tg, ENERGY_CARD);
    int member    = test_new_id(&tg, MEMBER_MUS);
    int live_bot  = test_new_id(&tg, LIVE_CARD);
    CHECK(energy_top >= 0 && member >= 0 && live_bot >= 0, "look fixtures resolve");
    if (energy_top < 0 || member < 0 || live_bot < 0) return;
    /* The C deck bag is TOP-INDEX-0 (test_parity_search_reorder harness note),
     * and test_add_to_deck appends, so the FIRST call lands at the top. */
    test_add_to_deck(&tg, energy_top);
    test_add_to_deck(&tg, member);
    test_add_to_deck(&tg, live_bot);

    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"reveal_until_chosen_card";
    e.source = (char *)"deck_top";
    e.target = (char *)"self";
    e.count = 3;
    snprintf(e.card_type_field, sizeof(e.card_type_field), "member_card");
    check_key_absent_from_extras(&e, "card_type");

    rb_effect_reveal_until_chosen_card(&tg.state, 0, &e);

    int pool[8];
    int n = rb_looked_at_pool(0, pool, 8);
    /* The reveal stops AT the member: the energy above it plus the member. It
     * must NOT continue to the live card, which is what the broken reader did
     * (NULL card_type -> card_is_live_pred). */
    CHECK_EQ(n, 2,
             "look: reveal_until_chosen_card terminated on card_type_field "
             "\"member_card\", not on the live card below it "
             "(look.rs:1384 reads effect.card_type_any())");
    CHECK(n >= 1 && pool[n - 1] == member,
          "look: the member card is the matched (last revealed) card");
}

/* ===================================================================== */
/* 4. look.c:917 -- rb_effect_reveal_until_target                         */
/*    Rust: look.rs:1328-1369 (public; also called from look.rs:1400).    */
/* ===================================================================== */

static void test_x_reveal_until_target_reads_card_type(void)
{
    /* Rust (look.rs:1336/1344): `card_type_owned` is the caller's chosen type
     * and the termination_check is `card_matches_type(card_db, cid,
     * card_type_owned.as_deref())`. A NULL type never matches a member. */
    static TestGame tg;
    test_game_new(&tg);
    int energy_top = test_new_id(&tg, ENERGY_CARD);
    int member    = test_new_id(&tg, MEMBER_MUS);
    int live_bot  = test_new_id(&tg, LIVE_CARD);
    CHECK(energy_top >= 0 && member >= 0 && live_bot >= 0, "look fixtures resolve");
    if (energy_top < 0 || member < 0 || live_bot < 0) return;
    /* The C deck bag is TOP-INDEX-0 (test_parity_search_reorder harness note),
     * and test_add_to_deck appends, so the FIRST call lands at the top. */
    test_add_to_deck(&tg, energy_top);
    test_add_to_deck(&tg, member);
    test_add_to_deck(&tg, live_bot);

    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"reveal_until_target";
    e.source = (char *)"deck_top";
    e.target = (char *)"self";
    e.count = 3;
    snprintf(e.card_type_field, sizeof(e.card_type_field), "member_card");
    check_key_absent_from_extras(&e, "card_type");

    rb_effect_reveal_until_target(&tg.state, 0, &e);

    int pool[8];
    int n = rb_looked_at_pool(0, pool, 8);
    CHECK_EQ(n, 2,
             "look: reveal_until_target terminated on card_type_field "
             "\"member_card\" (look.rs:1344 reads the chosen card_type)");
    /* look.rs:1362-1367 moves the matched card to the FRONT of the pool
     * (`matched_idx` reordering), so the member is pool[0], not pool[n-1]. */
    CHECK(n >= 1 && pool[0] == member,
          "look: reveal_until_target moved the matched member to the pool front "
          "(look.rs:1362-1364)");
}

/* ===================================================================== */
/* 5. resolver.c:595 -- apply_per_unit_cost_reduction                     */
/*    Rust: resolver.rs:1246-1248 -> apply_per_unit_cost_reduction        */
/*    (:1196-1253), which reads `mod_cost.per_unit_count_any()` at :1497.  */
/*    The reader is file-static, so this pins the DECODER contract the      */
/*    reader depends on, over the whole shipped corpus.                   */
/* ===================================================================== */

static void test_x_per_unit_count_is_never_an_extra(void)
{
    /* Walks the real bytecode with the engine's own decoder and asserts the
     * invariant the reader now relies on: every `per_unit_count` the wire
     * carries is in AbilityEffect::per_unit_count, and NONE of them is in the
     * extras table. If a future decoder change ever starts writing the key to
     * the extras table as well, this goes red and the field-first read is
     * still correct; if a node ever carries the value ONLY as an extra, the
     * field-first read would silently lose it and this goes red too. */
    long nodes_field = 0, nodes_extra = 0, abilities = 0;

    for (uint32_t id = 0; id < 40000u; id++) {
        uint32_t n = rb_card_num_abilities(id);
        if (!n) continue;
        for (uint32_t k = 0; k < n; k++) {
            Ability ab;
            if (!rb_decode_card_ability(id, k, &ab)) continue;
            x_walk_per_unit(ab.effect, &nodes_field, &nodes_extra, &abilities);
            rb_free_ability(&ab);
        }
    }
    CHECK(abilities > 0,
          "resolver: the corpus really has modify_cost+per_unit nodes (vacuity control)");
    CHECK(nodes_field > 0,
          "resolver: per_unit_count is present in the STRUCT FIELD (resolver.rs:1497)");
    CHECK_EQ(nodes_extra, 0,
            "resolver: per_unit_count is NEVER in the extras table, which is why "
            "resolver.c:595 read NULL and every per-unit reduction divided by 1");
}

/* ===================================================================== */
/* 6. card_type -- the same contract for the 1221-node key                */
/* ===================================================================== */

static void test_x_card_type_is_never_an_extra(void)
{
    long nodes_field = 0, nodes_extra = 0, nodes_reveal_chosen = 0,
         nodes_reveal_target = 0, nodes_mrhs = 0;

    /* Walks the whole decoded effect tree, not just the root: the corpus census
     * measured 4024 effect nodes, and `card_type` is carried on 1221 of them
     * across the tree (1221 includes children such as a sequential's steps). */
    for (uint32_t id = 0; id < 40000u; id++) {
        uint32_t n = rb_card_num_abilities(id);
        if (!n) continue;
        for (uint32_t k = 0; k < n; k++) {
            Ability ab;
            if (!rb_decode_card_ability(id, k, &ab)) continue;
            x_walk_card_type(ab.effect, &nodes_field, &nodes_extra,
                             &nodes_reveal_chosen, &nodes_reveal_target,
                             &nodes_mrhs);
            rb_free_ability(&ab);
        }
    }
    CHECK(nodes_field > 1000,
          "look/score: card_type is carried in the STRUCT FIELD on the corpus "
          "(vm.c:1098-1112 stores e->card_type_field and continues)");
    CHECK_EQ(nodes_extra, 0,
            "look/score: card_type is NEVER in the extras table, which is why "
            "look.c:905/917 and score.c:1031 all read NULL");
    /* The three readers are corpus-inert. Recorded as measurements so the next
     * agent does not re-derive them; the synthetic cases above are what pins
     * the readers themselves. */
    CHECK_EQ(nodes_reveal_chosen, 0,
            "look: no corpus ability uses reveal_until_chosen_card (so the fix is "
            "inert here; Rust says the same at look.rs:1395-1396)");
    CHECK_EQ(nodes_reveal_target, 0,
            "look: no corpus ability uses reveal_until_target");
    CHECK_EQ(nodes_mrhs, 0,
            "score: no corpus ability uses modify_required_hearts_success");
}

/* ===================================================================== */

int main(int argc, char **argv)
{
    if (argc > 1) only_case = argv[1];
    /* Load before forking: test_game_new does NOT load, and rb_decode_card_by_index
     * / rb_find_card_by_no both need the database resident in the child. */
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    signal(SIGSEGV, on_fatal_signal);
    signal(SIGABRT, on_fatal_signal);
    signal(SIGBUS,  on_fatal_signal);

    run("x/modify_required_hearts_success_reads_card_type",
        test_x_modify_required_hearts_success_reads_card_type);
    run("x/reveal_until_chosen_card_reads_card_type",
        test_x_reveal_until_chosen_card_reads_card_type);
    run("x/reveal_until_target_reads_card_type",
        test_x_reveal_until_target_reads_card_type);
    run("x/per_unit_count_is_never_an_extra",
        test_x_per_unit_count_is_never_an_extra);
    run("x/card_type_is_never_an_extra",
        test_x_card_type_is_never_an_extra);

    printf("\n==== test_parity_x_class: %d cases ====\n"
           "%d ok, %d failed, %d crashed\n"
           "%ld checks, %d failures\n",
           n_ok + n_failed + n_crashed, n_ok, n_failed, n_crashed,
           checks, failures);

    return (failures == 0 && n_failed == 0 && n_crashed == 0) ? 0 : 1;
}
