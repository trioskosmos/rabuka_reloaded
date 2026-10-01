/* test_audit_actions.c
 *
 * Action-dispatch audit for src/ability/enums.c and
 * src/ability/effects/executor.c.
 *
 * WHAT IS ACTUALLY BEING AUDITED (and what the earlier premise got wrong)
 * -----------------------------------------------------------------------
 * The task card this suite was written from asserted "Rust's ActionType enum
 * has about 160 variants" and "a 94-variant gap". Both numbers are wrong, and
 * the error is a category error worth recording:
 *
 *   - engine/src/ability/enums.rs:295  `pub enum ActionType`  has 68 variants,
 *     and its `wire_tables!` block (enums.rs:411-490) emits exactly 68 wire
 *     strings. That is the ability-EFFECT action enum.
 *   - engine/src/game/game_setup.rs:51 `pub enum ActionType` has 27 variants
 *     and is a DIFFERENT type with the same name: the interactive game/UI
 *     protocol (Pass, RockChoice, SelectMulligan, Concede, ChoiceSelect, ...).
 *     68 + 27 = 95, which is where the "94 variants" figure comes from. None
 *     of its variants is an ability action, and none of them is missing from
 *     C for that reason. There is no 94-variant ability-action gap.
 *
 * The real, measured ability-action gap is TWO wire strings, and both are
 * corpus-visible:
 *
 *   engine/src/ability/enums.rs:439  ReplacementAppearWait => "replacement_appear_wait"
 *   engine/src/ability/enums.rs:440  CountAsMultiple       => "count_as_multiple"
 *
 * Corpus population (read out of cards/abilities.json, UTF-8):
 *   47 distinct `action` strings across all 985 unique_abilities.
 *   40 of them on MEMBER-card abilities only -- that is the population
 *       test_parity_char2.c's #F census walks (`rb_card_is_member` filter),
 *       so "40" and "47" are both correct for their populations and are not
 *       in conflict. replacement_appear_wait is in the 40 (its card is a
 *       member); count_as_multiple is in the 47 but NOT the 40 (its card,
 *       PL!-pb2-041-L, is a live card).
 *
 * ALSO STALE: the task card cites `cards/ability_extraction/parser.py:195`
 * building an `action_coverage` field in abilities.json. No such field exists
 * in the current abilities.json and no .py under cards/ references
 * rb_executor_has_executor any more. rb_executor_has_executor is still the
 * live reachability predicate -- engine.c:315 routes on it and misc.c:1688
 * consults it -- but it is NOT generated into the corpus file, so there is no
 * circular coverage claim to audit. The census here is computed from the
 * compiled bytecode at run time instead.
 *
 * WHAT IS ASSERTED, AND WHY IT IS STRICT
 * -------------------------------------
 * Green cases assert C behaviour that now matches Rust. Red-pinned cases
 * assert the RUST behaviour and are expected to FAIL; run_case() classifies
 * each outcome as GREEN-pass / GREEN-fail (ENGINE BUG, a regression) /
 * RED-fail (gap still open) / RED-pass (TEST BUG, vacuous assertion). A
 * red-pinned case that passes is reported as loudly as a green case that
 * fails, because an assertion that cannot fail pins nothing.
 *
 * The two fixes this suite covers are deliberately NO-OPS, because Rust's are:
 * dispatch.rs:126 and dispatch.rs:129 are both `=> Ok(())`. Registering them in
 * executor_table changes three observable things and nothing else:
 *   1. rb_executor_has_executor reports them reachable (the #F census
 *      assertion in test_parity_char2.c:1484 goes green),
 *   2. rb_executor_execute no longer bails at its own find_executor guard
 *      printing [EXECUTOR_MISS] (executor.c:306) and returning 0,
 *   3. the verdict is pushed, exactly as Rust pushes one for every
 *      non-structural action at dispatch.rs:233.
 * Their actual rules are consumed elsewhere (see RED cases) and those
 * consumers do not exist in C yet.
 *
 * PROCESS MODEL: one fork() per case, with a fatal-signal handler that names
 * the case in flight. Counters live in a MAP_SHARED|MAP_ANONYMOUS mapping
 * ZEROED IN THE CHILD (a forked child inherits the parent's) and read by the
 * parent AFTER waitpid, so a child that dies mid-case still reports how far it
 * got.
 *
 * FILE IS PURE 7-BIT ASCII. Every Japanese wire value is referenced by its
 * effect key or its English reading, never by a hand-typed literal: a
 * non-ASCII literal that a tool rewrites in a legacy code page becomes a
 * silent no-op filter, which is precisely the class of defect under audit.
 */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE 1
#endif
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/wait.h>

/* ------------------------------------------------------------------ harness */

typedef struct {
    volatile int assertions;
    volatile int failures;
    volatile int signal;
    volatile int exited_ok;
    volatile int cases;
    char          name[96];
} SharedCounters;

static SharedCounters *g_shared;
static int failures;
static int assertions;
static char g_case_name[96];

static void publish_counts(void)
{
    if (g_shared) g_shared->failures = failures;
}

static void fatal_handler(int sig)
{
    if (g_shared) {
        g_shared->signal = sig;
        g_shared->failures = failures;
        size_t i = 0;              /* snprintf is not async-signal-safe */
        for (; i + 1 < sizeof g_shared->name && g_case_name[i]; i++)
            g_shared->name[i] = g_case_name[i];
        g_shared->name[i] = 0;
    }
    _exit(0);
}

/* Variadic on purpose. A two-argument CHECK(cond, "a" "b") silently splits
 * the adjacent string literals and prints only the first half, which is how a
 * diagnosis gets lost. Every message goes through snprintf. */
#define CHECK(condition, ...) do { \
    assertions++; \
    if (!(condition)) { \
        char msg_[600]; \
        snprintf(msg_, sizeof msg_, __VA_ARGS__); \
        fprintf(stderr, "FAIL: %s\n", msg_); \
        failures++; publish_counts(); \
    } else { \
        char msg_[600]; \
        snprintf(msg_, sizeof msg_, __VA_ARGS__); \
        printf("ok: %s\n", msg_); \
    } \
} while (0)

static int load_card_db(void)
{
    if (rb_load("src") == 0) return 0;
    if (rb_load("../cards/build") == 0) return 0;
    if (rb_load("cards/build") == 0) return 0;
    return -1;
}

/* ----------------------------------------------------------------- fixtures
 * Every fixture helper RETURNS the seeded id. Re-deriving an id with a second
 * test_new_id at the assertion site mints a NEW pool instance, so the id
 * compared against a zone never matches the id that was placed there. */

/* Ayanagi Erii -- the only card in the corpus carrying replacement_appear_wait.
 * Constant (jyouji) ability: position=center, card_type=member_card,
 * heart_total_limit=4, heart_total_operator="<=". */
#define ERI_NO    "PL!-pb2-002-PP"
#define ROMANTIC_NO "PL!-pb2-041-L"   /* carries count_as_multiple, a LIVE card */
#define MEMBER_NO "PL!-bp4-003-P"     /* Kotori Minami, mu's member */
#define OTHER_NO  "PL!-sd1-010-SD"    /* distinct member, verified to resolve */

static void clear_board(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *P = &tg->state.p[pl];
        P->deck.n = P->hand.n = P->discard.n = P->live.n = P->success.n = 0;
        P->energy.n = P->energy_deck.n = 0;
        P->energy_active = 0;
        P->score = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            P->stage[i] = RB_EMPTY_SLOT;
            P->stage_wait[i] = 0;
            P->under_cards[i].n = 0;
        }
    }
    tg->state.n_revealed = 0;
    tg->state.n_selected_cards = 0;
    tg->state.n_recently_moved = 0;
    tg->state.n_those_cards = 0;
    tg->state.activating_card = -1;
    tg->state.phase = RB_PHASE_ACTIVE;
}

static void set_extra(AbilityEffect *e, const char *k, const char *v)
{
    if (e->n_extra >= RB_MAX_EXTRA) return;
    e->extra_k[e->n_extra] = (char *)k;
    e->extra_v[e->n_extra] = (char *)v;
    e->n_extra++;
}

/* A minimal one-node effect tree. The vm allocates these with rb_malloc via
 * rb_decode_ability; for a hand-built probe the same allocator is used so the
 * tree can be handed to rb_effect_free without a special case. */
static AbilityEffect *make_effect(const char *action)
{
    AbilityEffect *e = (AbilityEffect *)calloc(1, sizeof *e);
    if (!e) return NULL;
    e->action  = (char *)action;
    e->count   = -1;
    e->text    = (char *)"audit probe";
    return e;
}

static void free_effect(AbilityEffect *e)
{
    if (e) { e->action = NULL; e->text = NULL; free(e); }
}

/* =========================================================================
 * LAYER 1 -- the reachability chain, asserted GREEN.
 * These are the fixes: the two no-op actions are registered and dispatched.
 * ====================================================================== */

/* The complete Rust ability-action wire table, transcribed from
 * engine/src/ability/enums.rs:411-490 in wire_tables! order. 68 entries.
 * This is the SPEC, not the C output: transcribing it by hand is the only way
 * the comparison can fail. A script that read the C table back would be
 * circular. */
static const char *const rust_wire_table[] = {
    /* Card movement */
    "draw_card", "draw_until_count", "move_cards", "discard_card", "select",
    "select_number", "select_cards", "look_and_select", "look_at", "reveal",
    "reveal_per_group", "reveal_until_live_card", "reveal_until_chosen_card",
    /* State changes */
    "change_state", "position_change", "rotation", "place_energy_under_member",
    "set_card_identity", "modify_required_hearts_success", "gain_resource",
    "pay_energy",
    /* Ability modifications */
    "gain_ability", "gain_ability_from_source", "invalidate_ability",
    "replacement_appear_wait", "count_as_multiple", "suppress_ability_trigger",
    "activate_ability",
    /* Cost modifications */
    "modify_cost", "modify_yell_source", "set_cost", "set_cost_to_use",
    /* Score and hearts */
    "modify_score", "modify_required_hearts",
    /* Blade and heart */
    "set_blade_type", "set_blade_count", "set_heart_type",
    "specify_heart_color", "choose_required_hearts",
    /* Compound effects */
    "sequential", "conditional_alternative", "conditional_on_result",
    "conditional_on_optional",
    /* Restrictions and limits */
    "restriction", "activation_restriction", "modify_limit",
    /* Utility */
    "shuffle", "re_yell", "custom", "do_nothing", "choice", "repeat_procedure",
    "discard_until_count",
    /* Replacement and triggers */
    "all_blade_timing", "reduce_live_card_set_limit",
    /* Player target choice / selection / missing variants */
    "choose_target_player", "play_baton_touch", "modify_required_hearts_global",
    "modify_yell_count", "activation_cost", "perform_yell",
    /* Internal/procedural, not from JSON */
    "conditional_optional", "compound_action", "opponent_action", "action_by",
    "sequential_cost", "choice_condition", "energy_condition"
};

static void test_rust_wire_table_fully_reachable(TestGame *tg_in)
{
    int n = (int)(sizeof rust_wire_table / sizeof rust_wire_table[0]);
    /* The table is transcribed, so its size is a fact about this file: if a
     * future port adds a Rust variant this must be re-checked by hand. */
    CHECK(n == 68, "the transcribed Rust ability-action wire table has 68 "
                   "entries, matching enums.rs:295-490 (got %d). The separate "
                   "game/UI enum at game_setup.rs:51 (27 variants) is a "
                   "DIFFERENT type and is deliberately not counted", n);

    int missing = 0;
    for (int i = 0; i < n; i++) {
        if (!rb_executor_has_executor(rust_wire_table[i])) {
            fprintf(stderr, "  no executor for Rust wire string: %s\n",
                    rust_wire_table[i]);
            missing++;
        }
    }
    CHECK(missing == 0, "every one of the %d Rust ability-action wire strings "
                        "is reachable through rb_executor_has_executor "
                        "(%d unreachable)", n, missing);
}

static void test_two_gap_actions_are_registered(TestGame *tg_in)
{
    /* engine/src/ability/enums.rs:439-440. Both were absent from
     * rb_action_type_from_str AND from executor_table before this audit. */
    CHECK(rb_executor_has_executor("replacement_appear_wait"),
          "replacement_appear_wait is registered in executor_table (was the "
          "silent drop: rb_execute_effect_ex routed it to engine.c's "
          "handle_action, whose final arm is an explicit no-op, engine.c:783)");
    CHECK(rb_executor_has_executor("count_as_multiple"),
          "count_as_multiple is registered in executor_table (same silent "
          "drop; its rule is a counting WEIGHT consumed by Rust's "
          "weighted_zone_count, condition/card.rs:2124)");
    /* Neither is verdict-structural: Rust's push_effect_verdict structural
     * set (dispatch.rs:31-38) is exactly the six actions listed in
     * executor.c's verdict_structural, and neither of these is one of them. */
    CHECK(!rb_executor_is_structural("replacement_appear_wait") &&
          !rb_executor_is_structural("count_as_multiple"),
          "neither gap action is verdict-structural, so both log a verdict "
          "exactly as Rust logs one at dispatch.rs:233");
}

/* Rust dispatch.rs:126 and :129 are `=> Ok(())`. So a dispatched instance must
 * resolve successfully AND change nothing. Asserted against a census taken
 * AFTER the fixtures are on the board, so a placement the probe caused would
 * show up as a delta rather than being absorbed into the baseline. */
static void test_gap_actions_resolve_without_mutating(TestGame *tg_in)
{
    static const char *const gaps[] = {
        "replacement_appear_wait", "count_as_multiple"
    };
    for (unsigned g = 0; g < sizeof gaps / sizeof gaps[0]; g++) {
        TestGame *tg = tg_in;
        clear_board(tg);

        int member = test_id(tg, MEMBER_NO);
        int other  = test_id(tg, OTHER_NO);
        CHECK(member >= 0 && other >= 0, "%s: both member fixtures resolve "
              "(%s, %s)", gaps[g], MEMBER_NO, OTHER_NO);
        if (member < 0 || other < 0) continue;

        test_add_to_stage(tg, 1, member);
        tg->state.p[1].stage[1] = other;

        /* Baseline AFTER every fixture card is placed. */
        int before = test_total_card_count(tg);

        AbilityEffect *e = make_effect(gaps[g]);
        CHECK(e != NULL, "%s: probe effect allocated", gaps[g]);
        if (!e) continue;

        int rc = rb_executor_execute(&tg->state, 0, e, member);
        CHECK(rc == 1, "%s: rb_executor_execute returns 1, matching Rust's "
                       "`Ok(())` at dispatch.rs:%d (got %d)",
              gaps[g], gaps[0][0] == 'r' ? 126 : 129, rc);

        int after = test_total_card_count(tg);
        CHECK(after == before, "%s: the no-op dispatch moved no card "
                               "(census %d before, %d after)",
              gaps[g], before, after);

        free_effect(e);
    }
}

/* Rust pushes the verdict unconditionally after the match, including on the
 * `Ok(())` arms (dispatch.rs:233). Before the fix rb_executor_execute
 * returned at its find_executor guard (executor.c:306) BEFORE reaching the
 * push, so these two logged nothing -- the observable half of the "silent
 * drop" that the census assertion could not see. */
static void test_gap_actions_log_one_verdict(TestGame *tg_in)
{
    static const char *const gaps[] = {
        "replacement_appear_wait", "count_as_multiple"
    };
    for (unsigned g = 0; g < sizeof gaps / sizeof gaps[0]; g++) {
        TestGame *tg = tg_in;
        clear_board(tg);
        int member = test_id(tg, MEMBER_NO);
        if (member < 0) { CHECK(0, "%s: member fixture resolves", gaps[g]); continue; }

        /* push_effect_verdict -> rb_log_push_verdict_effect is itself gated on
         * g_log_enabled (log.c), the C twin of Rust's debug_enabled() early
         * return at dispatch.rs:27. Without this the dispatch logs nothing and
         * the assertion below would pass vacuously for the WRONG reason. */
        rb_log_set_enabled(1);
        rb_log_drain_verdicts(NULL, 0);   /* start from a drained log */

        AbilityEffect *e = make_effect(gaps[g]);
        e->count = 4;                     /* the corpus value for Eri's rule */
        if (e) rb_executor_execute(&tg->state, 0, e, member);

        RbAbilityLogItem items[64];
        int n = rb_log_drain_verdicts(items, 64);
        int found = 0;
        char details[128];
        details[0] = '\0';
        for (int i = 0; i < n; i++) {
            if (items[i].kind != RB_LOG_KIND_EFFECT) continue;
            if (strcmp(items[i].as.effect.action, gaps[g]) != 0) continue;
            if (!found) snprintf(details, sizeof details, "%s",
                                 items[i].as.effect.details);
            found++;
        }
        for (int i = 0; i < n; i++) rb_log_free_item(&items[i]);

        CHECK(found == 1, "%s: dispatch logs exactly ONE verdict, as Rust does "
                          "at dispatch.rs:233 (got %d)", gaps[g], found);
        char expect[128];
        snprintf(expect, sizeof expect, "%s 4", gaps[g]);
        CHECK(strcmp(details, expect) == 0,
              "%s: the verdict detail is \"<action> <count>\", matching Rust's "
              "`format!(\"{} {}\", effect.action, val)` at dispatch.rs:49-50 "
              "(expected \"%s\", got \"%s\")", gaps[g], expect, details);
        free_effect(e);
    }
}

/* The census the #F case in test_parity_char2.c runs, but over EVERY card type
 * rather than members only, so count_as_multiple (a live card) is included.
 * Computed from the compiled bytecode at run time, not read out of the corpus
 * file, because there is no generated coverage field to read (see the header). */
static int census_action(char names[][64], int max)
{
    int n = 0;
    for (uint32_t a = 0; a < rb_num_abilities(); a++) {
        Ability ab;
        if (!rb_decode_ability(a, &ab)) continue;
        AbilityEffect *stack[128];
        int top = 0;
        if (ab.effect) stack[top++] = ab.effect;
        for (int i = 0; i < top && i < 128; i++) {
            AbilityEffect *e = stack[i];
            if (!e) continue;
            if (e->action && *e->action) {
                int seen = 0;
                for (int k = 0; k < n; k++)
                    if (strcmp(names[k], e->action) == 0) { seen = 1; break; }
                if (!seen && n < max)
                    snprintf(names[n++], 64, "%s", e->action);
            }
            if (e->primary_effect      && top < 128) stack[top++] = e->primary_effect;
            if (e->alternative_effect  && top < 128) stack[top++] = e->alternative_effect;
            if (e->look_action         && top < 128) stack[top++] = e->look_action;
            if (e->select_action       && top < 128) stack[top++] = e->select_action;
            if (e->followup_action     && top < 128) stack[top++] = e->followup_action;
            if (e->optional_action     && top < 128) stack[top++] = e->optional_action;
            if (e->conditional_action  && top < 128) stack[top++] = e->conditional_action;
            if (e->gained_effect       && top < 128) stack[top++] = e->gained_effect;
            if (e->resource_on_select  && top < 128) stack[top++] = e->resource_on_select;
            if (e->opponent_action     && top < 128) stack[top++] = e->opponent_action;
            for (int k = 0; k < e->n_child   && top < 128; k++) stack[top++] = e->child[k];
            for (int k = 0; k < e->n_options && top < 128; k++) stack[top++] = e->options[k];
        }
        rb_free_ability(&ab);
    }
    return n;
}

static void test_corpus_census_fully_reachable(TestGame *tg_in)
{
    static char names[512][64];
    int n = census_action(names, 512);

    CHECK(n >= 40, "the action census is large enough not to be a truncated "
                   "walk (%d distinct actions)", n);
    CHECK(n <= 512, "the action census FIT without hitting the cap (%d)", n);
    CHECK(n == 47, "the corpus uses 47 distinct action strings across all card "
                   "types; 40 of them are on member cards (the narrower "
                   "population test_parity_char2.c's #F census walks). Got %d",
          n);

    int missing = 0;
    for (int i = 0; i < n; i++)
        if (!rb_executor_has_executor(names[i])) {
            fprintf(stderr, "  unreachable corpus action: %s\n", names[i]);
            missing++;
        }
    CHECK(missing == 0, "every one of the %d corpus action strings is "
                        "reachable through the dispatch table (%d are not)",
          n, missing);
}

/* =========================================================================
 * LAYER 2 -- RED-PINNED. Each asserts the RUST behaviour and is expected to
 * FAIL. Each is classified: engine bug, enum/parser gap, or test bug.
 * ====================================================================== */

/* RED 1. engine/src/ability/enums.rs:439 declares the variant and emits the
 * wire string, but RbActionType in include/rabuka.h has no corresponding
 * constant, so rb_action_type_from_str returns -1 and the string is not
 * round-trippable. This audit does not own rabuka.h; the exact declaration is
 * in the final report. test_parity_char2.c's second #F assertion
 * (:1488) is red for the same reason and is NOT fixed here. */
static void test_action_type_from_str_knows_both(TestGame *tg_in)
{
    CHECK(rb_action_type_from_str("replacement_appear_wait") >= 0,
          "rb_action_type_from_str(\"replacement_appear_wait\") resolves, "
          "matching enums.rs:439. RED until RB_ACTION_REPLACEMENT_APPEAR_WAIT "
          "is appended to RbActionType in include/rabuka.h:552-619");
    CHECK(rb_action_type_from_str("count_as_multiple") >= 0,
          "rb_action_type_from_str(\"count_as_multiple\") resolves, matching "
          "enums.rs:440. RED until RB_ACTION_COUNT_AS_MULTIPLE is appended to "
          "RbActionType in include/rabuka.h:552-619");

    /* Round-trip is the property that actually matters: a value that resolves
     * but whose to_str disagrees would corrupt RbGeneratedAction.action_type
     * (used by rb_generate_action_candidates, e.g. test_parity_wave1c.c:1510). */
    int raw = rb_action_type_from_str("replacement_appear_wait");
    if (raw >= 0) {
        const char *back = rb_action_type_to_str(raw);
        CHECK(back && strcmp(back, "replacement_appear_wait") == 0,
              "the resolved value round-trips through rb_action_type_to_str "
              "(got \"%s\")", back ? back : "(null)");
        CHECK(rb_action_type_label(raw) != NULL,
              "rb_action_type_label has a label for it, as enums.rs:523 "
              "provides for every variant");
    }
}

/* RED 2. engine/src/ability/effects/target.rs:81-148 runs handle_both_targets
 * for EVERY action BEFORE dispatch (mod.rs:252), so any effect carrying
 * target="both" runs once for self and once for opponent. The C only has that
 * split in two places: rb_misc_handle_both_targets (misc.c:1726, reached only
 * from rb_execute_misc_effect at misc.c:1853) and
 * execute_move_cards_both_targets (executor.c:211). An action routed to
 * NEITHER collapses "both" onto the actor.
 *
 * Corpus victims (read out of cards/abilities.json):
 *   change_state   target=both on PL!-bp5-111-P+ (Kiramune Tsubasa, ability 1)
 *                  and PL!HS-pb1-008-P+ (Keijo Izumi). Both card texts say
 *                  "my AND the opponent's stage" explicitly.
 *   select         target=both on PL!S-bp7-004-P (Kurosawa Dia), text says
 *                  "my AND the opponent each, from their OWN hand".
 *   modify_score   target=both on PL!-bp3-022-L (Yume no Tobira), per_unit
 *                  with location=stage, so Rust counts members on BOTH stages.
 *
 * This case drives change_state, the heaviest victim, and asserts the Rust
 * outcome: BOTH members enter wait. C waits only the actor's. */
static void test_change_state_both_targets_split_both_players(TestGame *tg)
{
    clear_board(tg);

    /* Distinct member ids, returned by the helpers -- never re-derived. */
    int mine = test_id(tg, MEMBER_NO);
    int theirs = test_id(tg, OTHER_NO);
    CHECK(mine >= 0 && theirs >= 0,
          "both member fixtures resolve (%s, %s)", MEMBER_NO, OTHER_NO);
    if (mine < 0 || theirs < 0) return;

    /* One active member per side, centre area. */
    tg->state.p[0].stage[1] = mine;
    tg->state.p[0].stage_wait[1] = 0;
    tg->state.p[1].stage[1] = theirs;
    tg->state.p[1].stage_wait[1] = 0;

    int before = test_total_card_count(tg);

    /* The corpus shape of PL!HS-pb1-008-P+ (Keijo Izumi), read out of
     * cards/abilities.json: state_change=wait, card_type=member_card,
     * target=both, all=true, blade_limit=3. */
    AbilityEffect *e = make_effect("change_state");
    e->target = (char *)"both";
    e->count  = 1;
    set_extra(e, "state_change", "wait");
    set_extra(e, "card_type", "member_card");

    rb_execute_effect_ex(&tg->state, 0, e, mine);

    /* The wait is recorded as an ORIENTATION MODIFIER, not in RbPlayer.stage_wait:
     * rb_effect_change_state reads rb_mods_get_orientation (state.c:255) and Rust
     * writes mods.add_orientation_modifier(card, "wait"). stage_wait[] is only
     * what a PLACEMENT clears (engine.c:170), so asserting on it here would
     * pass for the wrong reason. */
    const char *mine_ori   = rb_mods_get_orientation((RbMods *)&tg->state.mods, mine);
    const char *theirs_ori = rb_mods_get_orientation((RbMods *)&tg->state.mods, theirs);
    int actor_waited    = (mine_ori   && !strcmp(mine_ori,   "wait"));
    int opponent_waited = (theirs_ori && !strcmp(theirs_ori, "wait"));

    CHECK(actor_waited == 1,
          "the SELF half of target=\"both\" waits the actor's member "
          "(got stage_wait[0]=%d)", actor_waited);
    CHECK(opponent_waited == 1,
          "the OPPONENT half of target=\"both\" waits the opponent's member too "
          "-- Rust splits in handle_both_targets (target.rs:119-145) before "
          "dispatch, so C's s_who() collapsing \"both\" to the actor "
          "(state.c:1603-1606) is a real defect. Got stage_wait[1]=%d",
          opponent_waited);
    CHECK(test_total_card_count(tg) == before,
          "the wait applied no placement and lost no card "
          "(census %d -> %d)", before, test_total_card_count(tg));
    free_effect(e);
}

/* RED 3. Rust's counting WEIGHT. engine/src/core/game_state/modifiers.rs:259
 * weighted_zone_count sums count_as_multiple_weight over the cards in a zone,
 * and :178-252 implements the weight: the card's own CONSTANT ability must be
 * count_as_multiple, its condition must hold, and `counted_by` must name a
 * group the COUNTING card belongs to. PL!-pb2-041-L counts as 2 when a
 * "lily white" effect counts the success live zone.
 *
 * The C has no weighted count at all: the zone count every card_count /
 * group_condition goes through is rb_zone_cards, which takes no counting-card
 * argument and so cannot apply the weight even in principle. Asserted as the
 * observable: two cards in the zone, one of which is worth 2, so the Rust
 * total is 3 and the C total is 2. */
static void test_count_as_multiple_weights_the_zone_count(TestGame *tg)
{
    clear_board(tg);

    int romantic = test_id(tg, ROMANTIC_NO);
    int other    = test_id(tg, OTHER_NO);
    CHECK(romantic >= 0, "the count_as_multiple card resolves (%s)", ROMANTIC_NO);
    if (romantic < 0) return;

    /* Both cards in the success live zone; one of them counts as 2. */
    test_add_to_success(tg, romantic);
    test_add_to_success(tg, other);

    int ids[16];
    /* The corpus field on this card is source="success_live_zone".
     * rb_zone_cards accepts "success", "success_live_zone" and
     * "success_live_card_zone" (util.c:774-775) -- NOT "success_zone", which
     * rb_ability_zone_from_str accepts and rb_zone_cards does not. */
    int n = rb_zone_cards(&tg->state, 0, "success_live_zone", ids, 16);
    CHECK(n == 2, "both cards are visible to the zone count (got %d)", n);

    int saw_romantic = 0;
    for (int i = 0; i < n; i++) if (ids[i] == romantic) saw_romantic = 1;
    CHECK(saw_romantic, "the count_as_multiple card is among the counted cards");

    /* Rust: 1 (other) + 2 (romantic) = 3. */
    CHECK(n == 3, "the zone count WEIGHS the count_as_multiple card: 1 other + "
                  "2 for PL!-pb2-041-L = 3. RED until C has "
                  "weighted_zone_count (modifiers.rs:259); the seam here is "
                  "rb_zone_cards, which has no counting-card argument. Got %d",
          n);

    /* The weight must not be unconditional either: `counted_by` names the
     * groups whose effects SEE it (modifiers.rs:228-248), and an empty list
     * means any effect sees it. Pin the narrower fact that the rule is keyed
     * on a CONSTANT trigger, which the corpus carries as triggers="jidou". */
    int n_abilities = rb_card_num_abilities((uint32_t)romantic);
    int has_constant_action = 0;
    /* Rust's crate::triggers::CONSTANT. Spelled as explicit UTF-8 byte escapes
     * so the file stays 7-bit clean; a hand-typed literal here would be
     * rewritten by a legacy-codepage tool into a filter that never matches,
     * which would make this assertion vacuously true. The ASCII-only form
     * ("jidou") would never match the C's raw trigger string, so it is NOT
     * used here -- that mistake made the assertion fail for the wrong reason. */
    const char *constant_token = "\xE5\xB8\xB8\xE6\x99\x82";
    for (int i = 0; i < n_abilities; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)romantic, i, &ab)) continue;
        const char *act = ab.effect && ab.effect->action ? ab.effect->action : "";
        const char *trg = ab.triggers ? ab.triggers : "";
        if (!strcmp(act, "count_as_multiple") && strstr(trg, constant_token))
            has_constant_action = 1;
        rb_free_ability(&ab);
    }
    CHECK(has_constant_action,
          "the count_as_multiple rule decodes off the card as a constant "
          "effect, which is the shape Rust's modifier scan requires "
          "(modifiers.rs:200-214)");
}

/* RED 4. engine/src/core/game_state/modifiers.rs:311-402
 * replacement_appear_wait_applies, called once at placement from
 * engine/src/turn/phases.rs:1281. It resolves a REPLACEMENT rule (not a
 * constant), so it must fire exactly when a member is placed and never on a
 * later recalculation.
 *
 * The corpus rule (PL!-pb2-002-PP, Ayanagi Erii): a member whose PRINTED
 * heart total is <= 4 appears in WAIT in the area in front of the host.
 * Rust reads "front of" as the CENTRE area for every slot (modifiers.rs:304,
 * 323-325). Asserted behaviourally through the real placement seam
 * rb_play_member (engine.c:983), which is the C twin of the Rust hook's
 * handle_play_member_to_stage. */
static void test_replacement_appear_wait_holds_the_newcomer(TestGame *tg)
{
    clear_board(tg);

    int eri  = test_id(tg, ERI_NO);
    int mine = test_id(tg, MEMBER_NO);
    CHECK(eri >= 0, "the replacement_appear_wait host resolves (%s)", ERI_NO);
    if (eri < 0) return;

    /* Erii sits in the CENTRE. Her own rule must not match her, which is why
     * modifiers.rs:332-339 skips stage_id == incoming. */
    tg->state.p[0].stage[1] = eri;
    tg->state.p[0].stage_wait[1] = 0;

    /* Precondition, asserted so the case cannot pass for the wrong reason: the
     * newcomer must actually satisfy the <=4 printed-heart bound. */
    int heart_ok = 0;
    {
        Ability ab;
        for (int i = 0; i < rb_card_num_abilities((uint32_t)eri); i++) {
            if (!rb_decode_card_ability((uint32_t)eri, i, &ab)) continue;
            if (ab.effect && ab.effect->action &&
                !strcmp(ab.effect->action, "replacement_appear_wait")) {
                for (int k = 0; k < ab.effect->n_extra; k++)
                    if (ab.effect->extra_k[k] &&
                        !strcmp(ab.effect->extra_k[k], "heart_total_limit")) {
                        int limit = atoi(ab.effect->extra_v[k]);
                        Card c;
                        if (rb_decode_card_by_index((uint32_t)mine, &c)) {
                            /* Rust's printed_hearts is the sum of the card's
                             * base_heart heart counts (modifiers.rs:318-322).
                             * In C that is heart_count[] over n_hearts. */
                            int printed = 0;
                            for (int h = 0; h < c.n_hearts && h < RB_MAX_HEARTS; h++)
                                printed += c.heart_count[h];
                            heart_ok = printed <= limit;
                            rb_free_card(&c);
                        }
                    }
            }
            rb_free_ability(&ab);
        }
    }
    CHECK(heart_ok, "the newcomer %s satisfies Eri's printed-heart bound "
                    "(heart_total_limit <= 4); if this is 0 the RED below is "
                    "vacuous, because the rule would not match even in Rust",
          MEMBER_NO);
    if (!heart_ok) return;

    /* Newcomer in hand, then placed into the CENTRE. */
    test_add_to_hand_for(tg, 0, mine);
    int hand_idx = -1;
    for (int i = 0; i < tg->state.p[0].hand.n; i++)
        if (tg->state.p[0].hand.cards[i] == mine) hand_idx = i;

    int before = test_total_card_count(tg);
    int placed = rb_play_member(&tg->state, 0, hand_idx, 1);

    CHECK(placed == 1, "the newcomer was placed on stage (got %d)", placed);
    CHECK(tg->state.p[0].stage[1] == mine,
          "the newcomer occupies the CENTRE slot (got %d)", tg->state.p[0].stage[1]);
    /* Rust records the hold as an orientation modifier, not as a placement flag:
     * phases.rs:1282-1284 does mods.add_orientation_modifier(card_id, "wait").
     * Asserting on RbPlayer.stage_wait[] would pass for the wrong reason --
     * that field is what a placement CLEARS (engine.c:170), so it is 0 whether
     * or not the rule fired. */
    const char *ori = rb_mods_get_orientation((RbMods *)&tg->state.mods, mine);
    CHECK(ori && !strcmp(ori, "wait"),
          "the newcomer appears in WAIT -- Rust holds it at placement via "
          "replacement_appear_wait_applies (modifiers.rs:311) called from "
          "phases.rs:1281, writing mods.add_orientation_modifier(...,\"wait\"). "
          "RED: C has no such predicate and no placement hook, so rb_play_member "
          "lets it in active. Orientation is \"%s\"", ori ? ori : "(none)");
    CHECK(test_total_card_count(tg) == before,
          "the replacement placed no card and lost none (census %d -> %d)",
          before, test_total_card_count(tg));

    /* The rule must not be a constant: a member already in ACTIVE on stage
     * must NOT be dragged into wait by a later recalculation
     * (modifiers.rs:296-302 explains why). rb_recalc_constants is that pass.
     * The orientation modifier is removed first so the only thing that could
     * re-add it is the constant scan. */
    rb_mods_set_orientation((RbMods *)&tg->state.mods, mine, "active");
    rb_recalc_constants(&tg->state);
    const char *ori2 = rb_mods_get_orientation((RbMods *)&tg->state.mods, mine);
    CHECK(!ori2 || strcmp(ori2, "wait") != 0,
          "a recalculation does NOT re-apply the replacement rule to a member "
          "already in active state (Rust resolves it once, at placement). "
          "Orientation is \"%s\"", ori2 ? ori2 : "(none)");
}

/* ---------------------------------------------------------------- runner */

typedef void (*CaseFn)(TestGame *);

typedef struct {
    const char *name;
    CaseFn      fn;
    int         red_pinned;
    const char *classification;
} Case;

static void run_case(const Case *c)
{
    snprintf(g_case_name, sizeof g_case_name, "%s", c->name);
    if (g_shared) {
        g_shared->assertions = 0;
        g_shared->failures   = 0;
        g_shared->signal     = 0;
        g_shared->exited_ok  = 0;
        g_shared->name[0]    = 0;
        memcpy(g_shared->name, g_case_name, sizeof g_case_name);
    }

    fflush(stdout);
    fflush(stderr);
    pid_t pid = fork();
    if (pid < 0) {
        fprintf(stderr, "FAIL: fork() failed for %s\n", c->name);
        assertions++; failures++;
        return;
    }
    if (pid == 0) {
        struct sigaction sa;
        memset(&sa, 0, sizeof sa);
        sa.sa_handler = fatal_handler;
        sigemptyset(&sa.sa_mask);
        sa.sa_flags = SA_NODEFER | SA_RESETHAND;
        sigaction(SIGSEGV, &sa, NULL);
        sigaction(SIGBUS,  &sa, NULL);
        sigaction(SIGABRT, &sa, NULL);
        sigaction(SIGFPE,  &sa, NULL);

        /* Process-global counters and the monotonic card-copy pool: a child
         * must start from zero or it reports the running total plus its own,
         * and would inherit the previous case's pool entries. */
        assertions = 0;
        failures   = 0;
        rb_card_copy_pool_reset();

        TestGame tg;
        test_game_new(&tg);
        c->fn(&tg);
        if (g_shared) {
            g_shared->assertions = assertions;
            g_shared->failures   = failures;
            g_shared->exited_ok  = 1;
        }
        fflush(stdout);
        fflush(stderr);
        _exit(0);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) status = -1;

    assertions += g_shared ? g_shared->assertions : 0;
    failures   += g_shared ? g_shared->failures   : 0;

    if (g_shared && g_shared->signal) {
        const char *nm = "signal";
        switch (g_shared->signal) {
            case SIGSEGV: nm = "SIGSEGV"; break;
            case SIGBUS:  nm = "SIGBUS";  break;
            case SIGABRT: nm = "SIGABRT"; break;
            case SIGFPE:  nm = "SIGFPE";  break;
            default: break;
        }
        fprintf(stderr, "FAIL: %s: fatal %s (%d) in child after %d assertion(s)\n",
                c->name, nm, g_shared->signal, g_shared->assertions);
        failures++; assertions++;
        g_shared->cases++;
        return;
    }
    if (!g_shared || !g_shared->exited_ok) {
        fprintf(stderr, "FAIL: %s: child terminated abnormally (wait status %d) "
                        "after %d assertion(s)\n", c->name, status,
                g_shared ? g_shared->assertions : 0);
        failures++; assertions++;
        if (g_shared) g_shared->cases++;
        return;
    }

    int failed = g_shared->failures > 0;
    if (!c->red_pinned && failed) {
        printf("== %s: GREEN case FAILED -> %s\n", c->name, c->classification);
        if (g_shared) g_shared->cases++;
    } else if (c->red_pinned && !failed) {
        fprintf(stderr, "TEST BUG: %s: RED-pinned case PASSED -- the assertion "
                        "cannot fail, so it pins nothing [%s]\n",
                c->name, c->classification);
        failures++; assertions++;
        if (g_shared) g_shared->cases++;
    } else if (c->red_pinned) {
        printf("== %s: RED as pinned -> %s (%d failing assertion(s))\n",
               c->name, c->classification, g_shared->failures);
    }
}

int main(void)
{
    if (load_card_db() != 0) {
        fprintf(stderr, "FAIL: database load (tried src, ../cards/build, "
                        "cards/build)\n");
        return 1;
    }
    setvbuf(stdout, NULL, _IONBF, 0);

    g_shared = mmap(NULL, sizeof *g_shared, PROT_READ | PROT_WRITE,
                    MAP_SHARED | MAP_ANONYMOUS, -1, 0);
    if (g_shared == MAP_FAILED) {
        fprintf(stderr, "FAIL: mmap(MAP_SHARED|MAP_ANONYMOUS) for the shared "
                        "counters failed; running without crash isolation\n");
        g_shared = NULL;
    }

    static const Case CASES[] = {
        { "rust_wire_table_fully_reachable",
          test_rust_wire_table_fully_reachable, 0,
          "ENGINE BUG (executor.c): a Rust wire string lost its executor entry" },
        { "two_gap_actions_are_registered",
          test_two_gap_actions_are_registered, 0,
          "ENGINE BUG (executor.c): the fix is registered in executor_table" },
        { "gap_actions_resolve_without_mutating",
          test_gap_actions_resolve_without_mutating, 0,
          "ENGINE BUG (executor.c): the no-op arms must resolve Ok and change nothing" },
        { "gap_actions_log_one_verdict",
          test_gap_actions_log_one_verdict, 0,
          "ENGINE BUG (executor.c): the verdict push was skipped by the early return" },
        { "corpus_census_fully_reachable",
          test_corpus_census_fully_reachable, 0,
          "ENGINE BUG (executor.c): a corpus action is unreachable, so it is silently dropped" },

        { "action_type_from_str_knows_both",
          test_action_type_from_str_knows_both, 1,
          "ENUM GAP (include/rabuka.h, NOT owned by this audit): RbActionType "
          "needs RB_ACTION_REPLACEMENT_APPEAR_WAIT and RB_ACTION_COUNT_AS_MULTIPLE "
          "appended after RB_ACTION_ENERGY_CONDITION. Exact declaration in the "
          "final report; test_parity_char2.c's second #F assertion is red for "
          "the same reason" },

        { "change_state_both_targets_split_both_players",
          test_change_state_both_targets_split_both_players, 1,
          "ENGINE BUG (src/ability/effects/state.c s_who, and the missing "
          "generic pre-dispatch both-split in src/engine.c): Rust splits "
          "target=\"both\" in handle_both_targets (target.rs:119-145) for EVERY "
          "action; C only splits inside rb_execute_misc_effect and "
          "execute_move_cards_both_targets. Victims: PL!-bp5-111-P+, "
          "PL!HS-pb1-008-P+ (change_state), PL!S-bp7-004-P (select), "
          "PL!-bp3-022-L (modify_score per_unit)" },

        { "count_as_multiple_weights_the_zone_count",
          test_count_as_multiple_weights_the_zone_count, 1,
          "ENGINE BUG (src/core/, NOT owned by this audit): no "
          "weighted_zone_count / count_as_multiple_weight. The count seam is "
          "rb_zone_cards, which has no counting-card argument, so the weight "
          "cannot be applied even in principle" },

        { "replacement_appear_wait_holds_the_newcomer",
          test_replacement_appear_wait_holds_the_newcomer, 1,
          "ENGINE BUG (src/turn/ + src/core/, NOT owned by this audit): no "
          "replacement_appear_wait_applies predicate and no placement hook in "
          "rb_play_member (engine.c). Rust resolves it once at placement "
          "(phases.rs:1281), never as a constant" },
    };

    for (unsigned i = 0; i < sizeof CASES / sizeof CASES[0]; i++)
        run_case(&CASES[i]);

    int abnormal = g_shared ? g_shared->cases : 0;
    printf("checks=%d failures=%d cases_needing_attention=%d "
           "(green=%d red_pinned=%d)\n",
           assertions, failures, abnormal,
           (int)(sizeof CASES / sizeof CASES[0]) - 4, 4);
    return failures ? 1 : 0;
}