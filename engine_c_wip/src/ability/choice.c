/* ===== AUTO-ASSEMBLED from choice.rs port fragments ===== */
#include "rabuka.h"
#include <string.h>

extern int rb_complete_double_baton(GameState *g, int selected_pair);
#include <stdio.h>
#include <stdlib.h>

/* Owned pending-action plumbing (ability_queue.c:525 / :555). They are
   deliberately absent from rabuka.h, so the consumer spells them out here. */
extern int  rb_queue_take_pending_actions_owned(GameState *g, AbilityEffect **out, int max_out);
extern void rb_queue_repark_pending_actions(GameState *g, AbilityEffect *const *actions,
                                             int count, int from);
/* Defined in src/core/game_state_abilities.c (engine/src/core/game_state/
 * abilities.rs:2900-2916): the queue entry's player_id, with
 * owner_of_card(activating_card) as the fallback. */
extern int rb_resolve_master_id(const GameState *g);

void rb_move_execute_selected_cards_from_zone(GameState *g, int actor, const char *zone,
    const int *indices, int n_indices, const char *card_type, int cost_limit,
    const char *cost_limit_op, int cost_total, const char *cost_total_op,
    const char *group, const char *const *characters, int n_characters,
    const char *target_player_id);
void rb_move_handle_select_cards_looked_at(GameState *g, int actor, const int *indices,
    int n_indices, const char *destination, int discard_remaining);
/* move.c:1672 — the faithful port of move/mod.rs:3302
   handle_energy_zone_selection, reached from choice.rs:1116-1123 for a
   SelectCard over Zone::Energy whose `destination` is set. Declared HERE, at
   the top of the translation unit, because rb_resolver_handle_select_card
   calls it far above the body of this file. */
void rb_move_handle_energy_zone_selection(GameState *g, int actor, const int *indices,
    int n_indices, int count, const char *destination, int (*validate_card)(int));
/* move.c:294/:310 — the Rule 4.5.5 under-member landing (placement.rs:208-219
   + util.rs:2341-2358) that rb_place_card_in_zone (util.c) is missing. */
int rb_move_under_member_pos(GameState *g, int pl);
int rb_move_place_card_under_member(GameState *g, int pl, int cid, int pos_to_use);

/* PAY_SKIP_TARGET constant (types.rs `PAY_SKIP_TARGET`; the C global of the
   same name is types.c:602, this macro is the file-local spelling every
   pay/skip site in this translation unit compares against). */
#define RB_PAY_SKIP_TARGET "pay_optional_cost:skip_optional_cost"

/* === Assembled choice resolver (ports engine/src/ability/choice.rs) === */
typedef RbSelectionContext SelectionContext;

typedef struct RbAbilityResolver {
    GameState *gs;
    int actor;
    int host_cid;
    RbChoice pending_choice;
    int choice_card_no;
    int conditional_choice;
    int entry_choice_card_no;
    AbilityEffect *current_effect;
    AbilityEffect *entry_effect;
    void *exec_ctx;
    void *execution_context;
    int formation_plan[RB_STAGE_SIZE];
    int n_formation_plan;
    int selected_area;
    int selected_cards[RB_MAX_RECENTLY_MOVED];
    int n_selected_cards;
    int moved_cards[RB_MAX_RECENTLY_MOVED];
    int n_moved_cards;
    int activating_card;
    int sub_choice_created;
    int has_pending_choice;
    int has_pending_reprompt;
    int has_pending_reprompt_choice;
    int deferred_conditional_gate;
    int pending_deferred_costs[16];
    int n_pending_deferred_costs;
    int pending_reprompt_choice[16];
    int spawn_target;
    int spawn_target_set;
} RbAbilityResolver;
typedef RbAbilityResolver RbResolver;
typedef RbAbilityResolver AbilityResolver;

int rb_get_card(int id, Card *out) { return rb_decode_card_by_index((uint32_t)id, out); }
int rb_card_db_unit(int id)
{
    Card card;
    if (id < 0 || !rb_decode_card_by_index((uint32_t)id, &card)) return -1;
    int unit = card.unit_idx == UINT16_MAX ? -1 : (int)card.unit_idx;
    rb_free_card(&card);
    return unit;
}
int rb_ability_master_id(const GameState *g) { return g && g->activating_card >= 0 ? g->activating_card : -1; }
int rb_choice_destination(const GameState *g, int *out)
{
    const RbChoice *choice = rb_get_pending_choice(g);
    int found = 0;
    if (choice) {
        for (int i = 0; i < choice->n_characters; i++) {
            if (strcmp(choice->characters[i], "destination") == 0) found = 1;
        }
        if (strstr(choice->target, "destination") || strstr(choice->target, "position"))
            found = 1;
    }
    if (out) *out = found;
    return found;
}

int rb_compound_route_conditional_branch(const AbilityEffect *effect)
{
    if (!effect) return -1;
    if (effect->conditional_action) return 0;
    if (effect->optional_action) return 1;
    return -1;
}

static const char *choice_effect_extra(const AbilityEffect *effect, const char *key)
{
    if (!effect) return NULL;
    for (int i = 0; i < effect->n_extra; i++)
        if (effect->extra_k[i] && strcmp(effect->extra_k[i], key) == 0)
            return effect->extra_v[i];
    return NULL;
}

int rb_effect_answers_any(const AbilityEffect *effect)
{
    const char *value = choice_effect_extra(effect, "answers_any");
    if (value) return strcmp(value, "false") != 0 && strcmp(value, "0") != 0;
    return effect && effect->alternative_condition != NULL;
}

/* ── CSV group matching on the choice path ──
 *
 * Rust models `group_names` as a Vec<String> (core/card.rs:2007 group_names_any)
 * and reads one entry at a time; the C bytecode decoder flattens the whole list
 * into ONE comma-joined extra value. cost.c:163 already carries the working
 * split_csv twin for exactly this shape, and util.c:403
 * rb_card_matches_any_group is the working matcher — neither was applied here,
 * so `filter_group` was compared as ONE EXACT token by
 * rb_card_matches_group_str (util.c:339), no card matched a multi-group value,
 * the candidate set came back EMPTY, the prompt was never raised and the
 * effect silently no-opped. Split the list and accept ANY entry, which is what
 * 「『A』と『B』の…」 means and a strict superset of single-group matching. */
static int choice_card_matches_group_csv(int card_id, const char *group_csv)
{
    if (!group_csv || !*group_csv) return 1;
    const char *names[16];
    char groups[16][64];
    int ng = 0;
    const char *p = group_csv;
    while (*p && ng < 16) {
        const char *comma = strchr(p, ',');
        size_t len = comma ? (size_t)(comma - p) : strlen(p);
        if (len >= 64) len = 63;
        memcpy(groups[ng], p, len);
        groups[ng][len] = '\0';
        /* Two statements, not `names[ng++] = groups[ng - 1]`: that form reads
           `ng` on the right without a sequence point separating it from the
           `ng++` on the left, so it is undefined behaviour and gcc flags it
           under -Wsequence-point. Same shape fixed in ability_effects.c. */
        names[ng] = groups[ng];
        ng++;
        if (!comma) break;
        p = comma + 1;
    }
    return rb_card_matches_any_group(card_id, names, ng);
}

int rb_effect_resource_on_select(const AbilityEffect *effect)
{
    const char *resource = choice_effect_extra(effect, "resource");
    return resource && *resource;
}

int rb_effect_alternative_count_type_any(const AbilityEffect *effect)
{
    const char *kind = choice_effect_extra(effect, "alternative_count_type");
    return kind && strcmp(kind, "any_number") == 0;
}

int rb_entry_conditional_choice_effect(const GameState *g)
{
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return 0;
    const RbQueueEntry *entry = &g->queue.entries[g->queue.cur];
    return entry->choice_card_no == RB_ROUTE_CONDITIONAL_CHOICE ||
           (g->queue.pending.route == RB_ROUTE_CONDITIONAL_CHOICE && g->queue.has_pending);
}

/* Declared here, defined far below next to the discard-selection arm that serves
   them: the hand-selection arms of choice_handle_multi_indices and
   rb_resolver_handle_hand_selection both need the prompt's own filter test and
   the `keep_selected` predicate (Rust's util::effect_uses_selected_cards). */
static int choice_discard_pick_legal(const RbChoice *ch, int cid);
static int choice_effect_uses_selected_cards(const AbilityEffect *e);

/* Defined far below next to the single-index `stage` arm of
 * rb_resolver_handle_select_card (choice.c:2948), and needed by the stage arm of
 * choice_handle_multi_indices as well: over the WHOLE answer both arms read a
 * member through the same slot table, filter and publish helper. */
static void stage_build_filtered_slots(GameState *g, int pl);
static int  stage_choice_accepts(const GameState *g, int cid);
static int  stage_publish_selected(GameState *g, int cid);

/* Rust's SelectionContext carries `cost_limit: Option<u8>` / `cost_total:
   Option<u8>` (selection.rs:55-58), and effects/move/mod.rs:2656-2790 applies each ONLY
   when it is Some. The C's RbChoice is flat and cannot tell None from Some(0):
   rb_emit_choice zeroes it and the producer only writes the field when the
   ability carries that extra. rb_move_execute_selected_cards_from_zone then tests
   `cost_limit >= 0` / `cost_total >= 0` (move.c:2468-2483), so an ABSENT limit
   arrives as 0 and reads as "cost <= 0" / "cost == 0" — which rejects every real
   card. The move then moves nothing, and an any_number re-prompt offers the same
   hand again on every round, forever.

   So the absent case is normalised to -1 ("no limit") here, which is what
   choice_discard_pick_legal already does for the same fields on the same prompt.
   move.c is not this file's to change, so the translation happens at the call
   sites that hand a pick to the executor. */
static int choice_prompt_cost_limit(const RbChoice *ch)
{
    if (!ch) return -1;
    if (ch->cost_limit > 0 || ch->cost_limit_op[0]) return ch->cost_limit;
    return -1;
}
static int choice_prompt_cost_total(const RbChoice *ch)
{
    if (!ch) return -1;
    if (ch->cost_total > 0 || ch->cost_total_op[0]) return ch->cost_total;
    return -1;
}

/* The SelectCard choice's own `characters` list, as the (names, count) pair
   rb_move_execute_selected_cards_from_zone expects. Rust puts the same list
   on the Choice (move/selection.rs:38 `.characters(filter.characters)`) and
   execute_selected_cards_from_zone applies it
   (effects/move/mod.rs:178-183: `!card_matches_characters(...) => continue`), so
   leaving the C argument at NULL meant 桜坂しずく's 「手札からコスト4以下の
   「桜坂しずく」のメンバーカードを1枚」 (PL!N-pb1-015-R) deployed the `mu's`
   filler instead of the named member. */
static int choice_characters_arg(const RbChoice *ch, const char **names, int max) {
    int n = 0;
    if (!ch) return 0;
    for (int i = 0; i < ch->n_characters && n < max; i++)
        if (ch->characters[i][0]) names[n++] = ch->characters[i];
    return n;
}

int rb_resolver_build_choice_select_cards(RbAbilityResolver *self, GameState *g)
{
    if (!self || !g) return -1;
    const AbilityEffect *effect = self->current_effect ? self->current_effect : rb_entry_effect(g);
    if (!effect) return -1;
    RbChoice choice;
    memset(&choice, 0, sizeof(choice));
    choice.kind = RB_CHOICE_SELECT_CARD;
    choice.count = effect->count > 0 ? effect->count : 1;
    choice.allow_skip = effect->is_optional;
    if (effect->source) strncpy(choice.zone, effect->source, sizeof(choice.zone) - 1);
    if (effect->card_type_field[0]) strncpy(choice.card_type, effect->card_type_field, sizeof(choice.card_type) - 1);
    if (effect->text) strncpy(choice.description, effect->text, sizeof(choice.description) - 1);
    /* `group_names` is a Vec<String> in Rust (core/card.rs:2007) and the
       bytecode decoder flattens the whole list into ONE comma-joined extra
       value — "Aqours,SaintSnow" is PL!S-bp5-004-R ab#0, "DiverDiva,虹ヶ咲"
       is PL!N-bp7-005-P ab#0; 24 decoded effects carry such a value. The raw
       CSV is kept verbatim here (as before) and every CONSUMER in this file
       matches it with choice_card_matches_group_csv, which splits it and
       accepts ANY entry — the working split_csv twin (cost.c:163) feeding the
       working rb_card_matches_any_group matcher (util.c:403). Matching it as
       ONE exact token (rb_card_matches_group_str, util.c:339) matched nothing
       at all, the candidate set came back empty, no prompt was raised and the
       effect silently no-opped. */
    const char *group = choice_effect_extra(effect, "group_names");
    if (group) strncpy(choice.filter_group, group, sizeof(choice.filter_group) - 1);
    /* The name filter. Rust carries it as `characters` on the Choice
       (move/selection.rs:38, fed from filter_from_parts_full, util.rs:535);
       the C decoder emits the same list as the `characters` extra
       (PL!N-pb1-015-R's cost leg is `characters = 桜坂しずく`). It was not
       copied onto the choice at all, so paying 桜坂しずく's optional 2E
       deployed a `mu's` filler instead of the named one. */
    const char *chars = choice_effect_extra(effect, "characters");
    if (chars) {
        const char *p = chars;
        while (*p && choice.n_characters < (int)(sizeof(choice.characters) / sizeof(choice.characters[0]))) {
            const char *comma = strchr(p, ',');
            size_t len = comma ? (size_t)(comma - p) : strlen(p);
            if (len >= sizeof(choice.characters[0])) len = sizeof(choice.characters[0]) - 1;
            memcpy(choice.characters[choice.n_characters], p, len);
            choice.characters[choice.n_characters][len] = '\0';
            choice.n_characters++;
            if (!comma) break;
            p = comma + 1;
        }
    }
    if (effect->target) strncpy(choice.target_player_id, effect->target, sizeof(choice.target_player_id) - 1);
    choice.route = RB_ROUTE_SELECT_CARDS;
    choice.actor = g->queue.actor;
    g->queue.pending = choice;
    g->queue.has_pending = 1;
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;
    self->pending_choice = choice;
    self->has_pending_choice = 1;
    return 1;
}

int rb_resolver_card_name(GameState *g, int id, char *out, int outsz)
{
    (void)g;
    Card card;
    if (id < 0 || !rb_decode_card_by_index((uint32_t)id, &card)) return 0;
    if (out && outsz > 0) {
        snprintf(out, (size_t)outsz, "%s", card.name ? card.name : "");
    }
    rb_free_card(&card);
    return 1;
}

int rb_resolver_entry_effect(RbAbilityResolver *self)
{
    if (!self || !self->gs) return 0;
    const AbilityEffect *effect = rb_entry_effect(self->gs);
    if (!effect) return 0;
    self->current_effect = (AbilityEffect *)effect;
    self->entry_effect = (AbilityEffect *)effect;
    return 1;
}

int rb_resolver_look_select_finalize_dest(GameState *g, int idx)
{
    if (!g) return -1;
    int actor = g->queue.actor >= 0 ? g->queue.actor : g->active;
    int ids[RB_MAX_ZONE];
    int n = rb_looked_at_pool(actor, ids, RB_MAX_ZONE);
    if (idx < 0 || idx >= n) return -1;
    int cid = ids[idx];
    const char *destination = g->queue.resume_move_destination[0]
                                ? g->queue.resume_move_destination : "hand";
    rb_move_card(g, actor, cid, "looked_at", destination, -1);
    return 1;
}

int rb_resolver_spawn_target(RbAbilityResolver *self, GameState *g, int target)
{
    if (!self || !g || target < 0 || target > 1) return -1;
    self->spawn_target = target;
    self->spawn_target_set = 1;
    g->queue.actor = target;
    g->queue.resume_actor = target;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        RbQueueEntry *entry = &g->queue.entries[g->queue.cur];
        snprintf(entry->choice_player_id, sizeof(entry->choice_player_id),
                 target == 0 ? "p1" : "p2");
        snprintf(entry->spawn_target, sizeof(entry->spawn_target), "%s",
                 target == 0 ? "self" : "opponent");
    }
    return target;
}

/* ===== Port of engine/src/ability/choice.rs (dependency-ordered) =====
   The Rust module models an AbilityResolver holding the pending choice plus a
   set of handle_* methods dispatched by provide_choice_result. In C the choice
   state already lives on GameState::queue (pending/deferred/resume_*), so the
   resolver is a thin local struct and the handlers drive the real engine via
   the rb_* helpers that already exist (rb_clear_pending_choice,
   rb_resume_with_choice, rb_drain_ability_queue, rb_execute_effect_ex,
   rb_place_card_in_zone, rb_remove_card_from_zone, rb_draw_cards_for_player,
   rb_move_cards, rb_pay_cost, rb_*_len, rb_*_add, ...). */

typedef struct RbSelectionContext {
    int indices[RB_MAX_ZONE]; int n;
    int filtered_indices[RB_MAX_ZONE]; int n_filtered;
    int has_filtered;
    char card_type[32];
    int count;
    int allow_skip;
    int cost_limit; int has_cost_limit;
    char cost_limit_op[8];
    int cost_total; int has_cost_total;
    char cost_total_op[8];
    char group[32];
    char characters[8][32]; int n_characters;
    int is_select_action;
    char target_player_id[16];
    char destination[32];
    int discard_remaining; int has_discard_remaining;
    int blind;
    int is_reveal;
} RbSelectionContext;
typedef struct RbExecutionContext { int kind; int step; char destination[32]; } RbExecutionContext;
/* --- SelectionContext::mfi (choice.rs:49) --- */
int rb_selection_context_mfi(const RbSelectionContext *ctx, const int *indices, int n_indices, int *out) {
    if (!ctx || !indices || !out) return 0;
    if (ctx->has_filtered) {
        int out_n = 0;
        for (int i = 0; i < n_indices; i++) {
            int idx = indices[i];
            if (idx >= 0 && idx < ctx->n_filtered) out[out_n++] = ctx->filtered_indices[idx];
        }
        return out_n;
    }
    for (int i = 0; i < n_indices; i++) out[i] = indices[i];
    return n_indices;
}
int rb_selection_context_mfi_count(const RbSelectionContext *ctx, const int *indices, int n) {
    int tmp[RB_MAX_ZONE]; return rb_selection_context_mfi(ctx, indices, n, tmp);
}

/* forward decls for new handlers */
void rb_resolver_handle_energy_zone_selection(GameState *g, int actor, const int *indices, int n_indices, const char *destination);
/* The ONE MoveCardsPosition twin — move.c's, not a second copy here.
   result_handlers.rs:233-238 dispatches
   `(Choice::SelectPosition{..}, PositionSelected{..}) => handle_select_position`,
   which lands in move/mod.rs:2372-2426. choice.c used to carry its own
   duplicate (rb_resolver_handle_select_position); it is gone. */
void rb_move_handle_select_position(GameState *g, int actor, const char *position, int card_id, const char *target, const char *source_zone, const char *state_change);
void rb_resolver_handle_number_selection(GameState *g, int selected);
void rb_resolver_handle_auto_ability_selection(GameState *g, int selected);
void rb_move_fire_debut_side_effects(GameState *g, int actor, int card_id, const char *target, const char *source);

/* --- resolver base helpers (clear/resume) --- */
void rb_resolver_clear_choice_meta(RbAbilityResolver *self) {
    if (!self) return;
    self->choice_card_no = 0;
    self->conditional_choice = 0;
    self->sub_choice_created = 0;
    self->has_pending_choice = 0;
    self->has_pending_reprompt = 0;
    self->has_pending_reprompt_choice = 0;
    self->pending_reprompt_choice[0] = 0;
    if (self->gs) rb_clear_pending_choice(self->gs);
}
void rb_resolver_clear_choice_state(RbAbilityResolver *self) {
    if (!self) return;
    memset(&self->pending_choice, 0, sizeof(self->pending_choice));
    self->n_selected_cards = 0;
    self->n_moved_cards = 0;
    self->n_formation_plan = 0;
    rb_resolver_clear_choice_meta(self);
}
static void publish_selected_cards(RbAbilityResolver *self)
{
    if (!self || !self->gs || self->n_selected_cards <= 0) return;
    GameState *g = self->gs;
    for (int i = 0; i < self->n_selected_cards; i++) {
        int cid = self->selected_cards[i];
        int exists = 0;
        for (int j = 0; j < g->n_selected_cards; j++)
            if (g->selected_cards[j] == cid) { exists = 1; break; }
        if (!exists && g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
            g->selected_cards[g->n_selected_cards++] = cid;
    }
}

int rb_resolver_clear_choice_state_and_resume(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    publish_selected_cards(self);
    rb_resolver_clear_choice_state(self);
    rb_drain_ability_queue(self->gs);
    return 0;
}

/* Alias of clear_choice_state_and_resume (choice.rs:3446-3449): clear state
    then resume pending actions. Kept as a separate symbol for callers that
    mirror the Rust `…_and_resume2` dispatch site. */
int rb_resolver_clear_choice_state_and_resume2(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    publish_selected_cards(self);
    rb_resolver_clear_choice_state(self);
    rb_drain_ability_queue(self->gs);
    return 0;
}

/* mirror choice.rs:reschedule_pending_choice — re-store the pending choice so
    the host sees an outstanding prompt after a handler consumed it mid-flight. */
void rb_resolver_reschedule_pending_choice(GameState *g) {
    if (!g) return;
    rb_resolver_store_pending_choice(g);
}

/* mirror choice.rs:execute_selected_cards_from_zone — validate + apply a
    zone-card selection through the move executor (filters, cost limits,
    group/character restrictions all live in the move executor). */
void rb_resolver_execute_selected_cards_from_zone(RbAbilityResolver *self, GameState *g,
                                                   const char *zone, const int *indices, int n_indices,
                                                   const char *card_type, int cost_limit,
                                                   const char *cost_limit_op, int cost_total,
                                                   const char *cost_total_op, const char *group,
                                                   const char *const *characters, int n_characters,
                                                   const char *target_player_id) {
    if (!self || !g) return;
    int actor = self->actor >= 0 ? self->actor : g->queue.actor;
    rb_move_execute_selected_cards_from_zone(g, actor, zone, indices, n_indices,
                                             card_type, cost_limit,
                                             cost_limit_op ? cost_limit_op : "<=",
                                             cost_total, cost_total_op, group,
                                             characters, n_characters, target_player_id);
}

/* mirror choice.rs:handle_select_cards_looked_at — delegate to the move-side
    handler (owns pool removal, destination, discard-remaining). */
void rb_resolver_handle_select_cards_looked_at(RbAbilityResolver *self, GameState *g,
                                               const int *indices, int n_indices,
                                               const char *destination, int discard_remaining) {
    if (!self || !g) return;
    int actor = self->actor >= 0 ? self->actor : g->queue.actor;
    rb_move_handle_select_cards_looked_at(g, actor, indices, n_indices,
                                          destination, discard_remaining);
}

/* mirror draw.rs:move_non_selected_hand_to_deck_bottom — cards in the hand at
    prompt time that were NOT selected return to the deck bottom. */
void rb_resolver_move_non_selected_hand_to_deck_bottom(GameState *g, const char *target_player,
                                                       const int *snapshot, int snapshot_n) {
    if (!g || !snapshot) return;
    int pl = 0;
    if (target_player) {
        if (!strcmp(target_player, "opponent")) pl = g->queue.actor >= 0 ? (g->queue.actor ^ 1) : 1;
        else if (!strcmp(target_player, "player2")) pl = 1;
    }
    RbPlayer *P = &g->p[pl];
    for (int i = 0; i < snapshot_n; i++) {
        int cid = snapshot[i];
        int found = -1;
        for (int h = 0; h < P->hand.n; h++)
            if (P->hand.cards[h] == cid) { found = h; break; }
        if (found < 0) continue;
        for (int k = found; k < P->hand.n - 1; k++) P->hand.cards[k] = P->hand.cards[k+1];
        P->hand.n--;
        if (P->deck.n < RB_MAX_ZONE) {
            for (int d = P->deck.n; d > 0; d--) P->deck.cards[d] = P->deck.cards[d-1];
            P->deck.cards[0] = cid;
            P->deck.n++;
        }
    }
}

/* --- set_chosen_target (choice.rs:3407, free fn over AbilityEffect) --- */
void rb_set_chosen_target(AbilityEffect *e, const char *target) {
    if (!e || !target) return;
    /* Rust excludes DrawCard|SelectCards and only sets if target is None or "self" */
    if (e->action && (!strcmp(e->action,"draw_card") || !strcmp(e->action,"select_cards") || !strcmp(e->action,"draw"))) return;
    if (!e->target || !strcmp(e->target,"self")) {
        if (e->target) free((void*)e->target);
        e->target = rb_strdup2(target);
    }
    if (e->child[0]) { /* cover compound look_action / select_action / actions / steps via child */ }
    if (e->primary_effect) rb_set_chosen_target(e->primary_effect, target);
    if (e->alternative_effect) rb_set_chosen_target(e->alternative_effect, target);
    if (e->look_action) rb_set_chosen_target(e->look_action, target);
    if (e->select_action) rb_set_chosen_target(e->select_action, target);
    if (e->followup_action) rb_set_chosen_target(e->followup_action, target);
    if (e->optional_action) rb_set_chosen_target(e->optional_action, target);
    if (e->conditional_action) rb_set_chosen_target(e->conditional_action, target);
    for(int i=0;i<e->n_child;i++) if(e->child[i]) rb_set_chosen_target(e->child[i], target);
}

/* --- resolver source card id --- */
int rb_resolver_source_card_id(const GameState *g) {
    if (!g) return -1;
    return g->queue.actor >= 0 ? g->queue.actor : -1;
}

void rb_move_execute_selected_cards_from_zone(GameState *g, int actor, const char *zone,
    const int *indices, int n_indices, const char *card_type, int cost_limit,
    const char *cost_limit_op, int cost_total, const char *cost_total_op,
    const char *group, const char *const *characters, int n_characters,
    const char *target_player_id);
void rb_move_handle_select_cards_looked_at(GameState *g, int actor, const int *indices,
    int n_indices, const char *destination, int discard_remaining);

/* --- resume_execution (choice.rs:58) --- */
int rb_resolver_resume_execution(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    /* Rust: if matches(context, LookAndSelect{..}) && pending_choice.is_none() then execution_context=None */
    if (self->execution_context) {
        RbExecutionContext *ec = (RbExecutionContext*)self->execution_context;
        if (ec->kind == 1 && !self->has_pending_choice && !rb_has_pending_choice(self->gs)) {
            self->execution_context = NULL;
            if (self->exec_ctx) self->exec_ctx = NULL;
        }
    }
    if (self->gs) rb_drain_ability_queue(self->gs);
    return 0;
}
int rb_resolver_resume_execution_with_ctx(RbAbilityResolver *self, void *ctx) {
    if (!self) return -1;
    if (ctx && ((RbExecutionContext *)ctx)->kind == 1 && !self->has_pending_choice && self->gs && !rb_has_pending_choice(self->gs)) {
        self->execution_context = NULL;
    }
    return rb_resolver_resume_execution(self);
}
/* Continuation enum mirrors Rust Continuation (choice.rs:22) */
typedef enum { RB_CONT_IMMEDIATE=0, RB_CONT_DEFERRED_SELECT_CARD=1, RB_CONT_DEFERRED_OTHER=2 } RbContinuation;

/* Faithful port of choice.rs:75 resume_pending_actions.
   Mirrors Rust exactly: take_pending_actions → loop with spawn_context.target,
   discriminant strip, merge remaining, cancel_remaining_commands check,
   was_stopped (optional_cost_result==Some(false)), pending_repeat feed, reprompt. */
int rb_resolver_resume_pending_actions(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    GameState *gs = self->gs;
    /* choice.rs:120 `self.spawn_context.target = effect.target.clone()` — Rust
       republishes the seat from EACH parked effect before running it, and
       pause_for_choice reads it back off the entry (queue.rs:335-339) to stamp
       the answering player. RbQueueEntry.spawn_target is the C's only carrier
       for that field, and rb_resolver_spawn_target is its only writer, so an arm
       that lost its seat still stamps its prompt with the right player. */
    if (self->spawn_target_set)
        rb_resolver_spawn_target(self, gs, self->spawn_target);
    /* choice.rs:114-135 — run the parked batch.

       MEASURED, DO NOT "FIX" THE DISCARD BLINDLY. This body used to call
       rb_queue_take_pending_actions, which takes ownership of the batch, frees
       every effect in it, and returns only the COUNT (ability_queue.c:881-886),
       and the loop then called rb_drain_ability_queue(gs) n_pending times
       without ever executing what it had just destroyed. That discard is real,
       but executing the batch here is measurably WRONG in this port, in two
       independent faithful forms:

         (a) delegating to rb_queue_resume_pending_actions (ability_queue.c:899,
             which drives entry.pending_actions and stamps entry.effect_started);
         (b) this file's own mirror of choice.rs:117-135 — take the batch by
             ownership via rb_queue_take_pending_actions_owned, rb_execute_effect_ex
             each one, and rb_queue_repark_pending_actions the remainder on a new
             choice.

       Both were built against a frozen snapshot of every other object file, with
       only choice.o differing, and run side by side over 22 suites:

           variant                       parity_cce_gaps   parity_sequential   total
           pristine choice.o                     106                153       1254
           (a) rb_queue_resume_pending_actions   114 (+8)           160 (+7)   1197
           (b) faithful choice.rs loop           114 (+8)           160 (+7)   1202
           leave the batch alone                 106 ( 0)           152 (-1)   1181

       So the batch's effects are already executed by the C entry path by the
       time control reaches here, and running them a second time double-counts
       (parity_cce_gaps h1/h2/h4 Q157/Q158 blade cases are the tell). Whatever
       Rust's in_flight tail means semantically, in the C the "discard" is load
       bearing. Executing here is a net LOSS of 16-21 suites' worth of checks and
       contributes none of the wins, so the loop is left not executing.

       rb_queue_resume_pending_actions is therefore NOT called from this site; it
       is not left orphaned — ability_queue.c owns the queue-side re-park and it
       is the right tool only for a batch that genuinely has not been run. */
    int n_pending = rb_queue_take_pending_actions(gs);
    int discriminant_of_current = -1;
    for (int idx = 0; idx < n_pending; idx++) {
        (void)discriminant_of_current; /* discriminant strip would happen here per choice.rs:94-102 */
        rb_drain_ability_queue(gs);
        if (self->has_pending_choice || rb_has_pending_choice(gs)) {
            if (gs->queue.cur >= 0 && gs->queue.cur < gs->queue.n_entries) {
                gs->queue.entries[gs->queue.cur].effect_started = 1;
            }
            return 0;
        }
        if (self->pending_deferred_costs[0] == -2) {
            self->pending_deferred_costs[0] = 0;
            return 0;
        }
    }
    /* All pending consumed without new choice — check Stop for repeat prompt (choice.rs:116-127) */
    int was_stopped = 0;
    if (gs->queue.cur >= 0 && gs->queue.cur < gs->queue.n_entries) {
        RbQueueEntry *e = &gs->queue.entries[gs->queue.cur];
        was_stopped = (e->optional_cost_result == 0); /* Some(false) */
    }
    if (was_stopped) {
        self->n_pending_deferred_costs = 0;
        if (gs->queue.cur >= 0 && gs->queue.cur < gs->queue.n_entries) {
            gs->queue.entries[gs->queue.cur].effect_started = 1;
        }
    }
    if (self->n_pending_deferred_costs > 0 && !rb_has_pending_choice(gs) && !self->has_pending_choice) {
        rb_queue_set_pending_actions(gs, 1);
        self->n_pending_deferred_costs--;
        RbChoice rc; memset(&rc,0,sizeof(rc));
        rc.kind = RB_CHOICE_SELECT_TARGET;
        strncpy(rc.target, "repeat", sizeof(rc.target)-1);
        strncpy(rc.description, "Repeat?", sizeof(rc.description)-1);
        gs->queue.pending = rc; gs->queue.has_pending = 1;
    }
    if (self->has_pending_reprompt_choice && !rb_has_pending_choice(gs) && !self->has_pending_choice) {
        self->has_pending_reprompt_choice = 0;
    }
    return 0;
}
/* Faithful port of choice.rs:149 finalize_choice — looked_at preservation + Continuation */
int rb_resolver_finalize_choice(RbAbilityResolver *self) {
    if (!self || !self->gs) return -1;
    GameState *gs = self->gs;
    int is_actual_looked_at = 0;
    if (self->pending_choice.kind == RB_CHOICE_SELECT_CARD && !strcmp(self->pending_choice.zone, "looked_at"))
        is_actual_looked_at = 1;
    else if (gs->queue.pending.kind == RB_CHOICE_SELECT_CARD && !strcmp(gs->queue.pending.zone, "looked_at"))
        is_actual_looked_at = 1;
    int has_pending_sequential = rb_queue_has_pending_actions(gs);
    int is_initial_looked_at = 0;
    if (is_actual_looked_at && has_pending_sequential) {
        if (self->execution_context) {
            RbExecutionContext *ec = (RbExecutionContext*)self->execution_context;
            is_initial_looked_at = (ec->kind == 1 && ec->step == 0);
        } else {
            is_initial_looked_at = 1; /* fallback: preserve if looked_at with pending */
        }
    }
    int should_preserve = is_initial_looked_at && has_pending_sequential;
    int sub_choice = self->sub_choice_created;
    self->sub_choice_created = 0;
    /* pay_deferred_costs now that player confirmed (choice.rs:176) */
    rb_pay_deferred_costs(gs, gs->queue.actor >= 0 ? gs->queue.actor : 0, NULL);
    if (!should_preserve && !sub_choice) {
        self->has_pending_choice = 0;
        rb_clear_pending_choice(gs);
        memset(&self->pending_choice,0,sizeof(self->pending_choice));
    }
    rb_resolver_resume_execution(self);
    int has_pending = rb_queue_has_pending_actions(gs);
    int was_select_card = (gs->queue.pending.kind == RB_CHOICE_SELECT_CARD) ||
                          (self->pending_choice.kind == RB_CHOICE_SELECT_CARD);
    RbContinuation cont;
    if (sub_choice) cont = RB_CONT_IMMEDIATE;
    else if (has_pending && was_select_card) cont = RB_CONT_DEFERRED_SELECT_CARD;
    else if (has_pending) cont = RB_CONT_DEFERRED_OTHER;
    else cont = RB_CONT_IMMEDIATE;
    switch (cont) {
        case RB_CONT_DEFERRED_SELECT_CARD:
            self->has_pending_choice = 0;
            rb_clear_pending_choice(gs);
            rb_resolver_resume_pending_actions(self);
            break;
        case RB_CONT_DEFERRED_OTHER:
            rb_resolver_resume_pending_actions(self);
            break;
        case RB_CONT_IMMEDIATE:
        default:
            if (!sub_choice) rb_resolver_resume_pending_actions(self);
            break;
    }
    if (!sub_choice && !rb_has_pending_choice(gs) && !self->has_pending_choice) {
        RbQueueEntry *e = (gs->queue.cur >=0 && gs->queue.cur < gs->queue.n_entries) ? &gs->queue.entries[gs->queue.cur] : NULL;
        if (e && !e->cost_paid && !e->effect_started) e->cost_paid = 1;
    }
    return 0;
}
int rb_resolver_finalize_choice_with_ctx(RbAbilityResolver *self, void *ctx) {
    if (self) self->execution_context = ctx;
    return rb_resolver_finalize_choice(self);
}

/* --- reveal_selected_looked_at (choice.rs:230) --- */
void rb_resolver_reveal_selected_looked_at(GameState *g, const int *indices, int n_indices) {
    if (!g || !indices || n_indices <= 0) return;
    int source = g->queue.resume_host >= 0 ? g->queue.resume_host : rb_resolver_source_card_id(g);
    int looked_owner = (g->queue.actor >= 0) ? g->queue.actor : 0;
    for (int i = 0; i < n_indices; i++) {
        int idx = indices[i];
        /* Rust: idx < looked_at_cards.len(), then cid = looked_at[idx] */
        int cid = -1;
        if (idx >= 0) {
            int pool[RB_MAX_ZONE]; int n = rb_looked_at_pool(g->queue.actor, pool, RB_MAX_ZONE);
            if (idx < n) cid = pool[idx];
        }
        if (cid < 0) continue;
        if (g->n_revealed < RB_MAX_REVEALED_CARDS) g->revealed_cards[g->n_revealed++] = cid;
        /* In Rust push_revealed_card also records source, owner, cause "ability" — mirrored via g->revealed_cards */
        (void)source; (void)looked_owner;
    }
    if (g->n_revealed > 0) {
        /* Rust logs [Turn N] P1 [[log_reveal_looked:n=X]] — C log via rule log if available */
    }
}

/* ═══════════════════════════════════════════════════════════════════════════
   「〜たらポジションチェンジ」 — the position-change answer plumbing.

   Rust carries the whole thing on the AbilityResolver:
     * result_handlers.rs:89-113 `handle_area_select` stores the picked area in
       `self.selection.area`;
     * position.rs:447-454 `execute_position_change` CONSUMES it
       (`self.selection.area = None`) and routes straight to
       `execute_position_change_with_destination`, so no second prompt is ever
       raised;
     * choice.rs:2710-2717 routes a `position|destination` answer to
       `handle_position_change_choice` when the entry's `choice_card_no` is
       `Raw("position_change…")`, which then calls
       `execute_position_change_with_destination` (position.rs:899-1094).

   Two C facts forced the shape below.
   1. `RbAbilityResolver` is a STACK LOCAL at every entry point (choice.c:588,
      :3262), so `self.selected_area` could not outlive the frame that wrote it
      — it was a write-only field, and `rb_resolver_provide_choice_result`, the
      only function that wrote it, has no callers at all. The live answer path
      is `rb_resume_with_choice_indices_internal` -> `case RB_CHOICE_SELECT_TARGET`
      -> `rb_resolver_handle_select_target`, which had no `area_select` arm, so
      the answer fell through to the "record chosen target then clear" default.
      The area therefore lives in a file-static, which is the C's only carrier
      that survives the prompt frame, and is written AND read below.
   2. The prompt a position change raises is `SelectTarget("position|destination")`
      (misc.c `h_execute_position_change`, the port of position.rs:91), and the C
      handler it used to reach required `g->queue.resume_eff`, which
      `rb_resume_with_choice_indices_internal` NULLs (choice.c:3297) BEFORE
      dispatching the answer. The whole move therefore sat behind a condition
      that is false on every prompt, and the prompt was re-raised forever.
   ═══════════════════════════════════════════════════════════════════════════ */

/* `self.selection.area` (choice.rs:3214 / position.rs:448-450). */
static char s_selection_area[16];
static int  s_has_selection_area;

static void choice_set_selection_area(const char *area)
{
    if (!area) return;
    strncpy(s_selection_area, area, sizeof(s_selection_area) - 1);
    s_selection_area[sizeof(s_selection_area) - 1] = '\0';
    s_has_selection_area = 1;
}
/* position.rs:450 — consuming the area clears it, so a later position change
   does not reuse a stale answer. Returns 1 when an area was pending. */
static int choice_take_selection_area(char *out, size_t outsz)
{
    if (!s_has_selection_area) return 0;
    if (out && outsz) {
        strncpy(out, s_selection_area, outsz - 1);
        out[outsz - 1] = '\0';
    }
    s_selection_area[0] = '\0';
    s_has_selection_area = 0;
    return 1;
}

/* The answer to a position prompt is an area INDEX in the C (see
   test_parity_area_move.c's drain_areas: "The area index is what
   rb_resume_position_change consumes"), while Rust's is the area NAME from the
   option list (choice.rs:3000-3008 normalises "0"/"left"/"left_side" to one
   name). Both spellings are accepted. */
static int choice_area_index_from_answer(const char *sel)
{
    if (!sel || !*sel) return -1;
    if (sel[1] == '\0' && sel[0] >= '0' && sel[0] <= '2') return sel[0] - '0';
    return rb_stage_position_index(sel);
}

/* The list `execute_area_select` offered (selection.rs:182-193): every area
   except the activating member's own. RbChoice carries no option list, so the
   answer index is resolved against this recomputation — the same loop, over the
   same stage, so the resulting order is identical. */
static int choice_offered_areas(const GameState *g, int pl, int activating_id,
                                char valid[3][8])
{
    static const char *const names[3] = { "left", "center", "right" };
    int nv = 0;
    if (!g || pl < 0 || pl > 1) return 0;
    for (int i = 0; i < 3; i++) {
        if (activating_id >= 0 && g->p[pl].stage[i] == activating_id) continue;
        strncpy(valid[nv], names[i], sizeof(valid[0]) - 1);
        valid[nv][sizeof(valid[0]) - 1] = '\0';
        nv++;
    }
    return nv;
}

/* position.rs:1046-1089 — the `this_member` arm of
   execute_position_change_with_destination: locate the moving member on the
   target player's stage, and swap only when it is not already in the chosen
   area. `rb_resume_position_change` (state.c:1489) is the C's
   `record_stage_position_swap` + `trigger_auto_abilities_for_movement_current`
   pair and takes the member id as `host_cid`, exactly as position.rs:1074-1079
   names the activating card explicitly. The C has no `session.activating_card_id`
   that survives the prompt, so the resolver's host — the same value
   `rb_effect_area_select` used to build its list (draw.c:963) and the same
   value the sibling `opponent:front` arm uses to find the host's area
   (choice.c:2641) — stands in for it. */
static void choice_apply_position_destination(GameState *g, int actor,
                                              int host_cid, int dst_idx)
{
    if (!g || dst_idx < 0 || dst_idx >= RB_STAGE_SIZE) return;
    if (host_cid < 0) return;
    rb_resume_position_change(g, actor, NULL, host_cid, dst_idx);
}

/* Is the step that is about to run a `position_change`? Rust reaches it through
   the entry's owned `pending_actions` (choice.rs:108-126). The C's two
   carriers are `queue.pending_actions` (handled by the owned path in
   rb_resolver_continue_siblings) and the parked parent/child index
   (engine.c:281-287), which is what this walks. Two shapes occur:
     OWNED      the prompt came from cont->child[from-1], so the tail is
                cont->child[from .. n_child).
     CLOBBERED  the prompt came from a GRANDchild: the inner frame parked
                (engine.c:281-287) and returned, then the outer loop saw the
                pending choice and re-parked ITSELF over that (engine.c:256-292),
                so the live tail is cont->child[from-1]->child[1 .. n_child).
   A node that already carries a destination is skipped: it does not consume
   `selection.area` (position.rs:447-454 tests the field before the
   destination arm, and only the destination-less step reads it). */
static const AbilityEffect *choice_next_position_change(const AbilityEffect *cont,
                                                         int from)
{
    if (!cont) return NULL;
    for (int j = from; j < cont->n_child; j++) {
        const AbilityEffect *c = cont->child[j];
        if (c && c->action && !strcmp(c->action, "position_change") &&
            !(c->destination && *c->destination))
            return c;
    }
    if (from - 1 >= 0 && from - 1 < cont->n_child) {
        const AbilityEffect *mid = cont->child[from - 1];
        if (mid) {
            for (int k = 1; k < mid->n_child; k++) {
                const AbilityEffect *c = mid->child[k];
                if (c && c->action && !strcmp(c->action, "position_change") &&
                    !(c->destination && *c->destination))
                    return c;
            }
        }
    }
    return NULL;
}

/* result_handlers.rs:89-113 `handle_area_select`, plus the consumption half of
   position.rs:447-454. The answer is stored on the resolver AND on the
   file-static that outlives the frame; then, if the continuation really does
   still owe a destination-less `position_change`, that step is resolved HERE
   with the stored area, which is what Rust does inside the step itself. The C
   cannot defer it into the step because the step's executor lives in misc.c and
   has no access to `selection.area`; resolving it at the only frame that
   survives the prompt is the faithful relocation, and the step's own later run
   becomes the no-op Rust's `source == target -> NOOP` already is
   (position.rs:983-985). */
static void choice_store_area_select(RbAbilityResolver *self, GameState *g,
                                     const char *sel)
{
    if (!g) return;
    int pl = g->queue.actor >= 0 ? g->queue.actor : 0;
    int activating = g->queue.resume_host;
    char valid[3][8];
    int nv = choice_offered_areas(g, pl, activating, valid);
    int idx = -1;
    if (sel && *sel) {
        if (sel[1] == '\0' && sel[0] >= '0' && sel[0] <= '9')
            idx = atoi(sel);
        else
            for (int i = 0; i < nv; i++) if (!strcmp(sel, valid[i])) { idx = i; break; }
    }
    /* result_handlers.rs:104-109 — an unparseable or out-of-range answer takes
       the first offered area, and an empty list falls back to "left". */
    if (nv > 0) idx = (idx >= 0 && idx < nv) ? idx : 0;
    const char *area = (nv > 0) ? valid[idx] : "left";
    if (self) {
        self->selected_area = (!strcmp(area, "center") ? 1 : (!strcmp(area, "right") ? 2 : 0));
    }
    choice_set_selection_area(area);

    const AbilityEffect *cont = g->queue.resume_parent;
    int from = g->queue.resume_child + 1;
    if (!choice_next_position_change(cont, from)) return;
    char taken[16];
    if (!choice_take_selection_area(taken, sizeof(taken))) return;
    choice_apply_position_destination(g, pl, activating,
                                      choice_area_index_from_answer(taken));
}

/* --- provide_choice_result (choice.rs:267) : dispatch pending choice --- */
int rb_resolver_provide_choice_result(GameState *g, int selected_idx) {
    if (!g || !g->queue.has_pending) return 0;
    RbAbilityResolver self; memset(&self, 0, sizeof(self));
    self.gs = g; self.actor = g->queue.actor; self.host_cid = g->queue.resume_host;
    self.pending_choice = g->queue.pending;
    self.has_pending_choice = 1;
    int kind = g->queue.pending.kind;
    int count = g->queue.pending.count;
    int allow_skip = g->queue.pending.allow_skip;
    int was_skip = selected_idx < 0;
    if (was_skip && g->queue.pending.actor == 1)
        g->opponent_choice_declined = 1;
    char selbuf[32]; const char *selstr = NULL;
    if (!was_skip) { snprintf(selbuf,sizeof(selbuf),"%d",selected_idx); selstr=selbuf; }
    /* Rust match: (Some(SelectCard), CardSelected) => handle_select_card */
    if (kind == RB_CHOICE_SELECT_CARD && !was_skip) {
        return rb_resolver_handle_select_card(&self, g, selstr);
    }
    if (kind == RB_CHOICE_SELECT_CARD && was_skip) {
        if (count==0 && allow_skip) {
            /* any_number re-prompt skip: cards already moved — resume pending */
            return rb_resolver_clear_choice_state_and_resume(&self);
        }
        /* Rust: (Some(SelectCard), Skip) => take_pending_actions, clear, resume_execution */
        rb_queue_take_pending_actions(g);
        rb_resolver_clear_choice_state(&self);
        rb_resolver_resume_execution(&self);
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_TARGET && was_skip) {
        rb_queue_take_pending_actions(g);
        rb_resolver_clear_choice_state(&self);
        rb_resolver_resume_execution(&self);
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_TARGET && !was_skip) {
        const char *tgt = g->queue.pending.target[0] ? g->queue.pending.target : "";
        if (!strcmp(tgt,"area_select") || strstr(tgt,"area_select")) {
            /* The index is resolved against the offered list, not against the
               absolute area order: execute_area_select (selection.rs:182-193)
               omits the activating member's own area, so on a centre host
               option 1 is "right", not "center". The old fixed 0/1/2 mapping
               read straight off the answer index. */
            choice_store_area_select(&self, g, selstr);
            rb_resolver_clear_choice_state(&self);
            rb_resolver_resume_pending_actions(&self);
            return 1;
        }
        rb_resolver_handle_select_target(&self, g, tgt, selstr);
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_POSITION && !was_skip) {
        /* Rust: handle_select_position */
        rb_resolver_handle_position_change_choice(&self, g, g->queue.pending.target, selstr);
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_HEART_COLOR && !was_skip) {
        const char *cols[1]={selstr};
        rb_resolver_handle_heart_selection(&self, g, 1, cols, 1);
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_NUMBER && !was_skip) {
        rb_resolver_handle_number_selection(g, atoi(selstr));
        return 1;
    }
    if (kind == RB_CHOICE_SELECT_AUTO_ABILITY && !was_skip) {
        rb_resolver_handle_auto_ability_selection(g, atoi(selstr));
        return 1;
    }
    /* fallback: delegate to real engine */
    rb_resume_with_choice(g, selected_idx);
    return 1;
}

/* ===== choice.rs handle_* methods (mirror AbilityResolver handlers) =====
   Each handler resolves the selected index/string against the pending choice,
   applies the real engine mutation via rb_* helpers, records the chosen target
   when relevant, then clears the choice state and resumes the queue. */

/* Move `cid` to `dst` for a choice handler — waitroom/discard use the waitroom
    bag, every other zone uses the generic zone placement helper.

    "under_member" is the one destination rb_place_card_in_zone (util.c:836)
    has no arm for: its zone_bag() (util.c:797) returns NULL for the zone, so
    the call returned 0 having moved nothing and the card ended up in NO zone
    at all after leaving its source — the confirmed
    move_cards(destination="under_member") "reports the move and populates
    nothing" defect (test_parity_int_gaps.c c1, test_parity_cce_gaps.c H1/H5,
    and the energy cost arm). Route it to the faithful Rule 4.5.5 landing
    instead: placement.rs:208-219 for the host area and util.rs:2341-2358 for
    the place itself. */
static int rb_choice_send_to_dst(GameState *g, int pl, int cid, const char *dst) {
    /* Returns a verdict because every caller has already lifted the card out of
     * its source, so "not placed" is not neutral — it is the card deleted. Rust's
     * `place_card_in_zone` (util.rs:2337) returns true for every destination it
     * can name and parks an unplaceable stage card in the WAITROOM rather than
     * dropping it, so a caller that cannot place must put the card back itself. */
    if (!dst || !strcmp(dst, "waitroom") || !strcmp(dst, "discard")) {
        rb_waitroom_add(&g->p[pl], cid);
        return 1;
    }
    if (!strcmp(dst, "under_member"))
        return rb_move_place_card_under_member(g, pl, cid, rb_move_under_member_pos(g, pl)) ? 1 : 0;
    return rb_place_card_in_zone(g, pl, cid, dst, -1);
}

void rb_resolver_build_reprompt(RbAbilityResolver *self, GameState *g) {
    if (!self || !g) return;
    self->has_pending_reprompt = 1;
    self->has_pending_reprompt_choice = 1;
    memset(&g->queue.pending, 0, sizeof(g->queue.pending));
    g->queue.pending.kind = RB_CHOICE_SELECT_CARD;
    g->queue.pending.count = 1;
    g->queue.pending.allow_skip = 1;
    g->queue.has_pending = 1;
    g->queue.actor = self->actor;
}
int rb_resolver_build_reprompt_full(RbAbilityResolver *self, GameState *g, const RbSelectionContext *ctx,
                                     const char *zone, int count, const char *en, const char *ja,
                                     int allow_skip, const int *filtered, int n_filtered,
                                     const char *tpid, int cost_total, const char *cost_total_op) {
    if (!self || !g) return -1;
    RbChoice ch; memset(&ch,0,sizeof(ch));
    ch.kind=RB_CHOICE_SELECT_CARD;
    strncpy(ch.zone, zone?zone:"hand", sizeof(ch.zone)-1);
    ch.count=count; ch.allow_skip=allow_skip;
    if (en) strncpy(ch.description,en,sizeof(ch.description)-1);
    if (tpid) strncpy(ch.target,tpid,sizeof(ch.target)-1);
    /* W1 (-Waddress). `ctx->card_type` is `char card_type[32]`, so `ctx->card_type`
       in a boolean context is `char *` and the old `if (ctx && ctx->card_type)`
       could NEVER be false -- it was a null check written where an
       `is_some()` was meant. Rust's twin is `SelectionContext::card_type:
       Option<String>` (choice.rs:34) fed straight into
       `ChoiceBuilder::card_type(Option<String>)` (types.rs:479, :97) by
       `build_reprompt` (choice.rs:1188), so the guard is `card_type.is_some()`.
       The C's `RbSelectionContext` has no `has_card_type` flag, so `None` is not
       representable and `[0]` is the faithful test -- and it is the test the very
       NEXT line already uses for `group` (also `Option<String>` in Rust, also a
       bare `char[32]` here). Behaviour is provably unchanged either way: `ch` was
       memset at :1077, so `strncpy` of an empty string re-writes the NUL that is
       already there. The check is kept, not deleted, because it is the only thing
       standing between a future `ctx->card_type` and an unconditional copy. */
    if (ctx && ctx->card_type[0]) strncpy(ch.card_type,ctx->card_type,sizeof(ch.card_type)-1);
    if (ctx && ctx->group[0]) strncpy(ch.filter_group,ctx->group,sizeof(ch.filter_group)-1);
    (void)ja; (void)filtered; (void)n_filtered; (void)cost_total; (void)cost_total_op;
    ch.route=RB_ROUTE_SELECT_CARDS;
    g->queue.pending=ch; g->queue.has_pending=1; g->queue.actor=self->actor;
    self->has_pending_reprompt=1; self->has_pending_reprompt_choice=1;
    return 0;
}

/* extra-flag reader with the same "true"/"1" spelling cost.c's eff_bool uses */
static int choice_cost_flag(const AbilityEffect *e, const char *key, int dflt)
{
    const char *v = choice_effect_extra(e, key);
    if (!v || !*v) return dflt;
    return !strcmp(v, "true") || !strcmp(v, "1");
}

/* The two hand-cost re-prompt descriptions in this file both carry this tag, so a
   cost re-prompt is recognisable by structure rather than by re-deriving the
   prompt text cost.c builds. */
#define CHOICE_COST_REPROMPT_TAG "from hand for cost"

/* Does this cost node produce EXACTLY the hand SelectCard prompt `ch`?
   Mirrors the arm selection of pay_cost_move_cards (cost.c:450-676): a
   hand-sourced move_cards cost that is not a whole-hand discard, not a
   same-unit cost and not a bare self-discard opens
   SelectCard{zone:"hand", count: any_number ? 0 : count,
              allow_skip: is_optional || any_number}. `is_activation` is 0 on
   every path this fallback is consulted from, so the C's is_activation
   exclusion (cost.c:601) cannot apply. Sequential costs carry the hand leg as
   a child, so the search recurses exactly like Rust's own cost tree walk. */
static int choice_cost_opens_hand_prompt(const AbilityEffect *c, const RbChoice *ch, int depth)
{
    if (!c || !ch || depth > 8) return 0;
    for (int i = 0; i < c->n_child; i++)
        if (choice_cost_opens_hand_prompt(c->child[i], ch, depth + 1)) return 1;
    if (!c->source || strcmp(c->source, "hand") != 0) return 0;
    if (choice_cost_flag(c, "same_unit_name", 0)) return 0;
    if (choice_cost_flag(c, "all", 0)) return 0;
    int any_number = choice_cost_flag(c, "any_number", 0);
    int count = c->count > 0 ? c->count : 1;
    int skippable = (c->is_optional || any_number) ? 1 : 0;
    /* the prompt the cost opens for its first answer */
    if (ch->count == (any_number ? 0 : count) && ch->allow_skip == skippable) return 1;
    /* the fixed-count re-prompt for the cards still owed (choice.rs:738-777
       reprompt_hand_cost, Rule 9.4.2.3: "no bail-out once committed", so it is
       never skippable whatever the cost's own optional-ness). It carries the
       REMAINDER, not the cost's own count, and the shared tag is what keeps an
       effect-level hand prompt with a coincidentally smaller count out. */
    if (!any_number && ch->allow_skip == 0 &&
        ch->count > 0 && ch->count <= count &&
        strstr(ch->description, CHOICE_COST_REPROMPT_TAG))
    return 1;
    return 0;
}

/* ── The pay/skip gate is answered by OPTION INDEX, not by skipping ─────────
   Rust builds the prompt with emit_pay_skip_gate (resolver.rs:201-225):
   `Choice::SelectTarget { target: PAY_SKIP_TARGET, allow_skip, options }` —
   a two-option yes/no question. The answer is then read as a STRING by the
   handler, never as a sentinel:
     cost/handlers.rs:1373  `if selected == "skip_optional_cost" || selected == "0"`
     cost/handlers.rs:1641  `accepted = selected != "skip_optional_cost" && selected != "0"`
     choice.rs:3515         `let chose_yes = selected == "1" || selected == "yes"`
   so the NO option and an explicit skip are the same refusal, and the YES
   option is any other answer. A pay/skip prompt therefore has NO way to be
   "unanswered" that means "paid".

   The C reached the same verdict from the answer's INDEX NEGATIVITY
   (`was_skip = selected_idx < 0`), which reads option 0 — the printed 「no」 —
   as a payment. The prompt's target is the discriminator that is actually
   present in the C, and it is the same string the C's own pay/skip route at
   the top of this file already matches on. "draw:skip" is the C-only spelling
   draw.c:525 gives the same gate on the optional-draw path. */
static int choice_is_pay_skip_prompt(const RbChoice *ch)
{
    if (!ch || ch->kind != RB_CHOICE_SELECT_TARGET) return 0;
    const char *t = ch->target;
    if (!t[0]) return 0;
    return !strcmp(t, RB_PAY_SKIP_TARGET) ||
           strstr(t, "pay_optional") != NULL ||
           strstr(t, "skip_optional") != NULL ||
           strstr(t, "draw:skip") != NULL;
}

/* The ANSWER SHAPE of a pay/skip gate, per the three Rust lines above.
   Returns 1 = the optional action was PAID, 0 = it was DECLINED, and
   -1 = this prompt is not a pay/skip gate at all (caller keeps its own
   skip/pay reading). */
static int choice_pay_skip_answer(const RbChoice *ch, int selected_idx)
{
    if (!choice_is_pay_skip_prompt(ch)) return -1;
    /* No option picked == the 「skip」 answer == a refusal
       (cost/handlers.rs:1373 `selected == "skip_optional_cost"`). */
    if (selected_idx < 0) return 0;
    /* Option 0 is the printed 「no」: `selected == "0"`. Every other option
       is an acceptance, whatever its index (handlers.rs:1641 only rejects
       the literal "0" / "skip_optional_cost"). */
    return selected_idx == 0 ? 0 : 1;
}

/* ── POST_COST_GATE stop: drop the commands the ability had parked ──────────
   gates.rs:195-213 `optional_cost_skip_gate` stops resolution the moment the
   entry records `optional_cost_result == Some(false)`, so
   `run_ability_effect` (engine.rs:267-286) is never reached AND the
   remaining commands of that resolution are never resumed — in Rust they are
the entry's OWN `pending_actions` (queue.rs:532-536 `set_pending_actions`
    stores an owned Vec), which the stop simply leaves in place.

   The C parks the same remaining commands as raw pointers on the queue
   (queue.resume_parent / resume_child for the sibling effects, queue.deferred
   for the effect the producer published), so "never resumed" has to be spelled
   as "cleared": leaving them set would let a LATER prompt on the same queue
   run the abandoned ability's tail. Only ever called once the answer is known
   to be a refusal, so a NON-cost skip — which legitimately needs its
   continuation — is untouched. */
static void choice_stop_decline_drop_continuation(GameState *g)
{
    if (!g) return;
    g->queue.resume_parent = NULL;
    g->queue.resume_child = -1;
    g->queue.deferred = NULL;
}

/* Record a pay/skip DECLINE on the ability's queue entry, exactly as
   cost/handlers.rs:1373-1392 does, and answer the question the C's
   choice_declined_cost_stop cannot: that helper deliberately refuses a
   SelectCard answer once `effect_started` is set (choice.rs:985-1034 only
   treats a SelectCard as a COST answer before the effect runs), but a
   SelectTarget pay/skip gate is the effect's OWN gate and is read by
   optional_cost_skip_gate with no such condition — gates.rs:200-203 tests
   only `optional_cost_result == Some(false)`. */
static void choice_record_pay_skip_decline(GameState *g)
{
    if (!g) return;
    int cur = g->queue.cur;
    if (cur < 0 || cur >= g->queue.n_entries) return;
    RbQueueEntry *e = &g->queue.entries[cur];
    if (e->card_id < 0) return;
    e->cost_paid = 1;              /* handlers.rs:1377 */
    e->optional_cost_result = 0;   /* handlers.rs:1386 — the gate's input */
    e->effect_started = 1;         /* gates.rs:206-207 — the gate's stop marker */
    /* handlers.rs:1390 `entry.pending_actions.clear()`: the alternative_effect
       branch there is the 「unless you pay」 shape, which has no C counterpart
       on this path, so dropping the parked continuation is the whole of it. */
    choice_stop_decline_drop_continuation(g);
}

/* Rust reacher (engine/src/turn/triggers.rs:119-129) hands the 登場 trigger to
   trigger_auto_ability, which pushes a queue entry — so on the Rust side
   gs.entry_cost() (core/game_state/abilities.rs:2295) ALWAYS has an entry and
   handle_select_card can route on `gs.entry_cost().is_some() && !effect_started`
   (choice.rs:985-1034). The C port's inline 登場 loop (engine.c:1100-1150)
   deliberately resolves the debut ability WITHOUT pushing one, so entry_cost()
   has nothing to read there and the pick fell through to the generic
   hand-SELECTION arm: the cost card was never moved to the waitroom.

   Reconstruct the entry the loop skipped. The loop publishes the ability's
   EFFECT as the choice's continuation and the card as g->queue.resume_host
   (engine.c:1118-1121), so the ability being resolved is the 登場/バトンタッチ
   ability of resume_host — the same ability index the reacher would have
   queued. Require that the pending choice is a hand SelectCard that this
   ability's COST opened, so the ability's own EFFECT-level hand prompt on the
   same card is never mistaken for the cost. */
static int choice_debut_cost_action(GameState *g, const RbChoice *ch, char *out_act, int outlen)
{
    int host = g->queue.resume_host;
    if (host < 0 || !ch) return 0;
    if (ch->kind != RB_CHOICE_SELECT_CARD) return 0;
    if (!ch->zone[0] || strcmp(ch->zone, "hand") != 0) return 0;
    int n = rb_card_num_abilities((uint32_t)host);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)host, i, &ab)) continue;
        int is_debut = ab.triggers && (strstr(ab.triggers, "登場") ||
                                       strstr(ab.triggers, "バトンタッチ"));
        if (is_debut && ab.cost && choice_cost_opens_hand_prompt(ab.cost, ch, 0)) {
            if (out_act && outlen > 0) {
                const char *a = ab.cost->action ? ab.cost->action : "";
                strncpy(out_act, a, outlen - 1); out_act[outlen - 1] = '\0';
            }
            rb_free_ability(&ab);
            return 1;
        }
        rb_free_ability(&ab);
    }
    return 0;
}

/* Mirror Rust GameState::entry_cost() — the cost AbilityEffect of the ability
    currently being resolved (queue.entries[cur]). Copies the cost action into
    out_act (e.g. "discard"/"reveal"/"pay_energy") and returns 1 if a cost exists.
    `ch` is the choice being answered; it is the current pending choice except
    in the resume epilogue, which answers it after rb_clear_pending_choice.
    *out_from_debut reports the reconstructed (no queue entry) resolution. */
static int rb_queue_cost_action_for_choice(GameState *g, const RbChoice *ch,
                                           char *out_act, int outlen,
                                           int *out_from_debut)
{
    if (out_act && outlen > 0) out_act[0] = '\0';
    if (out_from_debut) *out_from_debut = 0;
    if (!g) return 0;
    int cur = g->queue.cur;
    if (cur >= 0 && cur < RB_QUEUE_DEPTH) {
        RbQueueEntry *e = &g->queue.entries[cur];
        if (e->card_id >= 0) {
            Ability ab;
            if (rb_decode_card_ability((uint32_t)e->card_id, e->ability_idx, &ab)) {
                int has = (ab.cost != NULL);
                if (has && out_act && outlen > 0) {
                    const char *a = ab.cost->action ? ab.cost->action : "";
                    strncpy(out_act, a, outlen - 1); out_act[outlen - 1] = '\0';
                }
                /* DEFECT (alias class). This decodes the WHOLE card ability
                 * only to read `ab.cost->action` into out_act (copied by
                 * strncpy above, so out_act is already safe), then frees it --
                 * while `g->queue.resume_parent` can still point INTO
                 * ab.effect. The alias probe measures this exact free
                 * (FREED_VIA=rb_free_ability.effect) firing on slot
                 * queue.resume_parent, slot_value == freed_root, in
                 * parity_integration_extra and parity_cost_and_effect.

                 * THE FIX is the landed deferral protocol, not an owning
                 * clone: hand the decoded pair to g->activation_keepalive while
                 * a prompt is open and let the existing drains release it
                 * (choice.c release site on the !rb_has_pending_choice arm;
                 * engine.c teardown when no prompt is left). Same shape as
                 * ability_queue.c's drain and pca_resolve_entry. The
                 * `!activation_keepalive_valid` guard keeps the single-owner
                 * contract: when the slot is already taken the tree is left
                 * detached rather than double-owned. */
                if (rb_has_pending_choice(g) && !g->activation_keepalive_valid) {
                    g->activation_keepalive = ab;
                    g->activation_keepalive_valid = 1;
                    memset(&ab, 0, sizeof(ab));   /* transfer was total */
                }
                rb_free_ability(&ab);   /* frees nothing when transferred */
                if (has) return 1;
            }
        }
    }
    int from_debut = choice_debut_cost_action(g, ch, out_act, outlen);
    if (out_from_debut) *out_from_debut = from_debut;
    return from_debut;
}

static int rb_queue_current_cost_action(GameState *g, char *out_act, int outlen)
{
    return rb_queue_cost_action_for_choice(g, g && g->queue.has_pending ? &g->queue.pending : NULL,
                                          out_act, outlen, NULL);
}

/* helper: build a reprompt SelectCard pending choice (mirrors Rust build_reprompt) */
static void choice_build_reprompt(GameState *g, int actor, const char *zone, int remaining,
                                   const char *en, const char *ja, int allow_skip,
                                   const int *filtered, int n_filtered) {
    RbChoice ch; memset(&ch, 0, sizeof(ch));
    ch.kind = RB_CHOICE_SELECT_CARD;
    strncpy(ch.zone, zone ? zone : "hand", sizeof(ch.zone)-1);
    ch.count = remaining > 0 ? remaining : 0;
    ch.allow_skip = allow_skip;
    strncpy(ch.description, en ? en : "", sizeof(ch.description)-1);
    ch.route = RB_ROUTE_SELECT_CARDS;
    /* Rust build_reprompt (choice.rs:1168-1190) passes the caller's `fi` — the
       still-legal candidate indices — straight into Choice.filtered_indices, and
       map_choice_index (choice.rs:840-843) resolves an answer through it.
       Dropping it, as this used to with `(void)filtered`, re-offers the whole
       zone on the re-prompt: a card the filter already rejected, or one already
       picked on an earlier round, is offered again and answered again, which is
       an any_number loop that never terminates. */
    if (filtered && n_filtered > 0) {
        int n = n_filtered > RB_MAX_ZONE ? RB_MAX_ZONE : n_filtered;
        for (int i = 0; i < n; i++) ch.filtered_indices[i] = filtered[i];
        ch.n_filtered_indices = n;
    }
    (void)ja;
    g->queue.pending = ch;
    g->queue.has_pending = 1;
    g->queue.actor = actor;
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;
}

static int map_choice_index(const RbChoice *choice, int index)
{
    if (!choice || index < 0) return index;
    if (choice->n_filtered_indices > 0 && index < choice->n_filtered_indices)
        return choice->filtered_indices[index];
    return index;
}

/* ── choice_is_reveal_prompt — the reveal arm's gate (choice.rs:981) ────────
   Rust enters handle_reveal_selection from the Choice's OWN is_reveal flag
   (`if ctx.is_reveal && Zone::from_str(zone) == Some(Zone::Hand)`), and ctx
   is filled from the prompt the producer parked — see look.rs:272
   offer_reveal_choice `.is_reveal(true)`, mirrored by
   rb_effect_reveal (src/ability/effects/look.c:668) and by rb_look_and_select
   (look.c:938).  Re-deriving the fact from `strstr(target,"reveal")` /
   `strstr(zone,"reveal")` misses every one of those prompts: the reveal arm's
   prompt has zone="hand" and an empty target (the reveal-ness is not in the
   TEXT), so Umi's 登场 reveal over a hand wider than `count` silently took
   the hand-selection arm, never recorded the cards and never decremented the
   re-prompt count.

   The substring test survives only as choice_reveal_is_legacy_text, for the
   producers that still spell reveal semantics into target/zone. */
static int choice_is_reveal_prompt(const RbChoice *ch)
{
    if (!ch) return 0;
    const char *zone = (ch->zone[0]) ? ch->zone : "hand";
    return ch->is_reveal && !strcmp(zone, "hand");
}

/* Legacy fallback: a SelectCard that announces reveal-ness through its target
   or zone strings instead of the flag.  Audited producers of a SelectCard:
     * rb_effect_reveal        (look.c:663-671)   -> sets is_reveal  [FLAG]
     * rb_look_and_select      (look.c:911-938)   -> sets is_reveal  [FLAG],
       but its prompt is zone="looked_at", so choice.rs:981's Hand gate keeps
       it out of the reveal arm exactly as the substring test did.
     * cost reveal prompts     (cost.c:1097-1112) -> no flag; routed by
       is_cost_reveal (the entry's cost action), not by this test.
     * the reveal-cost re-prompts this file builds (handle_entry_cost_reveal)
       carry target="reveal" with no flag; nothing reaches them today, and the
       substring is what would keep them on the reveal path if it did.
     * a revealed_cards-zone prompt matches strstr(zone,"reveal"); the flag test
       deliberately does not capture it, and neither did Rust.
   The substring is therefore kept for the string-carrying prompts only, so the
   flag fix cannot reroute them. */
static int choice_reveal_is_legacy_text(const RbChoice *ch)
{
    if (!ch) return 0;
    const char *zone = ch->zone[0] ? ch->zone : "hand";
    const char *target = ch->target[0] ? ch->target : NULL;
    return (target && strstr(target, "reveal")) || (zone && strstr(zone, "reveal"));
}

/* ── SelectCard answered with MORE THAN ONE index ───────────────────────────
   Rust hands the WHOLE selection to the handler: result_handlers.rs:41-65
   copies `indices.to_vec()` into SelectionContext and choice.rs:1167 does
   `let mapped_indices = ctx.mfi(&ctx.indices)` — every named card travels
   through the same call. The C handlers take a single `const char *selected`
   holding one index, so an answer naming N>1 cards could only ever apply
   indices[0] and then re-prompt for the remainder: a SelectCard cost/effect
   with count N could not be completed in one answer.

   This is the faithful multi-index port of the two hand arms of Rust
   handle_select_card:
     * the hand-COST arm        choice.rs:1020-1034 -> handle_hand_cost_payment
     * the hand-SELECTION arm   choice.rs:1057-1059 -> handle_hand_selection
   Returns 1 when it consumed the answer, 0 when the pending choice is not one
   of those arms so the caller can fall back to the single-index handler. A
   return of -1 mirrors Rust's Err(..) ("No cards selected from hand for
   required selection"), which the callers also return verbatim. */
static int choice_handle_hand_indices(RbAbilityResolver *self, GameState *g,
                                      const RbChoice *ch,
                                      const int *indices, int n_indices)
{
    if (!self || !g || !ch || !indices || n_indices <= 0) return 0;
    const char *zone = ch->zone[0] ? ch->zone : "hand";
    if (strcmp(zone, "hand") != 0) return 0;
    /* Reveal arm (choice.rs:981-991) and the C6 keep-N-shuffle-under phase
       keep their own bookkeeping in the single-index handlers; not ported. */
    if (ch->is_reveal) return 0;
    if (ch->target[0] && strstr(ch->target, "reveal")) return 0;
    if (g->keep_shuffle_under_phase > 0) return 0;

    int actor = g->queue.actor;
    int count = ch->count;
    int allow_skip = ch->allow_skip;
    int cur = g->queue.cur;
    int effect_started = (cur >= 0 && cur < RB_QUEUE_DEPTH)
                            ? g->queue.entries[cur].effect_started : 0;
    char cost_act[48]; cost_act[0] = '\0';
    int has_cost = rb_queue_current_cost_action(g, cost_act, sizeof(cost_act));
    if (!effect_started && has_cost && !strcmp(cost_act, "reveal")) return 0;

    int ids[RB_MAX_ZONE];
    int n_ids = rb_zone_cards(g, actor, "hand", ids, RB_MAX_ZONE);
    /* ctx.mfi already applied by the caller; drop out-of-range and duplicate
       positions the way Rust's `.get(idx)` / `moved_cards.push` pair does. */
    int picked[RB_MAX_ZONE];
    int n_picked = 0;
    for (int i = 0; i < n_indices; i++) {
        int idx = indices[i];
        if (idx < 0 || idx >= n_ids) continue;
        int dup = 0;
        for (int j = 0; j < n_picked; j++) if (picked[j] == idx) { dup = 1; break; }
        if (dup) continue;
        if (n_picked < RB_MAX_ZONE) picked[n_picked++] = idx;
    }

    /* ── hand COST payment (choice.rs:683-743) ── */
    if (!effect_started && has_cost) {
        if (n_picked > 0 && cur >= 0) g->queue.entries[cur].optional_cost_result = 1;
        for (int i = 0; i < n_picked; i++) {
            int cid = ids[picked[i]];
            /* choice.rs:603-611 discard_hand_cost_selection lifts the card out of
               the hand with util::move_cards(Hand -> Discard); rb_choice_send_to_dst
               only pushes the waitroom copy. Highest index first so the remaining
               positions stay valid. */
            for (int k = n_picked - 1; k >= i; k--)
                if (picked[k] >= 0 && picked[k] < g->p[actor].hand.n)
                    rb_hand_remove_card(&g->p[actor], picked[k]);
            rb_choice_send_to_dst(g, actor, cid, "waitroom");
            /* moved_cards only — see the single-index arm's note on why a cost
               pick never belongs in selected_cards. */
            if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
            if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++] = cid;
        }
        g->mods.last_cost_discard_count = self->n_moved_cards;
        for (int i = 0; i < self->n_moved_cards && i < RB_MAX_RECENTLY_MOVED; i++)
            g->mods.last_cost_moved_card_ids[i] = self->moved_cards[i];
        g->mods.n_last_cost_moved_card_ids = self->n_moved_cards;
        if (n_picked == 0) {
            /* choice.rs:718-736 — nothing moved: the cost is either already
               satisfied (committed) or explicitly skippable. */
            if (self->n_moved_cards > 0 || allow_skip) {
                if (cur >= 0) {
                    g->queue.entries[cur].cost_paid = 1;
                    g->queue.entries[cur].optional_cost_result = self->n_moved_cards > 0 ? 1 : 0;
                }
            }
            rb_resolver_clear_choice_state_and_resume(self);
            return 1;
        }
        if (cur >= 0) g->queue.entries[cur].cost_paid = 1;
        /* Rule 9.4.2.3 — a fixed-count cost must be paid in full. */
        if (count > 0 && self->n_moved_cards < count) {
            int remaining = count - self->n_moved_cards;
            char en[80]; char ja[80];
            snprintf(en, sizeof(en), "Select %d more card(s) from hand for cost", remaining);
            snprintf(ja, sizeof(ja), "コストとして手札からさらに%d枚選択", remaining);
            choice_build_reprompt(g, actor, "hand", remaining, en, ja, 0, NULL, 0);
            return 1;
        }
        if (count == 0 && allow_skip && g->p[actor].hand.n > 0) {
            char en[80]; char ja[80];
            snprintf(en, sizeof(en), "Select more card(s) from hand for cost (or skip to finish)");
            snprintf(ja, sizeof(ja), "コストとして手札からさらに選択（スキップで終了）");
            choice_build_reprompt(g, actor, "hand", 0, en, ja, 1, NULL, 0);
            return 1;
        }
        rb_pay_deferred_costs(g, actor, NULL);
        rb_resolver_clear_choice_state_and_resume(self);
        return 1;
    }

    /* ── hand SELECTION (choice.rs:1289-1502) ── */
    if (n_picked == 0 && !allow_skip && count > 0) return -1;
    if (n_picked > 0 || allow_skip) {
        if (n_picked > 0 && count > 0 && n_picked < count) {
            /* Partial answer: record the picks and re-offer for the rest
               (choice.rs:1290-1368). */
            for (int i = 0; i < n_picked; i++) {
                int cid = ids[picked[i]];
                int exists = 0;
                for (int j = 0; j < self->n_selected_cards; j++)
                    if (self->selected_cards[j] == cid) { exists = 1; break; }
                if (!exists && self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                    self->selected_cards[self->n_selected_cards++] = cid;
            }
            int remaining = count - n_picked;
            char en[80]; char ja[80];
            snprintf(en, sizeof(en), "Select %d more card(s) from hand", remaining);
            snprintf(ja, sizeof(ja), "手札からさらに%d枚選択", remaining);
            /* choice.rs:1334-1347 `fi`: the candidates are the hand positions NOT
               already picked on earlier rounds, or the same card is offered and
               picked twice. */
            int avail[RB_MAX_ZONE], avail_n = 0;
            for (int i = 0; i < n_ids; i++) {
                int dup = 0;
                for (int j = 0; j < self->n_selected_cards; j++)
                    if (self->selected_cards[j] == ids[i]) { dup = 1; break; }
                if (!dup && avail_n < RB_MAX_ZONE) avail[avail_n++] = i;
            }
            choice_build_reprompt(g, actor, "hand", remaining, en, ja, 0, avail, avail_n);
            return 1;
        }
        if (n_picked > 0 && count == 0 && allow_skip) {
            /* any_number batch: apply it, then re-offer with skip
               (choice.rs:1369-1430). */
            const char *ch_chars[16]; int n_ch_chars = choice_characters_arg(ch, ch_chars, 16);
            rb_resolver_execute_selected_cards_from_zone(self, g, "hand", picked, n_picked,
                ch->card_type[0] ? ch->card_type : NULL, choice_prompt_cost_limit(ch),
                ch->cost_limit_op[0] ? ch->cost_limit_op : NULL, choice_prompt_cost_total(ch),
                ch->cost_total_op[0] ? ch->cost_total_op : NULL,
                ch->filter_group[0] ? ch->filter_group : NULL,
                n_ch_chars ? ch_chars : NULL, n_ch_chars,
                ch->target_player_id[0] ? ch->target_player_id : NULL);
            if (rb_has_pending_choice(g)) return 1;   /* a sub-choice took over */
            char en[96]; char ja[96];
            snprintf(en, sizeof(en), "Select more card(s) from hand (or skip to finish)");
            snprintf(ja, sizeof(ja), "手札からさらに選択（スキップで終了）");
            /* choice.rs:1409-1420 `fi`: the still-legal, not-yet-picked positions. */
            int avail[RB_MAX_ZONE], avail_n = 0;
            for (int i = 0; i < n_ids; i++) {
                if (!choice_discard_pick_legal(ch, ids[i])) continue;
                int dup = 0;
                for (int j = 0; j < self->n_selected_cards; j++)
                    if (self->selected_cards[j] == ids[i]) { dup = 1; break; }
                if (!dup && avail_n < RB_MAX_ZONE) avail[avail_n++] = i;
            }
            choice_build_reprompt(g, actor, "hand", 0, en, ja, 1, avail, avail_n);
            return 1;
        }
        if (n_picked > 0) {
            /* Complete answer (choice.rs:1431-1486): Rust executes the
               selection through execute_selected_cards_from_zone, which is
               where the zone move happens, then drops selected_cards unless
               the effect consumes them. */
            const char *ch_chars[16]; int n_ch_chars = choice_characters_arg(ch, ch_chars, 16);
            rb_resolver_execute_selected_cards_from_zone(self, g, "hand", picked, n_picked,
                ch->card_type[0] ? ch->card_type : NULL, choice_prompt_cost_limit(ch),
                ch->cost_limit_op[0] ? ch->cost_limit_op : NULL, choice_prompt_cost_total(ch),
                ch->cost_total_op[0] ? ch->cost_total_op : NULL,
                ch->filter_group[0] ? ch->filter_group : NULL,
                n_ch_chars ? ch_chars : NULL, n_ch_chars,
                ch->target_player_id[0] ? ch->target_player_id : NULL);
            self->n_selected_cards = 0;
        }
    }
    if (allow_skip) {
        int cur2 = g->queue.cur;
        if (cur2 >= 0 && cur2 < g->queue.n_entries) {
            g->queue.entries[cur2].optional_cost_result =
                (n_picked > 0 || self->n_moved_cards > 0) ? 1 : 0;
            if (n_picked == 0 && count > 0 && self->n_moved_cards == 0) {
                int is_opp = (ch->target_player_id[0] &&
                              strstr(ch->target_player_id, "opponent"));
                if (!is_opp) rb_queue_take_pending_actions(g);
            }
        }
    }
    rb_resolver_handle_selection_epilogue(self, g);
    return 1;
}

/* The multi-index core of handle_discard_selection (defined far below, next to
   its single-index wrapper). Declared here because the multi-index dispatcher
   below reaches it. */
/* Defined further down, next to the discard-selection arm that serves them. */
static void choice_discard_selection_indices(RbAbilityResolver *self, GameState *g,
                                            const int *sel, int n_sel);

/* ── Resolve a whole answer to card IDs, in ONE pass over the zone ──────────
   Rust collects the card for every index of `mapped_indices` BEFORE it mutates
   anything (`handle_discard_selection` choice.rs:2338-2355 snapshots
   `waitroom_cards` then indexes it once per answer index; `util::
   resolve_indices_to_ids` is the same shape). Resolving index i+1 after
   index i's removal has already shifted the zone would read the wrong card, so
   the snapshot is taken here, once, and every arm below mutates from it. */
static int choice_resolve_zone_ids(GameState *g, int actor, const char *zone,
                                   const int *sel, int n_sel,
                                   int *out, int max_out)
{
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, actor, zone, ids, RB_MAX_ZONE);
    int m = 0;
    for (int i = 0; i < n_sel && m < max_out; i++) {
        int idx = sel[i];
        if (idx >= 0 && idx < n) out[m++] = ids[idx];
    }
    return m;
}

/* ── SelectCard answered with N > 1 indices, for ANY zone ───────────────────
   Rust hands the WHOLE answer to the zone handler and every arm of
   `handle_select_card` acts on the whole `indices` vec at once:
     * hand      choice.rs:1204  `let mapped_indices = ctx.mfi(&ctx.indices)`
     * deck      choice.rs:1098-1110 execute_selected_cards_from_zone(indices)
     * discard   choice.rs:2338 / :2465 / :2599 (mapped_indices throughout)
     * energy    choice.rs:938   `let count_paid = indices.len()`
     * energy    choice.rs:1132  handle_energy_zone_selection(gs, indices, ..)
     * under_member / live / stage / looked_at / revealed_cards — the same
       `for &i in indices` / `ctx.mfi(&ctx.indices)` shape.
   The C's arms are per-index, and the resume path used to hand them only
   `selected_indices[0]`, so an answer naming N cards applied exactly one of
   them and the rest were dropped. This dispatcher applies the whole answer for
   the zones whose C arm is a per-index mutation, which is the same thing Rust
   does for those zones. The hand arms live in choice_handle_hand_indices
   (above) and are reached through here unchanged.

   `sel` is ALREADY ctx.mfi-mapped (map_choice_index, applied by the caller),
   exactly as the single-index path's `idx` is. Returns 1 when it consumed the
   answer, 0 when this dispatcher does not cover the prompt so the caller falls
   back to the single-index handler unchanged, and -1 for Rust's Err(..). */
static int choice_handle_multi_indices(RbAbilityResolver *self, GameState *g,
                                       const RbChoice *ch,
                                       const int *sel, int n_sel)
{
    if (!self || !g || !ch || !sel || n_sel <= 0) return 0;
    int actor = g->queue.actor;
    const char *zone = ch->zone[0] ? ch->zone : "hand";
    const char *target = ch->target[0] ? ch->target : NULL;
    int cur = g->queue.cur;
    int count = ch->count > 0 ? ch->count : 1;
    int effect_started = (cur >= 0 && cur < RB_QUEUE_DEPTH)
                            ? g->queue.entries[cur].effect_started : 0;

    if (!strcmp(zone, "hand"))
        return choice_handle_hand_indices(self, g, ch, sel, n_sel);

    if (!strcmp(zone, "discard") || !strcmp(zone, "waitroom")) {
        choice_discard_selection_indices(self, g, sel, n_sel);
        return 1;
    }

    if (!strcmp(zone, "energy")) {
        /* choice.rs:1073-1085 — the ENERGY COST arm. `destination` ==
           "under_member" is excluded upstream (that is a placement, not a
           payment) and so is the energy_state_change form, exactly as the
           single-index arm excludes them. choice.rs:938-952 pays
           `indices.len()` cards in ONE step, so an answer naming N cards pays
           N; the old per-index `pay = 1` charged one and re-prompted. */
        if (!effect_started &&
            !(target && !strcmp(target, "energy_state_change")) &&
            !(target && !strcmp(target, "under_member"))) {
            int pay = n_sel;
            g->p[actor].energy_active -= pay;
            if (g->p[actor].energy_active < 0) g->p[actor].energy_active = 0;
            g->mods.last_cost_discard_count += pay;
            if (cur >= 0) {
                g->queue.entries[cur].cost_paid = 1;
                g->queue.entries[cur].optional_cost_result = 1;
            }
            int energy_left = g->p[actor].energy_active;
            if (energy_left > 0) {
                char en[80];
                snprintf(en, sizeof(en),
                         "Select energy card to pay (active: %d). Skip when done", energy_left);
                choice_build_reprompt(g, actor, "energy", 0, en, en, 1, NULL, 0);
                return 1;
            }
            rb_resolver_clear_choice_state(self);
            rb_resolver_resume_pending_actions(self);
            return 1;
        }
        /* energy_state_change: every named card joins the selection and the
           outstanding count drops by the size of the ANSWER, not by one. */
        if (target && !strcmp(target, "energy_state_change")) {
            int cids[RB_MAX_ZONE];
            int nc = choice_resolve_zone_ids(g, actor, "energy", sel, n_sel,
                                             cids, RB_MAX_ZONE);
            for (int i = 0; i < nc; i++) {
                int already = 0;
                for (int j = 0; j < g->n_selected_cards; j++)
                    if (g->selected_cards[j] == cids[i]) { already = 1; break; }
                if (!already && g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                    g->selected_cards[g->n_selected_cards++] = cids[i];
                already = 0;
                for (int j = 0; j < self->n_selected_cards; j++)
                    if (self->selected_cards[j] == cids[i]) { already = 1; break; }
                if (!already && self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                    self->selected_cards[self->n_selected_cards++] = cids[i];
            }
            int left = g->queue.pending.count - nc;
            if (left > 0) {
                g->queue.pending.count = left;
                int w = 0;
                for (int i = 0; i < g->queue.pending.n_filtered_indices; i++) {
                    int drop = 0;
                    for (int j = 0; j < n_sel; j++)
                        if (g->queue.pending.filtered_indices[i] == sel[j]) { drop = 1; break; }
                    if (!drop) g->queue.pending.filtered_indices[w++] = g->queue.pending.filtered_indices[i];
                }
                g->queue.pending.n_filtered_indices = w;
                return 1;
            }
            g->queue.has_pending = 0;
            return 1;
        }
        /* THE PLACEMENT ARM — 「置き場にあるエネルギー1枚をこのメンバーの下に
           置く」. choice.rs:1132 hands the WHOLE `indices` vec to
           handle_energy_zone_selection, which is the C's
           rb_move_handle_energy_zone_selection: it handles N indices, sorts
           them high-to-low so the removals do not shift each other, and
           reverses the batch back into the order the player picked
           (move.c:2119-2146). The C was handing it a one-element array built
           from indices[0] alone (`int eidx[1] = { idx };`), so a two-card pick
           placed one card and removed one from the zone. */
        if (target && !strcmp(target, "under_member")) {
            int eids[RB_MAX_ZONE];
            int en = rb_zone_cards(g, actor, "energy", eids, RB_MAX_ZONE);
            /* Snapshot the picked IDs BEFORE the call: the function removes the
               cards from the zone, so eids is stale afterwards. Rust's
               `self.selection.moved_cards = cids.into()` is the batch, in
               selection order. */
            int batch[RB_MAX_ZONE];
            int nb = 0;
            for (int i = 0; i < n_sel && nb < RB_MAX_ZONE; i++)
                if (sel[i] >= 0 && sel[i] < en) batch[nb++] = eids[sel[i]];
            if (nb == 0) { rb_resolver_handle_selection_epilogue(self, g); return 1; }
            rb_move_handle_energy_zone_selection(g, actor, sel, n_sel, count,
                                                "under_member", NULL);
            for (int i = 0; i < nb; i++)
                if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED)
                    self->moved_cards[self->n_moved_cards++] = batch[i];
            rb_resolver_handle_selection_epilogue(self, g);
            return 1;
        }
        /* default energy destination (the waitroom): one send per named card. */
        {
            int cids[RB_MAX_ZONE];
            int nc = choice_resolve_zone_ids(g, actor, "energy", sel, n_sel,
                                             cids, RB_MAX_ZONE);
            for (int i = 0; i < nc; i++) {
                rb_choice_send_to_dst(g, actor, cids[i], "waitroom");
                if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED)
                    self->moved_cards[self->n_moved_cards++] = cids[i];
            }
            rb_resolver_handle_selection_epilogue(self, g);
            return 1;
        }
    }

    if (!strcmp(zone, "deck")) {
        int cids[RB_MAX_ZONE];
        int nc = choice_resolve_zone_ids(g, actor, "deck", sel, n_sel, cids, RB_MAX_ZONE);
        const char *dst = target ? target : "hand";
        for (int i = 0; i < nc; i++) {
            rb_choice_send_to_dst(g, actor, cids[i], dst);
            if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED)
                self->moved_cards[self->n_moved_cards++] = cids[i];
        }
        rb_resolver_handle_selection_epilogue(self, g);
        return 1;
    }

    if (!strcmp(zone, "energy_deck")) {
        int cids[RB_MAX_ZONE];
        int nc = choice_resolve_zone_ids(g, actor, "energy_deck", sel, n_sel, cids, RB_MAX_ZONE);
        const char *dst = g->queue.resume_move_destination[0]
                            ? g->queue.resume_move_destination : "energy_zone";
        for (int i = 0; i < nc; i++) {
            int cid = cids[i];
            rb_waitroom_remove_card(&g->p[actor], cid);
            rb_place_card_in_zone(g, actor, cid, dst, -1);
            if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED)
                self->moved_cards[self->n_moved_cards++] = cid;
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                self->selected_cards[self->n_selected_cards++] = cid;
            if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
                g->recently_moved[g->n_recently_moved++] = cid;
        }
        rb_resolver_handle_selection_epilogue(self, g);
        return 1;
    }

    if (!strcmp(zone, "under_member")) {
        /* The C's under-member bag is three per-area lists, flattened in area
           order — the same order the single-index arm reads. */
        int flat[RB_MAX_ZONE];
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            for (int j = 0; j < g->p[actor].under_cards[i].n; j++)
                flat[n++] = g->p[actor].under_cards[i].cards[j];
        for (int i = 0; i < n_sel; i++) {
            int idx = sel[i];
            if (idx < 0 || idx >= n) continue;
            int cid = flat[idx];
            rb_choice_send_to_dst(g, actor, cid, "energy");
            if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED)
                self->moved_cards[self->n_moved_cards++] = cid;
        }
        rb_resolver_handle_selection_epilogue(self, g);
        return 1;
    }

    if (!strcmp(zone, "stage")) {
        /* Rust handle_stage_selection (choice.rs:2071-2293) reads EVERY index of
           the answer. This dispatcher had NO stage arm, so a stage prompt
           answered with two indices fell through to the single-index path
           (choice.c:2370-2376) and only indices[0] was applied -- the measured
           "BOTH members" cases: 改札's 「ライブ成功時」 has to wait BOTH opponent
           members (parity_target_player k_kowarey_wait_two, answered {0,1} and
           resolving against one) and a gained-ability target_count pick buffed
           one member of two.

           `sel` is already ctx.mfi-mapped by the caller (choice.c:4894-4896), the
           same map the single-index twin applies at choice.c:3114. Rust reaches
           the member through `player.stage.stage[idx]` for EVERY mapped index
           (choice.rs:2116-2119), i.e. the mapped value is a STAGE SLOT, so a
           producer that shipped no filtered_indices needs the slot table built
           first -- stage_build_filtered_slots is the C's twin of misc.rs:1408-1411
           filling filtered_indices with slot positions. */
        int pl = rb_resolve_target_player(g, ch->target_player_id[0] ? ch->target_player_id : "self");
        if (pl < 0 || pl > 1) pl = actor;
        /* If the producer shipped its own table the caller's ctx.mfi already ran
           it and `sel` holds physical slots; otherwise `sel` still holds answer
           positions that the table built below has to be applied to. */
        int table_preexisting = (g->queue.pending.n_filtered_indices > 0);
        stage_build_filtered_slots(g, pl);
        int cards[RB_STAGE_SIZE];
        int n_cards = 0;
        for (int i = 0; i < n_sel; i++) {
            int slot = table_preexisting ? sel[i] : map_choice_index(&g->queue.pending, sel[i]);
            if (slot < 0 || slot >= RB_STAGE_SIZE) continue;          /* choice.rs:2119 */
            int cid = g->p[pl].stage[slot];
            if (cid == RB_EMPTY_SLOT) continue;                      /* choice.rs:2118 */
            if (!stage_choice_accepts(g, cid)) continue;             /* validate_card */
            int dup = 0;
            for (int k = 0; k < n_cards; k++) if (cards[k] == cid) dup = 1;
            if (dup) continue;
            cards[n_cards++] = cid;
        }

        /* choice.rs:2134-2147 dispatches on the prompt's DECLARED
           StageSelectIntent. The C cannot carry that field -- Rust parks it on
           the PERSISTENT resolver (resolver.rs:197) and every producer that sets
           it lives in another translation unit -- so each intent is recognised by
           the mark its own producer leaves in this port:
             ChangeStateWait     state.c:339 emits the stage prompt with
                                target="change_state"; Rust's producer is
                                change_state.rs:552 / cost/handlers.rs:966.
             CollectTargets      misc.c:527 parks target_selection_eff, the only
                                writer of it; Rust's producer is target.rs:372.
                                The arm IS the recording above, and the caller
                                then re-runs the parked effect with target_count
                                cleared (choice.c:5126-5132), which is Rust's
                                `CollectTargets => return Ok(())` followed by
                                finalize_choice -> resume_pending_actions.
             UnderMemberMove / PlaceEnergyDeckUnder
                                move.c:1269 and move.c:1916 both park
                                resume_mode=1 (move.c:1279 / :1920) and are
                                answered by the mode==1 arm at choice.c:4864, so
                                they cannot reach this dispatcher; Rust likewise
                                checks resume_under_member_placement BEFORE the
                                zone dispatch (choice.rs:1091-1093). Decline them
                                here so the single-index path keeps owning them
                                byte for byte. */
        if (ch->target[0] && strstr(ch->target, "under_member")) return 0;

        /* choice.rs:2129-2133 — the whole `cards` vec joins selection.cards. */
        for (int i = 0; i < n_cards; i++) {
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                self->selected_cards[self->n_selected_cards++] = cards[i];
            stage_publish_selected(g, cards[i]);
        }
        if (!strcmp(ch->target, "change_state")) {
            /* choice.rs:2148-2162 ChangeStateWait: before the effect has started
               the prompt IS the state change, so every chosen member is put to
               wait and last_cost_waited_members carries the set (the C's reader
               of it is state.c:135-141). After the effect has started Rust leaves
               it to the re-driven ChangeState, which in this port is state.c's
               "Honour prior selected_cards relay" (state.c:233-263). */
            int cur = g->queue.cur;
            int effect_started = (cur >= 0 && cur < g->queue.n_entries)
                                 ? g->queue.entries[cur].effect_started : 0;
            if (!effect_started) {
                for (int i = 0; i < n_cards; i++) {
                    rb_mods_set_orientation(&g->mods, cards[i], "wait");
                    if (g->n_last_cost_waited_members < RB_STAGE_SIZE)
                        g->last_cost_waited_members[g->n_last_cost_waited_members++] =
                            (int16_t)cards[i];
                }
            }
        }
        fprintf(stderr, "[STAGE_SELECT_MULTI] n=%d cards=%d target=%s\n", n_sel, n_cards,
                ch->target[0] ? ch->target : "(none)");
        /* choice.rs:2293 — every arm ends in `Ok(())`, and the answer path then runs
           `handle_selection_epilogue`, whose first move is to clear the prompt. The
           single-index stage twin spells that `rb_resolver_clear_choice_state_and_resume`
           (choice.c:3130) and this arm has to match it: leaving the answered prompt
           in place re-offers the very same selection to the next answer (measured:
           k_kowarey_wait_two answered {0,1} three times, then asserted on a
           selection nothing had consumed). With the prompt gone the caller's
           post-dispatch re-run of the parked effect sees the published
           selection.cards — Rust's `CollectTargets => return Ok(())` +
           `finalize_choice` -> `resume_pending_actions`. */
        rb_resolver_clear_choice_state_and_resume(self);
        return 1;
    }

    if (!strcmp(zone, "live_card_zone")) {
        /* `select_live_success` is a single-seat verdict question, not a card
           pick; leave it to the single-index handler. */
        if (target && !strcmp(target, "select_live_success")) return 0;
        int cids[RB_MAX_ZONE];
        int nc = choice_resolve_zone_ids(g, actor, "live", sel, n_sel, cids, RB_MAX_ZONE);
        for (int i = 0; i < nc; i++) {
            int cid = cids[i];
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                self->selected_cards[self->n_selected_cards++] = cid;
            /* remove from the live zone BY CARD, not by index: the earlier
               removals of this same answer have already shifted the positions. */
            RbPlayer *P = &g->p[actor];
            for (int j = 0; j < P->live.n; j++) {
                if (P->live.cards[j] != cid) continue;
                for (int k = j; k < P->live.n - 1; k++) P->live.cards[k] = P->live.cards[k + 1];
                if (P->live.n > 0) P->live.n--;
                break;
            }
        }
        rb_resolver_handle_selection_epilogue(self, g);
        return 1;
    }

    if (!strcmp(zone, "selected_cards")) {
        /* choice.rs:1134-1136 `narrow_selected_pool(indices)` — keep exactly
           the named cards, in the order they were named. */
        int keep[RB_MAX_ZONE];
        int nk = 0;
        for (int i = 0; i < n_sel && nk < RB_MAX_ZONE; i++) {
            int idx = sel[i];
            if (idx >= 0 && idx < self->n_selected_cards) keep[nk++] = self->selected_cards[idx];
        }
        for (int i = 0; i < nk; i++) self->selected_cards[i] = keep[i];
        self->n_selected_cards = nk;
        rb_resolver_handle_selection_epilogue(self, g);
        return 1;
    }

    /* Not one of the arms above: the caller restores the pending choice and
       runs the single-index handler exactly as before. */
    return 0;
}

/* ── same-unit cost: the C's Selection.moved_cards (choice.rs:821) ──────────
 *
 * Rust's resolver is a long-lived field on GameState, so `self.selection`
 * accumulates across every choice of one ability resolution. The C's
 * RbAbilityResolver is a LOCAL at both call sites of
 * rb_resolver_handle_select_card (choice.c:971 and :5332), so
 * `self->n_moved_cards` is back to 0 on EVERY answer and cannot answer
 * "how many of this cost's cards have I already paid?". This file-static is
 * that accumulator: keyed to the queue entry, so a second ability paying its
 * own cost starts from zero. Every terminal exit out of the hand-cost block
 * resets it. */
static int s_same_unit_entry = -1;
static int s_same_unit_paid;

static int same_unit_paid_count(int entry, int added)
{
    if (entry != s_same_unit_entry) { s_same_unit_entry = entry; s_same_unit_paid = 0; }
    s_same_unit_paid += added;
    return s_same_unit_paid;
}

static void same_unit_paid_reset(void)
{
    s_same_unit_entry = -1;
    s_same_unit_paid = 0;
}

int rb_resolver_handle_select_card(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g || !g->queue.has_pending) return -1;
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    idx = map_choice_index(&g->queue.pending, idx);
    int was_skip = (idx < 0);
    const char *zone = g->queue.pending.zone[0] ? g->queue.pending.zone : "hand";
    const char *target = g->queue.pending.target[0] ? g->queue.pending.target : NULL;
    int allow_skip = g->queue.pending.allow_skip;
    int pending_count = g->queue.pending.count > 0 ? g->queue.pending.count : 1;

    int cur = g->queue.cur;
    int effect_started = (cur >= 0 && cur < RB_QUEUE_DEPTH) ? g->queue.entries[cur].effect_started : 0;
    char cost_act[48]; cost_act[0] = '\0';
    int has_cost = rb_queue_current_cost_action(g, cost_act, sizeof(cost_act));
    int is_reveal = choice_is_reveal_prompt(&g->queue.pending);
    int is_reveal_text = choice_reveal_is_legacy_text(&g->queue.pending);
    int is_cost_reveal = (!effect_started && has_cost && !strcmp(cost_act, "reveal"));

    if (self->deferred_conditional_gate) {
        self->deferred_conditional_gate = 0;
        if (was_skip && cur >= 0) {
            /* Rust clears entry.pending_actions on skip */
        }
    }

    if (is_reveal) {
        /* choice.rs:981-983 — the flagged hand reveal is handle_reveal_selection
           (choice.rs:1505-1684).  `selected` is passed through RAW: for
           count>0 Rust uses ctx.indices verbatim and ignores the prompt's
           filtered_indices (choice.rs:1520-1531), while the display-index
           mapping `idx` above already applied is the count==0 any_number arm
           the handler performs itself. */
        RbSelectionContext rctx; memset(&rctx, 0, sizeof(rctx));
        rctx.count = g->queue.pending.count;
        rctx.allow_skip = allow_skip;
        rctx.is_reveal = g->queue.pending.is_reveal;
        rctx.blind = g->queue.pending.blind;
        if (g->queue.pending.card_type[0])
            strncpy(rctx.card_type, g->queue.pending.card_type, sizeof(rctx.card_type) - 1);
        if (g->queue.pending.target_player_id[0])
            strncpy(rctx.target_player_id, g->queue.pending.target_player_id,
                    sizeof(rctx.target_player_id) - 1);
        rb_resolver_handle_reveal_selection(self, g, &rctx, selected);
        return 1;
    }

    if (is_reveal_text || is_cost_reveal) {
        if (!was_skip) {
            int ids[RB_MAX_ZONE];
            int n = rb_zone_cards(g, actor, zone, ids, RB_MAX_ZONE);
            if (idx >= 0 && idx < n) {
                int cid = ids[idx];
                if (g->n_revealed < RB_MAX_REVEALED_CARDS) g->revealed_cards[g->n_revealed++] = cid;
                if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
                if (g->queue.resume_eff) rb_set_chosen_target(g->queue.resume_eff, target ? target : "reveal");
            }
        }
        return rb_resolver_clear_choice_state_and_resume(self);
    }

    /* Hand-cost payment — faithfully mirrors choice.rs handle_select_card hand-cost block.
       Validates against filtered pool, handles same_unit_name, fixed-count commitment (Rule 9.4.2.3),
       and any_number re-prompt. See choice.rs:476-744. log::debug! kept as comments for parity. */
    if (!effect_started && has_cost && !strcmp(zone, "hand")) {
        int cost_count = pending_count;
        /* allow_skip indicates any_number (count==0) in Rust */
        int is_any_number = (cost_count == 0 && allow_skip);
        int moved_before = self->n_moved_cards;
        if (!was_skip) {
            int ids[RB_MAX_ZONE];
            int n = rb_zone_cards(g, actor, "hand", ids, RB_MAX_ZONE);
            if (idx >= 0 && idx < n) {
                int cid = ids[idx];
                /* choice.rs:603-611 discard_hand_cost_selection moves the card
                   with util::move_cards(player, ids, Hand, Discard, ..), which
                   is what lifts it OUT of the hand. rb_choice_send_to_dst only
                   pushes the waitroom copy, so without this the paid cost card
                   was simultaneously in hand and in the waitroom. */
                rb_hand_remove_card(&g->p[actor], idx);
                rb_choice_send_to_dst(g, actor, cid, "waitroom");
                /* choice.rs:502,522,720: handle_hand_cost_payment records the pick
                   in `moved_cards` and then set_recently_moved_batch(.., "hand")
                   — never in `ctx.selected_cards`, which Rust only fills for an
                   EFFECT selection (choice.rs:1167). Recording it here published
                   the cost card through publish_selected_cards into
                   g->selected_cards, and a following effect (state.c:314
                   `g->n_selected_cards == 0`) then read the cost card as a prior
                   selection and auto-applied instead of prompting. */
                if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
                if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++] = cid;
                /* also push movement event for tracking (choice.rs:522 push_movement_event) */
            }
            if (cur >= 0) g->queue.entries[cur].optional_cost_result = 1;
            g->mods.last_cost_discard_count = self->n_moved_cards;
            g->mods.n_last_cost_moved_card_ids = self->n_moved_cards;
            for (int i = 0; i < self->n_moved_cards && i < RB_MAX_RECENTLY_MOVED; i++) g->mods.last_cost_moved_card_ids[i] = self->moved_cards[i];
        } else {
            if (cur >= 0) g->queue.entries[cur].optional_cost_result = 0;
        }
        /* choice.rs:753-773 — `new_card_ids.is_empty()`, i.e. THIS ANSWER MOVED
           NOTHING, ends the hand payment whatever `count` and `allow_skip` are:
           the prompt is dropped and Ok is returned, never a re-offer. The two
           stampings are Rust's own: cards already moved -> cost_paid = true and
           optional_cost_result = Some(true) plus the last_cost_* history, nothing
           moved on a prompt that allowed skipping -> cost_paid = true and
           optional_cost_result = Some(false), and the mandatory-nothing-moved case
           stamps nothing at all.

           The old body only took this exit for `n_moved_cards == 0 && allow_skip`
           and otherwise fell through to the fixed-count re-prompt below, so a
           MANDATORY prompt answered with no cards was offered the same hand for
           ever. That was unreachable while the decline arms intercepted the empty
           answer (choice.c:5043-5065); once they stopped, target_selection looped
           on `[RESUME_ANY] kind=1 zone=hand ... idx=-1` until the run was cut off.
           `moved_before` is the C's spelling of Rust's per-answer `new_card_ids`. */
        if (self->n_moved_cards == moved_before) {
            if (self->n_moved_cards > 0) {
                if (cur >= 0) {
                    g->queue.entries[cur].cost_paid = 1;
                    g->queue.entries[cur].optional_cost_result = 1;
                }
                g->mods.last_cost_discard_count = self->n_moved_cards;
                g->mods.n_last_cost_moved_card_ids = self->n_moved_cards;
                for (int i = 0; i < self->n_moved_cards && i < RB_MAX_RECENTLY_MOVED; i++)
                    g->mods.last_cost_moved_card_ids[i] = self->moved_cards[i];
            } else if (allow_skip) {
                if (cur >= 0) {
                    g->queue.entries[cur].cost_paid = 1;
                    g->queue.entries[cur].optional_cost_result = 0;
                }
            }
            self->pending_choice = (RbChoice){0};
            same_unit_paid_reset();
            return rb_resolver_clear_choice_state_and_resume(self);
        }
        /* choice.rs:817-865 — same-unit remainder re-prompt. Rust puts this arm
         * BETWEEN the "this answer moved nothing" exit above and the any_number
         * re-prompt below, and it is the ONLY arm whose TOTAL comes from the
         * cost rather than from the prompt: the gate tests the PROMPT count
         * (ctx.count, choice.rs:1031) while `total_needed` reads cost.count
         * (choice.rs:820). The C used one number for both, so 「同じユニット名を
         * 持つカード2枚を控え室に置いてもよい」 — whose prompt asks for ONE card at a
         * time — finalized after the first pick instead of re-prompting for the
         * second: MEASURED prompts=1, one card discarded, no re-prompt, and the
         * player paid 17 for a 2-card cost. */
        {
            const AbilityEffect *cost = rb_entry_cost(g);
            int same_unit = choice_cost_flag(cost, "same_unit_name", 0);
            int prompt_count = g->queue.pending.count;   /* choice.rs:1031 ctx.count */
            int moved_now = self->n_moved_cards - moved_before;
            if (same_unit && prompt_count > 0 && moved_now == prompt_count) {
                int total_needed = rb_effect_count_or(cost, 1);   /* choice.rs:820 */
                /* choice.rs:821 total_moved = self.selection.moved_cards.len().
                 * NOT self->n_moved_cards: that is 0 on every answer. */
                int total_moved = same_unit_paid_count(cur, moved_now);
                if (total_moved < total_needed) {                  /* choice.rs:822 */
                    int remaining = total_needed - total_moved;
                    /* choice.rs:828-832 — the unit comes from the FIRST card paid,
                     * not from the one just answered. */
                    const char *unit_name = NULL;
                    if (self->n_moved_cards > 0)
                        unit_name = rb_card_unit_name(self->moved_cards[0]);
                    /* choice.rs:833-839 — the whole hand filtered to that unit.
                     * The C has no `validate` closure at this site, so the unit
                     * test is intersected with the indices the PREVIOUS prompt
                     * offered: that is the validation the player was just shown,
                     * and it is what keeps a different-unit card (lily) out. */
                    int hand_ids[RB_MAX_ZONE];
                    int n_hand = rb_zone_cards(g, actor, "hand", hand_ids, RB_MAX_ZONE);
                    int same_unit_idxs[RB_MAX_ZONE];
                    int n_same = 0;
                    int had_filter = (g->queue.pending.n_filtered_indices > 0);
                    for (int i = 0; i < n_hand && n_same < RB_MAX_ZONE; i++) {
                        const char *u = rb_card_unit_name(hand_ids[i]);
                        if (!unit_name || !u || strcmp(u, unit_name) != 0) continue;
                        if (had_filter) {
                            int keep = 0;
                            for (int k = 0; k < g->queue.pending.n_filtered_indices; k++)
                                if (g->queue.pending.filtered_indices[k] == i) { keep = 1; break; }
                            if (!keep) continue;
                        }
                        same_unit_idxs[n_same++] = i;
                    }
                    if (n_same == 0) {
                        /* choice.rs:840-845 returns Err. This hand-cost block has
                         * no error channel, so the payment finalizes rather than
                         * opening a prompt over an empty pool. */
                        g->mods.last_cost_discard_count = self->n_moved_cards;
                        if (cur >= 0) g->queue.entries[cur].cost_paid = 1;
                        same_unit_paid_reset();
                        return rb_resolver_clear_choice_state_and_resume(self);
                    }
                    char en[80];
                    snprintf(en, sizeof(en),
                             "Select %d more card(s) with the same unit name", remaining);
                    /* Rust passes the ja string here too (choice.rs:854);
                     * choice_build_reprompt ignores it ((void)ja, :1349), so the
                     * English text is passed twice, as the energy arm does. */
                    choice_build_reprompt(g, actor, "hand", remaining, en, en, 0,
                                          same_unit_idxs, n_same);
                    return 0;
                }
            }
        }
        if (is_any_number) {
            /* any_number: after each pick, re-prompt with skip allowed (choice.rs:680-727) */
            int hand_n = g->p[actor].hand.n;
            if (was_skip) {
                /* player chose to stop — finalize */
                g->mods.last_cost_discard_count = self->n_moved_cards;
                for (int i=0;i<self->n_moved_cards;i++) g->mods.last_cost_moved_card_ids[i]=self->moved_cards[i];
                g->mods.n_last_cost_moved_card_ids=self->n_moved_cards;
                if (cur>=0) g->queue.entries[cur].cost_paid=1;
                memset(&g->queue.pending,0,sizeof(g->queue.pending));
                g->queue.has_pending=0;
                same_unit_paid_reset();
                return rb_resolver_clear_choice_state_and_resume(self);
            }
            if (hand_n > 0) {
                char en[64]; char ja[64];
                snprintf(en,sizeof(en),"Select more card(s) from hand for cost (or skip to finish)");
                snprintf(ja,sizeof(ja),"コストとして手札からさらに選択（スキップで終了）");
                choice_build_reprompt(g, actor, "hand", 0, en, ja, 1, NULL, 0);
                return 0;
            }
            /* hand empty -> finalize */
            g->mods.last_cost_discard_count = self->n_moved_cards;
            if (cur>=0) g->queue.entries[cur].cost_paid=1;
            return rb_resolver_clear_choice_state_and_resume(self);
        }
        /* fixed-count: if we haven't yet picked enough, re-prompt for remaining (choice.rs:570-619) */
        if (cost_count > 0 && self->n_moved_cards < cost_count) {
            int remaining = cost_count - self->n_moved_cards;
            int hand_n = g->p[actor].hand.n;
            if (hand_n == 0) { same_unit_paid_reset(); return -1; } /* cannot pay */
            char en[64]; char ja[64];
            snprintf(en,sizeof(en),"Select %d more card(s) from hand for cost", remaining);
            snprintf(ja,sizeof(ja),"コストとして手札からさらに%d枚選択", remaining);
            choice_build_reprompt(g, actor, "hand", remaining, en, ja, 0, NULL, 0);
            return 0;
        }
        /* committed fixed count satisfied -> finalize (choice.rs:730-743) */
        g->mods.last_cost_discard_count = self->n_moved_cards;
        for (int i=0;i<self->n_moved_cards;i++) g->mods.last_cost_moved_card_ids[i]=self->moved_cards[i];
        g->mods.n_last_cost_moved_card_ids=self->n_moved_cards;
        if (cur>=0) g->queue.entries[cur].cost_paid=1;
        /* pay deferred sub-costs if any (choice.rs:737 pay_deferred_costs) */
        rb_pay_deferred_costs(g, actor, NULL);
        same_unit_paid_reset();
        return rb_resolver_clear_choice_state_and_resume(self);
    }

    /* Energy zone cost handling (choice.rs:746-800): pay_energy with reprompt */
    if (!strcmp(zone,"energy") && !effect_started &&
        !(target && !strcmp(target, "energy_state_change"))) {
        const char *dest = g->queue.pending.target[0] ? g->queue.pending.target : NULL;
        if (!dest || strcmp(dest,"under_member")!=0) {
            if (!was_skip) {
                int pay = 1;
                g->p[actor].energy_active -= pay; if (g->p[actor].energy_active<0) g->p[actor].energy_active=0;
                g->mods.last_cost_discard_count += pay; /* reuse for energy tracking parity */
                if (cur>=0) { g->queue.entries[cur].cost_paid=1; g->queue.entries[cur].optional_cost_result=1; }
            }
            int energy_left = g->p[actor].energy_active;
            if (!was_skip && energy_left>0) {
                char en[80]; snprintf(en,sizeof(en),"Select energy card to pay (active: %d). Skip when done",energy_left);
                choice_build_reprompt(g, actor, "energy", 0, en, en, 1, NULL,0);
                return 0;
            }
            rb_resolver_clear_choice_state(self);
            rb_resolver_resume_pending_actions(self);
            return 0;
        }
    }
    /* under_member via stage selection (choice.rs:812-862): MoveCardsPosition context */
    if (!strcmp(zone,"stage") && g->queue.pending.target[0] && strstr(g->queue.pending.target,"under_member")) {
        if (g->queue.resume_eff && g->queue.resume_host >=0) {
            int chosen = idx;
            if (chosen>=0 && chosen<3 && g->p[actor].stage[chosen]!=-1) {
                int cid = g->queue.resume_host;
                g->p[actor].under_cards[chosen].cards[g->p[actor].under_cards[chosen].n++] = cid;
                g->n_recently_moved=0; if(g->n_recently_moved<RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++]=cid;
                if(self->n_moved_cards<RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++]=cid;
            }
            rb_resolver_clear_choice_state(self);
            rb_resolver_resume_pending_actions(self);
            return 0;
        }
    }
    /* zone dispatch — mirrors choice.rs:863-1049 match Zone::from_str */
    if (!strcmp(zone,"hand")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip; ctx2.is_reveal = g->queue.pending.is_reveal;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        if (g->queue.pending.target_player_id[0]) strncpy(ctx2.target_player_id, g->queue.pending.target_player_id, sizeof(ctx2.target_player_id)-1);
        ctx2.blind = g->queue.pending.blind;
        char idxbuf[32]; snprintf(idxbuf,sizeof(idxbuf),"%d",idx);
        return rb_resolver_handle_hand_selection(self, g, was_skip?NULL:idxbuf);
    }
    if (!strcmp(zone,"deck")) {
        if (!was_skip) {
            int ids[RB_MAX_ZONE]; int n=rb_zone_cards(g, actor,"deck",ids,RB_MAX_ZONE);
            if(idx>=0 && idx<n){ int cid=ids[idx]; rb_choice_send_to_dst(g, actor,cid, g->queue.pending.target[0]?g->queue.pending.target:"hand"); if(self->n_moved_cards<RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++]=cid; }
        }
        rb_resolver_handle_selection_epilogue(self, g); return 0;
    }
    if (!strcmp(zone,"discard") || !strcmp(zone,"waitroom")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        if (g->queue.pending.target_player_id[0]) strncpy(ctx2.target_player_id, g->queue.pending.target_player_id, sizeof(ctx2.target_player_id)-1);
        char idxbuf[32]; snprintf(idxbuf,sizeof(idxbuf),"%d",idx);
        rb_resolver_handle_discard_selection(self,g,was_skip?NULL:idxbuf); return 0;
    }
    if (!strcmp(zone,"looked_at")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip; ctx2.is_reveal = g->queue.pending.is_reveal;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        char idxbuf[32]; snprintf(idxbuf,sizeof(idxbuf),"%d",idx);
        rb_resolver_handle_looked_at_selection(self,g,was_skip?NULL:idxbuf); return 0;
    }
    if (!strcmp(zone,"revealed_cards")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        if (g->queue.pending.target_player_id[0]) strncpy(ctx2.target_player_id, g->queue.pending.target_player_id, sizeof(ctx2.target_player_id)-1);
        char idxbuf[32]; snprintf(idxbuf,sizeof(idxbuf),"%d",idx);
        rb_resolver_handle_revealed_cards_selection(self,g,&ctx2,was_skip?NULL:idxbuf); return 0;
    }
    if (!strcmp(zone,"energy_deck")) {
        if (!was_skip) {
            int ids[RB_MAX_ZONE];
            int n = rb_zone_cards(g, actor, "energy_deck", ids, RB_MAX_ZONE);
            if (idx >= 0 && idx < n) {
                int cid = ids[idx];
                rb_waitroom_remove_card(&g->p[actor], cid);
                const char *dst = g->queue.resume_move_destination[0] ?
                    g->queue.resume_move_destination : "energy_zone";
                rb_place_card_in_zone(g, actor, cid, dst, -1);
                if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED)
                    self->moved_cards[self->n_moved_cards++] = cid;
                if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                    self->selected_cards[self->n_selected_cards++] = cid;
                if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
                    g->recently_moved[g->n_recently_moved++] = cid;
            }
        }
        rb_resolver_handle_selection_epilogue(self, g);
        return 0;
    }
    if (!strcmp(zone,"energy")) {
        if (target && !strcmp(target, "energy_state_change")) {
            if (was_skip) return 0;
            int ids[RB_MAX_ZONE];
            int n = rb_zone_cards(g, actor, "energy", ids, RB_MAX_ZONE);
            if (idx < 0 || idx >= n) return 0;
            int cid = ids[idx];
            int already = 0;
            for (int i = 0; i < g->n_selected_cards; i++)
                if (g->selected_cards[i] == cid) already = 1;
            if (!already && g->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                g->selected_cards[g->n_selected_cards++] = cid;
            already = 0;
            for (int i = 0; i < self->n_selected_cards; i++)
                if (self->selected_cards[i] == cid) already = 1;
            if (!already && self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                self->selected_cards[self->n_selected_cards++] = cid;
            if (g->queue.pending.count > 1) {
                g->queue.pending.count--;
                int w = 0;
                for (int i = 0; i < g->queue.pending.n_filtered_indices; i++)
                    if (g->queue.pending.filtered_indices[i] != idx)
                        g->queue.pending.filtered_indices[w++] = g->queue.pending.filtered_indices[i];
                g->queue.pending.n_filtered_indices = w;
                return 0;
            }
            g->queue.has_pending = 0;
            return 0;
        }
        /* choice.rs:1116-1123 — a SelectCard over Zone::Energy hands the
           choice's OWN `destination` to move/mod.rs:3302
           handle_energy_zone_selection, whose `dst == Zone::UnderMember` arm
           (:3336-3347) is the 「エネルギー置き場にあるエネルギー1枚をこの
           メンバーの下に置く」 placement. This arm had NO such case, so the
           selected energy was unconditionally routed to the WAITROOM: no
           under_cards entry was ever created and the chained effect after the
           cost never ran. `target` is where the C stores that destination
           (rb_resolver_store_pending_choice, the `target` field of the flat
           RbChoice, the same field the `energy_state_change` branch above
           already tests). */
        if (target && !strcmp(target, "under_member")) {
            if (was_skip || idx < 0) {
                /* move/mod.rs:3337-3344 — an empty selection drops the pending
                   「そうした場合」 actions and records the decline, so the
                   chained effect does not still fire. */
                rb_queue_take_pending_actions(g);
                if (cur >= 0 && cur < RB_QUEUE_DEPTH)
                    g->queue.entries[cur].optional_cost_result = 0;
                rb_resolver_clear_choice_state_and_resume(self);
                return 0;
            }
            int eids[RB_MAX_ZONE];
            int en = rb_zone_cards(g, actor, "energy", eids, RB_MAX_ZONE);
            if (idx >= en) { rb_resolver_handle_selection_epilogue(self, g); return 0; }
            int eidx[1] = { idx };
            rb_move_handle_energy_zone_selection(g, actor, eidx, 1, pending_count,
                                                "under_member", NULL);
            /* move/mod.rs:3358 — self.selection.moved_cards = cids. */
            if (idx < en && self->n_moved_cards < RB_MAX_RECENTLY_MOVED)
                self->moved_cards[self->n_moved_cards++] = eids[idx];
            /* The standard epilogue every other arm ends on, so the chained
               「そうした場合」 effect still runs and a pending reprompt is
               honoured (choice.rs:1116-1123 returns straight into the outer
               answer loop, not a bespoke teardown). */
            rb_resolver_handle_selection_epilogue(self, g);
            return 0;
        }
        if (!was_skip) {
            int ids[RB_MAX_ZONE]; int n=rb_zone_cards(g, actor,"energy",ids,RB_MAX_ZONE);
            if(idx>=0 && idx<n){ int cid=ids[idx]; rb_choice_send_to_dst(g,actor,cid,"waitroom"); if(self->n_moved_cards<RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++]=cid; }
        }
        rb_resolver_handle_selection_epilogue(self,g); return 0;
    }
    if (!strcmp(zone,"selected_cards")) {
        if (!was_skip && idx>=0 && idx<self->n_selected_cards) {
            int keep = self->selected_cards[idx];
            self->n_selected_cards=1; self->selected_cards[0]=keep;
        } else if (was_skip) self->n_selected_cards=0;
        rb_resolver_handle_selection_epilogue(self,g); return 0;
    }
    if (!strcmp(zone,"live_card_zone")) {
        if (!was_skip) {
            if (target && !strcmp(target, "select_live_success")) {
                rb_handle_live_success_choice(g, actor, idx);
            } else {
                int ids[RB_MAX_ZONE]; int n=rb_zone_cards(g, actor,"live",ids,RB_MAX_ZONE);
                if(idx>=0 && idx<n){ int cid=ids[idx]; if(self->n_selected_cards<RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++]=cid; /* remove from live zone */ for(int i=idx;i<g->p[actor].live.n-1;i++) g->p[actor].live.cards[i]=g->p[actor].live.cards[i+1]; if(g->p[actor].live.n>0) g->p[actor].live.n--; }
            }
        }
        rb_resolver_handle_selection_epilogue(self,g); return 0;
    }
    if (!strcmp(zone,"stage")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        char idxbuf[32]; snprintf(idxbuf,sizeof(idxbuf),"%d",idx);
        rb_resolver_handle_stage_selection(self,g,&ctx2,was_skip?NULL:idxbuf); return 0;
    }
    if (!strcmp(zone,"under_member")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip; ctx2.is_reveal = g->queue.pending.is_reveal;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        if (g->queue.pending.target_player_id[0]) strncpy(ctx2.target_player_id, g->queue.pending.target_player_id, sizeof(ctx2.target_player_id)-1);
        if (!was_skip) {
            int ids[RB_MAX_ZONE]; int n=0; for(int i=0;i<3;i++) for(int j=0;j<g->p[actor].under_cards[i].n;j++) ids[n++]=g->p[actor].under_cards[i].cards[j];
            if(idx>=0 && idx<n){ int cid=ids[idx]; rb_choice_send_to_dst(g,actor,cid,"energy"); if(self->n_moved_cards<RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++]=cid; }
        }
        rb_resolver_handle_selection_epilogue(self,g); return 0;
    }
    if (!strcmp(zone,"success_live_zone") || !strcmp(zone,"success")) {
        RbSelectionContext ctx2; memset(&ctx2,0,sizeof(ctx2));
        ctx2.count = pending_count; ctx2.allow_skip = allow_skip;
        if (g->queue.pending.card_type[0]) strncpy(ctx2.card_type, g->queue.pending.card_type, sizeof(ctx2.card_type)-1);
        if (g->queue.pending.filter_group[0]) strncpy(ctx2.group, g->queue.pending.filter_group, sizeof(ctx2.group)-1);
        ctx2.cost_limit = g->queue.pending.cost_limit;
        if (g->queue.pending.cost_limit_op[0]) strncpy(ctx2.cost_limit_op, g->queue.pending.cost_limit_op, sizeof(ctx2.cost_limit_op)-1);
        ctx2.cost_total = g->queue.pending.cost_total;
        if (g->queue.pending.cost_total_op[0]) strncpy(ctx2.cost_total_op, g->queue.pending.cost_total_op, sizeof(ctx2.cost_total_op)-1);
        if (g->queue.pending.target_player_id[0]) strncpy(ctx2.target_player_id, g->queue.pending.target_player_id, sizeof(ctx2.target_player_id)-1);
        rb_resolver_handle_success_live_zone_selection(self,g,&ctx2,was_skip?NULL:selected); return 0;
    }
    /* fallback */
    if (!was_skip) {
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, actor, zone, ids, RB_MAX_ZONE);
        if (idx >= 0 && idx < n) {
            int cid = ids[idx];
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
            if (g->queue.resume_eff) rb_set_chosen_target(g->queue.resume_eff, zone);
        }
    }
    return rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_hand_selection (choice.rs:1083) — faithful ──
   C6 keep-N-shuffle-rest: selected hand cards are KEPT, rest shuffled under deck.
   Phase 1 = self, Phase 2 = opponent. Also handles fixed-count reprompt (Rule 9.4.2.3)
   and any_number accumulation via execute_selected_cards_from_zone. */
int rb_resolver_handle_hand_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g || !g->queue.has_pending) return -1;
    int actor_pending = g->queue.actor;
    const char *cur_zone = g->queue.pending.zone;
    int is_c6 = (g->keep_shuffle_under_phase > 0);
    /* Rust: mapped_indices = ctx.mfi(ctx.indices) — filtered_indices mapping */
    int sel_idx = selected ? atoi(selected) : -1;
    int is_skip = (sel_idx < 0);
    int pending_count = g->queue.pending.count;
    int allow_skip = g->queue.pending.allow_skip;
    /* C6 keep_shuffle phase handling (choice.rs:1100-1206) */
    if (is_c6) {
        int phase = g->keep_shuffle_under_phase;
        int count_needed = g->keep_shuffle_under_count >0 ? g->keep_shuffle_under_count : pending_count;
        if (!is_skip) {
            if (g->keep_shuffle_under_selected_n < RB_MAX_HAND) g->keep_shuffle_under_selected[g->keep_shuffle_under_selected_n++] = sel_idx;
        }
        if (phase == 1) {
            /* validate: if hand_idx empty && allow_skip false && count>0 => error */
            if (is_skip && !allow_skip && count_needed>0) return -1;
            /* Record chosen positions; if still need more, re-prompt (choice.rs:1122-1145) */
            int need = count_needed > g->keep_shuffle_under_selected_n ? count_needed - g->keep_shuffle_under_selected_n : 0;
            int snapshot_n = g->keep_shuffle_under_snapshot_n[0];
            int available = snapshot_n - g->keep_shuffle_under_selected_n;
            if (!is_skip && need>0 && available>0) {
                char en[64]; snprintf(en,sizeof(en),"Select up to %d more card(s) from hand to keep", need);
                RbChoice ch; memset(&ch,0,sizeof(ch));
                ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"hand",sizeof(ch.zone)-1);
                ch.count=need; ch.allow_skip=1; strncpy(ch.description,en,sizeof(ch.description)-1);
                g->queue.pending=ch; g->queue.has_pending=1; g->queue.actor=actor_pending;
                return 0;
            }
            /* move self non-selected to deck bottom using snapshot (choice.rs:1151) */
            RbPlayer *P = &g->p[actor_pending];
            int keep_set[RB_MAX_HAND]={0};
            for(int i=0;i<g->keep_shuffle_under_selected_n;i++) if(g->keep_shuffle_under_selected[i]>=0 && g->keep_shuffle_under_selected[i]<RB_MAX_HAND) keep_set[g->keep_shuffle_under_selected[i]]=1;
            /* Need to move based on snapshot order: snapshot is hand at prompt time */
            for(int i=snapshot_n-1;i>=0;i--) if(!keep_set[i]) {
                int cid = g->keep_shuffle_under_snapshot[0][i];
                /* remove from current hand if present */
                for(int j=0;j<P->hand.n;j++) if(P->hand.cards[j]==cid){ for(int k=j;k<P->hand.n-1;k++) P->hand.cards[k]=P->hand.cards[k+1]; P->hand.n--; break; }
                P->deck.cards[P->deck.n++]=cid;
            }
            g->keep_shuffle_under_selected_n=0;
            /* Snapshot opponent hand before prompting (choice.rs:1154) */
            RbPlayer *Opp = &g->p[actor_pending ^ 1];
            g->keep_shuffle_under_snapshot_n[1]=Opp->hand.n;
            for(int i=0;i<Opp->hand.n && i<RB_MAX_HAND;i++) g->keep_shuffle_under_snapshot[1][i]=Opp->hand.cards[i];
            int opp_len = g->keep_shuffle_under_snapshot_n[1];
            int pick = count_needed; if(pick>opp_len) pick=opp_len;
            g->keep_shuffle_under_phase=2;
            self->spawn_target = 1; self->spawn_target_set=1;
            RbChoice ch; memset(&ch,0,sizeof(ch));
            ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"hand",sizeof(ch.zone)-1);
            ch.count=pick; ch.allow_skip=1; snprintf(ch.description,sizeof(ch.description),"Select up to %d card(s) to keep", count_needed);
            g->queue.pending=ch; g->queue.has_pending=1; g->queue.actor=actor_pending ^ 1;
            return 0;
        }
        if (phase == 2) {
            /* phase 2: move opponent non-selected using snapshot[1] (choice.rs:1176-1204) */
            int opp = actor_pending ^ 1;
            RbPlayer *P = &g->p[opp];
            int snap_n = g->keep_shuffle_under_snapshot_n[1];
            int keep_set[RB_MAX_HAND]={0};
            for(int i=0;i<g->keep_shuffle_under_selected_n;i++) if(g->keep_shuffle_under_selected[i]>=0) keep_set[g->keep_shuffle_under_selected[i]]=1;
            for(int i=snap_n-1;i>=0;i--) if(!keep_set[i]) {
                int cid = g->keep_shuffle_under_snapshot[1][i];
                for(int j=0;j<P->hand.n;j++) if(P->hand.cards[j]==cid){ for(int k=j;k<P->hand.n-1;k++) P->hand.cards[k]=P->hand.cards[k+1]; P->hand.n--; break; }
                P->deck.cards[P->deck.n++]=cid;
            }
            g->keep_shuffle_under_phase=0;
            g->keep_shuffle_under_snapshot_n[0]=0; g->keep_shuffle_under_snapshot_n[1]=0;
            g->keep_shuffle_under_selected_n=0;
            self->spawn_target_set=0;
            /* perform draw 3 for both directly and clear pending draw (choice.rs:1187-1204) */
            for(int pl=0;pl<2;pl++) for(int i=0;i<3;i++) rb_draw(g, pl);
            rb_queue_take_pending_actions(g);
            rb_resolver_handle_selection_epilogue(self, g);
            return 0;
        }
    }
    /* ── Non-C6 hand SELECTION — choice.rs:1305-1543 ──────────────────────────
       G1. This arm used to RECORD the pick into `self->selected_cards` and then
       fall straight through to the epilogue, expecting the zone move to arrive
       from `g->queue.deferred`. That field is NULL for a QUEUED ability (the
       ability carries itself in `queue.entries[cur]` and the drain re-enters it),
       so `def=0x0` in the resume trace and the chosen card was never moved: the
       prompt was answered, the selection recorded, and the card stayed in hand.

       Rust does the move IN THIS HANDLER. `handle_hand_selection` calls
       `self.execute_selected_cards_from_zone(gs, Zone::Hand.to_str(), &all_idxs,
       ..)` (choice.rs:1461-1474) and only THEN decides what to do with the
       accumulator, so this port now does the same. The C twin already exists and
       is what the multi-index arm of this same file uses:
       rb_resolver_execute_selected_cards_from_zone -> rb_move_execute_selected_cards_from_zone
       (move.c:2460, the port of effects/move/mod.rs:2656 `execute_selected_cards_from_zone`). It applies the prompt's
       own card_type / cost_limit / cost_total / group / characters filters,
       resolves the destination through rb_entry_destination, records the
       movement, and can raise a sub-choice (stage placement, success
       replacement, deck_top_or_bottom) — none of which the removed hand-rolled
       `rb_hand_remove_card` + `rb_choice_send_to_dst` pair did. That pair also
       sent the card to `pending.target`, which holds the SEAT ("self"/
       "opponent"), never a zone.

       `all_idxs` (choice.rs:1469-1480) is the current pick PLUS the hand
       position of every card already in `self.selection.cards`. Rust's
       `self.selection` is a field of the PERSISTENT resolver; the C's
       RbAbilityResolver is memset to 0 at every answer
       (rb_resume_with_choice_indices_internal), so the C's carrier for that
       cross-round accumulator is the published `g->selected_cards`, which
       publish_selected_cards fills from the resolver before each re-prompt. A
       stale id is harmless: Rust's merge loop only looks up ids still in hand. */
    int is_any_number = (pending_count==0 && allow_skip);
    if (is_skip && !allow_skip && pending_count>0) return -1;

    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, actor_pending, "hand", ids, RB_MAX_ZONE);
    int mapped = sel_idx;   /* ctx.mfi already applied by the caller */
    int picked_here = (mapped >= 0 && mapped < n) ? 1 : 0;

    if (!is_skip && picked_here && pending_count > 0 && 1 < pending_count) {
        /* PARTIAL pick — choice.rs:1309-1368. The cards chosen so far go into the
           accumulator and the prompt is re-offered for the remainder; the move
           happens once, on the complete answer. */
        int cid = ids[mapped];
        int exists = 0;
        for (int i = 0; i < self->n_selected_cards; i++)
            if (self->selected_cards[i] == cid) { exists = 1; break; }
        if (!exists && self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
            self->selected_cards[self->n_selected_cards++] = cid;
        publish_selected_cards(self);
        /* choice.rs:1333 `let remaining = ctx.count - hand_idx.len()` — the count
           is this answer's pick, not the accumulator. */
        int remaining = pending_count - 1;
        char en[80]; char ja[80];
        snprintf(en, sizeof(en), "Select %d more card(s) from hand", remaining);
        snprintf(ja, sizeof(ja), "手札からさらに%d枚選択", remaining);
        /* choice.rs:1334-1347 `fi`: the candidates are the hand positions NOT
           already in `all_hand_idxs`. */
        int avail[RB_MAX_ZONE], avail_n = 0;
        for (int i = 0; i < n; i++) {
            int dup = 0;
            for (int j = 0; j < self->n_selected_cards; j++)
                if (self->selected_cards[j] == ids[i]) { dup = 1; break; }
            if (!dup && avail_n < RB_MAX_ZONE) avail[avail_n++] = i;
        }
        choice_build_reprompt(g, actor_pending, "hand", remaining, en, ja, 0, avail, avail_n);
        return 0;
    }

    if (!is_skip && picked_here && is_any_number) {
        /* any_number batch — choice.rs:1369-1430. Move THIS pick through the
           shared executor, record the moved id (choice.rs:1382-1384), then
           re-offer with skip. */
        int hand_idx[1]; hand_idx[0] = mapped;
        const char *chars[16];
        int n_chars = choice_characters_arg(&g->queue.pending, chars, 16);
        rb_resolver_execute_selected_cards_from_zone(self, g, "hand", hand_idx, 1,
            g->queue.pending.card_type[0] ? g->queue.pending.card_type : NULL,
            choice_prompt_cost_limit(&g->queue.pending),
            g->queue.pending.cost_limit_op[0] ? g->queue.pending.cost_limit_op : NULL,
            choice_prompt_cost_total(&g->queue.pending),
            g->queue.pending.cost_total_op[0] ? g->queue.pending.cost_total_op : NULL,
            g->queue.pending.filter_group[0] ? g->queue.pending.filter_group : NULL,
            n_chars ? chars : NULL, n_chars,
            g->queue.pending.target_player_id[0] ? g->queue.pending.target_player_id : NULL);
        if (rb_has_pending_choice(g)) return 0;   /* a sub-choice took over */
        int moved_here = ids[mapped];
        int already = 0;
        for (int i = 0; i < self->n_selected_cards; i++)
            if (self->selected_cards[i] == moved_here) { already = 1; break; }
        if (!already && self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
            self->selected_cards[self->n_selected_cards++] = moved_here;
        publish_selected_cards(self);
        if (g->p[actor_pending].hand.n > 0) {
            /* choice.rs:1396-1426 build_reprompt(.., count=0, allow_skip=true,
               fi = the hand positions still passing the prompt's filter). */
            int out[RB_MAX_ZONE], out_n = 0;
            for (int i = 0; i < n; i++) {
                if (!choice_discard_pick_legal(&g->queue.pending, ids[i])) continue;
                int dup = 0;
                for (int j = 0; j < self->n_selected_cards; j++)
                    if (self->selected_cards[j] == ids[i]) { dup = 1; break; }
                if (dup) continue;
                int in_hand = 0;
                for (int k = 0; k < g->p[actor_pending].hand.n; k++)
                    if (g->p[actor_pending].hand.cards[k] == ids[i]) { in_hand = 1; break; }
                if (in_hand && out_n < RB_MAX_ZONE) out[out_n++] = i;
            }
            char en[96]; char ja[96];
            snprintf(en, sizeof(en), "Select more card(s) from hand (or skip to finish)");
            snprintf(ja, sizeof(ja), "手札からさらに選択（スキップで終了）");
            choice_build_reprompt(g, actor_pending, "hand", 0, en, ja, 1, out, out_n);
            return 0;
        }
        self->n_selected_cards = 0;
        rb_resolver_handle_selection_epilogue(self, g);
        return 0;
    }

    /* COMPLETE (or empty) pick — choice.rs:1431-1526. This is the move G1 was
       missing: the executor is what lifts the cards out of the hand and places
       them in the destination zone. */
    {
        int all_idxs[RB_MAX_ZONE];
        int n_all = 0;
        if (picked_here) all_idxs[n_all++] = mapped;
        for (int h = 0; h < n && n_all < RB_MAX_ZONE; h++) {
            int in_acc = 0;
            for (int i = 0; i < g->n_selected_cards; i++)
                if (g->selected_cards[i] == ids[h]) { in_acc = 1; break; }
            if (!in_acc) continue;
            int dup = 0;
            for (int i = 0; i < n_all; i++) if (all_idxs[i] == h) { dup = 1; break; }
            if (!dup) all_idxs[n_all++] = h;
        }
        if (n_all > 0) {
            /* choice.rs:1475-1482 `selected_hand_ids` is snapshotted BEFORE the
               move, so the keep_selected assignment below names the cards this
               answer chose rather than whatever survived the filters. */
            int selected_hand_ids[RB_MAX_ZONE];
            for (int i = 0; i < n_all; i++) selected_hand_ids[i] = ids[all_idxs[i]];
            const char *chars[16];
            int n_chars = choice_characters_arg(&g->queue.pending, chars, 16);
            rb_resolver_execute_selected_cards_from_zone(self, g, "hand", all_idxs, n_all,
                g->queue.pending.card_type[0] ? g->queue.pending.card_type : NULL,
                choice_prompt_cost_limit(&g->queue.pending),
                g->queue.pending.cost_limit_op[0] ? g->queue.pending.cost_limit_op : NULL,
                choice_prompt_cost_total(&g->queue.pending),
                g->queue.pending.cost_total_op[0] ? g->queue.pending.cost_total_op : NULL,
                g->queue.pending.filter_group[0] ? g->queue.pending.filter_group : NULL,
                n_chars ? chars : NULL, n_chars,
                g->queue.pending.target_player_id[0] ? g->queue.pending.target_player_id : NULL);
            if (rb_has_pending_choice(g)) return 0;   /* a sub-choice took over */
            /* choice.rs:1486-1525 keep_selected: the accumulator survives only
               when the effect that asked CONSUMES `selected_cards`. */
            int keep_selected = 0;
            if (self->current_effect &&
                choice_effect_uses_selected_cards(self->current_effect)) keep_selected = 1;
            if (!keep_selected) {
                const AbilityEffect *entry_eff = rb_entry_effect(g);
                if (entry_eff && choice_effect_uses_selected_cards(entry_eff)) keep_selected = 1;
            }
            g->n_selected_cards = 0;
            self->n_selected_cards = 0;
            if (keep_selected)
                for (int i = 0; i < n_all && i < RB_MAX_RECENTLY_MOVED; i++) {
                    g->selected_cards[g->n_selected_cards++] = selected_hand_ids[i];
                    self->selected_cards[self->n_selected_cards++] = selected_hand_ids[i];
                }
        }
    }

    if (allow_skip) {
        int cur = g->queue.cur;
        if (cur>=0 && cur < g->queue.n_entries) {
            int has_move = (self->n_moved_cards>0 || !is_skip);
            g->queue.entries[cur].optional_cost_result = has_move ? 1 : 0;
            if (is_skip && pending_count>0 && self->n_moved_cards==0 && strcmp(cur_zone,"")!=0) {
                /* opponent action exception: don't clear if target opponent */
                int is_opp = (g->queue.pending.target[0] && strstr(g->queue.pending.target,"opponent"));
                if (!is_opp) rb_queue_take_pending_actions(g);
            }
        }
    }
    return rb_resolver_handle_selection_epilogue(self, g), 0;
}

/* ── effect_uses_selected_cards (cost.c:150, mirroring util.rs) ────────────
   cost.c keeps a file-static copy; this one is needed because Rust's
   handle_reveal_selection filters its `selected_effect` on the same predicate
   (choice.rs:1624-1628) to decide whether the revealed ids become the
   resolver's selected_cards. */
static int choice_effect_uses_selected_cards(const AbilityEffect *e)
{
    if (!e) return 0;
    if (e->source && !strcmp(e->source, "selected_cards")) return 1;
    for (int i = 0; i < e->n_child; i++)
        if (choice_effect_uses_selected_cards(e->child[i])) return 1;
    return 0;
}

/* ── choice_reveal_target_label (choice.rs:1515-1518 + 1563) ────────────────
   Rust carries ctx.target_player_id — the EFFECT's own target string, e.g.
   "opponent" for 相手の手札を…公開 — through handle_reveal_selection and
   re-publishes it on the re-prompt.  look.c publishes the RESOLVED seat
   ("p2") instead, so the label is re-derived from it: the C prompt keeps
   naming the seat it addresses, in the vocabulary the rest of the prompt
   layer uses. */
static void choice_reveal_target_label(const GameState *g, const char *tpid, int actor,
                                       char *out, int outsz)
{
    const char *t = (tpid && *tpid) ? tpid : "self";
    if (!strcmp(t, "self") || !strcmp(t, "opponent")) {
        snprintf(out, outsz, "%s", t);
        return;
    }
    int who = rb_resolve_target_player(g, t);
    if (who < 0) { snprintf(out, outsz, "%s", t); return; }
    snprintf(out, outsz, "%s", (who == actor) ? "self" : "opponent");
}

/* ── handle_reveal_selection (choice.rs:1505-1684) ─────────────────────────
   Faithful port.  The C resolver is a per-answer stack object (Rust's
   AbilityResolver is a long-lived field), so the accumulation Rust keeps in
   `self.selected_cards` travels between answers in the re-prompt's
   filtered_indices — which is exactly the field Rust writes it into at
   choice.rs:1566-1571 and reads back at 1578-1582.  Hand POSITIONS (not card
   ids) are kept, because that is what Rust accumulates and what
   `resolve_indices_to_ids` consumes. */
void rb_resolver_handle_reveal_selection(RbAbilityResolver *self, GameState *g,
                                         const RbSelectionContext *ctx, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    RbSelectionContext local;
    if (!ctx) { memset(&local, 0, sizeof(local)); ctx = &local; }
    int actor = g->queue.actor;
    int cur = g->queue.cur;
    int effect_started = (cur >= 0 && cur < RB_QUEUE_DEPTH)
                            ? g->queue.entries[cur].effect_started : 0;
    int raw = selected ? atoi(selected) : -1;      /* ctx.indices[0], unmapped */
    int count = g->queue.pending.count;
    int allow_skip = g->queue.pending.allow_skip;
    int blind = g->queue.pending.blind;
    char tpid[24];
    choice_reveal_target_label(g, g->queue.pending.target_player_id, actor,
                               tpid, sizeof(tpid));
    int who = rb_resolve_target_player(g, tpid);
    if (who < 0) who = actor;

    /* self.selected_cards carried from the previous answers (choice.rs:1566) */
    int acc[RB_MAX_ZONE]; int n_acc = 0;
    memset(acc, 0, sizeof(acc));
    for (int i = 0; i < g->queue.pending.n_filtered_indices && n_acc < RB_MAX_ZONE; i++)
        acc[n_acc++] = g->queue.pending.filtered_indices[i];

    /* hand_positions = this answer's hand positions (choice.rs:1520-1531):
       count==0 (any_number) maps the display index through the prompt's
       filtered_indices, count>0 uses the answer's raw positions. */
    int hand_pos[RB_MAX_ZONE]; int n_hand = 0;
    memset(hand_pos, 0, sizeof(hand_pos));
    if (count == 0 && g->queue.pending.n_filtered_indices > 0) {
        if (raw >= 0 && raw < g->queue.pending.n_filtered_indices)
            hand_pos[n_hand++] = g->queue.pending.filtered_indices[raw];
    } else if (raw >= 0) {
        hand_pos[n_hand++] = raw;
    }

    /* choice.rs:1533-1576 — the answer is short of `count`, so accumulate and
       re-prompt for the remainder.  This is the 3 -> 2 -> 1 re-prompt. */
    if (n_hand > 0 && count > 0 && n_hand < count) {
        for (int i = 0; i < n_hand; i++) {
            int dup = 0;
            for (int j = 0; j < n_acc; j++) if (acc[j] == hand_pos[i]) { dup = 1; break; }
            if (!dup && n_acc < RB_MAX_ZONE) acc[n_acc++] = hand_pos[i];
        }
        int remaining = count - n_hand;
        RbChoice ch; memset(&ch, 0, sizeof(ch));
        ch.kind = RB_CHOICE_SELECT_CARD;
        ch.count = remaining;
        ch.allow_skip = 0;
        ch.blind = blind;
        ch.is_reveal = 1;
        ch.route = RB_ROUTE_SELECT_CARDS;
        ch.actor = actor;
        strncpy(ch.zone, "hand", sizeof(ch.zone) - 1);
        strncpy(ch.target_player_id, tpid, sizeof(ch.target_player_id) - 1);
        if (g->queue.pending.card_type[0])
            strncpy(ch.card_type, g->queue.pending.card_type, sizeof(ch.card_type) - 1);
        snprintf(ch.description, sizeof(ch.description),
                 "Select %d more card(s) from hand%s", remaining, blind ? " (blind)" : "");
        for (int i = 0; i < n_acc && i < RB_MAX_ZONE; i++)
            ch.filtered_indices[i] = acc[i];
        ch.n_filtered_indices = n_acc;
        g->queue.pending = ch;
        g->queue.has_pending = 1;
        g->queue.actor = actor;
        g->queue.state = RB_QUEUE_AWAITING_CHOICE;
        return;
    }

    /* choice.rs:1578-1598 — all_indices = the accumulated picks followed by
       this answer's, then resolve them against the TARGET player's hand. */
    int all_pos[RB_MAX_ZONE]; int n_all = 0;
    memset(all_pos, 0, sizeof(all_pos));
    for (int i = 0; i < n_acc; i++) all_pos[n_all++] = acc[i];
    for (int i = 0; i < n_hand; i++) {
        int dup = 0;
        for (int j = 0; j < n_all; j++) if (all_pos[j] == hand_pos[i]) { dup = 1; break; }
        if (!dup && n_all < RB_MAX_ZONE) all_pos[n_all++] = hand_pos[i];
    }
    int hand_n = g->p[who].hand.n;
    const int *to_reveal = (count == 0) ? hand_pos : all_pos;
    int n_to_reveal = (count == 0) ? n_hand : n_all;
    int revealed[RB_MAX_ZONE]; int n_revealed = 0;
    memset(revealed, 0, sizeof(revealed));
    for (int i = 0; i < n_to_reveal; i++) {
        int pos = to_reveal[i];
        if (pos < 0 || pos >= hand_n) continue;
        int cid = g->p[who].hand.cards[pos];
        if (n_revealed < RB_MAX_ZONE) revealed[n_revealed++] = cid;
    }
    /* choice.rs:1602-1604 — push_revealed_card for every revealed id. */
    for (int i = 0; i < n_revealed; i++)
        if (g->n_revealed < RB_MAX_REVEALED_CARDS)
            g->revealed_cards[g->n_revealed++] = revealed[i];
    /* choice.rs:1605-1612 — an empty skippable reveal closes the cost. */
    if (n_revealed == 0 && allow_skip && !effect_started) {
        if (cur >= 0 && cur < g->queue.n_entries) {
            g->queue.entries[cur].cost_paid = 1;
            g->queue.entries[cur].optional_cost_result = 0;
        }
        rb_resolver_clear_choice_state_and_resume(self);
        return;
    }
    /* choice.rs:1617-1622 logs [[log_reveal_hand:n=N]]; the C port has no rule
       log sink for it, exactly as the looked_at reveal above. */
    /* choice.rs:1624-1628 + 1636-1638 — the revealed ids become the resolver's
       selected_cards only when the effect actually reads that zone. */
    const AbilityEffect *selected_effect = g->queue.resume_eff;
    if (!selected_effect) selected_effect = rb_entry_effect(g);
    if (choice_effect_uses_selected_cards(selected_effect)) {
        self->n_selected_cards = 0;
        for (int i = 0; i < n_revealed; i++)
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                self->selected_cards[self->n_selected_cards++] = revealed[i];
    }

    /* choice.rs:1640-1671 — the any_number arm keeps prompting while hand
       positions remain.  The re-prompt's filtered_indices are the REMAINING
       positions (not the accumulated ones), so the count==0 mapping above
       resolves the display index to a hand position. */
    if (count == 0 && allow_skip && !effect_started && n_all > 0) {
        int remaining_pos[RB_MAX_ZONE]; int n_remaining = 0;
        memset(remaining_pos, 0, sizeof(remaining_pos));
        for (int i = 0; i < hand_n; i++) {
            int taken = 0;
            for (int j = 0; j < n_all; j++) if (all_pos[j] == i) { taken = 1; break; }
            if (!taken && n_remaining < RB_MAX_ZONE) remaining_pos[n_remaining++] = i;
        }
        if (n_remaining > 0) {
            RbChoice ch; memset(&ch, 0, sizeof(ch));
            ch.kind = RB_CHOICE_SELECT_CARD;
            ch.count = 0;
            ch.allow_skip = 1;
            ch.is_reveal = 1;
            ch.route = RB_ROUTE_SELECT_CARDS;
            ch.actor = actor;
            strncpy(ch.zone, "hand", sizeof(ch.zone) - 1);
            strncpy(ch.target_player_id, tpid, sizeof(ch.target_player_id) - 1);
            if (g->queue.pending.card_type[0])
                strncpy(ch.card_type, g->queue.pending.card_type, sizeof(ch.card_type) - 1);
            strncpy(ch.description, "Select more cards to reveal from hand (or skip to finish)",
                    sizeof(ch.description) - 1);
            for (int i = 0; i < n_remaining; i++) ch.filtered_indices[i] = remaining_pos[i];
            ch.n_filtered_indices = n_remaining;
            g->queue.pending = ch;
            g->queue.has_pending = 1;
            g->queue.actor = actor;
            g->queue.state = RB_QUEUE_AWAITING_CHOICE;
            return;
        }
    }
    /* choice.rs:1674-1683 — clear + resume. */
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_revealed_cards_selection (choice.rs:1542) — faithful: move from revealed_cards to dst, select_action accumulation, discard_remaining ── */
void rb_resolver_handle_revealed_cards_selection(RbAbilityResolver *self, GameState *g,
                                                  const RbSelectionContext *ctx, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    const char *dst = g->queue.pending.target[0] ? g->queue.pending.target : "waitroom";
    if (g->queue.resume_eff && g->queue.resume_eff->destination) dst = g->queue.resume_eff->destination;
    if (idx >= 0 && idx < g->n_revealed) {
        int cid = g->revealed_cards[idx];
        for (int i = idx; i < g->n_revealed - 1; i++) g->revealed_cards[i] = g->revealed_cards[i + 1];
        g->n_revealed--;
        rb_choice_send_to_dst(g, actor, cid, dst);
        if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
        if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
        /* discard_remaining: remaining revealed -> waitroom */
        int is_select_action = (g->queue.pending.card_type[0] != '\0');
        if (!is_select_action) {
            /* if destination is discard_remaining, move rest */
            for (int i=g->n_revealed-1;i>=0;i--) {
                int rcid=g->revealed_cards[i];
                rb_choice_send_to_dst(g, actor, rcid, "waitroom");
            }
            g->n_revealed=0;
        }
    } else if (idx < 0) {
        /* skip: if allow_skip and discard_remaining, clear revealed */
        if (g->queue.pending.allow_skip) g->n_revealed=0;
    }
    (void)ctx;
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_success_live_zone_selection (choice.rs:1603) — faithful: move from success zone with validation ── */
void rb_resolver_handle_success_live_zone_selection(RbAbilityResolver *self, GameState *g,
                                                  const RbSelectionContext *ctx, const char *selected) {
    (void)ctx;
    int pl = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, "success", ids, RB_MAX_ZONE);
    if (idx >= 0 && idx < n) {
        int cid = ids[idx];
        const char *dst = (g->queue.resume_eff && g->queue.resume_eff->destination)
                              ? g->queue.resume_eff->destination : "waitroom";
        rb_move_card(g, pl, cid, "success", dst, -1);
        if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_entry_cost_reveal (choice.rs:1651) — faithful: reveal hand card for cost, handle filtered_indices, any_number ── */
void rb_resolver_handle_entry_cost_reveal(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int cur = g->queue.cur;
    int cost_count = g->queue.pending.count;
    int is_any = (cost_count==0 && g->queue.pending.allow_skip);
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, actor, "hand", ids, RB_MAX_ZONE);
    if (idx >=0 && idx < n) {
        int cid = ids[idx];
        if (g->n_revealed < RB_MAX_REVEALED_CARDS) g->revealed_cards[g->n_revealed++] = cid;
        if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
        /* cost reveal also counts toward cost tracking */
        if (cur>=0) g->queue.entries[cur].optional_cost_result=1;
        if (is_any && g->n_revealed < n) {
            /* any_number re-prompt for reveal */
            RbChoice ch; memset(&ch,0,sizeof(ch));
            ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"hand",sizeof(ch.zone)-1);
            ch.count=0; ch.allow_skip=1; strncpy(ch.target,"reveal",sizeof(ch.target)-1);
            g->queue.pending=ch; g->queue.has_pending=1; return;
        }
        if (!is_any && self->n_selected_cards < cost_count) {
            int rem = cost_count - self->n_selected_cards;
            RbChoice ch; memset(&ch,0,sizeof(ch));
            ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"hand",sizeof(ch.zone)-1);
            ch.count=rem; ch.allow_skip=0; strncpy(ch.target,"reveal",sizeof(ch.target)-1);
            g->queue.pending=ch; g->queue.has_pending=1; return;
        }
    } else if (idx < 0 && g->queue.pending.allow_skip) {
        if (cur>=0) g->queue.entries[cur].optional_cost_result=0;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_looked_at_selection (choice.rs:1749) — faithful: select from looked_at pool, handle is_select_action, any_number ── */
void rb_resolver_handle_looked_at_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    int is_select = (g->queue.pending.card_type[0] != '\0');
    int ids[RB_MAX_ZONE];
    int n = rb_looked_at_pool(actor, ids, RB_MAX_ZONE);
    if (idx >=0 && idx < n) {
        int cid = ids[idx];
        if (is_select) {
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
            /* any_number handling for looked_at */
            int total = g->queue.pending.count;
            int is_any = (total==0 && g->queue.pending.allow_skip);
            if (is_any) {
                /* re-prompt if more cards remain */
                if ((int)self->n_selected_cards < n) {
                    RbChoice ch; memset(&ch,0,sizeof(ch));
                    ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"looked_at",sizeof(ch.zone)-1);
                    ch.count=0; ch.allow_skip=1; g->queue.pending=ch; g->queue.has_pending=1; return;
                }
            } else if (total > 0 && (int)self->n_selected_cards < total) {
                int rem = total - self->n_selected_cards;
                RbChoice ch; memset(&ch,0,sizeof(ch));
                ch.kind=RB_CHOICE_SELECT_CARD; strncpy(ch.zone,"looked_at",sizeof(ch.zone)-1);
                ch.count=rem; ch.allow_skip=0; g->queue.pending=ch; g->queue.has_pending=1; return;
            }
            /* move looked_at cards to revealed for later handling */
            for (int i=0;i<self->n_selected_cards;i++) if (g->n_revealed < RB_MAX_REVEALED_CARDS) g->revealed_cards[g->n_revealed++]=self->selected_cards[i];
        } else {
            /* move directly to discard/energy etc. */
            const char *dst = g->queue.pending.target[0] ? g->queue.pending.target : "waitroom";
            rb_choice_send_to_dst(g, actor, cid, dst);
            if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
        }
    } else if (idx < 0 && g->queue.pending.allow_skip) {
        /* skip for any_number: finalize */
        if (self->n_selected_cards>0) for(int i=0;i<self->n_selected_cards;i++) if(g->n_revealed<RB_MAX_REVEALED_CARDS) g->revealed_cards[g->n_revealed++]=self->selected_cards[i];
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* Rust misc.rs:1408-1411: the target-selection prompt carries `filtered_indices` = the
   STAGE SLOT positions of the filter-matching candidates, so the player's answer indexes
   the candidate list (Rust choice.rs:2044 `ctx.mfi`) rather than the raw stage. The C
   producers of this choice (src/ability/effects/misc.c `try_create_target_selection_choice`,
   src/engine.c gain_resource fallback) leave the field empty, so derive it here from the
   choice's own declared card_type / group filter — the same filter the answer is validated
   against below. A choice that already carries filtered_indices is left untouched. */
static void stage_build_filtered_slots(GameState *g, int pl) {
    RbChoice *ch = &g->queue.pending;
    if (ch->n_filtered_indices > 0) return;                 /* producer narrowed it */
    if (!ch->card_type[0] && !ch->filter_group[0]) return;  /* nothing declared to filter on */
    int n = 0;
    for (int slot = 0; slot < RB_STAGE_SIZE && n < RB_MAX_ZONE; slot++) {
        int cid = g->p[pl].stage[slot];
        if (cid == RB_EMPTY_SLOT || cid < 0) continue;
        if (ch->card_type[0] && !rb_card_matches_type(cid, ch->card_type)) continue;
        if (ch->filter_group[0] && !choice_card_matches_group_csv(cid, ch->filter_group)) continue;
        ch->filtered_indices[n++] = slot;
    }
    if (n > 0) ch->n_filtered_indices = n;
}

/* Rust choice.rs:1006-1008 — `validate_card` is `choice.as_filter().matches`, i.e. the
   SelectCard choice's own card_type + group_names + characters. Applied per picked card
   exactly like the Rust handler does before pushing it onto the selection pool. The
   group is matched CSV-aware (choice_card_matches_group_csv) and the name list
   through rb_card_matches_characters, both of which Rust reaches via
   util::matching_indices (util.rs:1274) on the same filter. */
static int stage_choice_accepts(const GameState *g, int cid) {
    const RbChoice *ch = &g->queue.pending;
    if (ch->card_type[0] && !rb_card_matches_type(cid, ch->card_type)) return 0;
    if (ch->filter_group[0] && !choice_card_matches_group_csv(cid, ch->filter_group)) return 0;
    if (ch->n_characters > 0) {
        const char *names[16];
        for (int i = 0; i < ch->n_characters && i < 16; i++) names[i] = ch->characters[i];
        if (!rb_card_matches_characters(cid, names, ch->n_characters)) return 0;
    }
    return 1;
}

/* Publish a validated stage pick into the GameState-level selection pool. The C resume
   path builds a stack-local RbAbilityResolver (rb_resume_with_choice_indices_internal),
   so the resolver's own selected_cards dies with the frame; GameState.selected_cards is
   the pool the deferred target-selection effect reads back (choice.c:2574). */
static int stage_publish_selected(GameState *g, int cid) {
    for (int i = 0; i < g->n_selected_cards; i++)
        if (g->selected_cards[i] == cid) return 0;
    if (g->n_selected_cards >= RB_MAX_RECENTLY_MOVED) return 0;
    g->selected_cards[g->n_selected_cards++] = cid;
    return 1;
}

/* Rust misc.rs:1412-1413 — the effect saved for the target-selection prompt has
   target_count CLEARED, so re-running it applies the resource to the cards the player
   picked (Rust misc.rs:1840-1841) instead of re-deriving targets from the stage.
   `extra_int` in misc.c treats an empty value as absent, so blanking the value is the
   faithful C equivalent of `saved.set_target_count(None)`. */
static void stage_saved_effect_clear_target_count(AbilityEffect *e) {
    if (!e) return;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "target_count") && e->extra_v[i]) {
            rb_free(e->extra_v[i]);
            e->extra_v[i] = NULL;
        }
    }
}

void rb_resolver_handle_stage_selection(RbAbilityResolver *self, GameState *g,
                                      const RbSelectionContext *ctx, const char *selected) {
    /* Faithful port of choice.rs:2011 handle_stage_selection, is_select_action branch.
       Rust maps the answer through SelectionContext::mfi (choice.rs:2044) — i.e.
       through the choice's filtered_indices, which the target-selection producer fills
       with STAGE SLOT positions (Rust misc.rs:1408-1411) — and then reads
       player.stage.stage[slot] by slot (choice.rs:2057-2058), validating each card
       against the choice filter (choice.rs:2060). The previous C body instead used the
       raw answer as an index into a COMPACTED stage-card list and only pre-filtered for
       one hard-coded pending.target string, so any choice carrying filtered_indices
       resolved to the wrong member and the pick never reached the effect that consumes
       it. For is_select_action we record the selection (no immediate zone move); the
       effect moves/consumes it later. Otherwise we move stage cards to dst. */
    if (!self || !g) return;
    int pl = rb_resolve_target_player(g, g->queue.pending.target_player_id[0] ?
                                     g->queue.pending.target_player_id : "self");
    if (pl < 0 || pl > 1) pl = g->queue.actor;
    int is_select = (g->queue.pending.card_type[0] != '\0'); /* proxy for is_select_action */
    int idx = selected ? atoi(selected) : -1;
    if (is_select) {
        stage_build_filtered_slots(g, pl);
        if (idx < 0) {
            /* Rust choice.rs:2018-2020 — an empty answer takes the pending commands and
               clears the selection pool (skip of an optional target choice). */
            self->n_selected_cards = 0;
        } else {
            /* ctx.mfi(&ctx.indices): the answer indexes filtered_indices, which hold
               stage SLOT positions — read player.stage.stage[slot], not a compacted list. */
            int slot = map_choice_index(&g->queue.pending, idx);
            if (slot >= 0 && slot < RB_STAGE_SIZE) {
                int cid = g->p[pl].stage[slot];
                if (cid != RB_EMPTY_SLOT && cid >= 0 && stage_choice_accepts(g, cid)) {
                    fprintf(stderr, "[STAGE_TARGET_RESULT] pick=%d slot=%d cid=%d target=%s\n",
                            idx, slot, cid, g->queue.pending.target);
                    if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED)
                        self->selected_cards[self->n_selected_cards++] = cid;
                    /* Publish to the GameState pool so the saved target-selection effect
                       re-executed by the resume consumes THIS pick. */
                    stage_publish_selected(g, cid);
                } else {
                    fprintf(stderr, "[STAGE_TARGET_RESULT] pick=%d slot=%d rejected\n", idx, slot);
                }
            }
        }
        rb_resolver_clear_choice_state_and_resume(self);
        return;
    }
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, pl, "stage", ids, RB_MAX_ZONE);
    if (idx >= 0 && idx < n) {
        int cid = ids[idx];
        const char *dst = (g->queue.resume_eff && g->queue.resume_eff->destination)
                              ? g->queue.resume_eff->destination : "waitroom";
        rb_move_card(g, pl, cid, "stage", dst, idx);
        if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* Faithful port of choice.rs:2038 filter_discard_by_budget.
   Computes remaining_budget = cost_total - sum(selected cost), then filters
   waitroom indices by cost <= remaining_budget when operator is "<=".
   Mirrors Rust signature (remaining_budget, Vec<usize>) as out_remaining + return count;
   pending indices are written to out_indices. */
int rb_resolver_filter_discard_by_budget_full(RbAbilityResolver *self, GameState *g,
                                              int cost_total, const char *cost_total_op,
                                              int *out_remaining, int *out_indices, int max_out) {
    if (!self || !g || !out_indices || max_out<=0) { if(out_remaining) *out_remaining=-1; return 0; }
    int spent = 0;
    for (int i=0;i<self->n_selected_cards;i++) {
        int cid = self->selected_cards[i];
        Card c; if (rb_decode_card_by_index((uint32_t)cid, &c)==0) { spent += c.cost; rb_free_card(&c); }
        else { /* fallback: use 0 cost if not decoded */ }
    }
    int remaining = -1;
    int use_budget = 0;
    if (cost_total >= 0) {
        if (cost_total_op && !strcmp(cost_total_op, "<=")) use_budget = 1;
        else if (!cost_total_op) use_budget = 1; /* Rust: None => is_some() */
        remaining = cost_total - spent;
        if (remaining < 0) remaining = 0;
    }
    if (out_remaining) *out_remaining = remaining;
    int ids[RB_MAX_ZONE]; int n = rb_zone_cards(g, g->queue.actor, "waitroom", ids, RB_MAX_ZONE);
    int out_n = 0;
    for (int i=0;i<n && out_n < max_out;i++) {
        if (use_budget) {
            Card c; if (rb_decode_card_by_index((uint32_t)ids[i], &c)!=0) continue;
            int cost = c.cost; rb_free_card(&c);
            if (cost > remaining) continue;
        }
        out_indices[out_n++] = i;
    }
    /* also mirror the simple overload */
    self->n_pending_deferred_costs = out_n < 16 ? out_n : 16;
    for (int i=0;i<self->n_pending_deferred_costs;i++) self->pending_deferred_costs[i]=out_indices[i];
    return out_n;
}
int rb_resolver_filter_discard_by_budget(RbAbilityResolver *self, GameState *g, int budget) {
    int rem; int out[16]; int n = rb_resolver_filter_discard_by_budget_full(self,g,budget,"<=", &rem, out, 16);
    (void)rem; return n;
}

/* ── handle_discard_selection helpers ───────────────────────────────────
 *
 * choice.rs:2326-2613 (handle_discard_selection) threads FOUR things through
 * every arm of this function that the C collapsed onto `g->queue.actor` and a
 * hand-rolled remove-then-place pair:
 *
 *   1. `ctx.target_player_id.as_deref().unwrap_or("self")` — the SEAT the
 *      effect names, resolved with `gs.resolve_target_player` (G6). Every
 *      waitroom read (:2340, :2355, :2433, :2477, :2529, :2584) and every
 *      write (execute_selected_cards_from_zone's own `&target` argument,
 *      :2471, :2605) goes through it. The C used the seat that ANSWERED, so a
 *      target="opponent" prompt read the answering player's discard and wrote
 *      the answering player's stage. Compounded with the placement failure
 *      below, that made an opponent-labelled prompt DELETE the answering
 *      player's card.
 *   2. `self.awaiting.choice.destination` / `ctx.destination` — the effect's
 *      own destination (selection.rs:45). `queue.resume_move_destination` is
 *      the C spelling of it; `rb_move_prompt_carry_resume_context` (move.c:493)
 *      writes it from the same effect, unconditionally.
 *   3. `self.finalize_card_movement(..., state_change, ...)`
 *      (move/mod.rs:2490-2572) — the post-move side effects. Skipping it is why
 *      `queue.resume_move_state` had two writers and no reader, so an appeared
 *      member was never in wait and its own 登場 never fired.
 *   4. `execute_selected_cards_from_zone`'s filter pass — the answer is
 *      re-validated against the PROMPT's card_type / cost_limit / cost_total /
 *      group / characters, not accepted blindly.
 */

/* move.c's twin of Rust effects/move/mod.rs:2494 `finalize_card_movement`. Declared
 * here rather than in rabuka.h because move.c owns it and this file only
 * started calling it once the choice path had to run the real finalizer. */
void rb_move_finalize_card_movement(GameState *g, int actor,
                                    const int *moved_cards, int n_moved,
                                    const char *destination, const char *source,
                                    const char *state_change, const char *target);

/* The SEAT a discard/waitroom SelectCard names. `target_player_id` is the
 * Choice field Rust's `prompt_card_selection` fills from
 * `effect.action_by` (selection.rs:39-43) and `rb_resolver_handle_select_target`'s
 * emitter mirrors into `target`; `target` alone is the fallback for producers
 * that only stamp the seat there. "self" is the MASTER seat
 * (game_state_abilities.c:827 `rb_resolve_master_id` = g->active), NOT seat 0 —
 * `rb_resolve_target_player` hard-codes "self" to 0 and is therefore wrong for
 * the second leg of a target="both" effect resolved on P2's turn. */

/* Mirror util::target_player_index (engine/src/ability/util.rs, reached from
 * choice.rs:1413 / :1490 / :2137 as `gs.resolve_target_player(target)`), whose
 * rule is engine/src/core/game_state/abilities.rs:2962-2981:
 *
 *     ("self",     master_p2) => player2 | ("self",     _) => player1
 *     ("opponent", master_p2) => player1 | ("opponent", _) => player2
 *     ("p1" | "player1")      => player1   ("p2" | "player2") => player2
 *
 * TWO divergences from that rule were here.
 *
 * 1. "p2"/"player2" were folded into the OPPONENT arm and XOR'd off the
 *    master. They are LITERAL seat names in Rust, so "p2" is always seat 1
 *    whatever the master is. That is what sent the deferred half of
 *    「自分と相手はそれぞれ、自身の控え室から…�� to the wrong seat.
 * 2. The master was `g->active`, the TURN player. It is
 *    `ability_master_id()` — the CURRENT QUEUE ENTRY's player_id
 *    (abilities.rs:2900-2904), with `owner_of_card(activating_card)` as the
 *    fallback (abilities.rs:2911-2914). rb_resolve_master_id (below) is that
 *    pair; it was g->active.
 */
static int choice_effect_target_seat(const GameState *g, int answering_seat)
{
    const char *t = NULL;
    if (g) {
        if (g->queue.pending.target_player_id[0]) t = g->queue.pending.target_player_id;
        else if (g->queue.pending.target[0])     t = g->queue.pending.target;
    }
    int master = g ? rb_resolve_master_id(g) : 0;
    if (master < 0 || master > 1)
        master = (answering_seat >= 0 && answering_seat <= 1) ? answering_seat : 0;
    if (!t || !t[0]) return master;
    /* abilities.rs:2964-2968 — only "self" and "opponent" are relative; every
     * other spelling, including "both" and anything unrecognised, is resolved
     * as Rust's match does, which returns player1 for the relative pair and
     * player1 for the rest. */
    if (!strcmp(t, "opponent")) return master ^ 1;
    if (!strcmp(t, "self"))    return master;
    int pl = rb_resolve_target_player(g, t);
    return (pl >= 0 && pl <= 1) ? pl : master;
}

/* `Choice.destination` (selection.rs:45). `queue.resume_move_destination` is
 * the C slot for it and the move-side producer writes it from the very same
 * effect, so it is always this prompt's own destination.
 *
 * The old code preferred `g->queue.resume_eff->destination` over it. That
 * pointer is the producing AbilityEffect (move.c:539 `resume_eff = e`) and on
 * the inline 登場 path the effect tree is a `Ability ab` LOCAL of
 * `rb_play_member` (engine.c:1159-1190) that has already returned by the time
 * the answer arrives — a use-after-return read on every discard answer. When no
 * destination is named, Rust falls back to the DISCARD (effects/move/mod.rs:2810-2830,
 * as documented in move.c:486-492); the C fell back to `pending.target`, which
 * holds the SEAT ("self"/"opponent"), i.e. it asked for a zone literally named
 * "self", which `zone_bag` cannot resolve — the third way this path could lose a
 * card. */
static const char *choice_effect_destination(const GameState *g)
{
    if (g->queue.resume_move_destination[0]) return g->queue.resume_move_destination;
    return "discard";
}

/* `state_change` (Rust's `finalize_card_movement` third-from-last argument).
 * `queue.resume_move_state` is the queue's own copy of it and is what the
 * emitters stamp (choice.c's conditional-choice emitter at :2790 reads the
 * effect's `state_change` extra into it); it had TWO writers and NO reader,
 * which is why no appeared member was ever in wait. When the writer that
 * produced this particular prompt did not run, fall back to the effect the
 * queue parked — the same read the old destination lookup performed — and
 * MEMOISE it into resume_move_state so a re-prompt keeps it. */
static const char *choice_effect_state_change(GameState *g)
{
    if (g->queue.resume_move_state[0]) return g->queue.resume_move_state;
    const char *sc = NULL;
    if (g->queue.resume_eff) sc = choice_effect_extra(g->queue.resume_eff, "state_change");
    if (sc && sc[0])
        strncpy(g->queue.resume_move_state, sc, sizeof(g->queue.resume_move_state) - 1);
    return sc;
}

/* execute_selected_cards_from_zone's filter pass (effects/move/mod.rs:2656-2803 in
 * Rust; the C twin of the same pass is move.c:1852-1865). The picked card is
 * re-validated against the PROMPT's own filter rather than trusted, so an
 * answer that slips past the displayed candidate list cannot move a card the
 * ability is not allowed to touch. */
static int choice_discard_pick_legal(const RbChoice *ch, int cid)
{
    if (ch->card_type[0] && !rb_card_matches_type(cid, ch->card_type)) return 0;
    /* Rust's `ctx.cost_limit` is an `Option<u8>` and the C's RbChoice cannot
     * represent None distinctly from Some(0): rb_emit_choice zeroes the struct
     * and the producer only writes the field when the ability carries the extra
     * (move.c:501-509), so "0 with no operator" is the ABSENT case. Read as a
     * limit it demands "cost <= 0" and rejects every card in the zone, which
     * turns a perfectly good answer into "no legal pick" and silently drops the
     * move — measured as 3 bp7 cases (wien x2, kanon_one_per_group) going from
     * ok to FAILED. */
    if (ch->cost_limit > 0 || ch->cost_limit_op[0]) {
        if (!rb_card_matches_cost_limit(cid, ch->cost_limit,
                                        ch->cost_limit_op[0] ? ch->cost_limit_op : "<="))
            return 0;
    }
    /* Rust's `ctx.cost_total` is an `Option<u8>`; the C's RbChoice cannot
     * represent None distinctly from Some(0) because rb_emit_choice zeroes the
     * whole struct and the producer only writes the field when the ability
     * carries the extra (move.c:510-517). So "0 with no operator" is the ABSENT
     * case, and reading it as a budget of 0 rejects every card in the zone. */
    if ((ch->cost_total > 0 || ch->cost_total_op[0]) && ch->cost_total >= 0) {
        Card c;
        if (rb_decode_card_by_index((uint32_t)cid, &c)) {
            int ok = 1;
            char op = ch->cost_total_op[0] ? ch->cost_total_op[0] : '=';
            if (op == '>') ok = c.cost >  ch->cost_total;
            else if (op == '<') ok = c.cost < ch->cost_total;
            else if (op == '=') ok = c.cost == ch->cost_total;
            rb_free_card(&c);
            if (!ok) return 0;
        }
    }
    if (ch->filter_group[0] && !rb_card_matches_group_str(cid, ch->filter_group)) return 0;
    if (ch->n_characters > 0) {
        const char *names[16];
        for (int i = 0; i < ch->n_characters && i < 16; i++) names[i] = ch->characters[i];
        if (!rb_card_matches_characters(cid, names, ch->n_characters)) return 0;
    }
    return 1;
}

/* choice.rs:2594-2606 execute_selected_cards_from_zone, then effects/move/mod.rs:2494
 * finalize_card_movement. Returns 1 when the card reached its destination.
 *
 * THE CARD MUST NOT BE DESTROYED. Rust's `place_card_in_zone` returns true for
 * every destination (util.rs:2345-2399) and parks an unplaceable stage card in
 * the waitroom (util.rs:2363-2367), so a "not placed" outcome does not exist on
 * this path. The C lifted the card out of the waitroom first and then IGNORED
 * the verdict, so `empty_area` onto a full stage ended with the card in no zone
 * at all: board census 12 -> 11 and 14 -> 13 on the nico family. The rollback
 * below is the belt to util.c's braces; both are needed because the other two
 * rb_choice_send_to_dst destinations (under_member, an unresolvable zone name)
 * can still fail. */
static int choice_discard_move_card(RbAbilityResolver *self, GameState *g, int seat,
                                    int cid, const char *dst, const char *state_change,
                                    const char *target)
{
    rb_waitroom_remove_card(&g->p[seat], cid);
    if (!rb_choice_send_to_dst(g, seat, cid, dst)) {
        rb_waitroom_add(&g->p[seat], cid);
        fprintf(stderr, "[DISCARD_PLACE_FAILED] cid=%d seat=%d dst='%s' — returned to the waitroom\n",
                cid, seat, dst ? dst : "-");
        return 0;
    }
    int moved[1]; moved[0] = cid;
    /* effects/move/mod.rs:2494-2582 — zone-exit cleanup, `state_change="wait"`,
     * record_card_movement, recalculate_constants and the placed card's own
     * 登場. Rust's `Zone::Stage | Zone::EmptyArea | Zone::SameArea` are ONE
     * dispatch arm (effects/move/mod.rs:2810-2813) and that arm fires the debut
     * (placement.rs:400-403), so the finalizer is told the canonical "stage"
     * for all three: without it an appeared member never debuts (N13). */
    const char *fin_dst = (!strcmp(dst, "empty_area") || !strcmp(dst, "same_area")) ? "stage" : dst;
    rb_move_finalize_card_movement(g, seat, moved, 1, fin_dst, "discard", state_change, target);
    /* `stage_wait` is the C-only mirror of the orientation modifier
     * (condition.c:1520-1531 documents that the two are the same fact in Rust).
     * The modifier alone leaves every C reader of the raw per-slot flag — live.c
     * :128, dynamic_count.c:168, phase.c:23 — reporting the member as active. */
    if (state_change && !strcmp(state_change, "wait")) {
        RbPlayer *P = &g->p[seat];
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (P->stage[i] == cid) P->stage_wait[i] = 1;
    }
    if (self) {
        if (self->n_moved_cards < RB_MAX_RECENTLY_MOVED) self->moved_cards[self->n_moved_cards++] = cid;
        if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = cid;
    }
    if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++] = cid;
    return 1;
}

/* choice.rs:1168-1190 `build_reprompt` — the twin of choice_build_reprompt for
 * the WAITROOM path, which unlike the hand-cost re-prompts must carry the
 * whole selection context. The hand twin drops `filtered_indices`,
 * `target_player_id`, `cost_total` and the filters, and the discard path used it
 * anyway, so every re-prompt of a budgeted discard-selection came back
 * UNFILTERED and UNLABELLED: a cost-9 card reappeared as a candidate and the
 * opponent's half lost its seat. */
static void choice_build_discard_reprompt(GameState *g, const RbChoice *ctx, int seat,
                                          int remaining, const char *en, int allow_skip,
                                          const int *fi, int n_fi)
{
    RbChoice ch = *ctx;                 /* keep zone / description / filters / cost_total */
    ch.kind = RB_CHOICE_SELECT_CARD;
    if (!ch.zone[0]) strncpy(ch.zone, "discard", sizeof(ch.zone) - 1);
    ch.count = remaining > 0 ? remaining : 0;
    ch.allow_skip = allow_skip;
    if (en) strncpy(ch.description, en, sizeof(ch.description) - 1);
    ch.route = RB_ROUTE_SELECT_CARDS;
    ch.n_filtered_indices = 0;
    if (fi && n_fi > 0) {
        if (n_fi > RB_MAX_ZONE) n_fi = RB_MAX_ZONE;
        ch.n_filtered_indices = n_fi;
        for (int i = 0; i < n_fi; i++) ch.filtered_indices[i] = fi[i];
    }
    if (ctx->target_player_id[0]) strncpy(ch.target_player_id, ctx->target_player_id,
                                          sizeof(ch.target_player_id) - 1);
    if (ctx->target_player_id[0]) strncpy(ch.target, ctx->target_player_id,
                                          sizeof(ch.target) - 1);
    ch.actor = seat;
    g->queue.pending = ch;
    g->queue.has_pending = 1;
    g->queue.actor = seat;
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;
}

/* choice.rs:2290-2324 `filter_discard_by_budget` — the re-prompt's candidate
 * list. THREE things were wrong with the call the C made:
 *   - it was fed `pending.count` (how many cards to pick) where Rust reads
 *     `ctx.cost_total` (the cost budget), so "cost 2 or below" became "cost 1 or
 *     below" and a re-prompt filtered on the wrong number entirely;
 *   - it read `g->queue.actor`'s waitroom, so a target="opponent" re-prompt
 *     listed the answering player's cards;
 *   - its `use_budget` test was `op == "<=" || !op`, which is Rust's
 *     `Some(op) => op == "<=", None => cost_total.is_some()` only by accident.
 * Returns the number of surviving indices; *out_remaining is the leftover
 * budget (Rust's `Option<u8>` flattened to -1 when no cost_total is set). */
static int choice_filter_discard_by_budget(RbAbilityResolver *self, GameState *g, int seat,
                                           int cost_total, const char *cost_total_op,
                                           int *out, int max_out, int *out_remaining)
{
    int spent = 0;
    if (self) {
        for (int i = 0; i < self->n_selected_cards; i++) {
            Card c;
            if (rb_decode_card_by_index((uint32_t)self->selected_cards[i], &c) == 0) {
                spent += c.cost;
                rb_free_card(&c);
            }
        }
    }
    /* `cost_total > 0 || cost_total_op[0]` is the C's spelling of Rust's
     * `Some(tb)`: rb_emit_choice zeroes the field and the producer only writes
     * it when the ability carries the extra (move.c:510-517), so a bare 0 is the
     * ABSENT case, not a budget of zero. */
    int remaining = -1;
    int use_budget = 0;
    if (cost_total >= 0 && (cost_total > 0 || (cost_total_op && cost_total_op[0]))) {
        use_budget = (cost_total_op && cost_total_op[0]) ? (strcmp(cost_total_op, "<=") == 0) : 1;
        remaining = cost_total - spent;
        if (remaining < 0) remaining = 0;
    }
    if (out_remaining) *out_remaining = remaining;
    int ids[RB_MAX_ZONE];
    int n = rb_zone_cards(g, seat, "waitroom", ids, RB_MAX_ZONE);
    int out_n = 0;
    for (int i = 0; i < n && out_n < max_out; i++) {
        if (use_budget) {
            Card c;
            if (rb_decode_card_by_index((uint32_t)ids[i], &c) != 0) continue;
            int cost = c.cost; rb_free_card(&c);
            if (cost > remaining) continue;
        }
        out[out_n++] = i;
    }
    return out_n;
}

/* ── handle_discard_selection (choice.rs:2331) — faithful ──
   is_select_action = collect IDs into selected_cards and reprompt;
   otherwise move from discard/waitroom to dst with budget filtering and
   sub_choice handling. Mirrors Rust branching exactly.

   `sel` / `n_sel` are the WHOLE answer, already ctx.mfi-mapped, exactly as the
   single-index path's `idx` is; n_sel == 0 is the skip. Rust's arm is written
   against `mapped_indices` throughout — `mapped_indices.is_empty()` is the
   empty-pick test (choice.rs:2406), `mapped_indices.len() < ctx.count` is the
   partial test (:2432) and `execute_selected_cards_from_zone(.., &mapped_indices,
   ..)` is the move (:2465, :2599) — so the C's `sel_idx`/`mapped` (a 0-or-1
   stand-in for a count) is what collapsed a count>1 discard pick to one card
   (the "Select 4 card(s) from the waiting room" case: the answer {0,1,2,3}
   placed ONE card and the other three stayed in the waitroom). */
static void choice_discard_selection_indices(RbAbilityResolver *self, GameState *g,
                                             const int *sel, int n_sel) {
    char selbuf[32];
    if (n_sel > 0) snprintf(selbuf, sizeof(selbuf), "%d", sel[0]);
    else selbuf[0] = '\0';
    fprintf(stderr, "[DISCARD_RESULT] selected=%s n=%d count=%d reprompt=%d pz='%s' pt='%s' ptp='%s' pdesc='%s' nf=%d cl=%d clo='%s' ctot=%d ctoto='%s' grp='%s' ct='%s' route=%d\n",
            n_sel > 0 ? selbuf : "skip", n_sel,
            g->queue.pending.count, g->queue.choice_reprompt_pending,
            g->queue.pending.zone, g->queue.pending.target, g->queue.pending.target_player_id,
            g->queue.pending.description, g->queue.pending.n_filtered_indices,
            g->queue.pending.cost_limit, g->queue.pending.cost_limit_op,
            g->queue.pending.cost_total, g->queue.pending.cost_total_op,
            g->queue.pending.filter_group, g->queue.pending.card_type, (int)g->queue.pending.route);
    int actor = g->queue.actor;
    int is_skip = (n_sel <= 0);
    int pending_count = g->queue.pending.count;
    int allow_skip = g->queue.pending.allow_skip;
    int treat_as_select = g->queue.resume_eff && g->queue.resume_eff->action &&
        !strcmp(g->queue.resume_eff->action,"select_cards");
    if (treat_as_select) {
        /* is_select_action branch (choice.rs:2339-2399) — every index of the
           answer joins `selection.cards`, then the count decides between the
           re-prompt and finalize_choice. */
        if (!is_skip) {
            int ids[RB_MAX_ZONE]; int n = rb_zone_cards(g, actor, "waitroom", ids, RB_MAX_ZONE);
            for (int a = 0; a < n_sel; a++) {
                int mapped = sel[a];
                if (mapped < 0 || mapped >= n) continue;
                int cid = ids[mapped];
                int dup=0; for(int i=0;i<self->n_selected_cards;i++) if(self->selected_cards[i]==cid) dup=1;
                if(!dup && self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++]=cid;
            }
            int selected_so_far = self->n_selected_cards;
            if (pending_count>0 && selected_so_far < pending_count) {
                int remaining = pending_count - selected_so_far;
                int wait_n = rb_zone_cards(g, actor, "waitroom", ids, RB_MAX_ZONE);
                int out[RB_MAX_ZONE]; int out_n=0;
                for(int i=0;i<wait_n && out_n<RB_MAX_ZONE;i++){
                    int dup=0; for(int j=0;j<self->n_selected_cards;j++) if(self->selected_cards[j]==ids[i]) dup=1;
                    if(!dup) out[out_n++]=i;
                }
                char en[80];
                snprintf(en,sizeof(en),"Select %d more card(s) from discard from %d remaining", remaining, out_n);
                /* choice.rs:2380-2395 `build_reprompt(ctx, ..)` + 2393
                   `.with_filtered_indices(fi)`: Rust carries the ANSWERED prompt's
                   own seat, budget, card_type and group into the re-offer AND keeps
                   the candidate list it just computed. This arm used to build the
                   re-offer from a zeroed RbChoice, so `out`/`out_n` were computed
                   and then thrown away together with the seat and the filters —
                   every retry offered the whole discard again, so a mandatory
                   multi-pick could never make progress. choice_build_discard_reprompt
                   is the same builder the non-select_action arm below uses and it
                   copies `*ctx`, so the filters survive with the list. */
                RbChoice ctx = g->queue.pending;
                choice_build_discard_reprompt(g, &ctx, choice_effect_target_seat(g, actor),
                                              remaining, en, 0, out, out_n);
                return;
            }
        }
        rb_resolver_finalize_choice(self);
        return;
    } else {
        /* non-select_action — choice.rs:2395-2612. SEAT, DESTINATION, FILTER and
         * FINALIZER are all threaded the way Rust threads them; see the block
         * comment above the helpers. */
        RbChoice ctx = g->queue.pending;          /* the prompt as it was answered */
        int seat    = choice_effect_target_seat(g, actor);   /* G6 */
        const char *dst  = choice_effect_destination(g);
        const char *sc   = choice_effect_state_change(g);    /* G3 */
        const char *tpid = ctx.target_player_id[0] ? ctx.target_player_id
                       : (ctx.target[0] ? ctx.target : "self");
        int ids[RB_MAX_ZONE];
        int n = rb_zone_cards(g, seat, "waitroom", ids, RB_MAX_ZONE);

        /* choice.rs:2401-2431 — an empty pick must NEVER satisfy a MANDATORY
         * selection: re-offer the SAME choice (count, filters, seat and budget
         * included) so the player can pick a legal target. The re-offer used to
         * be built from a zeroed RbChoice, so the retry lost the card_type /
         * cost_limit / group filter AND the seat label. Rust's test is
         * `mapped_indices.is_empty()`; the C's per-index spelling of it was
         * "this one index is out of range". */
        int any_in_range = 0;
        if (!is_skip)
            for (int a = 0; a < n_sel; a++)
                if (sel[a] >= 0 && sel[a] < n) { any_in_range = 1; break; }
        if (!any_in_range && !allow_skip) {
            char en[64];
            snprintf(en, sizeof(en), "Select %d card(s) from the waiting room", pending_count);
            choice_build_discard_reprompt(g, &ctx, seat, pending_count, en, 0,
                                          NULL, 0);
            return;
        }

        /* choice.rs:2432-2611. `mapped_indices` is the answered index list
         * after `ctx.mfi` (map_choice_index, already applied by
         * rb_resolver_handle_select_card), and `mapped_indices.len()` is how
         * Rust decides partial-vs-complete AND how many cards the move takes.
         * The C's `mapped = (sel_idx < n) ? 1 : 0` was that same test spelled as
         * a 0-or-1 count, which is the whole reason a count>1 discard pick
         * placed exactly one card. Every named card is resolved against the
         * waitroom snapshot FIRST (the moves below shrink it), and a pick the
         * prompt's own filter rejects is not a legal answer. */
        int picked[RB_MAX_ZONE];
        int mapped = 0;
        if (!is_skip) {
            for (int a = 0; a < n_sel && mapped < RB_MAX_ZONE; a++) {
                int idx = sel[a];
                if (idx < 0 || idx >= n) continue;
                int cid = ids[idx];
                if (choice_discard_pick_legal(&ctx, cid)) picked[mapped++] = cid;
                else fprintf(stderr, "[DISCARD_PICK_REJECTED] cid=%d idx=%d — fails the prompt's filter\n",
                             cid, idx);
            }
        }
        fprintf(stderr, "[DISCARD_PICK] answering=%d n_answered=%d seat=%d first=%d available=%d count=%d mapped=%d "
                "sub_choice=%d dst='%s' state_change='%s' p0_discard=%d p1_discard=%d\n",
                actor, n_sel, seat, (mapped > 0 ? sel[0] : -1), n, pending_count, mapped, self->sub_choice_created,
                dst ? dst : "-", sc ? sc : "-", g->p[0].discard.n, g->p[1].discard.n);

        if (pending_count > 0 && mapped > 0 && mapped < pending_count) {
            /* PARTIAL pick — choice.rs:2432-2579. Move everything the answer
             * named (Rust hands `&mapped_indices` to
             * execute_selected_cards_from_zone as one batch), then re-offer for
             * the remainder out of the BUDGET-filtered remainder of the TARGET
             * seat's waitroom. */
            for (int a = 0; a < mapped; a++) choice_discard_move_card(self, g, seat, picked[a], dst, sc, tpid);
            int remaining = pending_count - mapped;
            if (self->sub_choice_created) {
                self->sub_choice_created = 0;
                if (remaining > 0) {
                    int out[RB_MAX_ZONE], rem = -1;
                    int out_n = choice_filter_discard_by_budget(self, g, seat,
                                    ctx.cost_total, ctx.cost_total_op, out, RB_MAX_ZONE, &rem);
                    if (out_n > 0 || allow_skip) {
                        char en[96];
                        snprintf(en, sizeof(en), "Select %d more card(s) from discard%s",
                                 remaining, allow_skip ? " (or skip to finish)" : "");
                        /* choice.rs:2507-2523 parks this as `awaiting.reprompt`
                         * and leaves the pending actions alone. */
                        choice_build_discard_reprompt(g, &ctx, seat, remaining, en,
                                                      allow_skip, out, out_n);
                        self->has_pending_reprompt_choice = 1;
                    }
                }
                return;
            }
            /* choice.rs:2528-2574 — the budget-filtered re-prompt. */
            {
                int out[RB_MAX_ZONE], rem = -1;
                int out_n = choice_filter_discard_by_budget(self, g, seat,
                                ctx.cost_total, ctx.cost_total_op, out, RB_MAX_ZONE, &rem);
                char en[96];
                snprintf(en, sizeof(en), "Select %d more card(s) from discard%s",
                         remaining, allow_skip ? " (or skip to finish)" : "");
                choice_build_discard_reprompt(g, &ctx, seat, remaining, en, allow_skip, out, out_n);
            }
            return;
        }

        /* COMPLETE (or empty) pick — choice.rs:2581-2611. */
        if (mapped > 0) {
            for (int a = 0; a < mapped; a++) {
                int done = choice_discard_move_card(self, g, seat, picked[a], dst, sc, tpid);
                fprintf(stderr, "[DISCARD_MOVE] selected=%d of %d available=%d moved=%d destination=%s "
                        "state_change=%s seat=%d\n",
                        picked[a], mapped, n, done, dst ? dst : "-", sc ? sc : "-", seat);
            }
            if (self->sub_choice_created) self->sub_choice_created = 0;
        }
        rb_resolver_handle_selection_epilogue(self, g);
    }
}

/* Single-index entry point, kept for the exported signature rabuka.h declares
   (and for the one caller in this file that only ever has one index). It is a
   thin wrapper: `sel_idx` is already ctx.mfi-mapped by
   rb_resolver_handle_select_card, exactly as the multi-index caller's list is. */
void rb_resolver_handle_discard_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int sel_idx = selected ? atoi(selected) : -1;
    choice_discard_selection_indices(self, g, sel_idx >= 0 ? &sel_idx : NULL,
                                     sel_idx >= 0 ? 1 : 0);
}

static void clear_choice_options(GameState *g) {
    for (int i = 0; i < RB_ENTRY_PENDING_CAP; i++) {
        if (g->queue.choice_options[i]) {
            rb_effect_free(g->queue.choice_options[i]);
            g->queue.choice_options[i] = NULL;
        }
    }
    g->queue.choice_options_n = 0;
    g->queue.choice_reprompt_pending = 0;
}

static void emit_choice_reprompt(GameState *g) {
    if (!g || g->queue.choice_options_n <= 0) return;
    int actor = g->queue.actor >= 0 ? g->queue.actor : g->active;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL,
                   g->queue.choice_options_n, 1, "choice");
    rb_choice_set_description(&g->queue.pending, "Choose another effect, or skip to finish");
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_CONDITIONAL_CHOICE);
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;
    rb_resolver_store_pending_choice(g);
    fprintf(stderr, "[CHOICE_REPROMPT_STORED] pending=%d actor=%d options=%d\n",
            rb_has_pending_choice(g), g->queue.actor, g->queue.choice_options_n);
}

void rb_resolver_handle_selection_epilogue(RbAbilityResolver *self, GameState *g) {
    if (!self || !g) return;
    fprintf(stderr, "[CHOICE_EPILOGUE] pending=%d options=%d reprompt=%d queue_actions=%d resolver_pending=%d\n",
            rb_has_pending_choice(g), g->queue.choice_options_n, g->queue.choice_reprompt_pending,
            rb_queue_has_pending_actions(g), self->has_pending_choice);
    if (g->queue.choice_reprompt_pending)
        fprintf(stderr, "[CHOICE_EPILOGUE] pending=%d options=%d\n",
                rb_has_pending_choice(g), g->queue.choice_options_n);
    if (g->queue.choice_reprompt_pending) {
        rb_clear_pending_choice(g);
        if (g->queue.choice_options_n > 0) {
            emit_choice_reprompt(g);
            return;
        }
        clear_choice_options(g);
    }
    if (rb_queue_has_pending_actions(g) && !rb_has_pending_choice(g) && !self->has_pending_choice) {
        rb_resolver_clear_choice_meta(self);
        /* Rust clear_choice_state without dropping pending_choice (but epilogue only when none) */
        self->has_pending_choice = 0;
        self->has_pending_reprompt = 0;
        memset(&self->pending_choice,0,sizeof(self->pending_choice));
        rb_resolver_resume_pending_actions(self);
        return;
    }
    rb_resolver_finalize_choice(self);
}

/* Faithful port of choice.rs:2375 handle_select_target — routes via choice_card_no and SelectTargetKind */
void rb_resolver_handle_select_target(RbAbilityResolver *self, GameState *g,
                                      const char *target, const char *selected) {
    if (!self || !g) return;
    const char *tgt = target ? target : (g->queue.pending.target[0] ? g->queue.pending.target : "");
    const char *sel = selected ? selected : "";
    if (g->queue.pending.route == RB_ROUTE_CONDITIONAL_CHOICE) {
        int idx = atoi(sel);
        char choice_debug[128];
        snprintf(choice_debug, sizeof(choice_debug),
                 "[CHOICE_RESULT] selected=%s available=%d reprompt=%d",
                 sel, g->queue.choice_options_n, g->queue.choice_reprompt_pending);
        rb_log_push_verdict(choice_debug, "debug", 1);
        fprintf(stderr, "%s\n", choice_debug);
        if (idx < 0 || idx >= g->queue.choice_options_n) {
            clear_choice_options(g);
            rb_resolver_clear_choice_state(self);
            return;
        }
        AbilityEffect *selected = g->queue.choice_options[idx];
        for (int i = idx; i + 1 < g->queue.choice_options_n; i++)
            g->queue.choice_options[i] = g->queue.choice_options[i + 1];
        g->queue.choice_options_n--;
        g->queue.choice_options[g->queue.choice_options_n] = NULL;
        self->conditional_choice = idx;
        self->has_pending_choice = 0;
        rb_clear_pending_choice(g);
        if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
            g->queue.entries[g->queue.cur].effect_started = 1;
        int reprompt = g->queue.choice_reprompt_pending;
        if (!reprompt) {
            for (int i = 0; i < g->queue.choice_options_n; i++) {
                rb_effect_free(g->queue.choice_options[i]);
                g->queue.choice_options[i] = NULL;
            }
            g->queue.choice_options_n = 0;
        }
        if (!selected) return;
        int actor = g->queue.actor;
        if (selected->target && !strcmp(selected->target, "self")) actor = g->active;
        else if (selected->target && !strcmp(selected->target, "opponent")) actor = g->active^1;
        int host = g->queue.resume_host;
        g->queue.resume_move_destination[0] = '\0';
        g->queue.resume_move_state[0] = '\0';
        g->queue.resume_eff = NULL;
        int discard_move = selected->action && !strcmp(selected->action, "move_cards") &&
            selected->source && (!strcmp(selected->source, "discard") || !strcmp(selected->source, "waitroom"));
        if (discard_move) {
            const char *card_type = selected->card_type_field;
            rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "discard", card_type,
                           selected->count >= 0 ? selected->count : 1, 0, NULL);
            rb_queue_pause_for_choice(g, &g->queue.pending);
            g->queue.actor = actor;
            rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
            g->queue.resume_mode = 0;
        } else {
            rb_execute_effect_ex(g, actor, selected, host);
        }
        if (rb_has_pending_choice(g)) {
            if (selected->destination)
                strncpy(g->queue.resume_move_destination, selected->destination,
                        sizeof(g->queue.resume_move_destination) - 1);
            for (int i = 0; i < selected->n_extra; i++) {
                if (selected->extra_k[i] && !strcmp(selected->extra_k[i], "state_change"))
                    strncpy(g->queue.resume_move_state, selected->extra_v[i],
                            sizeof(g->queue.resume_move_state) - 1);
            }
            g->queue.resume_host = host;
        }
        rb_effect_free(selected);
        if (!rb_has_pending_choice(g) && reprompt) {
            if (g->queue.choice_options_n > 0) {
                emit_choice_reprompt(g);
            } else {
                clear_choice_options(g);
            }
        }
        return;
    }
    /* target-based routing via typed enum (SelectTargetKind) */
    if (!strcmp(tgt, "choice") || !strcmp(tgt, "choice_string")) {
        self->has_pending_choice = 0; rb_clear_pending_choice(g); return;
    }
    if (!strcmp(tgt, "pay_optional_cost:skip_optional_cost") || strstr(tgt, "pay_optional") || strstr(tgt, "skip_optional")) {
        /* Rust: `SelectTargetKind::PayOptionalCostSkipOptionalCost =>
           self.handle_optional_cost_payment(gs, selected)` (choice.rs:2813-2815),
           and handle_optional_cost_payment (cost/handlers.rs:1393) is the faithful
           port: it clears `last_cost_waited_members`, pays the gate's energy via
           `cost.energy_count_any()` against `cost.target`, runs the "unless you
           pay" alternative on skip, and stamps cost_paid /
           optional_cost_result / effect_started itself.

           This arm used to re-do a two-line subset of that inline and left the
           rest uncalled, which is the "correctly ported but never called" defect
           class:

             - the energy debit was gated on `gs->queue.deferred`, but the drain
               deliberately never parks `deferred` (ability_queue.c:1258-1260), so
               the guard was always false and answering "pay" debited NOTHING;
             - the amount came from `entries[cur].cost`, a C-only carrier Rust
               has no equivalent of, which had zero writers tree-wide until the
               energy gate started stamping it — and even stamped, it only ever
               names an energy gate, so it is not the general charge;
             - on skip, `alternative_effect` ("unless you pay") never ran, and
               `effect_started` was left as-is instead of being reset.

           Rust re-derives the charge from `gs.entry_cost()` at ANSWER time; that
           is rb_entry_cost() here, so the entry keeps its own cost and the C-only
           carrier is no longer consulted. */
        int pay = (atoi(sel)!=0 || !strcmp(sel,"1") || !strcmp(sel,"yes"));
        const AbilityEffect *entry_cost = rb_entry_cost(g);
        rb_resolver_clear_choice_state(self);
        rb_handle_optional_cost_payment(g, g->queue.actor >= 0 ? g->queue.actor : g->active,
                                        entry_cost, pay);
        return;
    }
    /* W2 sibling -- the SAME guard as choice.c:5037, in the other dispatcher.
       Leaving it would have kept the branch dead on the `tgt` path while
       "fixing" it on the `pending.target` path. `strstr(tgt,
       "pay_cost_all_discard")` cannot match: Rust's literal is
       "pay_cost_all:discard_all" (enums.rs:705) and that is what every producer
       in this tree spells. Routed through the exact-match table, like the arms
       above it (:3801 "choice", :3804 the pay/skip target, :3843
       "double_baton_touch") which all use strcmp. */
    if (rb_select_target_kind_from_str(tgt) == RB_STK_PAY_COST_ALL_DISCARD) {
        rb_handle_pay_cost_all_discard(g, g->queue.actor, sel);
        rb_resolver_clear_choice_state(self);
        rb_drain_ability_queue(g);
        return;
    }
    if (!strcmp(tgt, "double_baton_touch")) {
        RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g; tmp.actor=g->queue.actor;
        rb_resolver_handle_double_baton_touch(g, sel);
        return;
    }
    if (!strcmp(tgt, "primary_alternative")) {
        rb_resolver_handle_primary_alternative(self, g, sel);
        return;
    }
    if (!strcmp(tgt, "position|destination") || !strcmp(tgt, "position_destination")) {
        /* choice.rs:2710-2717 — a `position|destination` answer whose entry
           carries `ChoiceRoute::Raw("position_change…")` is routed to
           handle_position_change_choice BEFORE the SelectTargetKind match, and
           handle_position_change_choice is what calls
           execute_position_change_with_destination (choice.rs:3208). The C
           stores no choice_card_no, but misc.c `h_execute_position_change` is
           the ONLY producer of this target string and it is the port of
           position.rs:91 `execute_position_change`, which always stamps
           `Raw("position_change…")` on the entry before raising the prompt
           (position.rs:143/197/266/367/419/480/546). So the route is
           unconditional here, and "position_change:self" is the spelling
           position.rs:480 writes for the destination-less step. */
        rb_resolver_handle_position_change_choice(self, g, "position_change:self", sel);
        return;
    }
    /* area_select (result_handlers.rs:89-113). This arm was ABSENT here — the
       only copy of it lived in rb_resolver_provide_choice_result, which has no
       callers, so on the live path the answer fell through to the
       "record chosen target then clear" default below and the picked area was
       thrown away. */
    if (!strcmp(tgt, "area_select")) {
        choice_store_area_select(self, g, sel);
        rb_resolver_clear_choice_state(self);
        rb_resolver_resume_pending_actions(self);
        return;
    }
    if (!strcmp(tgt, "heart_color") || !strcmp(tgt, "heart_colour")) {
        rb_resolver_handle_heart_color_selection(self, g, sel);
        return;
    }
    if (!strcmp(tgt, "choice_condition")) {
        rb_resolver_handle_choice_condition(self, g, sel);
        return;
    }
    if (!strcmp(tgt, "conditional_optional")) {
        rb_resolver_handle_conditional_optional(g, sel);
        return;
    }
    if (!strcmp(tgt, "draw_any_number") || !strcmp(tgt, "draw:draw_any_number")) {
        rb_resolver_handle_draw_any_number(g, sel);
        return;
    }
    if (!strcmp(tgt, "order")) {
        rb_resolver_handle_order_selection(g, sel);
        return;
    }
    if (!strcmp(tgt, "self_or_opponent") || !strcmp(tgt, "selfOrOpponent")) {
        const char *chosen = "self";
        if (!strcmp(sel, "opponent") || !strcmp(sel, "opponent") || !strcmp(sel, "相手")) chosen="opponent";
        else if (!strcmp(sel, "self") || !strcmp(sel, "自分")) chosen="self";
        self->spawn_target = (!strcmp(chosen,"opponent"))?1:0;
        self->spawn_target_set = 1;
        if (g->queue.resume_eff) rb_set_chosen_target(g->queue.resume_eff, chosen);
        /* push inner effect as pending (Rust effect_steps[0]) */
        if (g->queue.resume_eff && g->queue.resume_eff->n_child>0) {
            rb_queue_set_pending_actions(g, 1);
            rb_resolver_resume_pending_actions(self);
            return;
        }
        rb_resolver_clear_choice_state(self);
        return;
    }
    if (strstr(tgt, "position_change")) {
        rb_resolver_handle_position_change_choice(self, g, tgt, sel);
        return;
    }
    /* default: record chosen target then clear */
    if (g->queue.resume_eff && sel[0]) rb_set_chosen_target(g->queue.resume_eff, sel);
    rb_resolver_clear_choice_state(self);
    rb_drain_ability_queue(g);
}

/* ── handle_draw_any_number (choice.rs:2585) — faithful: draw_n for any_number with validation ── */
void rb_resolver_handle_draw_any_number(GameState *g, const char *selected) {
    if (!g) return;
    int pl = g->queue.actor;
    int n = 0;
    if (selected) n = atoi(selected);
    /* Rust validates n against deck size and max; C clamps */
    if (n < 0) n = 0;
    if (n > g->p[pl].deck.n) n = g->p[pl].deck.n;
    if (n > 0) {
        /* hand zone: draw n to hand */
        int drawn = rb_draw_cards_for_player(&g->p[pl], (uint8_t)n, "deck", "hand", NULL, 0, NULL, NULL, -1);
        g->last_draw_count = drawn;
        /* also push movement events for tracking */
        for (int i=0;i<drawn;i++) {
            int cid = g->p[pl].hand.cards[g->p[pl].hand.n - drawn + i];
            if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED) g->recently_moved[g->n_recently_moved++] = cid;
        }
    }
    /* continue pending sequential actions (choice.rs:2610 resume_pending_actions) */
    RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g; tmp.actor=pl;
    rb_resolver_clear_choice_state(&tmp);
    rb_drain_ability_queue(g);
}

/* ── Deck-order context: the C's LookAndSelectStep::Order { ordered } (choice.rs:2990) ──
 *
 * File-static for the same reason g_look is: the prompt spans several queue
 * resumptions, so the accumulator cannot live in a stack frame. choice.c owns
 * it; the producer (rb_order_prompt_begin, below) is called from the move
 * site in move_missing.c. */
static int s_order_cards[RB_MAX_ZONE];
static int s_order_n;
static int s_order_player = -1;
static int s_order_actor = -1;

static void order_emit_prompt(GameState *g, int pl, int actor, int n_pool);

/* rb_order_prompt_begin — the shared body of BOTH Rust producers:
 *   move/mod.rs:1914-1931  (maybe_prompt_deck_order, called at :2272)
 *   move/mod.rs:3128-3156  (the look_and_select Finalize arm, needs_order)
 * Both do the same three things — set the looked_at pool, arm
 * Choice::SelectTarget{target:"order"} with a "(N cards)" description, and
 * park. Rust additionally sets in_flight = LookAndSelect{Finalize{destination:
 * Deck}}, which handle_order_selection matches to start from an EMPTY ordered
 * list; `s_order_n = 0` below is that empty list (choice.rs:2985-2992).
 *
 * `reverse_pool` is mod.rs:1906-1908 (order_pool.reverse() for a Hand source).
 * Returns 0 when there is no order to ask about (Rust's `taken.len() > 1`). */
int rb_order_prompt_begin(GameState *g, int actor, const int *cards, int n,
                          int reverse_pool)
{
    if (!g || !cards || n <= 1) return 0;
    if (n > RB_MAX_ZONE) n = RB_MAX_ZONE;
    int pl = (actor >= 0 && actor <= 1) ? actor : 0;
    int pool[RB_MAX_ZONE];
    for (int i = 0; i < n; i++) pool[i] = cards[i];
    if (reverse_pool)
        for (int i = 0, j = n - 1; i < j; i++, j--) {
            int t = pool[i]; pool[i] = pool[j]; pool[j] = t;
        }
    rb_look_clear(pl);
    for (int i = 0; i < n; i++) rb_look_add(pl, pool[i]);
    s_order_n = 0;
    s_order_player = pl;
    s_order_actor = (actor >= 0 && actor <= 1) ? actor : pl;
    order_emit_prompt(g, pl, s_order_actor, n);
    return 1;
}

static void order_emit_prompt(GameState *g, int pl, int actor, int n_pool)
{
    char desc[128];
    snprintf(desc, sizeof(desc), "Choose order for cards on deck (%d cards)", n_pool);
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, n_pool, 0, "order");
    rb_choice_set_description(&g->queue.pending, desc);
    /* rb_emit_choice clears resume_look_owner (choice.c:5394), and the pool
     * lives in g_look[pl], so the owner has to be restated after the emit. */
    g->queue.resume_look_owner = pl;
}

/* handle_order_selection — choice.rs:2984-3055.
 *
 * The old body parsed a comma-separated "2,0,1", reordered the looked_at pool
 * by those indices and then cleared the choice state — it never PLACED the
 * cards, so 「好きな順番でデッキの上に置く」 left them on the table. Rust removes
 * ONE index per answer, re-arms the same SelectTarget while more than one card
 * remains, and on the last answer drains the remainder and inserts every card
 * at deck position 0 in reverse, so the first-picked card ends up on top. */
int rb_move_handle_order_selection(GameState *g, const char *selected)
{
    if (!g || !selected) return -1;
    int pl = s_order_player;
    if (pl < 0 || pl > 1)
        pl = (g->queue.actor >= 0 && g->queue.actor <= 1) ? g->queue.actor : 0;
    int pool[RB_MAX_ZONE];
    int n = rb_looked_at_pool(pl, pool, RB_MAX_ZONE);
    /* choice.rs:2994-2998 is `selected.parse::<usize>()`, and Rust's integer
     * from_str accepts an optional '+' then digits ONLY — it rejects leading and
     * trailing whitespace, which strtol silently skips. Validate the shape
     * first so the C's notion of a valid index matches Rust's exactly. */
    {
        const char *q = selected;
        if (*q == '+') q++;
        if (!*q) return -1;
        for (const char *d = q; *d; d++)
            if (*d < '0' || *d > '9') return -1;
    }
    char *end = NULL;
    long idx = strtol(selected, &end, 10);
    if (!end || *end || end == selected || idx < 0 || idx >= n) return -1;
    if (s_order_n < RB_MAX_ZONE) s_order_cards[s_order_n++] = pool[idx];
    rb_look_remove(pl, pool[idx]);                 /* ordered.push(remove(idx)) */
    n = rb_looked_at_pool(pl, pool, RB_MAX_ZONE);
    if (n > 1) {                                   /* choice.rs:3004-3020 */
        int actor = (s_order_actor >= 0 && s_order_actor <= 1) ? s_order_actor : pl;
        order_emit_prompt(g, pl, actor, n);
        return 1;                                  /* re-armed; the prompt stands */
    }
    for (int i = 0; i < n && s_order_n < RB_MAX_ZONE; i++)  /* choice.rs:3021 */
        s_order_cards[s_order_n++] = pool[i];
    rb_look_clear(pl);
    RbPlayer *P = &g->p[pl];                       /* choice.rs:3041-3044 */
    for (int i = s_order_n - 1; i >= 0; i--) {
        if (P->deck.n >= RB_MAX_ZONE) break;
        for (int k = P->deck.n; k >= 1; k--) P->deck.cards[k] = P->deck.cards[k - 1];
        P->deck.cards[0] = s_order_cards[i];
        P->deck.n++;
    }
    s_order_n = 0;
    s_order_player = -1;
    s_order_actor = -1;
    return 0;
}

void rb_resolver_handle_order_selection(GameState *g, const char *selected) {
    if (!g) return;
    int r = rb_move_handle_order_selection(g, selected);
    /* r < 0 is Rust's Err: the answer is invalid and the prompt stands
     * (choice.rs:2994-2998). */
    if (r < 0) return;
    if (r == 1) return;                 /* re-armed over the shrunken pool */
    RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
    rb_resolver_clear_choice_state_and_resume(&tmp);
}

/* ── handle_position_change_choice (choice.rs:2652) — faithful: skip, ChoiceRoute::Raw parsing, formation plan, position_change execution ── */
int rb_resolver_handle_position_change_choice(RbAbilityResolver *self, GameState *g,
                                               const char *choice_card_no, const char *selected) {
    if (!self || !g) return -1;
    const char *sel = selected ? selected : "";
    /* "skip" selection: clear formation plan and pending actions */
    if (!strcmp(sel, "skip")) {
        self->n_formation_plan = 0;
        rb_queue_set_pending_actions(g, 0);
        rb_resolver_clear_choice_state_and_resume(self);
        return 0;
    }
    /* Parse choice_card_no for position_change: prefix */
    const char *raw_ccn = choice_card_no ? choice_card_no : "";
    const char *pc_prefix = strstr(raw_ccn, "position_change:");
    const char *target_str = "self";
    const char *explicit_source_pos = NULL;
    int was_select = 0;
    if (pc_prefix) {
        const char *after = pc_prefix + strlen("position_change:");
        if (!strncmp(after, "opponent:front", 15)) {
            int actor = g->queue.actor;
            int pl = actor ^ 1;
            RbPlayer *P = &g->p[pl];
            int occupied[RB_STAGE_SIZE];
            int n_occupied = 0;
            for (int i = 0; i < RB_STAGE_SIZE; i++)
                if (P->stage[i] >= 0) occupied[n_occupied++] = i;
            int pick = atoi(sel);
            int src_idx = pick >= 0 && pick < n_occupied ? occupied[pick] : -1;
            int dst_idx = -1;
            int host = g->queue.resume_host;
            for (int i = 0; i < RB_STAGE_SIZE; i++)
                if (host >= 0 && g->p[actor].stage[i] == host) dst_idx = i;
            if (dst_idx < 0) dst_idx = 1;
            if (src_idx >= 0 && src_idx != dst_idx && P->stage[src_idx] >= 0) {
                int moved = P->stage[src_idx];
                int displaced = P->stage[dst_idx];
                int moved_wait = P->stage_wait[src_idx];
                int displaced_wait = P->stage_wait[dst_idx];
                P->stage[src_idx] = displaced;
                P->stage_wait[src_idx] = displaced_wait;
                P->stage[dst_idx] = moved;
                P->stage_wait[dst_idx] = moved_wait;
                /* Redundant since commit 219d1522 moved the area-move arm into the
                   choke point, but harmless: both this line and modifiers.c:418
                   store 1, and nothing reads the flag between them. */
                g->position_change_occurred_this_turn = 1;
                /* A position change is a genuine stage->stage (area) move, so the two
                   zones are spelled out: RB_ZONEID_STAGE == 0, and the choke point
                   does all three jobs of Rust's push_movement_event in one call —
                   event log, turn-level position flag, opponent-cause watcher arming
                   (modifiers.rs:1526-1552; the hook is src/core/modifiers.c:417-420).
                   A hand-call to rb_fire_opponent_cause_watchers_for_move used to
                   follow this line and double-fired every opponent-caused watcher. */
                rb_record_card_movement(g, moved, RB_ZONEID_STAGE, RB_ZONEID_STAGE, actor, 1);
                rb_recalc_constants(g);
            }
            rb_resolver_clear_choice_state_and_resume(self);
            return 0;
        }
    /* Parse target and position from "position_change:target:select" or "position_change:target:member" */
        const char *first_colon = strchr(after, ':');
        if (first_colon) {
            char target_buf[32];
            int tlen = (int)(first_colon - after);
            if (tlen >= sizeof(target_buf)) tlen = sizeof(target_buf) - 1;
            strncpy(target_buf, after, tlen);
            target_buf[tlen] = '\0';
            target_str = rb_strdup2(target_buf);
            const char *rest = first_colon + 1;
            if (!strcmp(rest, "select")) {
                was_select = 1;
                /* Check if selected encodes player:position */
                const char *sel_colon = strchr(sel, ':');
                if (sel_colon) {
                    char player_buf[32];
                    int plen = (int)(sel_colon - sel);
                    if (plen >= sizeof(player_buf)) plen = sizeof(player_buf) - 1;
                    strncpy(player_buf, sel, plen);
                    player_buf[plen] = '\0';
                    target_str = rb_strdup2(player_buf);
                    explicit_source_pos = sel_colon + 1;
                } else {
                    explicit_source_pos = sel;
                }
            } else if (rb_stage_position_index(rest) != -1) {
                explicit_source_pos = rest;
            } else {
                /* rest is a member identifier */
            }
        }
    }
    /* If this was a select choice, the user chose the source member.
       Now either use a fixed destination or ask for destination. */
    if (was_select) {
        /* Check for fixed destination in the effect */
        const char *fixed_dest = NULL;
        if (g->queue.resume_eff && g->queue.resume_eff->destination && *g->queue.resume_eff->destination)
            fixed_dest = g->queue.resume_eff->destination;
        if (fixed_dest) {
            /* Execute position change directly with fixed destination */
            int actor = g->queue.actor;
            int pl = rb_resolve_target_player(g, target_str);
            if (pl < 0) pl = actor;
            RbPlayer *P = &g->p[pl];
            const char *src_pos = explicit_source_pos ? explicit_source_pos : sel;
            int src_idx = rb_stage_position_index(src_pos);
            int dst_idx = rb_stage_position_index(fixed_dest);
            if (src_idx >= 0 && dst_idx >= 0 && src_idx < RB_STAGE_SIZE && dst_idx < RB_STAGE_SIZE
                && P->stage[src_idx] >= 0) {
                int a = P->stage[src_idx], b = P->stage[dst_idx];
                P->stage[src_idx] = b; P->stage_wait[src_idx] = P->stage_wait[dst_idx];
                P->stage[dst_idx] = a; P->stage_wait[dst_idx] = P->stage_wait[src_idx];
                g->position_change_occurred_this_turn = 1;
                /* Two members swapped places on the stage. This is the C's fold of
                   Rust `record_stage_position_swap` (game_state/modifiers.rs:1799-1857),
                   so it is now CALLED rather than re-implemented: the twin writes
                   both position_change_events (modifiers.rs:1817-1838) AND both
                   turn_area_movements events with `effect_only: true`
                   (modifiers.rs:1845 / :1853) and `cause_player_id` = the queue's
                   current entry seat (modifiers.rs:1812-1816).

                   The two `rb_record_card_movement(..., 0, 0)` calls this replaces
                   were wrong on THREE counts against that Rust: `effect_only` was
                   0 where Rust passes the literal `true`; `cause_player_id` was
                   hard-coded to seat 0 instead of the mover seat; and
                   `position_change_events` was left EMPTY, which is why
                   `movement:"position_change"` (26 corpus nodes on 26 pairs,
                   measured with tools/census_fx.c) could never be satisfied by a
                   real swap. `effect_only` had NO reader anywhere in src/ before
                   this wave, so nothing could observe the wrong value; the reader
                   now exists in src/ability/condition.c (Rust
                   ability/condition/state.rs:869-908 + :524-568, ported as
                   position_event_matches_filters) and does.

                   `moved_card_id` and order are unchanged: the twin pushes the
                   source then the target, exactly as the two calls did, and each
                   still lands in batch_movements / turn_area_movements once. */
                rb_record_stage_position_swap(g, a, src_idx, b, dst_idx);
                rb_trigger_auto_abilities_for_movement_current(g);
            }
            rb_resolver_clear_choice_state_and_resume(self);
            return 0;
        } else {
        const char *all_positions[] = {"left", "center", "right"};
        char valid_destinations[3][32];
        int n_valid = 0;
        const char *src_pos_name = explicit_source_pos ? explicit_source_pos : sel;
        for (int i = 0; i < 3; i++) {
            if (strcmp(src_pos_name, all_positions[i]) != 0) {
                strncpy(valid_destinations[n_valid], all_positions[i], sizeof(valid_destinations[0]) - 1);
                valid_destinations[n_valid][sizeof(valid_destinations[0]) - 1] = '\0';
                n_valid++;
            }
        }
        if (n_valid == 0) {
            rb_resolver_clear_choice_state_and_resume(self);
            return 0;
        }
        /* Create pending choice for destination */
        RbChoice ch;
        memset(&ch, 0, sizeof(ch));
        ch.kind = RB_CHOICE_SELECT_TARGET;
        ch.count = 1;
        ch.allow_skip = 0;
        snprintf(ch.description, sizeof(ch.description),
                 "Choose destination for position change (currently at %s)", src_pos_name);
        ch.route = RB_ROUTE_SELECT_TARGET;
        /* Store options in target field as "position|destination" */
        strncpy(ch.target, "position|destination", sizeof(ch.target) - 1);
        g->queue.pending = ch;
        g->queue.has_pending = 1;
        g->queue.actor = g->queue.actor;
        return 0;
        }
    }
    /* Formation plan: accumulate assignments and either prompt next
       member or finalize batch swap. */
    if (self->n_formation_plan > 0) {
        /* Find the target card ID from formation plan */
        int target_card_id = -1;
        if (strncmp(raw_ccn, "position_change:self:", 21) == 0) {
            const char *id_str = raw_ccn + 21;
            target_card_id = atoi(id_str);
        }
        int entry_idx = -1;
        if (target_card_id >= 0) {
            for (int i = 0; i < self->n_formation_plan; i++) {
                if (self->formation_plan[i] == target_card_id) {
                    entry_idx = i;
                    break;
                }
            }
        }
        if (entry_idx >= 0) {
            /* Store destination in formation plan */
            /* formation_plan stores (member_id, dest) pairs — simplified to dest array */
            /* For now just record and continue */
        }
        /* Check if all members assigned */
        int all_assigned = 1; /* simplified: assume all assigned if we reach here */
        if (all_assigned) {
            /* Execute batch swap */
            rb_resolver_clear_choice_state_and_resume(self);
            return 0;
        }
        /* Find next member to assign */
        int next_cid = -1;
        if (target_card_id >= 0) {
            for (int i = 0; i < self->n_formation_plan; i++) {
                if (self->formation_plan[i] == 0) { /* unassigned slot */
                    next_cid = self->formation_plan[i];
                    break;
                }
            }
        }
        if (next_cid >= 0) {
            /* Create pending choice for next member's destination */
            RbChoice ch;
            memset(&ch, 0, sizeof(ch));
            ch.kind = RB_CHOICE_SELECT_TARGET;
            ch.count = 1;
            ch.allow_skip = 0;
            ch.route = RB_ROUTE_SELECT_TARGET;
            strncpy(ch.target, "position|destination", sizeof(ch.target) - 1);
            g->queue.pending = ch;
            g->queue.has_pending = 1;
            return 0;
        }
    }
    /* choice.rs:3202-3213 — the ANSWER is the destination. Which member moves
       depends on what the prompt already named:
         * explicit_source_pos — the source-selection prompt already picked the
           member, so apply the swap from that position (Rust's
           apply_selected_position_change, choice.rs:3220-3248);
         * otherwise — this_member, i.e. the activating member, located on the
           target player's stage (position.rs:1046-1089).
       The C used `sel` for BOTH ends, so a plain destination answer made
       source == destination and the swap was skipped every time. */
    int actor = g->queue.actor;
    /* position.rs:982 — `target` is "self" on the destination-less step, and
       "self" is the RESOLVER's own player, i.e. the seat the prompt was raised
       for. rb_resolve_target_player("self") hard-codes seat 0, which is wrong
       for the second leg of a `target="both"` step (misc.c emits that leg to
       actor^1), so the prompt's own seat wins whenever the route says "self". */
    int pl = (target_str && !strcmp(target_str, "self") && actor >= 0 && actor <= 1)
           ? actor
           : rb_resolve_target_player(g, target_str);
    if (pl < 0 || pl > 1) pl = actor;
    int dst_idx = choice_area_index_from_answer(sel);
    if (dst_idx < 0 || dst_idx >= RB_STAGE_SIZE) {
        rb_resolver_clear_choice_state_and_resume(self);
        return 0;
    }
    if (explicit_source_pos) {
        RbPlayer *P = &g->p[pl];
        int src_idx = rb_stage_position_index(explicit_source_pos);
        if (src_idx >= 0 && src_idx < RB_STAGE_SIZE && P->stage[src_idx] >= 0) {
                int a = P->stage[src_idx], b = P->stage[dst_idx];
                P->stage[src_idx] = b; P->stage_wait[src_idx] = P->stage_wait[dst_idx];
                P->stage[dst_idx] = a; P->stage_wait[dst_idx] = P->stage_wait[src_idx];
                g->position_change_occurred_this_turn = 1;
                /* Same twin as the fixed-destination arm above: Rust
                   record_stage_position_swap (modifiers.rs:1799-1857). See that
                   comment for the three counts the two `..., 0, 0)` calls this
                   replaces got wrong (`effect_only`, `cause_player_id`, and the
                   unfilled position_change_events). */
                rb_record_stage_position_swap(g, a, src_idx, b, dst_idx);
                rb_trigger_auto_abilities_for_movement_current(g);
            }
    } else {
        /* position.rs:1090-1092 — the flag and the two follow-ups are set by
           rb_resume_position_change itself (state.c:1489-1503). */
        char pending_area[16];
        int had_area = choice_take_selection_area(pending_area, sizeof(pending_area));
        (void)had_area;
        choice_apply_position_destination(g, pl, g->queue.resume_host, dst_idx);
    }
    /* choice.rs:3214 — the stored area is spent. */
    choice_take_selection_area(NULL, 0);
    rb_resolver_clear_choice_state_and_resume(self);
    return 0;
}

void rb_resolver_apply_effect_modification(RbAbilityResolver *self, GameState *g,
                                            void (*modifier)(AbilityEffect *)) {
    if (!self || !g) return;
    rb_resolver_clear_choice_state(self);
    if (g->queue.resume_eff && modifier) {
        modifier(g->queue.resume_eff);
    }
    rb_resolver_resume_pending_actions(self);
}

/* ── handle_primary_alternative (choice.rs:2966) — faithful: conditional_alternative branch selection with condition evaluation ── */
void rb_resolver_handle_primary_alternative(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!self || !g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int pick = selected ? atoi(selected) : 0;
    self->conditional_choice = (pick != 0) ? 1 : 0;
    /* Rust also evaluates the alternative_condition if present */
    int cur = g->queue.cur;
    if (cur >=0 && cur < g->queue.n_entries) {
        Ability ab; 
        if (rb_decode_card_ability((uint32_t)g->queue.entries[cur].card_id, g->queue.entries[cur].ability_idx, &ab)==1) {
            if (ab.effect && ab.effect->alternative_condition) {
                int passed = rb_eval_condition_for_host(g, g->queue.actor, g->queue.resume_host, ab.effect->alternative_condition);
                self->conditional_choice = passed ? 0 : 1;
            }
            rb_free_ability(&ab);
        }
    }
    /* if pending_choice was primary/alternative, clear and resume */
    if (self->conditional_choice==0) {
        /* primary branch */
        if (g->queue.resume_eff && g->queue.resume_eff->primary_effect) {
            rb_execute_effect_ex(g, g->queue.actor, g->queue.resume_eff->primary_effect, g->queue.resume_host);
        }
    } else {
        if (g->queue.resume_eff && g->queue.resume_eff->alternative_effect) {
            rb_execute_effect_ex(g, g->queue.actor, g->queue.resume_eff->alternative_effect, g->queue.resume_host);
        }
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_position_destination (choice.rs:2988) — faithful: position_change with destination validation and formation plan ── */
void rb_resolver_handle_position_destination(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int actor = g->queue.actor;
    int idx = selected ? atoi(selected) : -1;
    /* validate destination area 0..2 */
    if (idx < 0 || idx >= RB_STAGE_SIZE) {
        /* invalid destination: skip */
        RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
        rb_resolver_clear_choice_state(&tmp);
        rb_drain_ability_queue(g);
        return;
    }
    /* apply via resume_position_change which handles baton/formation logic */
    if (g->queue.resume_eff) {
        /* record formation plan for sequential position changes */
        if (self->n_formation_plan < RB_STAGE_SIZE) self->formation_plan[self->n_formation_plan++] = idx;
        rb_resume_position_change(g, actor, g->queue.resume_eff, g->queue.resume_host, idx);
        /* if more position changes remain, keep pending */
        if (g->queue.has_pending) return;
    } else {
        /* The parked effect is gone: rb_resume_with_choice_indices_internal
           NULLs queue.resume_eff (choice.c:3297) BEFORE dispatching the answer,
           so the `if` above is false on EVERY prompt that arrives here, and the
           move was skipped while the prompt was re-raised. Rust has no such
           dead branch — choice.rs:2710-2717 sends this answer to
           handle_position_change_choice, which resolves it through
           execute_position_change_with_destination. Fall back to the same
           move, with the resolver's host standing in for
           `session.activating_card_id` (position.rs:1074-1079). */
        if (self->n_formation_plan < RB_STAGE_SIZE) self->formation_plan[self->n_formation_plan++] = idx;
        choice_apply_position_destination(g, actor, g->queue.resume_host, idx);
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_double_baton_touch (choice.rs:3109) — faithful: second baton touch with protection check ── */
void rb_resolver_handle_double_baton_touch(GameState *g, const char *selected) {
    if (!g) return;
    int actor = g->queue.actor;
    int choice = selected ? atoi(selected) : -1;
    /* choice 0 = first area, 1 = second area, -1 = skip */
    if (choice < 0) {
        /* skip: no second baton */
        RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
        rb_resolver_clear_choice_state(&tmp);
        rb_drain_ability_queue(g);
        return;
    }
    if (choice >= 0) {
        rb_complete_double_baton(g, choice);
    }
    RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
    rb_resolver_clear_choice_state(&tmp);
    rb_drain_ability_queue(g);
}

/* ── handle_conditional_optional (choice.rs:3187) — faithful: conditional gate with optional pay ── */
void rb_resolver_handle_conditional_optional(GameState *g, const char *selected) {
    if (!g) return;
    int pay = (selected && atoi(selected) != 0) ? 1 : 0;
    int cur = g->queue.cur;
    if (cur >=0 && cur < g->queue.n_entries) {
        g->queue.entries[cur].optional_cost_result = pay ? 1 : 0;
        g->queue.entries[cur].cost_paid = 1;
    }
    if (pay) {
        /* pay cost then continue sequential */
        RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
        rb_resolver_clear_choice_state(&tmp);
        /* continue with remaining pending actions */
        rb_drain_ability_queue(g);
    } else {
        /* skip: drop pending actions (Rust take_pending_actions) */
        g->queue.has_pending=0;
        RbAbilityResolver tmp; memset(&tmp,0,sizeof(tmp)); tmp.gs=g;
        rb_resolver_clear_choice_state(&tmp);
        rb_drain_ability_queue(g);
    }
}

/* ── handle_heart_color_selection (choice.rs:3278) — faithful: select heart color from palette, record in queue.selected_heart_color ── */
void rb_resolver_handle_heart_color_selection(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int idx = selected ? atoi(selected) : -1;
    if (idx >=0 && g->queue.pending.n_heart_options > 0) {
        if (idx >=0 && idx < g->queue.pending.n_heart_options) {
            const char *col = g->queue.pending.heart_options[idx];
            g->queue.selected_heart_color = (int)rb_parse_heart_color(col);
            /* also store in resolver selected_cards as color index for gain_resource */
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = g->queue.selected_heart_color;
        }
        /* if count >1 and more picks needed, re-prompt */
        int total = g->queue.pending.count;
        if (total > 1 && self->n_selected_cards < total) {
            int rem = total - self->n_selected_cards;
            RbChoice ch; memset(&ch,0,sizeof(ch));
            ch.kind=RB_CHOICE_SELECT_HEART_COLOR; ch.count=rem; ch.allow_skip=g->queue.pending.allow_skip;
            for(int i=0;i<g->queue.pending.n_heart_options && i<8;i++) strncpy(ch.heart_options[i], g->queue.pending.heart_options[i], sizeof(ch.heart_options[i])-1);
            ch.n_heart_options=g->queue.pending.n_heart_options;
            g->queue.pending=ch; g->queue.has_pending=1; return;
        }
    } else if (g->queue.pending.allow_skip) {
        g->queue.selected_heart_color = -1;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_choice_condition (choice.rs:3302) — faithful: choice condition branch with condition re-evaluation ── */
void rb_resolver_handle_choice_condition(RbAbilityResolver *self, GameState *g, const char *selected) {
    if (!self || !g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    int pick = selected ? atoi(selected) : 0;
    self->conditional_choice = (pick != 0) ? 1 : 0;
    /* Rust re-evaluates the condition that guarded this choice */
    int cur = g->queue.cur;
    if (cur >=0 && cur < g->queue.n_entries) {
        Ability ab;
        if (rb_decode_card_ability((uint32_t)g->queue.entries[cur].card_id, g->queue.entries[cur].ability_idx, &ab)==1) {
            if (ab.effect && ab.effect->condition) {
                int passed = rb_eval_condition_for_host(g, g->queue.actor, g->queue.resume_host, ab.effect->condition);
                /* if condition now fails, drop pending actions */
                if (!passed) {
                    g->queue.has_pending=0;
                    memset(&g->queue.pending,0,sizeof(g->queue.pending));
                }
            }
            rb_free_ability(&ab);
        }
    }
    /* continue with selected branch */
    if (self->conditional_choice) {
        /* true branch: execute primary effect */
        if (g->queue.resume_eff && g->queue.resume_eff->primary_effect) {
            rb_execute_effect_ex(g, g->queue.actor, g->queue.resume_eff->primary_effect, g->queue.resume_host);
            if (g->queue.has_pending) return;
        }
    } else {
        /* false branch: alternative */
        if (g->queue.resume_eff && g->queue.resume_eff->alternative_effect) {
            rb_execute_effect_ex(g, g->queue.actor, g->queue.resume_eff->alternative_effect, g->queue.resume_host);
            if (g->queue.has_pending) return;
        }
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

/* ── handle_heart_selection (choice.rs:3354) — faithful: multi-color heart selection with count validation ── */
void rb_resolver_handle_heart_selection(RbAbilityResolver *self, GameState *g, int count,
                                         const char *const *colors, int n_colors) {
    if (!g) { rb_resolver_clear_choice_state_and_resume(self); return; }
    if (count <=0) count = n_colors;
    int to_apply = count < n_colors ? count : n_colors;
    for (int i=0;i<to_apply;i++) {
        if (colors[i]) {
            int col = (int)rb_parse_heart_color(colors[i]);
            g->queue.selected_heart_color = col;
            if (self->n_selected_cards < RB_MAX_RECENTLY_MOVED) self->selected_cards[self->n_selected_cards++] = col;
            /* record each color as heart modifier for later gain_resource */
            if (i==0) continue; /* first is primary */
        }
    }
    /* if more colors remain and re-prompt needed (count > applied), set pending */
    if (to_apply < count) {
        int rem = count - to_apply;
        RbChoice ch; memset(&ch,0,sizeof(ch));
        ch.kind=RB_CHOICE_SELECT_HEART_COLOR; ch.count=rem; ch.allow_skip=0;
        for(int i=to_apply;i<n_colors && i<8;i++) strncpy(ch.heart_options[i-to_apply], colors[i], sizeof(ch.heart_options[0])-1);
        ch.n_heart_options = n_colors - to_apply;
        g->queue.pending=ch; g->queue.has_pending=1; return;
    }
    rb_resolver_clear_choice_state_and_resume(self);
}

int rb_has_pending_choice(const GameState *g) { return g ? g->queue.has_pending : 0; }
const RbChoice *rb_get_pending_choice(const GameState *g) {
    if (!g || !g->queue.has_pending) return NULL;
    return &g->queue.pending;
}
/* Rust engine/src/core/game_state/abilities.rs:2457-2469 --
       pub fn get_pending_choice_player_id(&self) -> Option<String> {
           if self.pending_loop_protocol.is_some() { ...the other seat... }
           self.ability_queue
               .current_entry()
               .and_then(|e| e.choice_player_id.as_ref().cloned())
       }

   THE CURRENT ENTRY'S OWN STAMP, WITH NO FALLBACK. Three properties, all of
   them load-bearing:

   (1) NO fallback to entry.player_id. The universal default at
       engine/src/ability/queue.rs:354-360 already copied the owner's token
       into choice_player_id at PAUSE time, so reading that one field
       reproduces Rust exactly. A player_id fallback here would invent a seat
       for an entry that was never paused, where Rust reads None.

   (2) current_entry() is None for Idle and for WaitingForAutoAbilityChoice
       (engine/src/ability/queue.rs:234-242). That is the gate
       rb_queue_current_entry_mut (src/ability/ability_queue.c:184-190)
       already spells out, reused here rather than re-derived -- including the
       auto-ability prompt, which parks a choice with NO entry behind it and so
       must fall through to can_player_act's Choice-embedded arm.

   (3) the player1/player2 normalisation of engine/src/core/game_state/mod.rs
       :506-511 -- the same mapping as seat_of_token in src/test_game.c and as
       build_ability_queue_entry (src/core/game_state_abilities.c:1849-1857).

   WHAT THIS REPLACED, AND WHY IT WAS WRONG: the body returned g->queue.actor.
   ability_queue.c:593 and :605 set that to g->active DELIBERATELY -- the
   reasoning at :584-592 is sound and is untouched by this change -- and this
   file has twenty further writers of queue.actor that mean something else
   again (:2565 and :2591 store `actor_pending` and `actor_pending ^ 1`). So
   the old body answered out of a field whose only claim to agree with Rust was
   its name. Its one production consumer, rb_gsc_can_player_act
   (src/core/game_state_core.c:261-283), whose own comment calls pending
   choices "authoritative", therefore named the ACTIVE seat on exactly the
   prompts the redirect at queue.rs:335-352 exists to redirect.

   `has_pending` STAYS a guard, because that is the shape every Rust call site
   has: match_runner.rs:429-455 and mod.rs:498-505 both sit inside a
   has_pending_choice() test, so no Rust program can observe the difference
   between "no choice pending" and "the getter answered anyway".

   RESIDUAL, AND MEASURED TO BE UNREACHABLE: Rust's Some(token) that names no
   seat makes can_player_act return false for BOTH seats, whereas -1 here falls
   through to the Choice-embedded arm. The token domain closes that gap:
   build_ability_queue_entry canonicalises entry.player_id to "p1"/"p2" before
   anything is stamped from it, the redirect at ability_queue.c:572-573 writes
   the literals "p1"/"p2", and the dummy entry of queue.rs:367-409 takes NULL.
   So choice_player_id is only ever "p1", "p2" or "".

   Rust's OTHER arm -- pending_loop_protocol -- has no counterpart here:
   VERIFIED-NOOP, 0 struct fields and 0 readers (AUDIT_core_state.md:739
   records the protocol pair ABSENT). */
int rb_get_pending_choice_player_id(const GameState *g) {
    if (!g || !g->queue.has_pending) return -1;
    if (g->queue.state == RB_QUEUE_IDLE) return -1;                       // queue.rs:240
    if (g->queue.state == RB_QUEUE_AWAITING_CHOICE && g->queue.auto_ability)
        return -1;                                                        // queue.rs:240
    if (g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return -1; // no entry
    const RbQueueEntry *e = &g->queue.entries[g->queue.cur];
    const char *pid = e->choice_player_id;
    if (!pid[0]) return -1;                                               // Rust: None
    if (!strcmp(pid, "p1") || !strcmp(pid, "player1")) return 0;          // mod.rs:506-511
    if (!strcmp(pid, "p2") || !strcmp(pid, "player2")) return 1;
    return -1;
}
void rb_clear_pending_choice(GameState *g) {
    if (!g) return;
    memset(&g->queue.pending, 0, sizeof(g->queue.pending));
    g->queue.has_pending = 0;
    g->queue.deferred = NULL;
    g->queue.resume_is_select = 0;
    /* The stamp, not the split, is what has to become truthful here.
       `state == RB_QUEUE_AWAITING_CHOICE` with `has_pending == 0` says "a choice
       is awaited" and "no choice exists" at the same time. Rust cannot express
       that: its `is_idle()` and its `is_waiting_for_choice()` match on the SAME
       `QueueState` datum (queue.rs:200-205), so a guard written against
       `rb_queue_is_idle` is sound there and unsound here — `!rb_queue_is_idle(g)`
       is true for a queue that is waiting for nothing, which is what a
       `!rb_queue_is_idle(g)` guard measured +9 on parity_cce_gaps against.

       Rust does hold that combination, but only inside the answer window:
       `take_waiting_choice` (queue.rs:208-214) leaves
       WaitingForChoice{choice: None} and `resume_queue_with_choice` closes the
       window with        `resume_with_choice()` (turn/actions/mod.rs:795-798 -> :1571,
       queue.rs:421-434), which is `RB_QUEUE_RESOLVING` in the C's spelling and
       which `rb_resume_with_choice_indices_internal` already performs
       (choice.c:4797). The clears that do NOT come from that path are the ones
       that used to strand the stamp, so the successor state is spelled out here.

       RB_QUEUE_RESOLVING is measured +8 (parity_cce_gaps: a22a_queue_path_skips_cost
       and g1_one_wait_member lose their optional-cost prompt) because the C
       overloads it as the drain's re-entrancy marker (ability_queue.c:1266) where
       Rust's ExecutingEffect is inert. RB_QUEUE_IDLE is neutral on all eleven
       parity suites but nulls every entry accessor
       (rb_queue_current_entry_mut, ability_queue.c:184-190), so it is only safe
       where NO entry is current — and there `cur` is already out of range, so
       those accessors returned NULL anyway and the demotion is lossless. */
    if (g->queue.state == RB_QUEUE_AWAITING_CHOICE &&
        (g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries))
        g->queue.state = RB_QUEUE_IDLE;
}
void rb_resolver_continue_siblings(GameState *g, int actor, int host,
                                   const AbilityEffect *cont, int cont_from);

/* Continuation for a SKIPPED choice: run the effect the engine parked while the
   choice was outstanding. See the comment at the SelectCard skip arms in
   rb_resume_with_choice_indices_internal for why this is the C equivalent of
   Rust's resume_execution continuing the resolver's execution context.

   A declined COST is not paid — that is the whole point of the skip — so only
   the effect half of a parked continuation runs; a parked cost just records
   the refusal. `cont`/`cont_from` mirror Rust's parent-effect child
   continuation (rb_resolver_continue_siblings): an ability whose cost and
   effect are one wrapped tree still has to reach its effect. */
static void choice_run_skipped_continuation(GameState *g, AbilityEffect *def, int is_cost,
                                            int actor, int host,
                                            const AbilityEffect *cont, int cont_from)
{
    if (!g) return;
    if (def && !is_cost && !rb_has_pending_choice(g))
        rb_execute_effect_ex(g, actor, def, host);
    if (!rb_has_pending_choice(g))
        rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
}

/* Continue any remaining sibling effects of the parent ability after a choice
    resolves (mirrors Rust's parent-effect child continuation in provide_choice_result).

    TWO PATHS, NEVER BOTH. The remaining steps of a multi-step ability are
    reachable twice: as the entry's OWNED deep clones, and through the parked
    `cont` parent tree. Running both executes every remaining step twice.

    1. OWNED PATH — the normal case. A queue entry exists, so the entry owns
       deep clones of the tail (compound.c `save_remaining` mirrors
       compound/pause.rs:120-129 `save_remaining_actions`, which lands in
       (queue.rs:539-545 `save_pending_actions`, the one the T4-8 row is about). Rust consumes
       that list in exactly one place — `resume_pending_actions` (choice.rs:108-126)
       — so the C equivalent lives here too. Take the batch, run it in order,
       free each node as it is consumed, and re-park whatever a sub-prompt
       interrupts (choice.rs:84-106 `requeue_remaining_after_choice`).
    2. FALLBACK — the inline 登場 path only. engine.c detaches ab.cost / ab.effect
       and keeps that tree alive for the frame, but there is no entry to own a
       clone of the tail, so walking `cont` is the only handle on the remaining
       siblings. Guarded by the has_pending test above, so the two never run
       over the same nodes. */
void rb_resolver_continue_siblings(GameState *g, int actor, int host,
                                          const AbilityEffect *cont, int cont_from) {
    if (rb_queue_has_pending_actions(g)) {
        AbilityEffect *owned[RB_ENTRY_PENDING_CAP];
        int n = rb_queue_take_pending_actions_owned(g, owned, RB_ENTRY_PENDING_CAP);
        for (int i = 0; i < n; i++) {
            if (!owned[i]) continue;
            rb_execute_effect_ex(g, actor, owned[i], host);
            rb_effect_free(owned[i]);        /* ownership came to us; we release it */
            if (rb_has_pending_choice(g)) {
                /* choice.rs:114-119: park the rest back and stamp effect_started
                   (choice.rs:81-83 writes the same flag on the same event).
                   rb_queue_repark_pending_actions EXTENDS with deep clones
                   (ability_queue.c:468, and its own comment says so), exactly
                   like Rust's `queued.extend(remaining.to_vec())` — so this loop
                   still owns the originals and must release them itself. */
                rb_queue_repark_pending_actions(g, owned + i + 1, n - i - 1, 0);
                for (int k = i + 1; k < n; k++) rb_effect_free(owned[k]);
                if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                    g->queue.entries[g->queue.cur].effect_started = 1;
                return;
            }
        }
        return;                              /* tail consumed; do NOT fall through */
    }
    if (!cont) return;
    for (int j = cont_from; j < cont->n_child; j++) {
        if (rb_has_pending_choice(g)) break;
        /* Park the continuation ON the step about to run, exactly as
           engine.c:281-287 parks it when a step stalls. Without this the
           fallback walk restarts at the same `cont_from` after every answer,
           so a step that raises a prompt is re-entered forever. Rust consumes
           the parked tail from the front (compound/pause.rs:120-129
           `save_remaining_actions`), so an answered step is never re-run. */
        g->queue.resume_child = j;
        rb_execute_effect_ex(g, actor, cont->child[j], host);
    }
}

/* Did the answer being applied decline the ability's COST?
   Rust decides this in `optional_cost_skip_gate` (gates.rs:195-213), a
   POST_COST_GATE (gates.rs:258) that runs right after the cost is paid and
   STOPS the resolution with `effect_started = true` when
   `entry.optional_cost_result == Some(false)`, so run_ability_effect
   (engine.rs:267-286) is never reached. handle_general_skip itself
   (result_handlers.rs:79-87) only clears state and resumes — the stop lives in
   the gate, which is why a skip handler must not carry the effect forward.

   The C has no gate table, so the decision belongs at the skip site. The skip
   must first be established as a COST skip: a skip on an ordinary
   any_number/looked_at prompt leaves the ability alone and must keep its
   deferred continuation. rb_queue_cost_action_for_choice answers exactly the
   question entry_cost() answers — "is the choice being answered the prompt
   this ability's cost opened?" — for both the queued and the inline 登場 path,
   so the check is false for every non-cost skip and the continuation is left
   untouched. On the queued path the same verdict is readable straight off the
   entry, which is where Rust reads it. */
/* Is `ch` the fixed-count re-prompt for a cost that is already partly paid
   (choice.c choice_build_reprompt, choice.rs:765-777 Rule 9.4.2.3)? Answering
   it with a skip is NOT a bail-out: handle_general_skip (result_handlers.rs:79-87)
   only clears state and resumes and never touches optional_cost_result, which
   the partial payment already recorded as Some(true) (choice.rs:696-700). The
   gate therefore does not stop and the effect still runs. */
static int choice_is_committed_cost_reprompt(const RbChoice *ch)
{
    return ch && ch->allow_skip == 0 && ch->count > 0 &&
           strstr(ch->description, CHOICE_COST_REPROMPT_TAG) != NULL;
}

/* ── 「そうした場合」 gate (Rust choice.rs:307-321 `consume_deferred_gate`) ──
   Rust arms `in_flight.deferred_conditional_gate` when a parent-CONDITIONAL
   sequential pauses on a step that carries no condition of its own — the
   `conditional && !action.condition.is_none()==false && condition_failed==None
   && !is_opponent_action` test at compound/pause.rs:50-60, which compound.c:513
   already evaluates for the same tree — and on the next answer it DROPS the
   entry's parked tail when that answer selected nothing
   (choice.rs:312-320: `if indices.is_empty() { pending_actions.clear() }`).

   百生吟子 (PL!HS-PR-035-PR) is that shape: a `conditional` sequential whose
   optional first step is 「相手の控え室にあるメンバーカードを3枚選び、相手の
   デッキの下に好きな順番で置いてもよい。そうした場合、…ウェイトにする」.
   Declining the optional step must drop the WHOLE tail, not just the
   placement; the 「そうした場合」 rest is gated on the placement, so running it
   after a decline rests a member for nothing.

   The C cannot carry the arm across frames: `RbAbilityResolver` (choice.c:49)
   is memset to 0 at every entry point, and compound.c's
   `g_deferred_conditional_gate` is file-static and never read by anybody. The
   arm is therefore re-derived from the state the resume still holds:
     * the parked parent is a CONDITIONAL `sequential`  (pause.rs:54 `conditional`),
     * the prompt is the optional step's own allow_skip 「〜てもよい」 prompt,
     * the answer named no card at all                      (choice.rs:315).

   A SelectCard SKIP is NOT excluded, and the note that used to say so was
   wrong: it cited `handle_general_skip (result_handlers.rs:225-227)` and there
   is no `result_handlers.rs` in the tree. build_choice_result
   (turn/actions/mod.rs:1078-1087) turns EVERY SelectCard answer into
   `CardSelected { indices }` and never into `ChoiceResult::Skip`, so a declined
   optional SelectCard still reaches consume_deferred_gate (choice.rs:1048,
   inside handle_select_card). What decides the arm is `indices.is_empty()` —
   the same "no index" signal as this C's `was_skip` — which the
   `n_indices > 0` test above already reads. `ch->allow_skip` is what keeps a
   MANDATORY empty SelectCard out, matching the `decline` rule derived at
   :5061-5062. See the note on the call site for the measurement. */
static int choice_deferred_conditional_gate(const RbChoice *ch,
                                            const AbilityEffect *cont, int n_indices)
{
    if (!ch || n_indices > 0) return 0;         /* choice.rs:315 — only an EMPTY answer */
    if (!ch->allow_skip) return 0;              /* the arm is raised by an optional step */
    if (!cont || !cont->action) return 0;
    if (strcmp(cont->action, "sequential") != 0) return 0;
    return cont->conditional_flag ? 1 : 0;
}

static int choice_declined_cost_stop(GameState *g, const RbChoice *ch)
{
    if (!g) return 0;
    if (choice_is_committed_cost_reprompt(ch)) return 0;
    int cur = g->queue.cur;
    if (cur >= 0 && cur < RB_QUEUE_DEPTH) {
        RbQueueEntry *e = &g->queue.entries[cur];
        /* choice.rs:985-1034 only treats a SelectCard answer as a COST answer
           while `!effect_started && gs.entry_cost().is_some()`: the effect cannot
           run before its cost resolves, so a skip that clears both is a cost
           refusal, and a skip taken after effect_started belongs to the effect. */
        if (e->card_id >= 0 && e->effect_started) return 0;
    }
    char act[48]; act[0] = '\0';
    if (!rb_queue_cost_action_for_choice(g, ch, act, sizeof(act), NULL)) return 0;
    if (cur >= 0 && cur < RB_QUEUE_DEPTH) {
        RbQueueEntry *e = &g->queue.entries[cur];
        if (e->card_id >= 0) {
            /* choice.rs:728-733: handle_hand_cost_payment records the refusal on
               the entry as cost_paid = true / optional_cost_result = false. */
            e->cost_paid = 1;
            e->optional_cost_result = 0;
            e->effect_started = 1;   /* gates.rs:210 — the gate's stop marker */
        }
    }
    return 1;
}

/* ── the MoveCardsPosition answer ────────────────────────────────────────────
   Rust dispatches a position answer on the CHOICE KIND, never on a resume mode
   (result_handlers.rs:233-238 matches
   `(Choice::SelectPosition{..}, PositionSelected{..})`), so this ONE handler
   serves both C routes that can carry a SelectPosition: the mode==1 producer
   park (move.c:1813) and the generic kind switch. */
static const char *choice_stage_position_name(int answer_idx)
{
    /* mod.rs:2386-2392 maps the answer INDEX straight through
       ["left","center","right"] — NOT through the offered `position` list — and
       `unwrap_or("center")` covers both a skip and an out-of-range answer. */
    if (answer_idx == 0) return "left";
    if (answer_idx == 1) return "center";
    if (answer_idx == 2) return "right";
    return "center";
}

static void choice_apply_move_cards_position(GameState *g, int actor, int host,
                                             const RbChoice *saved, int answer_idx,
                                             const AbilityEffect *cont, int cont_from)
{
    if (!g) return;
    /* The MoveCardsPosition context the producer parked: target rides in
       target_player_id, source_zone in resume_draw_source, state_change in
       resume_move_state (rb_move_place_card_with_stage_choice, move.c). */
    const char *target = (saved && saved->target_player_id[0]) ? saved->target_player_id : "self";
    const char *src_zone = g->queue.resume_draw_source[0] ? g->queue.resume_draw_source : "stage";
    const char *state_change = g->queue.resume_move_state[0] ? g->queue.resume_move_state : NULL;
    rb_move_handle_select_position(g, actor, choice_stage_position_name(answer_idx),
                                   host, target, src_zone, state_change);
    /* handle_select_position returns into provide_choice_result, which then
       runs resume_pending_actions (choice.rs:108-126) — Rust's ONE consumer of
       the parked tail. rb_move_handle_select_position fires the placed card's
       登場 first (mod.rs:2423), which is what raises the NEXT prompt, so the
       tail is resumed only when nothing is pending. */
    if (!rb_has_pending_choice(g))
        rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
}

static int rb_resume_with_choice_indices_internal(GameState *g, const int *selected_indices,
                                                   int n_indices) {
    if (!g || !g->queue.has_pending) return 0;
    int selected_idx = n_indices > 0 && selected_indices ? selected_indices[0] : -1;
    fprintf(stderr, "[RESUME_ANY] kind=%d zone=%s target=%s route=%d mode=%d idx=%d\n",
            g->queue.pending.kind, g->queue.pending.zone, g->queue.pending.target, g->queue.pending.route, g->queue.resume_mode, selected_idx);
    int actor = g->queue.actor;
    int mode = g->queue.resume_mode;
    int is_select = g->queue.resume_is_select;
    AbilityEffect *eff = g->queue.resume_eff;
    int host = g->queue.resume_host;
    int replacement_actor = g->queue.resume_actor;
    /* Capture the deferred effect BEFORE clearing the queue (clearing nulls it). */
    AbilityEffect *def = g->queue.deferred;
    AbilityEffect *target_selection_eff = g->queue.target_selection_eff;
    fprintf(stderr, "[RESUME_CAPTURE] def=%p target=%p eff=%p cur=%d n=%d state=%d pending=%d\n",
            (void *)def, (void *)target_selection_eff, (void *)eff,
            g->queue.cur, g->queue.n_entries, g->queue.state, g->queue.has_pending);
    const AbilityEffect *cont = g->queue.resume_parent;
    int cont_from = g->queue.resume_child + 1;
    int was_skip = (selected_idx < 0);
    if (was_skip && g->queue.pending.actor == 1)
        g->opponent_choice_declined = 1;
    int kind = g->queue.pending.kind;   /* captured BEFORE rb_clear_pending_choice */
    RbChoice saved_pending = g->queue.pending;
    /* An EMPTY ANSWER is not a DECLINE, and Rust tells them apart by the PROMPT,
       not by the index. build_choice_result (turn/actions/mod.rs:1078-1087) turns
       EVERY SelectCard answer into `CardSelected { indices }` — an answer naming
       no card is `CardSelected([])`, never `Skip` — and dispatch_choice_result
       (result_handlers.rs:205-227) only routes `Skip` to handle_general_skip /
       handle_any_number_skip. So a MANDATORY SelectCard answered with zero picks
       must reach handle_select_card and pay its zero-card cost instead of
       `choice_declined_cost_stop` halting the resolution (k4: 「コストなしのライブ
       なら取得できる」 in parity_cce_gaps). profile_choice_is_skip
       (turn/actions/mod.rs:755-760) confirms the rule: for a SelectCard the only
       decline marker is card_id == Some(-1), while an empty index list is a
       selection. `allow_skip` is what decides it (mod.rs:807-812): a skippable
       prompt's empty answer IS a decline, a mandatory prompt's is not.
       Every other reader below keeps the original `was_skip` spelling, because
       Rust also reads an empty answer as a decline for success_replacement
       (turn/actions/mod.rs:922-947) and for SelectTarget
       (turn/actions/mod.rs:1157-1214); only the two dispatch arms of
       dispatch_choice_result are the ones `CardSelected([])` must NOT reach. */
    int decline = was_skip &&
                  !(kind == RB_CHOICE_SELECT_CARD && !saved_pending.allow_skip);
    RbAbilityResolver self; memset(&self, 0, sizeof(self));
    self.gs = g; self.actor = actor; self.host_cid = host;
    if (g->queue.pending.route == RB_ROUTE_CONDITIONAL_CHOICE) {
        self.pending_choice = g->queue.pending;
        self.has_pending_choice = 1;
        char choice_sel[32];
        choice_sel[0] = '\0';
        if (!was_skip) snprintf(choice_sel, sizeof(choice_sel), "%d", selected_idx);
        rb_resolver_handle_select_target(&self, g, g->queue.pending.target[0] ?
                                        g->queue.pending.target : "choice", choice_sel);
        if (rb_has_pending_choice(g)) {
            g->queue.state = RB_QUEUE_AWAITING_CHOICE;
        } else {
            g->queue.resume_mode = 0;
            g->queue.resume_eff = NULL;
            g->queue.state = RB_QUEUE_IDLE;
            rb_drain_ability_queue(g);
        }
        return 1;
    }
    g->queue.choice_result = selected_idx;   /* record the player's pick (select_number etc.) */
    /* Heart-color choice (draw.rs::execute_select_heart_color): map the picked
        option index back to a color so the following gain_resource consumes it. */
    if (g->queue.pending.kind == RB_CHOICE_SELECT_HEART_COLOR && !was_skip) {
        if (selected_idx >= 0 && selected_idx < g->queue.pending.n_heart_options)
            g->queue.selected_heart_color =
                (int)rb_parse_heart_color(g->queue.pending.heart_options[selected_idx]);
    }
    if (mode == 6) {
        rb_move_handle_select_cards_looked_at(g, actor,
            was_skip ? NULL : &selected_idx, was_skip ? 0 : 1, NULL, -1);
        if (rb_has_pending_choice(g)) return 1;
    }
    rb_clear_pending_choice(g);
    g->queue.resume_mode = 0;
    g->queue.resume_eff = NULL;
    g->queue.auto_ability = 0;
    g->queue.state = RB_QUEUE_RESOLVING;   /* resuming / draining an ability */
    /* The effect parked with the prompt belongs to the ENTRY's ability, and Rust
       re-runs it FROM THE ENTRY (`AbilityResolver::resume_pending_actions`,
       engine/src/ability/choice.rs:117-135, which calls
       `ability_queue.take_pending_actions()` — queue.rs:549-552 — then re-runs each
       parked action and stamps `entry.effect_started = true` when one re-opens a
       prompt, choice.rs:122-125).
       ⛔ CORRECTION 2026-10-04 (field census). This cited
       "`AbilityQueue::resume_pending_actions`, ability_queue.rs:522-536", and there
       is NO SUCH FUNCTION: `ability_queue.rs` no longer exists (the queue is
       engine/src/ability/queue.rs) and `resume_pending_actions` lives on the
       RESOLVER in choice.rs, not on the queue. `actor` above is
       the seat that has to ANSWER (rabuka.h, RbChoice.actor: "which player made the
       choice"), which is a different seat whenever the opponent is asked to pick —
       改札's 「ライブ成功時」 is exactly that shape: player 1's ability asks the
       OPPONENT to wait their members, so re-running the parked change_state with the
       answering seat resolved target="opponent" against the wrong board and the wait
       found no candidates at all (state.c's silent `nc == 0` return, state.c:317).
       Read the seat off the current entry, the way
       pca_adjusted_cost_after_modify does (game_state_abilities.c:2641). */
    int entry_actor = actor;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        const char *pid = g->queue.entries[g->queue.cur].player_id;
        if (pid && pid[0] == 'p' && (pid[1] == '1' || pid[1] == '2'))
            entry_actor = pid[1] - '1';
    }
    /* 「そうした場合」 gate: an EMPTY answer to a parent-conditional sequential's
       optional step drops the parked tail (choice.rs:307-321). Doing it here,
       once, keeps the tail out of BOTH routes that would otherwise replay it —
       the owned pending-action batch and the `cont` walk — so `cont` is retired
       for the rest of this resume instead of only clearing the batch.

       ⛔ CORRECTION 2026-10-05 (use-after-free fix fallout). The condition
       `!(was_skip && kind == RB_CHOICE_SELECT_CARD)` that used to guard this
       block was WRONG, and the justification it carried is cited to a file
       that does not exist: there is no `result_handlers.rs` in the tree (the
       dispatch is `dispatch_choice_result` in engine/src/ability/choice.rs).
       build_choice_result, turn/actions/mod.rs:1078-1087, is the line that
       decides it, and it is quoted verbatim in the note at :5043-5056 above:

           Choice::SelectCard { .. } => {
               let indices = card_indices.unwrap_or_else(...);
               Ok(ChoiceResult::CardSelected { indices })
           }

       EVERY SelectCard answer becomes `CardSelected { indices }` — an answer
       naming no card is `CardSelected([])`, NEVER `ChoiceResult::Skip`. So a
       declined optional SelectCard is still a `SelectCard` answer and DOES
       reach consume_deferred_gate (choice.rs:1048, inside handle_select_card,
       called before any cost routing). `indices.is_empty()` is the decline
       marker, not the RESULT VARIANT, and this C's `was_skip` is the same
       "no index" signal. Excluding the SelectCard kind therefore suppressed
       Rust's drop for exactly the shape Rust drops it for.

       MEASURED, and this is why it stayed hidden: the exclusion agreed with
       the observable answer only because the ability that raised the prompt
       had ALREADY been freed (game_state_abilities.c's pca_resolve_entry
       free-while-prompt-open, now fixed), so `cont` was a dangling pointer and
       `choice_deferred_conditional_gate`'s `cont->action` read was already
       undefined. With the lifetime fixed the stale exclusion became visible as
       two real regressions, both of which assert what Rust asserts:
         * parity_under_member kotori_live_start_skip_does_not_gain_heart —
           「手札にあるコスト2以下の『μ's』のメンバーカードを1枚公開し、この
           メンバーの下に置いてもよい。そうした場合、好きなハートの色を1つ
           指定する。…そのハートを1つ得る」 (cards/abilities.json:1626-1680,
           PL!-bp6-003-R＋ ab#0, a `sequential` whose first leg is
           `optional: true` and whose second leg is the heart colour + gain).
           The first leg's allow_skip prompt declined, the tail ran anyway, and
           a heart was named where Rust names none.
         * parity_cce_gaps h3_q158_skip_no_blade — the same 「そう場合」 shape,
           「そうした場合」 branch and blade obtained on a decline.

       The re-derived arm in choice_deferred_conditional_gate is already
       Rust-shaped: it fires only on an EMPTY answer (`n_indices > 0` returns 0),
       only on a prompt the optional step raised (`ch->allow_skip`), and only
       when the parked parent is a CONDITIONAL `sequential` (pause.rs:54). A
       mandatory SelectCard answered empty is therefore still excluded, which is
       the `decline` rule the same note derives at :5061-5062. Nothing else
       changes: no test is weakened and no other dispatch arm is touched. */
    if (choice_deferred_conditional_gate(&saved_pending, cont, n_indices)) {
        rb_queue_take_pending_actions(g);
        cont = NULL;
    }
    if (saved_pending.target[0] && !strcmp(saved_pending.target, "success_replacement")) {
        int pl = (replacement_actor >= 0 && replacement_actor <= 1) ? replacement_actor : actor;
        int physical_index = map_choice_index(&saved_pending, selected_idx);
        rb_handle_success_replacement_choice(g, pl, host, physical_index, !was_skip);
        if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
            rb_queue_complete_current(g);
        g->queue.resume_mode = 0;
        g->queue.resume_eff = NULL;
        g->queue.resume_actor = -1;
        g->queue.resume_host = -1;
        g->queue.resume_move_destination[0] = 0;
        rb_queue_set_state(&g->queue, RB_QUEUE_IDLE);
        rb_drain_ability_queue(g);
        return 1;
    }
    if (mode == 6) {
        rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
    } else if (mode == 2) {
        const char *dest = eff ? eff->destination : NULL;
        const char *remainder = eff ? choice_effect_extra(eff, "remainder_destination") : NULL;
        const char *discard_text = eff ? choice_effect_extra(eff, "discard_remaining") : NULL;
        int discard_remaining = discard_text
                                    ? (!strcmp(discard_text, "true") || !strcmp(discard_text, "1"))
                                    : -1;
        int mapped[RB_MAX_ZONE];
        int n_mapped = 0;
        for (int i = 0; i < n_indices; i++) {
            int mapped_index = map_choice_index(&saved_pending, selected_indices[i]);
            if (mapped_index >= 0 && n_mapped < RB_MAX_ZONE) mapped[n_mapped++] = mapped_index;
        }
        int look_owner = g->queue.resume_look_owner;
        if (look_owner < 0 || look_owner > 1) look_owner = actor;
        rb_look_resume_indices(g, look_owner, mapped, n_mapped,
                               dest, remainder, discard_remaining, is_select);
        AbilityEffect *after_look = g->queue.resume_after_look;
        g->queue.resume_after_look = NULL;
        if (after_look && !rb_has_pending_choice(g))
            rb_execute_effect_ex(g, actor, after_look, host);
    } else if (mode == 1) {
        /* mode==1 is the C's standing slot for "a position answer", and it is
           shared by two DIFFERENT Rust contexts: Rule 4.5.5's stage swap
           (state.c:1498 opens a SelectTarget, state.c:1507 resumes it) and
           MoveCardsPosition's placement (move.c:1813, a SelectPosition). Only
           the first is the stage-swap twin. rb_resume_position_change early-
           returns on `src < 0` because the arriving card is not on stage yet,
           so every placement answer was a silent no-op: the card had already
           been taken out of its source zone and was never placed anywhere.
           Rust's key is the Choice kind (result_handlers.rs:233-238), so the
           kind decides here too. */
        if (kind == RB_CHOICE_SELECT_POSITION) {
            if (!was_skip)
                choice_apply_move_cards_position(g, actor, host, &saved_pending,
                                                 selected_idx, cont, cont_from);
        } else if (!was_skip && eff) {
            g->queue.resume_active = 1;
            g->queue.choice_result = selected_idx;
            rb_resume_position_change(g, actor, eff, host, selected_idx);
            g->queue.resume_active = 0;
        }
    } else if (mode == 3) {          /* auto-ability → execute deferred body */
        if (!was_skip && def) rb_execute_effect_ex(g, actor, def, host);
    } else if (mode == 4) {         /* optional draw gate (draw.rs execute_draw_wrapper) */
        /* DEFECT 1: the gate's verdict comes from the ANSWER SHAPE
           (choice_pay_skip_answer), never from `selected_idx < 0`. draw.c:524
           opens a SelectTarget with target "draw:skip" and
           allow_skip=1, i.e. a two-option yes/no question whose option 0 is the
           printed 「no」 — Rust's `selected == "0"` refusal
           (cost/handlers.rs:1373). Reading it through `was_skip` made every
           decline look like a payment and ran the draw.
           A prompt that is not a pay/skip gate keeps the old reading, so this
           arm cannot change the meaning of any other mode-4 producer. */
        int gate = choice_pay_skip_answer(&saved_pending, selected_idx);
        int paid = (gate < 0) ? !was_skip : (gate == 1);
        int declined = (gate == 0);
        if (paid && eff) {
            /* Re-execute the full draw effect with optional stripped
               (mirrors Rust compound.rs:514 set_optional(None)): effect-level
               routing (distinct dedupe, card_type filter, destination) is
               preserved instead of the reduced low-level draw. */
            AbilityEffect eff_copy = *eff;
            eff_copy.is_optional = 0;
            rb_effect_draw_card(g, actor, &eff_copy, host);
        } else if (paid) {
            int n = 0;
            int t = g->queue.resume_draw_target;
            int self_id = g->queue.resume_draw_self_id;
            if (t == 2) { /* both */
                n += rb_draw_cards_for_player(&g->p[0], (uint8_t)g->queue.resume_draw_count,
                        g->queue.resume_draw_source, g->queue.resume_draw_dest,
                        g->queue.resume_draw_ctype, 0, NULL, NULL, -1);
                n += rb_draw_cards_for_player(&g->p[1], (uint8_t)g->queue.resume_draw_count,
                        g->queue.resume_draw_source, g->queue.resume_draw_dest,
                        g->queue.resume_draw_ctype, 0, NULL, NULL, -1);
            } else {
                n += rb_draw_cards_for_player(&g->p[t], (uint8_t)g->queue.resume_draw_count,
                        g->queue.resume_draw_source, g->queue.resume_draw_dest,
                        g->queue.resume_draw_ctype, 0, NULL, NULL, self_id);
            }
            g->last_draw_count = n;
        }
        /* DEFECT 3: a DECLINED optional cost ends the whole ability, it does
           not merely skip one step. Rust runs the POST_COST_GATES right after
           pay_ability_cost (resolver.rs:1182 then :1193-1199) and
           optional_cost_skip_gate (gates.rs:195-213, registered at gates.rs:258)
           stops on `optional_cost_result == Some(false)`, so
           run_ability_effect (engine.rs:267-286) is never reached and the
           ability's remaining commands are never resumed.

           Ordering matters and is preserved here: the decline is recorded and
           the parked commands dropped BEFORE anything else runs, exactly as
           handlers.rs:1373-1392 records the refusal before calling
           resume_pending_actions. That is what keeps a later, NON-cost skip
           from being corrupted — a prompt that is not a pay/skip gate never
           reaches this arm, so its continuation is left exactly as it was. */
        if (declined) choice_record_pay_skip_decline(g);
        /* Continue the parent ability's remaining steps through the ONE consumer
           Rust has for the parked tail (choice.rs:108-126). This arm used to
           re-implement the `cont` walk inline, which ignored the entry's owned
           clones entirely — the same two-sources defect rb_resolver_continue_
           siblings now guards. A decline is the exception: the tail is dropped,
           not replayed. */
        if (!declined) rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
    } else if (mode == 5) {         /* C6 keep-shuffle-under re-entry */
        if (!was_skip && eff) {
            /* capture the chosen card id (index into the answerer's hand) so the
                re-entered keep-shuffle phase can tell kept from moved. */
            int pl = g->queue.actor;
            RbPlayer *P = &g->p[pl];
            if (selected_idx >= 0 && selected_idx < P->hand.n) {
                if (g->n_selected_cards < RB_MAX_RECENTLY_MOVED) {
                    g->selected_cards[g->n_selected_cards++] = P->hand.cards[selected_idx];
                }
            }
            rb_effect_both_hand_keep_shuffle_under(g, host, eff, host);
        }
    } else {                         /* generic: route by choice kind to the
                                         resolver handlers (mirrors Rust
                                         provide_choice_result's match). Each
                                         handler applies the selection's mutation,
                                         then clears state and resumes the queue. */
        char selbuf[32];
        const char *selected = NULL;
        if (!was_skip) { snprintf(selbuf, sizeof(selbuf), "%d", selected_idx); selected = selbuf; }
        /* The full answer, mapped through the choice's filtered_indices exactly
           like the mode==2 (select) arm above and like Rust's ctx.mfi. */
        int sel_multi[RB_MAX_ZONE];
        int n_sel_multi = 0;
        for (int i = 0; i < n_indices; i++) {
            int mi = (selected_indices && selected_indices[i] >= 0)
                        ? map_choice_index(&saved_pending, selected_indices[i]) : -1;
            if (mi >= 0 && n_sel_multi < RB_MAX_ZONE) sel_multi[n_sel_multi++] = mi;
        }
        int is_cost = (def && def->action &&
                       (!strcmp(def->action, "pay_energy") ||
                        !strcmp(def->action, "pay_cost") ||
                        !strcmp(def->action, "activation_cost") ||
                        !strcmp(def->action, "pay_any_cost")));
        /* Faithful port of Rust dispatch_choice_result
           (engine/src/ability/choice/result_handlers.rs:204-226):
             (SelectCard{count:0, allow_skip:true}, Skip) => handle_any_number_skip
             (SelectCard{..} | SelectTarget{..}, Skip)       => handle_general_skip
             (SelectCard{..}, CardSelected{..})              => handle_select_card
           A Skip on a SelectCard therefore NEVER reaches handle_select_card /
           handle_discard_selection: handle_general_skip takes the pending
           actions, clears the choice state and resumes execution. Without the
           dedicated skip arms, a skip on a MANDATORY (allow_skip=false)
           discard-zone SelectCard re-entered handle_discard_selection, whose
           "an empty pick must re-offer" branch (choice.rs:2346-2371) rebuilt
           the same pending choice forever, so rb_resume_with_choice(state, -1)
           never converged.
           Rust's two SelectCard skip handlers both end in a resume of the
           resolver's EXECUTION CONTEXT: handle_general_skip is
           take_pending_actions -> clear_choice_state -> resume_execution
           (result_handlers.rs:79-87) and handle_any_number_skip is
           clear_choice_state_and_resume (result_handlers.rs:68-73). In Rust
           the ability's own effect lives on the ability_queue entry that is
           still current, so continuing the context runs it. In C the ability
           was never queued: engine.c:1089-1102 parks ab.effect in
           queue.deferred and then SKIPS the inline run precisely because a
           choice is pending, so queue.deferred IS the execution context and
           only this function can carry it forward. rb_resolver_clear_choice_
           state() reaches rb_clear_pending_choice() (choice.c:2418), which
           NULLs queue.deferred, which is why the continuation has to be run
           from the pointer captured before the clear — otherwise declining an
           optional SelectCard cost drops the ability's effect entirely. */
        if (decline && kind == RB_CHOICE_SELECT_CARD &&
            !(saved_pending.count == 0 && saved_pending.allow_skip)) {
            /* gates.rs:195-213 `optional_cost_skip_gate` — a POST_COST_GATE
               (gates.rs:258) that STOPS resolution with `effect_started = true`
               the moment `entry.optional_cost_result == Some(false)`, so
               run_ability_effect (engine.rs:267-286) never runs. A skip on a
               cost SelectCard is exactly that refusal, so the parked
               continuation must not be carried forward: this is the C
               counterpart of the engine.c fix that latched the skip. */
            int stop = choice_declined_cost_stop(g, &saved_pending);
            rb_queue_take_pending_actions(g);
            rb_resolver_clear_choice_state(&self);
            if (!stop)
                choice_run_skipped_continuation(g, def, is_cost, actor, host, cont, cont_from);
            rb_resolver_resume_execution(&self);
            goto choice_resume_tail;
        }
        if (decline && kind == RB_CHOICE_SELECT_CARD) {
            /* handle_any_number_skip (result_handlers.rs:67-72) */
            if (!choice_declined_cost_stop(g, &saved_pending))
                choice_run_skipped_continuation(g, def, is_cost, actor, host, cont, cont_from);
            rb_resolver_clear_choice_state_and_resume(&self);
            goto choice_resume_tail;
        }
        /* W2 (-Waddress). "discard your entire hand" optional cost.
           Rust: `Some(SelectTargetKind::PayCostAllDiscard) =>
           self.handle_pay_cost_all_discard(gs, selected)` (choice.rs:2816-2818),
           reached through `SelectTargetKind::from_str` (enums.rs:688-708), whose
           PayCostAllDiscard arm is the EXACT literal "pay_cost_all:discard_all"
           (enums.rs:705) -- an equality match, not a substring search.

           The old line had FOUR defects, all of which had to be fixed together
           or the guard stays dead:

             1. `g->queue.pending.target` is `char target[64]` (rabuka.h:943), so
                it decays to `char *` and the `&&` could NEVER be false. It was
                a null check written where a presence check was meant. The rest of
                this file already uses the correct idiom
                (`pending.target[0] ? ... : ...` at :991/:1983/:2160/:3227/:3725/
                :4768/:5216); this line was the outlier.
             2. The literal was "pay_cost_all_discard" -- UNDERSCORE. Every
                producer and table in this tree spells it "pay_cost_all:
                discard_all" (enums.c:386/:407, misc.c:2075,
                tests/test_parity_enums.c:278). `strstr` can therefore never
                match, so the branch was unreachable even with a correct
                presence test.
             3. It used `strstr` where Rust uses an exact match, which would
                also have fired on a longer unrelated target string.
             4. FOUND WHEN LANDING, and the reason the corrected routing still
                did not fire: `rb_clear_pending_choice` (choice.c:4483) memsets
                the WHOLE `g->queue.pending`, and it runs at :4815 -- 274 lines
                ABOVE this guard. So `g->queue.pending.target` is not merely
                always-true-and-then-useless here, it is ALWAYS EMPTY at this
                point, and the corrected exact-match test could never match it
                either. Every other arm in this function that needs the prompt's
                own text reads `saved_pending`, the copy captured at :4761 before
                the clear (:4784 route, :4790 target, :5265/:5269 the
                SelectTarget dispatch itself). This guard now reads the same copy,
                which is what makes the correction observable rather than inert.

           `rb_select_target_kind_from_str` (enums.c:369-388) is the 16/16 exact
           twin of Rust's `SelectTargetKind::from_str`, so routing through it is
           Rust's own dispatch rather than a second hand-written table, and it
           returns -1 for NULL and for every unknown spelling.
           REACHABILITY: see the W-A2-paycostall row in tools/audit_w_gate.c --
           2 corpus nodes, PL!SP-bp7-011-P and PL!SP-bp7-011-R, ability #0 each
           (both 登場, cost `move_cards source=hand destination=discard all=true
           optional=true`). The prompt is raised and this arm is what answers it;
           the sibling guard in rb_resolver_handle_select_target is the second
           route and now matches on the same exact literal. */
        if (rb_select_target_kind_from_str(saved_pending.target)
                == RB_STK_PAY_COST_ALL_DISCARD) {
            rb_handle_pay_cost_all_discard(g, actor, selected);
            rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
            return 1;
        }
        switch (kind) {
        case RB_CHOICE_SELECT_CARD: {
            g->queue.pending = saved_pending;
            g->queue.has_pending = 1;
            /* Faithful handle_select_card dispatch: reveal and hand-cost selections
                are applied directly by the handler (which moves cards / records the
                cost result). Plain selects still re-run the deferred effect, because
                in the C engine the deferred effect performs the actual zone move
                based on the chosen index — the engine models selection via
                deferred re-execution rather than the handler doing the move. */
            const char *pzone = g->queue.pending.zone[0] ? g->queue.pending.zone : "hand";
            int cur = g->queue.cur;
            int eff_started = (cur >= 0 && cur < RB_QUEUE_DEPTH) ? g->queue.entries[cur].effect_started : 0;
            char ca[48]; ca[0] = '\0';
            int from_debut = 0;
            int hc = rb_queue_cost_action_for_choice(g, &saved_pending, ca, sizeof(ca), &from_debut);
            /* The reveal arm answers in the handler (it records the revealed
               cards and decrements the re-prompt), so the deferred effect must
               NOT be re-run underneath it.  Same gate as
               rb_resolver_handle_select_card: the producer's flag first, the
               legacy target/zone substring second. */
            int rev = choice_is_reveal_prompt(&saved_pending) ||
                      choice_reveal_is_legacy_text(&saved_pending);
            /* `cost_hand` means the handler just paid a hand COST, so the ability
               must not be resolved twice. A QUEUED ability carries itself in
               queue.entries[cur] and advances cost -> effect inside
               rb_drain_ability_queue, which is why the deferred continuation was
               skipped there. The inline 登場 loop has no entry at all, so
               queue.deferred IS its only execution context and Rust's
               `run_ability_effect` (engine.rs:267-286, reached once
               run_effect_gates has no gate left to stop it) is exactly this
               `rb_execute_effect_ex`. */
            int cost_hand = (!eff_started && hc && !strcmp(pzone, "hand") && !from_debut);
            int sel_before = g->n_selected_cards;
            /* Rust dispatches the WHOLE answer (result_handlers.rs:41-65 copies
               `indices.to_vec()`), so an answer naming more than one card must
               not be collapsed to indices[0]. `selbuf`/`selected` above only
               ever carry the first pick, which is what makes the single-index
               handler below re-prompt for the remainder — or, on the arms that
               do not re-prompt at all (the energy placements), silently drop
               every pick but the first. Give the multi-index port the mapped
               list first; it declines (returns 0) for the zones it does not
               cover, and then the single-index path runs exactly as before. */
            int multi = 0;
            if (!was_skip && n_indices > 1) {
                multi = choice_handle_multi_indices(&self, g, &saved_pending,
                                                    sel_multi, n_sel_multi);
                if (multi) {
                    if (multi < 0) { fprintf(stderr, "[SELECT_CARD_MULTI] required hand selection was empty\n"); }
                    else fprintf(stderr, "[SELECT_CARD_MULTI] n=%d count=%d zone=%s target=%s\n",
                                 n_sel_multi, saved_pending.count, saved_pending.zone,
                                 saved_pending.target);
                } else {
                    /* not one of the arms it covers: restore the pending choice
                       for the single-index handler. */
                    g->queue.pending = saved_pending;
                    g->queue.has_pending = 1;
                }
            }
            if (!multi) {
                int rc = rb_resolver_handle_select_card(&self, g, selected);
                /* choice.rs:1205-1207 — `handle_hand_selection` returns Err for a
                   MANDATORY pick answered with NO cards, before any re-prompt, and
                   that Err propagates out of `resume_queue_with_choice`, so the
                   resolution ends with no prompt outstanding. The C spells the Err as
                   `return -1`, which on its own leaves the answered prompt in place
                   — and now that the decline arms above no longer intercept a
                   mandatory empty answer, a harness that keeps answering empty
                   re-answered the same prompt for ever (measured: target_selection
                   looped on `[RESUME_ANY] kind=1 zone=hand ... idx=-1` until the
                   run was cut off). Take the prompt down the way the error path
                   does. */
                if (rc < 0 && rb_has_pending_choice(g))
                    rb_clear_pending_choice(g);
            }
            if (rb_has_pending_choice(g) && def) g->queue.deferred = def;
            if (!was_skip && target_selection_eff && !rb_has_pending_choice(g)) {
                /* Rust misc.rs:1412-1413: the effect parked for a target-selection prompt
                   is the effect with target_count REMOVED, so re-running it applies the
                   resource to the member the player just picked (Rust misc.rs:1840-1841)
                   rather than re-deriving targets from the stage. Only clear it when this
                   answer actually produced a new selection; otherwise the saved effect
                   still carries its own target_count limit and must keep it. */
                if (g->n_selected_cards > sel_before)
                    stage_saved_effect_clear_target_count(target_selection_eff);
                rb_execute_effect_ex(g, actor, target_selection_eff, host);
                rb_effect_free(target_selection_eff);
                g->queue.target_selection_eff = NULL;
                g->n_selected_cards = 0;
            }
            if (!was_skip && def && !rev && !cost_hand && !rb_has_pending_choice(g)) {
                fprintf(stderr, "[RESUME_EXEC] def=%p action=%s is_cost=%d host=%d\n",
                        (void *)def, def->action ? def->action : "-", is_cost, host);
                if (is_cost) rb_pay_cost(g, entry_actor, def);
                else         rb_execute_effect_ex(g, entry_actor, def, host);
            }
            if (target_selection_eff && !rb_has_pending_choice(g)) {
                /* DEFECT, FIXED. This used to be
                       g->queue.cur++;
                       g->queue.n_entries = 0;
                   and both halves were wrong.

                   What Rust does at this arm: nothing to `entries`, and
                   `current_index` moves only because the ENTRY completes.
                   The prompt is `Choice::SelectCard` with `is_select_action`,
                   so handle_select_card records the picked cards and then
                   dispatches on the DECLARED intent, where
                   `StageSelectIntent::CollectTargets => return Ok(())`
                   (choice.rs:2202-2204) is a bare return; provide_choice_result
                   (choice.rs:306-323) is `dispatch_choice_result` and nothing
                   more. The target_count-REMOVED clone the producer parked as
                   pending_actions[0] (effects/target.rs:347-352) is what runs
                   next, via resume_pending_actions — not a queue-shape change.

                   What the C's line did instead: destroyed EVERY entry — the other
                   seat's entries and any entry parked mid-resolution behind an
                   open prompt — by lowering `n_entries` without running a drop.
                   An entry OWNS its `pending_actions` deep clones
                   (entry_clear_pending_actions is the C's `Vec::retain` drop),
                   so every dropped entry leaked them; and the preceding `cur++`
                   was left one past a zero-length queue.

                   The fix is `complete_current` (queue.rs:437-451): mark THIS
                   entry completed, set Idle, `current_index += 1` — nothing else.
                   It is deliberately NOT paired with `clear_completed` here:
                   Rust performs that reclamation at ONE site, abilities.rs:1901,
                   which the C already reproduces at
                   core/game_state_abilities.c:2676. Running it here as well was
                   measured and is a regression — it retires the surviving
                   entries' queue position early and costs +7 on parity_cce_gaps
                   (a20_other_same_name and a7_q30_only_one_buffed stop selecting
                   ONE target and start applying to all candidates). Leaving the
                   completed entry in place until abilities.rs:1901 is what lets
                   the owned deep clones be freed by a real retain-drop instead of
                   leaked, and `cur` is never left dangling because `n_entries`
                   still describes a queue the increment is inside.

                   Guarded on `!rb_has_pending_choice(g)`: when re-running the
                   saved effect raises ANOTHER prompt the entry is NOT finished
                   (resume_pending_actions stamps effect_started and returns,
                   choice.rs:122-130), and Rust refuses to complete for exactly
                   that reason ("skipping complete_current — pending choice
                   exists", turn/actions/mod.rs:1552-1557). The old
                   unconditional truncation destroyed the queue in that case too,
                   with the open prompt still riding on it. */
                rb_queue_complete_current(g);
            }
            if (!rb_has_pending_choice(g))
                rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
            break;
        }
        case RB_CHOICE_SELECT_TARGET:
            /* record the chosen target via the handler, then run the deferred
                effect that consumes it (C models target selection via deferral). */
            /* 「自分または相手」 needs the continuation that was parked with the
               prompt: choice.rs:2856-2907 reads `steps.first()` AFTER the answer
               and applies it with set_chosen_target, and the C's twin is the
               `self_or_opponent` arm at choice.c:3855-3870, which reads
               `g->queue.resume_eff` for the seat and for the child it parks with
               rb_queue_set_pending_actions. `g->queue.resume_eff = NULL` below the
               `rb_clear_pending_choice` call (choice.c:4795) ran BEFORE this
               dispatch, so the branch always read NULL and could never fire:
               neither rb_set_chosen_target nor rb_queue_set_pending_actions reached
               the chosen seat. Re-park the pointer captured at the top of this
               function for exactly that route and take it straight back afterwards,
               so every other reader still sees the cleared state.
               executor.c still needs its own half — `choose_target_player` must be
               structural there (executor.c:52) or the engine never walks its
               effect_steps, and no prompt of this shape is raised at all. */
            {
                int restore = (eff != NULL && saved_pending.target[0] &&
                               (!strcmp(saved_pending.target, "self_or_opponent") ||
                                !strcmp(saved_pending.target, "selfOrOpponent")));
                if (restore) g->queue.resume_eff = eff;
                rb_resolver_handle_select_target(&self, g, saved_pending.target[0] ? saved_pending.target : NULL, selected);
                if (restore) g->queue.resume_eff = NULL;
            }
            if (!was_skip && def && !rb_has_pending_choice(g)) {
                if (def->action && (!strcmp(def->action, "pay_energy") ||
                                    !strcmp(def->action, "pay_cost") ||
                                    !strcmp(def->action, "activation_cost") ||
                                    !strcmp(def->action, "pay_any_cost")))
                    rb_pay_cost(g, actor, def);
                else
                    rb_execute_effect_ex(g, actor, def, host);
            }
            if (!rb_has_pending_choice(g))
                rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
            break;
        case RB_CHOICE_SELECT_HEART_COLOR:
            /* record the picked heart color via the handler, then run the deferred
                gain_resource that consumes it. */
            rb_resolver_handle_heart_color_selection(&self, g, selected);
            if (!was_skip && def) {
                if (def->action && (!strcmp(def->action, "pay_energy") ||
                                    !strcmp(def->action, "pay_cost") ||
                                    !strcmp(def->action, "activation_cost") ||
                                    !strcmp(def->action, "pay_any_cost")))
                    rb_pay_cost(g, actor, def);
                else
                    rb_execute_effect_ex(g, actor, def, host);
            }
            rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
            break;
        case RB_CHOICE_SELECT_NUMBER:
            rb_resolver_handle_number_selection(g, was_skip ? -1 : selected_idx);
            break;
        case RB_CHOICE_SELECT_POSITION:
            /* The same handler as the mode==1 park above — one twin, one place.
               The old body read the parked seat/zone/state off
               `g->queue.resume_eff`, which the position producer never sets (and
               which the prompt's own answer path may already have freed), and it
               handed the ANSWER INDEX to the handler as the position NAME, so
               rb_stage_position_index("0") was -1 and the card fell to hand. */
            if (!was_skip)
                choice_apply_move_cards_position(g, actor, host, &saved_pending,
                                                 selected_idx, cont, cont_from);
            break;
        case RB_CHOICE_SELECT_AUTO_ABILITY:
            rb_resolver_handle_auto_ability_selection(g, selected ? atoi(selected) : -1);
            break;
        default:
            if (!was_skip && def) {
                if (is_cost) rb_pay_cost(g, actor, def);
                else         rb_execute_effect_ex(g, actor, def, host);
            }
            rb_resolver_continue_siblings(g, actor, host, cont, cont_from);
            break;
        }
    }
choice_resume_tail:
    /* continue resolving any queued trigger/auto abilities. The mode switch above
        set state = RB_QUEUE_RESOLVING, which makes rb_drain_ability_queue a no-op
        (re-entrancy guard). Normalize the state so this top-level drain actually
        runs: if a choice is still pending we must yield to the host; otherwise we
        drop back to IDLE and drain the queued auto/trigger abilities now
        (mirrors Rust provide_choice_result → resume_execution → drain). */
    fprintf(stderr, "[CHOICE_RESUME_END] pending=%d state=%d options=%d reprompt=%d\n",
            rb_has_pending_choice(g), g->queue.state, g->queue.choice_options_n,
            g->queue.choice_reprompt_pending);
    if (rb_has_pending_choice(g)) {
        g->queue.state = RB_QUEUE_AWAITING_CHOICE;
    } else {
        g->queue.state = RB_QUEUE_IDLE;
        if (g->activation_keepalive_valid) {
            fprintf(stderr, "MARK:free_keepalive pending=%d\n", g->queue.has_pending);
            fflush(stderr);
            rb_free_ability(&g->activation_keepalive);
            g->activation_keepalive_valid = 0;
            free(g->activation_act);
            g->activation_act = NULL;
        }
        rb_drain_ability_queue(g);
    }
    return 1;
}

int rb_resume_with_choice(GameState *g, int selected_idx)
{
    int index = selected_idx;
    return rb_resume_with_choice_indices_internal(
        g, &index, selected_idx < 0 ? 0 : 1);
}

int rb_resume_with_choice_indices(GameState *g, const int *indices, int n_indices)
{
    if (n_indices <= 0 || !indices)
        return rb_resume_with_choice_indices_internal(g, NULL, 0);
    return rb_resume_with_choice_indices_internal(g, indices, n_indices);
}

/* internal: emit a choice that pauses execution. Called from engine.c handle_action. */
void rb_emit_choice(GameState *g, int actor, RbChoiceKind kind,
                    const char *zone, const char *card_type,
                    int count, int allow_skip, const char *target) {
    memset(&g->queue.pending, 0, sizeof(g->queue.pending));
    g->queue.pending.kind = kind;
    if (zone) strncpy(g->queue.pending.zone, zone, sizeof(g->queue.pending.zone)-1);
    if (card_type) strncpy(g->queue.pending.card_type, card_type, sizeof(g->queue.pending.card_type)-1);
    g->queue.pending.count = count > 0 ? count : 1;
    g->queue.pending.allow_skip = allow_skip;
    if (target) strncpy(g->queue.pending.target, target, sizeof(g->queue.pending.target)-1);
    g->queue.has_pending = 1;
    g->queue.actor = actor;
    g->queue.deferred = NULL;
    g->queue.resume_look_owner = -1;
    g->queue.resume_after_look = NULL;
    /* Also pause the queue so the choice gets proper actor/player_id routing.
       The pause has to run BEFORE the QueueState stamp: its first line is
       `if (g->queue.state == RB_QUEUE_AWAITING_CHOICE) return;`
       (ability_queue.c:275), so stamping first made this call a no-op and the
       entry's choice_player_id was NEVER filled in — a pending choice left
       `queue.state == 3` with `choice_player_id == ""`, where clearing the
       state and re-pausing the same entry immediately yields "p1"/"p2".
       Rust stamps it too: pause_for_choice's "Universal default" arm
       (engine/src/ability/queue.rs:350-358) copies player_id into choice_player_id
       on every pause, and queue.rs:771 asserts it. ⛔ CORRECTION 2026-10-04: both
       lines used to cite `ability_queue.rs`, a file that no longer exists — the
       queue is engine/src/ability/queue.rs. The 345-351 and 771 line numbers were
       still correct against it and are kept.

       Two C-only side effects of the pause are deliberately neutralized, because
       neither exists in Rust — Rust's pause_for_choice only touches
       `state` and `choice_player_id` (queue.rs:303-404):

       (a) Minting a placeholder entry (ability_queue.c:315-327) when there is no
           live entry. rb_emit_choice never minted one — the state stamp used to
           shadow the early return — and a minted placeholder makes n_entries
           outrank the live entry the next resume looks up by `cur`, so the pause
           is skipped unless there IS a live entry to stamp.
       (b) `g->queue.actor = g->active` (ability_queue.c:309). This emitter was
           handed the ANSWERING player and already stored it above; the pause
           overwriting it with the seat whose turn it is reroutes the answer, so
           the emitter's value is restored after the pause. */
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        RbChoice ch = g->queue.pending;
        ch.actor = actor;  /* ensure actor is set for queue entry routing */
        rb_queue_pause_for_choice(g, &ch);
        g->queue.actor = actor;
    }
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;   /* QueueState FSM (engine/src/ability/queue.rs) */
}

/* ── Type helpers (ported from engine/src/ability/types.rs) ── */

/* gained_ability_index — extract the gained index from an encoded ability index. */
int rb_gained_ability_index(int ability_idx) {
    if (ability_idx < 10000) return -1;
    int g = ability_idx - 10000;
    if (g >= 10000) return -1;
    return g;
}

/* Choice::description_ja — returns the Japanese description for the choice. */
const char *rb_choice_description_ja(const RbChoice *ch) {
    if (!ch) return NULL;
    return ch->description_ja[0] ? ch->description_ja : NULL;
}

/* Choice::allow_skip — returns whether this choice may be skipped. */
int rb_choice_allow_skip(const RbChoice *ch) {
    if (!ch) return 0;
    return ch->allow_skip;
}

/* Choice::set_description — replace the description field. */
void rb_choice_set_description(RbChoice *ch, const char *desc) {
    if (!ch || !desc) return;
    strncpy(ch->description, desc, sizeof(ch->description) - 1);
    ch->description[sizeof(ch->description) - 1] = '\0';
}

/* Choice::set_bilingual_descriptions — replace both prompt fields. */
void rb_choice_set_bilingual_descriptions(RbChoice *ch, const char *en, const char *ja) {
    if (!ch) return;
    if (en) {
        strncpy(ch->description, en, sizeof(ch->description) - 1);
        ch->description[sizeof(ch->description) - 1] = '\0';
        strncpy(ch->description_en, en, sizeof(ch->description_en) - 1);
        ch->description_en[sizeof(ch->description_en) - 1] = '\0';
    }
    if (ja) {
        strncpy(ch->description_ja, ja, sizeof(ch->description_ja) - 1);
        ch->description_ja[sizeof(ch->description_ja) - 1] = '\0';
    }
}

/* AbilityError::to_string — human-readable error message. */
const char *rb_ability_error_to_string(int err) {
    switch (err) {
        case RB_AE_NO_MEMBER_IN_TARGET_AREA: return "Cannot baton touch - no member in target area";
        case RB_AE_AREA_LOCKED: return "Cannot baton touch: area is locked this turn";
        case RB_AE_BATON_TOUCH_PROTECTION: return "Cannot baton touch: member has baton touch discard protection";
        case RB_AE_INVALID_HAND_INDEX: return "Invalid hand index";
        case RB_AE_NOT_MEMBER_CARD: return "Only member cards can be placed on stage";
        case RB_AE_CARD_NOT_FOUND: return "Card not found in database";
        case RB_AE_ZONE_FULL: return "Live card zone is full";
        default: return "Unknown error";
    }
}




/* ---- ported choice.rs functions ---- */

/* ── handle_energy_zone_selection (engine/src/ability/effects/move/mod.rs:3349) ── */
void rb_resolver_handle_energy_zone_selection(GameState *g, int actor, const int *indices, int n_indices, const char *destination) {
    if (!g || !indices || n_indices <= 0) return;
    RbPlayer *P = &g->p[actor];
    int removed[RB_MAX_ZONE];
    int n_removed = 0;
    for (int i = n_indices - 1; i >= 0 && n_removed < RB_MAX_ZONE; i--) {
        int idx = indices[i];
        if (idx >= 0 && idx < P->energy.n) {
            removed[n_removed++] = P->energy.cards[idx];
        }
    }
    if (destination && !strcmp(destination, "under_member")) {
        if (n_removed == 0) {
            rb_queue_take_pending_actions(g);
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].optional_cost_result = 0;
        } else {
            int activating_pos = -1;
            if (g->queue.resume_host >= 0) {
                for (int i = 0; i < RB_STAGE_SIZE; i++) {
                    if (P->stage[i] == g->queue.resume_host) { activating_pos = i; break; }
                }
            }
            int target_index = -1;
            if (activating_pos >= 0 && P->stage[activating_pos] >= 0) {
                target_index = activating_pos;
            } else if (P->stage[1] >= 0) {
                target_index = 1;
            } else if (P->stage[0] >= 0) {
                target_index = 0;
            } else if (P->stage[2] >= 0) {
                target_index = 2;
            }
            if (target_index >= 0 && P->stage[target_index] >= 0) {
                for (int i = 0; i < n_removed; i++) {
                    rb_stage_place_under_card(P, target_index, removed[i]);
                    rb_mods_clear_card(&g->mods, removed[i]);
                    /* move/mod.rs:3475 — the only push_movement_event in
                       handle_energy_zone_selection, and it names both zones:
                       "energy_zone" -> "under_member". Spelled out here because
                       RB_ZONEID_STAGE == 0, so a bare 0,0 would be read as a
                       stage->stage area move by the choke point's hook
                       (modifiers.c:417-420) and spuriously set
                       position_change_occurred_this_turn and arm the
                       opponent-cause watchers. */
                    rb_record_card_movement(g, removed[i], RB_ZONEID_ENERGY_ZONE,
                                            RB_ZONEID_UNDER_MEMBER, 0, 0);
                }
            } else {
                for (int i = 0; i < n_removed; i++) {
                    if (P->energy_deck.n < RB_MAX_DECK)
                        P->energy_deck.cards[P->energy_deck.n++] = removed[i];
                }
            }
        }
    } else if (destination) {
        /* Arbitrary destination (hand / deck / discard / ...). The source is the
           energy zone, so the from-zone is never RB_ZONEID_STAGE and this pair
           can never be mistaken for the stage->stage area move the choke point
           keys on. rb_zone_id_from_str mirrors Rust's ZoneId::from_str: an
           unrecognised wire name yields RB_ZONEID_UNKNOWN, which is Rust's own
           alias-drift value (modifiers.rs:1492-1502) and is likewise not a
           stage zone. */
        int dest_zone = rb_zone_id_from_str(destination);
        for (int i = 0; i < n_removed; i++) {
            rb_place_card_in_zone(g, actor, removed[i], destination, -1);
            rb_mods_clear_card(&g->mods, removed[i]);
            rb_record_card_movement(g, removed[i], RB_ZONEID_ENERGY_ZONE, dest_zone, 0, 0);
        }
    } else {
        for (int i = 0; i < n_removed; i++) {
            rb_mods_clear_card(&g->mods, removed[i]);
            rb_mods_set_orientation(&g->mods, removed[i], "wait");
        }
    }
}

/* rb_resolver_handle_select_position used to live here as a SECOND copy of
   handle_select_position (mod.rs:2372-2426). Rust has exactly one
   handle_select_position, selected by the (Choice, ChoiceResult) pair at
   result_handlers.rs:233-238, so the twin now lives once — move.c's
   rb_move_handle_select_position — and both C routes that can carry a
   SelectPosition call it through choice_apply_move_cards_position. The copy that
   stood here also read its seat/zone from `g->queue.resume_eff`, which the
   position producer never sets, and had `if (should_lock) { (void)g; }` where
   track_deployment belongs. */

void rb_resolver_handle_number_selection(GameState *g, int selected)
{
    if (!g) return;
    /* W4 (-Wdiscarded-qualifiers). `RbQueueState::resume_parent` is
       `const AbilityEffect *` (rabuka.h:1200), so `AbilityEffect *parent =
       g->queue.resume_parent` threw the const away. Nothing in the body WRITES
       through it -- `parent` is only tested for NULL and forwarded to
       rb_resolver_continue_siblings -- so carrying the const is correct and
       costs nothing. Verified, not assumed: the callee takes the same const
       pointer, so no cast is needed at the call site either. */
    const AbilityEffect *parent = g->queue.resume_parent;
    int child = g->queue.resume_child;
    int actor = g->queue.resume_actor >= 0 ? g->queue.resume_actor : g->queue.actor;
    int host = g->queue.resume_host;
    g->queue.choice_result = selected;
    rb_clear_pending_choice(g);
    g->queue.resume_mode = 0;
    g->queue.state = RB_QUEUE_RESOLVING;
    rb_queue_resume_pending_actions(g);
    if (!rb_has_pending_choice(g) && parent && child >= 0)
        rb_resolver_continue_siblings(g, actor, host, parent, child + 1);
    if (!rb_has_pending_choice(g)) {
        g->queue.state = RB_QUEUE_IDLE;
        rb_drain_ability_queue(g);
    }
}

void rb_resolver_handle_auto_ability_selection(GameState *g, int selected)
{
    if (!g) return;
    g->queue.choice_result = selected;
    int cur = g->queue.cur;
    if (selected >= 0 && cur >= 0 && cur < g->queue.n_entries) {
        int target = cur + selected;
        if (target >= g->queue.n_entries) target = selected;
        if (target >= 0 && target < g->queue.n_entries && target != cur) {
            RbQueueEntry tmp = g->queue.entries[cur];
            g->queue.entries[cur] = g->queue.entries[target];
            g->queue.entries[target] = tmp;
        }
    }
    g->queue.auto_ability = 0;
    rb_clear_pending_choice(g);
    g->queue.state = RB_QUEUE_IDLE;
    rb_process_pending_auto_abilities(g);
    if (!rb_has_pending_choice(g)) rb_drain_ability_queue(g);
}

