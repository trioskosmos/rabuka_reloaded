/* effects/custom.c — faithful C translation of
 * engine/src/ability/effects/dispatch.rs:238-287
 *     AbilityResolver::execute_custom(gs, effect, action_str)
 *
 * Reached from engine/src/ability/effects/dispatch.rs:175
 *     ActionType::Custom => resolver.execute_custom(gs, effect, effect.action.to_str())
 *
 * STATUS AS OF 2026-10-03: THIS FUNCTION IS NOW THE LIVE ROUTE.
 * src/ability/effects/executor.c's `custom` arm used to fall through to
 * execute_misc, and execute_misc's table (src/ability/effects/misc.c:2073)
 * called misc.c's h_custom — a second copy of the same Rust function. That arm
 * now calls rb_execute_custom_effect directly (executor.c:379-392), so this is
 * the one copy that runs. misc.c's h_custom is dead; the exact lines to delete
 * are recorded in PARITY_CHECKLIST.md F1 (misc.c is another agent's file).
 *
 * THE CLONE LIFETIME (why this file was previously refused a promotion, and how
 * it is fixed). `let mut routed = effect.clone()` (dispatch.rs:240) is a Rust
 * LOCAL that outlives the nested execute_move_cards call, and the borrow checker
 * then guarantees that anything the mover parks for later has been CLONED by
 * the mover itself. The C has no such guarantee: rb_effect_move_cards parks the
 * pointer it was handed in `queue.resume_eff`
 * (move.c:rb_move_prompt_carry_resume_context, move.c:687) and choice.c reads
 * it back after this frame is gone. So a heap clone freed on return (what this
 * file used to do) is a use-after-free, and misc.c's live copy was worse still:
 * it hands the mover the address of a STACK struct.
 *
 * The fix is the same keep-alive discipline src/ability/compound.c already uses
 * for its sequential-step copies (`seq_keep_push` / `seq_keep_gc`,
 * compound.c:99-201): the routed clone lives in a pool, is PINNED while this C
 * frame holds it as an argument, and is reclaimed only once NO live pointer in
 * the owning GameState refers to it. The pool is a DEEP clone, so releasing a
 * slot is rb_effect_free and nothing here aliases the decoded ability tree.
 * Where compound.c keeps SHALLOW copies (it must: its slots are `action.clone()`
 * of a node whose children are shared), this file deep-clones because it also
 * REWRITES action/source/destination on the copy and must not write through to
 * the decoded tree.
 *
 * The two remaining copies differed, and the difference was not cosmetic:
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

/* ─────────────────────── routed-effect keep-alive ──────────────────────────────
 *
 * Rust's `let mut routed = effect.clone()` is a local that OUTLIVES the nested
 * execute_move_cards call, so the borrow checker forces the mover to clone
 * anything it wants to keep. C has no such guarantee: rb_effect_move_cards
 * parks the pointer it was handed in `queue.resume_eff` (move.c:687) and the
 * answer handler reads it back long after this frame returns. The pool below
 * gives the routed clone exactly the lifetime Rust's borrow checker gives the
 * local: it is pinned while this frame holds it, and reclaimed once no live
 * pointer in the owning GameState refers to it.
 *
 * Modelled on compound.c's seq_keep_* pool (compound.c:99-201), which solves the
 * same problem for sequential-step copies; two differences are deliberate:
 *   - the slots are DEEP clones, so rb_effect_free is the matching destructor
 *     and no slot ever aliases the decoded ability tree;
 *   - custom_keep_settle() frees an already-unreferenced slot on the spot,
 *     which is what makes the no-park case (arm 2) cost nothing.
 *
 * A slot must look dead on TWO consecutive gc passes before it is freed. That
 * one-pass grace is load-bearing, exactly as in compound.c:154-158:
 * rb_resume_with_choice_indices_internal copies queue.resume_eff into a local
 * and only then NULLs the field, and a drain in between can re-enter a custom
 * arm and run a gc. A single-pass gc frees the slot inside that window and
 * reintroduces the use-after-free this pool exists to remove.
 *
 * Slots are tagged with their owning GameState so two states in one process
 * cannot reclaim each other's clones. Like compound.c's pool, nothing tears this
 * one down at exit: a clone still parked when the process ends is reclaimed by
 * the OS, and a live GameState keeps at most CUSTOM_KEEP_CAP of them. */
#define CUSTOM_KEEP_CAP 16
static AbilityEffect *g_custom_keep[CUSTOM_KEEP_CAP];
static GameState      *g_custom_keep_owner[CUSTOM_KEEP_CAP];
static int            g_custom_keep_pinned[CUSTOM_KEEP_CAP];
static int            g_custom_keep_stale[CUSTOM_KEEP_CAP];
static int            g_custom_keep_n = 0;

/* Is this routed clone still reachable from a live continuation slot? The slot
 * list is compound.c's seq_keep_reachable set (compound.c:107-132): every place
 * that parks an AbilityEffect pointer across a pause. */
static int custom_keep_reachable(const GameState *g, const AbilityEffect *e) {
    if (!g || !e) return 0;
    if (g->queue.resume_eff == e) return 1;
    if (g->queue.deferred == e) return 1;
    if (g->queue.target_selection_eff == e) return 1;
    if (g->queue.resume_after_look == e) return 1;
    for (int i = 0; i < g->queue.n_entries; i++) {
        const RbQueueEntry *en = &g->queue.entries[i];
        for (int k = 0; k < en->pending_actions_n; k++)
            if (en->pending_actions[k] == e) return 1;
    }
    for (int i = 0; i < g->queue.pending_repeat_actions_n; i++)
        if (g->queue.pending_repeat_actions[i] == e) return 1;
    for (int i = 0; i < g->queue.choice_options_n; i++)
        if (g->queue.choice_options[i] == e) return 1;
    for (int i = 0; i < g->queue.n_pending_deferred_costs; i++)
        if (g->queue.pending_deferred_costs[i] == e) return 1;
    for (int i = 0; i < g->n_delayed_gained_effects; i++)
        if (g->delayed_gained_effects[i].effect == e) return 1;
    return 0;
}

static void custom_keep_drop(int idx) {
    rb_effect_free(g_custom_keep[idx]);
    g_custom_keep[idx] = NULL;
    g_custom_keep_owner[idx] = NULL;
    g_custom_keep_pinned[idx] = 0;
    g_custom_keep_stale[idx] = 0;
    for (int j = idx; j + 1 < g_custom_keep_n; j++) {
        g_custom_keep[j] = g_custom_keep[j + 1];
        g_custom_keep_owner[j] = g_custom_keep_owner[j + 1];
        g_custom_keep_pinned[j] = g_custom_keep_pinned[j + 1];
        g_custom_keep_stale[j] = g_custom_keep_stale[j + 1];
    }
    g_custom_keep_n--;
}

static void custom_keep_gc(GameState *g) {
    for (int i = 0; i < g_custom_keep_n; ) {
        if (g_custom_keep_pinned[i]) { i++; continue; }
        if (g_custom_keep_owner[i] == g && custom_keep_reachable(g, g_custom_keep[i])) {
            g_custom_keep_stale[i] = 0;
            i++;
            continue;
        }
        if (!g_custom_keep_stale[i]) { g_custom_keep_stale[i] = 1; i++; continue; }
        custom_keep_drop(i);
    }
}

/* Deep-clone `src` into a pool slot and PIN it: this C frame holds it as an
 * argument to the callee, so no gc that re-enters through the callee may
 * reclaim it. */
static AbilityEffect *custom_keep_push(GameState *g, const AbilityEffect *src) {
    custom_keep_gc(g);
    if (g_custom_keep_n >= CUSTOM_KEEP_CAP) {
        /* Only reachable if more than CUSTOM_KEEP_CAP custom continuations are
           parked at once; recycle the oldest unpinned slot rather than fail. */
        for (int i = 0; i < g_custom_keep_n; i++)
            if (!g_custom_keep_pinned[i]) { custom_keep_drop(i); break; }
        if (g_custom_keep_n >= CUSTOM_KEEP_CAP) custom_keep_drop(0);
    }
    AbilityEffect *slot = rb_effect_deep_clone(src);
    if (!slot) return NULL;
    g_custom_keep[g_custom_keep_n] = slot;
    g_custom_keep_owner[g_custom_keep_n] = g;
    g_custom_keep_pinned[g_custom_keep_n] = 1;
    g_custom_keep_stale[g_custom_keep_n] = 0;
    g_custom_keep_n++;
    return slot;
}

/* The callee has returned. Unpin, then free on the spot if nothing parked the
 * clone — Rust drops the local at exactly this point — and otherwise leave it
 * for custom_keep_gc to reclaim once the continuation is done with it. */
static void custom_keep_settle(GameState *g, AbilityEffect *slot) {
    if (!slot) return;
    for (int i = 0; i < g_custom_keep_n; i++) {
        if (g_custom_keep[i] != slot) continue;
        g_custom_keep_pinned[i] = 0;
        if (!custom_keep_reachable(g, slot)) custom_keep_drop(i);
        return;
    }
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
           move_cards keeps count and every selection filter. Held in the
           keep-alive pool for the duration of the call: the mover parks this
           very pointer in queue.resume_eff when it has to ask a question, and
           the answer is read back after this frame is gone. */
        AbilityEffect *routed = custom_keep_push(g, effect);
        if (!routed) return 0;
        routed->action      = (char *)"move_cards";
        if (!routed->source)       routed->source      = (char *)"looked_at";
        if (!routed->destination)  routed->destination = (char *)"deck_top";
        rb_effect_move_cards(g, actor, routed);
        custom_keep_settle(g, routed);
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
        AbilityEffect *ge = custom_keep_push(g, effect);
        if (!ge) return 0;
        ge->text = (char *)text;
        if (!ge->target) ge->target = (char *)"self";
        rb_gain_ability(g, actor, ge);
        custom_keep_settle(g, ge);
        return 1;
    }

    /* 3) Parse gap: the parser could not type this effect. Nothing to run. */
    custom_log_parser_gap(g);
    return 1;
}
