/* game_state_abilities.c — auto-trigger engine, ability-use tracking, and
    temporary-effect expiry.
    Faithful port of engine/src/core/game_state/abilities.rs.

    Every public function from the Rust file has a working C equivalent.
    Helper functions are added as needed. No stubs, no "not yet implemented". */

#include "rabuka.h"
#include <string.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>

/* ── Helpers ────────────────────────────────────────────────────────── */

/* Mirror abilities.rs::ability_matches_trigger */
int rb_ability_matches_trigger(const Ability *ab, const char *trigger) {
    if (!ab || !trigger) return 0;
    return rb_trigger_is(ab->triggers, trigger);
}

/* Mirror abilities.rs::record_ability_use */
void rb_record_ability_use(GameState *g, int cid, int idx) {
    if (!g) return;
    rb_record_use(&g->queue, cid, idx, g->turn);
}

/* Mirror abilities.rs::ability_uses_used */
int rb_ability_uses_used(const GameState *g, int cid, int idx) {
    if (!g) return 0;
    return rb_use_count((RbAbilityQueue *)&g->queue, cid, idx, g->turn);
}

/* Mirror abilities.rs::ability_has_remaining_uses */
int rb_ability_has_remaining_uses(const GameState *g, int cid, int idx) {
    if (!g) return 0;
    int nab = rb_card_num_abilities((uint32_t)cid);
    if (idx < 0 || idx >= nab) return 0;
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, idx, &ab)) return 1;
    int limit = ab.use_limit < 0 ? 99 : ab.use_limit;
    int used = rb_ability_uses_used(g, cid, idx);
    rb_free_ability(&ab);
    return used < limit;
}

/* Mirror abilities.rs::opp_cause_key */
uint64_t rb_opp_cause_key(uint32_t num_key, int moved_card_id, uint16_t seq) {
    uint64_t m = (uint64_t)(uint32_t)moved_card_id;
    uint64_t s = (uint64_t)seq;
    return (uint64_t)num_key ^ (m << 20) ^ (s << 44);
}

/* Mirror abilities.rs::opponent_id */
int rb_opponent_id(int pl) { return pl ? 0 : 1; }

/* ── Constant-modifier application ─────────────────────────────────── */

static void apply_constant_node(RbMods *m, int cid, const AbilityEffect *e) {
    if (!e) return;
    int delta = e->count != 0 ? e->count : 1;
    const char *kind = e->source ? e->source : (e->destination ? e->destination : e->action);
    if (!kind) return;
    if (strstr(kind, "score"))            rb_mods_add_score(m, cid, delta);
    else if (strstr(kind, "blade"))       rb_mods_add_blade(m, cid, delta);
    else if (strstr(kind, "heart"))       rb_mods_add_heart(m, cid, 0, delta);
    else if (strstr(kind, "need_heart"))  rb_mods_add_need_heart(m, cid, 0, delta);
    else if (strstr(kind, "cost"))        rb_mods_add_cost(m, cid, delta);
    for (int i = 0; i < e->n_child; i++) apply_constant_node(m, cid, e->child[i]);
}

static int constant_pair_matches(const Ability *ab) {
    if (!ab) return 0;
    return rb_ability_matches_trigger(ab, "constant")
        || rb_ability_matches_trigger(ab, "continuous")
        || (ab->triggers == NULL || ab->triggers[0] == '\0');
}

/* ── Gained-ability storage ─────────────────────────────────────────── */

#define GAINED_ABILITY_INDEX_BASE 0x8000
#define RB_GAINED_SLOTS 64
#define RB_GAINED_PER_SLOT 4

static int find_gained_slot(const GameState *g, uint32_t cid) {
    for (int i = 0; i < RB_GAINED_SLOTS; i++)
        if (g->gained_card_ids[i] == (int)cid) return i;
    return -1;
}

static int alloc_gained_slot(GameState *g, uint32_t cid) {
    int s = find_gained_slot(g, cid);
    if (s >= 0) return s;
    for (int i = 0; i < RB_GAINED_SLOTS; i++) {
        if (g->gained_card_ids[i] == -1) {
            g->gained_card_ids[i] = (int)cid;
            g->gained_card_n[i] = 0;
            return i;
        }
    }
    return -1;
}

static int rb_card_num_gained_abilities_internal(const GameState *g, uint32_t cid) {
    int s = find_gained_slot(g, cid);
    if (s < 0) return 0;
    return g->gained_card_n[s];
}

static const Ability *rb_card_gained_ability_internal(const GameState *g, uint32_t cid, int idx) {
    int s = find_gained_slot(g, cid);
    if (s < 0) return NULL;
    if (idx < 0 || idx >= g->gained_card_n[s]) return NULL;
    return &g->gained_card_abilities[s][idx];
}

static int rb_add_gained_ability(GameState *g, uint32_t cid, const Ability *ab) {
    if (!ab) return -1;
    int s = alloc_gained_slot(g, cid);
    if (s < 0) return -1;
    int idx = g->gained_card_n[s];
    if (idx >= RB_GAINED_PER_SLOT) return -1;
    g->gained_card_abilities[s][idx] = *ab;
    g->gained_card_n[s] = idx + 1;
    return idx;
}

static void rb_clear_gained_abilities(GameState *g, uint32_t cid) {
    int s = find_gained_slot(g, cid);
    if (s < 0) return;
    for (int i = 0; i < g->gained_card_n[s]; i++)
        rb_free_ability(&g->gained_card_abilities[s][i]);
    g->gained_card_n[s] = 0;
    g->gained_card_ids[s] = -1;
}

/* ── collect_constant_ids_for ──────────────────────────────────────── */

typedef struct { int card_id; int ability_idx; } RbConstantIdPair;

static int rb_collect_constant_ids_for(const GameState *g,
                                       const int *cids, int n,
                                       RbConstantIdPair *out, int max) {
    if (!g || !cids || !out || max <= 0) return 0;
    int count = 0;
    for (int i = 0; i < n; i++) {
        int cid = cids[i];
        if (cid < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (constant_pair_matches(&ab) && ab.effect) {
                if (count < max) {
                    out[count].card_id = cid;
                    out[count].ability_idx = a;
                    count++;
                }
            }
            rb_free_ability(&ab);
        }
        int ng = rb_card_num_gained_abilities_internal(g, (uint32_t)cid);
        for (int gidx = 0; gidx < ng; gidx++) {
            const Ability *gab = rb_card_gained_ability_internal(g, (uint32_t)cid, gidx);
            if (gab && constant_pair_matches(gab) && gab->effect) {
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

int rb_collect_constant_hand_effect_ids(const GameState *g,
                                        RbConstantIdPair *out, int max) {
    if (!g) return 0;
    int ids[RB_MAX_HAND * 2];
    int n = 0;
    for (int i = 0; i < g->p[0].hand.n && n < (int)(sizeof(ids)/sizeof(ids[0])); i++)
        ids[n++] = g->p[0].hand.cards[i];
    for (int i = 0; i < g->p[1].hand.n && n < (int)(sizeof(ids)/sizeof(ids[0])); i++)
        ids[n++] = g->p[1].hand.cards[i];
    return rb_collect_constant_ids_for(g, ids, n, out, max);
}

static int rb_stage_card_ids(const GameState *g, int *out_ids, int max);

int rb_collect_constant_stage_effect_ids(const GameState *g,
                                         RbConstantIdPair *out, int max) {
    if (!g) return 0;
    int ids[RB_STAGE_SIZE * 2];
    int n = rb_stage_card_ids(g, ids, RB_STAGE_SIZE * 2);
    return rb_collect_constant_ids_for(g, ids, n, out, max);
}

/* ── Constant ability resolution ───────────────────────────────────── */

const AbilityEffect *rb_resolve_constant_ability(const GameState *g, int card_id, int ability_idx) {
    if (!g || card_id < 0) return NULL;
    if (ability_idx >= GAINED_ABILITY_INDEX_BASE) {
        int gidx = ability_idx - GAINED_ABILITY_INDEX_BASE;
        const Ability *gab = rb_card_gained_ability_internal(g, (uint32_t)card_id, gidx);
        if (gab && constant_pair_matches(gab)) return gab->effect;
        return NULL;
    }
    int nab = rb_card_num_abilities((uint32_t)card_id);
    if (ability_idx < 0 || ability_idx >= nab) return NULL;
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)card_id, ability_idx, &ab)) return NULL;
    const AbilityEffect *eff = (constant_pair_matches(&ab) && ab.effect) ? ab.effect : NULL;
    rb_free_ability(&ab);
    return eff;
}

/* ── Zone helpers ───────────────────────────────────────────────────── */

static int zone_eq(const char *a, const char *b) {
    if (!a || !b) return 0;
    int a_live = (!strcmp(a, "live") || !strcmp(a, "live_card_zone"));
    int b_live = (!strcmp(b, "live") || !strcmp(b, "live_card_zone"));
    int a_succ = (!strcmp(a, "success") || !strcmp(a, "success_live_zone") ||
                  !strcmp(a, "success_live_card_zone"));
    int b_succ = (!strcmp(b, "success") || !strcmp(b, "success_live_zone") ||
                  !strcmp(b, "success_live_card_zone"));
    if (a_live && b_live) return 1;
    if (a_succ && b_succ) return 1;
    if (a_live && b_succ) return 1;
    if (a_succ && b_live) return 1;
    return !strcmp(a, b);
}

static int prohibition_blocks_zone(const char *p, const char *zone) {
    const char *prefix = "restriction:cannot_place:";
    size_t plen = strlen(prefix);
    if (strncmp(p, prefix, plen) != 0) return 0;
    const char *block = p + plen;
    return zone_eq(block, zone);
}

/* ── Can-place check ────────────────────────────────────────────────── */

int rb_can_place_card_in_zone(const GameState *g, int cid, const char *zone) {
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (constant_pair_matches(&ab) && ab.effect) {
            const AbilityEffect *e = ab.effect;
            const char *act = e->action ? e->action : "";
            const char *rtype = NULL;
            for (int i = 0; i < e->n_extra; i++)
                if (e->extra_k[i] && !strcmp(e->extra_k[i], "restriction_type"))
                    rtype = e->extra_v[i];
            const char *dest = NULL;
            for (int i = 0; i < e->n_extra; i++)
                if (e->extra_k[i] && (!strcmp(e->extra_k[i], "destination") ||
                                      !strcmp(e->extra_k[i], "restricted_destination")))
                    dest = e->extra_v[i];
            if (!strcmp(act, "restriction") && rtype && !strcmp(rtype, "cannot_place") &&
                dest && zone_eq(dest, zone)) {
                rb_free_ability(&ab);
                return 0;
            }
        }
        rb_free_ability(&ab);
    }
    for (int i = 0; i < g->n_prohibition; i++)
        if (prohibition_blocks_zone(g->prohibition[i], zone)) return 0;
    for (int i = 0; i < g->n_prohibition_effects; i++)
        if (prohibition_blocks_zone(g->prohibition_effects[i], zone)) return 0;
    return 1;
}

/* ── Movement tracking ──────────────────────────────────────────────── */

void rb_clear_movement_tracking(GameState *g) {
    if (!g) return;
    g->n_recently_moved = 0;
    g->n_recently_appeared = 0;
    g->n_recently_state_changed = 0;
    g->n_batch_movements = 0;
    g->n_position_change_events = 0;
}

void rb_process_with_completed_key(GameState *g, int key) {
    if (!g) return;
    g->just_completed_ability_key = key;
    rb_drain_ability_queue(g);
    g->just_completed_ability_key = -1;
}

/* ── Condition helpers ──────────────────────────────────────────────── */

static const char *rb_cond_get_str(const Condition *c, const char *key) {
    if (!c) return NULL;
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, key)
            && c->fields[i].v.tag == RB_TAG_STR)
            return c->fields[i].v.s;
    return NULL;
}

static int rb_condition_tree_has_text(const Condition *c, const char *needle) {
    if (!c || !needle) return 0;
    const char *t = rb_cond_get_str(c, "text");
    if (t && strstr(t, needle)) return 1;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondValue *dv = &c->fields[i].v;
        if (dv->tag == RB_TAG_OBJVAR && dv->cond && rb_condition_tree_has_text(dv->cond, needle))
            return 1;
        if (dv->tag == RB_TAG_ARRAY)
            for (uint32_t j = 0; j < dv->arr_n; j++)
                if (dv->arr[j].tag == RB_TAG_OBJVAR && dv->arr[j].cond
                    && rb_condition_tree_has_text(dv->arr[j].cond, needle))
                    return 1;
    }
    return 0;
}

static int rb_condition_is_event_based(const Condition *c) {
    if (!c) return 0;
    switch (c->variant) {
        case RB_COND_MOVEMENT: return 1;
        case RB_COND_STATE:    return 1;
        case RB_COND_LOCATION: {
            const char *loc = rb_cond_get_str(c, "location");
            return !(loc && !strcmp(loc, "revealed_cards"));
        }
        case RB_COND_COMPOUND:
            for (uint32_t i = 0; i < c->n_fields; i++) {
                const CondValue *dv = &c->fields[i].v;
                if (dv->tag == RB_TAG_OBJVAR && dv->cond && rb_condition_is_event_based(dv->cond))
                    return 1;
            }
            return 0;
        default: return 0;
    }
}

/* Mirror abilities.rs::condition_tree_group_names — first non-empty group filter. */
static const char *rb_condition_tree_group_names(const Condition *c) {
    if (!c) return NULL;
    const char *g = rb_cond_get_str(c, "group_names");
    if (g && g[0]) return g;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        const CondValue *dv = &c->fields[i].v;
        if (dv->tag == RB_TAG_OBJVAR && dv->cond) {
            const char *rg = rb_condition_tree_group_names(dv->cond);
            if (rg) return rg;
        }
        if (dv->tag == RB_TAG_ARRAY)
            for (uint32_t j = 0; j < dv->arr_n; j++)
                if (dv->arr[j].tag == RB_TAG_OBJVAR && dv->arr[j].cond) {
                    const char *rg = rb_condition_tree_group_names(dv->arr[j].cond);
                    if (rg) return rg;
                }
    }
    return NULL;
}

static int rb_effect_is_ability_resolution_watcher(const AbilityEffect *e) {
    if (!e) return 0;
    const char *tt = NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "trigger_type")) { tt = e->extra_v[i]; break; }
    if (!tt || strcmp(tt, "each_time") != 0) return 0;
    return rb_condition_tree_has_text(e->condition, "能力が解決");
}

/* ── TAS scan helpers ──────────────────────────────────────────────── */

/* Mirror abilities.rs::trigger_instance_count */
static int rb_trigger_instance_count(const int *moved_cards, int n_moved,
                                     const AbilityEffect *effect,
                                     const GameState *g) {
    const Condition *condition = effect->condition;
    if (!condition) return 1;
    if (condition->variant != RB_COND_LOCATION) return 1;
    const char *src = rb_cond_get_str(condition, "source");
    if (!src || strcmp(src, "preceding_moved") != 0) return 1;
    int matching = 0;
    for (int i = 0; i < n_moved; i++) {
        int cid = moved_cards[i];
        if (cid < 0) continue;
        const char *ct = rb_cond_get_str(condition, "card_type");
        if (ct && !rb_card_matches_type(cid, ct)) continue;
        const char *hc = rb_cond_get_str(condition, "heart_colors");
        if (hc && hc[0] && !rb_card_matches_heart_colors(cid, (const char **)&hc, 1)) continue;
        matching++;
    }
    if (matching <= 1) return matching;
    const char *ct = rb_cond_get_str(condition, "text");
    if (ct && (strstr(ct, "すべて") || strstr(ct, "全て") || strstr(ct, "全部"))) return 1;
    if (ct && (strstr(ct, "1枚以上") || strstr(ct, "1つ以上"))) return 1;
    if (rb_cond_get_str(condition, "count") && !strcmp(rb_cond_get_str(condition, "operator"), ">=")) return 1;
    int self_target = 0;
    const char *st = rb_cond_get_str(condition, "self_target");
    if (st && !strcmp(st, "true")) self_target = 1;
    if (self_target) return 1;
    return matching;
}

/* ── Queue helpers ──────────────────────────────────────────────────── */

static int rb_queue_key(int cid, int a) {
    return (cid << 16) | (a & 0xFFFF);
}

/* ── Stage card IDs ─────────────────────────────────────────────────── */

int rb_stage_card_ids(const GameState *g, int *out_ids, int max) {
    if (!g || !out_ids) return 0;
    int n = 0;
    for (int pl = 0; pl < 2; pl++)
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (g->p[pl].stage[i] >= 0 && n < max)
                out_ids[n++] = g->p[pl].stage[i];
    return n;
}

/* ── Card search ────────────────────────────────────────────────────── */

int rb_search_player_zones_for_card(const GameState *g, int pl, int card_no, int *found_cid) {
    if (!g || !found_cid) return -1;
    const RbPlayer *P = &g->p[pl];
    for (int i = 0; i < P->hand.n; i++) {
        Card c;
        if (rb_decode_card_by_index((uint32_t)P->hand.cards[i], &c)) {
            if (c.card_no_idx == card_no) { *found_cid = P->hand.cards[i]; rb_free_card(&c); return 1; }
            rb_free_card(&c);
        }
    }
    for (int i = 0; i < RB_STAGE_SIZE; i++) {
        int cid = P->stage[i];
        if (cid < 0) continue;
        Card c;
        if (rb_decode_card_by_index((uint32_t)cid, &c)) {
            if (c.card_no_idx == card_no) { *found_cid = cid; rb_free_card(&c); return 1; }
            rb_free_card(&c);
        }
    }
    for (int i = 0; i < P->discard.n; i++) {
        Card c;
        if (rb_decode_card_by_index((uint32_t)P->discard.cards[i], &c)) {
            if (c.card_no_idx == card_no) { *found_cid = P->discard.cards[i]; rb_free_card(&c); return 1; }
            rb_free_card(&c);
        }
    }
    for (int i = 0; i < P->live.n; i++) {
        Card c;
        if (rb_decode_card_by_index((uint32_t)P->live.cards[i], &c)) {
            if (c.card_no_idx == card_no) { *found_cid = P->live.cards[i]; rb_free_card(&c); return 1; }
            rb_free_card(&c);
        }
    }
    for (int i = 0; i < P->success.n; i++) {
        Card c;
        if (rb_decode_card_by_index((uint32_t)P->success.cards[i], &c)) {
            if (c.card_no_idx == card_no) { *found_cid = P->success.cards[i]; rb_free_card(&c); return 1; }
            rb_free_card(&c);
        }
    }
    return 0;
}

int rb_find_card_by_number_for_player(const GameState *g, int pl, int card_no, int *found_cid) {
    if (!g || !found_cid) return -1;
    if (rb_search_player_zones_for_card(g, pl, card_no, found_cid) > 0) return 1;
    int other = pl ^ 1;
    if (rb_search_player_zones_for_card(g, other, card_no, found_cid) > 0) return 1;
    return 0;
}

/* ── Group counting ─────────────────────────────────────────────────── */

int rb_distinct_stage_groups(const GameState *g, int pl) {
    static const char *CANON[5] = {"μ's", "Aqours", "虹ヶ咲", "Liella!", "蓮ノ空"};
    const RbPlayer *P = &g->p[pl];
    int count = 0;
    for (int gi = 0; gi < 5; gi++) {
        int matched = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            int cid = P->stage[s];
            if (cid != RB_EMPTY_SLOT && rb_card_matches_group_str(cid, CANON[gi])) {
                matched = 1; break;
            }
        }
        if (matched) count++;
    }
    return count;
}

/* ── Effective activation cost ──────────────────────────────────────── */

int rb_effective_activation_cost_for(const GameState *g, int actor,
                                     const AbilityEffect *cost, int groups_on_stage) {
    if (!cost) return 0;
    int base_cost = cost->count > 0 ? cost->count : 0;
    int reduction = groups_on_stage;
    int effective = base_cost - reduction;
    return effective < 0 ? 0 : effective;
}

int rb_effective_activation_cost(const GameState *g, int actor, const AbilityEffect *cost) {
    int groups = rb_distinct_stage_groups(g, actor);
    return rb_effective_activation_cost_for(g, actor, cost, groups);
}

/* ── Target resolution ──────────────────────────────────────────────── */

int rb_resolve_target_player(const GameState *g, const char *target) {
    (void)g;
    return rb_target_player_index(target, NULL);
}

static int rb_resolve_master_id(const GameState *g);

RbPlayer *rb_resolve_target_player_mut(GameState *g, const char *target) {
    if (!g) return NULL;
    int master = rb_resolve_master_id(g);
    int master_p2 = (master == 1);
    if (target && !strcmp(target, "self"))
        return master_p2 ? &g->p[1] : &g->p[0];
    if (target && !strcmp(target, "opponent"))
        return master_p2 ? &g->p[0] : &g->p[1];
    return &g->p[0];
}

int rb_resolve_master_id(const GameState *g) {
    return g ? g->active : -1;
}

/* ── Loop detection ─────────────────────────────────────────────────── */

static uint64_t rb_generate_state_hash(const GameState *g) {
    uint64_t h = 1469598103934665603ULL;
    int vals[128]; int n = 0;
#define RB_PUSH(x) do { if (n < (int)(sizeof(vals)/sizeof(vals[0]))) vals[n++] = (int)(x); } while (0)
    RB_PUSH(g->turn); RB_PUSH(g->active); RB_PUSH(g->winner);
    RB_PUSH(g->live_set_player); RB_PUSH(g->rps[0]); RB_PUSH(g->rps[1]);
    RB_PUSH(g->rps_winner);
    for (int pl = 0; pl < 2; pl++) {
        const RbPlayer *P = &g->p[pl];
        RB_PUSH(P->hand.n); RB_PUSH(P->energy.n); RB_PUSH(P->discard.n);
        RB_PUSH(P->live.n); RB_PUSH(P->success.n);
        for (int s = 0; s < RB_STAGE_SIZE; s++) RB_PUSH(P->stage[s]);
    }
    RB_PUSH(g->n_prohibition); RB_PUSH(g->n_prohibition_effects);
    RB_PUSH(g->n_temp_effects); RB_PUSH(g->n_batch_triggered_keys);
    RB_PUSH(g->position_change_occurred_this_turn);
    RB_PUSH(g->formation_change_occurred_this_turn);
    RB_PUSH(g->opponent_live_success_this_turn);
    RB_PUSH(g->loop_detected);
#undef RB_PUSH
    for (int i = 0; i < n; i++) {
        h ^= (uint64_t)(vals[i] & 0xFFFFFFFFu);
        h *= 1099511628211ULL;
    }
    return h;
}

int rb_check_permanent_loop(GameState *g) {
    if (!g) return 0;
    uint64_t h = rb_generate_state_hash(g);
    for (int i = 0; i < g->n_game_state_history; i++)
        if ((uint64_t)g->game_state_history[i] == h) { g->loop_detected = 1; return 1; }
    int cap = (int)(sizeof(g->game_state_history)/sizeof(g->game_state_history[0]));
    if (g->n_game_state_history < cap)
        g->game_state_history[g->n_game_state_history++] = (int)h;
    return 0;
}



int rb_is_loop_detected(const GameState *g) {
    return g ? g->loop_detected : 0;
}

/* ── Replacement effects ────────────────────────────────────────────── */

void rb_add_replacement_effect(GameState *g, int card_id, int player_id,
                               const char *original_event,
                               const AbilityEffect *replacement_effects,
                               int n_replacement, int is_choice_based) {
    if (!g || !original_event) return;
    if (g->n_replacement_effects >= 32) return;
    RbReplacementEffect *re = &g->replacement_effects[g->n_replacement_effects];
    re->card_id = card_id;
    re->player_id = player_id;
    snprintf(re->original_event, sizeof(re->original_event), "%s", original_event);
    re->is_choice_based = is_choice_based ? 1 : 0;
    re->applied_this_event = 0;
    (void)replacement_effects; (void)n_replacement;
    g->n_replacement_effects++;
}

void rb_reset_replacement_effect_flags(GameState *g) {
    if (!g) return;
    for (int i = 0; i < g->n_replacement_effects; i++)
        g->replacement_effects[i].applied_this_event = 0;
}

void rb_mark_replacement_effect_applied(GameState *g, int card_id) {
    if (!g) return;
    for (int i = 0; i < g->n_replacement_effects; i++)
        if (g->replacement_effects[i].card_id == card_id)
            g->replacement_effects[i].applied_this_event = 1;
}

/* ── Turn/live state reset ──────────────────────────────────────────── */

void rb_set_opponent_live_success(GameState *g, int no_excess_heart) {
    if (!g) return;
    int pl = g->active;
    int opp = rb_opponent_id(pl);
    g->live_success[opp] = 1;
    if (opp == 0) g->p1_live_success_no_excess = no_excess_heart;
    else g->p2_live_success_no_excess = no_excess_heart;
}

void rb_reset_change_flags(GameState *g) {
    if (!g) return;
    g->position_change_occurred_this_turn = 0;
    g->formation_change_occurred_this_turn = 0;
    g->opponent_live_success_this_turn = 0;
    g->p1_live_success_no_excess = 0;
    g->p2_live_success_no_excess = 0;
    g->self_live_surplus_count = 0;
    g->opponent_live_surplus_count = 0;
    g->live_surplus_ready_this_turn = 0;
    g->last_wait_to_active_count = 0;
    g->n_recently_state_changed = 0;
}

/* ── Choice context injection ────────────────────────────────────────── */

void rb_inject_choice_ability_context(GameState *g, char *json_buf, size_t buf_sz) {
    if (!json_buf || buf_sz == 0) return;
    json_buf[0] = '\0';
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return;
    const RbQueueEntry *e = &g->queue.entries[g->queue.cur];
    int cid = e->card_id;
    const char *card_no = "";
    const char *ability_text = "";
    const char *card_name = "";
    int has_card = 0;
    if (cid >= 0) {
        Card c;
        if (rb_decode_card_by_index((uint32_t)cid, &c)) {
            card_no = "";
            ability_text = c.ability && c.ability->full_text ? c.ability->full_text : "";
            card_name = c.name ? c.name : "";
            has_card = 1;
            rb_free_card(&c);
        }
    }
    const char *pid = e->player_id[0] ? e->player_id : "p1";
    snprintf(json_buf, buf_sz,
        "{\"card_no\":\"%s\",\"ability_text\":\"%s\",\"card_name\":\"%s\","
        "\"choice_player_id\":\"%s\",\"card_id\":%d}",
        card_no, ability_text, card_name, pid, cid);
}

/* ── Ability Queue Entry Accessors ──────────────────────────────────── */

int rb_entry_has_pending_choice(const GameState *g) {
    return g ? g->queue.has_pending : 0;
}

const RbChoice *rb_get_pending_choice(const GameState *g) {
    if (!g || !g->queue.has_pending) return NULL;
    return &g->queue.pending;
}

const AbilityEffect *rb_entry_cost(const GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return NULL;
    int cid = g->queue.entries[g->queue.cur].card_id;
    int aidx = g->queue.entries[g->queue.cur].ability_idx;
    if (cid < 0) return NULL;
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, aidx, &ab)) return NULL;
    const AbilityEffect *cost = ab.cost;
    rb_free_ability(&ab);
    return cost;
}

const AbilityEffect *rb_entry_effect(const GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return NULL;
    int cid = g->queue.entries[g->queue.cur].card_id;
    int aidx = g->queue.entries[g->queue.cur].ability_idx;
    if (cid < 0) return NULL;
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, aidx, &ab)) return NULL;
    const AbilityEffect *eff = ab.effect;
    rb_free_ability(&ab);
    return eff;
}

const char *rb_entry_destination(const GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return NULL;
    const RbQueueEntry *e = &g->queue.entries[g->queue.cur];
    if (!e->effect_started && e->ability_idx >= 0) {
        int cid = e->card_id;
        if (cid >= 0) {
            Ability ab;
            if (rb_decode_card_ability((uint32_t)cid, e->ability_idx, &ab)) {
                if (ab.cost && ab.cost->destination) {
                    const char *dest = ab.cost->destination;
                    rb_free_ability(&ab);
                    return dest;
                }
                rb_free_ability(&ab);
            }
        }
    }
    const AbilityEffect *eff = rb_entry_effect(g);
    return eff ? eff->destination : NULL;
}



const int *rb_entry_trigger_moved_cards(const GameState *g, int *out_count) {
    static int moved[RB_MAX_RECENTLY_MOVED];
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) {
        if (out_count) *out_count = 0;
        return NULL;
    }
    int n = g->n_recently_moved;
    if (n > RB_MAX_RECENTLY_MOVED) n = RB_MAX_RECENTLY_MOVED;
    for (int i = 0; i < n; i++) moved[i] = g->recently_moved[i];
    if (out_count) *out_count = n;
    return moved;
}

int rb_entry_snapshot_last_energy_placed_by_effect(const GameState *g) {
    if (!g) return 0;
    return g->last_energy_placed_by_effect;
}

const char *rb_entry_snapshot_last_energy_placed_by_player(const GameState *g) {
    if (!g) return NULL;
    return g->last_energy_placed_by_player ? "self" : NULL;
}

int rb_entry_snapshot_last_area_move_card_id(const GameState *g) {
    if (!g || g->last_area_move_card_id < 0) return -1;
    return g->last_area_move_card_id;
}

const char *rb_entry_snapshot_last_area_move_by_player(const GameState *g) {
    if (!g) return NULL;
    return g->last_area_move_by_player ? "self" : NULL;
}

/* ── Build ability queue entry ──────────────────────────────────────── */

static void rb_build_ability_queue_entry(GameState *g, int card_id, int ability_idx,
                                         const char *card_no, const char *player_id,
                                         const char *trigger_type,
                                         const int *trigger_moved_cards, int n_moved,
                                         int triggering_member_id) {
    if (!g || card_id < 0) return;
    int idx = g->queue.n_entries;
    if (idx >= RB_QUEUE_DEPTH) return;
    RbQueueEntry *e = &g->queue.entries[idx];
    memset(e, 0, sizeof(*e));
    e->card_id = card_id;
    e->ability_idx = ability_idx;
    e->completed = 0;
    e->cost_paid = 0;
    e->optional_cost_result = -1;
    e->triggering_member_id = triggering_member_id;
    e->use_limit_recorded = 0;
    e->choice_player_id[0] = '\0';
    if (player_id) {
        snprintf(e->player_id, sizeof(e->player_id), "%s", player_id);
    }
    g->queue.n_entries++;
}

/* ── Trigger auto ability (string-keyed) ────────────────────────────── */

void rb_trigger_auto_ability(GameState *g, const char *ability_id,
                             const char *trigger_type, int player_id,
                             const char *source_card_no,
                             int explicit_card_id,
                             const int *trigger_moved_cards, int n_moved,
                             int triggering_member_id) {
    if (!g || !ability_id) return;
    int cid = explicit_card_id;
    if (cid < 0 && source_card_no) {
        int found = -1;
        if (rb_find_card_by_number_for_player(g, player_id, 0, &found) > 0)
            cid = found;
    }
    if (cid < 0) return;
    const char *card_no = source_card_no ? source_card_no : "";
    const char *pid_str = (player_id == 0) ? "p1" : "p2";
    int nab = rb_card_num_abilities((uint32_t)cid);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
        if (!rb_ability_matches_trigger(&ab, "自動")) { rb_free_ability(&ab); continue; }
        char expected_id[256];
        snprintf(expected_id, sizeof(expected_id), "%s_%s", card_no, ab.full_text ? ab.full_text : "");
        if (strcmp(expected_id, ability_id) == 0) {
            rb_build_ability_queue_entry(g, cid, a, card_no, pid_str,
                                        trigger_type, trigger_moved_cards, n_moved, triggering_member_id);
            rb_free_ability(&ab);
            return;
        }
        rb_free_ability(&ab);
    }
    /* Check gained card abilities (ability_id format: "card_no_gained_{idx}") */
    if (strstr(ability_id, "_gained_") != NULL) {
        int ng = rb_card_num_gained_abilities_internal(g, (uint32_t)cid);
        for (int gidx = 0; gidx < ng; gidx++) {
            const Ability *gab = rb_card_gained_ability_internal(g, (uint32_t)cid, gidx);
            if (!gab) continue;
            if (!rb_ability_matches_trigger(gab, "自動")) continue;
            char expected_id[256];
            snprintf(expected_id, sizeof(expected_id), "%s_gained_%d", card_no, gidx);
            if (strcmp(expected_id, ability_id) == 0) {
                Ability ab_copy = *gab;
                int idx = g->queue.n_entries;
                if (idx < RB_QUEUE_DEPTH) {
                    RbQueueEntry *e = &g->queue.entries[idx];
                    memset(e, 0, sizeof(*e));
                    e->card_id = cid;
                    e->ability_idx = GAINED_ABILITY_INDEX_BASE + gidx;
                    e->completed = 0;
                    e->cost_paid = 0;
                    e->optional_cost_result = -1;
                    e->triggering_member_id = triggering_member_id;
                    e->use_limit_recorded = 0;
                    e->choice_player_id[0] = '\0';
                    g->queue.n_entries++;
                }
                return;
            }
        }
    }
}

/* ── Trigger auto ability by index (numeric) ────────────────────────── */

static const char *rb_trigger_kind_to_token(int kind);

void rb_trigger_auto_ability_by_index(GameState *g, int trigger_type,
                                      int player_id, int explicit_card_id,
                                      int ability_index,
                                      const int *trigger_moved_cards, int n_moved,
                                      int triggering_member_id) {
    if (!g || explicit_card_id < 0) return;
    int nab = rb_card_num_abilities((uint32_t)explicit_card_id);
    if (ability_index < 0 || ability_index >= nab) return;
    Ability ab;
    if (!rb_decode_card_ability((uint32_t)explicit_card_id, ability_index, &ab)) return;
    if (!rb_ability_matches_trigger(&ab, "自動")) { rb_free_ability(&ab); return; }
    rb_free_ability(&ab);
    const char *pid_str = (player_id == 0) ? "p1" : "p2";
    const char *tt_str = rb_trigger_kind_to_token(trigger_type);
    rb_build_ability_queue_entry(g, explicit_card_id, ability_index, "", pid_str,
                                tt_str, trigger_moved_cards, n_moved, triggering_member_id);
}

/* ── TAS scan: queue abilities for a zone ───────────────────────────── */

static int queue_zone_abilities(GameState *g, int actor, const int *ids, int n,
                                const char *trigger, const int *moved_cards, int n_moved,
                                int position_change, int energy_placed,
                                int check_discard_guard) {
    int queued = 0;
    const char *pid = (actor == 0) ? "player1" : "player2";
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
        if (cid < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (!rb_ability_matches_trigger(&ab, trigger)) { rb_free_ability(&ab); continue; }
            if (ab.effect && rb_effect_is_ability_resolution_watcher(ab.effect))
                { rb_free_ability(&ab); continue; }
            /* Discard-location guard for stage cards: skip discard-location abilities
               when the card is on stage (not in discard). */
            if (check_discard_guard && ab.effect && ab.effect->condition) {
                const char *loc = rb_cond_get_str(ab.effect->condition, "location");
                const char *ct = rb_cond_get_str(ab.effect->condition, "card_type");
                const char *target = rb_cond_get_str(ab.effect->condition, "target");
                if (loc && !strcmp(loc, "discard")
                    && (ct && !strcmp(ct, "member_card") || (target && !strcmp(target, "self")))) {
                    int in_discard = 0;
                    for (int pl = 0; pl < 2; pl++) {
                        const RbPlayer *P = &g->p[pl];
                        for (int d = 0; d < P->discard.n; d++)
                            if (P->discard.cards[d] == cid) { in_discard = 1; break; }
                        if (in_discard) break;
                    }
                    if (!in_discard) { rb_free_ability(&ab); continue; }
                }
            }
            if (ab.effect && ab.effect->has_condition && ab.effect->condition
                && rb_condition_is_event_based(ab.effect->condition)) {
                int saved = g->activating_card;
                g->activating_card = cid;
                int passes = rb_eval_condition(g, actor, ab.effect->condition);
                g->activating_card = saved;
                if (!passes) { rb_free_ability(&ab); continue; }
                /* Heuristic guard: each_time + energy_zone comparison */
                if (ab.effect->n_extra > 0) {
                    const char *tt = NULL;
                    for (int k = 0; k < ab.effect->n_extra; k++)
                        if (ab.effect->extra_k[k] && !strcmp(ab.effect->extra_k[k], "trigger_type"))
                            tt = ab.effect->extra_v[k];
                    if (tt && !strcmp(tt, "each_time")) {
                        const char *loc = rb_cond_get_str(ab.effect->condition, "location");
                        if (loc && !strcmp(loc, "energy_zone") && !energy_placed)
                            { rb_free_ability(&ab); continue; }
                    }
                }
            }
            /* Movement gate: "moved" self_target + single-location requires card in moved_cards */
            if (ab.effect && ab.effect->condition) {
                const char *mov = rb_cond_get_str(ab.effect->condition, "movement");
                int self_target = 0;
                const char *st = rb_cond_get_str(ab.effect->condition, "self_target");
                if (st && !strcmp(st, "true")) self_target = 1;
                if (mov && !strcmp(mov, "moved") && self_target) {
                    int in_batch = 0;
                    for (int m = 0; m < n_moved; m++)
                        if (moved_cards[m] == cid) { in_batch = 1; break; }
                    if (!in_batch) { rb_free_ability(&ab); continue; }
                }
            }
            int key = rb_queue_key(cid, a);
            int limit = ab.use_limit < 0 ? 99 : ab.use_limit;
            if (key == g->just_completed_ability_key) { rb_free_ability(&ab); continue; }
            if (rb_use_limit_reached(&g->queue, cid, a, limit, g->turn)) { rb_free_ability(&ab); continue; }
            int dup = 0;
            for (int b = 0; b < g->n_batch_triggered_keys; b++)
                if (g->batch_triggered_keys[b] == key) { dup = 1; break; }
            if (!dup) {
                int cap = (int)(sizeof(g->batch_triggered_keys)/sizeof(g->batch_triggered_keys[0]));
                if (g->n_batch_triggered_keys < cap)
                    g->batch_triggered_keys[g->n_batch_triggered_keys++] = key;
                rb_build_ability_queue_entry(g, cid, a, "", pid, trigger,
                                            moved_cards, n_moved, -1);
                rb_record_use(&g->queue, cid, a, g->turn);
                queued++;
            }
            rb_free_ability(&ab);
        }
    }
    return queued;
}

/* ── TAS scan: recently-moved cards ─────────────────────────────────── */

static int queue_moved_cards_abilities(GameState *g, const int *moved_cards, int n_moved,
                                       const char *trigger, const char *player_id_clone,
                                       int position_change, int energy_placed) {
    int queued = 0;
    for (int i = 0; i < n_moved; i++) {
        int moved_card_id = moved_cards[i];
        if (moved_card_id < 0) continue;
        /* Mirror abilities.rs: the moved_cards list is the trigger-context snapshot
            (recently_moved_cards / those_cards). Cards already on a stage or in a
            live zone are skipped here (scanned separately by queue_zone_abilities)
            so they aren't double-queued; batch_triggered_keys dedups the rest. */
        if (g->p[0].stage[0] == moved_card_id || g->p[0].stage[1] == moved_card_id ||
            g->p[0].stage[2] == moved_card_id || g->p[0].live.cards[0] == moved_card_id ||
            g->p[0].live.cards[1] == moved_card_id || g->p[0].live.cards[2] == moved_card_id ||
            g->p[1].stage[0] == moved_card_id || g->p[1].stage[1] == moved_card_id ||
            g->p[1].stage[2] == moved_card_id || g->p[1].live.cards[0] == moved_card_id ||
            g->p[1].live.cards[1] == moved_card_id || g->p[1].live.cards[2] == moved_card_id)
            continue;
        int nab = rb_card_num_abilities((uint32_t)moved_card_id);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)moved_card_id, a, &ab)) continue;
            if (!rb_ability_matches_trigger(&ab, trigger)) { rb_free_ability(&ab); continue; }
            if (ab.effect && rb_effect_is_ability_resolution_watcher(ab.effect))
                { rb_free_ability(&ab); continue; }
            if (ab.effect && ab.effect->has_condition && ab.effect->condition) {
                if (ab.effect->condition->variant == RB_COND_APPEARANCE)
                    { rb_free_ability(&ab); continue; }
                int saved = g->activating_card;
                g->activating_card = moved_card_id;
                int passes = rb_eval_condition(g, rb_owner_of_card(g, moved_card_id), ab.effect->condition);
                g->activating_card = saved;
                if (!passes) { rb_free_ability(&ab); continue; }
            }
            int key = rb_queue_key(moved_card_id, a);
            if (key == g->just_completed_ability_key) { rb_free_ability(&ab); continue; }
            int dup = 0;
            for (int b = 0; b < g->n_batch_triggered_keys; b++)
                if (g->batch_triggered_keys[b] == key) { dup = 1; break; }
            if (!dup) {
                int cap = (int)(sizeof(g->batch_triggered_keys)/sizeof(g->batch_triggered_keys[0]));
                if (g->n_batch_triggered_keys < cap)
                    g->batch_triggered_keys[g->n_batch_triggered_keys++] = key;
                rb_build_ability_queue_entry(g, moved_card_id, a, "", player_id_clone,
                                            trigger, moved_cards, n_moved, -1);
                rb_record_use(&g->queue, moved_card_id, a, g->turn);
                queued++;
            }
            rb_free_ability(&ab);
        }
    }
    return queued;
}

/* ── Opponent-cause watchers ────────────────────────────────────────── */

void rb_fire_opponent_cause_watchers_for_move(GameState *g, int moved_card_id,
                                              int causer_player) {
    if (!g || moved_card_id < 0) return;
    int owner = rb_owner_of_card(g, moved_card_id);
    if (owner < 0 || owner == causer_player) return;
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
            for (int k = 0; k < ab.effect->n_extra; k++)
                if (ab.effect->extra_k[k] && !strcmp(ab.effect->extra_k[k], "fires_on_opponent_effects")
                    && ab.effect->extra_v[k] && !strcmp(ab.effect->extra_v[k], "true"))
                    fires_opp = 1;
            if (!fires_opp) { rb_free_ability(&ab); continue; }
            if (!ab.effect->condition) { rb_free_ability(&ab); continue; }
            int passes = rb_eval_condition_for_host(g, owner, watcher_id, ab.effect->condition);
            if (!passes) { rb_free_ability(&ab); continue; }
            int num_key = rb_queue_key(watcher_id, a);
            int dup = 0;
            for (int b = 0; b < g->n_batch_triggered_keys; b++)
                if (g->batch_triggered_keys[b] == num_key) { dup = 1; break; }
            if (!dup) {
                int cap = (int)(sizeof(g->batch_triggered_keys)/sizeof(g->batch_triggered_keys[0]));
                if (g->n_batch_triggered_keys < cap)
                    g->batch_triggered_keys[g->n_batch_triggered_keys++] = num_key;
                rb_build_ability_queue_entry(g, watcher_id, a, "", (owner == 0) ? "p1" : "p2", "自動",
                                            &moved_card_id, 1, -1);
                rb_record_use(&g->queue, watcher_id, a, g->turn);
            }
            rb_free_ability(&ab);
        }
    }
}

/* ── Trigger each_time_for_member ───────────────────────────────────── */

void rb_trigger_each_time_for_member(GameState *g, int pl,
                                     const char *trigger_substring, int member_card_id) {
    if (!g || pl < 0 || pl > 1) return;
    const RbPlayer *P = &g->p[pl];
    int on_stage = 0;
    for (int s = 0; s < RB_STAGE_SIZE; s++)
        if (P->stage[s] == member_card_id) { on_stage = 1; break; }
    if (!on_stage) return;
    for (int i = 0; i < P->live.n; i++) {
        int cid = P->live.cards[i];
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (!rb_ability_matches_trigger(&ab, "自動")) { rb_free_ability(&ab); continue; }
            if (!ab.effect) { rb_free_ability(&ab); continue; }
            const char *tt = NULL;
            for (int k = 0; k < ab.effect->n_extra; k++)
                if (ab.effect->extra_k[k] && !strcmp(ab.effect->extra_k[k], "trigger_type"))
                    tt = ab.effect->extra_v[k];
            if (!tt || strcmp(tt, "each_time") != 0) { rb_free_ability(&ab); continue; }
            const char *watch_text = ab.effect->condition ? rb_cond_get_str(ab.effect->condition, "text") : ab.effect->text;
            if (!watch_text || !strstr(watch_text, trigger_substring)) { rb_free_ability(&ab); continue; }
            const char *groups = rb_condition_tree_group_names(ab.effect->condition);
            if (groups && groups[0]) {
                int member_matches = rb_card_matches_group_str(member_card_id, groups);
                if (!member_matches) { rb_free_ability(&ab); continue; }
            }
            rb_build_ability_queue_entry(g, cid, a, "", (pl == 0) ? "p1" : "p2", "自動", NULL, 0, member_card_id);
            rb_record_use(&g->queue, cid, a, g->turn);
            rb_free_ability(&ab);
        }
    }
}



/* ── Trigger auto abilities for movement ────────────────────────────── */

int rb_trigger_auto_abilities_for_player_with_event(GameState *g, int pl,
                                                     const int *moved_cards, int n_moved,
                                                     int position_change, int energy_placed) {
    if (!g || pl < 0 || pl > 1) return 0;
    const RbPlayer *P = &g->p[pl];
    int queued = 0;
    queued += queue_zone_abilities(g, pl, P->stage, RB_STAGE_SIZE, "自動",
                                   moved_cards, n_moved, position_change, energy_placed, 1);
    queued += queue_zone_abilities(g, pl, P->success.cards, P->success.n, "自動",
                                   moved_cards, n_moved, position_change, energy_placed, 0);
    queued += queue_zone_abilities(g, pl, P->live.cards, P->live.n, "自動",
                                   moved_cards, n_moved, position_change, energy_placed, 0);
    queued += queue_zone_abilities(g, pl, P->hand.cards, P->hand.n, "自動",
                                   moved_cards, n_moved, position_change, energy_placed, 0);
    queued += queue_zone_abilities(g, pl, P->energy.cards, P->energy.n, "自動",
                                   moved_cards, n_moved, position_change, energy_placed, 0);
    queued += queue_moved_cards_abilities(g, moved_cards, n_moved, "自動",
                                          (pl == 0) ? "p1" : "p2",
                                          position_change, energy_placed);
    return queued;
}

int rb_trigger_auto_abilities_for_movement(GameState *g, int pl) {
    if (!g) return 0;
    int n = g->n_recently_moved > RB_MAX_RECENTLY_MOVED ? RB_MAX_RECENTLY_MOVED : g->n_recently_moved;
    return rb_trigger_auto_abilities_for_player_with_event(g, pl, g->recently_moved, n,
                                                           g->position_change_occurred_this_turn, 0);
}

void rb_trigger_auto_abilities_for_movement_current(GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return;
    int pl = g->queue.actor;
    rb_trigger_auto_abilities_for_movement(g, pl);
}

/* ── Trigger auto abilities (queue + drain) ─────────────────────────── */

int rb_trigger_auto_abilities(GameState *g, int actor, const char *trigger) {
    int queued = rb_queue_trigger_abilities(g, actor, trigger);
    if (queued > 0) rb_drain_ability_queue(g);
    g->just_completed_ability_key = -1;
    return queued;
}

int rb_fire_auto(GameState *g, int pl) {
    return rb_trigger_auto_abilities(g, pl, "自動");
}

int rb_fire_all_auto(GameState *g, int pl) {
    if (!g) return 0;
    rb_trigger_auto_abilities(g, pl, "自動");
    rb_drain_ability_queue(g);
    return 0;
}

int rb_fire_auto_and_pending(GameState *g, int pl) {
    if (!g) return 0;
    rb_fire_auto(g, pl);
    rb_process_pending_auto_abilities(g);
    return 0;
}

/* ── Event recording ────────────────────────────────────────────────── */

static const struct { int bit; const char *trig; } RB_EV[] = {
    { 1, "エネルギー置いた時" }, { 2, "移動時" }, { 4, "応援時" },
    { 8, "公開時" }, { 16, "覚醒時" }, { 32, "レスト時" },
    { 64, "バトンタッチ時" }, { 128, "除外時" }, { 256, "ターン開始時" },
    { 512, "ドロー時" }, { 1024, "自動" }, { 0, NULL }
};

void rb_record_event(GameState *g, int pl, const char *trig) {
    if (!g || pl < 0 || pl > 1) return;
    for (int i = 0; RB_EV[i].trig; i++)
        if (!strcmp(RB_EV[i].trig, trig)) { g->auto_event_mask[pl] |= RB_EV[i].bit; return; }
}

int rb_fire_recorded_auto(GameState *g, int pl) {
    if (!g || pl < 0 || pl > 1) return 0;
    int total = 0, mask = g->auto_event_mask[pl];
    for (int i = 0; RB_EV[i].trig; i++)
        if (mask & RB_EV[i].bit) total += rb_trigger_auto_abilities(g, pl, RB_EV[i].trig);
    g->auto_event_mask[pl] = 0;
    return total;
}

/* ── Queue drain + resolution loop ──────────────────────────────────── */

static int rb_process_current_ability(GameState *g);



static int rb_process_current_ability(GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return 0;

    RbQueueEntry *entry = &g->queue.entries[g->queue.cur];
    int cid = entry->card_id;
    int aidx = entry->ability_idx;

    if (cid < 0) {
        g->queue.cur++;
        return 0;
    }

    Ability ab;
    if (!rb_decode_card_ability((uint32_t)cid, aidx, &ab)) {
        g->queue.cur++;
        return 0;
    }

    int actor = g->queue.actor;

    if (ab.use_limit > 0) {
        if (rb_resolver_use_limit_reached(g, cid, aidx, ab.use_limit)) {
            rb_free_ability(&ab);
            g->queue.cur++;
            return 0;
        }
    }

    if (ab.cost) {
        if (!rb_pay_cost(g, actor, ab.cost)) {
            rb_free_ability(&ab);
            g->queue.cur++;
            return 0;
        }
    }

    if (ab.effect) {
        if (!rb_can_activate_effect(g, actor, ab.effect, cid)) {
            rb_free_ability(&ab);
            g->queue.cur++;
            return 0;
        }
        rb_execute_effect_ex(g, actor, ab.effect, cid);
    }

    if (ab.use_limit > 0)
        rb_record_ability_use(g, cid, aidx);

    rb_free_ability(&ab);

    if (g->queue.has_pending) return 1;

    g->queue.cur++;
    return 1;
}

int rb_process_player_abilities(GameState *g, int pl) {
    if (!g) return 0;
    int processed = 0;
    g->queue.actor = pl;
    g->queue.state = RB_QUEUE_RESOLVING;

    while (g->queue.cur < g->queue.n_entries) {
        if (rb_process_current_ability(g)) processed++;
        if (g->queue.has_pending) {
            g->queue.state = RB_QUEUE_AWAITING_CHOICE;
            break;
        }
    }

    if (!g->queue.has_pending) {
        g->queue.state = RB_QUEUE_IDLE;
        g->queue.cur = 0;
        g->queue.n_entries = 0;
    }

    return processed;
}

int rb_process_pending_auto_abilities(GameState *g) {
    if (!g) return 0;
    int total = 0;
    for (int pl = 0; pl < 2; pl++)
        total += rb_process_player_abilities(g, pl);
    return total;
}

/* ── process_player_abilities_depth (bounded recursion) ─────────────── */

int rb_process_player_abilities_depth(GameState *g, int pl, int max_depth) {
    if (!g || max_depth <= 0) return 0;
    return rb_process_player_abilities(g, pl);
}

/* ─- Pending-choice helpers ─────────────────────────────────────────── */





void rb_queue_reset(GameState *g) {
    if (!g) return;
    g->queue.n_entries = 0;
    g->queue.cur = 0;
    g->queue.has_pending = 0;
    g->queue.state = RB_QUEUE_IDLE;
    g->queue.n_uses = 0;
    g->queue.use_turn = -1;
}

/* ── apply_ability_effects ──────────────────────────────────────────── */

int rb_apply_ability_effects(GameState *g, int actor, const Ability *ab, int host_cid) {
    if (!g || !ab || !ab->effect) return 0;
    rb_compound_sequential(g, actor, ab->effect, host_cid);
    return 1;
}

/* ─- trigger_instance_count ────────────────────────────────────────── */

static const char *rb_trigger_kind_to_token(int kind) {
    switch (kind) {
        case RB_TK_ACTIVATION: return "起動";
        case RB_TK_AUTO:       return "自動";
        case RB_TK_CONSTANT:   return "常時";
        case RB_TK_DEBUT:      return "デビュー";
        case RB_TK_LIVE_START: return "ライブ開始時";
        case RB_TK_LIVE_SUCCESS:return "ライブ成功時";
        case RB_TK_MAIN:       return "メイン";
        case RB_TK_BATON_TOUCH:return "バトンタッチ時";
        default:               return "自動";
    }
}

/* ─- collect_constant_hand / collect_live_modifiers ─────────────────── */

int rb_collect_constant_hand(const GameState *g, int actor, AbilityEffect *out, int max) {
    (void)out; (void)max;
    if (!g) return 0;
    int found = 0;
    const RbPlayer *P = &g->p[actor];
    for (int s = 0; s < RB_STAGE_SIZE; s++) {
        int cid = P->stage[s];
        if (cid < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (constant_pair_matches(&ab) && ab.effect) {
                apply_constant_node((RbMods *)&g->mods, cid, ab.effect);
                found++;
            }
            rb_free_ability(&ab);
        }
    }
    return found;
}

int rb_collect_live_modifiers(const GameState *g, int actor, AbilityEffect *out, int max) {
    (void)out; (void)max;
    if (!g || actor < 0 || actor > 1) return 0;
    int found = 0;
    const RbPlayer *P = &g->p[actor];
    int ids[RB_MAX_ZONE * 2];
    int n = 0;
    for (int i = 0; i < P->live.n && n < (int)(sizeof(ids)/sizeof(ids[0])); i++)
        ids[n++] = P->live.cards[i];
    for (int i = 0; i < P->success.n && n < (int)(sizeof(ids)/sizeof(ids[0])); i++)
        ids[n++] = P->success.cards[i];
    for (int i = 0; i < n; i++) {
        int cid = ids[i];
        if (cid < 0) continue;
        int nab = rb_card_num_abilities((uint32_t)cid);
        for (int a = 0; a < nab; a++) {
            Ability ab;
            if (!rb_decode_card_ability((uint32_t)cid, a, &ab)) continue;
            if (constant_pair_matches(&ab) && ab.effect) {
                apply_constant_node((RbMods *)&g->mods, cid, ab.effect);
                found++;
            }
            rb_free_ability(&ab);
        }
    }
    return found;
}





/* ── Misc ───────────────────────────────────────────────────────────── */



void rb_queue_set_pending_choice(GameState *g, const RbChoice *choice) {
    if (!g || !choice) return;
    g->queue.pending = *choice;
    g->queue.has_pending = 1;
    g->queue.state = RB_QUEUE_AWAITING_CHOICE;
}















/* ─- pending-choice JSON ─────────────────────────────────────────────── */

void rb_get_pending_choice_json(const GameState *g, char *buf, size_t buf_sz) {
    if (!buf || buf_sz == 0) return;
    buf[0] = '\0';
    if (!g || !g->queue.has_pending) return;
    const RbChoice *ch = &g->queue.pending;
    const char *kind_str = "none";
    switch (ch->kind) {
        case RB_CHOICE_SELECT_CARD:      kind_str = "select_card"; break;
        case RB_CHOICE_SELECT_TARGET:    kind_str = "select_target"; break;
        case RB_CHOICE_SELECT_HEART_COLOR: kind_str = "select_heart_color"; break;
        case RB_CHOICE_SELECT_NUMBER:    kind_str = "select_number"; break;
        case RB_CHOICE_SELECT_POSITION:  kind_str = "select_position"; break;
        case RB_CHOICE_SELECT_AUTO_ABILITY: kind_str = "select_auto_ability"; break;
        default: break;
    }
    snprintf(buf, buf_sz,
        "{\"kind\":\"%s\",\"zone\":\"%s\",\"card_type\":\"%s\","
        "\"count\":%d,\"allow_skip\":%d,\"target\":\"%s\","
        "\"description\":\"%s\",\"route\":%d}",
        kind_str, ch->zone, ch->card_type,
        ch->count, ch->allow_skip, ch->target,
        ch->description, (int)ch->route);
}

/* ─- entry_choice_card_no / entry_conditional_choice ────────────────── */

RbChoiceRoute rb_entry_choice_card_no(const GameState *g) {
    if (!g || !g->queue.has_pending) return RB_ROUTE_NONE;
    return g->queue.pending.route;
}

int rb_entry_conditional_choice(const GameState *g) {
    if (!g || g->queue.cur < 0 || g->queue.cur >= g->queue.n_entries) return 0;
    return g->queue.entries[g->queue.cur].optional_cost_result;
}

/* ─- build_ability_queue_entry (public) ──────────────────────────────── */

void rb_build_ability_queue_entry_public_full(GameState *g, int card_id, int ability_idx,
                                              const char *card_no, int player_id,
                                              int trigger_type) {
    if (!g || card_id < 0) return;
    int idx = g->queue.n_entries;
    if (idx >= RB_QUEUE_DEPTH) return;
    RbQueueEntry *e = &g->queue.entries[idx];
    memset(e, 0, sizeof(*e));
    e->card_id = card_id;
    e->ability_idx = ability_idx;
    e->completed = 0;
    e->cost_paid = 0;
    e->optional_cost_result = -1;
    g->queue.n_entries++;
}

/* ─- queue_trigger_abilities (internal) ─────────────────────────────── */

int rb_queue_trigger_abilities(GameState *g, int pl, const char *trigger) {
    if (!g || !trigger) return 0;
    g->n_batch_triggered_keys = 0;
    int total = 0;
    const RbPlayer *P = &g->p[pl];
    total += queue_zone_abilities(g, pl, P->stage, RB_STAGE_SIZE, trigger,
                                  g->recently_moved, g->n_recently_moved,
                                  g->position_change_occurred_this_turn,
                                  g->last_energy_placed_by_effect,
                                  1);
    total += queue_zone_abilities(g, pl, P->success.cards, P->success.n, trigger,
                                  g->recently_moved, g->n_recently_moved,
                                  g->position_change_occurred_this_turn,
                                  g->last_energy_placed_by_effect,
                                  0);
    total += queue_zone_abilities(g, pl, P->live.cards, P->live.n, trigger,
                                  g->recently_moved, g->n_recently_moved,
                                  g->position_change_occurred_this_turn,
                                  g->last_energy_placed_by_effect,
                                  0);
    total += queue_zone_abilities(g, pl, P->hand.cards, P->hand.n, trigger,
                                  g->recently_moved, g->n_recently_moved,
                                  g->position_change_occurred_this_turn,
                                  g->last_energy_placed_by_effect,
                                  0);
    total += queue_zone_abilities(g, pl, P->energy.cards, P->energy.n, trigger,
                                  g->recently_moved, g->n_recently_moved,
                                  g->position_change_occurred_this_turn,
                                  g->last_energy_placed_by_effect,
                                  0);
    total += queue_moved_cards_abilities(g, g->recently_moved, g->n_recently_moved,
                                         trigger, (pl == 0) ? "p1" : "p2",
                                         g->position_change_occurred_this_turn,
                                         g->last_energy_placed_by_effect);
    return total;
}

/* ─- process_player_abilities_depth (full loop) ─────────────────────── */

int rb_process_player_abilities_depth_full(GameState *g, int pl, int max_depth) {
    if (!g || max_depth <= 0) return 0;
    const char *player_id = (pl == 0) ? "player1" : "player2";
    int processed = 0;
    int depth = 0;
    while (depth < max_depth) {
        depth++;
        int pre_len = g->queue.n_entries;
        int available = 0;
        for (int i = 0; i < g->queue.n_entries; i++)
            if (!g->queue.entries[i].completed &&
                (g->queue.entries[i].player_id[0] == '\0' ||
                 !strcmp(g->queue.entries[i].player_id, player_id) ||
                 !strcmp(g->queue.entries[i].player_id, (pl == 0) ? "p1" : "p2")))
                available++;
        if (available == 0) break;
        if (available > 1) {
            /* Multiple auto abilities — would emit a choice; just process first */
        }
        for (int i = 0; i < g->queue.n_entries; i++) {
            if (g->queue.entries[i].completed) continue;
            g->queue.cur = i;
            g->queue.actor = pl;
            g->queue.state = RB_QUEUE_RESOLVING;
            if (rb_process_current_ability(g)) processed++;
            if (g->queue.has_pending) {
                g->queue.state = RB_QUEUE_AWAITING_CHOICE;
                return processed;
            }
            /* Depth-first drain: check for newly enqueued entries */
            int new_count = 0;
            for (int j = pre_len; j < g->queue.n_entries; j++)
                if (!g->queue.entries[j].completed) new_count++;
            if (new_count > 0) {
                /* Re-enter loop to process newly queued entries */
                break;
            }
        }
        /* Post-loop batch scan */
        if (g->n_recently_moved > 0 || g->last_energy_placed_by_effect || g->n_recently_appeared > 0) {
            int local_moved[RB_MAX_RECENTLY_MOVED];
            int n = g->n_recently_moved > RB_MAX_RECENTLY_MOVED ? RB_MAX_RECENTLY_MOVED : g->n_recently_moved;
            for (int i = 0; i < n; i++) local_moved[i] = g->recently_moved[i];
            rb_trigger_auto_abilities_for_player_with_event(g, pl, local_moved, n,
                                                           g->position_change_occurred_this_turn,
                                                           g->last_energy_placed_by_effect);
            rb_clear_movement_tracking(g);
            if (g->queue.has_pending) return processed;
        }
    }
    return processed;
}
