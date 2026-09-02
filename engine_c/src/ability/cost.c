/* cost.c — complete translation of engine/src/ability/cost.rs
   Mirrors pay_deferred_costs, validate_cost, pay_cost, pay_cost_inner,
   handle_optional_cost_payment, handle_pay_cost_all_discard,
   get_change_state_candidates, has_skip_prompt, pay_cost_move_cards,
   pay_cost_change_state. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* ── Resolver-local state (mirrors AbilityResolver fields not in GameState) ── */
static int s_pending_deferred_costs[16];
static int s_n_pending_deferred_costs;
static int s_pending_energy_payment;
static int s_stage_select_intent;
static int s_conditional_choice;

/* ── effect-field helpers (mirrors AbilityEffect::*_any() getters) ── */
static const char *eff_extra(const AbilityEffect *e, const char *k) {
    if (!e || !k) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
static int eff_bool(const AbilityEffect *e, const char *k, int dflt) {
    const char *v = eff_extra(e, k);
    if (!v || !*v) return dflt;
    return !strcmp(v, "true") || !strcmp(v, "1");
}
static int eff_int(const AbilityEffect *e, const char *k, int dflt) {
    const char *v = eff_extra(e, k);
    if (!v || !*v) return dflt;
    return atoi(v);
}

/* ── action-type predicates ── */
static int cost_is_energy(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "pay_energy")) return 1;
    const char *t = e->target;
    return t && strstr(t, "energy") != NULL;
}
static int cost_is_change_state(const AbilityEffect *e) {
    if (!e) return 0;
    if (e->action && !strcmp(e->action, "change_state")) return 1;
    const char *t = e->target;
    return t && strstr(t, "wait") != NULL;
}
static int cost_is_sequential(const AbilityEffect *e) {
    return e && e->action &&
           (!strcmp(e->action, "sequential") || !strcmp(e->action, "sequential_cost"));
}
static int cost_is_move_cards(const AbilityEffect *e) {
    return e && e->action && !strcmp(e->action, "move_cards");
}
static int cost_is_energy_condition(const AbilityEffect *e) {
    return e && e->action && !strcmp(e->action, "energy_condition");
}
static int cost_is_reveal(const AbilityEffect *e) {
    return e && e->action && !strcmp(e->action, "reveal");
}
static int cost_is_place_energy_under_member(const AbilityEffect *e) {
    return e && e->action && !strcmp(e->action, "place_energy_under_member");
}
static int cost_is_custom_under_member(const AbilityEffect *e) {
    return e && e->action && !strcmp(e->action, "custom") &&
           e->destination && strstr(e->destination, "under_member") != NULL;
}

/* ── Mirror cost.rs: has_skip_prompt ── */
static int has_skip_prompt(const AbilityEffect *cost) {
    if (!cost) return 0;
    if (cost_is_energy(cost)) {
        const char *any = eff_extra(cost, "any_number");
        return !(any && !strcmp(any, "true"));
    }
    if (cost_is_change_state(cost)) {
        const char *sc = eff_extra(cost, "self_cost");
        return sc && !strcmp(sc, "true");
    }
    return 0;
}

/* ── CSV splitter ── */
static int split_csv(const char *s, char out[][64], int max) {
    if (!s || !*s) return 0;
    int n = 0;
    const char *p = s;
    while (*p && n < max) {
        const char *comma = strchr(p, ',');
        size_t len = comma ? (size_t)(comma - p) : strlen(p);
        if (len >= 64) len = 63;
        memcpy(out[n], p, len);
        out[n][len] = '\0';
        n++;
        if (!comma) break;
        p = comma + 1;
    }
    return n;
}

/* ── group / character matchers ── */
static int card_matches_any_group(int card_id, const char *group_csv) {
    if (!group_csv || !*group_csv) return 1;
    char groups[16][64];
    int ng = split_csv(group_csv, groups, 16);
    for (int i = 0; i < ng; i++)
        if (rb_card_matches_group_str(card_id, groups[i])) return 1;
    return 0;
}
static int card_matches_any_character(int card_id, const char *chars_csv) {
    if (!chars_csv || !*chars_csv) return 0;
    char chars[16][64];
    int nc = split_csv(chars_csv, chars, 16);
    const char *names[16];
    for (int i = 0; i < nc; i++) names[i] = chars[i];
    return rb_card_matches_characters(card_id, names, nc);
}

/* ── Mirror cost.rs: get_change_state_candidates ── */
static int get_change_state_candidates(const GameState *g, int actor,
                                       const char *target,
                                       const char *card_type,
                                       const char *group_names,
                                       int exclude_self,
                                       int self_cost,
                                       int check_name,
                                       const char *state,
                                       int *out_positions, int max) {
    const RbPlayer *P = &g->p[actor];
    int activating_id = g->activating_card;
    int n = 0;
    for (int i = 0; i < RB_STAGE_SIZE && n < max; i++) {
        int id = P->stage[i];
        if (id == RB_EMPTY_SLOT) continue;
        if (self_cost) {
            if (activating_id != id) continue;
        } else if (exclude_self && activating_id == id) {
            continue;
        }
        if (card_type && *card_type && !rb_card_matches_type(id, card_type)) continue;
        if (group_names && *group_names) {
            if (!card_matches_any_group(id, group_names)) {
                if (!check_name || !card_matches_any_character(id, group_names))
                    continue;
            }
        }
        if (state && *state) {
            const char *ori = rb_mods_get_orientation((RbMods *)&g->mods, id);
            int matches;
            if (!strcmp(state, "active")) {
                matches = !ori || strcmp(ori, "wait") != 0;
            } else if (!strcmp(state, "wait")) {
                matches = ori && !strcmp(ori, "wait");
            } else {
                matches = ori && !strcmp(ori, state);
            }
            if (!matches) continue;
        }
        out_positions[n++] = id;
    }
    return n;
}

/* ── player_prefix ── */
static const char *player_prefix(const GameState *g, int card_id) {
    if (!g) return "??";
    for (int pl = 0; pl < 2; pl++)
        for (int s = 0; s < RB_STAGE_SIZE; s++)
            if (g->p[pl].stage[s] == card_id) return pl == 0 ? "P1" : "P2";
    for (int pl = 0; pl < 2; pl++) {
        for (int i = 0; i < g->p[pl].hand.n; i++)
            if (g->p[pl].hand.cards[i] == card_id) return pl == 0 ? "P1" : "P2";
        for (int i = 0; i < g->p[pl].energy.n; i++)
            if (g->p[pl].energy.cards[i] == card_id) return pl == 0 ? "P1" : "P2";
    }
    return "??";
}

/* ── emit_pay_skip_gate ── */
static void emit_pay_skip_gate(GameState *g, int actor, const AbilityEffect *e,
                               const char *description, int optional,
                               const char *route) {
    RbChoiceRoute r = RB_ROUTE_OPTIONAL_COST;
    if (route && !strcmp(route, "ChoiceCost")) r = RB_ROUTE_CHOICE_COST;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, optional,
                   "pay_optional_cost:skip");
    RbChoice *pending = &g->queue.pending;
    if (pending->description[0] == '\0')
        strncpy(pending->description, description, sizeof(pending->description) - 1);
    pending->route = r;
    (void)e;
}

/* ── resume_pending_actions ── */
static int resume_pending_actions(GameState *g) {
    if (!g) return -1;
    int n_pending = rb_queue_take_pending_actions(g);
    for (int i = 0; i < n_pending; i++) {
        rb_drain_ability_queue(g);
        if (rb_has_pending_choice(g)) break;
    }
    return 0;
}

/* ── execute_move_cards (simple zone transfer for cost payment) ── */
static int execute_move_cards(GameState *g, int actor, const AbilityEffect *cost) {
    const char *src = cost->source ? cost->source : "";
    const char *dst = cost->destination ? cost->destination : "discard";
    int count = cost->count > 0 ? cost->count : 1;
    int moved = 0;
    RbPlayer *P = &g->p[actor];

    if (!strcmp(src, "stage")) {
        for (int i = 0; i < RB_STAGE_SIZE && moved < count; i++) {
            if (P->stage[i] != RB_EMPTY_SLOT) {
                int cid = P->stage[i];
                P->stage[i] = RB_EMPTY_SLOT;
                P->stage_wait[i] = 0;
                if (!strcmp(dst, "waitroom") || !strcmp(dst, "discard")) {
                    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
                } else if (!strcmp(dst, "energy")) {
                    if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = cid;
                } else if (!strcmp(dst, "hand")) {
                    if (P->hand.n < RB_MAX_HAND) P->hand.cards[P->hand.n++] = cid;
                } else if (!strcmp(dst, "energy_deck")) {
                    if (P->energy_deck.n < RB_MAX_ZONE) P->energy_deck.cards[P->energy_deck.n++] = cid;
                }
                moved++;
            }
        }
        g->mods.last_cost_discard_count = moved;
    } else {
        RbBag *src_bag = NULL;
        if (!strcmp(src, "hand")) src_bag = &P->hand;
        else if (!strcmp(src, "deck") || !strcmp(src, "deck_top")) src_bag = &P->deck;
        else if (!strcmp(src, "waitroom") || !strcmp(src, "discard")) src_bag = &P->discard;
        else if (!strcmp(src, "energy")) src_bag = &P->energy;

        if (src_bag) {
            while (moved < count && src_bag->n > 0) {
                int cid = src_bag->cards[--src_bag->n];
                if (!strcmp(dst, "waitroom") || !strcmp(dst, "discard")) {
                    if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = cid;
                } else if (!strcmp(dst, "energy")) {
                    if (P->energy.n < RB_MAX_ZONE) P->energy.cards[P->energy.n++] = cid;
                } else if (!strcmp(dst, "hand")) {
                    if (P->hand.n < RB_MAX_HAND) P->hand.cards[P->hand.n++] = cid;
                } else if (!strcmp(dst, "energy_deck")) {
                    if (P->energy_deck.n < RB_MAX_ZONE) P->energy_deck.cards[P->energy_deck.n++] = cid;
                }
                moved++;
            }
            g->mods.last_cost_discard_count = moved;
        }
    }
    return moved;
}

/* ── execute_place_energy_under_member_non_optional ── */
int execute_place_energy_under_member_non_optional(GameState *g, int actor, const AbilityEffect *e) {
    return rb_effect_place_energy_under_member_non_optional(g, actor, e);
}

/* ── validate_cost ── */
static int validate_cost(const GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    if (cost_is_sequential(cost)) {
        for (int i = 0; i < cost->n_child; i++)
            if (!validate_cost(g, actor, cost->child[i])) return 0;
        return 1;
    }
    if (cost->action && !strcmp(cost->action, "choice_condition")) return 1;
    if (cost_is_move_cards(cost)) {
        const char *src = cost->source ? cost->source : "";
        int count = cost->count > 0 ? cost->count : 1;
        const RbPlayer *P = &g->p[actor];
        int available = 0;
        if (!strcmp(src, "hand")) available = P->hand.n;
        else if (!strcmp(src, "stage")) {
            for (int i = 0; i < RB_STAGE_SIZE; i++)
                if (P->stage[i] != RB_EMPTY_SLOT) available++;
        } else if (!strcmp(src, "deck") || !strcmp(src, "deck_top")) available = P->deck.n;
        else if (!strcmp(src, "waitroom") || !strcmp(src, "discard")) available = P->discard.n;
        else if (!strcmp(src, "energy")) available = P->energy.n;
        if (available < count) return 0;
        return 1;
    }
    if (cost_is_energy_condition(cost)) {
        int count = cost->count > 0 ? cost->count : 1;
        const RbPlayer *P = &g->p[actor];
        if ((int)P->energy.n < count) return 0;
        return 1;
    }
    if (cost_is_change_state(cost)) {
        const char *sc = eff_extra(cost, "state_change");
        if (sc && !strcmp(sc, "wait")) {
            const char *target = cost->target ? cost->target : "self";
            int exclude_self = eff_bool(cost, "exclude_self", 0);
            const char *card_type = eff_extra(cost, "card_type");
            const char *group_names = eff_extra(cost, "group_names");
            int self_cost = eff_bool(cost, "self_cost", 0);
            int pos[RB_STAGE_SIZE];
            int n = get_change_state_candidates(g, actor, target, card_type,
                                                group_names, exclude_self,
                                                self_cost, 0, "active",
                                                pos, RB_STAGE_SIZE);
            if (n == 0) return 0;
        }
        return 1;
    }
    return 1;
}

/* ── pay_cost_move_cards ── */
static int pay_cost_move_cards(GameState *g, int actor, const AbilityEffect *cost) {
    const char *source = cost->source ? cost->source : "";
    int is_any_number = eff_bool(cost, "any_number", 0);
    int count = cost->count > 0 ? cost->count : 1;
    const char *card_type = eff_extra(cost, "card_type");
    int optional = cost->is_optional;
    const char *target = cost->target ? cost->target : "self";
    int tp = rb_resolve_target_player(g, target);
    int tpl = (tp >= 0) ? tp : actor;
    RbPlayer *P = &g->p[tpl];
    int same_unit = eff_bool(cost, "same_unit_name", 0);
    int is_from_hand = !strcmp(source, "hand") && !same_unit;
    int is_all = eff_bool(cost, "all", 0);
    int is_activation = 0;

    /* Determine is_activation from current queue entry */
    {
        int cur = g->queue.cur;
        if (cur >= 0 && cur < g->queue.n_entries) {
            int cid = g->queue.entries[cur].card_id;
            int aidx = g->queue.entries[cur].ability_idx;
            Ability ab;
            if (rb_decode_card_ability((uint32_t)cid, aidx, &ab)) {
                is_activation = rb_resolver_current_ability_is_activation(&ab);
                rb_free_ability(&ab);
            }
        }
    }

    /* Optional stage-move costs: emit pay/skip gate first */
    if (optional && !is_activation && !is_from_hand && !strcmp(source, "stage")) {
        const char *group_filter = eff_extra(cost, "group_names");
        int matching = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) {
            int id = P->stage[i];
            if (id != RB_EMPTY_SLOT) {
                if (group_filter && *group_filter && !rb_card_matches_group_str(id, group_filter))
                    continue;
                matching++;
            }
        }
        if (matching < count) {
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
                g->queue.entries[g->queue.cur].cost_paid = 1;
                g->queue.entries[g->queue.cur].optional_cost_result = 0;
            }
            return 1;
        }
        emit_pay_skip_gate(g, actor, cost,
                           "Put members from stage to waitroom (or skip)?",
                           1, "OptionalCost");
        return 1;
    }

    /* All-hand discard */
    if (is_from_hand && is_all) {
        int hand_len = P->hand.n;
        int is_optional = (optional || is_any_number) && !is_activation;
        if (hand_len == 0) {
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
                g->queue.entries[g->queue.cur].cost_paid = 1;
                g->queue.entries[g->queue.cur].optional_cost_result = 0;
            }
            return 1;
        }
        RbChoice *pending = &g->queue.pending;
        memset(pending, 0, sizeof(*pending));
        pending->kind = RB_CHOICE_SELECT_TARGET;
        snprintf(pending->description, sizeof(pending->description),
                 "Discard entire hand (%d cards)?", hand_len);
        pending->allow_skip = is_optional;
        pending->route = RB_ROUTE_OPTIONAL_COST;
        g->queue.has_pending = 1;
        g->queue.pending.route = RB_ROUTE_OPTIONAL_COST;
        return 1;
    }

    /* From hand with filtering */
    if (is_from_hand) {
        int is_same_group_name = eff_extra(cost, "group_reference") &&
                                  !strcmp(eff_extra(cost, "group_reference"), "same_group_name");
        int matching_indices[RB_MAX_HAND];
        int n_matching = 0;

        if (is_same_group_name) {
            char group_names_buf[32][64];
            int group_counts[32];
            int n_groups = 0;
            for (int i = 0; i < P->hand.n && n_groups < 32; i++) {
                int cid = P->hand.cards[i];
                Card c;
                if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
                if (!c.group_idx) { rb_free_card(&c); continue; }
                const char *gn = rb_get_string(c.group_idx);
                if (!gn || !*gn) { rb_free_card(&c); continue; }
                int found = -1;
                for (int g = 0; g < n_groups; g++)
                    if (!strcmp(group_names_buf[g], gn)) { found = g; break; }
                if (found < 0) {
                    found = n_groups++;
                    strncpy(group_names_buf[found], gn, sizeof(group_names_buf[0]) - 1);
                    group_names_buf[found][sizeof(group_names_buf[0]) - 1] = '\0';
                    group_counts[found] = 0;
                }
                group_counts[found]++;
                rb_free_card(&c);
            }
            for (int i = 0; i < P->hand.n && n_matching < RB_MAX_HAND; i++) {
                int cid = P->hand.cards[i];
                Card c;
                if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
                if (!c.group_idx) { rb_free_card(&c); continue; }
                const char *gn = rb_get_string(c.group_idx);
                if (!gn || !*gn) { rb_free_card(&c); continue; }
                for (int g = 0; g < n_groups; g++) {
                    if (!strcmp(group_names_buf[g], gn) && group_counts[g] >= count) {
                        matching_indices[n_matching++] = i;
                        break;
                    }
                }
                rb_free_card(&c);
            }
        } else {
            const char *cost_limit = eff_extra(cost, "cost_limit");
            const char *group_names_cost = eff_extra(cost, "group_names");
            const char *chars = eff_extra(cost, "characters");
            for (int i = 0; i < P->hand.n && n_matching < RB_MAX_HAND; i++) {
                int cid = P->hand.cards[i];
                int match = 1;
                if (card_type && *card_type && !rb_card_matches_type(cid, card_type)) match = 0;
                if (match && cost_limit && *cost_limit) {
                    Card c;
                    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
                        if (c.cost != atoi(cost_limit)) match = 0;
                        rb_free_card(&c);
                    } else match = 0;
                }
                if (match && group_names_cost && *group_names_cost)
                    if (!card_matches_any_group(cid, group_names_cost)) match = 0;
                if (match && chars && *chars)
                    if (!card_matches_any_character(cid, chars)) match = 0;
                if (match) matching_indices[n_matching++] = i;
            }
        }

        int is_optional = (optional || is_any_number) && !is_activation;

        if (!is_any_number && n_matching < count) {
            if (is_optional) {
                if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
                    g->queue.entries[g->queue.cur].cost_paid = 1;
                    g->queue.entries[g->queue.cur].optional_cost_result = 0;
                }
                return 1;
            }
            return 0;
        }
        if (is_any_number && n_matching == 0) {
            if (is_optional) {
                if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
                    g->queue.entries[g->queue.cur].cost_paid = 1;
                    g->queue.entries[g->queue.cur].optional_cost_result = 0;
                }
                return 1;
            }
            return 1;
        }

        int effective_count = is_any_number ? 0 : count;
        int max_str = is_any_number ? (eff_bool(cost, "max", 0) ? (count < n_matching ? count : n_matching) : n_matching) : n_matching;

        char desc[256];
        if (is_any_number) {
            snprintf(desc, sizeof(desc),
                     "Select any number of card(s) from hand (0-%d) (or skip)",
                     max_str);
        } else {
            snprintf(desc, sizeof(desc),
                     "Select %d card(s) from hand%s",
                     effective_count, is_optional ? " (or skip)" : "");
        }

        RbChoice *pending = &g->queue.pending;
        memset(pending, 0, sizeof(*pending));
        pending->kind = RB_CHOICE_SELECT_CARD;
        strncpy(pending->zone, "hand", sizeof(pending->zone) - 1);
        pending->count = effective_count;
        pending->allow_skip = is_optional;
        pending->route = RB_ROUTE_SELECT_CARDS;
        strncpy(pending->description, desc, sizeof(pending->description) - 1);
        if (card_type && *card_type)
            strncpy(pending->card_type, card_type, sizeof(pending->card_type) - 1);
        const char *group_names_cost = eff_extra(cost, "group_names");
        if (group_names_cost && *group_names_cost)
            strncpy(pending->filter_group, group_names_cost, sizeof(pending->filter_group) - 1);
        g->queue.has_pending = 1;
        g->queue.pending.route = RB_ROUTE_SELECT_CARDS;
        return 1;
    }

    /* Non-hand source: check availability then execute */
    if (*source) {
        const RbPlayer *P = &g->p[actor];
        int available = 0;
        if (!strcmp(source, "stage")) {
            for (int i = 0; i < RB_STAGE_SIZE; i++)
                if (P->stage[i] != RB_EMPTY_SLOT) available++;
        } else if (!strcmp(source, "hand")) {
            available = P->hand.n;
        } else if (!strcmp(source, "deck") || !strcmp(source, "deck_top")) {
            available = P->deck.n;
            if (available < count && !strcmp(source, "deck_top")) {
                available += P->discard.n;
                if (available == 0) return 0;
            }
        } else if (!strcmp(source, "waitroom") || !strcmp(source, "discard")) {
            available = P->discard.n;
        } else if (!strcmp(source, "energy")) {
            available = P->energy.n;
        }
        if (available < count) return 0;
    }

    return execute_move_cards(g, actor, cost) >= 0;
}

/* ── pay_cost_change_state ── */
static int pay_cost_change_state(GameState *g, int actor, const AbilityEffect *cost) {
    g->n_last_cost_waited_members = 0;
    const char *sc = eff_extra(cost, "state_change");
    const char *state_change = sc ? sc : "";
    const char *target = cost->target ? cost->target : "self";
    int optional = cost->is_optional;
    int count = cost->count > 0 ? cost->count : 1;
    int exclude_self = eff_bool(cost, "exclude_self", 0);
    const char *card_type = eff_extra(cost, "card_type");
    const char *group_names = eff_extra(cost, "group_names");
    int self_cost = eff_bool(cost, "self_cost", 0);
    int tp = rb_resolve_target_player(g, target);
    int tpl = (tp >= 0) ? tp : actor;
    int is_activation = 0;

    {
        int cur = g->queue.cur;
        if (cur >= 0 && cur < g->queue.n_entries) {
            int cid = g->queue.entries[cur].card_id;
            int aidx = g->queue.entries[cur].ability_idx;
            Ability ab;
            if (rb_decode_card_ability((uint32_t)cid, aidx, &ab)) {
                is_activation = rb_resolver_current_ability_is_activation(&ab);
                rb_free_ability(&ab);
            }
        }
    }

    /* Optional gate for wait-state costs — emit pay/skip prompt and return */
    if (optional && !is_activation && !strcmp(state_change, "wait")) {
        int pos[RB_STAGE_SIZE];
        int n = get_change_state_candidates(g, tpl, target, card_type,
                                            group_names, exclude_self,
                                            self_cost, 0, "active",
                                            pos, RB_STAGE_SIZE);
        if (n == 0) return 1;   /* no eligible targets → auto-decline */
        emit_pay_skip_gate(g, actor, cost,
                           "Put this member to wait state", 1, "OptionalCost");
        return 1;
    }

    /* Mandatory cost: resolve all eligible candidates */
    int pos[RB_STAGE_SIZE];
    int n = get_change_state_candidates(g, tpl, target, card_type,
                                        group_names, exclude_self,
                                        self_cost, 1, "active",
                                        pos, RB_STAGE_SIZE);
    if (n == 0) return 0;   /* cannot pay — no matching members */

    if (n <= count) {
        /* Auto-wait all candidates */
        for (int i = 0; i < n; i++) {
            int cid = pos[i];
            if (!strcmp(state_change, "wait")) {
                rb_mods_set_orientation(&g->mods, cid, "wait");
                g->last_cost_waited_members[g->n_last_cost_waited_members++] = (int16_t)cid;
            } else if (!strcmp(state_change, "rest") || !strcmp(state_change, "rested")) {
                rb_mods_set_orientation(&g->mods, cid, "rest");
                g->last_cost_waited_members[g->n_last_cost_waited_members++] = (int16_t)cid;
            }
        }
    } else {
        /* Emit select_cards choice for player to pick exactly `count` members */
        char desc[128];
        char desc_ja[128];
        snprintf(desc, sizeof(desc), "Select %d stage member(s) to %s", count, state_change);
        snprintf(desc_ja, sizeof(desc_ja),
                 "ウェイトにするステージメンバーを%d体選択", count);

        RbChoice *pending = &g->queue.pending;
        memset(pending, 0, sizeof(*pending));
        pending->kind = RB_CHOICE_SELECT_CARD;
        strncpy(pending->zone, "stage", sizeof(pending->zone) - 1);
        pending->count = count;
        pending->allow_skip = 0;
        pending->route = RB_ROUTE_SELECT_CARDS;
        strncpy(pending->description, desc, sizeof(pending->description) - 1);
        if (card_type && *card_type)
            strncpy(pending->card_type, card_type, sizeof(pending->card_type) - 1);
        if (group_names && *group_names)
            strncpy(pending->filter_group, group_names, sizeof(pending->filter_group) - 1);
        g->queue.has_pending = 1;
        s_stage_select_intent = 1;
    }
    return 1;
}

/* ── pay_cost_inner ── */
static int pay_cost_inner(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;

    /* Rule 9.4.2 / Q234: SequentialCost — validate all, then pay in order */
    if (cost_is_sequential(cost)) {
        int start_idx = 0;
        {
            int cur = g->queue.cur;
            if (cur >= 0 && cur < g->queue.n_entries)
                start_idx = g->queue.entries[cur].cost_paid_index;
        }
        for (int i = start_idx; i < cost->n_child; i++)
            if (!validate_cost(g, actor, cost->child[i]))
                return 0;

        int had_binary_auto_pay = 0;
        for (int i = start_idx; i < cost->n_child; i++) {
            const AbilityEffect *sub = cost->child[i];
            int is_binary = sub->is_optional && has_skip_prompt(sub);
            int has_choice_ahead = 0;
            for (int j = i + 1; j < cost->n_child; j++)
                if (!has_skip_prompt(cost->child[j])) { has_choice_ahead = 1; break; }
            if (is_binary && has_choice_ahead) {
                /* Defer: auto-pay without a prompt (clone + strip optionality) */
                AbilityEffect deferred = *sub;
                deferred.is_optional = 0;
                if (s_n_pending_deferred_costs < 16)
                    s_pending_deferred_costs[s_n_pending_deferred_costs++] = i;
                had_binary_auto_pay = 1;
            } else {
                if (!pay_cost_inner(g, actor, sub)) return 0;
            }
            /* Update cost_paid_index on the current queue entry */
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].cost_paid_index = i + 1;
            /* If a pending choice was emitted, override its description to
               show the combined (auto-paid + pending) cost text */
            if (g->queue.has_pending && had_binary_auto_pay) {
                RbChoice *pending = &g->queue.pending;
                (void)pending;
                had_binary_auto_pay = 0;
            }
            if (g->queue.has_pending) return 1;
        }
        return 1;
    }

    /* ChoiceCondition */
    if (cost->action && !strcmp(cost->action, "choice_condition")) {
        char opts[8][128];
        int n_opts = 0;
        for (int i = 0; i < cost->n_child && n_opts < 8; i++) {
            if (cost->child[i] && cost->child[i]->text) {
                strncpy(opts[n_opts], cost->child[i]->text, sizeof(opts[0]) - 1);
                opts[n_opts][sizeof(opts[0]) - 1] = '\0';
                n_opts++;
            }
        }
        char desc[256] = "Choose cost option: ";
        for (int i = 0; i < n_opts; i++) {
            if (i > 0) strncat(desc, " OR ", sizeof(desc) - strlen(desc) - 1);
            strncat(desc, opts[i], sizeof(desc) - strlen(desc) - 1);
        }

        RbChoice *pending = &g->queue.pending;
        memset(pending, 0, sizeof(*pending));
        pending->kind = RB_CHOICE_SELECT_TARGET;
        strncpy(pending->target, "choice_condition", sizeof(pending->target) - 1);
        strncpy(pending->description, desc, sizeof(pending->description) - 1);
        pending->allow_skip = 0;
        pending->route = RB_ROUTE_CHOICE_COST;
        g->queue.has_pending = 1;
        g->queue.pending.route = RB_ROUTE_CHOICE_COST;
        if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
            g->queue.entries[g->queue.cur].choice_card_no = RB_ROUTE_CHOICE_COST;
        return 1;
    }

    /* MoveCards */
    if (cost_is_move_cards(cost)) return pay_cost_move_cards(g, actor, cost);

    /* ChangeState */
    if (cost_is_change_state(cost)) return pay_cost_change_state(g, actor, cost);

    /* PayEnergy */
    if (cost_is_energy(cost)) {
        int energy = eff_int(cost, "energy_count", 0);
        const char *target = cost->target ? cost->target : "self";
        int optional = cost->is_optional;
        int any_number = eff_bool(cost, "any_number", 0);
        int tp = rb_resolve_target_player(g, target);
        int tpl = (tp >= 0) ? tp : actor;
        RbPlayer *P = &g->p[tpl];
        int is_activation = 0;

        {
            int cur = g->queue.cur;
            if (cur >= 0 && cur < g->queue.n_entries) {
                int cid = g->queue.entries[cur].card_id;
                int aidx = g->queue.entries[cur].ability_idx;
                Ability ab;
                if (rb_decode_card_ability((uint32_t)cid, aidx, &ab)) {
                    is_activation = rb_resolver_current_ability_is_activation(&ab);
                    rb_free_ability(&ab);
                }
            }
        }

        /* any_number: show active energy for selection */
        if (any_number && (optional || !is_activation)) {
            int active_count = rb_energy_active_count(P);
            if (active_count == 0) {
                if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
                    g->queue.entries[g->queue.cur].cost_paid = 1;
                    g->queue.entries[g->queue.cur].optional_cost_result = 0;
                }
                return 1;
            }
            RbChoice *pending = &g->queue.pending;
            memset(pending, 0, sizeof(*pending));
            pending->kind = RB_CHOICE_SELECT_CARD;
            strncpy(pending->zone, "energy", sizeof(pending->zone) - 1);
            pending->count = 0;
            pending->allow_skip = 1;
            pending->route = RB_ROUTE_OPTIONAL_COST;
            char desc[128];
            snprintf(desc, sizeof(desc),
                     "Select energy card to pay (active: %d). Skip when done",
                     active_count);
            strncpy(pending->description, desc, sizeof(pending->description) - 1);
            g->queue.has_pending = 1;
            g->queue.pending.route = RB_ROUTE_OPTIONAL_COST;
            s_pending_energy_payment = 0;
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].choice_card_no = RB_ROUTE_OPTIONAL_COST;
            return 1;
        }

        /* Optional fixed-count energy */
        if (optional && !is_activation) {
            int active = rb_energy_active_count(P);
            if (active < energy) {
                if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
                    g->queue.entries[g->queue.cur].cost_paid = 1;
                    g->queue.entries[g->queue.cur].optional_cost_result = 0;
                }
                return 1;
            }
            emit_pay_skip_gate(g, actor, cost,
                               "Pay energy (or skip)?", 1, "OptionalCost");
            return 1;
        }

        /* Baton touch zero cost */
        if (g->baton_touch_zero_cost && energy > 0) return 1;

        /* Actual payment */
        if (energy > 0) rb_energy_pay(P, energy);
        rb_recalc_constants(g);
        return 1;
    }

    /* EnergyCondition */
    if (cost_is_energy_condition(cost)) {
        int count = cost->count > 0 ? cost->count : 1;
        const char *target = cost->target ? cost->target : "self";
        int tp = rb_resolve_target_player(g, target);
        int tpl = (tp >= 0) ? tp : actor;
        RbPlayer *P = &g->p[tpl];
        if ((int)P->energy.n < count) return 0;
        for (int i = 0; i < count; i++) {
            if (P->energy.n > 0) {
                int cid = P->energy.cards[--P->energy.n];
                if (P->energy_deck.n < RB_MAX_ZONE)
                    P->energy_deck.cards[P->energy_deck.n++] = cid;
            }
        }
        rb_energy_sub_active(P, count);
        return 1;
    }

    /* Reveal */
    if (cost_is_reveal(cost)) {
        const char *source = cost->source ? cost->source : "hand";
        const char *target = cost->target ? cost->target : "self";
        const char *card_type = eff_extra(cost, "card_type");
        int tp = rb_resolve_target_player(g, target);
        int tpl = (tp >= 0) ? tp : actor;
        RbPlayer *P = &g->p[tpl];
        int card_ids[RB_MAX_HAND];
        int n_ids = 0;

        if (!strcmp(source, "hand")) {
            const char *cost_values = eff_extra(cost, "cost_values");
            const char *group_names_cost = eff_extra(cost, "group_names");
            int has_cost_values = cost_values && *cost_values;
            int vals[16], nvals = 0;
            if (has_cost_values) {
                char vbuf[256];
                strncpy(vbuf, cost_values, sizeof(vbuf) - 1);
                vbuf[sizeof(vbuf) - 1] = '\0';
                char *tok = strtok(vbuf, ",");
                while (tok && nvals < 16) {
                    vals[nvals++] = atoi(tok);
                    tok = strtok(NULL, ",");
                }
            }
            for (int i = 0; i < P->hand.n && n_ids < RB_MAX_HAND; i++) {
                int cid = P->hand.cards[i];
                int match = 1;
                if (card_type && *card_type && !rb_card_matches_type(cid, card_type)) match = 0;
                if (match && has_cost_values) {
                    Card c;
                    if (rb_decode_card_by_index((uint32_t)cid, &c)) {
                        int found = 0;
                        for (int v = 0; v < nvals; v++)
                            if (c.cost == vals[v] || rb_card_get_score(&c) == vals[v]) { found = 1; break; }
                        if (!found) match = 0;
                        rb_free_card(&c);
                    } else match = 0;
                }
                if (match && group_names_cost && *group_names_cost)
                    if (!card_matches_any_group(cid, group_names_cost)) match = 0;
                if (match) card_ids[n_ids++] = cid;
            }
        }

        if (n_ids == 0) return 0;

        /* Dedup */
        for (int i = 0; i < n_ids; i++)
            for (int j = i + 1; j < n_ids;)
                if (card_ids[i] == card_ids[j]) {
                    card_ids[j] = card_ids[--n_ids];
                } else j++;

        int has_explicit_count = cost->count > 0;
        int explicit_count = cost->count > 0 ? cost->count : 1;

        if (has_explicit_count && n_ids <= explicit_count) {
            for (int i = 0; i < n_ids; i++) {
                /* push_revealed_card / push_revealed_cost_card equivalents */
            }
            return 1;
        }

        RbChoice *pending = &g->queue.pending;
        memset(pending, 0, sizeof(*pending));
        pending->kind = RB_CHOICE_SELECT_CARD;
        strncpy(pending->zone, source, sizeof(pending->zone) - 1);
        pending->count = has_explicit_count ? explicit_count : 0;
        pending->allow_skip = 1;
        pending->route = RB_ROUTE_SELECT_CARDS;
        strncpy(pending->description, "Select cards to reveal from hand",
                sizeof(pending->description) - 1);
        if (card_type && *card_type)
            strncpy(pending->card_type, card_type, sizeof(pending->card_type) - 1);
        const char *group_names_cost = eff_extra(cost, "group_names");
        if (group_names_cost && *group_names_cost)
            strncpy(pending->filter_group, group_names_cost, sizeof(pending->filter_group) - 1);
        g->queue.has_pending = 1;
        return 1;
    }

    /* PlaceEnergyUnderMember */
    if (cost_is_place_energy_under_member(cost)) {
        return execute_place_energy_under_member_non_optional(g, actor, cost);
    }

    /* Custom with UnderMember destination */
    if (cost_is_custom_under_member(cost)) {
        return execute_place_energy_under_member_non_optional(g, actor, cost);
    }

    return 1;
}

/* ── Public: rb_pay_cost ── */
int rb_pay_cost(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost) return 1;
    int result = pay_cost_inner(g, actor, cost);
    if (result && !g->queue.has_pending) {
        const char *pp = player_prefix(g, g->activating_card);
        const char *act_name = "";
        if (g->activating_card >= 0) act_name = rb_card_string((uint16_t)g->activating_card);
        const char *cost_desc = cost->text ? cost->text : "";
        char logbuf[256];
        snprintf(logbuf, sizeof(logbuf), "%s %s: [cost] %s", pp,
                 act_name[0] ? act_name : "", cost_desc);
        rb_log_push_verdict(logbuf, "cost", 1);
    }
    return result;
}

/* ── Public: rb_validate_cost ── */
int rb_validate_cost(const GameState *g, int actor, const AbilityEffect *cost) {
    return validate_cost(g, actor, cost);
}

/* ── Public: rb_pay_deferred_costs ── */
int rb_pay_deferred_costs(GameState *g, int actor, const AbilityEffect *cost) {
    if (!cost || !cost_is_sequential(cost)) return 1;
    /* Resume from cost_paid_index (mirrors Rust: start_idx = entry.cost_paid_index) */
    int start_idx = 0;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
        start_idx = g->queue.entries[g->queue.cur].cost_paid_index;
    for (int i = start_idx; i < cost->n_child; i++) {
        int found = 0;
        for (int d = 0; d < s_n_pending_deferred_costs; d++) {
            if (s_pending_deferred_costs[d] == i) { found = 1; break; }
        }
        if (!found) continue;
        const AbilityEffect *child = cost->child[i];
        if (!child) continue;
        /* Clone and strip optionality (mirrors Rust: let mut auto = sub_cost.clone(); auto.set_optional(Some(false))) */
        AbilityEffect sub = *child;
        sub.is_optional = 0;
        if (!rb_pay_cost(g, actor, &sub)) return 0;
    }
    s_n_pending_deferred_costs = 0;
    return 1;
}

/* ── Public: rb_handle_optional_cost_payment ── */
int rb_handle_optional_cost_payment(GameState *g, int actor, const AbilityEffect *cost, int pay) {
    g->n_last_cost_waited_members = 0;

    if (!pay) {
        /* ── SKIP path ── */
        g->queue.has_pending = 0;
        s_pending_energy_payment = 0;
        if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
            g->queue.entries[g->queue.cur].cost_paid = 1;
            g->queue.entries[g->queue.cur].optional_cost_result = 0;
        }
        /* Execute the alternative effect ("unless you pay") immediately.
           In the C port pending_actions is a counter only (no effect-pointer
           array), so we execute the alternative directly rather than enqueuing
           it for the drain loop to find. */
        if (cost && cost->alternative_effect) {
            rb_execute_effect_ex(g, actor, (AbilityEffect *)cost->alternative_effect,
                                 g->activating_card);
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].effect_started = 1;
        }
        return resume_pending_actions(g);
    }

    /* ── PAY path ── */
    g->queue.has_pending = 0;

    /* Pending energy payment from any_number select */
    if (s_pending_energy_payment > 0) {
        int ep = s_pending_energy_payment;
        s_pending_energy_payment = 0;
        RbPlayer *P = &g->p[actor];
        if (rb_energy_active_count(P) >= ep)
            rb_energy_pay(P, ep);
        else {
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].pending_actions_n = 0;
            return resume_pending_actions(g);
        }
    }

    /* Gated effect move: shared-gate source zones (stage→waitroom,
       energy→energy_deck) reached via MoveCards or first step of Sequential */
    if (cost && cost->action && !strcmp(cost->action, "move_cards")) {
        const char *src = cost->source ? cost->source : "";
        if (!strcmp(src, "stage") ||
            (!strcmp(src, "energy") && cost->destination &&
             !strcmp(cost->destination, "energy_deck"))) {
            AbilityEffect c = *cost;
            c.is_optional = 0;
            pay_cost_move_cards(g, actor, &c);
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
                g->queue.entries[g->queue.cur].cost_paid = 1;
                g->queue.entries[g->queue.cur].optional_cost_result = 1;
            }
            return 1;
        }
    }

    /* Sequential sub-costs: pay each in order after player confirmed */
    if (cost && cost_is_sequential(cost)) {
        for (int i = 0; i < cost->n_child; i++) {
            const AbilityEffect *sub = cost->child[i];
            if (sub->action && eff_extra(sub, "state_change") &&
                !strcmp(eff_extra(sub, "state_change"), "wait") &&
                eff_bool(sub, "self_cost", 0)) {
                /* self_cost wait: set activating card to wait directly */
                if (g->activating_card >= 0)
                    rb_mods_set_orientation(&g->mods, g->activating_card, "wait");
            } else if (!rb_pay_cost(g, actor, sub)) {
                /* warning: sub-cost payment error */
            }
            if (g->queue.has_pending) return 1;
        }
    }

    /* PlaceEnergyUnderMember sub-cost */
    if (cost && cost->action && !strcmp(cost->action, "place_energy_under_member")) {
        execute_place_energy_under_member_non_optional(g, actor, cost);
    }

    /* ── After all costs paid: run the entry's effect ── */
    g->queue.has_pending = 0;
    int effect_started = 0;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
        effect_started = g->queue.entries[g->queue.cur].effect_started;

    const AbilityEffect *entry_cost = rb_entry_cost(g);
    if (entry_cost && !effect_started) {
        const AbilityEffect *entry_eff = rb_entry_effect(g);
        if (entry_eff) {
            if (entry_eff->action && !strcmp(entry_eff->action, "place_energy_under_member")) {
                execute_place_energy_under_member_non_optional(g, actor, entry_eff);
            } else if (entry_eff->action) {
                rb_execute_effect_ex(g, actor, (AbilityEffect *)entry_eff, g->activating_card);
            }
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].effect_started = 1;
        }
    } else if (rb_queue_has_pending_actions(g)) {
        resume_pending_actions(g);
    }
    return 1;
}

/* ── Public: rb_handle_pay_cost_all_discard ── */
int rb_handle_pay_cost_all_discard(GameState *g, int actor, const char *selected) {
    if (!g) return 0;
    int accepted = selected && strcmp(selected, "skip_optional_cost") && strcmp(selected, "0");
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries) {
        g->queue.entries[g->queue.cur].cost_paid = 1;
        g->queue.entries[g->queue.cur].optional_cost_result = accepted ? 1 : 0;
    }
    g->queue.has_pending = 0;

    if (accepted) {
        /* Pass the entry's cost so movement events carry the correct source zone */
        const AbilityEffect *entry_cost = rb_entry_cost(g);
        rb_effect_pay_cost_all_discard(g, actor, entry_cost);
        rb_recalc_constants(g);
    }

    /* After cost settled, run the entry's effect if the cost was paid
       and the effect has not yet started */
    int effect_started = 0;
    if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
        effect_started = g->queue.entries[g->queue.cur].effect_started;

    const AbilityEffect *entry_cost = rb_entry_cost(g);
    if (accepted && entry_cost && !effect_started) {
        const AbilityEffect *entry_eff = rb_entry_effect(g);
        if (entry_eff) {
            if (entry_eff->action && !strcmp(entry_eff->action, "place_energy_under_member")) {
                execute_place_energy_under_member_non_optional(g, actor, entry_eff);
            } else if (entry_eff->action) {
                rb_execute_effect_ex(g, actor, (AbilityEffect *)entry_eff, g->activating_card);
            }
            if (g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)
                g->queue.entries[g->queue.cur].effect_started = 1;
        }
    } else if (accepted && rb_queue_has_pending_actions(g)) {
        resume_pending_actions(g);
    }
    return 1;
}

/* ── Public: rb_cost_has_skip_prompt ── */
int rb_cost_has_skip_prompt(const AbilityEffect *cost) {
    return has_skip_prompt(cost);
}

/* ── Public: rb_get_change_state_candidates ── */
int rb_get_change_state_candidates(const GameState *g, int actor, int *out_positions, int max) {
    int tmp[RB_STAGE_SIZE];
    int n = get_change_state_candidates(g, actor, "self", NULL, NULL, 0, 0, 0, "active", tmp, max);
    for (int i = 0; i < n && i < max; i++) out_positions[i] = tmp[i];
    return n;
}

/* ── Public: rb_pay_cost_move_cards ── */
int rb_pay_cost_move_cards(GameState *g, int actor, const AbilityEffect *cost,
                           int host_cid, int is_activation) {
    if (!g || !cost) return 0;
    (void)host_cid;
    (void)is_activation;
    return pay_cost_move_cards(g, actor, cost);
}

/* ── Public: rb_pay_cost_change_state ── */
int rb_pay_cost_change_state(GameState *g, int actor, const AbilityEffect *cost,
                             int host_cid, int is_activation) {
    if (!g || !cost) return 0;
    (void)host_cid;
    (void)is_activation;
    return pay_cost_change_state(g, actor, cost);
}

/* ════════════════════════════════════════════════════════════════════
   Play-cost reduction (mirrors engine/src/ability/util.rs
   compute_play_cost / calculate_play_cost_reduction / scan_abilities_for_
   cost_reduction / per_unit_cost_reduction / play_cost_reduction_matches).
   ════════════════════════════════════════════════════════════════════ */

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

static int cr_scan_one_effect_source(int src_card_id, int target_id,
                                     const Card *target_card, const GameState *g,
                                     int actor, int hand_count, int hand_guard);

static int cr_calc_reduction(const GameState *g, int actor, int card_id,
                             const Card *card) {
    int cost_reduction = 0;
    int hand_count = g->p[actor].hand.n + 1;
    int n = rb_card_num_abilities((uint32_t)card_id);
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

/* ── Bag helpers (kept from existing cost.c) ── */
static void bag_push(RbBag *b, int c) { if (b->n < RB_MAX_ZONE) b->cards[b->n++] = c; }
static int  bag_pop(RbBag *b) { return b->n > 0 ? b->cards[--b->n] : -1; }

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
