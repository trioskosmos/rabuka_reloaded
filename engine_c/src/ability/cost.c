/* cost.c — ability cost payment.
   Mirror engine/src/ability/cost.rs (pay_deferred_costs, validate_cost,
   pay_cost, pay_cost_inner, handle_optional_cost_payment, get_change_state_
   candidates, has_skip_prompt).

   The C decoder stores an ability's cost as an AbilityEffect. Its `action`
   is the gate: "sequential"/"sequential_cost" wraps sub-costs in child[],
   while a single cost (pay_energy / change_state / move_cards / ...) carries
   its own `action`. This file recurses through sequential costs and pays the
   leaf costs. Optional costs are auto-skipped in the headless model (the host
   would have offered a pay/skip prompt); this matches cost.rs' skip path. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

/* local bag helpers (RbBag is a plain int array) */
static void bag_push(RbBag *b, int c) { if (b->n < RB_MAX_ZONE) b->cards[b->n++] = c; }
static int  bag_pop(RbBag *b) { return b->n > 0 ? b->cards[--b->n] : -1; }

/* Is cost-component `e` an energy payment? */
static int cost_is_energy(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "pay_energy")) return 1;
    const char *t = e->target;
    return t && strstr(t, "energy") != NULL;
}
/* Is cost-component `e` a change-state (put a member to wait) payment? */
static int cost_is_change_state(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "change_state")) return 1;
    const char *t = e->target;
    return t && strstr(t, "wait") != NULL;
}

/* Mirror cost.rs: get_change_state_candidates — list stage positions whose
   member can be put into wait state to satisfy a change_state cost.
   Fills out_positions (cap max) and returns the count. Rust filters by
   orientation == "active"; the C model tracks that with stage_wait==0. */
int rb_get_change_state_candidates(const GameState *g, int actor,
                                    int *out_positions, int max) {
    const RbPlayer *P = &g->p[actor];
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE && n < max; i++) {
        if (P->stage[i] != RB_EMPTY_SLOT && !P->stage_wait[i])
            out_positions[n++] = i;
    }
    return n;
}

/* Mirror cost.rs: has_skip_prompt */
static int cost_has_skip_prompt(const AbilityEffect *cost) {
    if (!cost) return 0;
    if (cost_is_energy(cost)) return 1;            /* pay_energy w/o any_number */
    if (cost_is_change_state(cost)) return cost->is_optional;
    return 0;
}

/* Resolve a source-zone wire name to the player's bag (stage handled separately). */
static RbBag *cost_source_bag(RbPlayer *P, const char *src) {
    if (!src) return NULL;
    if (!strcmp(src, "hand")) return &P->hand;
    if (!strcmp(src, "deck") || !strcmp(src, "deck_top")) return &P->deck;
    if (!strcmp(src, "waitroom") || !strcmp(src, "discard")) return &P->discard;
    if (!strcmp(src, "energy")) return &P->energy;
    return NULL;
}
static int cost_count_in_source(const GameState *g, int actor, const char *src) {
    const RbPlayer *P = &g->p[actor];
    if (!src) return 0;
    if (!strcmp(src, "stage")) {
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] != RB_EMPTY_SLOT) n++;
        return n;
    }
    RbBag *b = cost_source_bag((RbPlayer *)P, src);
    return b ? b->n : 0;
}
/* Move up to `count` cards from a source zone to the actor's discard (Rust
   pay_cost_move_cards execution path for a cost). Returns cards moved. */
static int cost_move_from_source(GameState *g, int actor, const char *src, int count) {
    RbPlayer *P = &g->p[actor];
    int moved = 0;
    if (!src) return 0;
    if (!strcmp(src, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE && moved < count; i++) {
            if (P->stage[i] != RB_EMPTY_SLOT) {
                int cid = P->stage[i];
                P->stage[i] = RB_EMPTY_SLOT; P->stage_wait[i] = 0;
                bag_push(&P->discard, cid); moved++;
            }
        }
        g->mods.last_cost_discard_count = moved;
        return moved;
    }
    RbBag *b = cost_source_bag(P, src);
    if (!b) return 0;
    while (moved < count && b->n > 0) {
        int cid = bag_pop(b);
        bag_push(&P->discard, cid);
        moved++;
    }
    g->mods.last_cost_discard_count = moved;
    return moved;
}

static int cost_is_sequential(const AbilityEffect *e) {
    return e->action &&
           (!strcmp(e->action, "sequential") || !strcmp(e->action, "sequential_cost"));
}

/* Mirror cost.rs:validate_cost (single cost). Returns 1 if payable. */
static int validate_one(const GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    if (cost_is_sequential(cost)) {
        for (int i = 0; i < cost->n_child; i++)
            if (!validate_one(g, actor, cost->child[i])) return 0;
        return 1;
    }
    if (cost_is_energy(cost)) {
        int need = cost->count > 0 ? cost->count : 1;
        return g->p[actor].energy_active >= need;
    }
    if (cost_is_change_state(cost)) {
        const char *sc = NULL;
        for (int i = 0; i < cost->n_extra; i++)
            if (cost->extra_k[i] && !strcmp(cost->extra_k[i], "state_change")) sc = cost->extra_v[i];
        if (sc && !strcmp(sc, "wait")) {
            int pos[RB_STAGE_SIZE];
            return rb_get_change_state_candidates(g, actor, pos, RB_STAGE_SIZE) > 0;
        }
        return 1;
    }
    if (cost->action && !strcmp(cost->action, "move_cards")) {
        const char *src = cost->source ? cost->source : "";
        int count = cost->count > 0 ? cost->count : 1;
        return cost_count_in_source(g, actor, src) >= count;
    }
    return 1; /* pay/unconditional costs always payable */
}

/* Mirror cost.rs:validate_cost */
int rb_validate_cost(const GameState *g, int actor, const AbilityEffect *cost) {
    return validate_one(g, actor, cost);
}

/* Mirror cost.rs:pay_cost_inner (single cost). Returns 1 on success. */
static int pay_one(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    if (cost_is_sequential(cost)) {
        for (int i = 0; i < cost->n_child; i++)
            if (!pay_one(g, actor, cost->child[i])) return 0;
        return 1;
    }
    /* Optional costs are auto-skipped headless (skip path in cost.rs). */
    if (cost->is_optional) return 1;

    if (cost_is_energy(cost)) {
        int amt = cost->count > 0 ? cost->count : 1;
        RbPlayer *P = &g->p[actor];
        P->energy_active -= amt;
        if (P->energy_active < 0) P->energy_active = 0;
        rb_recalc_constants(g);
        return 1;
    }
    if (cost_is_change_state(cost)) {
        const char *sc = NULL;
        for (int i = 0; i < cost->n_extra; i++)
            if (cost->extra_k[i] && !strcmp(cost->extra_k[i], "state_change")) sc = cost->extra_v[i];
        if (sc && !strcmp(sc, "wait")) {
            int pos[RB_STAGE_SIZE];
            int n = rb_get_change_state_candidates(g, actor, pos, RB_STAGE_SIZE);
            if (n > 0) g->p[actor].stage_wait[pos[0]] = 1;
        }
        return 1;
    }
    if (cost->action && !strcmp(cost->action, "move_cards")) {
        const char *src = cost->source ? cost->source : "";
        int count = cost->count > 0 ? cost->count : 1;
        cost_move_from_source(g, actor, src, count);
        return 1;
    }
    return 1;
}

/* Mirror cost.rs:pay_cost */
int rb_pay_cost(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    if (!rb_validate_cost(g, actor, cost)) return 0;
    return pay_one(g, actor, cost);
}

/* Mirror cost.rs:pay_deferred_costs — settle deferred (post-effect) costs. */
int rb_pay_deferred_costs(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    return rb_pay_cost(g, actor, cost);
}

/* Mirror cost.rs:handle_optional_cost_payment — pay the optional cost if the
   player chose to (pay != 0); skip otherwise. Returns the chosen flag. */
int rb_handle_optional_cost_payment(GameState *g, int actor, const AbilityEffect *cost, int pay) {
    if (pay && cost) rb_pay_cost(g, actor, cost);
    return pay;
}

/* Mirror cost.rs:has_skip_prompt — does this cost carry a "may skip" prompt? */
int rb_cost_has_skip_prompt(const AbilityEffect *cost) {
    if (!cost) return 0;
    return cost_has_skip_prompt(cost);
}

/* ── Play-cost reduction (mirror engine/src/ability/util.rs
//    compute_play_cost / calculate_play_cost_reduction / scan_abilities_for_
//    cost_reduction / per_unit_cost_reduction / play_cost_reduction_matches).
//    These compute the deploy cost of a member card from hand: base cost minus
//    stage/live-zone reduction auras plus success-zone increases, with an
//    optional constant set-override ("コストはNになる"). ── */

/* decoded-effect field reader (mirrors util.rs `*_any()` getters) */
static const char *cr_eff_extra(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
static int cr_eff_int(const AbilityEffect *e, const char *k, int dflt) {
    const char *v = cr_eff_extra(e, k);
    if (!v || !*v) return dflt;
    return atoi(v);
}

/* Mirror util.rs::find_modify_cost — find a ModifyCost sub-effect with matching
   operation/location, recursing into sequential compounds. */
static const AbilityEffect *cr_find_modify_cost(const AbilityEffect *e,
                                                const char *op, const char *loc) {
    if (!e) return NULL;
    if (e->action && !strcmp(e->action, "modify_cost")) {
        const char *eop = cr_eff_extra(e, "operation");
        const char *eloc = cr_eff_extra(e, "location");
        if ((!op || (eop && !strcmp(eop, op))) &&
            (!loc || (eloc && !strcmp(eloc, loc))))
            return e;
    }
    if (e->action && (!strcmp(e->action, "sequential") ||
                     !strcmp(e->action, "sequential_cost"))) {
        for (int i = 0; i < e->n_child; i++) {
            const AbilityEffect *f = cr_find_modify_cost(e->child[i], op, loc);
            if (f) return f;
        }
    }
    return NULL;
}

/* Mirror util.rs::play_cost_reduction_matches */
static int cr_reduction_matches(const AbilityEffect *e, int card_id, const Card *card) {
    const char *gn = cr_eff_extra(e, "group_names");
    if (gn && !rb_card_matches_group_str(card_id, gn)) return 0;
    const char *limit = cr_eff_extra(e, "cost_limit");
    if (limit && card->cost != atoi(limit)) return 0;
    if (!rb_cost_threshold_met(card, e)) return 0;
    const char *ct = cr_eff_extra(e, "card_type");
    if (ct && strcmp(ct, "member_card") != 0) return 0;
    const char *af = cr_eff_extra(e, "ability_filter");
    if (af && !strcmp(af, "no_ability") && rb_card_num_abilities((uint32_t)card_id) > 0)
        return 0;
    return 1;
}

/* Mirror util.rs::per_unit_cost_reduction */
static int cr_per_unit_reduction(const AbilityEffect *e, const GameState *g,
                                 int actor, int hand_count) {
    const char *pul  = cr_eff_extra(e, "per_unit_type");
    const char *loc  = cr_eff_extra(e, "location");
    const char *zone = pul ? pul : (loc ? loc : "hand");
    int raw_count;
    if (!strcmp(zone, "stage") && cr_eff_extra(e, "group_names")) {
        const char *gn = cr_eff_extra(e, "group_names");
        const RbPlayer *P = &g->p[actor];
        int n = 0;
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            int id = P->stage[s];
            if (id != RB_EMPTY_SLOT && rb_card_matches_group_str(id, gn)) n++;
        }
        raw_count = n;
    } else {
        raw_count = hand_count;
    }
    int per_unit_count = e->per_unit_count > 0 ? e->per_unit_count : 1;
    const char *ex = cr_eff_extra(e, "exclude_self");
    int exclude_self = ex && !strcmp(ex, "true");
    int effective = exclude_self ? (raw_count > 0 ? raw_count - 1 : 0) : raw_count;
    int value = cr_eff_int(e, "value", 1);
    return (effective / per_unit_count) * value;
}

/* Mirror util.rs::scan_abilities_for_cost_reduction — scan one ModifyCost
   (subtract, hand) effect on a source card. Returns reduction (>=0) or -1. */
static int cr_scan_one_effect(const AbilityEffect *eff, int target_id,
                              const Card *target_card, const GameState *g,
                              int actor, int hand_count, int hand_guard) {
    if (!eff || !eff->action || strcmp(eff->action, "modify_cost") != 0) return -1;
    const char *op  = cr_eff_extra(eff, "operation");
    const char *loc = cr_eff_extra(eff, "location");
    if (!(op && !strcmp(op, "subtract"))) return -1;
    if (!(loc && !strcmp(loc, "hand"))) return -1;
    if (hand_guard && eff->condition) {
        for (uint32_t i = 0; i < eff->condition->n_fields; i++) {
            if (eff->condition->fields[i].key &&
                !strcmp(eff->condition->fields[i].key, "location") &&
                eff->condition->fields[i].v.tag == RB_TAG_STR &&
                eff->condition->fields[i].v.s &&
                !strcmp(eff->condition->fields[i].v.s, "hand"))
                return -1;
        }
    }
    const char *gn = cr_eff_extra(eff, "group_names");
    if (gn && !rb_card_matches_group_str(target_id, gn)) return -1;
    const char *limit = cr_eff_extra(eff, "cost_limit");
    if (limit && target_card->cost != atoi(limit)) return -1;
    if (!rb_cost_threshold_met(target_card, eff)) return -1;
    const char *ct = cr_eff_extra(eff, "card_type");
    if (ct && strcmp(ct, "member_card") != 0) return -1;
    const char *af = cr_eff_extra(eff, "ability_filter");
    if (af && !strcmp(af, "no_ability") &&
        rb_card_num_abilities((uint32_t)target_id) > 0)
        return -1;
    if (eff->per_unit) return cr_per_unit_reduction(eff, g, actor, hand_count);
    return cr_eff_int(eff, "value", 1);
}

/* forward decl */
static int cr_scan_one_effect_source(int src_card_id, int target_id,
                                     const Card *target_card, const GameState *g,
                                     int actor, int hand_count, int hand_guard);

/* Mirror util.rs::calculate_play_cost_reduction */
static int cr_calc_reduction(const GameState *g, int actor, int card_id,
                             const Card *card) {
    int cost_reduction = 0;
    int hand_count = g->p[actor].hand.n + 1;
    int n = rb_card_num_abilities((uint32_t)card_id);
    /* 1. self reduction */
    for (int ai = 0; ai < n; ai++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, ai, &ab)) continue;
        if (ab.effect) {
            const AbilityEffect *mc = cr_find_modify_cost(ab.effect, "subtract", "hand");
            if (mc && cr_reduction_matches(mc, card_id, card)) {
                if (mc->per_unit)
                    cost_reduction = cr_per_unit_reduction(mc, g, actor, hand_count);
                else {
                    int v = cr_eff_int(mc, "value", 1);
                    if (v > cost_reduction) cost_reduction = v;
                }
            }
        }
        rb_free_ability(&ab);
    }
    /* 2. stage auras (stack) */
    {
        const RbPlayer *P = &g->p[actor];
        for (int s = 0; s < RB_STAGE_SIZE; s++) {
            int id = P->stage[s];
            if (id == RB_EMPTY_SLOT) continue;
            int r = cr_scan_one_effect_source(id, card_id, card, g, actor,
                                             hand_count, 1);
            if (r >= 0) cost_reduction += r;
        }
    }
    /* 3. success live-zone auras (max, only if nothing yet) */
    if (cost_reduction == 0) {
        const RbPlayer *P = &g->p[actor];
        for (int i = 0; i < P->success.n; i++) {
            int id = P->success.cards[i];
            int r = cr_scan_one_effect_source(id, card_id, card, g, actor,
                                             hand_count, 0);
            if (r >= 0) { if (r > cost_reduction) cost_reduction = r; break; }
        }
    }
    return cost_reduction;
}

/* Scan all abilities of a source card for a qualifying cost-reduction effect. */
static int cr_scan_one_effect_source(int src_card_id, int target_id,
                                     const Card *target_card, const GameState *g,
                                     int actor, int hand_count, int hand_guard) {
    int n = rb_card_num_abilities((uint32_t)src_card_id);
    for (int ai = 0; ai < n; ai++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)src_card_id, ai, &ab)) continue;
        int r = -1;
        if (ab.effect)
            r = cr_scan_one_effect(ab.effect, target_id, target_card, g,
                                   actor, hand_count, hand_guard);
        rb_free_ability(&ab);
        if (r >= 0) return r;
    }
    return -1;
}

/* Mirror util.rs::compute_play_cost — single source of truth for deploy cost. */
int rb_compute_play_cost(const GameState *g, int actor, int card_id,
                         int set_override) {
    Card card;
    if (!rb_decode_card_by_index((uint32_t)card_id, &card)) return 0;
    int base_cost = card.cost;
    int hand_count = g->p[actor].hand.n + 1;
    int reduction = cr_calc_reduction(g, actor, card_id, &card);
    int increase = 0;
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int ai = 0; ai < n; ai++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, ai, &ab)) continue;
        if (ab.effect && ab.effect->action &&
            !strcmp(ab.effect->action, "modify_cost")) {
            const char *op  = cr_eff_extra(ab.effect, "operation");
            const char *loc = cr_eff_extra(ab.effect, "location");
            if (op && (!strcmp(op, "increase") || !strcmp(op, "add")) &&
                loc && !strcmp(loc, "success_live_zone")) {
                int per_unit_count = ab.effect->per_unit_count > 0
                                     ? ab.effect->per_unit_count : 1;
                int success_count = g->p[actor].success.n;
                int multiplier = ab.effect->count > 0 ? ab.effect->count : 1;
                increase = (success_count / per_unit_count) * multiplier;
            }
        }
        rb_free_ability(&ab);
    }
    int cost = base_cost - reduction + increase;
    if (cost < 0) cost = 0;
    if (cost > 255) cost = 255;
    if (set_override != 0 && set_override != base_cost)
        cost = (int)rb_saturate_u8(set_override);
    rb_free_card(&card);
    return cost;
}

/* Mirror cost.rs::handle_pay_cost_all_discard — settle an "optional: discard your
    entire hand" cost. `accepted` is true unless the player skipped (selected ==
    "skip_optional_cost" or "0"). On accept, every hand card is moved to the
    waitroom (discard) as a real zone change, the movement is recorded, and — only
    if the cost was paid — the ability's effect is executed (mirrors the
    colon-gated "may discard X: draw Y" pattern: skip ⇒ no effect). */
int rb_handle_pay_cost_all_discard(GameState *g, int actor, const char *selected) {
    if (!g) return 0;
    int accepted = (selected && strcmp(selected, "skip_optional_cost") &&
                    strcmp(selected, "0"));
    int cur = g->queue.cur;
    if (cur >= 0 && cur < RB_QUEUE_DEPTH) {
        g->queue.entries[cur].cost_paid = 1;
        g->queue.entries[cur].optional_cost_result = accepted ? 1 : 0;
    }
    if (accepted) {
        const char *target_str = "self";
        const char *source = "hand";
        if (cur >= 0 && cur < RB_QUEUE_DEPTH) {
            RbQueueEntry *e = &g->queue.entries[cur];
            if (e->card_id >= 0) {
                Ability ab;
                if (rb_decode_card_ability((uint32_t)e->card_id, e->ability_idx, &ab) && ab.cost) {
                    if (ab.cost->target && *ab.cost->target) target_str = ab.cost->target;
                    if (ab.cost->source && *ab.cost->source) source = ab.cost->source;
                    rb_free_ability(&ab);
                }
            }
        }
        int tp = rb_resolve_target_player(g, target_str);
        int tpl = (tp >= 0) ? tp : actor;
        RbPlayer *P = &g->p[tpl];
        (void)source;
        int moved = P->hand.n;
        for (int i = 0; i < moved; i++) {
            int cid = P->hand.cards[i];
            if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
            if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
                g->recently_moved[g->n_recently_moved++] = cid;
        }
        g->mods.last_cost_discard_count = moved;
        P->hand.n = 0;
        rb_recalc_constants(g);
    }
    /* Only run the effect if the optional cost was paid. */
    if (accepted && g->queue.resume_eff) {
        int eff_started = (cur >= 0 && cur < RB_QUEUE_DEPTH) ? g->queue.entries[cur].effect_started : 0;
        (void)eff_started;
        rb_execute_effect_ex(g, actor, g->queue.resume_eff, g->queue.resume_host);
        if (cur >= 0 && cur < RB_QUEUE_DEPTH) g->queue.entries[cur].effect_started = 1;
    }
    return 1;
}

/* -- cost.c: pay_cost_move_cards -- */
int rb_pay_cost_move_cards(GameState *g, int actor, const AbilityEffect *cost,
                            int host_cid, int is_activation) {
    if (!g || !cost) return 0;
    const char *source = cost->source ? cost->source : "";
    int count = cost->count > 0 ? cost->count : 1;
    int is_optional = cost->is_optional;
    const char *target = cost->target ? cost->target : "self";
    int tp = rb_resolve_target_player(g, target);
    int tpl = (tp >= 0) ? tp : actor;
    RbPlayer *P = &g->p[tpl];
    if (!strcmp(source, "hand")) {
        int moved = 0;
        while (moved < count && P->hand.n > 0) {
            int cid = P->hand.cards[--P->hand.n];
            if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
            moved++;
        }
        return 1;
    }
    if (!strcmp(source, "stage")) {
        int moved = 0;
        for (int i = 0; i < RB_STAGE_SIZE && moved < count; i++) {
            if (P->stage[i] != RB_EMPTY_SLOT) {
                int cid = P->stage[i];
                P->stage[i] = RB_EMPTY_SLOT;
                if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
                moved++;
            }
        }
        return 1;
    }
    return 1;
}

/* -- cost.c: pay_cost_change_state -- */
int rb_pay_cost_change_state(GameState *g, int actor, const AbilityEffect *cost,
                              int host_cid, int is_activation) {
    if (!g || !cost) return 0;
    int count = cost->count > 0 ? cost->count : 1;
    const char *target = cost->target ? cost->target : "self";
    int tp = rb_resolve_target_player(g, target);
    int tpl = (tp >= 0) ? tp : actor;
    RbPlayer *P = &g->p[tpl];
    int changed = 0;
    for (int i = 0; i < RB_STAGE_SIZE && changed < count; i++) {
        if (P->stage[i] != RB_EMPTY_SLOT) {
            rb_mods_set_orientation(&g->mods, P->stage[i], "wait");
            changed++;
        }
    }
    return 1;
}
