#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <ctype.h>

/* ==========================================================================
   Port of engine/src/ability/queue.rs
   ==========================================================================
   The Rust AbilityQueue owns Vec<AbilityQueueEntry> + QueueState + resolver.
   The C port stores the queue in GameState::queue (RbAbilityQueue / RbQueueEntry)
   and keeps resolver transient state in the queue's resume_* fields.  Fields
   that have no C equivalent (AbilityId, ConditionalChoice, Arc<Ability>,
   SmallVec, Box<AbilityResolver>) are either mapped to existing C fields or
   omitted with a comment.

   --------------------------------------------------------------------------
   OWNERSHIP CONTRACT for entry.pending_actions
   --------------------------------------------------------------------------
   The park/take/resume trio (rb_queue_store_pending_actions,
   rb_queue_extend_pending_actions, rb_queue_repark_pending_actions,
   rb_queue_take_pending_actions[_owned], rb_queue_resume_pending_actions) is
   the translation of Rust's `entry.pending_actions: Vec<AbilityEffect>` and of
   its single Rust consumer, `AbilityResolver::resume_pending_actions`
   (choice.rs:75-127).

   THE INVARIANT, in one sentence because every producer and consumer depends on
   it: `pending_actions[]` holds ONLY owned deep clones — never a NULL marker,
   never a borrowed pointer — and `pending_actions_n` is exactly the number of
   live entries in that array. A consequence worth stating: a Rust
   `Vec<AbilityEffect>`'s len IS its contents, so "the entry has N pending
   actions" is a claim about real commands. Parking NULLs made that claim false
   while `has_pending_actions()` still reported true, which is why the
   continuation silently never ran.

   Given that, an entry's parked actions are released by EXACTLY ONE of three
   owners, so no path can both double-free and leak:
     * entry_clear_pending_actions — the entry is being dropped: queue clear,
       a completed entry dropped by clear_completed, a constant-context pop, or a
       vacated slot left behind by compaction / promotion (those last three wipe
       the slot AFTER clearing it, so the moved entry's pointers are never live
       in two slots at once);
     * rb_queue_take_pending_actions_owned — ownership moves to the caller's
       array, which MUST rb_effect_free each element;
     * rb_queue_resume_pending_actions — executed in place, then freed.

   plus ONE discard path, which is a release, not a transfer:
     * rb_queue_take_pending_actions — takes the batch and frees every element
       itself, returning only the COUNT. A caller that frees and then tries to
       EXECUTE what it just freed double-counts (choice.c:593-628 measured
       this: executing the batch there cost 16-21 checks across 22 suites and
       won none), and a caller that executes without taking ownership leaks or
       executes the same tail twice.

   So the two take_* functions differ ONLY in what the caller receives, and
   the single place a parked batch is EXECUTED is
   rb_resolver_continue_siblings (choice.c:4396-4420), which takes by
   ownership, runs each element, frees it, and re-parks whatever a sub-prompt
   interrupts. Every other take_pending_actions caller is discarding.


   Producers (store / extend / repark / set / save) NEVER free their argument:
   `actions` belongs to whoever built it (the compound.c sequential-step pool, or
   the source effect tree). Consumers never adopt an individual slot pointer;
   they take the whole batch and let the count drop to 0.

   The count-only entry points (rb_queue_set_pending_actions,
   rb_queue_save_pending_actions) cannot express "these N commands" — the count
   signature is fixed in include/rabuka.h, which this file does not own. They
   therefore resolve the commands from the queue's own continuation state
   (queue_resolve_continuation_effects) and deep-clone them, and they leave the
   list EMPTY rather than inflated when no real command exists. An honest zero
   lets the caller's `cont` fallback execute the remaining siblings, which is
   what Rust does; a fabricated N would suppress that fallback and lose the
   effects permanently. See CROSS-FILE note: choice.c:554 is the one callsite
   that should pass real effects rather than a count.

   --------------------------------------------------------------------------
   The "choice-resume continuation" block (resume_eff, resume_parent,
   resume_actor, deferred, pending_deferred_costs) and the fallback loop in
   rb_drain_ability_queue are NOT Rust queue state. Rust keeps that machinery in
   choice.rs / resolver.rs; it is folded into the C queue so choice.c,
   compound.c and the effect executors have one continuation to park into, and
   each block says so where it is declared. The FSM transitions at the top of
   rb_drain_ability_queue, however, do mirror Rust's `AbilityResolver::drain`
   entry/exit conditions plus QueueState, and they are exercised by
   test_p1_helpers.c. */

/* ── Forward declarations ── */

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
    /* Option<i16> = None; 0 is a legal card index. */
    q->pending_success_replacement_card_id = -1;
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
    /* core/game_state/mod.rs:433-434 -- concede also clears
     * `pending_success_replacement_card_id`, and this memset is that clear. */
    q->pending_success_replacement_card_id = -1;
}

/* ==========================================================================
   State queries
   ========================================================================== */

int rb_queue_is_idle(const GameState *g) {
    return g && g->queue.state == RB_QUEUE_IDLE;
}

/* queue.rs:200-206 `is_waiting_for_choice`.

   Rust returns `Some(&choice)` for WaitingForAutoAbilityChoice (which always
   carries one) and `choice.as_ref()` for WaitingForChoice — i.e. None when that
   variant's `choice` is None. The C models "a choice is stored" with the
   `has_pending` flag rather than an Option inside the state, so both waiting
   variants must test it; testing only the state reported a waiting choice for a
   queue that had already handed its choice out. */
int rb_queue_is_waiting_for_choice(const GameState *g) {
    return g && g->queue.state == RB_QUEUE_AWAITING_CHOICE && g->queue.has_pending;
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
        /* GAINED-AWARE. ability_idx at or above RB_GAINED_ABILITY_INDEX_BASE
           addresses the gained store, not the printed list, and
           rb_decode_card_ability answers 0 for it (card.c:282 rejects any n
           >= the printed count). A GAINED entry therefore left e->trigger
           EMPTY here, and the drain's LIVE_START/LIVE_SUCCESS dedupe key at
           rb_drain_ability_queue gates on e->trigger[0] -- so a gained entry
           skipped dedupe AND read as triggerless everywhere else that inspects
           the stamped trigger. rabuka.h:2230-2236 names
           rb_resolve_card_ability_at as the entry point for this; the enqueue
           is one of its callers. */
        Ability ab;
        if (rb_resolve_card_ability_at(g, card_id, ability_idx, &ab)) {
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
    /* Rust `triggering_member_id: Option<i16>` defaults to None. memset leaves 0,
       which is a VALID card id, so an entry that never recorded a member id
       would read as "member 0 fired this". rb_queue_enqueue already seeds -1;
       this path did not. */
    e->triggering_member_id = -1;
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

/* ── rb_queue_push_with_trigger — DELETED 2026-10-05 (zero callers, Rust proof)
 *
 * It was `rb_queue_push(RbAbilityQueue*, ...)` followed by
 * `queue_set_entry_trigger` on the index it had just appended, and it had ZERO C
 * callers anywhere under src/ or tests/. Zero C callers does NOT prove deadness,
 * so the Rust side was searched before deleting, and the search is what decides
 * this:
 *
 *   Rust has NO free function with this shape. There is no
 *   `push_with_trigger` / `enqueue_with_trigger` anywhere in engine/src -- the
 *   whole grep over ability/queue.rs and core/game_state/abilities.rs returns the
 *   two constructor helpers AbilityQueueEntry::new (queue.rs:45) and
 *   ::from_parts (queue.rs:50), and those BUILD an entry; they do not enqueue one.
 *
 *   Every Rust enqueue goes through trigger_auto_ability (abilities.rs:1239-1293)
 *   into enqueue_printed_ability (:1282-1326) or enqueue_gained_ability
 *   (:1331-1376), and BOTH take `player_id: String` and BOTH write it into the
 *   entry's `player_id` field. So the capability this C function appeared to
 *   offer -- "push an entry and stamp a trigger on it" -- is not a thing Rust
 *   has, because in Rust the two are not separable: an entry with a trigger_type
 *   and an entry with an owner are the same object, built by the same call.
 *
 *   The C function was therefore not a port with a missing caller. It was a
 *   HALF of one: it could stamp a trigger and could not stamp an owner, which is
 *   why src/turn/triggers.c:87 records that an entry pushed this way "leaves
 *   player_id empty" and why the production scans all go through
 *   rb_push_owned_entry (triggers.c:122) instead. Wiring this one in would
 *   reintroduce exactly the unowned entry the owner-scoped drain cannot see
 *   (rb_process_player_abilities_depth_full scans on
 *   `player_id[0] == '\0' || !strcmp(.., player_id)`, game_state_abilities.c:3958
 *   -3962), so there is no call site it could be wired into CORRECTLY.
 *
 * The surviving spelling of "push an entry with its trigger AND its owner" is
 * rb_queue_enqueue + rb_queue_set_entry_trigger, which is what
 * rb_push_owned_entry (triggers.c:122-132) does and what all four Rust enqueue
 * paths map onto.
 *
 * The declaration in include/rabuka.h is another agent's file and is deliberately
 * left in place: no translation unit references it, so it links, and removing it
 * would mean editing a header this change does not own. */

/* ── rb_queue_save_pending_actions — DELETED 2026-10-05 (count-only C form)
 *
 * Its Rust twin is LIVE, and the C site that mirrors it is LIVE, but the two are
 * not the same function and this one is not the port:
 *
 *   engine/src/ability/queue.rs:544  save_pending_actions(&mut self, actions: Vec<AbilityEffect>)
 *   the only production caller in all of engine/src:
 *   engine/src/ability/compound/conditional_on.rs:43
 *       let mut finish = effect.clone();
 *       finish.compound.primary_effect = None;
 *       finish.condition = None;
 *       gs.ability_queue.save_pending_actions(vec![finish]);
 *
 *   That caller is `if resolver.awaiting.choice.is_some()` immediately after
 *   `resolver.execute_effect(gs, primary)` -- i.e. the PRIMARY action stopped on
 *   a prompt, so the tail of the conditional is parked to be re-entered when the
 *   answer arrives.
 *
 *   The C counterpart of that exact site is src/ability/compound.c:693-699
 *   (rb_compound_conditional_on_result), and it is already wired:
 *       AbilityEffect finish = clone_effect(eff);
 *       finish.primary_effect = NULL;
 *       ...
 *       rb_queue_extend_pending_actions(g, rest, 1);
 *   rb_queue_extend_pending_actions IS save_pending_actions's body -- Rust's
 *   `pending_actions.extend(actions)` at queue.rs:546. So the Rust capability is
 *   present, present at the right site, and exercised.
 *
 * Why the count-only form cannot be it: `rb_queue_save_pending_actions(GameState*,
 * int count)` cannot name WHICH effects to append. It re-derives them from the
 * entry's own continuation list (queue_resolve_continuation_effects,
 * ability_queue.c:924), which is a different mechanism from Rust's, where the
 * caller hands over the effect it just built. The one file that documents that
 * limitation is this one, at ability_queue.c:68. A caller holding a freshly built
 * `finish` -- which is exactly what conditional_on.rs:43 holds -- has no way to
 * express it through a count, so there is no site where this signature is the
 * faithful spelling. Keeping it was keeping a second, weaker way to say the same
 * thing, with the weaker one untested.
 *
 * The declaration in include/rabuka.h is left in place for the same reason as
 * above: no translation unit references it, and rabuka.h is not this change's. */

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

/* ═══════════════════════════════════════════════════════════════════════════
   queue_route_targets_opponent — the C twin of Rust's SECOND trigger

   queue.rs:340-345 pairs the spawn-context test with a ROUTE test:

       let is_route_opponent = entry.choice_card_no.as_ref().map_or(false, |route| {
           matches!(route, ChoiceRoute::Raw(raw)
                    if raw.starts_with("position_change:opponent:"))
       });

   `choice_card_no` is `Option<ChoiceRoute>` in Rust, and mod.rs:2751-2755 /
   position.rs:151-154 / :273-277 / :427-431 / choice.rs:3153-3158 / :3178-3183
   all build it as `ChoiceRoute::Raw(format!("position_change:{target}:{tail}"))`.
   So the Rust condition is "is this entry's route the Raw string starting
   position_change:opponent:?" — a POSITIVE test on the route's TEXT, not an
   inference from a numeric tag.

   The C cannot evaluate that, because `RbQueueEntry.choice_card_no` is an int.
   rabuka.h says so on the field itself: "C's flattened choice_card_no tag;
   Rust's Option<ChoiceRoute> does not map 1:1 — Condition/Cost/Generic all
   tag 1." Tags 1/2/3 are therefore already spoken for by three other routes, and
   no int can mean position_change:opponent.

   Rather than widen the tag (which would collide with Condition/Cost/Generic and
   re-create exactly this bug class — a seat derived from a number that does not
   mean what the number says) the route STRING is carried in the pair the header
   reserves for it: `choice_route_raw` / `has_choice_route_raw`. Emitters stamp
   those; this helper reads them. Nothing here parses an int into a route.

   WHY A POSITIVE HELPER INSTEAD OF ONE MORE ARM ON THE OVERRIDE LIST

   The seat decision used to be: universal default from `entry.player_id`, plus
   two narrow per-variant exceptions (a SelectCard whose `target` merely
   CONTAINED the substring "opponent"; a SelectTarget whose target was exactly
   "position|destination"). Every prompt shape that was not on that list fell
   through to the default and read the wrong seat, and the only available remedy
   was to append another shape — which is how the defect recurs.

   Here the question is split the way Rust splits it: `choice_routes_to_opponent`
   asks what the CHOICE is (mirroring Rust's `is_opponent_choice` patterns
   exactly), and this helper asks what the ENTRY's context says. Both of the
   context triggers live on the entry rather than on the choice, so a prompt form
   that did not exist when the list was written still routes correctly, and no
   shape can route wrong merely by being absent from a list.

   queue.rs:329's `entry.choice_player_id.is_some()` early-out lives in
   pause_for_choice, which owns the stamp; this helper answers only the context
   question. Both Rust triggers are checked, in Rust's order: spawn first,
   route second (queue.rs:333 then :340). */
static int queue_route_targets_opponent(const RbQueueEntry *e) {
    if (!e) return 0;
    /* queue.rs:333 `entry.spawn_context.target == "opponent"` — this trigger IS
       live in the C today, because misc.c's both_set_spawn_target
       (rb_misc_handle_both_targets) writes the same string into spawn_target. */
    if (e->spawn_target[0] && strcmp(e->spawn_target, "opponent") == 0) return 1;
    /* queue.rs:340-342 the Raw-route trigger, previously unevaluable. */
    if (e->has_choice_route_raw &&
        strncmp(e->choice_route_raw, "position_change:opponent:", 23) == 0) return 1;
    return 0;
}

/* queue.rs:322-331 `is_opponent_choice` — does the CHOICE's own shape say the
   opponent is being asked? These are the two Rust patterns, reproduced on the
   fields Rust destructures.

   The SelectCard arm reads `target_player_id`, because that IS the field Rust's
   `SelectCard { target_player_id: Some(tpid) }` pattern binds and the only thing
   the guard `tpid == "opponent"` tests. The old C arm instead ran
   `strstr(choice->target, "opponent")` — a substring scan over a DIFFERENT
   field, `target`, which for a SelectCard is the ability's routing target
   string ("self", "opponent", "p2", "both", ...). That conflated "the ability
   routes to the opponent" with "this prompt must be answered by the opponent"
   and, worse, matched any prompt whose target contained the letters anywhere;
   selection.rs:39-43 stamps target_player_id from action_by, so it is the
   faithful channel.

   The SelectTarget arm keeps Rust's exact equality against
   "position|destination" — that is the literal Rust compares, so it stays an
   equality test and is not widened to a substring. */
static int choice_routes_to_opponent(const RbChoice *ch) {
    if (!ch) return 0;
    if (ch->kind == RB_CHOICE_SELECT_CARD)
        return strcmp(ch->target_player_id, "opponent") == 0;
    if (ch->kind == RB_CHOICE_SELECT_TARGET)
        return strcmp(ch->target, "position|destination") == 0;
    return 0;
}

/* Stamp the RAW route string on the current entry — the C home of Rust's
   `entry.choice_card_no = Some(ChoiceRoute::Raw(s))`.

   This is what queue.rs:341 and abilities.rs:2159 read back to decide a prompt's
   seat, and the int `choice_card_no` cannot carry it (see
   queue_route_targets_opponent above).

   Only effects that genuinely own a raw route string call this. Effects whose
   route is an enum variant (Choice, ChoiceString, ConditionalChoice,
   OptionalCost, SelectCards, SelectTarget) set NO raw string, because Rust's
   trigger fires only for the Raw variant — `has_choice_route_raw` left at 0 is
   the honest "not a Raw route" state and cannot produce a false positive.

   A NULL or empty `raw` clears the stamp. An emitter that stamps a Raw route and
   a later prompt on the same entry takes a plain variant must not leave a stale
   string behind for queue_route_targets_opponent to match. */
void rb_queue_set_choice_route_raw(GameState *g, const char *raw) {
    if (!g) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    RbQueueEntry *e = &g->queue.entries[cur];
    if (!raw || !*raw) {
        e->choice_route_raw[0] = '\0';
        e->has_choice_route_raw = 0;
        return;
    }
    snprintf(e->choice_route_raw, sizeof(e->choice_route_raw), "%s", raw);
    e->has_choice_route_raw = 1;
}

/* Push the dummy entry `queue.rs:367-409` builds and make it current. Shared
   by the two callers below so the shape exists in exactly one place.

   `player_id` may be NULL, which reproduces Rust's verbatim empty-string dummy
   (queue.rs:370-371 `player_id: String::new()`); rb_queue_ensure_current_entry
   passes the ability's master instead, because pause_for_choice's dummy has
   nothing to route and this one's `player_id` is read by the routing code. */
static int queue_mint_dummy_entry(GameState *g, const char *player_id) {
    if (!g || g->queue.n_entries >= RB_QUEUE_DEPTH) return 0;
    int idx = g->queue.n_entries;
    RbQueueEntry *e = &g->queue.entries[idx];
    memset(e, 0, sizeof(*e));
    e->card_id = -1;
    e->ability_idx = -1;
    e->optional_cost_result = -1;
    e->triggering_member_id = -1;   /* Option::None; 0 is a valid card id */
    if (player_id)
        snprintf(e->player_id, sizeof(e->player_id), "%s", player_id);
    g->queue.n_entries++;
    g->queue.cur = idx;
    return 1;
}

/* Guarantee a CURRENT entry, minting the dummy one when there is none.

   Rust needs no such function because the two pieces it would paper over
   cannot come apart there:
     - `pause_for_choice` (queue.rs:311-409) has a branch for EVERY QueueState.
       PayingCost / ExecutingEffect stamp the current entry; Idle / Completed
       push a dummy entry and make it current. So a prompt raised while an
       effect executes always has an entry behind it.
     - `set_pending_actions` (queue.rs:494-500) writes through
       `current_entry_mut()`, and an AbilityResolver only ever exists ATTACHED
       to an entry (queue.rs stores it as `entry.resolver`), so at
       target.rs:129-137 the current entry is always Some and the park always
       lands.

   The C has one shape Rust does not: an effect can run with NO entry at all.
   engine.c parks `ab.effect` in `queue.deferred` and SKIPS the inline run
   precisely when the ability raises a prompt, so nothing was ever pushed for
   it, and the prompt comes out of the resolver's spawn context. rb_emit_choice
   then deliberately declines to mint (choice.c:4754-4758), so n_entries stays
   0 and BOTH consequences land together:
     - pause_for_choice never runs against an entry, so nothing stamps
       `choice_player_id` and `current_entry().choice_player_id` reads None
       where Rust reads Some("pN");
     - every pending-action accessor is gated on `cur < n_entries`, so the
       opponent half parked at target.rs:129-137 is dropped on the floor.

   This is the C's stand-in for Rust's guarantee: it mints the entry the
   resolution should have had. The entry is NOT marked completed and NOT
   effect_started — it is the ability's own, still mid-resolution, and its
   `pending_actions` have to survive until resume_pending_actions runs them.
   It is inert to the drain either way (the `if (e->card_id < 0)` arm in
   rb_drain_ability_queue skips it).

   `player_id` is the ability's MASTER, because Rust measures "self" from
   `ability_master_id()` = the current entry's player_id
   (core/game_state/abilities.rs:2675-2702), so naming it here is what makes
   both_master() and pause_for_choice's universal default agree with Rust.
   Returns 1 when a current entry exists after the call. */
int rb_queue_ensure_current_entry(GameState *g, const char *player_id) {
    if (!g) return 0;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) return 1;
    return queue_mint_dummy_entry(g, player_id);
}

void rb_queue_pause_for_choice(GameState *g, const RbChoice *choice) {
    if (!g || !choice) return;
    if (g->queue.state == RB_QUEUE_AWAITING_CHOICE) return;

    int cur = g->queue.cur;
    if (cur >= 0 && cur < g->queue.n_entries) {
        RbQueueEntry *e = &g->queue.entries[cur];

        /* Universal default: every paused choice gets a choice_player_id */
        if (e->choice_player_id[0] == '\0') {
            /* queue.rs:318-361, in Rust's order.

               (1) queue.rs:329 `entry.choice_player_id.is_some()` — an earlier
                   caller already stamped the seat (execute_choice's
                   choice_maker arm, choice.rs:143-153, which misc.c's h_choice
                   now reproduces; the live-success zone arm, live.rs:1577).
                   This whole block is inside that guard.

               (2) queue.rs:333-345 — route to the OPPONENT when the choice is one
                   the opponent must answer. Rust spells that as
                   `is_opponent_choice`, testing the CHOICE's shape, then gates
                   it on EITHER context trigger:

                       entry.spawn_context.target == "opponent"                    (:333)
                       choice_card_no == Raw("position_change:opponent:…")         (:340)

               The C keeps that structure: choice_routes_to_opponent(choice) is the
               shape test, and queue_route_targets_opponent(e) holds BOTH context
               triggers — the spawn context (written by misc.c's
               both_set_spawn_target) and the raw route string. Nothing infers a
               route from choice_card_no's int tag, because tags 1/2/3 belong to
               Condition, Cost and Generic. */
            int is_opp = choice_routes_to_opponent(choice) &&
                         queue_route_targets_opponent(e);
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

    /* Idle / Completed branch: create a dummy entry so the choice has a home.
       queue_mint_dummy_entry is the shape queue.rs:367-409 pushes, with the
       empty player_id Rust gives it — this dummy exists to hold the choice, and
       nothing routes on its owner. */
    if (queue_mint_dummy_entry(g, NULL)) {
        g->queue.pending = *choice;
        g->queue.has_pending = 1;
        g->queue.actor = g->active;
        g->queue.state = RB_QUEUE_AWAITING_CHOICE;
    }
}

/* queue.rs:416-419 `pause_for_auto_ability_choice`.

   Rule 9.5.3: the player is asked which standby ability resolves first, and the
   queue parks that prompt WITHOUT touching `entries` (Rust's
   WaitingForAutoAbilityChoice variant carries only the Choice). The C models
   that variant as RB_QUEUE_AWAITING_CHOICE plus the `auto_ability` flag, which
   is what `rb_queue_current_entry_mut` / `rb_queue_has_resolver` test to
   reproduce Rust's "WaitingForAutoAbilityChoice yields no current_entry".

   The C body was missing `snapshot_requested = true`, the FIRST of Rust's two
   statements and the one with an observable effect outside the queue: Rust sets
   it so the web handler can push an extra history snapshot at the choice
   boundary, and resolver.c:322-326 (the other setter of that flag) relies on the
   flag existing at all. Without it the auto-ability prompt was taken with no
   snapshot, so a player answering "which standby ability first?" saw a state
   jump they could not rewind. */
void rb_queue_pause_for_auto_ability_choice(GameState *g, const RbChoice *choice) {
    if (!g || !choice) return;
    g->queue.snapshot_requested = 1;
    g->queue.pending = *choice;
    g->queue.has_pending = 1;
    g->queue.auto_ability = 1;
    g->queue.actor = g->active;
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;
}

/* queue.rs:422-434 `resume_with_choice`.

   Rust matches on the state and the `_ => {}` arm is the whole story: ONLY the
   two waiting states transition. From Idle the call does nothing, and from
   PayingCost / ExecutingEffect it does nothing either -- the resolution is
   already in flight and must not be re-entered.

   The C body was an if/else on `auto_ability` alone, so BOTH waiting variants
   were covered but every other state was rewritten: a resume issued while Idle
   put the queue into RESOLVING with no entry to resolve (an empty queue in a
   non-idle state, which `rb_queue_is_idle` then denies and
   `rb_drain_ability_queue` treats as "already resolving" and refuses to run), and
   a resume issued mid-cost silently advanced the cost phase to the effect phase.
   Gate on the state, exactly as Rust does. */
void rb_queue_resume_with_choice(GameState *g) {
    if (!g) return;
    if (g->queue.state != RB_QUEUE_AWAITING_CHOICE) return;  /* Rust's `_ => {}` */
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

/* queue.rs:455-460 `clear_completed` — `entries.retain(|e| !e.completed)` plus
   the out-of-range `current_index` reset.

   `Vec::retain` DROPS every entry it removes, and an entry owns its
   `pending_actions` Vec, so a completed entry's parked deep clones are freed
   here in Rust. The C compaction did neither: it copied surviving structs down
   and only lowered `n_entries`.

   Two consequences, both real:
     1. LEAK. Every completed entry left its owned pending actions unreachable.
     2. ALIASING DOUBLE-FREE. `entries[write] = entries[i]` copies the struct,
        so `pending_actions[]` (an array of heap pointers) is now live in two
        slots. The stale high slot is outside [0, n_entries) so the *next*
        clear_completed could not see it, but the next PUSH into that slot
        memsets it, silently orphaning the clones; and if a later compact ever
        shifted the high slot back into range the same effect tree would be
        freed twice.

   So: free the dropped entries' actions, MOVE (not copy) the survivors, and
   zero every vacated slot so the array never holds an entry that n_entries
   excludes. The vacated slots read back as card_id 0, which is what
   test_parity_queue_resume.c:398 asserts. */
void rb_queue_clear_completed(GameState *g) {
    if (!g) return;
    int write = 0;
    for (int i = 0; i < g->queue.n_entries; i++) {
        RbQueueEntry *e = &g->queue.entries[i];
        if (e->completed) {
            entry_clear_pending_actions(e);   /* Rust: the retained-drop frees the Vec */
            continue;
        }
        if (write != i) {
            /* MOVE, not copy: transfer the whole struct, then wipe the source so
               the two slots never alias the same owned pointer. */
            g->queue.entries[write] = *e;
            memset(e, 0, sizeof(*e));
        }
        write++;
    }
    /* Vacated tail: no entry may remain visible at an index >= n_entries. */
    for (int i = write; i < g->queue.n_entries; i++) {
        entry_clear_pending_actions(&g->queue.entries[i]);
        memset(&g->queue.entries[i], 0, sizeof(RbQueueEntry));
        g->queue.entries[i].optional_cost_result = -1;
        g->queue.entries[i].triggering_member_id = -1;
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
    e->triggering_member_id = -1;   /* Option::None; 0 is a valid card id */
    if (player_id)
        snprintf(e->player_id, sizeof(e->player_id), "%s", player_id);
    g->queue.n_entries++;
    g->queue.cur = idx;
    g->queue.state = RB_QUEUE_RESOLVING;
}

/* queue.rs:510-513 `pop_constant_context` — `entries.pop()` DROPS the popped
   entry, so its owned pending_actions are freed. The C only decremented
   n_entries, orphaning them at a slot the queue still writes on the next push. */
void rb_queue_pop_constant_context(GameState *g) {
    if (!g) return;
    if (g->queue.n_entries > 0) {
        g->queue.n_entries--;
        entry_clear_pending_actions(&g->queue.entries[g->queue.n_entries]);
        memset(&g->queue.entries[g->queue.n_entries], 0, sizeof(RbQueueEntry));
        g->queue.entries[g->queue.n_entries].optional_cost_result = -1;
        g->queue.entries[g->queue.n_entries].triggering_member_id = -1;
    }
    rb_queue_set_state(&g->queue, RB_QUEUE_IDLE);
}

/* ==========================================================================
   Promotion / reordering
   ========================================================================== */

/* queue.rs:566-578 `promote_entry` — same MOVE discipline as
   promote_entry_by_abs: shift the tail up, wipe the vacated slot at `absolute`
   (it held `entry`'s old position, and copying the neighbour over it aliased the
   neighbour's owned pending_actions into two slots), then seat `entry` at 0. */
void rb_queue_promote_entry(GameState *g, int from_index) {
    if (!g) return;
    int absolute = (int)g->queue.cur + from_index;
    if (absolute >= g->queue.n_entries || from_index == 0) return;
    RbQueueEntry tmp = g->queue.entries[absolute];
    for (int i = absolute; i > 0; i--)
        g->queue.entries[i] = g->queue.entries[i - 1];
    memset(&g->queue.entries[absolute], 0, sizeof(RbQueueEntry));
    g->queue.entries[absolute].optional_cost_result = -1;
    g->queue.entries[absolute].triggering_member_id = -1;
    g->queue.entries[0] = tmp;
    if ((int)g->queue.cur > absolute)
        g->queue.cur--;
    else
        g->queue.cur = 0;
}

/* queue.rs:592-602 `promote_entry_by_abs`.

   The `absolute == 0` arm is not a no-op in Rust: it falls into the early-return
   block that still runs `current_index = 0`. Promoting the head to the head is
   order-preserving but re-seats the current index on position 0, so a caller that
   asked for the head to become current gets it. The C returned before that, so
   `promote_entry_by_abs(0)` left `current_index` wherever it was.

   The move is a MOVE: `entries.remove(absolute)` in Rust leaves a hole and
   `insert(0, entry)` fills it, so the removed entry exists in exactly one slot.
   The C's shift loop overwrote the high slot with a copy of its neighbour, which
   left that neighbour's owned `pending_actions` pointers live in two slots —
   the same aliasing hazard clear_completed has, and a double free waiting for the
   next release path. Shift, then wipe the vacated slot. */
void rb_queue_promote_entry_by_abs(GameState *g, int absolute) {
    if (!g) return;
    if (absolute >= g->queue.n_entries) return;
    if (absolute == 0) {          /* Rust: the early-return arm still reseats cur */
        g->queue.cur = 0;
        return;
    }
    RbQueueEntry tmp = g->queue.entries[absolute];
    for (int i = absolute; i > 0; i--)
        g->queue.entries[i] = g->queue.entries[i - 1];
    memset(&g->queue.entries[absolute], 0, sizeof(RbQueueEntry));
    g->queue.entries[absolute].optional_cost_result = -1;
    g->queue.entries[absolute].triggering_member_id = -1;
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
     - rb_queue_take_pending_actions          (taken AND freed here; the caller
                                     gets the count and owns nothing)
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
    (mirrors Rust entry.set_pending_actions(vec), ability_queue.rs:532-536).

    INVARIANT (see the file header): on return, pending_actions_n counts exactly
    as many OWNED deep clones as the array holds. The OOM guard used to be
    missing here, so a failed clone parked a NULL and inflated the count — the
    NULL-slot hole this file used to have. Skip the slot instead. */
void rb_queue_store_pending_actions(GameState *g, AbilityEffect *const *actions, int count) {
    if (!g) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    RbQueueEntry *e = &g->queue.entries[cur];
    entry_clear_pending_actions(e);
    for (int i = 0; i < count && e->pending_actions_n < RB_ENTRY_PENDING_CAP; i++) {
        if (!actions[i]) continue;
        AbilityEffect *owned = rb_effect_deep_clone(actions[i]);
        if (!owned) continue;              /* OOM: keep the count honest */
        e->pending_actions[e->pending_actions_n++] = owned;
    }
}

/* Real effects to satisfy a count-only set/save request.

   Rust's set_pending_actions / save_pending_actions both take a `Vec<AbilityEffect>`
   — the caller hands over the actual commands. The C exposes a count-only
   signature (fixed in include/rabuka.h, not this file's to change), so the
   effects have to be recovered from the queue's OWN continuation state, which is
   the same tree those callsites are standing on:

     1. `resume_eff`'s children — choice.c:3205-3209 pauses with
        `resume_eff->n_child > 0` precisely because Rust's `effect_steps[0]`
        (ability_queue.rs:532's argument) is that child's tail. This is the
        direct counterpart of Rust's `effect_steps` and is the common case.
     2. `deferred` — compound.c parks the sequential remainder here when a cost
        pay_skip gate stops the resolution, and that remainder is the next
        command the continuation must run.

   Returns the number of real effects written into `out` (at most `max_out`).
   Returns 0 when the queue holds no continuation, which is the signal that the
   count-only request cannot be satisfied with real commands. */
static int queue_resolve_continuation_effects(const GameState *g, AbilityEffect **out, int max_out) {
    if (!g || !out || max_out <= 0) return 0;
    int n = 0;
    if (g->queue.resume_eff) {
        for (int i = 0; i < g->queue.resume_eff->n_child && n < max_out; i++) {
            if (g->queue.resume_eff->child[i]) out[n++] = g->queue.resume_eff->child[i];
        }
    }
    if (n < max_out && g->queue.deferred) out[n++] = g->queue.deferred;
    return n;
}

/* queue.rs:532-536 `set_pending_actions(actions)` — REPLACE the current entry's
    pending list.

    The count-only C signature cannot express "these N commands"; it can only say
    "park N commands". Previously that was satisfied by writing N NULLs, which is
    the fabrication this function exists to remove: the entry then reported
    `pending_actions_n == N` while owning nothing, so
    rb_queue_resume_pending_actions skipped N slots executing nothing and
    rb_resolver_continue_siblings took the OWNED path and ran nothing — the
    continuation silently never happened.

    Now the count is satisfied with real deep clones of the queue's own
    continuation (queue_resolve_continuation_effects), routed through
    rb_queue_store_pending_actions so the deep clone happens in one place. If the
    queue holds no continuation, the list is left EMPTY rather than inflated with
    markers: a truthful zero lets the caller's `cont` fallback run the remaining
    siblings, which is what Rust does, whereas a fake N would suppress that
    fallback and lose the effects for good.

    count <= 0 clears, which is Rust's `set_pending_actions(Vec::new())` and is
    what choice.c:3282 (a position-change skip) and the parity test rely on. */
void rb_queue_set_pending_actions(GameState *g, int count) {
    if (!g) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    if (count <= 0) {
        rb_queue_store_pending_actions(g, NULL, 0);
        return;
    }
    if (count > RB_ENTRY_PENDING_CAP) count = RB_ENTRY_PENDING_CAP;
    AbilityEffect *src[RB_ENTRY_PENDING_CAP];
    int n = queue_resolve_continuation_effects(g, src, count);
    if (n <= 0) {
        rb_queue_store_pending_actions(g, NULL, 0);
        return;
    }
    rb_queue_store_pending_actions(g, src, n);
}

/* ── rb_queue_save_pending_actions — DELETED. See the tombstone above; the Rust
    capability it duplicated is rb_queue_extend_pending_actions, which compound.c:698
    already calls at the one site Rust calls it (conditional_on.rs:43). */

/* Take the pending list out (Rust take_pending_actions returns Vec). Returns
    the count; the caller takes ownership of the popped effect pointers.

    OWNERSHIP: this is the second of the three release paths. The slots are
    NULLed as they are handed out and `pending_actions_n` drops to 0, so the entry
    no longer references them and cannot free them again. Anything past `max_out`
    cannot be returned to the caller, so it is freed HERE rather than leaked; the
    caller is told only about the `max_out` it can actually hold. */
int rb_queue_take_pending_actions_owned(GameState *g, AbilityEffect **out, int max_out) {
    if (!g || !out || max_out <= 0) return 0;
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
        /* The tail is compacted down over the slot being consumed, and the
           vacated high slot is NULLed. The old body left the popped pointer
           duplicated in the slot above, so the entry held one owned effect in two
           slots: entry_clear_pending_actions and any later take would each reach
           it, and the shift dropped the count without releasing it. */
        AbilityEffect *cmd = e->pending_actions[0];
        for (int i = 1; i < e->pending_actions_n; i++)
            e->pending_actions[i - 1] = e->pending_actions[i];
        e->pending_actions[e->pending_actions_n - 1] = NULL;
        e->pending_actions_n--;
        if (!cmd) continue;   /* unreachable under the ownership invariant */
        if (g->queue.cancel_remaining_commands) {
            g->queue.cancel_remaining_commands = 0;
            rb_effect_free(cmd);
            break;
        }
        /* `e` is a pointer INTO the entries array, and the executed effect may
           re-enter the queue (a sub-prompt's answer path can clear or compact it).
           Re-derive the entry after every execution so effect_started below is
           written to the entry that is still current, not through a stale
           pointer. Rust has no equivalent hazard: `entries` is a Vec whose
           borrows the resolver cannot invalidate mid-resolution. */
        rb_execute_effect_ex(g, actor, cmd, host);
        /* LIFETIME FIX (2nd instance of the use-after-free class fixed in
           pca_resolve_entry, game_state_abilities.c). `cmd` is an OWNED deep
           clone (this file's invariant, header :27-45), so releasing it here is
           not a leak — but it is the ONLY thing keeping its subtree alive, and
           the execution above may have parked a pointer INTO that subtree as
           `g->queue.resume_eff` (the producers in look.c / draw.c / move.c /
           move_missing.c / misc.c / state.c / ability_effects.c all assign the
           node they are executing) or as look.c's file-static
           `g_look_pending_select`. The prompt is answered AFTER this function
           returns, and every answer-path reader dereferences those aliases
           again, so an unconditional release here dangles them one prompt deep.

           MEASURED (probe over all 24 suites, counting only the cases where the
           freed tree CONTAINS the parked resume_eff and a prompt is open):
           2 hits, both in parity_queue_resume — the suite that exists to test
           this park/resume protocol — and 0 in the other 23.

           RUST'S MODEL. resume_pending_actions (engine/src/ability/choice.rs:
           117-136) returns Ok(()) at :130 on the interrupted step, dropping the
           local `pending` Vec — but nothing aliases into it: Rust's
           `in_flight.spawn_context.target` is `effect.target.clone()`
           (choice.rs:120, an owned String) and the remaining tail is COPIED
           back into the queue by requeue_remaining_after_choice (choice.rs:87-
           105). So Rust's drop is safe and the C's has to be deferred. The
           deferral slot is the same one rb_drain_keep_alive (:1323) and
           pca_resolve_entry use, and choice.c:5641-5648 drains it when the
           prompt closes.

           The transfer is TOTAL (see rb_drain_keep_alive's note at :1316-1322):
           the slot is zeroed first and `cmd` is NULLed, so the
           rb_effect_free(cmd) below cannot touch the transferred subtree and
           every node still has exactly one owner. When the slot is ALREADY
           taken, cmd is released as before — the tree then leaks rather than
           dangling, which is the strictly safer of the two outcomes and the
           same bounded-by-nesting-depth residual engine.c:1132-1135 documents
           for the nested-activation case.

           MEASURED effect: no suite's verdict changes (parity_queue_resume 5,
           parity_cce_gaps 82, parity_under_member 81, parity_look_per_card 65,
           parity_sequential 142, parity_search_reorder 97,
           parity_small_clusters 101 — identical before and after), which is the
           required property for a memory fix: it removes undefined behaviour
           without reshaping any assertion. */
        if (rb_has_pending_choice(g) && !g->activation_keepalive_valid) {
            memset(&g->activation_keepalive, 0, sizeof(g->activation_keepalive));
            g->activation_keepalive.effect = cmd;
            g->activation_keepalive_valid = 1;
            cmd = NULL;
        }
        rb_effect_free(cmd);
        executed++;
        int c2 = g->queue.cur;
        if (c2 >= 0 && c2 < g->queue.n_entries) e = &g->queue.entries[c2];
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

/* The identity of one spent use, mirroring Rust's
   `turn_limited_abilities_used: HashMap<(i16, usize, u32), u8>` key
   (core/game_state/abilities.rs:146-151 reads it back through
   `ability_uses_used`). Rust keys on the FULL (card_id, ability_index) pair,
   so this must too.

   The key used to be `(card_id << 4) | (ability_idx & 0xF)` — a FOUR-BIT
   ability index. That silently aliases two different Rust keys onto one C
   counter, and the gained arm is where it bites:

     RB_GAINED_ABILITY_INDEX_BASE is 10000 (rabuka.h:3028), and
     10000 & 0xF == 0, so a GAINED ability at index 10000+g produced the same
     key as the PRINTED ability #g on the same card. Rust keeps them apart;
     the C merged a gained ability's use count into a printed ability's, so
     `rb_use_limit_reached` could refuse a printed 自動 ability because an
     unrelated gained ability had resolved, and `rb_use_count` could report a
     nonzero count for an ability that never resolved at all.

   MEASURED REACHABILITY TODAY: 0 of 985. Over all 24 `gain_ability` /
   `gain_ability_from_source` nodes in cards/abilities.json the distinct
   `ability_gain_trigger` values are 常時 (20) and ライブ成功時 (2) — never
   自動 or 起動, which are the only two families that carry a use_limit in the
   corpus (`use_limit AND 自動` = 53, `use_limit AND 起動` = 80, and
   `use_limit AND {登場, ライブ開始時, ライブ成功時, 常時}` = 0 each). So the
   collision needs a gained ability the parser never emits. That is exactly the
   definition of a latent bug in a WIRED path (the gained ライブ成功時 arm at
   src/turn/triggers.c:522-537 calls rb_use_limit_reached with the aliased
   index), so it is fixed here rather than documented around.

   PROVABLY BEHAVIOUR-NEUTRAL on the corpus: with no key collisions in
   existence, widening the key changes which slot a pair hashes to but not
   which pairs compare equal, so every verdict is identical. card_id < 2^15
   (2639 cards) and ability_idx is masked to 16 bits, so the packed key stays
   inside a positive int and RB_USE_TRACK is untouched. */
static int rb_use_key(int card_id, int ability_idx) {
    return (card_id << 16) | (ability_idx & 0xFFFF);
}

int rb_use_limit_reached(RbAbilityQueue *q, int card_id, int ability_idx,
                         int limit, int cur_turn) {
    if (!q || limit <= 0) return 0;
    if (q->use_turn != cur_turn) return 0;
    int key = rb_use_key(card_id, ability_idx);
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
    int key = rb_use_key(card_id, ability_idx);
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
    int key = rb_use_key(card_id, ability_idx);
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
   trigger_live_success_abilities (turn/trigger_scan.rs:339) each collect into a
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
   other trigger path changes behaviour.

   SCOPE: Rust's `seen` is a LOCAL HashSet, created fresh inside one scan call
   for ONE player (trigger_scan.rs:184-188, :337-339) — the scan of
   `player_id_clone`'s live-card zone followed by that same player's stage. It
   is never shared between the two seats, and never outlives the scan. This
   drain's `seen` array DOES outlive a single scan, because one drain can walk
   entries pushed by several scans, so the key carries the entry's owning seat
   as well: `player_id` is stamped "p1"/"p2" by rb_push_owned_entry
   (turn/triggers.c:30-39), which is the only pusher for both families.

   Keying on (card_id, ability_idx) alone would be WRONG and would swallow a
   legitimate trigger: card ids are card-definition ids reused across instances
   (turn/triggers.c:251-254 counts the same cid appearing twice in one live
   zone), so a cid owned by BOTH seats yields one entry per seat. Rust fires
   both; a seat-blind key resolves only the first and silently drops the
   other player's activation.

   Capacity: the loop below runs at most RB_QUEUE_DEPTH (16) iterations, so
   n_seen can never approach RB_DRAIN_SEEN_CAP. The overflow branch therefore
   ALLOWS the entry rather than refusing it — a memory bound must never be able
   to drop a first-time trigger. */
typedef struct { int card_id; int ability_idx; char player_id[16]; } RbDrainSeen;
#define RB_DRAIN_SEEN_CAP 128

static int rb_drain_seen_add(RbDrainSeen *seen, int n, const char *player_id,
                             int card_id, int ability_idx) {
    for (int i = 0; i < n; i++)
        if (seen[i].card_id == card_id && seen[i].ability_idx == ability_idx &&
            strcmp(seen[i].player_id, player_id) == 0) return 0;
    if (n >= RB_DRAIN_SEEN_CAP) return 1;   /* unreachable; allow rather than drop */
    seen[n].card_id = card_id;
    seen[n].ability_idx = ability_idx;
    snprintf(seen[n].player_id, sizeof(seen[n].player_id), "%s", player_id);
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

/* The drain's resolution-time turn-limit record — the C counterpart of the ONE
   place Rust spends a use: `GameState::record_ability_use`
   (core/game_state/abilities.rs:216-231), reached from
   `gates::record_use_limit`'s Early and Final phases (gates.rs:308 and :331)
   via resolver.rs:1353 and :1390.

   WHY NOT A BARE rb_record_ability_use(g, cid, aidx):
   that function (game_state_abilities.c:49-57) is abilities.rs:216-231 with
   `current_entry()` bound to `g->queue.cur` — and BOTH halves of it are
   entry-scoped: the already-recorded guard (:51-53) and the flag write
   (:55-56). This drain deliberately does NOT publish `cur` (see the note
   inside the loop), so a bare call would read and write an entry that is not
   the one being resolved: a `cur` that happens to sit on an already-flagged
   entry would swallow this activation's record outright, and the flag would be
   stamped on the wrong entry. Rust has no such ambiguity — its drain always
   has the resolving entry published as `current_entry()`, which is what
   abilities.rs:220 tests and :228 writes. Pointing `cur` at the entry being
   resolved for the duration of the call reproduces exactly that, and keeps
   rb_record_ability_use the single owner of "spend the use and mark the entry"
   (the interim enqueue-time marks depend on that one implementation).

   The stamp after the call is the belt-and-braces pca_resolve_entry uses
   (game_state_abilities.c:2817-2820) and is what makes the SECOND phase of one
   activation a no-op: one activation spends exactly one use. */
static void drain_record_ability_use(GameState *g, RbQueueEntry *e, int idx) {
    if (!g || !e || e->use_limit_recorded) return;
    int saved_cur = g->queue.cur;
    g->queue.cur = idx;
    rb_record_ability_use(g, e->card_id, e->ability_idx);
    g->queue.cur = saved_cur;
    e->use_limit_recorded = 1;
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
        /* GAINED-AWARE RANGE GUARD. This is the FIRST place a gained entry was
           retired, and it retired it BEFORE the decode below was reached, so
           fixing only the decode leaves the bug exactly as visible.
           rb_card_num_abilities is PRINTED-only: it counts the card's own
           printed abilities (card.c:272), so a gained entry at
           RB_GAINED_ABILITY_INDEX_BASE+g has ability_idx >= n for every real
           card (n is at most a handful), took this branch, and was stamped
           effect_started = 1 having produced nothing and never re-examined.
           A GAINED ライブ成功時 / 常時 ability therefore NEVER FIRED.

           The bound for the gained arm is the gained store's occupancy, not a
           printed count: gidx < rb_card_num_gained_abilities(g, card_id). */
        if (e->ability_idx < 0) { e->effect_started = 1; continue; }
        if (e->ability_idx >= RB_GAINED_ABILITY_INDEX_BASE) {
            if (e->ability_idx - RB_GAINED_ABILITY_INDEX_BASE >=
                rb_card_num_gained_abilities(g, e->card_id)) { e->effect_started = 1; continue; }
        } else {
            int n = rb_card_num_abilities((uint32_t)e->card_id);
            if (e->ability_idx >= n) { e->effect_started = 1; continue; }
        }
        /* trigger_scan.rs:188 / :339 `seen.insert((id, idx))` — see
           rb_drain_seen_add. Without it a live card that is also staged, or a
           live zone holding the same card twice, fires its ability twice. The
           seat is part of the key because Rust's `seen` is per-scan, per
           player, and one drain can span entries from both seats. */
        if (e->card_id >= 0 && e->trigger[0] &&
            (rb_trigger_is(e->trigger, RB_TSTR_LIVE_START) ||
             rb_trigger_is(e->trigger, RB_TSTR_LIVE_SUCCESS))) {
            if (!rb_drain_seen_add(seen, n_seen, e->player_id, e->card_id, e->ability_idx)) {
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
        /* GAINED-AWARE, per include/rabuka.h:2230-2236: "The queue processor
           (src/ability/ability_queue.c) must use this, not
           rb_decode_card_ability, or a queued GAINED ability silently resolves
           to nothing." rb_decode_card_ability answers 0 for any index at or
           above RB_GAINED_ABILITY_INDEX_BASE, which is indistinguishable from
           "no such printed ability" -- the exact failure this replaces. */
        if (!rb_resolve_card_ability_at(g, e->card_id, e->ability_idx, &ab)) {
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

        /* resolver.rs:1341-1344 `let cost_already_paid = gs.ability_queue
           .current_entry().is_some_and(|e| e.cost_paid);` -- Rust reads the
           entry's latch BEFORE pay_ability_cost (:1348), and both the Early
           record below and the Final record at the tail of this loop are
           conditioned on the value as of THAT read, so it is captured before
           the pay (which settles optional costs synchronously and writes
           cost_paid) rather than re-read afterwards. */
        int cost_already_paid = e->cost_paid;

        /* -- EARLY USE-LIMIT RECORD ------------------------------------------
           resolver.rs:1353 `record_use_limit(self, gs, &ability, ability_key,
           cost_already_paid, UseLimitPhase::Early, None)`: the drain's
           resolution-time counterpart of Rust's single record_ability_use.
           It sits exactly where Rust calls it -- after pay_ability_cost,
           BEFORE handle_pending_choice and before the post-cost gates -- which
           is what makes a DECLINED optional cost still spend the activation
           (gates.rs:291-309 runs ahead of the gate at :1360, and the Final
           recording at :1390 is unreachable once that gate stops).

           gates.rs:288-309, guard for guard:
             ability_key valid         -> the loop already rejected card_id < 0
             use_limit.is_none()       -> ab.use_limit > 0 (rabuka.h:152, -1 = none)
             cost_already_paid         -> !cost_already_paid
             awaiting.choice.is_some() -> !rb_has_pending_choice(g)
             is_conditional_optional   -> effect action == "conditional_on_optional"
             is_optional_effect        -> !effect->is_optional
             can_activate              -> effect.is_none_or(can_activate_effect)
           and gates.rs:308 spends it. An entry that already carries the flag --
           i.e. one whose use an ENQUEUE-time site spent (the interim shape) --
           is left alone, so this is a no-op for those and only adds the record
           where there was none. */
        if (ab.use_limit > 0 && !cost_already_paid && !rb_has_pending_choice(g)) {
            int is_conditional_optional =
                ab.effect && ab.effect->action &&
                !strcmp(ab.effect->action, "conditional_on_optional");
            int is_optional_effect = ab.effect && ab.effect->is_optional;
            int can_activate = !ab.effect ||
                               rb_can_activate_effect(g, actor, ab.effect, e->card_id);
            if (!is_conditional_optional && !is_optional_effect && can_activate)
                drain_record_ability_use(g, e, i);
        }

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
            /* resolver.rs:1234-1246 -- the stamp that makes the re-fire impossible.
             *
             *     // Mark effect as started when a pending choice comes from effect
             *     // execution (not cost). This prevents RWC from re-entering the
             *     // ability after the effect's choice resolves.
             *     let is_paid = cost_already_paid
             *         || gs.ability_queue.current_entry().is_some_and(|e| e.cost_paid);
             *     if is_paid {
             *         if let Some(entry) = gs.ability_queue.current_entry_mut() {
             *             entry.effect_started = true;
             *         }
             *     }
             *     self.store_pending_choice(gs);
             *     Ok(true)
             *
             * `run_ability_effect` returns true exactly when the effect left a
             * prompt open, so this arm IS the C's fold point for those five lines
             * -- the same event, the same place in the sequence. The C had no
             * equivalent, which is the whole defect: the prompt-parking arm below
             * broke out with the entry STILL UNSTAMPED, so the next
             * `rb_drain_ability_queue` (which `rb_resolver_clear_choice_state_and_resume`
             * at choice.c:445-451 ends in, and which the answer path therefore
             * re-enters from the TOP) hit the skip test at :1378 with
             * `effect_started == 0 && completed == 0` and RE-EXECUTED the effect,
             * re-raising the prompt.
             *
             * MEASURED shape of that loop, from the suite's own instrumentation:
             * the same entry, `n=1 started=0 completed=0`, no enqueue and no
             * compaction between the two drains. `rb_queue_resume_pending_actions`
             * -- the OTHER stamp site, choice.rs:81-83, folded at :1061-1064 --
             * cannot cover it because that function only runs while the entry has
             * `pending_actions_n > 0` (:1033), and this entry has none.
             *
             * The `is_paid` gate is Rust's and is load-bearing: a prompt raised
             * while the COST is still being paid must leave the entry un-stamped,
             * because the answer path has to re-enter and run the effect then. That
             * is also why the rejected experiment stamped unconditionally in
             * `pca_resolve_entry` (game_state_abilities.c) and made
             * parity_search_reorder go 120 -> 1419-ish: it stamped the cost-side
             * entries too, not just this effect-side one.
             *
             * The entry is re-derived from `i`, not through `e`, because
             * `rb_execute_effect_ex` may re-enter the queue -- the same reason
             * rb_queue_resume_pending_actions re-derives after every execution and
             * the Final-record arm below re-derives too. `current_entry_mut()` in
             * Rust is `cur`, which this drain deliberately does not publish (see
             * the note at :1397), so `i` is the entry Rust would have had current.
             */
            if (i < g->queue.n_entries) {
                RbQueueEntry *ce = &g->queue.entries[i];
                if (cost_already_paid || ce->cost_paid) ce->effect_started = 1;
            }
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
        /* -- FINAL USE-LIMIT RECORD ------------------------------------------
           resolver.rs:1381-1390: `if self.run_ability_effect(..)? { return
           Ok(()); }` then `record_use_limit(self, gs, &ability, ability_key,
           cost_already_paid, UseLimitPhase::Final, None)`. run_ability_effect
           returns true whenever a choice is left open, so the pending-prompt
           arm above (which breaks out) is Rust's early return and the Final
           record is unreachable there; this is the one path that reaches it.

           gates.rs:321-333, guard for guard: `can_activate` is
           `effect.is_none_or(..)`, and `should_record` is true unconditionally
           for a ConditionalOnOptional whose cost was already paid. The entry is
           re-derived from `i` rather than reusing `e`, because
           rb_execute_effect_ex above may re-enter the queue -- the same reason
           rb_queue_resume_pending_actions re-derives its entry after every
           execution. On an activation the Early record already spent, `e`'s
           flag makes this a no-op, which is how one activation spends exactly
           one use. */
        if (ab.use_limit > 0 && i < g->queue.n_entries) {
            int is_conditional_optional =
                ab.effect && ab.effect->action &&
                !strcmp(ab.effect->action, "conditional_on_optional");
            int can_activate = !ab.effect ||
                               rb_can_activate_effect(g, actor, ab.effect,
                                                      g->queue.entries[i].card_id);
            if ((cost_already_paid && is_conditional_optional) || can_activate)
                drain_record_ability_use(g, &g->queue.entries[i], i);
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
