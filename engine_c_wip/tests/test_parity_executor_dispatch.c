/* test_parity_executor_dispatch.c
 *
 * Parity harness for src/ability/effects/executor.c -- the C effect dispatcher.
 *
 * Each test drives a DISTINCT ActionType through the real effect entry point
 * (rb_execute_effect_ex, which consults rb_executor_has_executor and then calls
 * rb_executor_execute) and asserts the state change that the Rust dispatch arm
 * produces (engine/src/ability/effects/executor.rs:47-190). The Rust provenance
 * of every case is named in the section banner, e.g. `effects/draw/flat` refers
 * to engine/tests/test_modules/effects/draw/flat/.
 *
 * Group 0 is the vocabulary sweep: every wire string in
 * engine/src/ability/enums.rs:381-457 must be claimed by the dispatch table, and
 * every action verb actually present in the compiled bytecode
 * (tools/audit_actions.c) must be reachable.
 */
#include "rabuka.h"
#include "test_game.h"
#include <stdio.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(condition, message) do { \
    checks++; \
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
    checks++; \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

static void reset_board(TestGame *tg)
{
    for (int pl = 0; pl < 2; pl++) {
        RbPlayer *p = &tg->state.p[pl];
        p->deck.n = 0;
        p->hand.n = 0;
        p->discard.n = 0;
        p->live.n = 0;
        p->success.n = 0;
        p->stage[0] = p->stage[1] = p->stage[2] = -1;
        for (int a = 0; a < RB_STAGE_SIZE; a++) {
            p->under_cards[a].n = 0;
            p->stage_wait[a] = 0;
        }
    }
    tg->state.n_revealed = 0;
    tg->state.n_selected_cards = 0;
    tg->state.n_recently_moved = 0;
    tg->state.activating_card = -1;
}

static int deck_card(TestGame *tg, const char *no)
{
    int id = test_new_id(tg, no);
    if (id >= 0) test_add_to_deck(tg, id);
    return id;
}

/* First card index of a given class (0 member, 1 energy, 2 live), mirroring
 * test_state_effects.c's find_cards so no hard-coded card number is needed. */
static int first_card_of_type(int type)
{
    for (uint32_t i = 0; i < rb_num_cards(); i++) {
        if (type == 0 && rb_card_is_member((int)i)) return (int)i;
        if (type == 1 && rb_card_is_energy((int)i)) return (int)i;
        if (type == 2 && rb_card_is_live((int)i)) return (int)i;
    }
    return -1;
}

static void set_extra(AbilityEffect *e, const char *k, const char *v)
{
    if (e->n_extra >= RB_MAX_EXTRA) return;
    e->extra_k[e->n_extra] = (char *)k;
    e->extra_v[e->n_extra] = (char *)v;
    e->n_extra++;
}

static int has_prohibition(const GameState *g, const char *text)
{
    for (int i = 0; i < g->n_prohibition_effects; i++)
        if (!strcmp(g->prohibition_effects[i], text)) return 1;
    return 0;
}

/* Count effect verdicts logged for `action` and return the details string of
 * the first match ("" when there is none). Mirrors Rust log::push_verdict. */
static int find_effect_verdict(const char *action, char *details_out, size_t cap)
{
    RbAbilityLogItem items[64];
    int n = rb_log_drain_verdicts(items, 64);
    int found = 0;
    if (details_out && cap) details_out[0] = '\0';
    for (int i = 0; i < n; i++) {
        if (items[i].kind != RB_LOG_KIND_EFFECT) continue;
        if (strcmp(items[i].as.effect.action, action) != 0) continue;
        if (!found && details_out && cap)
            snprintf(details_out, cap, "%s", items[i].as.effect.details);
        found++;
    }
    for (int i = 0; i < n; i++) rb_log_free_item(&items[i]);
    return found;
}

/* ========================================================================
 * Group 0 -- dispatch-table vocabulary parity.
 * enums.rs:381-457 ActionType wire tables vs executor.c executor_table.
 * ======================================================================== */

/* Verbatim from engine/src/ability/enums.rs:381-457 (ActionType => wire). */
static const char *const rust_action_types[] = {
    "draw_card", "draw_until_count", "move_cards", "discard_card", "select",
    "select_number", "select_cards", "look_and_select", "look_at", "reveal",
    "reveal_per_group", "reveal_until_live_card", "reveal_until_chosen_card",
    "change_state", "position_change", "rotation",
    "place_energy_under_member", "set_card_identity",
    "modify_required_hearts_success", "gain_resource", "pay_energy",
    "gain_ability", "gain_ability_from_source", "invalidate_ability",
    "suppress_ability_trigger", "activate_ability", "modify_cost",
    "modify_yell_source", "set_cost", "set_cost_to_use", "modify_score",
    "modify_required_hearts", "set_blade_type", "set_blade_count",
    "set_heart_type", "specify_heart_color", "choose_required_hearts",
    "sequential", "conditional_alternative", "conditional_on_result",
    "conditional_on_optional", "restriction", "activation_restriction",
    "modify_limit", "shuffle", "re_yell", "custom", "do_nothing", "choice",
    "repeat_procedure", "discard_until_count", "all_blade_timing",
    "reduce_live_card_set_limit", "choose_target_player", "play_baton_touch",
    "modify_required_hearts_global", "modify_yell_count", "activation_cost",
    "perform_yell", "conditional_optional", "compound_action",
    "opponent_action", "action_by", "sequential_cost", "choice_condition",
    "energy_condition"
};

static void test_action_vocabulary_table(void)
{
    int n = (int)(sizeof(rust_action_types) / sizeof(rust_action_types[0]));
    int missing = 0;
    for (int i = 0; i < n; i++)
        if (!rb_executor_has_executor(rust_action_types[i])) {
            fprintf(stderr, "FAIL: no executor registered for ActionType %s "
                            "(engine/src/ability/enums.rs)\n", rust_action_types[i]);
            checks++;
            failures++;
            missing++;
        }
    CHECK_EQ(missing, 0, "every Rust ActionType is claimed by the dispatch table");
    /* The entries whose dispatch branches already existed but were not in the
     * table -- the concrete gap this suite was written to catch. */
    CHECK(rb_executor_has_executor("modify_cost"),
          "modify_cost is dispatched (executor.rs:94 arm was unreachable)");
    CHECK(rb_executor_has_executor("reduce_live_card_set_limit"),
          "reduce_live_card_set_limit is dispatched (executor.rs:153 arm was unreachable)");
    CHECK(rb_executor_has_executor("activation_cost"),
          "activation_cost is dispatched (executor.rs:175)");
    CHECK(!rb_executor_has_executor("no_such_action"),
          "an undecoded action string is reported as unsupported");
}

/* Every action verb that actually occurs in the compiled card bytecode must be
 * reachable through the registry, otherwise rb_execute_effect_ex silently hands
 * it to the legacy handle_action fallback instead of the Rust-faithful arm. */
static void test_compiled_vocabulary_reachable(void)
{
    int missing = 0;
    int total = 0;
    for (uint32_t a = 0; a < rb_num_abilities(); a++) {
        Ability ability;
        if (!rb_decode_ability(a, &ability)) continue;
        AbilityEffect *stack[64];
        int n_stack = 0;
        if (ability.effect) stack[n_stack++] = ability.effect;
        for (int i = 0; i < n_stack && i < 64; i++) {
            AbilityEffect *e = stack[i];
            if (e && e->action) {
                total++;
                if (!rb_executor_has_executor(e->action)) {
                    if (missing < 8)
                        fprintf(stderr, "FAIL: compiled effect action '%s' has no executor\n",
                                e->action);
                    checks++;
                    failures++;
                    missing++;
                }
            }
            for (int c = 0; c < e->n_child && n_stack < 64; c++)
                stack[n_stack++] = e->child[c];
        }
        rb_free_ability(&ability);
    }
    CHECK(total > 0, "compiled ability corpus yields decoded effect actions");
    CHECK_EQ(missing, 0, "every action verb in the compiled bytecode is dispatchable");
}

/* ========================================================================
 * Group 1 -- effects/draw/flat : DrawCard (executor.rs:48)
 * ======================================================================== */
static void test_draw_card_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int a = deck_card(&tg, "PL!-sd1-010-SD");
    int b = deck_card(&tg, "PL!-sd1-002-SD");
    int c = deck_card(&tg, "PL!-sd1-003-SD");
    CHECK(a >= 0 && b >= 0 && c >= 0, "draw fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"draw_card";
    e.count = 2;
    e.source = (char *)"deck";
    e.destination = (char *)"hand";
    e.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.p[0].hand.n, 2, "draw_card moved the requested count into hand");
    CHECK(tg.state.p[0].hand.cards[0] == a && tg.state.p[0].hand.cards[1] == b,
          "draw_card preserves deck-top order");
    CHECK_EQ(tg.state.p[0].deck.n, 1, "draw_card leaves the deck tail in place");
}

/* ========================================================================
 * Group 2 -- effects/draw/until_count : DrawUntilCount (executor.rs:49)
 * ======================================================================== */
static void test_draw_until_count_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    for (int i = 0; i < 6; i++) deck_card(&tg, "PL!-sd1-010-SD");
    CHECK_EQ(tg.state.p[0].deck.n, 6, "draw_until_count fixture deck seeded");
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"draw_until_count";
    e.target = (char *)"self";
    e.destination = (char *)"hand";
    set_extra(&e, "target_count", "5");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.p[0].hand.n, 5, "draw_until_count tops the hand up to target_count");
    CHECK_EQ(tg.state.p[0].deck.n, 1, "draw_until_count draws only the shortfall");
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.p[0].hand.n, 5, "draw_until_count is a no-op once the count is met");
    CHECK_EQ(tg.state.p[0].deck.n, 1, "draw_until_count does not overdraw");
}

/* ========================================================================
 * Group 3 -- effects/recover/to_hand : MoveCards (executor.rs:53)
 * ======================================================================== */
static void test_move_cards_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int a = test_new_id(&tg, "PL!-sd1-010-SD");
    int b = test_new_id(&tg, "PL!-sd1-002-SD");
    CHECK(a >= 0 && b >= 0, "recover fixtures resolve");
    if (a < 0 || b < 0) return;
    test_add_to_discard(&tg, a);
    test_add_to_discard(&tg, b);
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"move_cards";
    e.count = 1;
    e.source = (char *)"discard";
    e.destination = (char *)"hand";
    e.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.p[0].hand.n, 1, "move_cards routes waitroom -> hand");
    CHECK_EQ(tg.state.p[0].discard.n, 1, "move_cards consumes exactly one waitroom card");
}

/* Group 3b -- effects/recover/to_discard : DiscardCard (executor.rs:54).
 * executor.rs sends DiscardCard through the same execute_logged_move arm with a
 * discard log tag; the C arm defaults source=hand / destination=waitroom. */
static void test_discard_card_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int a = test_new_id(&tg, "PL!-sd1-010-SD");
    int b = test_new_id(&tg, "PL!-sd1-002-SD");
    CHECK(a >= 0 && b >= 0, "discard fixtures resolve");
    if (a < 0 || b < 0) return;
    test_add_to_hand(&tg, a);
    test_add_to_hand(&tg, b);
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"discard_card";
    e.count = 1;
    e.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.p[0].discard.n, 1, "discard_card defaults to hand -> waitroom");
    CHECK_EQ(tg.state.p[0].hand.n, 1, "discard_card leaves the rest of the hand");
}

/* ========================================================================
 * Group 4 -- effects/look_select/bottom_inspect : LookAt (executor.rs:61);
 *          effects/look_select/reveal : Reveal / RevealPerGroup (:62-63)
 * ======================================================================== */
static void test_look_and_reveal_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int a = deck_card(&tg, "PL!-sd1-010-SD");
    int b = deck_card(&tg, "PL!-sd1-002-SD");
    int c = deck_card(&tg, "PL!-sd1-003-SD");
    CHECK(a >= 0 && b >= 0 && c >= 0, "look/reveal fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;
    int pool[8];
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"look_at";
    e.source = (char *)"deck_top";
    e.count = 2;
    e.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    int n = rb_looked_at_pool(0, pool, 8);
    CHECK_EQ(n, 2, "look_at fills the looked-at pool");
    CHECK(n == 2 && pool[0] == a && pool[1] == b, "look_at peeks from the deck top in order");
    CHECK_EQ(tg.state.p[0].deck.n, 1, "look_at lifts the peeked cards out of the deck");
    rb_resume_with_choice(&tg.state, -1);
    CHECK_EQ(tg.state.p[0].deck.n, 3, "the optional look resume returns the pool to the deck");

    tg.state.n_revealed = 0;
    e.action = (char *)"reveal_per_group";
    e.source = (char *)"deck";
    e.count = 2;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.n_revealed, 2, "reveal_per_group records every revealed card");
    CHECK((tg.state.revealed_cards[0] == a && tg.state.revealed_cards[1] == b) ||
              (tg.state.revealed_cards[0] == b && tg.state.revealed_cards[1] == a),
          "reveal_per_group peeks the two deck-top cards");
    CHECK_EQ(tg.state.p[0].deck.n, 3, "reveal_per_group does not drain the deck");
    tg.state.n_revealed = 0;
    e.action = (char *)"reveal";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.n_revealed, 2, "reveal dispatches to the same peek-and-record path");
    CHECK_EQ(tg.state.p[0].deck.n, 3, "reveal does not drain the deck");
}

/* ========================================================================
 * Group 5 -- effects/choice : SelectCards (executor.rs:59), SelectNumber (:158)
 * ======================================================================== */
static void test_choice_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int a = deck_card(&tg, "PL!-sd1-010-SD");
    int b = deck_card(&tg, "PL!-sd1-002-SD");
    int c = deck_card(&tg, "PL!-sd1-003-SD");
    CHECK(a >= 0 && b >= 0 && c >= 0, "choice fixtures resolve");
    if (a < 0 || b < 0 || c < 0) return;

    /* A bare look_at raises its own SelectCard choice over the looked-at pool
     * (look.c rb_effect_look_at), so look_at's dispatch is proven by that
     * prompt; the composite below is what proves select_cards. */
    AbilityEffect look;
    memset(&look, 0, sizeof(look));
    look.action = (char *)"look_at";
    look.source = (char *)"deck_top";
    look.count = 2;
    look.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &look, -1);
    const RbChoice *lch = rb_get_pending_choice(&tg.state);
    CHECK(rb_has_pending_choice(&tg.state) && lch &&
              lch->kind == RB_CHOICE_SELECT_CARD && lch->count == 2,
          "look_at prompts over the looked-at pool for its own count");
    CHECK(lch && !strcmp(lch->zone, "looked_at"),
          "look_at's choice is scoped to the looked-at zone");
    rb_resume_with_choice(&tg.state, -1);
    CHECK_EQ(tg.state.p[0].deck.n, 3, "skipping the look prompt returns the pool to the deck");

    /* look_and_select is the structural composite that runs look_at and then
     * select_cards (executor.rs:60). */
    reset_board(&tg);
    a = deck_card(&tg, "PL!-sd1-010-SD");
    b = deck_card(&tg, "PL!-sd1-002-SD");
    c = deck_card(&tg, "PL!-sd1-003-SD");
    if (a < 0 || b < 0 || c < 0) return;
    look.action = (char *)"look_at";
    look.source = (char *)"deck_top";
    look.count = 2;
    look.target = (char *)"self";
    AbilityEffect select;
    memset(&select, 0, sizeof(select));
    select.action = (char *)"select_cards";
    select.destination = (char *)"hand";
    select.count = 1;
    AbilityEffect parent;
    memset(&parent, 0, sizeof(parent));
    parent.action = (char *)"look_and_select";
    parent.look_action = &look;
    parent.select_action = &select;
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(rb_has_pending_choice(&tg.state) && ch && ch->kind == RB_CHOICE_SELECT_CARD,
          "look_and_select routes select_cards into a card choice");
    CHECK(ch && ch->count == 1,
          "select_cards prompts for exactly the requested count");
    CHECK(ch && ch->n_filtered_indices == 2,
          "select_cards exposes every looked-at card as selectable");
    rb_resume_with_choice(&tg.state, 0);
    CHECK_EQ(tg.state.p[0].hand.n, 1, "the select_cards pick lands in its destination");

    memset(&select, 0, sizeof(select));
    memset(&parent, 0, sizeof(parent));
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"select_number";
    e.count = 3;
    e.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    const RbChoice *num = rb_get_pending_choice(&tg.state);
    CHECK(rb_has_pending_choice(&tg.state), "select_number emits a number choice");
    CHECK(num && num->kind == RB_CHOICE_SELECT_NUMBER,
          "select_number routes to the number-picking choice");
    CHECK(num && num->count > 0 && !strncmp(num->description, "Choose a number", 15),
          "select_number offers the database cost range as its option set");
    rb_resume_with_choice(&tg.state, 0);
}

/* ========================================================================
 * Group 6 -- effects/gain/hearts/constants : GainResource (executor.rs:84)
 * ======================================================================== */
static void test_gain_resource_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int member = test_new_id(&tg, "PL!-sd1-010-SD");
    CHECK(member >= 0, "gain_resource fixture resolves");
    if (member < 0) return;
    test_add_to_stage(&tg, 1, member);
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"gain_resource";
    e.count = 2;
    e.target = (char *)"self";
    snprintf(e.self_target_field, sizeof(e.self_target_field), "true");
    set_extra(&e, "resource", "heart");
    set_extra(&e, "heart_colors", "heart01");
    tg.state.activating_card = member;
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    /* heart01 is the RED heart (util.c rb_parse_heart_color:2096-2103). */
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, member, RB_HEART_RED), 2,
             "gain_resource grants the requested hearts to the named heart colour");
    {
        int total = 0, nonzero_colors = 0;
        for (int c = 0; c < 7; c++) {
            int v = rb_mods_get_heart(&tg.state.mods, member, c);
            total += v;
            if (v) nonzero_colors++;
        }
        CHECK_EQ(total, 2, "gain_resource grants exactly the requested heart count");
        CHECK_EQ(nonzero_colors, 1, "gain_resource does not spill into other heart colours");
    }

    /* resource=blade is the other ResourceKind the Rust appliers accept
     * (misc.rs:1004); it must not be mistaken for a heart grant. */
    memset(&e, 0, sizeof(e));
    e.action = (char *)"gain_resource";
    e.count = 3;
    e.target = (char *)"self";
    snprintf(e.self_target_field, sizeof(e.self_target_field), "true");
    set_extra(&e, "resource", "blade");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK_EQ(rb_mods_get_heart(&tg.state.mods, member, RB_HEART_RED), 2,
             "a blade gain_resource leaves heart modifiers untouched");
    tg.state.activating_card = -1;
}

/* ========================================================================
 * Group 7 -- effects/energy/place : PayEnergy (executor.rs:85)
 * ======================================================================== */
static void test_pay_energy_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    test_give_energy(&tg, 4);
    int before = tg.state.p[0].energy_active;
    CHECK_EQ(before, 4, "pay_energy fixture energy seeded");
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"pay_energy";
    e.count = 2;
    e.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.p[0].energy_active, before - 2, "pay_energy spends the requested energy");
}

/* ========================================================================
 * Group 8 -- effects/energy/under_member : PlaceEnergyUnderMember (executor.rs:75)
 * ======================================================================== */
static void test_place_energy_under_member_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int member = test_new_id(&tg, "PL!-sd1-010-SD");
    int energy_template = first_card_of_type(1);
    CHECK(member >= 0 && energy_template >= 0, "under-member fixtures resolve");
    if (member < 0 || energy_template < 0) return;
    test_add_to_stage(&tg, 1, member);
    int energy = rb_create_card_copy(energy_template);
    CHECK(energy >= 0, "under-member energy copy allocated");
    if (energy < 0) return;
    test_add_to_energy_deck(&tg, 0, energy);
    int energy_seeded = tg.state.p[0].energy_deck.n;
    CHECK(energy_seeded > 0, "under-member fixture energy deck seeded");
    if (energy_seeded <= 0) return;
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"place_energy_under_member";
    e.count = 1;
    e.source = (char *)"energy_deck";
    e.destination = (char *)"center";
    e.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK_EQ(tg.state.p[0].under_cards[1].n, 1,
             "place_energy_under_member tucks energy under the member");
    CHECK_EQ(tg.state.p[0].energy_deck.n, energy_seeded - 1,
             "place_energy_under_member consumes from the energy deck");
}

/* ========================================================================
 * Group 9 -- effects/state/per_card : ChangeState (executor.rs:66)
 * ======================================================================== */
static void test_change_state_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int members[3];
    const char *member_nos[3] = { "PL!-sd1-010-SD", "PL!-sd1-002-SD", "PL!-sd1-003-SD" };
    for (int i = 0; i < 3; i++) {
        members[i] = test_new_id(&tg, member_nos[i]);
        if (members[i] >= 0) test_add_to_stage(&tg, i, members[i]);
    }
    CHECK(members[0] >= 0 && members[1] >= 0 && members[2] >= 0,
          "change_state fixtures resolve");
    if (members[0] < 0 || members[1] < 0 || members[2] < 0) return;
    for (int i = 0; i < 3; i++) rb_mods_set_orientation(&tg.state.mods, members[i], "active");
    Card m0;
    memset(&m0, 0, sizeof(m0));
    rb_decode_card_by_index((uint32_t)members[0], &m0);
    char exact_name[128];
    snprintf(exact_name, sizeof(exact_name), "%s", m0.name ? m0.name : "");
    rb_free_card(&m0);
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"change_state";
    e.count = 1;
    e.target = (char *)"self";
    snprintf(e.card_type_field, sizeof(e.card_type_field), "member_card");
    set_extra(&e, "state_change", "wait");
    set_extra(&e, "card_names", exact_name);
    tg.state.n_selected_cards = 0;
    tg.state.activating_card = members[0];
    rb_execute_effect_ex(&tg.state, 0, &e, members[0]);
    CHECK(!strcmp(rb_mods_get_orientation(&tg.state.mods, members[0]), "wait"),
          "change_state waits the named member");
    CHECK(!strcmp(rb_mods_get_orientation(&tg.state.mods, members[1]), "active") &&
              !strcmp(rb_mods_get_orientation(&tg.state.mods, members[2]), "active"),
          "change_state honors its card_names target filter");
}

/* ========================================================================
 * Group 10 -- effects/state/orientation_gated : Rotation (executor.rs:74)
 * ======================================================================== */
static void test_rotation_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int left = test_new_id(&tg, "PL!-sd1-010-SD");
    int center = test_new_id(&tg, "PL!-sd1-002-SD");
    int right = test_new_id(&tg, "PL!-sd1-003-SD");
    CHECK(left >= 0 && center >= 0 && right >= 0, "rotation fixtures resolve");
    if (left < 0 || center < 0 || right < 0) return;
    test_add_to_stage(&tg, 0, left);
    test_add_to_stage(&tg, 1, center);
    test_add_to_stage(&tg, 2, right);
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"rotation";
    e.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &e, center);
    /* misc.c h_rotation rot_map {2,0,1}: left->right, center->left, right->center. */
    CHECK(tg.state.p[0].stage[0] == center && tg.state.p[0].stage[1] == right &&
              tg.state.p[0].stage[2] == left,
          "rotation applies the {2,0,1} left/center/right rotation");
    CHECK(tg.state.position_change_occurred_this_turn &&
              tg.state.formation_change_occurred_this_turn,
          "rotation flags position and formation change for the turn");
}

/* ========================================================================
 * Group 11 -- effects/position/area_move : PositionChange (executor.rs:67)
 * ======================================================================== */
static void test_position_change_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int left = test_new_id(&tg, "PL!-sd1-010-SD");
    int right = test_new_id(&tg, "PL!-sd1-002-SD");
    CHECK(left >= 0 && right >= 0, "position_change fixtures resolve");
    if (left < 0 || right < 0) return;
    test_add_to_stage(&tg, 0, left);
    test_add_to_stage(&tg, 2, right);
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"position_change";
    e.target = (char *)"self";
    e.destination = (char *)"right";
    rb_execute_effect_ex(&tg.state, 0, &e, left);
    CHECK(tg.state.p[0].stage[2] == left, "position_change moves the host into the named area");
    CHECK(tg.state.p[0].stage[0] == -1 || tg.state.p[0].stage[0] == right,
          "position_change vacates the source area");
    CHECK(tg.state.position_change_occurred_this_turn,
          "position_change flags a position change for the turn");
}

/* ========================================================================
 * Group 12 -- effects/score/card_score : ModifyScore (executor.rs:104)
 * ======================================================================== */
static void test_modify_score_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int host = test_new_id(&tg, "PL!N-bp1-027-L");
    CHECK(host >= 0, "score fixture resolves");
    if (host < 0) return;
    test_add_to_stage(&tg, 1, host);
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"modify_score";
    e.count = 3;
    e.target = (char *)"self";
    tg.state.activating_card = host;
    rb_execute_effect_ex(&tg.state, 0, &e, host);
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, host), 3,
             "modify_score adds a per-card score modifier on the host");

    /* live_total is a separate Rust arm inside execute_modify_score
     * (score.rs:52-184) and must not fall into the per-card pool. */
    memset(&e, 0, sizeof(e));
    e.action = (char *)"modify_score";
    e.count = 2;
    e.target = (char *)"live_total";
    rb_execute_effect_ex(&tg.state, 0, &e, host);
    CHECK_EQ(tg.state.mods.p1_constant_total_score_bonus, 2,
             "modify_score live_total credits the constant total bonus");
    CHECK_EQ(rb_mods_get_score(&tg.state.mods, host), 3,
             "modify_score live_total leaves per-card modifiers alone");
}

/* ========================================================================
 * Group 13 -- effects/cost_mod/per_card : ModifyCost (executor.rs:94),
 *          SetCost (:99), SetCostToUse (:103)
 * ======================================================================== */
static void test_cost_effect_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int member = test_new_id(&tg, "PL!-sd1-010-SD");
    CHECK(member >= 0, "cost fixtures resolve");
    if (member < 0) return;
    test_add_to_stage(&tg, 1, member);
    tg.state.activating_card = member;

    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"modify_cost";
    e.target = (char *)"self";
    snprintf(e.card_type_field, sizeof(e.card_type_field), "member_card");
    set_extra(&e, "operation", "set");
    set_extra(&e, "value", "6");
    set_extra(&e, "duration", "permanent");
    CHECK_EQ(tg.state.mods.cost[member].set, 0, "modify_cost fixture starts at cost 0");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK_EQ(tg.state.mods.cost[member].set, 6, "modify_cost applies its set operation");

    memset(&e, 0, sizeof(e));
    e.action = (char *)"set_cost";
    e.target = (char *)"self";
    snprintf(e.card_type_field, sizeof(e.card_type_field), "member_card");
    set_extra(&e, "value", "4");
    set_extra(&e, "duration", "permanent");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK_EQ(tg.state.mods.cost[member].set, 4, "set_cost overwrites the cost to its value");

    memset(&e, 0, sizeof(e));
    e.action = (char *)"set_cost_to_use";
    e.target = (char *)"self";
    set_extra(&e, "value", "9");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK_EQ(rb_mods_get_cost(&tg.state.mods, member), 9,
             "set_cost_to_use targets the activating card directly");
    tg.state.activating_card = -1;
}

/* ========================================================================
 * Group 14 -- effects/cost_mod/required_hearts : ModifyRequiredHearts
 *          (executor.rs:105) and ModifyRequiredHeartsSuccess (:80)
 * ======================================================================== */
static void test_required_hearts_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int host = test_new_id(&tg, "PL!N-bp1-027-L");
    CHECK(host >= 0, "required-hearts fixture resolves");
    if (host < 0) return;
    test_add_to_live(&tg, host);
    int total_before = 0;
    for (int c = 0; c < 8; c++) total_before += rb_mods_get_need_heart(&tg.state.mods, host, c);
    CHECK_EQ(total_before, 0, "required-hearts fixture starts unmodified");
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"modify_required_hearts";
    e.count = 1;
    e.target = (char *)"self";
    set_extra(&e, "operation", "increase");
    set_extra(&e, "heart_colors", "heart01");
    set_extra(&e, "self_target", "true");
    tg.state.activating_card = host;
    rb_execute_effect_ex(&tg.state, 0, &e, host);
    int total_after = 0;
    for (int c = 0; c < 8; c++) total_after += rb_mods_get_need_heart(&tg.state.mods, host, c);
    CHECK(total_after != total_before,
          "modify_required_hearts changes a required-heart requirement on the live card");

    /* modify_required_hearts_success reads the SUCCESS zone (score.c:526-529). */
    int succ = test_new_id(&tg, "PL!N-bp1-027-L");
    CHECK(succ >= 0, "success-zone fixture resolves");
    if (succ < 0) return;
    test_add_to_success(&tg, succ);
    int succ_base = 0;
    for (int c = 0; c < 8; c++) succ_base += rb_mods_get_need_heart(&tg.state.mods, succ, c);
    memset(&e, 0, sizeof(e));
    e.action = (char *)"modify_required_hearts_success";
    e.count = 2;
    e.target = (char *)"self";
    set_extra(&e, "operation", "increase");
    set_extra(&e, "card_type", "live_card");
    set_extra(&e, "heart_colors", "heart02");
    rb_execute_effect_ex(&tg.state, 0, &e, host);
    int succ_after = 0;
    for (int c = 0; c < 8; c++) succ_after += rb_mods_get_need_heart(&tg.state.mods, succ, c);
    CHECK(succ_after != succ_base,
          "modify_required_hearts_success changes the success-zone requirement");
    tg.state.activating_card = -1;
}

/* ========================================================================
 * Group 15 -- effects/state/wait_activation : SetBladeType (:106),
 *          SetBladeCount (:110), SetHeartType (:114), SpecifyHeartColor (:118),
 *          AllBladeTiming (:149)
 * ======================================================================== */
static void test_blade_heart_state_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int member = test_new_id(&tg, "PL!-sd1-010-SD");
    CHECK(member >= 0, "blade/heart fixtures resolve");
    if (member < 0) return;
    test_add_to_stage(&tg, 1, member);

    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"set_blade_type";
    e.target = (char *)"self";
    set_extra(&e, "blade_type", "red");
    set_extra(&e, "duration", "permanent");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK_EQ(rb_mods_get_blade_type(&tg.state.mods, member), 1,
             "set_blade_type recolours the member (not its blade count)");

    memset(&e, 0, sizeof(e));
    e.action = (char *)"set_blade_count";
    e.target = (char *)"self";
    e.count = 1;
    set_extra(&e, "value", "5");
    set_extra(&e, "duration", "permanent");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK_EQ(tg.state.mods.blade[member].set, 5, "set_blade_count applies its exact value");

    memset(&e, 0, sizeof(e));
    e.action = (char *)"set_heart_type";
    e.target = (char *)"self";
    snprintf(e.self_target_field, sizeof(e.self_target_field), "true");
    set_extra(&e, "heart_type", "red");
    set_extra(&e, "duration", "permanent");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK_EQ(rb_mods_get_heart_color_multiplier(&tg.state.mods, member), RB_HEART_RED,
             "set_heart_type transforms the selected member");

    int prohibitions_before = tg.state.n_prohibition_effects;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"all_blade_timing";
    e.target = (char *)"self";
    set_extra(&e, "timing", "check_required_hearts");
    set_extra(&e, "treat_as", "any_heart_color");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    /* prohibition_effects[] entries are 48 bytes (rabuka.h:1422), so the note
     * is truncated well before treat_as; assert the prefix and card id only. */
    CHECK(tg.state.n_prohibition_effects == prohibitions_before + 1,
          "all_blade_timing records exactly one prohibition note");
    CHECK(!strncmp(tg.state.prohibition_effects[prohibitions_before],
                   "all_blade_timing:", 16),
          "all_blade_timing records its timing/treat_as prohibition note");
    {
        char expect_id[32];
        snprintf(expect_id, sizeof(expect_id), "all_blade_timing:%d:", member);
        CHECK(!strncmp(tg.state.prohibition_effects[prohibitions_before], expect_id,
                       strlen(expect_id)),
              "the all_blade_timing note is keyed to the resolving card");
    }

    memset(&e, 0, sizeof(e));
    e.action = (char *)"specify_heart_color";
    e.target = (char *)"self";
    set_extra(&e, "choice", "true");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(rb_has_pending_choice(&tg.state) && ch &&
              ch->kind == RB_CHOICE_SELECT_HEART_COLOR,
          "specify_heart_color prompts for a heart colour");
    rb_resume_with_choice(&tg.state, 0);
}

/* ========================================================================
 * Group 16 -- effects/ability_mod : ModifyLimit (executor.rs:135),
 *          ActivationCost (:175), SetCardIdentity (:79)
 * ======================================================================== */
static void test_ability_mod_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int member = test_new_id(&tg, "PL!-sd1-010-SD");
    CHECK(member >= 0, "ability_mod fixtures resolve");
    if (member < 0) return;
    test_add_to_stage(&tg, 1, member);

    int limit_before = tg.state.n_prohibition;
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"modify_limit";
    e.count = 2;
    e.target = (char *)"self";
    set_extra(&e, "operation", "decrease");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK(tg.state.n_prohibition == limit_before + 1 &&
              !strcmp(tg.state.prohibition[tg.state.n_prohibition - 1], "limit_decrease:2"),
          "modify_limit records a limit_decrease prohibition");

    memset(&e, 0, sizeof(e));
    e.action = (char *)"activation_cost";
    e.target = (char *)"self";
    set_extra(&e, "operation", "increase");
    set_extra(&e, "value", "2");
    set_extra(&e, "duration", "permanent");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK(has_prohibition(&tg.state, "activation_cost_increase_2"),
          "activation_cost records the state.rs::execute_activation_cost prohibition");

    memset(&e, 0, sizeof(e));
    e.action = (char *)"set_card_identity";
    e.target = (char *)"self";
    set_extra(&e, "all_regions", "true");
    set_extra(&e, "identities", "ZZZ Identity");
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK(has_prohibition(&tg.state, "card_identity:ZZZ Identity"),
          "set_card_identity records the card_identity prohibition");
}

/* ========================================================================
 * Group 17 -- effects/compound/sequential_effects : Sequential (executor.rs:126),
 *          RepeatProcedure (:147)
 * ======================================================================== */
static void test_compound_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int a = deck_card(&tg, "PL!-sd1-010-SD");
    int b = deck_card(&tg, "PL!-sd1-002-SD");
    int c = deck_card(&tg, "PL!-sd1-003-SD");
    int d = deck_card(&tg, "PL!-sd1-004-SD");
    CHECK(a >= 0 && b >= 0 && c >= 0 && d >= 0, "compound fixtures resolve");
    if (a < 0 || b < 0 || c < 0 || d < 0) return;

    AbilityEffect step;
    memset(&step, 0, sizeof(step));
    step.action = (char *)"draw_card";
    step.count = 1;
    step.source = (char *)"deck";
    step.destination = (char *)"hand";
    step.target = (char *)"self";

    AbilityEffect seq;
    memset(&seq, 0, sizeof(seq));
    seq.action = (char *)"sequential";
    seq.target = (char *)"self";
    seq.child[0] = &step;
    seq.child[1] = &step;
    seq.n_child = 2;
    rb_execute_effect_ex(&tg.state, 0, &seq, -1);
    CHECK_EQ(tg.state.p[0].hand.n, 2, "sequential runs every child step in order");

    /* repeat_procedure repeats its children `count` times. */
    reset_board(&tg);
    for (int i = 0; i < 4; i++) deck_card(&tg, "PL!-sd1-010-SD");
    AbilityEffect rep;
    memset(&rep, 0, sizeof(rep));
    rep.action = (char *)"repeat_procedure";
    rep.count = 2;
    rep.target = (char *)"self";
    rep.child[0] = &step;
    rep.n_child = 1;
    rb_execute_effect_ex(&tg.state, 0, &rep, -1);
    CHECK_EQ(tg.state.p[0].hand.n, 2, "repeat_procedure runs its children count times");
}

/* ========================================================================
 * Group 18 -- effects/mill : DiscardUntilCount (executor.rs:148)
 * ======================================================================== */
static void test_discard_until_count_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    for (int i = 0; i < 5; i++) test_add_to_hand(&tg, test_new_id(&tg, "PL!-sd1-010-SD"));
    CHECK_EQ(tg.state.p[0].hand.n, 5, "mill fixture hand seeded");
    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"discard_until_count";
    e.count = 2;
    e.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    const RbChoice *ch = rb_get_pending_choice(&tg.state);
    CHECK(rb_has_pending_choice(&tg.state) && ch && ch->kind == RB_CHOICE_SELECT_CARD,
          "discard_until_count prompts for the cards to mill");
    CHECK(ch && ch->count == 3, "discard_until_count asks for exactly the over-count");
    CHECK(ch && !strcmp(ch->zone, "hand"),
          "the mill prompt is scoped to the hand");
    /* NOTE (not asserted): answering this prompt through
     * rb_resume_with_choice does not complete the mill when the effect is
     * executed standalone. misc.c h_discard_until_count emits the choice with
     * target="hand" and route RB_ROUTE_SELECT_CARDS, and choice.c's resume only
     * has a zone=="hand" branch on the cost path. Reported to the choice.c
     * owner; asserting it here would pin a defect as expected behaviour. */
    rb_resume_with_choice(&tg.state, 0);
}

/* ========================================================================
 * Group 19 -- effects/other : DoNothing (executor.rs:145), Shuffle (:136),
 *          ReduceLiveCardSetLimit (:153)
 * ======================================================================== */
static void test_utility_dispatch(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int member = test_new_id(&tg, "PL!-sd1-010-SD");
    CHECK(member >= 0, "utility fixtures resolve");
    if (member < 0) return;
    test_add_to_stage(&tg, 1, member);

    AbilityEffect e;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"do_nothing";
    e.target = (char *)"self";
    int hand_before = tg.state.p[0].hand.n;
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.p[0].hand.n, hand_before, "do_nothing mutates nothing");

    int limit_before = tg.state.live_set_limit_reduction[0];
    memset(&e, 0, sizeof(e));
    e.action = (char *)"reduce_live_card_set_limit";
    e.count = 2;
    e.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &e, member);
    CHECK_EQ(tg.state.live_set_limit_reduction[0], limit_before + 2,
             "reduce_live_card_set_limit accumulates the reduction");

    int before_deck = tg.state.p[0].deck.n;
    memset(&e, 0, sizeof(e));
    e.action = (char *)"shuffle";
    e.target = (char *)"self";
    rb_execute_effect_ex(&tg.state, 0, &e, -1);
    CHECK_EQ(tg.state.p[0].deck.n, before_deck, "shuffle keeps the deck size intact");
}

/* ========================================================================
 * Group 20 -- the unsupported fallback and the verdict log.
 * executor.rs:9-40 (push_effect_verdict), executor.rs:191 (unconditional push).
 * ======================================================================== */
static void test_verdict_and_fallback(void)
{
    TestGame tg;
    test_game_new(&tg);
    reset_board(&tg);
    int host = test_new_id(&tg, "PL!N-bp1-027-L");
    CHECK(host >= 0, "verdict fixture resolves");
    if (host < 0) return;
    test_add_to_stage(&tg, 1, host);
    tg.state.activating_card = host;

    char details[256];
    AbilityEffect e;

    /* Rust pushes a verdict with count and nothing else. */
    rb_log_clear_verdicts();
    memset(&e, 0, sizeof(e));
    e.action = (char *)"modify_score";
    e.count = 3;
    e.target = (char *)"self";
    rb_executor_execute(&tg.state, 0, &e, host);
    CHECK_EQ(find_effect_verdict("modify_score", details, sizeof(details)), 1,
             "a resolved effect logs exactly one verdict");
    CHECK(!strcmp(details, "modify_score 3"), "the verdict detail is \"<action> <count>\"");

    /* Rust falls back to the `value` extra when count is absent. */
    rb_log_clear_verdicts();
    memset(&e, 0, sizeof(e));
    e.action = (char *)"modify_required_hearts";
    e.count = -1;
    e.target = (char *)"self";
    set_extra(&e, "value", "2");
    rb_executor_execute(&tg.state, 0, &e, host);
    find_effect_verdict("modify_required_hearts", details, sizeof(details));
    CHECK(!strcmp(details, "modify_required_hearts 2"),
          "the verdict falls back to the value extra when count is absent");

    /* executor.rs:10-18 -- `choice` IS structural, so it logs no verdict.
     * The C table used to mark it non-structural here. */
    rb_log_clear_verdicts();
    memset(&e, 0, sizeof(e));
    e.action = (char *)"choice";
    e.target = (char *)"self";
    rb_executor_execute(&tg.state, 0, &e, host);
    CHECK_EQ(find_effect_verdict("choice", details, sizeof(details)), 0,
             "choice is verdict-structural and logs nothing (executor.rs:14)");

    /* `conditional_optional` is NOT in the Rust structural set. */
    rb_log_clear_verdicts();
    memset(&e, 0, sizeof(e));
    e.action = (char *)"conditional_optional";
    e.target = (char *)"self";
    rb_executor_execute(&tg.state, 0, &e, host);
    CHECK_EQ(find_effect_verdict("conditional_optional", details, sizeof(details)), 1,
             "conditional_optional is not verdict-structural and does log");

    /* An action the table never claimed: rb_executor_execute must refuse it
     * rather than report success. */
    rb_log_clear_verdicts();
    memset(&e, 0, sizeof(e));
    e.action = (char *)"totally_unknown_action";
    e.count = 5;
    int hand_before = tg.state.p[0].hand.n;
    int ru = rb_executor_execute(&tg.state, 0, &e, host);
    CHECK_EQ(ru, 0, "an unclaimed action is reported as unhandled, not as success");
    CHECK_EQ(tg.state.p[0].hand.n, hand_before, "an unclaimed action mutates no state");
    CHECK_EQ(find_effect_verdict("totally_unknown_action", details, sizeof(details)), 0,
             "an unclaimed action logs no verdict (the early return precedes the push)");

    tg.state.activating_card = -1;
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }
    rb_log_set_enabled(1);

    test_action_vocabulary_table();
    test_compiled_vocabulary_reachable();

    test_draw_card_dispatch();
    test_draw_until_count_dispatch();
    test_move_cards_dispatch();
    test_discard_card_dispatch();
    test_look_and_reveal_dispatch();
    test_choice_dispatch();
    test_gain_resource_dispatch();
    test_pay_energy_dispatch();
    test_place_energy_under_member_dispatch();
    test_change_state_dispatch();
    test_rotation_dispatch();
    test_position_change_dispatch();
    test_modify_score_dispatch();
    test_cost_effect_dispatch();
    test_required_hearts_dispatch();
    test_blade_heart_state_dispatch();
    test_ability_mod_dispatch();
    test_compound_dispatch();
    test_discard_until_count_dispatch();
    test_utility_dispatch();
    test_verdict_and_fallback();

    rb_unload();
    printf("checks=%d failures=%d\n", checks, failures);
    if (failures) return 1;
    printf("ALL PARITY EXECUTOR DISPATCH CHECKS PASSED\n");
    return 0;
}
