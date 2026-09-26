/* test_parity_queue_resume.c
 *
 * Parity coverage for the ability-queue / choice-suspension machinery:
 *   Rust twin  engine/src/ability_queue.rs  (AbilityQueue + AbilityQueueEntry)
 *   C port     engine_c_wip/src/ability/ability_queue.c
 *
 * Covers, in order:
 *   1. queue lifecycle + FIFO ordering          (ability_queue.rs:146-274, 661-793)
 *   2. pause_for_choice / resume_with_choice    (ability_queue.rs:276-397)
 *   3. complete_current / clear_completed      (ability_queue.rs:400-423)
 *   4. constant-context push/pop stacking       (ability_queue.rs:429-475)
 *   5. promote / set_current / pending_entries  (ability_queue.rs:483-573)
 *   6. deferred pending-action parking         (ability_queue.rs:494-523 + choice.rs:75-146)
 *   7. multi-step engine resolution with a suspended choice
 *   8. REGRESSION GUARD: skipping *every* choice in a chain must terminate.
 *
 * The last one is the guard for the `tests/baton_touch`
 * "skip stopped re-presenting a choice" loop: a queue that keeps a resolved or
 * skipped entry at the head re-presents its choice forever.
 */

#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <string.h>

/* ── queue entry points that exist in ability_queue.c but are not (yet)
      declared in rabuka.h.  Signatures copied verbatim from the definitions
      in src/ability/ability_queue.c; declared here so this suite does not
      need a header edit owned by another agent. ─────────────────────────── */
extern void        rb_queue_pause_for_auto_ability_choice(GameState *g, const RbChoice *choice);
extern void        rb_queue_resume_with_choice(GameState *g);
extern void        rb_queue_clear_completed(GameState *g);
extern const RbQueueEntry *rb_queue_get_entry(const GameState *g, int index);
extern int         rb_queue_pending_entries(const GameState *g, int *indices, int max_indices);
extern const char *rb_queue_entry_player_id(const GameState *g, int index);
extern void        rb_queue_push_constant_context(GameState *g, const char *player_id);
extern void        rb_queue_iter(const GameState *g, void (*fn)(const RbQueueEntry *, void *), void *ctx);
extern void        rb_queue_dump_state(const GameState *g, char *buf, size_t buf_sz);

static int failures;
static int assertions;

#define CHECK(condition, message) do { \
    assertions++; \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

#define CHECK_EQ(actual, expected, message) do { \
    assertions++; \
    int actual_value = (actual); \
    int expected_value = (expected); \
    if (actual_value != expected_value) { \
        fprintf(stderr, "FAIL: %s (got %d expected %d)\n", message, actual_value, expected_value); \
        failures++; \
    } else { \
        printf("ok: %s\n", message); \
    } \
} while (0)

/* ── helpers ───────────────────────────────────────────────────────────── */

static GameState fresh_queue_state(void)
{
    GameState g;
    memset(&g, 0, sizeof(g));
    g.winner = -1;
    g.turn = 1;
    g.active = 0;
    g.cheer_check_base = -1;
    rb_queue_init(&g.queue);
    return g;
}

/* Enqueue a distinguishable entry: card_id identifies the entry, ability_idx
   is -1 so the queue never touches the card database. */
static int push_entry(GameState *g, int card_id, const char *player_id)
{
    return rb_queue_enqueue(g, card_id, -1, -1, player_id, NULL);
}

static RbChoice make_target_choice(const char *target)
{
    RbChoice ch;
    memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_TARGET;
    snprintf(ch.target, sizeof(ch.target), "%s", target ? target : "");
    snprintf(ch.description, sizeof(ch.description), "%s", target ? target : "test");
    ch.allow_skip = 0;
    ch.actor = 0;
    return ch;
}

static int count_iter_cb(const RbQueueEntry *e, void *ctx)
{
    (void)e;
    (*(int *)ctx)++;
}

typedef struct { int completed; int total; int ids[RB_QUEUE_DEPTH]; } IterCtx;
static void collect_iter_cb(const RbQueueEntry *e, void *ctx)
{
    IterCtx *it = (IterCtx *)ctx;
    if (it->total < RB_QUEUE_DEPTH) it->ids[it->total] = e->card_id;
    it->total++;
    if (e->completed) it->completed++;
}

/* Drain a choice chain by skipping every prompt.  Returns the number of skips
   performed, or -1 if it did not converge within `limit` rounds.  This is the
   regression guard: a queue that retains a resolved/skipped entry re-presents
   its choice forever, so a non-terminating drain is the failure signal. */
static int drain_by_skip(GameState *g, int limit)
{
    int guard = 0;
    while (rb_has_pending_choice(g)) {
        if (guard >= limit) return -1;
        rb_resume_with_choice(g, -1);
        guard++;
    }
    return guard;
}

/* A look_at + select_cards pair; the select is optional so a skip is legal. */
static void fill_optional_look_select(AbilityEffect *parent)
{
    AbilityEffect *look = (AbilityEffect *)calloc(1, sizeof(AbilityEffect));
    AbilityEffect *select = (AbilityEffect *)calloc(1, sizeof(AbilityEffect));
    look->action = "look_at";
    look->source = "deck_top";
    look->count = 2;
    select->action = "select_cards";
    select->destination = "hand";
    select->count = 1;
    select->is_optional = 1;
    parent->action = "look_and_select";
    parent->look_action = look;
    parent->select_action = select;
}

static void free_optional_look_select(AbilityEffect *parent)
{
    rb_effect_free(parent->look_action);
    rb_effect_free(parent->select_action);
    parent->look_action = NULL;
    parent->select_action = NULL;
}

static void clear_zones(GameState *g, int pl)
{
    RbPlayer *P = &g->p[pl];
    P->deck.n = 0;
    P->hand.n = 0;
    P->discard.n = 0;
    P->live.n = 0;
    P->success.n = 0;
}

/* ══════════════════════════════════════════════════════════════════════
   1. lifecycle + ordering  (ability_queue.rs:147-272, tests 661-793)
   ══════════════════════════════════════════════════════════════════════ */

static void test_queue_starts_idle(void)
{
    GameState g = fresh_queue_state();
    CHECK(rb_queue_is_idle(&g), "queue_starts_idle: is_idle");
    CHECK_EQ(rb_queue_len(&g), 0, "queue_starts_idle: len == 0");
    CHECK(rb_queue_is_empty(&g), "queue_starts_idle: is_empty");
    CHECK(!rb_queue_is_waiting_for_choice(&g), "queue_starts_idle: no waiting choice");
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_IDLE, "queue_starts_idle: state == Idle");
}

static void test_enqueue_preserves_fifo_order(void)
{
    GameState g = fresh_queue_state();
    CHECK_EQ(push_entry(&g, 101, "p1"), 0, "fifo: first enqueue lands at index 0");
    CHECK_EQ(push_entry(&g, 102, "p1"), 1, "fifo: second enqueue lands at index 1");
    CHECK_EQ(push_entry(&g, 103, "p2"), 2, "fifo: third enqueue lands at index 2");
    CHECK_EQ(rb_queue_len(&g), 3, "fifo: len counts every entry");
    CHECK(!rb_queue_is_empty(&g), "fifo: queue is no longer empty");
    CHECK_EQ(rb_queue_get_entry(&g, 0)->card_id, 101, "fifo: entry 0 keeps its card");
    CHECK_EQ(rb_queue_get_entry(&g, 1)->card_id, 102, "fifo: entry 1 keeps its card");
    CHECK_EQ(rb_queue_get_entry(&g, 2)->card_id, 103, "fifo: entry 2 keeps its card");
    CHECK(rb_queue_get_entry(&g, 3) == NULL, "fifo: get_entry is bounds-checked");
    CHECK(strcmp(rb_queue_entry_player_id(&g, 2), "p2") == 0,
          "fifo: entry_player_id returns the enqueuing player");
}

static void test_start_next_skips_completed(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    push_entry(&g, 102, "p1");
    g.queue.entries[0].completed = 1;
    CHECK(rb_queue_start_next(&g), "start_next_skips_completed: start_next reports a start");
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_PAYING_COST,
             "start_next_skips_completed: state == PayingCost");
    CHECK_EQ(rb_queue_current_entry(&g), 1,
             "start_next_skips_completed: current entry is the first uncompleted one");
}

static void test_start_next_refuses_when_busy(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    CHECK(rb_queue_start_next(&g), "start_next_refuses_when_busy: first start_next succeeds");
    CHECK(!rb_queue_start_next(&g), "start_next_refuses_when_busy: second start_next is refused");
    CHECK_EQ(rb_queue_current_entry(&g), 0,
             "start_next_refuses_when_busy: current entry did not move");
}

static void test_complete_marks_entry_completed(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    rb_queue_start_next(&g);
    rb_queue_complete_current(&g);
    CHECK(rb_queue_get_entry(&g, 0)->completed,
          "complete_marks_entry_completed: entry 0 is completed");
    CHECK(rb_queue_is_idle(&g), "complete_marks_entry_completed: back to idle");
    CHECK_EQ(rb_queue_current_entry(&g), -1,
             "complete_marks_entry_completed: idle queue has no current entry");
}

static void test_sequential_complete_advances_index(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    push_entry(&g, 102, "p1");
    CHECK(rb_queue_start_next(&g), "sequential_complete: first start_next");
    CHECK_EQ(rb_queue_current_entry(&g), 0, "sequential_complete: current is entry 0");
    rb_queue_complete_current(&g);
    CHECK_EQ(rb_queue_current_entry(&g), 1, "sequential_complete: index advanced to 1");
    CHECK(rb_queue_start_next(&g), "sequential_complete: second start_next succeeds");
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_PAYING_COST,
             "sequential_complete: state == PayingCost for entry 1");
    CHECK_EQ(rb_queue_current_entry(&g), 1, "sequential_complete: current is entry 1");
}

static void test_all_completed_returns_idle(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    g.queue.entries[0].completed = 1;
    push_entry(&g, 102, "p1");
    rb_queue_start_next(&g);
    rb_queue_complete_current(&g);
    CHECK(!rb_queue_start_next(&g), "all_completed_returns_idle: nothing left to start");
    CHECK(rb_queue_is_idle(&g), "all_completed_returns_idle: queue is idle");
}

static void test_has_entry_with_id_detects_duplicates(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    CHECK(rb_queue_has_entry_with_id(&g, 101, -1), "has_entry_with_id detects an enqueued entry");
    CHECK(!rb_queue_has_entry_with_id(&g, 199, -1), "has_entry_with_id rejects an unknown entry");
}

/* ══════════════════════════════════════════════════════════════════════
   2. pause_for_choice / resume_with_choice
      (ability_queue.rs:276-397, tests 707-785)
   ══════════════════════════════════════════════════════════════════════ */

static void test_pause_and_resume_choice(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    rb_queue_start_next(&g);
    RbChoice ch = make_target_choice("test");
    rb_queue_pause_for_choice(&g, &ch);
    CHECK(rb_queue_is_waiting_for_choice(&g), "pause_and_resume_choice: is_waiting_for_choice");
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_AWAITING_CHOICE,
             "pause_and_resume_choice: state == WaitingForChoice");
    CHECK_EQ(g.queue.has_pending, 1, "pause_and_resume_choice: a choice is stored");
    CHECK(strcmp(g.queue.pending.target, "test") == 0,
          "pause_and_resume_choice: the stored choice is the one passed in");
    CHECK_EQ(rb_queue_current_entry(&g), 0, "pause_and_resume_choice: entry index is preserved");

    rb_queue_resume_with_choice(&g);
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_RESOLVING,
             "pause_and_resume_choice: resume moves to ExecutingEffect");
    CHECK(!rb_queue_is_waiting_for_choice(&g),
          "pause_and_resume_choice: resume clears the waiting state");
    CHECK_EQ(rb_queue_current_entry(&g), 0,
             "pause_and_resume_choice: resume re-enters the same entry");
}

static void test_pause_assigns_choice_player_id(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    rb_queue_start_next(&g);
    RbChoice ch = make_target_choice("test");
    rb_queue_pause_for_choice(&g, &ch);
    CHECK(strcmp(g.queue.entries[0].choice_player_id, "p1") == 0,
          "pause assigns choice_player_id = the entry's player");

    GameState g2 = fresh_queue_state();
    push_entry(&g2, 102, "p2");
    rb_queue_start_next(&g2);
    RbChoice ch2 = make_target_choice("test");
    rb_queue_pause_for_choice(&g2, &ch2);
    CHECK(strcmp(g2.queue.entries[0].choice_player_id, "p2") == 0,
          "pause assigns choice_player_id for p2-owned entries too");
}

static void test_pause_is_noop_when_already_awaiting(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    rb_queue_start_next(&g);
    RbChoice first = make_target_choice("first");
    rb_queue_pause_for_choice(&g, &first);
    RbChoice second = make_target_choice("second");
    rb_queue_pause_for_choice(&g, &second);
    CHECK(strcmp(g.queue.pending.target, "first") == 0,
          "pause_for_choice is a no-op while already WaitingForChoice");
    CHECK_EQ(g.queue.n_entries, 1, "pause_for_choice no-op does not grow the queue");
}

static void test_pause_from_idle_creates_dummy_entry(void)
{
    GameState g = fresh_queue_state();
    RbChoice ch = make_target_choice("orphan");
    rb_queue_pause_for_choice(&g, &ch);
    CHECK_EQ(g.queue.n_entries, 1, "pause while Idle pushes a dummy entry");
    CHECK_EQ(g.queue.entries[0].card_id, -1, "the dummy entry carries no card");
    CHECK_EQ(g.queue.entries[0].ability_idx, -1, "the dummy entry carries no ability index");
    CHECK_EQ(g.queue.entries[0].completed, 0, "the dummy entry starts uncompleted");
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_AWAITING_CHOICE,
             "the dummy entry parks the choice in WaitingForChoice");
    CHECK_EQ(g.queue.cur, 0, "the dummy entry becomes the current entry");
}

static void test_auto_ability_choice_returns_to_idle(void)
{
    GameState g = fresh_queue_state();
    g.queue.snapshot_requested = 0;
    RbChoice ch = make_target_choice("test");
    rb_queue_pause_for_auto_ability_choice(&g, &ch);
    CHECK(rb_queue_is_waiting_for_choice(&g), "auto ability choice is a waiting choice");
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_AWAITING_CHOICE,
             "auto ability choice parks in WaitingForChoice");
    CHECK_EQ(g.queue.snapshot_requested, 1,
             "pause_for_auto_ability_choice requests a history snapshot");
    CHECK_EQ(g.queue.auto_ability, 1, "auto ability choice is flagged auto");
    CHECK_EQ(g.queue.n_entries, 0,
             "pause_for_auto_ability_choice pushes no queue entry");
    rb_queue_resume_with_choice(&g);
    CHECK(rb_queue_is_idle(&g), "auto ability resume returns to idle");
    CHECK_EQ(g.queue.auto_ability, 0, "auto ability resume clears the auto flag");
}

static void test_resume_is_noop_when_not_waiting(void)
{
    GameState g = fresh_queue_state();
    rb_queue_resume_with_choice(&g);
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_IDLE,
             "resume_with_choice on an idle queue is a no-op");

    push_entry(&g, 101, "p1");
    rb_queue_start_next(&g);
    rb_queue_resume_with_choice(&g);
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_PAYING_COST,
             "resume_with_choice while PayingCost is a no-op");
}

/* ══════════════════════════════════════════════════════════════════════
   3. completion clearing  (ability_queue.rs:400-423, 758-769)
   ══════════════════════════════════════════════════════════════════════ */

static void test_clear_completed_drops_finished_entries(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    push_entry(&g, 102, "p1");
    push_entry(&g, 103, "p1");
    g.queue.entries[0].completed = 1;
    g.queue.entries[2].completed = 1;
    rb_queue_clear_completed(&g);
    CHECK_EQ(rb_queue_len(&g), 1, "clear_completed keeps only the uncompleted entry");
    CHECK_EQ(rb_queue_get_entry(&g, 0)->card_id, 102,
             "clear_completed compacts the survivors in order");
    CHECK_EQ(g.queue.entries[1].card_id, 0,
             "clear_completed leaves no stale duplicate of the moved entry");
}

static void test_clear_completed_resets_out_of_range_index(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    push_entry(&g, 102, "p1");
    g.queue.cur = 2;
    g.queue.entries[0].completed = 1;
    g.queue.entries[1].completed = 1;
    rb_queue_clear_completed(&g);
    CHECK_EQ(rb_queue_len(&g), 0, "clear_completed empties an all-completed queue");
    CHECK_EQ(g.queue.cur, 0,
             "clear_completed resets current_index when it runs past the end");
}

static void test_is_entry_available_and_pending_entries(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    push_entry(&g, 102, "p1");
    g.queue.entries[0].completed = 1;
    CHECK(!rb_queue_is_entry_available(&g, 0), "is_entry_available rejects a completed entry");
    CHECK(rb_queue_is_entry_available(&g, 1), "is_entry_available accepts a pending entry");
    CHECK(!rb_queue_is_entry_available(&g, 9), "is_entry_available is bounds-checked");
    int indices[RB_QUEUE_DEPTH];
    CHECK_EQ(rb_queue_pending_entries(&g, indices, RB_QUEUE_DEPTH), 1,
             "pending_entries counts only uncompleted entries");
    CHECK_EQ(indices[0], 1, "pending_entries reports the surviving index");
}

static void test_queue_iter_visits_every_entry(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    push_entry(&g, 102, "p1");
    push_entry(&g, 103, "p1");
    g.queue.entries[1].completed = 1;
    IterCtx ctx;
    memset(&ctx, 0, sizeof(ctx));
    rb_queue_iter(&g, collect_iter_cb, &ctx);
    CHECK_EQ(ctx.total, 3, "iter visits every entry");
    CHECK_EQ(ctx.completed, 1, "iter sees the completed flag");
    CHECK_EQ(ctx.ids[0], 101, "iter yields entries in queue order");
    CHECK_EQ(ctx.ids[2], 103, "iter yields the last entry");
    int n = 0;
    rb_queue_iter(&g, count_iter_cb, &n);
    CHECK_EQ(n, 3, "iter invokes the callback once per entry");
}

/* ══════════════════════════════════════════════════════════════════════
   4. constant-context stacking  (ability_queue.rs:429-475)
   ══════════════════════════════════════════════════════════════════════ */

static void test_constant_context_stacking(void)
{
    GameState g = fresh_queue_state();
    rb_queue_push_constant_context(&g, "p1");
    CHECK_EQ(rb_queue_len(&g), 1, "push_constant_context adds one entry");
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_RESOLVING,
             "push_constant_context moves to ExecutingEffect");
    CHECK_EQ(g.queue.entries[0].card_id, -1, "the constant context has no card");
    CHECK_EQ(g.queue.entries[0].choice_player_id[0], '\0',
             "the constant context starts with no choice owner");
    CHECK(strcmp(rb_queue_entry_player_id(&g, 0), "p1") == 0,
          "the constant context carries the requested player");

    rb_queue_push_constant_context(&g, "p2");
    CHECK_EQ(rb_queue_len(&g), 2, "a second constant context stacks on top");
    CHECK(strcmp(rb_queue_entry_player_id(&g, 0), "p1") == 0,
          "stacking preserves the outer constant context");
    CHECK(strcmp(rb_queue_entry_player_id(&g, 1), "p2") == 0,
          "the inner constant context carries its own player");
    CHECK_EQ(g.queue.cur, 1, "the inner constant context is current");

    rb_queue_pop_constant_context(&g);
    CHECK_EQ(rb_queue_len(&g), 1, "pop_constant_context removes only the inner context");
    CHECK(strcmp(rb_queue_entry_player_id(&g, 0), "p1") == 0,
          "pop_constant_context restores the outer context");
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_IDLE,
             "pop_constant_context returns the queue to idle");

    rb_queue_pop_constant_context(&g);
    CHECK_EQ(rb_queue_len(&g), 0, "popping the last context empties the queue");
}

/* ══════════════════════════════════════════════════════════════════════
   5. promotion / current-index control  (ability_queue.rs:525-573)
   ══════════════════════════════════════════════════════════════════════ */

static void test_promote_entry_moves_to_head(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    push_entry(&g, 102, "p1");
    push_entry(&g, 103, "p1");
    g.queue.cur = 0;
    rb_queue_promote_entry(&g, 2);
    CHECK_EQ(rb_queue_get_entry(&g, 0)->card_id, 103,
             "promote_entry(from_index=2) moves that entry to the head");
    CHECK_EQ(rb_queue_get_entry(&g, 1)->card_id, 101, "the head shifts down one slot");
    CHECK_EQ(rb_queue_get_entry(&g, 2)->card_id, 102, "the tail keeps its order");
    CHECK_EQ(g.queue.cur, 0, "promote_entry resets current_index to 0");
}

static void test_promote_entry_relative_to_current(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    push_entry(&g, 102, "p1");
    push_entry(&g, 103, "p1");
    g.queue.cur = 1;
    rb_queue_promote_entry(&g, 1);
    CHECK_EQ(rb_queue_get_entry(&g, 0)->card_id, 103,
             "promote_entry resolves from_index relative to current_index");
    CHECK_EQ(g.queue.cur, 0, "promote_entry resets current_index to 0");
    /* from_index 0 is a no-op (Rust ability_queue.rs:530) */
    GameState g2 = fresh_queue_state();
    push_entry(&g2, 201, "p1");
    push_entry(&g2, 202, "p1");
    g2.queue.cur = 0;
    rb_queue_promote_entry(&g2, 0);
    CHECK_EQ(rb_queue_get_entry(&g2, 0)->card_id, 201,
             "promote_entry(0) does not reorder the queue");
}

static void test_promote_by_abs_and_set_current(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    push_entry(&g, 102, "p1");
    push_entry(&g, 103, "p1");
    rb_queue_promote_entry_by_abs(&g, 2);
    CHECK_EQ(rb_queue_get_entry(&g, 0)->card_id, 103,
             "promote_entry_by_abs moves the absolute index to the head");
    CHECK_EQ(g.queue.cur, 0, "promote_entry_by_abs resets current_index");

    rb_queue_set_current_entry(&g, 2);
    CHECK_EQ(rb_queue_state(&g.queue), RB_QUEUE_IDLE, "set_current_entry does not change state");
    CHECK_EQ(rb_queue_get_entry(&g, 0)->card_id, 103,
             "set_current_entry does not reorder the queue");
    CHECK_EQ(g.queue.cur, 2, "set_current_entry sets current_index");
    rb_queue_set_current_entry(&g, 99);
    CHECK_EQ(g.queue.cur, 2, "set_current_entry is bounds-checked");
    rb_queue_promote_entry_by_abs(&g, 99);
    CHECK_EQ(g.queue.cur, 2, "promote_entry_by_abs is bounds-checked");
}

/* ══════════════════════════════════════════════════════════════════════
   6. deferred pending actions  (ability_queue.rs:494-523)
   ══════════════════════════════════════════════════════════════════════ */

static void test_pending_action_bookkeeping(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    rb_queue_start_next(&g);

    AbilityEffect a;
    AbilityEffect b;
    memset(&a, 0, sizeof(a));
    memset(&b, 0, sizeof(b));
    a.action = "look_at";
    b.action = "look_at";
    AbilityEffect *batch[2] = { &a, &b };
    CHECK(!rb_queue_has_pending_actions(&g), "a fresh entry has no pending actions");
    rb_queue_store_pending_actions(&g, batch, 2);
    CHECK(rb_queue_has_pending_actions(&g), "set_pending_actions parks the batch");
    CHECK_EQ(rb_queue_take_pending_actions(&g), 2,
             "take_pending_actions drains the whole batch");
    CHECK(!rb_queue_has_pending_actions(&g), "the batch is empty after take");
    CHECK_EQ(rb_queue_take_pending_actions(&g), 0, "taking twice yields nothing");

    AbilityEffect c;
    memset(&c, 0, sizeof(c));
    c.action = "look_at";
    AbilityEffect *more[1] = { &c };
    rb_queue_store_pending_actions(&g, more, 1);
    rb_queue_save_pending_actions(&g, more, 1);
    CHECK_EQ(rb_queue_take_pending_actions(&g), 2,
             "save_pending_actions appends to the parked batch");
    CHECK_EQ(rb_queue_set_pending_actions(&g, 0), 0,
             "set_pending_actions(0) is a no-op on an empty batch");
}

static void test_pending_actions_need_a_current_entry(void)
{
    GameState g = fresh_queue_state();
    AbilityEffect a;
    memset(&a, 0, sizeof(a));
    a.action = "look_at";
    AbilityEffect *batch[1] = { &a };
    rb_queue_store_pending_actions(&g, batch, 1);
    CHECK(!rb_queue_has_pending_actions(&g),
          "pending actions are only parked on the current entry");
    CHECK_EQ(rb_queue_take_pending_actions(&g), 0,
             "take_pending_actions yields nothing without a current entry");
}

static void test_resolver_ownership_round_trip(void)
{
    GameState g = fresh_queue_state();
    push_entry(&g, 101, "p1");
    CHECK(!rb_queue_has_resolver(&g), "a fresh entry has no resolver");
    rb_queue_start_next(&g);
    rb_queue_set_resolver(&g);
    CHECK(rb_queue_has_resolver(&g), "an attached resolver is visible");
    CHECK_EQ(rb_queue_take_resolver(&g), 1, "an attached resolver can be taken");
    CHECK(!rb_queue_has_resolver(&g), "a taken resolver is detached");
    CHECK_EQ(rb_queue_take_resolver(&g), 0, "a resolver cannot be taken twice");
    rb_queue_complete_current(&g);
    CHECK(!rb_queue_has_resolver(&g), "an idle queue exposes no resolver");
}

/* ══════════════════════════════════════════════════════════════════════
   7. multi-step engine resolution with a suspended choice
   ══════════════════════════════════════════════════════════════════════ */

static void test_optional_look_select_skip_terminates(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg.state, 0);
    int c0 = test_new_id(&tg, "PL!-sd1-010-SD");
    int c1 = test_new_id(&tg, "PL!-sd1-002-SD");
    if (c0 < 0 || c1 < 0) { CHECK(0, "optional look fixtures resolve"); return; }
    test_add_to_deck(&tg, c0);
    test_add_to_discard(&tg, c1);
    AbilityEffect parent;
    memset(&parent, 0, sizeof(parent));
    fill_optional_look_select(&parent);
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    CHECK(rb_has_pending_choice(&tg.state), "an optional look-and-select suspends on a choice");
    CHECK_EQ(rb_queue_state(&tg.state.queue), RB_QUEUE_AWAITING_CHOICE,
             "the engine pauses the queue in WaitingForChoice");
    int skips = drain_by_skip(&tg.state, 8);
    CHECK(skips >= 0, "skipping the optional look-and-select terminates");
    CHECK_EQ(skips, 1, "the optional look-and-select presents exactly one prompt");
    CHECK(!rb_has_pending_choice(&tg.state), "no choice remains after the skip");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "a skipped optional selection takes no card");
    free_optional_look_select(&parent);
}

static void test_select_routes_card_to_hand(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg.state, 0);
    int c0 = test_new_id(&tg, "PL!-sd1-010-SD");
    int c1 = test_new_id(&tg, "PL!-sd1-002-SD");
    if (c0 < 0 || c1 < 0) { CHECK(0, "look-select fixtures resolve"); return; }
    test_add_to_deck(&tg, c0);
    test_add_to_discard(&tg, c1);
    AbilityEffect parent;
    memset(&parent, 0, sizeof(parent));
    fill_optional_look_select(&parent);
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    CHECK(rb_has_pending_choice(&tg.state), "look-and-select offers the selection");
    rb_resume_with_choice(&tg.state, 0);
    CHECK(!rb_has_pending_choice(&tg.state), "answering the selection clears the choice");
    CHECK_EQ(tg.state.p[0].hand.n, 1, "answering the selection moves one card to hand");
    free_optional_look_select(&parent);
}

static void test_resume_pending_actions_stops_at_new_choice(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg.state, 0);
    int c0 = test_new_id(&tg, "PL!-sd1-010-SD");
    int c1 = test_new_id(&tg, "PL!-sd1-002-SD");
    if (c0 < 0 || c1 < 0) { CHECK(0, "parked look-select fixtures resolve"); return; }
    test_add_to_deck(&tg, c0);
    test_add_to_discard(&tg, c1);

    /* Build two independent optional look-and-select commands and park them on
       the current entry (mirrors Rust entry.pending_actions, ability_queue.rs:
       494-523, consumed one at a time by choice.rs:75-113). */
    AbilityEffect first;
    AbilityEffect second;
    memset(&first, 0, sizeof(first));
    memset(&second, 0, sizeof(second));
    fill_optional_look_select(&first);
    fill_optional_look_select(&second);

    push_entry(&tg.state, -1, "p1");
    rb_queue_start_next(&tg.state);
    AbilityEffect *batch[2] = { &first, &second };
    rb_queue_store_pending_actions(&tg.state, batch, 2);
    CHECK(rb_queue_has_pending_actions(&tg.state), "both commands are parked on the entry");

    int executed = rb_queue_resume_pending_actions(&tg.state);
    CHECK_EQ(executed, 1, "resume_pending_actions stops as soon as a choice appears");
    CHECK(rb_has_pending_choice(&tg.state), "the first parked command suspends on a choice");
    CHECK(rb_queue_has_pending_actions(&tg.state),
          "the un-run remainder stays parked on the entry");

    rb_resume_with_choice(&tg.state, -1);
    executed = rb_queue_resume_pending_actions(&tg.state);
    CHECK_EQ(executed, 1, "the second parked command runs after the first choice resolves");
    CHECK(rb_has_pending_choice(&tg.state), "the second command suspends on its own choice");
    rb_resume_with_choice(&tg.state, -1);
    CHECK(!rb_has_pending_choice(&tg.state), "the whole parked batch drains");
    CHECK(!rb_queue_has_pending_actions(&tg.state),
          "resume_pending_actions empties the batch once every command is consumed");
    free_optional_look_select(&first);
    free_optional_look_select(&second);
}

/* ══════════════════════════════════════════════════════════════════════
   8. REGRESSION GUARD — skipping every choice in a chain terminates
   ══════════════════════════════════════════════════════════════════════ */

static void test_skip_every_choice_in_sequential_chain_terminates(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg.state, 0);
    int cards[4];
    const char *nos[4] = { "PL!-sd1-010-SD", "PL!-sd1-002-SD",
                           "PL!-sd1-003-SD", "PL!-sd1-005-SD" };
    int ok = 1;
    for (int i = 0; i < 4; i++) {
        cards[i] = test_new_id(&tg, nos[i]);
        if (cards[i] < 0) ok = 0;
    }
    if (!ok) { CHECK(0, "sequential chain fixtures resolve"); return; }
    test_add_to_deck(&tg, cards[0]);
    test_add_to_discard(&tg, cards[1]);
    test_add_to_discard(&tg, cards[2]);
    test_add_to_discard(&tg, cards[3]);

    /* A `sequential` with two optional look-and-select children: each child
       emits its own choice, so the chain is two suspension round-trips. */
    AbilityEffect chain;
    AbilityEffect a;
    AbilityEffect b;
    memset(&chain, 0, sizeof(chain));
    memset(&a, 0, sizeof(a));
    memset(&b, 0, sizeof(b));
    fill_optional_look_select(&a);
    fill_optional_look_select(&b);
    chain.action = "sequential";
    chain.n_child = 2;
    chain.child[0] = &a;
    chain.child[1] = &b;

    rb_execute_effect_ex(&tg.state, 0, &chain, -1);
    CHECK(rb_has_pending_choice(&tg.state), "the sequential chain suspends on its first choice");

    int skips = drain_by_skip(&tg.state, 8);
    if (skips < 0) {
        char dump[2048];
        rb_queue_dump_state(&tg.state, dump, sizeof(dump));
        fprintf(stderr, "queue state at non-convergence:\n%s", dump);
    }
    CHECK(skips >= 0, "skipping every choice in a sequential chain terminates");
    CHECK(!rb_has_pending_choice(&tg.state), "no choice is left re-presenting itself");
    CHECK_EQ(tg.state.p[0].hand.n, 0, "a fully skipped chain takes no card");
    free_optional_look_select(&a);
    free_optional_look_select(&b);
}

static void test_skip_chain_does_not_grow_the_queue(void)
{
    TestGame tg;
    test_game_new(&tg);
    clear_zones(&tg.state, 0);
    int c0 = test_new_id(&tg, "PL!-sd1-010-SD");
    int c1 = test_new_id(&tg, "PL!-sd1-002-SD");
    if (c0 < 0 || c1 < 0) { CHECK(0, "queue-growth fixtures resolve"); return; }
    test_add_to_deck(&tg, c0);
    test_add_to_discard(&tg, c1);
    AbilityEffect parent;
    memset(&parent, 0, sizeof(parent));
    fill_optional_look_select(&parent);
    rb_execute_effect_ex(&tg.state, 0, &parent, -1);
    int entries_before = tg.state.queue.n_entries;
    int skips = drain_by_skip(&tg.state, 8);
    CHECK(skips >= 0, "the drain converges");
    CHECK_EQ(tg.state.queue.n_entries, entries_before,
             "a resolved/skipped choice does not add a queue entry");
    CHECK_EQ(tg.state.queue.cur, 0,
             "a resolved/skipped choice leaves the current entry in place");
    CHECK_EQ(rb_queue_get_entry(&tg.state, 0)->effect_started, 1,
             "the drained entry is marked effect_started so it is not re-run");
    free_optional_look_select(&parent);
}

static void test_baton_touch_skip_chain_terminates(void)
{
    /* The exact scenario from tests/test_baton_touch.c's
       drain_choices_by_skip: playing into a full lane offers a chain of
       prompts, every one of which is legal to decline. */
    TestGame game;
    test_game_new(&game);
    clear_zones(&game.state, 0);
    clear_zones(&game.state, 1);
    int target = test_id(&game, "PL!SP-bp2-011-R");
    int arriver = test_id(&game, "PL!HS-sd1-008-SD");
    int filler = test_id(&game, "PL!-sd1-010-SD");
    if (target < 0 || arriver < 0 || filler < 0) {
        CHECK(0, "baton touch fixtures resolve");
        return;
    }
    game.state.p[0].stage[1] = target;
    for (int i = 0; i < 10; i++)
        game.state.p[0].deck.cards[game.state.p[0].deck.n++] = filler;
    test_give_energy(&game, 25);
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = arriver;
    game.state.p[0].hand.cards[game.state.p[0].hand.n++] = filler;
    test_play_to_stage(&game, arriver, 1);

    int skips = drain_by_skip(&game.state, 32);
    if (skips < 0) {
        char dump[2048];
        rb_queue_dump_state(&game.state, dump, sizeof(dump));
        fprintf(stderr, "baton-touch queue state at non-convergence:\n%s", dump);
    }
    CHECK(skips >= 0, "baton touch: skipping every choice in the chain terminates");
    CHECK(!rb_has_pending_choice(&game.state),
          "baton touch: no choice is re-presented after a skip");
}

int main(void)
{
    if (rb_load("src") != 0) {
        fprintf(stderr, "FAIL: database load\n");
        return 1;
    }

    /* Queue FSM / bookkeeping — no card data required, but run after rb_load so
       rb_queue_enqueue's ability decode path is available if it is reached. */
    test_queue_starts_idle();
    test_enqueue_preserves_fifo_order();
    test_start_next_skips_completed();
    test_start_next_refuses_when_busy();
    test_complete_marks_entry_completed();
    test_sequential_complete_advances_index();
    test_all_completed_returns_idle();
    test_has_entry_with_id_detects_duplicates();
    test_pause_and_resume_choice();
    test_pause_assigns_choice_player_id();
    test_pause_is_noop_when_already_awaiting();
    test_pause_from_idle_creates_dummy_entry();
    test_auto_ability_choice_returns_to_idle();
    test_resume_is_noop_when_not_waiting();
    test_clear_completed_drops_finished_entries();
    test_clear_completed_resets_out_of_range_index();
    test_is_entry_available_and_pending_entries();
    test_queue_iter_visits_every_entry();
    test_constant_context_stacking();
    test_promote_entry_moves_to_head();
    test_promote_entry_relative_to_current();
    test_promote_by_abs_and_set_current();
    test_pending_action_bookkeeping();
    test_pending_actions_need_a_current_entry();
    test_resolver_ownership_round_trip();

    /* Engine-level suspension / resumption. */
    test_optional_look_select_skip_terminates();
    test_select_routes_card_to_hand();
    test_resume_pending_actions_stops_at_new_choice();
    test_skip_every_choice_in_sequential_chain_terminates();
    test_skip_chain_does_not_grow_the_queue();
    test_baton_touch_skip_chain_terminates();

    rb_unload();
    if (failures) {
        printf("PARITY QUEUE RESUME: %d/%d assertions failed\n", failures, assertions);
        return 1;
    }
    printf("ALL QUEUE RESUME CHECKS PASSED (%d assertions)\n", assertions);
    return 0;
}
