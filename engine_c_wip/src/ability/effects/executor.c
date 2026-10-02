#include "rabuka.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    const char *action;
    int structural;
} ExecutorEntry;

/* `structural` is the C dispatcher's own "this handler owns its children" flag
 * (consumed by rb_execute_effect_ex's generic pre-order child walk, engine.c).
 * It is deliberately NOT the same set as the verdict-suppression set below —
 * the two were conflated here until the Rust structural list was ported. */
static const ExecutorEntry executor_table[] = {
    {"draw_card", 0}, {"draw", 0}, {"draw_until_count", 0},
    {"move_cards", 0}, {"discard_card", 0}, {"gain_resource", 0},
    {"change_state", 0}, {"modify_score", 0}, {"gain_score", 0},
    {"modify_required_hearts", 0}, {"set_cost", 0}, {"set_cost_to_use", 0},
    {"set_blade_type", 0}, {"set_blade_count", 0}, {"set_heart_type", 0},
    {"activate_ability", 0}, {"invalidate_ability", 0},
    {"suppress_ability_trigger", 0}, {"gain_ability", 0},
    {"gain_ability_from_source", 0}, {"play_baton_touch", 0},
    {"reveal", 0}, {"reveal_per_group", 0}, {"select", 0},
    {"select_number", 0}, {"look_at", 0}, {"select_cards", 0},
    {"modify_required_hearts_global", 0}, {"modify_yell_count", 0},
    {"place_energy_under_member", 0}, {"activation_cost", 0},
    {"energy_placement", 0}, {"energy_state_change", 0},
    {"position_change", 0}, {"rotation", 0}, {"choice", 0},
    {"pay_energy", 0}, {"set_card_identity", 0}, {"discard_until_count", 0},
    {"restriction", 0}, {"re_yell", 0}, {"activation_restriction", 0},
    {"modify_limit", 0}, {"shuffle", 0}, {"custom", 0},
    {"modify_yell_source", 0}, {"specify_heart_color", 0},
    {"modify_required_hearts_success", 0}, {"all_blade_timing", 0},
    {"reveal_until_live_card", 0}, {"reveal_until_chosen_card", 0},
    /* choose_target_player OWNS its effect_steps, so it must be structural.
     * engine/src/ability/effects/mod.rs:261 routes an effect through
     * `normalized_sequential_steps` only when
     * `action_type == Sequential || action_type == LookAndSelect`
     * (move/mod.rs:168-170 returns None for everything else), so for
     * ChooseTargetPlayer the ROOT effect is dispatched FIRST and its steps
     * run ONCE, AFTER the seat has been chosen, via the continuation the
     * choice parks. Marking it non-structural made engine.c:220-223's generic
     * pre-order child walk run `effect_steps` EAGERLY, before the answer.
     * All six corpus instances (PL!N-bp3-010, PL!N-bp4-002, PL!S-bp3-007,
     * PL!S-bp1-008, PL!S-PR-041, PL!S-bp7-013) are the root effect and all
     * carry effect_steps. Structural only means "the dispatcher does not
     * pre-walk the children" — it does NOT suppress the verdict: the
     * verdict-suppression set is verdict_structural below, exactly as
     * dispatch.rs:30-38 spells it. */
    {"choose_target_player", 1}, {"choose_required_hearts", 0},
    {"perform_yell", 0}, {"repeat_procedure", 1}, {"sequential", 1},
    {"conditional_alternative", 1}, {"look_and_select", 1},
    {"conditional_on_result", 1}, {"conditional_on_optional", 1},
    {"conditional_optional", 1}, {"compound_action", 1},
    {"opponent_action", 1}, {"action_by", 1}, {"sequential_cost", 1},
    {"choice_condition", 1}, {"energy_condition", 1}, {"do_nothing", 0},
    /* engine/src/ability/enums.rs:314 ModifyCost => "modify_cost" and
     * enums.rs:352 ReduceLiveCardSetLimit => "reduce_live_card_set_limit".
     * Both were missing from this table even though the dispatch branches below
     * already existed, so the branches were unreachable from
     * rb_execute_effect_ex (which consults rb_executor_has_executor) and from
     * rb_execute_effect_via_registry. 21 compiled effects use modify_cost and
     * 2 use reduce_live_card_set_limit (tools/audit_actions.c). */
    {"modify_cost", 0}, {"reduce_live_card_set_limit", 0},
    /* Two corpus actions that Rust deliberately does NOT dispatch. Adding them
     * here is not a behaviour change: `rb_execute_effect_ex` (engine.c:315) only
     * routes through this table, so an action missing from it fell through to
     * engine.c's `handle_action`, whose final arm is an explicit no-op
     * (engine.c:783 "Unknown verbs: explicit no-op so they remain visible").
     * The observable defects were that (a) `rb_executor_has_executor` -- the
     * reachability predicate the #F corpus census asserts on
     * (test_parity_char2.c:1472) -- reported them uncovered, and (b)
     * `rb_executor_execute` bailed at its own `find_executor` guard printing
     * [EXECUTOR_MISS] (executor.c:306) and returning 0 WITHOUT pushing the
     * verdict, where Rust pushes one for every non-structural action
     * (dispatch.rs:233).
     *
     * Both are no-op arms on the Rust side too, and the comments there are the
     * reason they must stay no-ops here:
     *   - replacement_appear_wait (dispatch.rs:126) is a REPLACEMENT rule
     *     resolved once at placement by GameState::replacement_appear_wait_applies
     *     (turn/phases.rs:1281); dispatching it would re-apply it at an
     *     unrelated trigger.
     *   - count_as_multiple (dispatch.rs:129) is a counting WEIGHT consumed by
     *     GameState::weighted_zone_count (condition/card.rs:2124).
     * Neither consumer exists in C yet; that is tracked by the red tests in
     * tests/test_audit_actions.c, NOT here. */
    {"replacement_appear_wait", 0}, {"count_as_multiple", 0}
};

/* engine/src/ability/effects/executor.rs:10-18 — push_effect_verdict's
 * `matches!` structural set, verbatim. Only these six suppress the verdict
 * log entry. Note `choice` IS in the Rust set and `conditional_optional`
 * is NOT; the previous reuse of rb_executor_is_structural got both backwards
 * and additionally suppressed the verdict for six actions Rust logs. */
static const char *const verdict_structural[] = {
    "compound_action", "sequential", "choice", "conditional_alternative",
    "conditional_on_result", "conditional_on_optional"
};

static const ExecutorEntry *find_executor(const char *action)
{
    if (!action) return NULL;
    for (size_t i = 0; i < sizeof(executor_table) / sizeof(executor_table[0]); i++)
        if (strcmp(executor_table[i].action, action) == 0)
            return &executor_table[i];
    return NULL;
}

int rb_executor_has_executor(const char *action)
{
    return find_executor(action) != NULL;
}

int rb_executor_is_structural(const char *action)
{
    const ExecutorEntry *entry = find_executor(action);
    return entry && entry->structural;
}

static const char *effect_extra(const AbilityEffect *effect, const char *key)
{
    if (!effect || !key) return NULL;
    for (int i = 0; i < effect->n_extra; i++)
        if (effect->extra_k[i] && strcmp(effect->extra_k[i], key) == 0)
            return effect->extra_v[i];
    return NULL;
}

/* engine/src/ability/effects/executor.rs:10-18 */
static int verdict_is_structural(const char *action)
{
    if (!action) return 0;
    for (size_t i = 0; i < sizeof(verdict_structural) / sizeof(verdict_structural[0]); i++)
        if (strcmp(verdict_structural[i], action) == 0) return 1;
    return 0;
}

/* engine/src/ability/effects/executor.rs:9-40 */
static void push_effect_verdict(const AbilityEffect *effect)
{
    if (!effect || verdict_is_structural(effect->action)) return;
    char details[128];
    /* Rust: effect.count.or(effect.value_any()) — count first, then the
     * `value` extra; an absent value yields the bare action label. */
    const char *value_text = effect->count >= 0 ? NULL : effect_extra(effect, "value");
    if (effect->count >= 0)
        snprintf(details, sizeof(details), "%s %d", effect->action, effect->count);
    else if (value_text && *value_text)
        snprintf(details, sizeof(details), "%s %s", effect->action, value_text);
    else
        snprintf(details, sizeof(details), "%s", effect->action);
    rb_log_push_verdict_effect(effect->text ? effect->text : "",
                               effect->action ? effect->action : "", details);
}

static int execute_misc(GameState *g, int actor, const AbilityEffect *effect, int host_cid)
{
    int resolved = 0;
    return rb_execute_misc_effect_ex(g, actor, &g->p[actor], effect, host_cid, &resolved);
}

/* ───────────────── the generic target="both" split ─────────────────────────────
 *
 * Rule 9.8 / Q158. engine/src/ability/effects/mod.rs:246-254 runs
 * AbilityResolver::handle_both_targets (target.rs:81-148) for EVERY action,
 * BEFORE the action dispatch and before the compound routing, so an effect
 * carrying target="both" is executed once with target="self" and once with
 * target="opponent" rather than once against a single collapsed player.
 *
 * The C had that split in only two places, both of them single-action:
 *   - rb_misc_handle_both_targets (misc.c:1726), reached only from
 *     rb_execute_misc_effect (misc.c:1853), i.e. only for the misc action table;
 *   - execute_move_cards_both_targets (previously executor.c:211), reached only
 *     from rb_executor_execute's own `move_cards` arm.
 * Any other action collapsed `both` onto the actor, because the per-player
 * resolvers all fall back to the actor for an unrecognised target string
 * (state.c:1603-1606 s_who; score.c:582-585 owner; draw.c's both-hand handler
 * `int pl = actor`). Four corpus cards are affected, and their text is explicit:
 *   change_state  PL!-bp5-111-P+ 綺羅ツバサ / PL!HS-pb1-008-P+ 桂城 泉
 *                 「自分と相手のステージ」 — only the actor's members waited.
 *   select        PL!S-bp7-004-P 黒澤ダイヤ
 *                 「自分と相手はそれぞれ、自身の手札」 — only the actor's hand.
 *   modify_score  PL!-bp3-022-L ユメノトビラ (per_unit + location=stage)
 *                 Rust counts members on BOTH stages; C scored off player1.
 *
 * The split therefore lives at the TOP of rb_executor_execute, which is the
 * C twin of Rust's `execute_effect` dispatch seam and sits above every action
 * arm in the table below. That is the same position the Rust call occupies:
 * before dispatch, and before the compound routing that Sequential and
 * LookAndSelect take (mod.rs:261).
 *
 * Delegated, not reimplemented. rb_misc_handle_both_targets is the faithful
 * port of target.rs:81-148 and already carries the three exemptions in Rust's
 * order (target.rs:87-91 then :95-117), the master-from-the-queue-entry
 * resolution (target.rs:121 `set_action_by`, abilities.rs:2675-2702), the
 * self-first ordering (target.rs:119-126), the defer-the-opponent-arm-when-self
 * opened a choice park (target.rs:129-138), and the s_activating_card save.
 * Duplicating it here would be a second implementation free to drift from the
 * first, which is exactly the failure mode this seam was split across in the
 * first place. The `move_cards` copy that used to live here is deleted: it was
 * reachable only from the `move_cards` arm below, which the new top-of-function
 * check now intercepts first, so it had become dead the moment this landed.
 *
 * NO DOUBLE APPLICATION. Both arms carry target "self"/"opponent", never
 * "both", and rb_misc_handle_both_targets keys on the LITERAL string "both"
 * (misc.c:1727, `!e->target || strcmp(e->target,"both")`), so re-entering it
 * from an arm returns 0 and the split cannot recurse. Its own arms re-dispatch
 * through rb_executor_execute (misc.c:1685-1697, both_dispatch_arm), which is
 * this function again — hence the check must be, and is, keyed on "both" alone.
 * A rewritten target matches nothing: "self" and "opponent" are the only other
 * values any of the exempt tests can compare against, and the two
 * multi-target-to-deck halves additionally require target == "deck".
 *
 * The wrapper pushes NO verdict of its own when the split fires, because Rust
 * returns from execute_effect at mod.rs:253 without ever reaching
 * dispatch.rs:233 — only the two arms log one. Returning early here, above the
 * dispatch table and above push_effect_verdict, reproduces that exactly. */

static int execute_modify_required_hearts_global(GameState *g, int actor,
                                                  const AbilityEffect *effect)
{
    const char *colors[8];
    int n_colors = 0;
    const char *csv = effect_extra(effect, "heart_colors");
    if (!csv) csv = effect_extra(effect, "heart_color");
    if (csv) {
        char buffer[256];
        snprintf(buffer, sizeof(buffer), "%s", csv);
        char *token = strtok(buffer, ",");
        while (token && n_colors < 8) {
            colors[n_colors++] = token;
            token = strtok(NULL, ",");
        }
    }
    int value = effect->count >= 0 ? effect->count : 1;
    const char *value_text = effect_extra(effect, "value");
    if (value_text) value = atoi(value_text);
    const char *operation = effect_extra(effect, "operation");
    if (!operation) operation = "increase";
    rb_execute_modify_required_hearts_standard(g, actor, operation, value,
                                                colors, n_colors,
                                                effect->target ? effect->target : "self",
                                                effect->text);
    return 1;
}

static int execute_repeat(GameState *g, int actor, AbilityEffect *effect, int host_cid)
{
    int count = effect->count >= 0 ? effect->count : 1;
    if (effect->repeat_limit > 0) count = effect->repeat_limit + 1;
    for (int r = 0; r < count; r++) {
        for (int i = 0; i < effect->n_child; i++) {
            rb_execute_effect_ex(g, actor, effect->child[i], host_cid);
            if (rb_has_pending_choice(g)) return 1;
        }
    }
    return 1;
}

int rb_executor_execute(GameState *g, int actor, AbilityEffect *effect, int host_cid)
{
    if (!g || !effect || !effect->action) return 0;
    /* engine/src/ability/effects/mod.rs:184-185 --
     *     let mut dbg = AbDebug::new();
     *     dbg.effect(effect);
     * the FIRST two statements of AbilityResolver::execute_effect, ahead of the
     * trace!, ahead of gate_effect_activation, ahead of the both-targets split
     * and ahead of the sequential routing. rb_executor_execute is that
     * function's dispatch seam, so the tracer line belongs here, before the
     * find_executor guard: even a string Rust cannot reach traces on its way
     * to being rejected.
     *
     * A per-effect AbDebug, separate from the one resolve_ability owns --
     * resolver.c:717-719 is the same treatment for can_activate_effect's
     * resolver.rs:581-582 call. Both write the one global trace buffer
     * (debug.c:26), which resolver.c:1198-1199 flushes on every resolution
     * exit. DEBUG TRACING ONLY: rb_abdebug_effect returns immediately unless
     * rb_ability_debug_enabled() (debug.c:218), matching debug.rs:163-165. */
    RbAbDebug dbg;
    rb_abdebug_init(&dbg);
    rb_abdebug_effect(&dbg, effect);
    if (!find_executor(effect->action)) {
        fprintf(stderr, "[EXECUTOR_MISS] action=%s\n", effect->action);
        return 0;
    }
    /* Rule 9.8 / Q158 — engine/src/ability/effects/mod.rs:246-254 runs
     * handle_both_targets (target.rs:81-148) for EVERY action, before the
     * dispatch and before the compound routing. This is the same seam, so an
     * effect with target="both" is executed once for self and once for opponent
     * instead of collapsing onto one player. See the long comment above for the
     * exemption list and the double-application argument.
     *
     * It sits AFTER the find_executor guard on purpose: Rust's match is
     * exhaustive over ActionType, so an action with no executor here is not a
     * shape Rust can reach, and keeping the guard first preserves the
     * [EXECUTOR_MISS] diagnostic for exactly the actions that have no arm. */
    if (rb_misc_handle_both_targets(g, actor, effect))
        return 1;
    if (rb_ability_debug_enabled())
        fprintf(stderr, "[EXECUTOR] action=%s host=%d cond=%d\n",
                effect->action, host_cid, effect->has_condition);
    const char *action = effect->action;
    int result = 1;

    if (strcmp(action, "draw_card") == 0 || strcmp(action, "draw") == 0) {
        result = rb_effect_draw_card(g, actor, effect, host_cid);
    } else if (strcmp(action, "draw_until_count") == 0) {
        fprintf(stderr, "[EXECUTOR_DRAW_UNTIL] actor=%d ptr=%p\n", actor, (void *)effect);
        rb_effect_draw_until_count(g, actor, effect);
    } else if (strcmp(action, "move_cards") == 0) {
        /* The target="both" split already ran at the top of this function
         * (mod.rs:246-254), so reaching this arm means the effect is
         * single-player, or is one of the two shapes handle_both_targets
         * exempts: position_change never gets here, and a multi-target-to-deck
         * move_cards is handled inside rb_effect_move_cards itself
         * (move_missing.c:139, the C twin of execute_move_cards_both at
         * engine/src/ability/move_cards.rs:3411-3445). */
        rb_effect_move_cards(g, actor, effect);
    } else if (strcmp(action, "discard_card") == 0) {
        /* engine/src/ability/effects/dispatch.rs:74 —
         *   ActionType::DiscardCard => resolver.execute_logged_move(gs, effect,
         *                                                    "[[log_discard]]")
         * i.e. the SAME shared tail as MoveCards (:73), and
         * move/mod.rs:3594-3603 is that tail in full:
         *   self.rule_log_activated(gs, log_tag);
         *   self.owner.executing = Some(effect.clone());
         *   self.execute_move_cards(gs, effect)
         * It reads `effect.source` / `effect.destination` exactly as decoded.
         * There is NO defaulting anywhere on the path — `source_or("none")`
         * appears only in the [EFFECT] TRACE line (mod.rs:190), never as a
         * value handed to the mover.
         *
         * The copy this replaces defaulted an absent source to "hand" and an
         * absent destination to "waitroom", so a corpus discard_card whose text
         * carried no zone fields still moved hand->waitroom here. That is a C
         * invention: with the defaults gone, the effect's own fields stand and
         * a zone-less discard_card moves nothing, which is what Rust does. */
        rb_effect_move_cards(g, actor, effect);
    } else if (strcmp(action, "select") == 0) {
        rb_effect_select_effect(g, actor, effect, host_cid);
    } else if (strcmp(action, "select_cards") == 0) {
        rb_effect_select_cards(g, actor, effect);
    } else if (strcmp(action, "look_and_select") == 0) {
        rb_effect_look_and_select(g, actor, effect);
    } else if (strcmp(action, "select_number") == 0) {
        rb_effect_select_number(g, actor, effect);
    } else if (strcmp(action, "look_at") == 0) {
        rb_effect_look_at(g, actor, effect);
    } else if (strcmp(action, "reveal") == 0) {
        rb_effect_reveal(g, actor, effect);
    } else if (strcmp(action, "reveal_per_group") == 0) {
        rb_effect_reveal_per_group(g, actor, effect);
    } else if (strcmp(action, "reveal_until_live_card") == 0) {
        rb_effect_reveal_until_live_card(g, actor, effect);
    } else if (strcmp(action, "reveal_until_chosen_card") == 0) {
        rb_effect_reveal_until_chosen_card(g, actor, effect);
    } else if (strcmp(action, "change_state") == 0) {
        rb_effect_change_state(g, actor, effect, host_cid);
    } else if (strcmp(action, "position_change") == 0 ||
               strcmp(action, "rotation") == 0) {
        result = execute_misc(g, actor, effect, host_cid);
    } else if (strcmp(action, "gain_resource") == 0 ||
               strcmp(action, "place_energy_under_member") == 0 ||
               strcmp(action, "play_baton_touch") == 0 ||
               strcmp(action, "pay_energy") == 0 ||
               strcmp(action, "discard_until_count") == 0 ||
               strcmp(action, "restriction") == 0 ||
               strcmp(action, "activation_restriction") == 0 ||
               strcmp(action, "choice") == 0 ||
               strcmp(action, "choose_target_player") == 0 ||
               strcmp(action, "choose_required_hearts") == 0 ||
               strcmp(action, "re_yell") == 0 ||
               strcmp(action, "perform_yell") == 0 ||
               strcmp(action, "shuffle") == 0) {
        result = execute_misc(g, actor, effect, host_cid);
    } else if (strcmp(action, "custom") == 0) {
        /* engine/src/ability/effects/mod.rs:94-97 (prepare_opponent_routing):
         * "Empty action (default) with action_by means it was entirely handled
         * by opponent" — a Custom effect that carries `action_by` was already
         * routed to the opponent by the wrapper, so the executor must do
         * NOTHING. Without this the C fell into execute_misc and re-ran the
         * effect against the actor. */
        if (rb_effect_action_by_any(effect)) {
            result = 1;
        } else {
            /* engine/src/ability/effects/dispatch.rs:172 —
             *   ActionType::Custom => resolver.execute_custom(gs, effect,
             *                                                       effect.action.to_str())
             * i.e. this function, reached DIRECTLY from the dispatch table. It
             * used to fall through to execute_misc, whose table (misc.c:2073)
             * called misc.c's h_custom — a second copy of the same Rust
             * function, and the copy that actually ran. custom.c's
             * rb_execute_custom_effect is now the single owner: it deep-clones
             * the whole effect before rewriting action/source/destination
             * (dispatch.rs:240 `let mut routed = effect.clone()`) where
             * h_custom hand-built a near-empty struct and dropped `count`,
             * `text` and the child effects, and it holds that clone in a
             * keep-alive pool for the duration of the call because
             * rb_effect_move_cards parks the pointer it is handed in
             * queue.resume_eff (move.c:687) — h_custom passed the address of a
             * STACK struct, which dangles as soon as the choice is answered.
             * `action.to_str()` for ActionType::Custom is "custom"
             * (enums.rs:469), which is also the only consumer of the argument
             * (the arm-2 `text` fallback, dispatch.rs:254-257). */
            result = rb_execute_custom_effect(g, actor, effect, "custom");
        }
    } else if (strcmp(action, "modify_score") == 0) {
        /* engine/src/ability/effects/dispatch.rs:135 —
         *   ActionType::ModifyScore => resolver.execute_modify_score(gs, effect)
         * with NO precondition and NO special case. score.rs:165-180 shows what
         * that executor actually is:
         *   let value: u8 = effect.value_any().unwrap_or(0);   (score.rs:174 -- 0,
         *       not 1, and not effect.count)
         *   let target = effect.target_name().to_string();     (score.rs:175 --
         *       target_name() defaults to "self", card.rs:2366-2368)
         * and the body NEVER writes `player.score`: a modify_score effect only
         * reaches card-scoped / live_total score modifiers.
         *
         * The arm this replaces added straight into `g->p[actor].score` and
         * defaulted the value to 1, for a hostless, targetless, non-self-target
         * effect. Both are invented: the write had no Rust counterpart at all,
         * and the 1 made a value-less effect behave like +1 against the
         * player's running total instead of +0 against the modifier table.
         * rb_execute_modify_score already implements score.rs faithfully
         * (score.c:497-498 takes value_any().unwrap_or(0); score.c:499 applies
         * target_name()'s "self" default and resolves the owner at score.c:524).
         *
         * gain_score keeps the invented player-score bump: Rust has NO such
         * ActionType (it is one of the four C-only table entries, called from
         * nothing), so there is no Rust behaviour for it to diverge from. It is
         * corpus-dead and test-dead; it is left exactly as it was. */
        result = rb_execute_modify_score(g, actor, effect);
    } else if (strcmp(action, "gain_score") == 0) {
        const char *operation = effect_extra(effect, "operation");
        int value = effect->count >= 0 ? effect->count : 1;
        const char *value_text = effect_extra(effect, "value");
        if (value_text) value = atoi(value_text);
        int delta = operation && !strcmp(operation, "remove") ? -value : value;
        g->p[actor].score += delta;
        result = 1;
    } else if (strcmp(action, "modify_required_hearts") == 0 ||
               strcmp(action, "modify_required_hearts_success") == 0) {
        result = strcmp(action, "modify_required_hearts_success") == 0
                     ? rb_execute_modify_required_hearts_success(g, actor, effect)
                     : rb_execute_modify_required_hearts(g, actor, effect);
    } else if (strcmp(action, "modify_required_hearts_global") == 0) {
        result = execute_modify_required_hearts_global(g, actor, effect);
    } else if (strcmp(action, "set_cost") == 0) {
        rb_effect_set_cost(g, actor, effect, host_cid);
    } else if (strcmp(action, "set_cost_to_use") == 0) {
        rb_effect_set_cost_to_use(g, actor, effect, host_cid);
    } else if (strcmp(action, "modify_cost") == 0) {
        rb_effect_modify_cost(g, actor, effect, host_cid);
    } else if (strcmp(action, "modify_yell_count") == 0) {
        result = rb_execute_modify_yell_count(g, actor, effect);
    } else if (strcmp(action, "set_blade_type") == 0) {
        rb_effect_set_blade_type(g, actor, effect, host_cid);
    } else if (strcmp(action, "set_blade_count") == 0) {
        rb_effect_set_blade_count(g, actor, effect, host_cid);
    } else if (strcmp(action, "set_heart_type") == 0) {
        rb_effect_set_heart_type(g, actor, effect, host_cid);
    } else if (strcmp(action, "specify_heart_color") == 0) {
        rb_effect_specify_heart_color(g, actor, effect, host_cid);
    } else if (strcmp(action, "activate_ability") == 0) {
        result = rb_translated_execute_activate_ability(g, actor, effect, host_cid);
    } else if (strcmp(action, "invalidate_ability") == 0) {
        result = rb_translated_execute_invalidate_ability(g, actor, effect);
    } else if (strcmp(action, "suppress_ability_trigger") == 0) {
        result = rb_translated_execute_suppress_ability_trigger(g, actor, effect);
    } else if (strcmp(action, "gain_ability") == 0) {
        result = rb_translated_execute_gain_ability_effect(g, actor, effect, host_cid);
    } else if (strcmp(action, "gain_ability_from_source") == 0) {
        result = rb_translated_execute_gain_ability_from_source(g, actor, effect, host_cid);
    } else if (strcmp(action, "set_card_identity") == 0) {
        result = rb_translated_execute_set_card_identity_effect(g, actor, effect, host_cid);
    } else if (strcmp(action, "modify_limit") == 0) {
        result = rb_execute_modify_limit(g, actor, effect);
    } else if (strcmp(action, "reduce_live_card_set_limit") == 0) {
        rb_effect_reduce_live_card_set_limit(g, actor, effect, host_cid);
    } else if (strcmp(action, "all_blade_timing") == 0) {
        rb_effect_all_blade_timing(g, actor, effect, host_cid);
    } else if (strcmp(action, "energy_placement") == 0) {
        rb_effect_energy_placement(g, actor, effect);
    } else if (strcmp(action, "energy_state_change") == 0) {
        rb_effect_energy_state_change(g, actor, effect);
    } else if (strcmp(action, "activation_cost") == 0) {
        /* engine/src/ability/effects/executor.rs:175-178. The table already
         * claimed activation_cost, so the branch was missing and the effect
         * silently resolved to nothing instead of reaching
         * state.rs::execute_activation_cost. */
        rb_effect_activation_cost(g, actor, effect, host_cid);
    } else if (strcmp(action, "sequential") == 0) {
        rb_compound_sequential(g, actor, effect, host_cid);
    } else if (strcmp(action, "conditional_alternative") == 0) {
        rb_compound_conditional_alternative(g, actor, effect, -1, host_cid);
    } else if (strcmp(action, "conditional_on_result") == 0) {
        rb_compound_conditional_on_result(g, actor, effect, host_cid);
    } else if (strcmp(action, "conditional_on_optional") == 0 ||
               strcmp(action, "conditional_optional") == 0) {
        rb_compound_conditional_on_optional(g, actor, effect, -1, host_cid);
    } else if (strcmp(action, "repeat_procedure") == 0) {
        result = execute_repeat(g, actor, effect, host_cid);
    } else if (strcmp(action, "compound_action") == 0) {
        /* engine/src/ability/effects/dispatch.rs:226-231 — CompoundAction is in
         * the SAME `=> Ok(())` arm as OpponentAction / ActionBy / SequentialCost
         * / ChoiceCondition / EnergyCondition, and that arm's own comment is
         * explicit about why: "CompoundAction / ActionBy -- structural tags only
         * (see push_effect_verdict, and effect.action_by for the tag itself)".
         * Nothing in Rust ever runs a CompoundAction's primary_effect or
         * child[0]: the expansion belongs to
         * `compound::sequential::execute_sequential_effect`, reached through
         * `normalized_sequential_steps` (mod.rs:261) and the repeated-procedure
         * expansion — never from dispatch.
         *
         * This arm used to run primary_effect, else child[0]: a second,
         * competing expansion that fires at dispatch time, ahead of any gate,
         * and fires even for a CompoundAction that carries no steps at all. The
         * action is corpus-dead (0 decoded occurrences), so this removes an
         * unreachable divergence rather than introducing one. */
        result = 1;
    } else if (strcmp(action, "opponent_action") == 0) {
        /* engine/src/ability/effects/mod.rs:73-105 (prepare_opponent_routing).
         * Rust clones the NESTED `opponent_action` effect — never the wrapper —
         * forces target="opponent" when the nested target is absent or "self",
         * executes that clone, and then the wrapper itself resolves to Ok(true)
         * (nothing further to do).
         *
         * The previous code instead re-dispatched the WRAPPER with only
         * `target` rewritten, so `routed.action` was still "opponent_action" and
         * rb_executor_execute recursed on itself until the engine.c depth cap
         * (64) stopped it: the nested opponent effect never ran, at the cost of
         * 64 stack frames per occurrence. */
        const AbilityEffect *nested = effect->opponent_action;
        if (nested) {
            AbilityEffect routed = *nested;
            if (!routed.target || !strcmp(routed.target, "self"))
                routed.target = (char *)"opponent";
            rb_execute_effect_ex(g, actor, &routed, host_cid);
        }
        result = 1;
    } else if (strcmp(action, "modify_yell_source") == 0) {
        const char *source = effect_extra(effect, "yell_source");
        if (!source) source = effect->source;
        int target = rb_target_player_index(effect->target, actor == 0 ? "p1" : "p2");
        if (target < 0) target = actor;
        if (target >= 0 && target < 2 && source) {
            snprintf(g->yell_source[target], sizeof(g->yell_source[target]), "%s", source);
            g->p[target].yell_from_bottom = !strcmp(source, "deck_bottom") ||
                                           !strcmp(source, "bottom");
        }
        result = 1;
    } else if (strcmp(action, "action_by") == 0 ||
               strcmp(action, "choice_condition") == 0 ||
               strcmp(action, "energy_condition") == 0 ||
               strcmp(action, "do_nothing") == 0) {
        result = 1;
    } else if (strcmp(action, "replacement_appear_wait") == 0 ||
               strcmp(action, "count_as_multiple") == 0) {
        /* engine/src/ability/effects/dispatch.rs:126 and :129 -- both are
         * `=> Ok(())`, NOT executors. They reach this arm only so that the
         * verdict below is pushed exactly as Rust pushes it at dispatch.rs:233;
         * their rules are consumed by the placement / count paths, which in C
         * do not exist yet. See the executor_table comment above. */
        result = 1;
    } else if (strcmp(action, "sequential_cost") == 0) {
        /* engine/src/ability/effects/executor.rs:187 — SequentialCost is in the
         * same `=> Ok(())` group as CompoundAction/OpponentAction/ActionBy:
         * the sub-costs are already paid in order by the dedicated cost path
         * (engine/src/ability/cost/handlers.rs:957-967, the C twin is
         * cost.c's cost_is_sequential), so the effect executor must do nothing.
         * It previously fell through to the unsupported-action arm below and
         * reported result=0 ("no state change") instead of Rust's Ok(()).
         * 28 compiled abilities declare a sequential_cost cost. */
        result = 1;
    } else {
        /* Unsupported fallback. Rust's match is exhaustive over ActionType, so
         * this arm only fires for a string that never decoded as an action.
         * Mirror Rust's "no state change" outcome rather than reporting
         * success; the verdict is still logged below. */
        result = 0;
    }

    /* engine/src/ability/effects/executor.rs:191 — the verdict is pushed
     * unconditionally after the match, including on the Err arms. */
    push_effect_verdict(effect);
    return result;
}

/* DELETED 2026-10-02: rb_execute_effect_via_registry was a zero-caller pure
 * alias of rb_executor_execute -- a second public name for the same body. Every
 * live route already reaches the dispatcher by rb_executor_execute: engine.c
 * :300-301 from rb_execute_effect_ex, and the internal re-dispatches in
 * executor.c / misc.c / move_missing.c all call it by name. A second entry point
 * is a third thing free to drift from the first, with no reader to catch it. The
 * body is gone; the declaration at rabuka.h:2982 is dropped by that file's
 * owner. Until it is, this translation unit inherits the header prototype and
 * simply never defines it, which is why the tree still links. */
