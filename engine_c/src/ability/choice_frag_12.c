/* engine_c/src/ability/choice_frag_12.c
 *
 * Port of engine/src/ability/choice.rs fragments:
 *   - handle_double_baton_touch        (choice.rs:3109-3185)
 *   - handle_conditional_optional      (choice.rs:3187-3276)
 *
 * Rust `self` (AbilityResolver) is folded into GameState / the ability queue;
 * `self.gs` maps to `GameState *g`. The two boundary routines
 * rb_resolver_handle_selection_epilogue and rb_resolver_clear_choice_state_and_resume
 * are owned by the resolver (declared extern, NOT defined here) — they mirror
 * Rust clear_choice_state + resume_pending_actions and the in-between choice
 * bookkeeping (choice_card_no / optional_cost_result / route + set_pending_actions).
 */

#include "rabuka.h"
#include <string.h>
#include <stdint.h>
#include <stdatomic.h>

/* ── extern resolver helpers (defined in resolver, call by name) ── */
extern void rb_resolver_handle_selection_epilogue(GameState *g,
                                                   const char *selected,
                                                   int chose_yes);
extern void rb_resolver_clear_choice_state_and_resume(GameState *g);

/* ── in-fragment forward declarations ── */
static int  rb_resolver_parse_area(const char *name);
static void rb_resolver_record_baton_touch(GameState *g, int player_slot,
                                           int arriving_id);

/* ───────────────────────────────────────────────────────────────────────────
 * handle_double_baton_touch (choice.rs:3109-3185)
 * selected: "skip" | "left,center" | "left,right" | "center,right"
 * ───────────────────────────────────────────────────────────────────────────*/
void rb_resolver_handle_double_baton_touch(GameState *g, const char *selected) {
    if (!g || !selected) return;

    /* choice.rs:3114 — "skip" clears state and returns Ok */
    if (!strcmp(selected, "skip")) {
        rb_resolver_clear_choice_state_and_resume(g);
        return;
    }

    /* choice.rs:3119-3123 — split into exactly two area names */
    const char *comma = strchr(selected, ',');
    if (!comma || comma == selected ||
        selected[strlen(selected) - 1] == ',') {
        /* Invalid double baton selection (not exactly two areas) */
        rb_resolver_clear_choice_state_and_resume(g);
        return;
    }
    char a1[16], a2[16];
    size_t n1 = (size_t)(comma - selected);
    if (n1 >= sizeof(a1)) n1 = sizeof(a1) - 1;
    memcpy(a1, selected, n1);
    a1[n1] = '\0';
    strncpy(a2, comma + 1, sizeof(a2) - 1);
    a2[sizeof(a2) - 1] = '\0';

    int idx1 = rb_resolver_parse_area(a1);   /* choice.rs:3124 area_from_name */
    int idx2 = rb_resolver_parse_area(a2);
    if (idx1 > 2 || idx2 > 2) {              /* choice.rs:3134 */
        rb_resolver_clear_choice_state_and_resume(g);
        return;
    }

    /* choice.rs:3142-3155 — replace both members (move to waitroom) */
    int pl = 0; /* double baton choice is always for player1 (g->p[0]) */
    int *stage   = g->p[pl].stage;
    int *arrived = g->stage_arrived[pl];
    if (idx1 < 3 && stage[idx1] != RB_EMPTY_SLOT) {
        int old = stage[idx1];
        if (g->p[pl].discard.n < RB_MAX_ZONE)
            g->p[pl].discard.cards[g->p[pl].discard.n++] = old;
        stage[idx1] = RB_EMPTY_SLOT;
        arrived[idx1] = 0; /* Rule 9.6.2.1.2.1: member left stage, clear tracking */
    }
    if (idx2 < 3 && stage[idx2] != RB_EMPTY_SLOT) {
        int old = stage[idx2];
        if (g->p[pl].discard.n < RB_MAX_ZONE)
            g->p[pl].discard.cards[g->p[pl].discard.n++] = old;
        stage[idx2] = RB_EMPTY_SLOT;
        arrived[idx2] = 0;
    }

    /* choice.rs:3168-3174 — arriving = first non-empty stage slot (standalone
     * path; known misidentification limitation, see Rust comment block). */
    int arriving = RB_EMPTY_SLOT;
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        if (stage[i] != RB_EMPTY_SLOT) { arriving = stage[i]; break; }
    }

    /* choice.rs:3159-3176 — record_baton_touch called TWICE deliberately
     * (mirrors canonical double-baton path in phases.rs). Do NOT "fix" to once. */
    rb_resolver_record_baton_touch(g, pl, arriving);
    rb_resolver_record_baton_touch(g, pl, arriving);

    /* choice.rs:3183-3184 — clear_choice_state + resume_pending_actions */
    rb_resolver_clear_choice_state_and_resume(g);
}

/* ───────────────────────────────────────────────────────────────────────────
 * handle_conditional_optional (choice.rs:3187-3276)
 * selected: "1" | "yes" (accept) | anything else (decline)
 * ───────────────────────────────────────────────────────────────────────────*/
void rb_resolver_handle_conditional_optional(GameState *g, const char *selected) {
    if (!g) return;

    /* choice.rs:3198-3219 — per-queue-entry safety timeout against a runaway
     * optional-cost re-trigger loop. Reset when the active (card, ability) key
     * changes so it can never silently clear an unrelated game's queue. */
    static _Atomic unsigned CHOICE_CALLS  = 0;
    static _Atomic unsigned LAST_CARD     = UINT32_MAX;
    static _Atomic unsigned LAST_ABILITY  = UINT32_MAX;

    unsigned key_card = UINT32_MAX, key_ability = UINT32_MAX;
    if (g->queue.n_entries > 0 && g->queue.cur >= 0 &&
        g->queue.cur < g->queue.n_entries) {
        const RbQueueEntry *e = &g->queue.entries[g->queue.cur];
        key_card    = (unsigned)(e->card_id < 0 ? -1 : e->card_id);
        key_ability = (unsigned)e->ability_idx;
    }
    if (atomic_load(&LAST_CARD) != key_card ||
        atomic_load(&LAST_ABILITY) != key_ability) {
        atomic_store(&CHOICE_CALLS, 0);
        atomic_store(&LAST_CARD, key_card);
        atomic_store(&LAST_ABILITY, key_ability);
    }
    if (atomic_fetch_add(&CHOICE_CALLS, 1) > 200000) {
        /* abort rather than hang forever */
        rb_queue_clear(&g->queue);
        return;
    }

    int chose_yes = (selected &&
                     (!strcmp(selected, "1") || !strcmp(selected, "yes")));

    /* choice.rs:3244-3257 — record use_limit when the player chose to pay (NOT
     * when declined). Record against the ability ACTUALLY being resolved
     * (entry.ability_index), not the first ability on the card. */
    if (chose_yes && g->queue.n_entries > 0 && g->queue.cur >= 0 &&
        g->queue.cur < g->queue.n_entries) {
        const RbQueueEntry *e = &g->queue.entries[g->queue.cur];
        if (e->card_id >= 0)
            rb_record_ability_use(g, e->card_id, e->ability_idx);
    }

    /* choice.rs:3220-3273 — epilogue sets choice_card_no / optional_cost_result
     * (after clear), selects the conditional branch via route_conditional_branch,
     * and arms optional_moves_all_moved + set_pending_actions. */
    rb_resolver_handle_selection_epilogue(g, selected, chose_yes);

    /* choice.rs:3274 — clear_choice_state + resume_pending_actions */
    rb_resolver_clear_choice_state_and_resume(g);
}

/* ───────────────────────────────────────────────────────────────────────────
 * Local helpers
 * ───────────────────────────────────────────────────────────────────────────*/

/* choice.rs:3124 area_from_name */
static int rb_resolver_parse_area(const char *name) {
    if (!strcmp(name, "left"))   return 0;
    if (!strcmp(name, "center")) return 1;
    if (!strcmp(name, "right"))  return 2;
    return 999;
}

/* Mirror GameState::record_baton_touch (modifiers.rs:1299) — increments the
 * per-player count and pushes the arriving card id. */
static void rb_resolver_record_baton_touch(GameState *g, int player_slot,
                                           int arriving_id) {
    if (player_slot == 0)
        g->baton_touch_count_p1 += 1;
    else
        g->baton_touch_count_p2 += 1;
    if (arriving_id >= 0 &&
        g->n_baton_touch_arriving_card_ids < 16)
        g->baton_touch_arriving_card_ids[
            g->n_baton_touch_arriving_card_ids++] = arriving_id;
}
