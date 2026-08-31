#include "rabuka.h"
#include <string.h>
#include <stdio.h>

/* ── Ability queue (mirrors engine/src/ability_queue.rs) ──
   The C queue model uses a flat array of entries with a state machine.
   Rust's AbilityQueueEntry carries ~1.9 KB of state; the C equivalent
   stores only the fields the engine actually uses (card_id, ability_idx,
   cost_paid, effect_started) and reads the rest from GameState on demand. */

int rb_queue_push(RbAbilityQueue *q, int card_id, int ability_idx) {
    if (!q || q->n_entries >= RB_QUEUE_DEPTH) return 0;
    q->entries[q->n_entries].card_id = card_id;
    q->entries[q->n_entries].ability_idx = ability_idx;
    q->entries[q->n_entries].cost_paid = 0;
    q->entries[q->n_entries].effect_started = 0;
    q->n_entries++;
    return 1;
}
void rb_queue_clear(RbAbilityQueue *q) { if (q) { memset(q, 0, sizeof(*q)); q->selected_heart_color = -1; } }
int rb_queue_has_pending(const RbAbilityQueue *q) { return q && q->n_entries > 0; }
RbQueueState rb_queue_state(const RbAbilityQueue *q) {
    return q ? q->state : RB_QUEUE_IDLE;
}
void rb_queue_set_state(RbAbilityQueue *q, RbQueueState s) {
    if (q) q->state = s;
}
void rb_choice_set_route(RbChoice *ch, RbChoiceRoute r) {
    if (ch) ch->route = r;
}

int rb_use_limit_reached(RbAbilityQueue *q, int card_id, int ability_idx, int limit, int cur_turn) {
    if (!q || limit <= 0) return 0;
    if (q->use_turn != cur_turn) return 0;
    int key = (card_id << 4) | (ability_idx & 0xF);
    for (int i = 0; i < q->n_uses; i++) if (q->use_keys[i] == key) return q->use_counts[i] >= limit;
    return 0;
}
void rb_record_use(RbAbilityQueue *q, int card_id, int ability_idx, int cur_turn) {
    if (!q) return;
    if (q->use_turn != cur_turn) { q->n_uses = 0; q->use_turn = cur_turn; }
    int key = (card_id << 4) | (ability_idx & 0xF);
    for (int i = 0; i < q->n_uses; i++) if (q->use_keys[i] == key) { q->use_counts[i]++; return; }
    if (q->n_uses < RB_USE_TRACK) { q->use_keys[q->n_uses] = key; q->use_counts[q->n_uses] = 1; q->n_uses++; }
}
int rb_use_count(RbAbilityQueue *q, int card_id, int ability_idx, int cur_turn) {
    if (!q || q->use_turn != cur_turn) return 0;
    int key = (card_id << 4) | (ability_idx & 0xF);
    for (int i = 0; i < q->n_uses; i++) if (q->use_keys[i] == key) return q->use_counts[i];
    return 0;
}

/* Return the player index that currently owns `cid` (searches stage/hand/
   energy/live/success/discard/deck), or -1. Mirrors Rust's player.contains_card. */
int rb_owner_of_card(const GameState *g, int cid) {
    if (!g || cid < 0) return -1;
    const RbBag *zones[6];
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        zones[0] = &P->hand;   zones[1] = &P->deck;   zones[2] = &P->discard;
        zones[3] = &P->energy;  zones[4] = &P->live;   zones[5] = &P->success;
        for (int z = 0; z < 6; z++) {
            const RbBag *Z = zones[z];
            for (int i = 0; i < Z->n; i++) if (Z->cards[i] == cid) return pl;
        }
        for (int s = 0; s < RB_STAGE_SIZE; s++) if (P->stage[s] == cid) return pl;
    }
    return -1;
}

/* ── Queue introspection (mirrors AbilityQueue methods) ── */

int rb_queue_is_idle(const GameState *g) {
    return g && g->queue.state == RB_QUEUE_IDLE;
}

int rb_queue_has_entry_with_id(const GameState *g, int card_id, int ability_idx) {
    if (!g) return 0;
    for (int i = 0; i < g->queue.n_entries; i++) {
        if (g->queue.entries[i].card_id == card_id && g->queue.entries[i].ability_idx == ability_idx)
            return 1;
    }
    return 0;
}

int rb_queue_current_entry(const GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return -1;
    return g->queue.cur;
}

int rb_queue_is_entry_available(const GameState *g, int idx) {
    if (!g || idx < 0 || idx >= g->queue.n_entries) return 0;
    return !g->queue.entries[idx].effect_started;
}

/* Start the next queued entry (mirrors start_next). Returns 1 if one was started. */
int rb_queue_start_next(GameState *g) {
    if (!g || g->queue.n_entries == 0) return 0;
    if (g->queue.state != RB_QUEUE_IDLE) return 0;
    while ((int)g->queue.cur < g->queue.n_entries) {
        if (!g->queue.entries[g->queue.cur].effect_started) {
            g->queue.state = RB_QUEUE_PAYING_COST;
            return 1;
        }
        g->queue.cur++;
    }
    return 0;
}

/* Complete the current entry (mirrors complete_current). */
void rb_queue_complete_current(GameState *g) {
    if (!g) return;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        g->queue.entries[g->queue.cur].completed = 1;
    }
    g->queue.state = RB_QUEUE_IDLE;
    g->queue.cur++;
}

/* Make a new queue entry (mirrors enqueue). Returns index or -1. */
int rb_queue_make_entry(GameState *g, int card_id, int ability_idx) {
    if (!g || g->queue.n_entries >= RB_QUEUE_DEPTH) return -1;
    int idx = g->queue.n_entries;
    g->queue.entries[idx].card_id = card_id;
    g->queue.entries[idx].ability_idx = ability_idx;
    g->queue.entries[idx].cost_paid = 0;
    g->queue.entries[idx].effect_started = 0;
    g->queue.n_entries++;
    return idx;
}

/* Promote entry to front of queue (mirrors promote_entry). */
void rb_queue_promote_entry(GameState *g, int from_index) {
    if (!g) return;
    int absolute = (int)g->queue.cur + from_index;
    if (absolute >= g->queue.n_entries || from_index == 0) return;
    RbQueueEntry tmp = g->queue.entries[absolute];
    for (int i = absolute; i > 0; i--) g->queue.entries[i] = g->queue.entries[i-1];
    g->queue.entries[0] = tmp;
    if ((int)g->queue.cur > absolute) g->queue.cur--;
    else g->queue.cur = 0;
}

/* Promote entry by absolute index (mirrors promote_entry_by_abs). */
void rb_queue_promote_entry_by_abs(GameState *g, int absolute) {
    if (!g || absolute >= g->queue.n_entries) return;
    RbQueueEntry tmp = g->queue.entries[absolute];
    for (int i = absolute; i > 0; i--) g->queue.entries[i] = g->queue.entries[i-1];
    g->queue.entries[0] = tmp;
    g->queue.cur = 0;
}

/* Set current entry index (mirrors set_current_entry). */
void rb_queue_set_current_entry(GameState *g, int absolute) {
    if (!g || absolute >= g->queue.n_entries) return;
    g->queue.cur = (uint8_t)absolute;
}

/* Check if current entry has pending actions (mirrors has_pending_actions). */
int rb_queue_has_pending_actions(const GameState *g) {
    if (!g) return 0;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return 0;
    return g->queue.entries[cur].pending_actions_n > 0;
}

/* Drain every queued ability whose effect has not started. */
int rb_drain_ability_queue(GameState *g) {
    if (!g) return 0;
    if (g->queue.state == RB_QUEUE_RESOLVING) return 0;
    if (g->queue.n_entries == 0) { rb_queue_set_state(&g->queue, RB_QUEUE_IDLE); return 0; }
    rb_queue_set_state(&g->queue, RB_QUEUE_RESOLVING);
    int ran = 0;
    for (int i = 0; i < g->queue.n_entries; i++) {
        RbQueueEntry *e = &g->queue.entries[i];
        if (e->effect_started) continue;
        e->effect_started = 1;
        int n = rb_card_num_abilities((uint32_t)e->card_id);
        if (e->ability_idx < 0 || e->ability_idx >= n) continue;
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)e->card_id, e->ability_idx, &ab)) continue;
        int actor = rb_owner_of_card(g, e->card_id);
        if (actor < 0) actor = g->active;
        if (ab.effect) {
            rb_execute_effect_ex(g, actor, ab.effect, e->card_id);
            g->n_recently_moved = 0;
        }
        rb_free_ability(&ab);
        g->just_completed_ability_key = (e->card_id << 16) | (e->ability_idx & 0xFFFF);
        ran++;
        if (rb_has_pending_choice(g)) {
            rb_queue_set_state(&g->queue, RB_QUEUE_AWAITING_CHOICE);
            break;
        }
    }
    if (rb_queue_state(&g->queue) == RB_QUEUE_RESOLVING)
        rb_queue_set_state(&g->queue, RB_QUEUE_DRAINING);
    if (g->queue.n_entries > 0 && !rb_has_pending_choice(g))
        rb_queue_set_state(&g->queue, RB_QUEUE_IDLE);
    return ran;
}
