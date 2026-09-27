#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

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

#define CHECK_EQ_STR(actual, expected, message) do { \
    const char *actual_value = (actual); \
    const char *expected_value = (expected); \
    if (strcmp(actual_value, expected_value) != 0) { \
        fprintf(stderr, "FAIL: %s (got %s expected %s)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

static void process_abilities(TestGame *tg)
{
    rb_trigger_live_start(&tg->state, 0);
    rb_process_pending_auto_abilities(&tg->state);
    while (test_has_pending_choice(tg)) {
        int indices[] = {0};
        test_select_indices(tg, indices, 1);
        rb_process_pending_auto_abilities(&tg->state);
    }
}

static void setup_live_phase_with_hearts(TestGame *tg)
{
    tg->state.phase = RB_PHASE_LIVE_SET;
    tg->state.stage_hearts[0][0] = 7;
    tg->state.stage_hearts[0][1] = 2;
    tg->state.stage_hearts[0][5] = 6;
    tg->state.stage_hearts[0][7] = 10;
}

static void target_count_1_gain_resource_chooses_one_of_many(void)
{
    TestGame tg;
    test_game_new(&tg);
    int member_a = test_id(&tg, "PL!N-PR-003-PR");
    int member_b = test_id(&tg, "PL!N-PR-005-PR");
    int phoenix = test_id(&tg, "PL!N-pb1-038-L");
    int stellar = test_id(&tg, "PL!N-pb1-039-L");
    tg.state.p[0].stage[0] = -1;
    tg.state.p[0].stage[1] = -1;
    tg.state.p[0].stage[2] = -1;
    tg.state.p[0].stage[1] = member_a;
    tg.state.p[0].stage[0] = member_b;
    test_add_to_live(&tg, phoenix);
    test_add_to_live(&tg, stellar);
    setup_live_phase_with_hearts(&tg);
    process_abilities(&tg);
    int a_heart06 = test_get_heart_modifier(&tg, member_a, 5);
    int b_heart06 = test_get_heart_modifier(&tg, member_b, 5);
    CHECK_EQ(a_heart06 + b_heart06, 4,
             "Total +4 heart06 across both members");
    CHECK(a_heart06 == 4 || b_heart06 == 4,
          "Exactly one member got the full +4 heart06 buff");
}

/* Port of engine/tests/test_modules/rules/targeting/target_selection_test.rs:67-97
   (target_selection_distinguishes_repeated_card_instances). The same card number
   appears twice on stage; picking option 1 must buff the RIGHT-side instance only. */
static void target_selection_distinguishes_repeated_card_instances(void)
{
    TestGame tg;
    test_game_new(&tg);
    int target_a = test_id(&tg, "PL!N-PR-003-PR");
    int target_b = test_new_id(&tg, "PL!N-PR-003-PR");
    int stellar = test_id(&tg, "PL!N-pb1-039-L");
    tg.state.p[0].stage[0] = target_a;
    tg.state.p[0].stage[1] = RB_EMPTY_SLOT;
    tg.state.p[0].stage[2] = target_b;
    test_add_to_live(&tg, stellar);
    setup_live_phase_with_hearts(&tg);

    rb_trigger_live_start(&tg.state, 0);
    rb_process_pending_auto_abilities(&tg.state);
    CHECK(test_has_pending_choice(&tg), "target-selection prompt expected");
    int pick[] = {1};
    test_select_indices(&tg, pick, 1);
    rb_process_pending_auto_abilities(&tg.state);

    CHECK_EQ(test_get_heart_modifier(&tg, target_a, RB_HEART_PURPLE), 0,
             "unpicked repeated instance gets no heart06");
    CHECK_EQ(test_get_heart_modifier(&tg, target_b, RB_HEART_PURPLE), 4,
             "picked repeated instance gets the full +4 heart06");
}

static void distinct_card_name_prevents_same_card_twice(void)
{
    TestGame tg;
    test_game_new(&tg);
    int live_card = test_id(&tg, "PL!SP-bp1-026-L");
    test_add_to_live(&tg, live_card);
    tg.state.stage_hearts[0][0] = 5;
    tg.state.stage_hearts[0][7] = 5;
    tg.state.phase = RB_PHASE_LIVE_SET;
    process_abilities(&tg);
    int h02 = rb_mods_get_need_heart(&tg.state.mods, live_card, 1);
    if (h02 != 0) {
        fprintf(stderr,
                "FAIL: condition unmet: required hearts must be unchanged, got modifier %d\n",
                h02);
        failures++;
    } else {
        printf("ok: condition unmet: required hearts must be unchanged, got modifier 0\n");
    }
}

static void target_count_on_draw_until_count(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card = test_id(&tg, "PL!N-PR-028-PR");
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    tg.state.p[0].hand.n = 0;
    test_add_to_hand(&tg, card);
    test_add_to_hand(&tg, filler);
    test_add_to_hand(&tg, filler);
    test_give_energy(&tg, 30);
    for (int i = 0; i < 10; i++) {
        test_add_to_deck(&tg, filler);
    }
    test_play_to_stage(&tg, card, 1);
    CHECK(test_has_pending_choice(&tg), "optional discard-2 cost prompt expected");
    CHECK_EQ_STR(test_pending_choice_type(&tg), "SelectCard",
                 "expected SelectCard (hand, count=2, allow_skip)");
    rb_resume_with_choice(&tg.state, 0);
    rb_drain_ability_queue(&tg.state);
    if (test_has_pending_choice(&tg)) {
        rb_resume_with_choice(&tg.state, 1);
        rb_drain_ability_queue(&tg.state);
    }
    while (test_has_pending_choice(&tg)) {
        rb_resume_with_choice(&tg.state, -1);
    }
    CHECK_EQ(tg.state.p[0].hand.n, 5,
             "draw_until_count must fill the hand to target_count=5");
}

/* ==========================================================================
   Wave-2 canary status: "target selection +4".

   The four checks above are engine/tests/test_modules/rules/targeting/
   target_selection_test.rs transposed verbatim and they are GREEN on master
   unmodified: the +4 delta the canary reported is gone. The prompt it came
   from is the target_count=1 stage picker, and the [STAGE_TARGET_RESULT]
   pick -> slot -> cid chain now resolves to the picked instance, so exactly
   one member of a many-member stage takes the full +4 heart06.

   What follows covers the rest of the target-selection / executor-dispatch
   surface owned by src/ability/resolver.c and src/ability/effects/executor.c.
   ========================================================================== */

/* Defined in src/ability/resolver.c but not yet declared in rabuka.h. */
int rb_distinct_stage_groups(const GameState *g, int pl);
int rb_resolver_cached_condition_verdict(const GameState *g, int actor,
                                         const char *cond_text, int *result);
void rb_resolver_store_condition_verdict(GameState *g, int actor,
                                         const char *cond_text, int result);
void rb_resolver_drain_verdicts(GameState *g);
void rb_resolver_drain_verdicts_since(GameState *g, int snapshot);
void rb_resolver_push_verdict(GameState *g, const char *text, const char *kind, int passed);

static void set_extra(AbilityEffect *e, const char *k, const char *v)
{
    if (e->n_extra >= RB_MAX_EXTRA) return;
    e->extra_k[e->n_extra] = (char *)k;
    e->extra_v[e->n_extra] = (char *)v;
    e->n_extra++;
}

/* First member card in the database that belongs to `group`.
   rb_card_matches_group_str derives the group from the series, exactly as
   core/card.rs does, so this stays correct if card numbers move. */
static int first_member_of_group(const char *group)
{
    uint32_t n = rb_num_cards();
    for (uint32_t i = 0; i < n; i++) {
        if (!rb_card_is_member((int)i)) continue;
        if (rb_card_matches_group_str((int)i, group)) return (int)i;
    }
    return -1;
}

/* Two members from different groups, so the per-group cost discount has a
   non-zero unit count to divide by. */
static int stage_two_groups(TestGame *tg)
{
    int liella = first_member_of_group("Liella!");
    int aqours = first_member_of_group("Aqours");
    if (liella < 0) liella = test_id(tg, "PL!N-PR-003-PR");
    if (aqours < 0) aqours = test_id(tg, "PL!N-PR-005-PR");
    test_add_to_stage(tg, 1, liella);
    test_add_to_stage(tg, 2, aqours);
    return rb_distinct_stage_groups(&tg->state, 0);
}

/* resolver.rs:872-910 pay_ability_cost + 1168-1185 apply_modify_cost_to_ability_cost
   + 1196-1218 apply_per_unit_cost_reduction.
   A per-group discount folds into `energy_count`, which is the field rb_pay_cost
   actually charges for pay_energy (cost.c:828) - never into `count`, which a
   pay_energy cost may also carry. Control run first: same board, no modify_cost. */
static void modify_cost_per_group_discounts_the_energy_cost(void)
{
    TestGame tg;
    test_game_new(&tg);
    int groups = stage_two_groups(&tg);
    CHECK(groups >= 1, "at least one distinct group is on stage for the discount");

    AbilityEffect cost; memset(&cost, 0, sizeof(cost));
    cost.action = (char *)"pay_energy";
    cost.count = 6;                        /* a pay_energy cost may carry both fields */
    cost.target = (char *)"self";
    set_extra(&cost, "energy_count", "6"); /* the field rb_pay_cost charges */

    AbilityEffect mod; memset(&mod, 0, sizeof(mod));
    mod.action = (char *)"modify_cost";
    mod.count = 1;                         /* 1 energy per group */
    set_extra(&mod, "operation", "subtract");
    set_extra(&mod, "per_unit", "true");
    set_extra(&mod, "per_unit_type", "group_name");

    /* Control: no modify_cost in the tree, so the printed 6 is charged. */
    Ability plain; memset(&plain, 0, sizeof(plain));
    plain.use_limit = -1;
    plain.cost = &cost;
    plain.effect = &mod;
    plain.effect = NULL;

    test_give_energy(&tg, 20);
    int before = tg.state.p[0].energy_active;
    int resolved = 0;
    rb_resolve_ability(&tg.state, 0, &plain, 0, -1, &resolved);
    int control_paid = before - tg.state.p[0].energy_active;
    CHECK_EQ(control_paid, 6, "control: without modify_cost the full 6 energy is charged");

    Ability ab; memset(&ab, 0, sizeof(ab));
    ab.use_limit = -1;
    ab.cost = &cost;
    ab.effect = &mod;

    before = tg.state.p[0].energy_active;
    rb_resolve_ability(&tg.state, 0, &ab, 0, -1, &resolved);
    int paid = before - tg.state.p[0].energy_active;

    int expected = 6 - groups;
    if (expected < 0) expected = 0;
    CHECK(groups < 6, "the discount is strictly positive on this board");
    CHECK_EQ(paid, expected,
             "the per-group discount lands on energy_count (the charged field), not count");
}

/* resolver.rs:1205-1218 - the group_name branch returns early for any cost whose
   action is not PayEnergy, so a move_cards cost keeps its printed count. */
static void modify_cost_per_group_ignores_a_move_cards_cost(void)
{
    TestGame tg;
    test_game_new(&tg);
    stage_two_groups(&tg);
    int filler = test_id(&tg, "PL!-sd1-010-SD");
    for (int i = 0; i < 4; i++) test_add_to_hand(&tg, filler);

    AbilityEffect cost; memset(&cost, 0, sizeof(cost));
    cost.action = (char *)"move_cards";
    cost.count = 3;
    cost.source = (char *)"hand";
    cost.destination = (char *)"waitroom";
    cost.target = (char *)"self";

    AbilityEffect mod; memset(&mod, 0, sizeof(mod));
    mod.action = (char *)"modify_cost";
    mod.count = 1;
    set_extra(&mod, "operation", "subtract");
    set_extra(&mod, "per_unit", "true");
    set_extra(&mod, "per_unit_type", "group_name");

    Ability ab; memset(&ab, 0, sizeof(ab));
    ab.use_limit = -1;
    ab.cost = &cost;
    ab.effect = &mod;

    int resolved = 0;
    rb_resolve_ability(&tg.state, 0, &ab, 0, -1, &resolved);
    CHECK(test_has_pending_choice(&tg), "the mandatory hand-discard cost opens a choice");
    CHECK_EQ(tg.state.queue.pending.count, 3,
             "a per-group discount never shrinks a non-energy cost (still discard 3)");
}

/* resolver.rs:333-361 cached_condition_verdict / store_condition_verdict.
   Both string-keyed entry points used to be `(void)` no-ops, so a stored verdict
   could never be read back. */
static void condition_verdict_cache_round_trips(void)
{
    TestGame tg;
    test_game_new(&tg);
    int card = test_id(&tg, "PL!N-PR-028-PR");
    int entry = rb_queue_make_entry(&tg.state, card, 0);
    CHECK(entry >= 0, "a queue entry exists to host the condition cache");
    tg.state.queue.cur = entry;

    int cached = 12345;
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "heart_count>=3", &cached),
             0, "an unknown condition text is a cache miss");
    CHECK_EQ(cached, 0, "a cache miss leaves the out parameter at 0");

    rb_resolver_store_condition_verdict(&tg.state, 0, "heart_count>=3", 1);
    cached = 0;
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "heart_count>=3", &cached),
             1, "a stored verdict is found again");
    CHECK_EQ(cached, 1, "the stored verdict value is returned");

    rb_resolver_store_condition_verdict(&tg.state, 0, "heart_count>=3", 0);
    cached = 1;
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "heart_count>=3", &cached),
             1, "re-storing replaces the previous verdict for the same key");
    CHECK_EQ(cached, 0, "the replaced verdict is the failing one");

    cached = 1;
    CHECK_EQ(rb_resolver_cached_condition_verdict(&tg.state, 0, "heart_count>=4", &cached),
             0, "a different condition text is a separate key");
}

/* log.rs:19-27 drain_verdicts / push_verdict. These entry points were `(void)`
   no-ops, so a buffered verdict could never be pushed-and-drained. */
static void verdict_buffer_drains(void)
{
    TestGame tg;
    test_game_new(&tg);
    rb_log_set_enabled(1);
    rb_log_clear_verdicts();
    rb_resolver_push_verdict(&tg.state, "text", "kind", 1);
    rb_log_push_verdict("more", "kind", 0);
    CHECK_EQ(rb_log_buffer_len(), 2, "two verdicts are buffered");
    rb_resolver_drain_verdicts(&tg.state);
    CHECK_EQ(rb_log_buffer_len(), 0, "rb_resolver_drain_verdicts empties the buffer");
    rb_log_set_enabled(0);
}

/* executor.rs:47-190 - the match is exhaustive over ActionType, so every Rust arm
   must be reachable from rb_executor_execute. */
static void executor_dispatch_table_is_complete(void)
{
    static const char *const rust_actions[] = {
        "draw_card", "draw_until_count", "move_cards", "discard_card", "select",
        "select_cards", "look_and_select", "look_at", "reveal", "reveal_per_group",
        "reveal_until_live_card", "reveal_until_chosen_card", "change_state",
        "position_change", "rotation", "place_energy_under_member", "set_card_identity",
        "modify_required_hearts_success", "gain_resource", "pay_energy", "gain_ability",
        "gain_ability_from_source", "invalidate_ability", "suppress_ability_trigger",
        "activate_ability", "modify_cost", "modify_yell_source", "set_cost",
        "set_cost_to_use", "modify_score", "modify_required_hearts", "set_blade_type",
        "set_blade_count", "set_heart_type", "specify_heart_color", "choose_required_hearts",
        "sequential", "conditional_alternative", "conditional_on_result",
        "conditional_on_optional", "restriction", "activation_restriction", "modify_limit",
        "shuffle", "re_yell", "custom", "do_nothing", "choice", "repeat_procedure",
        "discard_until_count", "all_blade_timing", "reduce_live_card_set_limit",
        "choose_target_player", "select_number", "play_baton_touch",
        "modify_required_hearts_global", "modify_yell_count", "activation_cost",
        "perform_yell", "conditional_optional", "compound_action", "opponent_action",
        "action_by", "sequential_cost", "choice_condition", "energy_condition"
    };
    int n = (int)(sizeof(rust_actions) / sizeof(rust_actions[0]));
    int missing = 0;
    for (int i = 0; i < n; i++)
        if (!rb_executor_has_executor(rust_actions[i])) {
            fprintf(stderr, "FAIL: executor table is missing the Rust arm %s\n",
                    rust_actions[i]);
            missing++;
        }
    CHECK_EQ(missing, 0, "every ActionType arm of executor.rs is dispatchable");
    CHECK(!rb_executor_has_executor("no_such_action"),
          "a string that never decoded as an action has no executor");
}

/* Run one effect through the dispatcher and return the number of EFFECT-kind
   verdict entries it produced. A handler may push cost/condition verdicts of its
   own; push_effect_verdict owns only the effect entry. */
static int effect_verdicts_for(TestGame *tg, AbilityEffect *e, RbAbilityLogItem *out)
{
    rb_log_clear_verdicts();
    rb_executor_execute(&tg->state, 0, e, -1);
    RbAbilityLogItem items[16];
    int n = rb_log_drain_verdicts(items, 16);
    int found = 0;
    for (int i = 0; i < n; i++) {
        if (items[i].kind == RB_LOG_KIND_EFFECT && out && found < 8) out[found++] = items[i];
        else rb_log_free_item(&items[i]);
    }
    return found;
}

/* executor.rs:9-40 push_effect_verdict - the matches! structural set is
   {CompoundAction, Sequential, Choice, ConditionalAlternative,
   ConditionalOnResult, ConditionalOnOptional} and ONLY that set suppresses the
   verdict entry (conditional_optional is deliberately NOT in it). */
static void verdict_suppression_matches_the_rust_set(void)
{
    TestGame tg;
    test_game_new(&tg);
    rb_log_set_enabled(1);
    static const char *const suppressed[] = {
        "compound_action", "sequential", "choice", "conditional_alternative",
        "conditional_on_result", "conditional_on_optional"
    };
    static const char *const logged[] = {
        "conditional_optional", "do_nothing", "action_by", "repeat_procedure",
        "opponent_action", "gain_resource", "modify_score", "reveal"
    };
    int bad = 0;
    RbAbilityLogItem items[8];
    for (int i = 0; i < (int)(sizeof(suppressed) / sizeof(suppressed[0])); i++) {
        AbilityEffect e; memset(&e, 0, sizeof(e));
        e.action = (char *)suppressed[i];
        int n = effect_verdicts_for(&tg, &e, items);
        for (int k = 0; k < n; k++) rb_log_free_item(&items[k]);
        if (n != 0) {
            fprintf(stderr, "FAIL: %s must not push a verdict entry (pushed %d)\n",
                    suppressed[i], n);
            bad++;
        }
    }
    for (int i = 0; i < (int)(sizeof(logged) / sizeof(logged[0])); i++) {
        AbilityEffect e; memset(&e, 0, sizeof(e));
        e.action = (char *)logged[i];
        int n = effect_verdicts_for(&tg, &e, items);
        for (int k = 0; k < n; k++) rb_log_free_item(&items[k]);
        if (n != 1) {
            fprintf(stderr, "FAIL: %s must push exactly one verdict entry (pushed %d)\n",
                    logged[i], n);
            bad++;
        }
    }
    rb_log_set_enabled(0);
    CHECK_EQ(bad, 0,
             "push_effect_verdict suppresses exactly the six Rust structural actions");
}

/* executor.rs:22-33 - details is "action value" when the effect carries a
   count/value and the bare action label when it carries neither. */
static void verdict_details_use_count_then_value(void)
{
    TestGame tg;
    test_game_new(&tg);
    rb_log_set_enabled(1);
    RbAbilityLogItem items[8];

    AbilityEffect e; memset(&e, 0, sizeof(e));
    e.action = (char *)"gain_resource";
    e.count = 4;
    int n = effect_verdicts_for(&tg, &e, items);
    CHECK_EQ(n, 1, "a counted effect pushes one verdict entry");
    if (n == 1) {
        CHECK_EQ_STR(items[0].as.effect.details, "gain_resource 4",
                     "count renders as action + count");
        rb_log_free_item(&items[0]);
    }

    AbilityEffect v; memset(&v, 0, sizeof(v));
    v.action = (char *)"modify_score";
    v.count = -1;
    set_extra(&v, "value", "2");
    n = effect_verdicts_for(&tg, &v, items);
    CHECK_EQ(n, 1, "a value-only effect pushes one verdict entry");
    if (n == 1) {
        CHECK_EQ_STR(items[0].as.effect.details, "modify_score 2",
                     "value_any() fills in when count is absent");
        rb_log_free_item(&items[0]);
    }

    AbilityEffect b; memset(&b, 0, sizeof(b));
    b.action = (char *)"reveal";
    b.count = -1;
    n = effect_verdicts_for(&tg, &b, items);
    CHECK_EQ(n, 1, "a bare effect pushes one verdict entry");
    if (n == 1) {
        CHECK_EQ_STR(items[0].as.effect.details, "reveal",
                     "no count and no value renders the bare action label");
        rb_log_free_item(&items[0]);
    }
    rb_log_set_enabled(0);
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    target_count_1_gain_resource_chooses_one_of_many();
    target_selection_distinguishes_repeated_card_instances();
    distinct_card_name_prevents_same_card_twice();
    target_count_on_draw_until_count();
    modify_cost_per_group_discounts_the_energy_cost();
    modify_cost_per_group_ignores_a_move_cards_cost();
    condition_verdict_cache_round_trips();
    verdict_buffer_drains();
    executor_dispatch_table_is_complete();
    verdict_suppression_matches_the_rust_set();
    verdict_details_use_count_then_value();
    rb_unload();
    if (failures) return 1;
    printf("ALL TARGET SELECTION CHECKS PASSED\n");
    return 0;
}
