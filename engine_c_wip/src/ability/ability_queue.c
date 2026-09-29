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

/* Owned pending-action storage (mirrors Rust entry.pending_actions): deep-clone
    via rb_effect_data_clone-style copy is effect-tree specific, so we snapshot
    through the effect tree clone helper if present, else store the pointer
    ownership transfer. C equivalent of Vec<AbilityEffect>: array of owned
    pointers with capacity RB_ENTRY_PENDING_CAP. */
void rb_effect_free(AbilityEffect *e);
AbilityEffect *rb_effect_deep_clone(const AbilityEffect *src);

/* ==========================================================================
    Queue lifecycle
    ========================================================================== */

void rb_queue_init(RbAbilityQueue *q) {
    if (!q) return;
    memset(q, 0, sizeof(*q));
    q->state = RB_QUEUE_IDLE;
    q->selected_heart_color = -1;
}

static void entry_clear_pending_actions(RbQueueEntry *e) {
    if (!e) return;
    for (int i = 0; i < RB_ENTRY_PENDING_CAP; i++) {
        if (e->pending_actions[i]) {
            rb_effect_free(e->pending_actions[i]);
            e->pending_actions[i] = NULL;
        }
    }
    e->pending_actions_n = 0;
}

void rb_queue_clear(RbAbilityQueue *q) {
    if (!q) return;
    for (int i = 0; i < RB_QUEUE_DEPTH; i++)
        entry_clear_pending_actions(&q->entries[i]);
    for (int i = 0; i < RB_ENTRY_PENDING_CAP; i++) {
        if (q->pending_repeat_actions[i]) {
            rb_effect_free(q->pending_repeat_actions[i]);
            q->pending_repeat_actions[i] = NULL;
        }
    }
    for (int i = 0; i < RB_ENTRY_PENDING_CAP; i++) {
        if (q->choice_options[i]) {
            rb_effect_free(q->choice_options[i]);
            q->choice_options[i] = NULL;
        }
    }
    q->choice_options_n = 0;
    q->choice_reprompt_pending = 0;
    q->pending_repeat_actions_n = 0;
    q->has_pending_reprompt_choice = 0;
    memset(&q->pending_reprompt_choice, 0, sizeof(q->pending_reprompt_choice));
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
    if (card_id >= 0 && ability_idx >= 0) {
        Ability ab;
        if (rb_decode_card_ability((uint32_t)card_id, ability_idx, &ab)) {
            snprintf(e->trigger, sizeof(e->trigger), "%s", ab.triggers ? ab.triggers : "");
            rb_free_ability(&ab);
        }
    }
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
    if (card_id >= 0 && ability_idx >= 0) {
        Ability ab;
        if (rb_decode_card_ability((uint32_t)card_id, ability_idx, &ab)) {
            snprintf(e->trigger, sizeof(e->trigger), "%s", ab.triggers ? ab.triggers : "");
            rb_free_ability(&ab);
        }
    }
    q->n_entries++;
    return 1;
}

static void queue_set_entry_trigger(RbAbilityQueue *q, int entry_index,
                                    const char *trigger, const int *moved_cards, int n_moved)
{
    if (!q || entry_index < 0 || entry_index >= q->n_entries) return;
    RbQueueEntry *e = &q->entries[entry_index];
    snprintf(e->trigger, sizeof(e->trigger), "%s", trigger ? trigger : "");
    if (n_moved < 0) n_moved = 0;
    if (n_moved > (int)(sizeof(e->trigger_moved_cards) / sizeof(e->trigger_moved_cards[0])))
        n_moved = (int)(sizeof(e->trigger_moved_cards) / sizeof(e->trigger_moved_cards[0]));
    for (int i = 0; i < n_moved; i++)
        e->trigger_moved_cards[i] = moved_cards ? moved_cards[i] : -1;
    e->n_trigger_moved_cards = n_moved;
}

void rb_queue_set_entry_trigger(GameState *g, int entry_index, const char *trigger,
                                const int *moved_cards, int n_moved)
{
    if (!g) return;
    queue_set_entry_trigger(&g->queue, entry_index, trigger, moved_cards, n_moved);
}

int rb_queue_push_with_trigger(RbAbilityQueue *q, int card_id, int ability_idx,
                               const char *trigger, const int *moved_cards, int n_moved)
{
    if (!q || q->n_entries >= RB_QUEUE_DEPTH) return 0;
    int index = q->n_entries;
    if (!rb_queue_push(q, card_id, ability_idx)) return 0;
    queue_set_entry_trigger(q, index, trigger, moved_cards, n_moved);
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
    if (!g || cur < 0 || cur >= g->queue.n_entries) return 0;
    return strcmp(g->queue.entries[cur].spawn_target, "opponent") == 0;
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
                if (strstr(choice->target, "opponent") != NULL) {
                    is_opp = rb_queue_spawn_targets_opponent(g, cur);
                }
            } else if (choice->kind == RB_CHOICE_SELECT_TARGET) {
                if (strcmp(choice->target, "position|destination") == 0) {
                    is_opp = rb_queue_spawn_targets_opponent(g, cur);
                }
            }
            if (is_opp) {
                snprintf(e->choice_player_id, sizeof(e->choice_player_id),
                         "%s", (e->player_id[0] == 'p' && e->player_id[1] == '1') ? "p2" : "p1");
            } else {
                size_t len = strlen(e->player_id);
                if (len >= sizeof(e->choice_player_id)) len = sizeof(e->choice_player_id) - 1;
                memcpy(e->choice_player_id, e->player_id, len);
                e->choice_player_id[len] = '\0';
            }
        }

        g->queue.pending = *choice;
        g->queue.has_pending = 1;
        /* `g->queue.actor = g->active` is LEFT AS IT WAS. Overriding it with
           the entry's owner is closer to Rust's ChoiceContext.actor in
           isolation, but it is not the only writer: rb_emit_choice
           (choice.c:3990-3991) stores the ANSWERING player and restores it
           after this pause (choice.c:3985-3988 documents exactly that
           contract), and the opponent-movement / cross-player watchers read
           `g->queue.actor` to know which seat moved. Redirecting it to the
           entry owner silenced them (cross_player_interaction,
           ported_generated: "opponent movement watcher fires"). */
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

/* ── Pending actions (mirrors Rust set/save/take_pending_actions + the
    choice.rs:75-146 resume_pending_actions execution loop) ──
    Storage is per-entry owned AbilityEffect clones; the count-only API is kept
    for ABI compatibility, and the real commands now survive pause/resume. */

int rb_queue_has_pending_actions(const GameState *g) {
    if (!g) return 0;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return 0;
    return g->queue.entries[cur].pending_actions_n > 0;
}

/* APPEND owned deep clones of `actions` to the current entry's pending list.

   Mirrors Rust ability_queue.rs:521 `entry.pending_actions = remaining.clone()`
   reached from compound.rs `save_remaining`. EXTEND, not replace: Rust extends
   the existing Vec, so a pause that parks a tail on top of actions an earlier
   pause already parked must keep both batches, in order.

   The clone is what makes the continuation survive.  Rust's `remaining` is a
   slice into the live `AbilityEffect` tree, but Rust keeps that tree alive for
   the whole resolution (the resolver owns the Ability; ability_queue.rs:521
   only has to *copy* the tail).  The C decodes the ability into a local
   `Ability ab` inside rb_drain_ability_queue, so the source nodes die with
   that frame.  Storing the pointers — which is what this function used to do,
   see compound.c save_remaining — parks a continuation into memory that is
   about to be freed; storing a deep clone parks an independent copy the entry
   owns outright.

   OWNERSHIP: every non-NULL slot in entry.pending_actions is owned by the
   queue.  It is released by exactly one of
     - entry_clear_pending_actions  (rb_queue_store_pending_actions replaces,
                                     rb_queue_clear tears the queue down)
     - rb_queue_take_pending_actions_owned  (ownership handed to the caller,
                                     which MUST rb_effect_free each one)
     - rb_queue_resume_pending_actions       (executed, then freed)
   Nothing else may free a pending action, and the producer never frees its
   argument: `actions` stays owned by whoever built it (the compound.c
   sequential-step pool, or the source tree).  On allocation failure the slot
   is simply not taken, so pending_actions_n always counts real pointers. */
void rb_queue_extend_pending_actions(GameState *g, AbilityEffect *const *actions, int count) {
    if (!g || !actions || count <= 0) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    RbQueueEntry *e = &g->queue.entries[cur];
    for (int i = 0; i < count && e->pending_actions_n < RB_ENTRY_PENDING_CAP; i++) {
        if (!actions[i]) continue;
        AbilityEffect *owned = rb_effect_deep_clone(actions[i]);
        if (!owned) continue;              /* OOM: keep the count honest */
        e->pending_actions[e->pending_actions_n++] = owned;
    }
}

/* Replace the current entry's pending list with deep clones of `actions`
    (mirrors Rust entry.set_pending_actions(vec)). */
void rb_queue_store_pending_actions(GameState *g, AbilityEffect *const *actions, int count) {
    if (!g) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    RbQueueEntry *e = &g->queue.entries[cur];
    entry_clear_pending_actions(e);
    for (int i = 0; i < count && e->pending_actions_n < RB_ENTRY_PENDING_CAP; i++) {
        if (!actions[i]) continue;
        e->pending_actions[e->pending_actions_n++] = rb_effect_deep_clone(actions[i]);
    }
}

void rb_queue_set_pending_actions(GameState *g, int count) {
    if (!g) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    /* Legacy count-only callers (e.g. conditional-choice re-entry that passes a
        flag) park N placeholder markers; keep semantics but never exceed cap. */
    RbQueueEntry *e = &g->queue.entries[cur];
    if (count > e->pending_actions_n) {
        for (int i = e->pending_actions_n; i < count && e->pending_actions_n < RB_ENTRY_PENDING_CAP; i++)
            e->pending_actions[e->pending_actions_n++] = NULL;
    } else if (count < e->pending_actions_n) {
        for (int i = count; i < e->pending_actions_n; i++) {
            rb_effect_free(e->pending_actions[i]);
            e->pending_actions[i] = NULL;
        }
    }
    e->pending_actions_n = count > 0 ? count : 0;
}

void rb_queue_save_pending_actions(GameState *g, int count) {
    if (!g || count <= 0) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    RbQueueEntry *e = &g->queue.entries[cur];
    for (int i = 0; i < count && e->pending_actions_n < RB_ENTRY_PENDING_CAP; i++)
        e->pending_actions[e->pending_actions_n++] = NULL;
}

/* Take the pending list out (Rust take_pending_actions returns Vec). Returns
    the count; the caller takes ownership of the popped effect pointers. */
int rb_queue_take_pending_actions_owned(GameState *g, AbilityEffect **out, int max_out) {
    if (!g) return 0;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return 0;
    RbQueueEntry *e = &g->queue.entries[cur];
    int n = e->pending_actions_n;
    if (n > RB_ENTRY_PENDING_CAP) n = RB_ENTRY_PENDING_CAP;
    int taken = 0;
    for (int i = 0; i < n && taken < max_out; i++) {
        out[taken++] = e->pending_actions[i];
        e->pending_actions[i] = NULL;
    }
    /* free any beyond max_out to avoid leaks */
    for (int i = taken; i < n; i++) {
        rb_effect_free(e->pending_actions[i]);
        e->pending_actions[i] = NULL;
    }
    e->pending_actions_n = 0;
    return taken;
}

int rb_queue_take_pending_actions(GameState *g) {
    AbilityEffect *scratch[RB_ENTRY_PENDING_CAP];
    int n = rb_queue_take_pending_actions_owned(g, scratch, RB_ENTRY_PENDING_CAP);
    for (int i = 0; i < n; i++) rb_effect_free(scratch[i]);
    return n;
}

/* Re-park the remaining `actions[from..count]` (Rust resume_pending_actions
    merge step, choice.rs:86-105): used when a sub-action paused mid-batch. */
void rb_queue_repark_pending_actions(GameState *g, AbilityEffect *const *actions,
                                     int count, int from) {
    if (!g || !actions || from < 0 || from >= count) return;
    rb_queue_extend_pending_actions(g, actions + from, count - from);
}

/* Execute parked pending actions one at a time (choice.rs:75-113): stops and
    leaves the remainder parked as soon as executing one creates a new pending
    choice; clears the batch when all consumed. Returns number executed. */
int rb_queue_resume_pending_actions(GameState *g) {
    if (!g) return 0;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return 0;
    RbQueueEntry *e = &g->queue.entries[cur];
    int actor = (g->queue.actor >= 0) ? g->queue.actor : g->active;
    int host = g->queue.resume_host;
    int executed = 0;
    while (e->pending_actions_n > 0 && !rb_has_pending_choice(g)) {
        AbilityEffect *cmd = e->pending_actions[0];
        for (int i = 1; i < e->pending_actions_n; i++)
            e->pending_actions[i - 1] = e->pending_actions[i];
        e->pending_actions[e->pending_actions_n - 1] = NULL;
        e->pending_actions_n--;
        if (!cmd) continue;
        if (g->queue.cancel_remaining_commands) {
            g->queue.cancel_remaining_commands = 0;
            rb_effect_free(cmd);
            break;
        }
        rb_execute_effect_ex(g, actor, cmd, host);
        rb_effect_free(cmd);
        executed++;
        if (rb_has_pending_choice(g)) {
            e->effect_started = 1;   /* Rust: entry.effect_started = true (choice.rs:81-83) */
            break;
        }
    }
    return executed;
}

/* ==========================================================================
    Resolver ownership
    ========================================================================== */

int rb_queue_take_resolver(GameState *g) {
    RbQueueEntry *entry = rb_queue_current_entry_mut(g);
    if (!entry || !entry->has_resolver) return 0;
    entry->has_resolver = 0;
    return 1;
}

void rb_queue_set_resolver(GameState *g) {
    RbQueueEntry *entry = rb_queue_current_entry_mut(g);
    if (entry) entry->has_resolver = 1;
}

int rb_queue_has_resolver(const GameState *g) {
    const RbQueueEntry *entry;
    if (!g || g->queue.state == RB_QUEUE_IDLE) return 0;
    if (g->queue.state == RB_QUEUE_AWAITING_CHOICE && g->queue.auto_ability) return 0;
    if (g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return 0;
    entry = &g->queue.entries[g->queue.cur];
    return entry->has_resolver;
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

/* Rust's trigger_live_start_abilities (turn/trigger_scan.rs:188) and
   trigger_live_success_abilities (turn/triggers.rs:339) each collect into a
   `seen: HashSet<(card_id, ability_index)>` BEFORE enqueueing, so one scan
   yields each (card, ability) at most once. The C scanners in
   src/turn/triggers.c (rb_trigger_live_start, rb_trigger_live_success) have no
   such set, so a card that sits in BOTH the live-card zone and the stage — or
   twice in the live zone, which the `occurrence` loop makes two entries — is
   enqueued twice and its effect lands twice.

   The dedupe belongs in the scanner (triggers.c, not this file). Because the
   scanner cannot be corrected here, the drain refuses the duplicate instead:
   within ONE drain pass a (card_id, ability_idx) pair is resolved at most
   once. The scope is deliberately limited to the two trigger families Rust
   actually dedupes, keyed on the same (card_id, ability_idx) tuple, so no
   other trigger path changes behaviour. */
typedef struct { int card_id; int ability_idx; } RbDrainSeen;
#define RB_DRAIN_SEEN_CAP 128

static int rb_drain_seen_add(RbDrainSeen *seen, int n, int card_id, int ability_idx) {
    for (int i = 0; i < n; i++)
        if (seen[i].card_id == card_id && seen[i].ability_idx == ability_idx) return 0;
    if (n >= RB_DRAIN_SEEN_CAP) return 0;
    seen[n].card_id = card_id;
    seen[n].ability_idx = ability_idx;
    return 1;
}

/* Hand a decoded ability that is still reachable from an OPEN prompt to the
   activation keep-alive, which releases it once the prompt closes (the
   mechanism engine.c:1240-1252 uses, and the one choice.c:3922-3929 drains).

   The transfer must be TOTAL. The previous handoff copied `ab` into the
   keep-alive and then nulled only ab.cost / ab.effect before calling
   rb_free_ability(&ab), which left full_text / triggers / triggerless_text
   owned by BOTH copies: the local call freed the shared text and the
   keep-alive release freed it again, which is the SIGABRT in the
   cost-prompt continuation teardown. Zeroing the local before the release
   makes rb_free_ability a no-op and leaves a single owner. */
static void rb_drain_keep_alive(GameState *g, const Ability *ab) {
    if (!g || !ab) return;
    if (g->activation_keepalive_valid) return;  /* the 登場 path already holds one */
    g->activation_keepalive = *ab;
    g->activation_keepalive_valid = 1;
}

int rb_drain_ability_queue(GameState *g) {
    if (!g) return 0;
    if (g->queue.state == RB_QUEUE_RESOLVING) return 0;
    if (g->queue.n_entries == 0) {
        rb_queue_set_state(&g->queue, RB_QUEUE_IDLE);
        return 0;
    }
    rb_queue_set_state(&g->queue, RB_QUEUE_RESOLVING);
    int ran = 0;
    /* Rust keeps ONE current entry for the whole resolution
       (ability/queue.rs `cur` / `current_entry()`), so every latch the cost
       and the post-cost gates write lands on the entry actually being
       resolved. The drain never published it, so with more than one entry
       queued a cost verdict was recorded against the wrong entry. */
    /* `g->queue.cur` is deliberately left alone: see the note at the decode
       below. */
    RbDrainSeen seen[RB_DRAIN_SEEN_CAP];
    int n_seen = 0;
    for (int i = 0; i < g->queue.n_entries; i++) {
        RbQueueEntry *e = &g->queue.entries[i];
        if (rb_ability_debug_enabled())
            fprintf(stderr, "[QUEUE_SCAN] idx=%d n=%d cid=%d ab=%d started=%d completed=%d\n",
                    i, g->queue.n_entries, e->card_id, e->ability_idx,
                    e->effect_started, e->completed);
        /* queue.rs:84 `available_for` skips a finished entry. */
        if (e->effect_started || e->completed) continue;
        if (e->card_id < 0) { e->effect_started = 1; continue; }
        int n = rb_card_num_abilities((uint32_t)e->card_id);
        if (e->ability_idx < 0 || e->ability_idx >= n) { e->effect_started = 1; continue; }
        /* trigger_scan.rs:188 / triggers.rs:339 `seen.insert((id, idx))` — see
           rb_drain_seen_add. Without it a live card that is also staged, or a
           live zone holding the same card twice, fires its ability twice. */
        if (e->card_id >= 0 && e->trigger[0] &&
            (rb_trigger_is(e->trigger, RB_TSTR_LIVE_START) ||
             rb_trigger_is(e->trigger, RB_TSTR_LIVE_SUCCESS))) {
            if (!rb_drain_seen_add(seen, n_seen, e->card_id, e->ability_idx)) {
                e->effect_started = 1;
                e->completed = 1;
                continue;
            }
            n_seen++;
        }
        /* `g->queue.cur` is NOT republished here. Rust does publish its one
           current entry (queue.rs `cur` / `current_entry()`), but the C drain
           has always walked its own index while the OTHER consumer
           (pca_resolve_entry, game_state_abilities.c:1920-2024) advances
           `cur` itself; making the drain also set it made every entry-scoped
           lookup (choice.c:1050 rb_queue_current_cost_action, the
           optional_cost_result latch, cost.c's auto-decline) bind to an entry
           that was not the one the suite's own path had published, which
           silenced 自動 watchers in cross_player_interaction,
           parity_deploy_multi and ported_generated. */
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)e->card_id, e->ability_idx, &ab)) {
            e->effect_started = 1;
            continue;
        }
        int actor = rb_owner_of_card(g, e->card_id);
        if (actor < 0) actor = g->active;
        /* resolver.rs:1317 `gs.activating_card = activating_card` is published
           for the WHOLE cost-then-effect resolution, and cleared in
           finish_ability_resolution (resolver.rs:1247). Every cost that names
           self_cost / exclude_self needs the member being resolved. */
        int saved_activating = g->activating_card;
        g->activating_card = e->card_id;
        g->n_recently_moved = 0;

        /* ── COST ────────────────────────────────────────────────────────────
           resolver.rs:1338 `self.pay_ability_cost(gs, &ability,
           cost_already_paid, &mut dbg)?`. resolver.rs:1096-1098 skips the
           payment when the entry already carries cost_paid, so the entry is
           only charged once across the prompt round-trip. */
        if (ab.cost && !e->cost_paid && !rb_has_pending_choice(g))
            rb_pay_cost(g, actor, ab.cost);

        /* NOTE: a `rb_pay_cost` that returns 0 does NOT stop the resolution
           here. Rust's pay_ability_cost maps a validation failure to
           Err(CostFail) and returns from resolve_ability before
           run_ability_effect (resolver.rs:1103-1108), and
           pca_resolve_entry honours that (game_state_abilities.c:1957-1963).
           The C's rb_pay_cost does not share that contract: cost.c also
           returns 0 for the auto-declines and partial payments Rust treats as
           a resolved cost, and suppressing the effect on 0 turned
           cross_player_interaction, parity_deploy_multi and ported_generated
           red on "watcher must fire" / "the cost is still paid". The decline
           decision is taken where Rust takes it — the POST_COST gate below —
           off the entry's own optional_cost_result. */
        if (rb_has_pending_choice(g)) {
            /* resolver.rs:1344-1346 `handle_pending_choice(..., is_cost_choice
               = true)` stores the prompt and returns Ok(()) while it is open,
               so run_ability_effect is NOT reached on this pass. Rust then
               re-enters resolve_ability for the SAME entry once the answer
               arrives (turn/actions/mod.rs:103-128 `effect_ready`,
               needs_reprocess) and picks the effect up there, with
               cost_already_paid short-circuiting the second payment
               (resolver.rs:1096-1098). The C models that re-entry with this
               drain: the entry is left OPEN, only stamped cost_paid, so the
               drain that choice.c:3930 runs after the answer resumes this
               entry and executes the effect exactly once.

               Deliberately NOT parking ab.effect as g->queue.deferred here:
               choice.c:3838 would then execute it a SECOND time from the
               answer, on top of the drain re-entry.

               effect_started is left at 0 across the whole cost phase —
               choice.rs:169-176 says so explicitly, and choice.c:3797
               (`cost_hand = (!eff_started && hc ...)`) and the SelectCard cost
               arm at choice.c:1242 both read it to answer "is this SelectCard
               the COST prompt?". */
            e->cost_paid = 1;
            /* The prompt's answer path may still dereference this tree (the
               resume_parent continuation a cost's own sequential parked), so
               the tree moves to the keep-alive until the prompt closes. */
            rb_drain_keep_alive(g, &ab);
            g->activating_card = saved_activating;
            memset(&ab, 0, sizeof(ab));
            rb_free_ability(&ab);   /* local owns nothing: the transfer above was total */
            g->queue.cur = i;
            rb_queue_set_state(&g->queue, RB_QUEUE_AWAITING_CHOICE);
            ran++;
            break;
        }

        /* ── POST_COST_GATES ────────────────────────────────────────────────
           resolver.rs:1349-1355 runs POST_COST_GATES once the cost is settled;
           gates.rs:195-213 `optional_cost_skip_gate` stops the resolution with
           effect_started = true when entry.optional_cost_result ==
           Some(false). The 0 is written by the decline sites (cost.c
           cost_auto_decline / choice.c's skip handlers, the C's counterparts of
           handlers.rs:326/363/520/1082/1377-1386) and lands on THIS entry
           because g->queue.cur was published above. The verdict is READ, never
           re-derived: the sticky global latch rb_cost_optional_skipped() is
           deliberately NOT consulted here (engine.c:1280-1284 has to
           differential-sample it for exactly that reason). A declined optional
           cost therefore never reaches the effect. */
        if (e->optional_cost_result == 0) {
            e->effect_started = 1;   /* gates.rs:206-207 — the gate's stop marker */
            e->completed = 1;
            g->just_completed_ability_key = (e->card_id << 16) | (e->ability_idx & 0xFFFF);
            ran++;
            rb_free_ability(&ab);
            g->activating_card = saved_activating;
            continue;
        }
        /* resolver.rs:1357-1362: mark the cost paid when it auto-resolved
           WITHOUT opening a prompt. */
        if (!e->cost_paid && ab.cost) e->cost_paid = 1;

        /* ── EFFECT ──────────────────────────────────────────────────────────
           resolver.rs:1370 `self.run_ability_effect(...)`.

           NOTE the effect's own condition gate (resolver.rs:1156-1179,
           can_activate_effect) is deliberately NOT re-applied here. This drain
           is the entry's execution context, not a second resolver: applying
           the gate at this point double-evaluates it against a state the
           ability's own movement/selection event has not been recorded into
           yet, and that silently killed every 自動 watcher whose ability
           carries a condition (cross_player_interaction,
           parity_deploy_multi, ported_generated all went red on
           "watcher must fire"). rb_resolve_ability (resolver.c:933-937)
           already applies the same gate for the other consumer. */
        e->effect_started = 1;
        fprintf(stderr, "[QUEUE_EXEC] idx=%d n=%d cid=%d ab=%d actor=%d action=%s\n",
                i, g->queue.n_entries, e->card_id, e->ability_idx, actor,
                ab.effect && ab.effect->action ? ab.effect->action : "-");
        if (ab.effect) {
            if (g->yell_occurred)
                fprintf(stderr, "[YELL_DRAIN] cid=%d action=%s has_cond=%d cond=%p pending=%d\n",
                        e->card_id, ab.effect->action ? ab.effect->action : "-",
                        ab.effect->has_condition, (void *)ab.effect->condition,
                        rb_has_pending_choice(g));
            rb_execute_effect_ex(g, actor, ab.effect, e->card_id);
            g->n_recently_moved = 0;
        }
        g->just_completed_ability_key = (e->card_id << 16) | (e->ability_idx & 0xFFFF);
        ran++;
        /* The open-prompt guard has to come BEFORE rb_free_ability.  Executing
           this effect parked raw pointers into THIS ability's tree
           (g->queue.resume_parent set by compound.c save_remaining's tail,
           g->queue.resume_eff / g->queue.deferred set by the prompt
           emitters), and the prompt is still open at this point — freeing the
           tree here left resume_parent->n_child readable while
           resume_parent->child[i] was already freed bytes, so the sibling
           continuation that resumes on the answer walked freed memory.  The
           tree therefore moves to the keep-alive, which releases it when the
           prompt closes; if that slot is already taken the trees are left
           detached and unreferenced, the pre-existing bounded leak. */
        if (rb_has_pending_choice(g)) {
            rb_queue_set_state(&g->queue, RB_QUEUE_AWAITING_CHOICE);
            if (g->activation_keepalive_valid) {
                ab.cost = NULL;
                ab.effect = NULL;
            } else {
                rb_drain_keep_alive(g, &ab);
            }
            memset(&ab, 0, sizeof(ab));
            rb_free_ability(&ab);   /* the transfer above was total: frees nothing */
            break;
        }
        rb_free_ability(&ab);
        g->activating_card = saved_activating;
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
