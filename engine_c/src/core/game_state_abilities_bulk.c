/* ═══════════════════════════════════════════════════════════════════════
 * Bulk port of remaining abilities.rs functions (1–15).
 * Append to engine_c/src/core/game_state_abilities.c
 * ═══════════════════════════════════════════════════════════════════════ */

#include "rabuka.h"
#include <stdint.h>
#include <string.h>
#include <stdio.h>

/* Forward declarations for helpers not yet in rabuka.h */
static int rb_card_num_gained_abilities(uint32_t cid);
static const Ability *rb_card_gained_ability(uint32_t cid, int idx);

typedef struct {
    int card_id;
    int ability_idx;
} RbConstantIdPair;

static int rb_constant_pair_ability_matches(const Ability *ab)
{
    if (!ab) return 0;
    return rb_ability_matches_trigger(ab, "constant")
        || rb_ability_matches_trigger(ab, "continuous")
        || (ab->triggers == NULL || ab->triggers[0] == '\0');
}

/* (1) collect_constant_ids_for — scan card ids for constant abilities. */
static int rb_collect_constant_ids_for(const GameState *g,
                                       const int *cids, int n,
                                       RbConstantIdPair *out, int max)
{
    if (!g || !cids || !out || max <= 0) return 0;
    int count = 0;
    for (int i = 0; i < n; i++) {
        int cid = cids[i];
        if (cid < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (rb_constant_pair_ability_matches(&ab) && ab.effect) {
                if (count < max) {
                    out[count].card_id = cid;
                    out[count].ability_idx = a;
                    count++;
                }
            }
            rb_free_ability(&ab);
        }
        int ng = rb_card_num_gained_abilities((uint32_t)cid);
        for (int gidx = 0; gidx < ng; gidx++) {
            const Ability *gab = rb_card_gained_ability((uint32_t)cid, gidx);
            if (gab && rb_constant_pair_ability_matches(gab) && gab->effect) {
                if (count < max) {
                    out[count].card_id = cid;
                    out[count].ability_idx = GAINED_ABILITY_INDEX_BASE + gidx;
                    count++;
                }
            }
        }
    }
    return count;
}

/* (2) collect_constant_hand_effect_ids */
int rb_collect_constant_hand_effect_ids(const GameState *g,
                                        RbConstantIdPair *out, int max)
{
    if (!g) return 0;
    int ids[RB_MAX_HAND * 2];
    int n = 0;
    for (int i = 0; i < g->p[0].hand.n && n < (int)(sizeof(ids)/sizeof(ids[0])); i++)
        ids[n++] = g->p[0].hand.cards[i];
    for (int i = 0; i < g->p[1].hand.n && n < (int)(sizeof(ids)/sizeof(ids[0])); i++)
        ids[n++] = g->p[1].hand.cards[i];
    return rb_collect_constant_ids_for(g, ids, n, out, max);
}

/* (3) collect_constant_stage_effect_ids */
int rb_collect_constant_stage_effect_ids(const GameState *g,
                                         RbConstantIdPair *out, int max)
{
    if (!g) return 0;
    int ids[RB_STAGE_SIZE * 2];
    int n = rb_stage_card_ids(g, ids, RB_STAGE_SIZE * 2);
    return rb_collect_constant_ids_for(g, ids, n, out, max);
}

/* (4) fire_opponent_cause_watchers_for_move */
void rb_fire_opponent_cause_watchers_for_move(GameState *g, int moved_card_id,
                                              int causer_player)
{
    if (!g || moved_card_id < 0) return;
    int owner = rb_owner_of_card(g, moved_card_id);
    if (owner < 0) return;
    if (owner == causer_player) return;

    const RbPlayer *op = &g->p[owner];
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int watcher_id = op->stage[s];
        if (watcher_id < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)watcher_id);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)watcher_id, a, &ab)) continue;
            if (!rb_ability_matches_trigger(&ab, "自動")) { rb_free_ability(&ab); continue; }
            if (!ab.effect) { rb_free_ability(&ab); continue; }
            int fires_opp = 0;
            for (int k = 0; k < ab.effect->n_extra; k++) {
                if (ab.effect->extra_k[k] && !strcmp(ab.effect->extra_k[k], "fires_on_opponent_effects")
                    && ab.effect->extra_v[k] && !strcmp(ab.effect->extra_v[k], "true")) {
                    fires_opp = 1; break;
                }
            }
            if (!fires_opp) { rb_free_ability(&ab); continue; }
            if (!ab.effect->condition) { rb_free_ability(&ab); continue; }
            int passes = rb_eval_condition_for_host(g, owner, watcher_id,
                                                     ab.effect->condition);
            if (!passes) { rb_free_ability(&ab); continue; }
            int num_key = (watcher_id << 16) | (a & 0xFFFF);
            int dup = 0;
            for (int b = 0; b < g->n_batch_triggered_keys; b++)
                if (g->batch_triggered_keys[b] == num_key) { dup = 1; break; }
            if (!dup) {
                int cap = (int)(sizeof(g->batch_triggered_keys)/sizeof(g->batch_triggered_keys[0]));
                if (g->n_batch_triggered_keys < cap)
                    g->batch_triggered_keys[g->n_batch_triggered_keys++] = num_key;
                rb_queue_push(&g->queue, watcher_id, a);
                rb_record_use(&g->queue, watcher_id, a, g->turn);
            }
            rb_free_ability(&ab);
        }
    }
}

/* (5) trigger_auto_ability — string-keyed auto-ability trigger */
void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             int trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id)
{
    if (!g || !ability_id) return;
    int cid = explicit_card_id;
    if (cid < 0 && source_card_no) {
        int found = -1;
        if (rb_find_card_by_number_for_player(g, player_id, 0, &found) > 0)
            cid = found;
    }
    if (cid < 0) return;
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (!rb_ability_matches_trigger(&ab, "自動")) { rb_free_ability(&ab); continue; }
        char expected[256];
        snprintf(expected, sizeof(expected), "%s_%s", source_card_no ? source_card_no : "", ab.full_text ? ab.full_text : "");
        if (strcmp(ability_id, expected) != 0) { rb_free_ability(&ab); continue; }
        rb_queue_push(&g->queue, cid, a);
        rb_record_use(&g->queue, cid, a, g->turn);
        rb_free_ability(&ab);
        return;
    }
    (void)trigger_type; (void)trigger_moved_cards; (void)n_moved; (void)triggering_member_id;
}

/* (6) trigger_auto_ability_by_index — numeric-index version */
void rb_trigger_auto_ability_by_index(GameState *g, int trigger_type,
                                     int player_id, int explicit_card_id,
                                     int ability_index,
                                     const int *trigger_moved_cards, int n_moved,
                                     int triggering_member_id)
{
    if (!g || explicit_card_id < 0) return;
    int nab = rb_card_num_abilities((uint32_t)explicit_card_id);
    if (ability_index < 0 || ability_index >= nab) return;
    rb_queue_push(&g->queue, explicit_card_id, ability_index);
    rb_record_use(&g->queue, explicit_card_id, ability_index, g->turn);
    (void)trigger_type; (void)player_id;
    (void)trigger_moved_cards; (void)n_moved; (void)triggering_member_id;
}

/* (7) condition_tree_group_names — recursive search for group filter */
static const char **rb_condition_tree_group_names(const Condition *cond)
{
    static const char *groups[32];
    if (!cond) return NULL;
    const char *gn = rb_cond_get_str(cond, "group_names");
    if (gn && gn[0]) {
        static char buf[1024];
        static const char *parsed[32];
        strncpy(buf, gn, sizeof(buf) - 1);
        buf[sizeof(buf) - 1] = '\0';
        int n = 0;
        char *tok = strtok(buf, ",");
        while (tok && n < 32) {
            parsed[n++] = tok;
            tok = strtok(NULL, ",");
        }
        if (n > 0) {
            for (int i = 0; i < n; i++) groups[i] = parsed[i];
            groups[n] = NULL;
            return groups;
        }
    }
    for (uint32_t i = 0; i < cond->n_fields; i++) {
        const CondValue *dv = &cond->fields[i].v;
        if (dv->tag == RB_TAG_OBJVAR && dv->cond) {
            const char **found = rb_condition_tree_group_names(dv->cond);
            if (found) return found;
        }
        if (dv->tag == RB_TAG_ARRAY) {
            for (uint32_t j = 0; j < dv->arr_n; j++) {
                if (dv->arr[j].tag == RB_TAG_OBJVAR && dv->arr[j].cond) {
                    const char **found = rb_condition_tree_group_names(dv->arr[j].cond);
                    if (found) return found;
                }
            }
        }
    }
    return NULL;
}

const char **rb_condition_tree_group_names_pub(const Condition *cond)
{
    return rb_condition_tree_group_names(cond);
}

/* (8) process_player_abilities_depth — already exists in game_state_abilities.c
 * (line 964). The existing stub calls rb_process_player_abilities once;
 * the full Rust version re-enters up to max_depth times. To upgrade,
 * replace the existing body with:
 *
 *   if (!g || max_depth <= 0) return 0;
 *   int total = 0;
 *   for (int depth = 0; depth < max_depth; depth++) {
 *       int processed = rb_process_player_abilities(g, pl);
 *       total += processed;
 *       if (processed == 0) break;
 *       if (g->queue.has_pending) break;
 *   }
 *   return total;
 */

/* (9) get_pending_choice_json — stub (no serde in C port) */
void rb_get_pending_choice_json(const GameState *g, char *buf, size_t buf_sz)
{
    if (!buf || buf_sz == 0) return;
    buf[0] = '\0';
    if (!g) return;
    snprintf(buf, buf_sz, "{}");
}

/* (10) entry_choice_card_no — returns ChoiceRoute for current entry */
RbChoiceRoute rb_entry_choice_card_no(const GameState *g)
{
    if (!g || !g->queue.has_pending) return RB_ROUTE_NONE;
    return g->queue.pending.route;
}

/* (11) entry_conditional_choice — returns conditional-choice flag */
int rb_entry_conditional_choice(const GameState *g)
{
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return 0;
    return g->queue.entries[g->queue.cur].optional_cost_result;
}

/* (12) resolve_target_player_mut — mutable version of resolve_target_player */
RbPlayer *rb_resolve_target_player_mut(GameState *g, const char *target)
{
    if (!g) return NULL;
    int master = rb_resolve_master_id(g);
    int master_p2 = (master == 1);
    if (target && !strcmp(target, "self"))
        return master_p2 ? &g->p[1] : &g->p[0];
    if (target && !strcmp(target, "opponent"))
        return master_p2 ? &g->p[0] : &g->p[1];
    return &g->p[0];
}

/* (13) owner_of_card — already exists in src/ability/ability_queue.c
 * (line 55). The existing implementation searches stage/hand/deck/discard/
 * energy/live/success zones and returns the player index (0/1) or -1. */

/* (14) check_expired_effects — already exists in src/turn/triggers.c
 * (line 390). The existing implementation reverts blade/score/cost/heart/
 * need_heart modifiers for expired temporary effects and compacts the array. */

/* (15) reset_loop_detection — already exists in src/core/tracking.c
 * (line 15). The existing implementation clears n_game_state_history and
 * loop_detected. */

/* Stubs for gained-ability helpers (not yet implemented in C port) */
static int rb_card_num_gained_abilities(uint32_t cid) { (void)cid; return 0; }
static const Ability *rb_card_gained_ability(uint32_t cid, int idx) { (void)cid; (void)idx; return NULL; }
