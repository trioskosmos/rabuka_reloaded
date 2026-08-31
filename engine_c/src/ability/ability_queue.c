#include "rabuka.h"
#include <string.h>
#include <stdio.h>

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

/* Drain every queued ability whose effect has not started, executing its
   effect tree with host_cid = the card that owns the ability (Rust's
   activating_card). Stops if an ability queues a pending choice so the host
   can resume the rest later. Returns count executed.
   Mirror ability_queue.rs drain + execute path. */
int rb_drain_ability_queue(GameState *g) {
    if (!g) return 0;
    /* Re-entrancy guard: an auto ability's effect can move a member, which re-enters
        rb_fire_auto → rb_drain_ability_queue. Without this guard the chain recurses
        unbounded and overflows the stack. The in-progress drain continues (and the
        outer call resumes after the choice/child yields), so queued abilities still
        run — just not simultaneously nested. */
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
            /* NOTE: clear_recently_moved_batch must happen AFTER the effect runs.
               The ability's condition (eval_movement) is re-checked at execute time
               and reads g->recently_moved; clearing it beforehand wipes the movement
               event recorded by push_movement_event, so move-triggered abilities would
               never satisfy their condition. Rust validates the condition at queue time
               and clears the batch scope only after execution. */
            rb_execute_effect_ex(g, actor, ab.effect, e->card_id);
            g->n_recently_moved = 0; /* batch-scope per queue entry */
        }
        rb_free_ability(&ab);
        /* Mirror Rust's just_completed_ability_key: record which ability just
            resolved so an auto-trigger scan can skip re-enqueueing it (prevents
            an auto ability from recursively re-triggering itself). */
        g->just_completed_ability_key = (e->card_id << 16) | (e->ability_idx & 0xFFFF);
        ran++;
        if (rb_has_pending_choice(g)) {
            /* yield to host; resume re-enters the loop (FSM stays AwaitingChoice
               until the choice resolves and the queue is drained again). */
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
