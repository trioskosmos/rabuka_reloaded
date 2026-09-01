#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <ctype.h>

/* ==========================================================================
   Port of engine/src/ability_queue.rs
   ==========================================================================
   The Rust AbilityQueue owns Vec<AbilityQueueEntry> + QueueState + resolver.
   The C port stores the queue in GameState::queue (RbAbilityQueue / RbQueueEntry)
   and keeps resolver transient state in the queue's resume_* fields.  Fields
   that have no C equivalent (AbilityId, ConditionalChoice, Arc<Ability>,
   SmallVec, Box<AbilityResolver>) are either mapped to existing C fields or
   omitted with a comment.  */

/* ── Forward declarations ── */
static int rb_queue_spawn_targets_opponent(const GameState *g, int cur);

/* ==========================================================================
   Queue lifecycle
   ========================================================================== */

void rb_queue_init(RbAbilityQueue *q) {
    if (!q) return;
    memset(q, 0, sizeof(*q));
    q->state = RB_QUEUE_IDLE;
    q->selected_heart_color = -1;
}

void rb_queue_clear(RbAbilityQueue *q) {
    if (!q) return;
    memset(q, 0, sizeof(*q));
    q->state = RB_QUEUE_IDLE;
    q->selected_heart_color = -1;
}

/* ==========================================================================
   State queries
   ========================================================================== */

int rb_queue_is_idle(const GameState *g) {
    return g && g->queue.state == RB_QUEUE_IDLE;
}

int rb_queue_is_waiting_for_choice(const GameState *g) {
    return g && g->queue.state == RB_QUEUE_AWAITING_CHOICE;
}

RbQueueState rb_queue_state(const RbAbilityQueue *q) {
    return q ? q->state : RB_QUEUE_IDLE;
}

void rb_queue_set_state(RbAbilityQueue *q, RbQueueState s) {
    if (q) q->state = s;
}

/* ==========================================================================
   Entry accessors
   ========================================================================== */

int rb_queue_current_entry(const GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return -1;
    return g->queue.cur;
}

RbQueueEntry *rb_queue_current_entry_mut(GameState *g) {
    if (!g) return NULL;
    if (g->queue.state == RB_QUEUE_IDLE) return NULL;
    if (g->queue.state == RB_QUEUE_AWAITING_CHOICE && g->queue.auto_ability) return NULL;
    if (g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return NULL;
    return &g->queue.entries[g->queue.cur];
}

const RbQueueEntry *rb_queue_get_entry(const GameState *g, int index) {
    if (!g || index < 0 || index >= g->queue.n_entries) return NULL;
    return &g->queue.entries[index];
}

int rb_queue_len(const GameState *g) {
    return g ? g->queue.n_entries : 0;
}

int rb_queue_is_empty(const GameState *g) {
    return !g || g->queue.n_entries == 0;
}

int rb_queue_has_pending(const RbAbilityQueue *q) {
    return q && q->n_entries > 0;
}

void rb_queue_iter(const GameState *g, void (*fn)(const RbQueueEntry *, void *), void *ctx) {
    if (!g || !fn) return;
    for (int i = 0; i < g->queue.n_entries; i++) {
        fn(&g->queue.entries[i], ctx);
    }
}

/* ==========================================================================
   Enqueue / push
   ========================================================================== */

int rb_queue_enqueue(GameState *g, int card_id, int ability_idx,
                     int ability_index, const char *player_id,
                     const char *card_no) {
    if (!g || g->queue.n_entries >= RB_QUEUE_DEPTH) return -1;
    int idx = g->queue.n_entries;
    RbQueueEntry *e = &g->queue.entries[idx];
    memset(e, 0, sizeof(*e));
    e->card_id = card_id;
    e->ability_idx = ability_idx;
    e->cost_paid = 0;
    e->effect_started = 0;
    e->completed = 0;
    e->optional_cost_result = -1;
    e->cost_paid_index = 0;
    e->choice_card_no = 0;
    e->pending_actions_n = 0;
    e->triggering_member_id = -1;
    e->use_limit_recorded = 0;
    e->n_cond_cache = 0;
    if (player_id) snprintf(e->player_id, sizeof(e->player_id), "%s", player_id);
    if (card_no) {
        /* card_no is stored only in Rust; C port looks it up from the card db */
        (void)card_no;
    }
    g->queue.n_entries++;
    return idx;
}

int rb_queue_push(RbAbilityQueue *q, int card_id, int ability_idx) {
    if (!q || q->n_entries >= RB_QUEUE_DEPTH) return 0;
    RbQueueEntry *e = &q->entries[q->n_entries];
    memset(e, 0, sizeof(*e));
    e->card_id = card_id;
    e->ability_idx = ability_idx;
    e->cost_paid = 0;
    e->effect_started = 0;
    e->completed = 0;
    e->optional_cost_result = -1;
    e->cost_paid_index = 0;
    e->choice_card_no = 0;
    e->pending_actions_n = 0;
    e->n_cond_cache = 0;
    e->player_id[0] = '\0';
    q->n_entries++;
    return 1;
}

int rb_queue_make_entry(GameState *g, int card_id, int ability_idx) {
    if (!g || g->queue.n_entries >= RB_QUEUE_DEPTH) return -1;
    return rb_queue_enqueue(g, card_id, ability_idx, ability_idx, "p1", NULL);
}

/* ==========================================================================
   Ability processing
   ========================================================================== */

int rb_queue_start_next(GameState *g) {
    if (!g) return 0;
    if (g->queue.state != RB_QUEUE_IDLE) return 0;
    while (g->queue.cur < g->queue.n_entries) {
        if (!g->queue.entries[g->queue.cur].completed) {
            g->queue.state = RB_QUEUE_PAYING_COST;
            return 1;
        }
        g->queue.cur++;
    }
    return 0;
}

void rb_queue_complete_current(GameState *g) {
    if (!g) return;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        g->queue.entries[g->queue.cur].completed = 1;
    }
    g->queue.state = RB_QUEUE_IDLE;
    g->queue.cur++;
}

/* ==========================================================================
   Choice pausing / resumption
   ========================================================================== */

static int rb_queue_spawn_targets_opponent(const GameState *g, int cur) {
    /* The Rust queue checks entry.resolver.spawn_context.target == "opponent".
       The C queue does not carry a per-entry resolver.  Callers that know the
       spawn target must set entry.choice_player_id before calling
       rb_queue_pause_for_choice.  This helper is a best-effort fallback that
       inspects the global resume state; it returns 0 (conservative) when the
       resolver is not directly reachable.  */
    (void)g;
    (void)cur;
    return 0;
}

void rb_queue_pause_for_choice(GameState *g, const RbChoice *choice) {
    if (!g || !choice) return;
    if (g->queue.state == RB_QUEUE_AWAITING_CHOICE) return;

    int cur = g->queue.cur;
    if (cur >= 0 && cur < g->queue.n_entries) {
        RbQueueEntry *e = &g->queue.entries[cur];

        /* Universal default: every paused choice gets a choice_player_id */
        if (e->choice_player_id[0] == '\0') {
            /* G1/G3 opponent-routing fallback (mirrors AbilityQueue::pause_for_choice).
               The primary routing happens in the caller (actions.rs / choice.rs).
               Here we only apply the default if the caller did not set it.  */
            int is_opp = 0;
            if (choice->kind == RB_CHOICE_SELECT_CARD) {
                /* target field may contain "target_player_id:opponent" or similar */
                if (strstr(choice->target, "opponent") != NULL) {
                    is_opp = rb_queue_spawn_targets_opponent(g, cur);
                }
            }
            if (is_opp) {
                snprintf(e->choice_player_id, sizeof(e->choice_player_id),
                         "%s", (e->player_id[0] == 'p' && e->player_id[1] == '1') ? "p2" : "p1");
            } else {
                snprintf(e->choice_player_id, sizeof(e->choice_player_id),
                         "%s", e->player_id);
            }
        }

        g->queue.pending = *choice;
        g->queue.has_pending = 1;
        g->queue.actor = g->active;
        g->queue.state = RB_QUEUE_AWAITING_CHOICE;
        return;
    }

    /* Idle / Completed branch: create a dummy entry so the choice has a home */
    if (g->queue.n_entries < RB_QUEUE_DEPTH) {
        int idx = g->queue.n_entries;
        RbQueueEntry *e = &g->queue.entries[idx];
        memset(e, 0, sizeof(*e));
        e->card_id = -1;
        e->ability_idx = -1;
        e->optional_cost_result = -1;
        g->queue.n_entries++;
        g->queue.cur = idx;
        g->queue.pending = *choice;
        g->queue.has_pending = 1;
        g->queue.actor = g->active;
        g->queue.state = RB_QUEUE_AWAITING_CHOICE;
    }
}

void rb_queue_pause_for_auto_ability_choice(GameState *g, const RbChoice *choice) {
    if (!g || !choice) return;
    g->queue.pending = *choice;
    g->queue.has_pending = 1;
    g->queue.auto_ability = 1;
    g->queue.actor = g->active;
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;
}

void rb_queue_resume_with_choice(GameState *g) {
    if (!g) return;
    if (g->queue.auto_ability) {
        g->queue.auto_ability = 0;
        g->queue.has_pending = 0;
        g->queue.state = RB_QUEUE_IDLE;
    } else {
        g->queue.has_pending = 0;
        g->queue.state = RB_QUEUE_RESOLVING;
    }
}

/* ==========================================================================
   Queue maintenance
   ========================================================================== */

void rb_queue_clear_completed(GameState *g) {
    if (!g) return;
    int write = 0;
    for (int i = 0; i < g->queue.n_entries; i++) {
        if (!g->queue.entries[i].completed) {
            if (write != i) g->queue.entries[write] = g->queue.entries[i];
            write++;
        }
    }
    g->queue.n_entries = write;
    if (g->queue.cur > g->queue.n_entries) g->queue.cur = 0;
}

void rb_queue_push_constant_context(GameState *g, const char *player_id) {
    if (!g || g->queue.n_entries >= RB_QUEUE_DEPTH) return;
    int idx = g->queue.n_entries;
    RbQueueEntry *e = &g->queue.entries[idx];
    memset(e, 0, sizeof(*e));
    e->card_id = -1;
    e->ability_idx = -1;
    e->optional_cost_result = -1;
    if (player_id)
        snprintf(e->player_id, sizeof(e->player_id), "%s", player_id);
    g->queue.n_entries++;
    g->queue.cur = idx;
    g->queue.state = RB_QUEUE_RESOLVING;
}

void rb_queue_pop_constant_context(GameState *g) {
    if (!g) return;
    if (g->queue.n_entries > 0) {
        g->queue.n_entries--;
    }
    rb_queue_set_state(&g->queue, RB_QUEUE_IDLE);
}

/* ==========================================================================
   Promotion / reordering
   ========================================================================== */

void rb_queue_promote_entry(GameState *g, int from_index) {
    if (!g) return;
    int absolute = (int)g->queue.cur + from_index;
    if (absolute >= g->queue.n_entries || from_index == 0) return;
    RbQueueEntry tmp = g->queue.entries[absolute];
    for (int i = absolute; i > 0; i--)
        g->queue.entries[i] = g->queue.entries[i - 1];
    g->queue.entries[0] = tmp;
    if ((int)g->queue.cur > absolute)
        g->queue.cur--;
    else
        g->queue.cur = 0;
}

void rb_queue_promote_entry_by_abs(GameState *g, int absolute) {
    if (!g || absolute >= g->queue.n_entries) return;
    RbQueueEntry tmp = g->queue.entries[absolute];
    for (int i = absolute; i > 0; i--)
        g->queue.entries[i] = g->queue.entries[i - 1];
    g->queue.entries[0] = tmp;
    g->queue.cur = 0;
}

void rb_queue_set_current_entry(GameState *g, int absolute) {
    if (!g || absolute >= g->queue.n_entries) return;
    g->queue.cur = (uint8_t)absolute;
}

/* ==========================================================================
   Pending actions
   ========================================================================== */

int rb_queue_has_pending_actions(const GameState *g) {
    if (!g) return 0;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return 0;
    return g->queue.entries[cur].pending_actions_n > 0;
}

void rb_queue_set_pending_actions(GameState *g, int count) {
    if (!g) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    g->queue.entries[cur].pending_actions_n = count > 0 ? count : 0;
}

void rb_queue_save_pending_actions(GameState *g, int count) {
    if (!g || count <= 0) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    g->queue.entries[cur].pending_actions_n += count;
}

int rb_queue_take_pending_actions(GameState *g) {
    if (!g) return 0;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return 0;
    int n = g->queue.entries[cur].pending_actions_n;
    g->queue.entries[cur].pending_actions_n = 0;
    return n;
}

/* ==========================================================================
   Resolver stubs
   The C port keeps resolver transient state in GameState::queue resume_*
   fields, so per-entry Box<AbilityResolver> is not needed.  */

int rb_queue_take_resolver(GameState *g) {
    (void)g;
    return 0;
}

void rb_queue_set_resolver(GameState *g) {
    (void)g;
}

int rb_queue_has_resolver(const GameState *g) {
    (void)g;
    return 0;
}

/* ==========================================================================
   Availability / introspection
   ========================================================================== */

int rb_queue_is_entry_available(const GameState *g, int idx) {
    if (!g || idx < 0 || idx >= g->queue.n_entries) return 0;
    return !g->queue.entries[idx].completed;
}

int rb_queue_has_entry_with_id(const GameState *g, int card_id, int ability_idx) {
    if (!g) return 0;
    for (int i = 0; i < g->queue.n_entries; i++) {
        if (g->queue.entries[i].card_id == card_id &&
            g->queue.entries[i].ability_idx == ability_idx)
            return 1;
    }
    return 0;
}

int rb_queue_pending_entries(const GameState *g, int *indices, int max_indices) {
    if (!g || !indices) return 0;
    int count = 0;
    for (int i = 0; i < g->queue.n_entries && count < max_indices; i++) {
        if (!g->queue.entries[i].completed)
            indices[count++] = i;
    }
    return count;
}

const char *rb_queue_entry_player_id(const GameState *g, int index) {
    if (!g || index < 0 || index >= g->queue.n_entries) return NULL;
    return g->queue.entries[index].player_id[0] ? g->queue.entries[index].player_id : NULL;
}

/* ==========================================================================
   Debug / dump
   ========================================================================== */

void rb_queue_dump_state(const GameState *g, char *buf, size_t buf_sz) {
    if (!g || !buf || buf_sz == 0) return;
    int pos = 0;
    pos += snprintf(buf + pos, buf_sz - pos, "state=%d\n", g->queue.state);
    pos += snprintf(buf + pos, buf_sz - pos, "cur=%d\n", g->queue.cur);
    pos += snprintf(buf + pos, buf_sz - pos, "n_entries=%d\n", g->queue.n_entries);
    for (int i = 0; i < g->queue.n_entries && pos < (int)buf_sz; i++) {
        const RbQueueEntry *e = &g->queue.entries[i];
        pos += snprintf(buf + pos, buf_sz - pos,
            "  [%d] card=%d ab#%d player=%s completed=%d cost_paid=%d "
            "effect_started=%d optional_cost_result=%d pending_actions=%d\n",
            i, e->card_id, e->ability_idx, e->player_id,
            e->completed, e->cost_paid, e->effect_started,
            e->optional_cost_result, e->pending_actions_n);
    }
}

/* ==========================================================================
   Use-limit tracking
   ========================================================================== */

int rb_use_limit_reached(RbAbilityQueue *q, int card_id, int ability_idx,
                         int limit, int cur_turn) {
    if (!q || limit <= 0) return 0;
    if (q->use_turn != cur_turn) return 0;
    int key = (card_id << 4) | (ability_idx & 0xF);
    for (int i = 0; i < q->n_uses; i++)
        if (q->use_keys[i] == key) return q->use_counts[i] >= limit;
    return 0;
}

void rb_record_use(RbAbilityQueue *q, int card_id, int ability_idx, int cur_turn) {
    if (!q) return;
    if (q->use_turn != cur_turn) {
        q->n_uses = 0;
        q->use_turn = cur_turn;
    }
    int key = (card_id << 4) | (ability_idx & 0xF);
    for (int i = 0; i < q->n_uses; i++)
        if (q->use_keys[i] == key) { q->use_counts[i]++; return; }
    if (q->n_uses < RB_USE_TRACK) {
        q->use_keys[q->n_uses] = key;
        q->use_counts[q->n_uses] = 1;
        q->n_uses++;
    }
}

int rb_use_count(RbAbilityQueue *q, int card_id, int ability_idx, int cur_turn) {
    if (!q || q->use_turn != cur_turn) return 0;
    int key = (card_id << 4) | (ability_idx & 0xF);
    for (int i = 0; i < q->n_uses; i++)
        if (q->use_keys[i] == key) return q->use_counts[i];
    return 0;
}

/* ==========================================================================
   Owner lookup
   ========================================================================== */

int rb_owner_of_card(const GameState *g, int cid) {
    if (!g || cid < 0) return -1;
    const RbBag *zones[6];
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        zones[0] = &P->hand;   zones[1] = &P->deck;   zones[2] = &P->discard;
        zones[3] = &P->energy;  zones[4] = &P->live;   zones[5] = &P->success;
        for (int z = 0; z < 6; z++) {
            const RbBag *Z = zones[z];
            for (int i = 0; i < Z->n; i++)
                if (Z->cards[i] == cid) return pl;
        }
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            if (P->stage[s] == cid) return pl;
    }
    return -1;
}

/* ==========================================================================
   Queue drain
   ========================================================================== */

int rb_drain_ability_queue(GameState *g) {
    if (!g) return 0;
    if (g->queue.state == RB_QUEUE_RESOLVING) return 0;
    if (g->queue.n_entries == 0) {
        rb_queue_set_state(&g->queue, RB_QUEUE_IDLE);
        return 0;
    }
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

/* ==========================================================================
   Choice route helper
   ========================================================================== */

void rb_choice_set_route(RbChoice *ch, RbChoiceRoute r) {
    if (ch) ch->route = r;
}
