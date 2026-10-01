/* effects/custom.c — faithful C translation of
 * engine/src/ability/effects/dispatch.rs:238-287
 *     AbilityResolver::execute_custom(gs, effect, action_str)
 *
 * Reached from engine/src/ability/effects/dispatch.rs:175
 *     ActionType::Custom => resolver.execute_custom(gs, effect, effect.action.to_str())
 *
 * STATUS AS OF 2026-10-02: THIS FUNCTION IS NOT THE LIVE ROUTE. The `custom`
 * action is dispatched by src/ability/effects/executor.c:330-342, which routes
 * it to execute_misc, and execute_misc's table (src/ability/effects/misc.c:2073)
 * calls misc.c's h_custom. That is the copy that runs. This file's copy has no
 * caller (verified by name across src/ and tests/), so this file is a SECOND
 * copy of one Rust function, not the port of it. It was previously a 10-line
 * trampoline into rb_translated_execute_custom_effect; that trampoline target
 * was a THIRD copy in ability_effects.c and has been deleted, leaving the two
 * copies below.
 *
 * WHY THE ROUTE COULD NOT BE CONSOLIDATED HERE: executor.c, misc.c,
 * include/rabuka.h and the Makefile are not this file's to edit. Making this
 * the single owner needs a ONE-LINE change in executor.c — see the report; it
 * was measured in the isolation snapshot and is neutral-to-positive.
 *
 * Of the two surviving copies THIS one is the faithful one, and the difference
 * is not cosmetic:
 *   arm 1  Rust: `let mut routed = effect.clone();` — a FULL clone, so `count`
 *             and every selection filter reach the routed move_cards. This file
 *             uses rb_effect_deep_clone. h_custom hand-builds a struct and copies
 *             count/target/optional/distinct/per_unit/card_type/self_target/extras,
 *             which drops `text`, `gained_effect` and the child effects.
 *   arm 2  Rust passes `text` and `target` EXPLICITLY (dispatch.rs:260-268):
 *             text = effect.text when non-empty else action_str, and
 *             target = effect.target_any().unwrap_or("self"). This file puts
 *             both on the clone. h_custom calls rb_gain_ability with neither.
 *   arm 3  identical in both copies.
 *
 * The `action_by` short-circuit lives ABOVE the dispatch, in mod.rs:131-134
 * ("Empty action (default) with action_by means it was entirely handled by
 * opponent"), and executor.c:338-339 mirrors it, so this function only ever
 * sees a Custom effect with NO action_by.
 *
 * The three arms below are Rust's three arms, in Rust's order:
 *   1. placement_order_any() == Some(PlacementOrder::AnyOrder)
 *        -> `let mut routed = effect.clone();` (a FULL clone: count, target,
 *           optional, distinct, per_unit, card_type, self_target and the whole
 *           extras array all survive), rewrite action/source/destination, then
 *           execute_move_cards. A hand-built near-empty struct instead loses
 *           `count`, and the routed move_cards then moves nothing.
 *   2. duration_any().is_some()
 *        -> execute_gain_ability. `text` is effect.text when non-empty, else
 *           the action_str (dispatch.rs:260-264); `target` defaults to "self".
 *   3. otherwise: the parser could not type this effect. Nothing to run; push
 *      "<pp> <act_name>: [[log_custom_effect]]" to the rule log.
 */
#include "rabuka.h"

#include <string.h>
#include <stdio.h>

/* alloc.c owns the clone/free pair; forward-declared rather than added to
   include/rabuka.h because include/rabuka.h belongs to another file. */
AbilityEffect *rb_effect_deep_clone(const AbilityEffect *src);
void rb_effect_free(AbilityEffect *e);

/* Value of extra key `k`, or NULL (mirrors the private helpers of misc.rs that
   the other two copies of this function each carry their own of). */
static const char *custom_extra(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

/* dispatch.rs:278-284 — "<player_prefix> <activating card name>: [[log_custom_effect]]"
   util.rs:490-502 player_prefix: the activating card's owner (searched across
   both players' stage / live / hand), else the active player. */
static const char *custom_player_prefix(const GameState *g, int card_id) {
    if (card_id >= 0) {
        for (int pl = 0; pl < 2; pl++) {
            const RbPlayer *P = &g->p[pl];
            for (int q = 0; q < RB_STAGE_SIZE; q++) if (P->stage[q] == card_id) return pl == 0 ? "P1" : "P2";
            for (int i = 0; i < P->live.n; i++)      if (P->live.cards[i] == card_id) return pl == 0 ? "P1" : "P2";
            for (int i = 0; i < P->hand.n; i++)      if (P->hand.cards[i] == card_id) return pl == 0 ? "P1" : "P2";
        }
    }
    return g->active == 0 ? "P1" : "P2";
}

static void custom_log_parser_gap(GameState *g) {
    char act_name[96];
    act_name[0] = '\0';
    if (g->activating_card >= 0) {
        Card c;
        if (rb_decode_card_by_index((uint32_t)g->activating_card, &c)) {
            if (c.name) strncpy(act_name, c.name, sizeof(act_name) - 1);
            act_name[sizeof(act_name) - 1] = '\0';
            rb_free_card(&c);
        }
    }
    const char *pp = custom_player_prefix(g, g->activating_card);
    char line[224];
    snprintf(line, sizeof(line), "%s %s: [[log_custom_effect]]", pp, act_name);
    rb_log_push_verdict(line, "rule_log", 1);
}

int rb_execute_custom_effect(GameState *g, int actor, AbilityEffect *effect,
                             const char *action_str)
{
    (void)action_str;  /* used only for the gain_ability text fallback below */
    if (!g || !effect) return 0;

    /* 1) Deck reordering: placement_order=any_order -> move_cards
     *    looked_at -> deck_top (dispatch.rs:245-256). */
    const char *order = custom_extra(effect, "placement_order");
    if (order && !strcmp(order, "any_order")) {
        /* `let mut routed = effect.clone();` -- the FULL effect, so the routed
           move_cards keeps count and every selection filter. */
        AbilityEffect *routed = rb_effect_deep_clone(effect);
        if (!routed) return 0;
        routed->action      = (char *)"move_cards";
        if (!routed->source)       routed->source      = (char *)"looked_at";
        if (!routed->destination)  routed->destination = (char *)"deck_top";
        rb_effect_move_cards(g, actor, routed);
        rb_effect_free(routed);
        return 1;
    }

    /* 2) Complex conditional scoring / gain_ability: has duration
     *    (dispatch.rs:259-274). */
    const char *duration = custom_extra(effect, "duration");
    if (duration && *duration) {
        /* Rust passes `text` explicitly; rb_gain_ability reads it off the effect,
           so put the resolved text on the clone rather than duplicating the
           whole gain_ability signature here. */
        const char *text = (effect->text && *effect->text) ? effect->text
                        : (action_str ? action_str : "");
        AbilityEffect *ge = rb_effect_deep_clone(effect);
        if (!ge) return 0;
        ge->text = (char *)text;
        if (!ge->target) ge->target = (char *)"self";
        rb_gain_ability(g, actor, ge);
        rb_effect_free(ge);
        return 1;
    }

    /* 3) Parse gap: the parser could not type this effect. Nothing to run. */
    custom_log_parser_gap(g);
    return 1;
}
