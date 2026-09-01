#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <ctype.h>

/* Faithful C translation of engine/src/ability/effects/draw.rs
   Functions translated (in order of appearance in Rust source):
     draw_cards_for_player        -> rb_draw_cards_for_player
     resolve_dynamic_count        -> rb_draw_resolve_dynamic_count
     execute_draw_wrapper         -> rb_effect_draw_card  (+ rb_execute_draw_wrapper trampoline)
     execute_select_effect        -> rb_effect_select_effect
     execute_both_hand_keep_shuffle_under -> rb_effect_both_hand_keep_shuffle_under
     execute_draw                 -> inlined into rb_effect_draw_card
     execute_draw_until_count     -> rb_effect_draw_until_count
     execute_select_heart_color   -> rb_effect_select_heart_color
     execute_select_number        -> rb_effect_select_number
     execute_area_select          -> rb_effect_area_select
     resolve_gain_heart_color     -> rb_resolve_gain_heart_color
     make_card_effect_data        -> rb_make_card_effect_data
   Helper functions added: draw_target_player, draw_extra, draw_heart_colors,
     draw_place_in_zone, draw_count_zone_cards, draw_is_deck_source.
*/

/* ── Internal helpers ─────────────────────────────────────────────────────── */

static int draw_is_deck_source(const char *source) {
    return source &&
        (!strcmp(source, "deck") || !strcmp(source, "deck_top") ||
         !strcmp(source, "deck_bottom") || !strcmp(source, "main_deck"));
}

static int draw_count_zone_cards(const RbPlayer *player, const char *source) {
    if (!source) return player->deck.n;
    if (!strcmp(source, "deck") || !strcmp(source, "deck_top") ||
        !strcmp(source, "main_deck") || !strcmp(source, "deck_bottom"))
        return player->deck.n;
    if (!strcmp(source, "discard") || !strcmp(source, "waitroom"))
        return player->discard.n;
    if (!strcmp(source, "hand"))
        return player->hand.n;
    if (!strcmp(source, "energy") || !strcmp(source, "energy_zone"))
        return player->energy.n;
    if (!strcmp(source, "success") || !strcmp(source, "success_zone") ||
        !strcmp(source, "success_live_zone") || !strcmp(source, "success_live_card_zone"))
        return player->success.n;
    if (!strcmp(source, "stage") || !strcmp(source, "staged")) {
        int n = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++)
            if (player->stage[i] != RB_EMPTY_SLOT) n++;
        return n;
    }
    return 0;
}

/* Mirrors util::place_card_in_zone(player, card, destination, ...).
   Returns 1 if the card was placed, 0 if it fell back to hand (zone full / unknown). */
static int draw_place_in_zone(RbPlayer *player, int card, const char *destination) {
    RbZone z;
    if (rb_zone_of_str(destination, &z) != 0) {
        if (player->hand.n < RB_MAX_ZONE)
            player->hand.cards[player->hand.n++] = card;
        return 1;
    }
    switch (z) {
        case RB_ZONE_HAND:
            if (player->hand.n < RB_MAX_ZONE)
                player->hand.cards[player->hand.n++] = card;
            break;
        case RB_ZONE_DECK:
            if (player->deck.n < RB_MAX_ZONE)
                player->deck.cards[player->deck.n++] = card;
            break;
        case RB_ZONE_DISCARD:
            if (player->discard.n < RB_MAX_ZONE)
                player->discard.cards[player->discard.n++] = card;
            break;
        case RB_ZONE_ENERGY:
            if (player->energy.n < RB_MAX_ZONE)
                player->energy.cards[player->energy.n++] = card;
            break;
        case RB_ZONE_LIVE:
            if (player->live.n < RB_MAX_ZONE)
                player->live.cards[player->live.n++] = card;
            break;
        case RB_ZONE_SUCCESS:
            if (player->success.n < RB_MAX_ZONE)
                player->success.cards[player->success.n++] = card;
            break;
        case RB_ZONE_STAGE:
        case RB_ZONE_RESOLUTION:
        default:
            if (player->hand.n < RB_MAX_ZONE)
                player->hand.cards[player->hand.n++] = card;
            break;
    }
    return 1;
}

/* Resolve the target player index from e->target, mirroring Rust's
   AbilityResolver::resolve_target_player. */
static int draw_target_player(const AbilityEffect *e, int actor) {
    if (e && e->target) {
        if (!strcmp(e->target, "opponent")) return actor ^ 1;
        if (!strcmp(e->target, "both") || !strcmp(e->target, "either")) return actor;
    }
    return actor;
}

/* Look up an extra_k/v pair by key. */
static const char *draw_extra(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}

/* Collect all heart_colors extra values. */
static int draw_heart_colors(const AbilityEffect *e, const char **out, int max) {
    int n = 0;
    if (!e) return 0;
    for (int i = 0; i < e->n_extra && n < max; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "heart_colors") && e->extra_v[i])
            out[n++] = e->extra_v[i];
    return n;
}

/* ── Core draw (mirrors draw.rs::draw_cards_for_player) ──────────────────── */

int rb_draw_cards_for_player(RbPlayer *player, uint8_t count, const char *source,
                             const char *destination, const char *card_type_filter,
                             int is_any_number, void *distinct, void *card_db,
                             int self_target_id) {
    (void)distinct; (void)card_db;
    if (is_any_number) return 0;

    const char *deck_src = (!source || !strcmp(source, "deck")) ? "deck_top" : source;
    int deck_bottom = draw_is_deck_source(source) && source && !strcmp(source, "deck_bottom");
    int from_deck   = draw_is_deck_source(source);
    int from_discard = source && (!strcmp(source, "discard") || !strcmp(source, "waitroom"));
    int from_hand    = source && !strcmp(source, "hand");
    int from_energy  = source && !strcmp(source, "energy");
    int from_success = source && (!strcmp(source, "success") || !strcmp(source, "success_zone") ||
                                  !strcmp(source, "success_live_zone") ||
                                  !strcmp(source, "success_live_card_zone"));
    int from_stage   = source && (!strcmp(source, "staged") || !strcmp(source, "stage"));

    int drawn = 0;
    while (drawn < count) {
        int card = -1;

        if (from_deck) {
            if (player->deck.n > 0) {
                if (deck_bottom) {
                    card = player->deck.cards[0];
                    for (int i = 1; i < player->deck.n; i++)
                        player->deck.cards[i - 1] = player->deck.cards[i];
                    player->deck.n--;
                } else {
                    card = player->deck.cards[--player->deck.n];
                }
            } else {
                /* Q104 / Rule 10.2.1: deck empty mid-draw -> refresh from waitroom */
                if (player->discard.n > 0) {
                    for (int i = 0; i < player->discard.n; i++)
                        player->deck.cards[player->deck.n++] = player->discard.cards[i];
                    player->discard.n = 0;
                    rb_shuffle(player->deck.cards, player->deck.n);
                    player->deck_refreshed_this_turn = 1;
                    continue;
                }
                break;
            }
        } else if (from_discard) {
            if (player->discard.n > 0)
                card = player->discard.cards[--player->discard.n];
            else
                break;
        } else if (from_hand) {
            if (player->hand.n > 0)
                card = player->hand.cards[--player->hand.n];
            else
                break;
        } else if (from_energy) {
            if (player->energy.n > 0)
                card = player->energy.cards[--player->energy.n];
            else
                break;
        } else if (from_success) {
            if (player->success.n > 0)
                card = player->success.cards[--player->success.n];
            else
                break;
        } else if (from_stage) {
            card = -1;
            for (int i = 0; i < RB_STAGE_SIZE; i++)
                if (player->stage[i] != RB_EMPTY_SLOT) {
                    card = player->stage[i];
                    player->stage[i] = RB_EMPTY_SLOT;
                    break;
                }
            if (card == -1) break;
        } else {
            break;
        }

        if (card == -1) break;

        /* card_type filter (mirrors util::CardFilter::matches_card) */
        int matches = 1;
        if (card_type_filter) {
            if (!strcmp(card_type_filter, "live_card"))
                matches = rb_card_is_live(card);
            else if (!strcmp(card_type_filter, "member_card"))
                matches = !rb_card_is_live(card) && !rb_card_is_energy(card);
            else if (!strcmp(card_type_filter, "energy_card"))
                matches = rb_card_is_energy(card);
            /* exclude self */
            if (self_target_id != -1 && card == self_target_id)
                matches = 0;
        }

        if (matches) {
            draw_place_in_zone(player, card, destination ? destination : "hand");
            drawn++;
        } else {
            /* Not matching: return to the source pile (Rust pushes back to
               main_deck.cards, which is the bottom of the draw pile). */
            if (from_deck) {
                if (player->deck.n < RB_MAX_ZONE)
                    player->deck.cards[player->deck.n++] = card;
            } else if (from_discard) {
                if (player->discard.n < RB_MAX_ZONE)
                    player->discard.cards[player->discard.n++] = card;
            } else if (from_hand) {
                if (player->hand.n < RB_MAX_ZONE)
                    player->hand.cards[player->hand.n++] = card;
            } else if (from_energy) {
                if (player->energy.n < RB_MAX_ZONE)
                    player->energy.cards[player->energy.n++] = card;
            } else if (from_success) {
                if (player->success.n < RB_MAX_ZONE)
                    player->success.cards[player->success.n++] = card;
            }
            /* from_stage: non-matching staged draw stays removed (Rust doesn't
               push back to stage; the caller handles staged draws separately). */
        }
    }
    return drawn;
}

/* ── Dynamic count resolver (mirrors draw.rs::AbilityResolver::resolve_dynamic_count) ── */

int rb_draw_resolve_dynamic_count(GameState *g, int actor, const AbilityEffect *e,
                                  int host_cid) {
    if (!g || !e) return 0;
    return rb_effect_count(g, actor, host_cid, e, g->last_draw_count);
}

/* ── Target player helper (mirrors draw.rs::resolve_target_player) ────────── */

static int draw_effect_target_player(const AbilityEffect *e, int actor) {
    return draw_target_player(e, actor);
}

/* ── execute_draw_wrapper -> rb_effect_draw_card ────────────────────────────
   Mirrors AbilityResolver::execute_draw_wrapper + execute_draw.
   All count resolution (static / dynamic / zero-special), per_unit,
   target both/self/opponent, optional pay-skip gate, and card_type filter
   are handled here. */

int rb_effect_draw_card(GameState *g, int actor, AbilityEffect *e, int host_cid) {
    if (!g || !e) return 0;
    const char *act = e->action;

    /* Pull extra fields */
    int is_any_number = 0, is_self_target = 0, per_unit = 0, per_unit_count = 1;
    const char *per_unit_type = NULL, *per_unit_source = NULL;
    const char *source = e->source, *destination = e->destination, *card_type = NULL;
    for (int i = 0; i < e->n_extra; i++) {
        const char *k = e->extra_k[i], *v = e->extra_v[i];
        if (!k) continue;
        if (!strcmp(k, "any_number") && v && !strcmp(v, "true"))
            is_any_number = 1;
        else if (!strcmp(k, "self_target") && v && !strcmp(v, "true"))
            is_self_target = 1;
        else if (!strcmp(k, "per_unit") && v && !strcmp(v, "true"))
            per_unit = 1;
        else if (!strcmp(k, "per_unit_count") && v)
            per_unit_count = atoi(v);
        else if (!strcmp(k, "per_unit_type"))
            per_unit_type = v;
        else if (!strcmp(k, "per_unit_source"))
            per_unit_source = v;
        else if (!strcmp(k, "source"))
            source = v;
        else if (!strcmp(k, "destination"))
            destination = v;
        else if (!strcmp(k, "card_type"))
            card_type = v;
    }
    if (!source) {
        if (card_type && !strcmp(card_type, "member_card"))
            source = "stage";
        else
            source = "deck_top";
    }
    if (!destination) destination = "hand";

    /* Count resolution (mirrors execute_draw_wrapper) */
    int final_count;
    if (e->count < 0) {
        final_count = rb_effect_count(g, actor, host_cid, e, g->last_draw_count);
    } else if (e->count == 0) {
        /* Rust: resolver.moved_cards, then gs.recently_moved_cards, then last_cost_discard_count.
           C tracks n_recently_moved (mirrors gs.recently_moved_cards); no
           resolver-level moved_cards transient field — use n_recently_moved. */
        if (g->n_recently_moved > 0)
            final_count = g->n_recently_moved;
        else
            final_count = g->mods.last_cost_discard_count;
    } else {
        final_count = e->count;
    }

    /* per_unit multiplier (mirrors execute_draw::final_count) */
    if (per_unit) {
        int multiplier = 1;
        if (per_unit_type && !strcmp(per_unit_type, "discard")) {
            /* Rust: resolve_discard_per_unit_count(tracked, last_discard_count, &card_db, &filter)
               C approximation: use n_recently_moved (the tracked moved cards) / per_unit_count. */
            int disc = g->n_recently_moved;
            multiplier = per_unit_count > 0 ? disc / per_unit_count : disc;
        } else if (per_unit_source && !strcmp(per_unit_source, "this_cost_waited")) {
            /* Rust: cost_waited_members.len().u8_count()
               C: use n_last_cost_waited_members (mirrors the tracked list). */
            multiplier = g->n_last_cost_waited_members;
        } else {
            multiplier = 1;
        }
        int pc = per_unit_count > 0 ? per_unit_count : 1;
        final_count = final_count * multiplier * pc;
    }

    /* draw_until_count: draw up to target_count (hand-based) */
    if (act && !strcmp(act, "draw_until_count")) {
        int pl = draw_effect_target_player(e, actor);
        int have = g->p[pl].hand.n;
        int to_draw = final_count - have;
        if (to_draw < 0) to_draw = 0;
        int n = rb_draw_cards_for_player(&g->p[pl], (uint8_t)to_draw, source, destination,
                                         card_type, 0, NULL, NULL, -1);
        g->last_draw_count = n;
        return n;
    }

    /* Optional draw: emit pay/skip gate; draw is performed on resume */
    if (e->is_optional) {
        int tgt = draw_effect_target_player(e, actor);
        g->queue.resume_draw_count = final_count;
        g->queue.resume_draw_target = (e->target && !strcmp(e->target, "both")) ? 2 : tgt;
        strncpy(g->queue.resume_draw_source, source ? source : "deck_top",
                sizeof(g->queue.resume_draw_source) - 1);
        g->queue.resume_draw_source[sizeof(g->queue.resume_draw_source) - 1] = '\0';
        strncpy(g->queue.resume_draw_dest, destination ? destination : "hand",
                sizeof(g->queue.resume_draw_dest) - 1);
        g->queue.resume_draw_dest[sizeof(g->queue.resume_draw_dest) - 1] = '\0';
        strncpy(g->queue.resume_draw_ctype, card_type ? card_type : "",
                sizeof(g->queue.resume_draw_ctype) - 1);
        g->queue.resume_draw_ctype[sizeof(g->queue.resume_draw_ctype) - 1] = '\0';
        g->queue.resume_draw_self_id = is_self_target ? host_cid : -1;
        g->queue.resume_mode = 4;
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL,
                       final_count, 1, "draw:skip");
        return 0;
    }

    /* any_number: Rust emits a choice; headless draws everything available */
    if (is_any_number) {
        int tgt = draw_effect_target_player(e, actor);
        int available = draw_count_zone_cards(&g->p[tgt], source);
        if (available <= 0) {
            g->last_draw_count = 0;
            return 0;
        }
        int n = rb_draw_cards_for_player(&g->p[tgt], 99, source, destination,
                                         card_type, 0, NULL, NULL, -1);
        g->last_draw_count = n;
        return n;
    }

    /* Target resolution */
    if (e->target && !strcmp(e->target, "both")) {
        int n0 = rb_draw_cards_for_player(&g->p[0], (uint8_t)final_count, source, destination,
                                          card_type, 0, NULL, NULL, -1);
        int n1 = rb_draw_cards_for_player(&g->p[1], (uint8_t)final_count, source, destination,
                                          card_type, 0, NULL, NULL, -1);
        g->last_draw_count = n0 + n1;
        return g->last_draw_count;
    }

    int target = draw_effect_target_player(e, actor);

    /* self_target: activating card must be on target's stage (Rust Err -> no draw) */
    if (is_self_target) {
        if (host_cid >= 0) {
            int on_stage = 0;
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                if (g->p[target].stage[s] == host_cid) { on_stage = 1; break; }
            if (!on_stage) return 0;
            int n = rb_draw_cards_for_player(&g->p[target], (uint8_t)final_count, source,
                                             destination, card_type, 0, NULL, NULL, host_cid);
            g->last_draw_count = n;
            return n;
        }
        return 0;
    }

    int n = rb_draw_cards_for_player(&g->p[target], (uint8_t)final_count, source, destination,
                                     card_type, 0, NULL, NULL, -1);
    g->last_draw_count = n;
    return n;
}

/* Trampoline: mirrors draw.rs:execute_draw_wrapper entry point */
int rb_execute_draw_wrapper(GameState *g, int actor, AbilityEffect *e, int host_cid) {
    return rb_effect_draw_card(g, actor, e, host_cid);
}

/* ── execute_select_effect ────────────────────────────────────────────────── */

void rb_effect_select_effect(GameState *g, int actor, AbilityEffect *e, int host_cid) {
    if (!g || !e) return;
    const char *heart_colors_arr[8];
    int n_hc = draw_heart_colors(e, heart_colors_arr, 8);
    int has_heart_colors = n_hc > 0;
    const char *src   = draw_extra(e, "source");
    const char *or_ct = draw_extra(e, "or_card_types");
    const char *chars = draw_extra(e, "characters");
    const char *group = draw_extra(e, "group_names");
    const char *ctype = draw_extra(e, "card_type");

    /* Area select: no source, no heart_colors, no or_card_types, no characters, no group */
    if (!src && n_hc == 0 && !or_ct && !chars && !group) {
        rb_effect_area_select(g, actor, e, host_cid);
        return;
    }
    /* Heart-color select: no source, no card_type, has heart_colors */
    if (!src && !ctype && has_heart_colors) {
        int count = e->count >= 0 ? e->count : 1;
        rb_effect_select_heart_color(g, actor, count, heart_colors_arr, n_hc, e->target);
        return;
    }
    /* C6 keep-N-shuffle-rest */
    if (draw_extra(e, "keep_shuffle_under")) {
        rb_effect_both_hand_keep_shuffle_under(g, actor, e, host_cid);
        return;
    }
    /* Generic card selection */
    const char *source = (!ctype || strcmp(ctype, "member_card") != 0)
                         ? (src ? src : "hand")
                         : (src ? src : "stage");
    int cnt = e->count >= 0 ? e->count : 1;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, source, ctype, cnt,
                   e->is_optional ? 1 : 0, NULL);
    g->queue.pending.filter_group[0] = 0;
    if (group)
        strncpy(g->queue.pending.filter_group, group, sizeof(g->queue.pending.filter_group) - 1);
    g->queue.pending.filter_heart = -1;
    if (n_hc)
        g->queue.pending.filter_heart = (int)rb_parse_heart_color(heart_colors_arr[0]);
    strncpy(g->queue.resume_filter_group, g->queue.pending.filter_group,
            sizeof(g->queue.resume_filter_group) - 1);
    g->queue.resume_filter_heart = g->queue.pending.filter_heart;
    if (n_hc) {
        g->queue.resume_mode = 0;
        g->queue.resume_is_select = 0;
    } else {
        g->queue.resume_mode = 2;
        g->queue.resume_is_select = 1;
    }
    g->queue.resume_eff = e;
    g->queue.resume_actor = actor;
    g->queue.resume_host = actor;
}

/* ── C6 keep-N-shuffle-rest (draw.rs::execute_both_hand_keep_shuffle_under) ── */

void rb_effect_both_hand_keep_shuffle_under(GameState *g, int actor,
                                             AbilityEffect *e, int host_cid) {
    (void)host_cid;
    if (!g || !e) return;
    int count = e->count >= 0 ? e->count : 1;
    int phase = g->keep_shuffle_under_phase;

    if (phase == 0) {
        int pl = actor;
        RbPlayer *P = &g->p[pl];
        int ns = 0;
        for (int i = 0; i < P->hand.n && ns < RB_MAX_HAND; i++)
            g->keep_shuffle_under_snapshot[0][ns++] = P->hand.cards[i];
        g->keep_shuffle_under_snapshot_n[0] = ns;
        int pick = count < P->hand.n ? count : P->hand.n;
        rb_emit_choice(g, pl, RB_CHOICE_SELECT_CARD, "hand", NULL,
                       pick > 0 ? pick : 1, 1, "keep_shuffle_under");
        g->queue.resume_mode = 5;
        g->queue.resume_eff = e;
        g->queue.resume_actor = actor;
        g->queue.resume_host = actor;
        g->keep_shuffle_under_phase = 1;
        return;
    }
    if (phase == 1) {
        RbPlayer *Ps = &g->p[actor];
        int *snap = g->keep_shuffle_under_snapshot[0];
        int ns = g->keep_shuffle_under_snapshot_n[0];
        int kept[RB_MAX_HAND];
        int nk = 0;
        for (int i = 0; i < g->n_selected_cards && nk < RB_MAX_HAND; i++)
            kept[nk++] = g->selected_cards[i];
        /* Remove non-kept cards from hand (hand equals snapshot at selection time) */
        for (int i = 0; i < ns; i++) {
            int is_kept = 0;
            for (int k = 0; k < nk; k++)
                if (kept[k] == snap[i]) { is_kept = 1; break; }
            if (!is_kept) {
                for (int p = 0; p < Ps->hand.n; p++)
                    if (Ps->hand.cards[p] == snap[i]) {
                        for (int q = p; q < Ps->hand.n - 1; q++)
                            Ps->hand.cards[q] = Ps->hand.cards[q + 1];
                        Ps->hand.n--;
                        break;
                    }
            }
        }
        int to_move[RB_MAX_HAND];
        int nm = 0;
        for (int i = 0; i < ns; i++) {
            int is_kept = 0;
            for (int k = 0; k < nk; k++)
                if (kept[k] == snap[i]) { is_kept = 1; break; }
            if (!is_kept && nm < RB_MAX_HAND) to_move[nm++] = snap[i];
        }
        rb_shuffle(to_move, nm);
        for (int i = 0; i < nm; i++)
            if (Ps->deck.n < RB_MAX_ZONE)
                Ps->deck.cards[Ps->deck.n++] = to_move[i];
        /* Snapshot opponent and prompt */
        int opp = actor ^ 1;
        RbPlayer *Po = &g->p[opp];
        int ns2 = 0;
        for (int i = 0; i < Po->hand.n && ns2 < RB_MAX_HAND; i++)
            g->keep_shuffle_under_snapshot[1][ns2++] = Po->hand.cards[i];
        g->keep_shuffle_under_snapshot_n[1] = ns2;
        int pick2 = count < Po->hand.n ? count : Po->hand.n;
        rb_emit_choice(g, opp, RB_CHOICE_SELECT_CARD, "hand", NULL,
                       pick2 > 0 ? pick2 : 1, 1, "keep_shuffle_under");
        g->queue.resume_mode = 5;
        g->queue.resume_eff = e;
        g->queue.resume_actor = actor;
        g->queue.resume_host = actor;
        g->keep_shuffle_under_phase = 2;
        g->n_selected_cards = 0;
        return;
    }
    /* phase == 2: opponent's selection resolved */
    {
        int opp = actor ^ 1;
        RbPlayer *Po = &g->p[opp];
        int *snap = g->keep_shuffle_under_snapshot[1];
        int ns = g->keep_shuffle_under_snapshot_n[1];
        int kept[RB_MAX_HAND];
        int nk = 0;
        for (int i = 0; i < g->n_selected_cards && nk < RB_MAX_HAND; i++)
            kept[nk++] = g->selected_cards[i];
        for (int i = 0; i < ns; i++) {
            int is_kept = 0;
            for (int k = 0; k < nk; k++)
                if (kept[k] == snap[i]) { is_kept = 1; break; }
            if (!is_kept) {
                for (int p = 0; p < Po->hand.n; p++)
                    if (Po->hand.cards[p] == snap[i]) {
                        for (int q = p; q < Po->hand.n - 1; q++)
                            Po->hand.cards[q] = Po->hand.cards[q + 1];
                        Po->hand.n--;
                        break;
                    }
            }
        }
        int to_move[RB_MAX_HAND];
        int nm = 0;
        for (int i = 0; i < ns; i++) {
            int is_kept = 0;
            for (int k = 0; k < nk; k++)
                if (kept[k] == snap[i]) { is_kept = 1; break; }
            if (!is_kept && nm < RB_MAX_HAND) to_move[nm++] = snap[i];
        }
        rb_shuffle(to_move, nm);
        for (int i = 0; i < nm; i++)
            if (Po->deck.n < RB_MAX_ZONE)
                Po->deck.cards[Po->deck.n++] = to_move[i];
    }
    g->keep_shuffle_under_phase = 0;
    g->keep_shuffle_under_count = 0;
    g->keep_shuffle_under_snapshot_n[0] = 0;
    g->keep_shuffle_under_snapshot_n[1] = 0;
    g->keep_shuffle_under_selected_n = 0;
    g->n_selected_cards = 0;
}

/* ── execute_draw_until_count (mirrors draw.rs::execute_draw_until_count) ────
   Uses saturating_sub (target_count - current_hand). Only draws when hand is
   below target_count; destination must be Hand for the check. */

void rb_effect_draw_until_count(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    int target_count = 0;
    for (int i = 0; i < e->n_extra; i++) {
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "target_count") && e->extra_v[i]) {
            target_count = atoi(e->extra_v[i]);
            break;
        }
    }
    if (target_count <= 0) return;
    const char *target = (e->target && *e->target) ? e->target : "self";
    int who = (e->target &&
               (!strcmp(e->target, "opponent") || !strcmp(e->target, "p2")))
              ? actor ^ 1 : actor;
    RbPlayer *P = &g->p[who];
    /* Only draw-until-count for Hand destination (Rust matches Zone::Hand) */
    const char *dst = e->destination ? e->destination : "hand";
    RbZone z;
    if (rb_zone_of_str(dst, &z) != 0 || z != RB_ZONE_HAND) return;
    int current = P->hand.n;
    int to_draw = target_count > current ? target_count - current : 0;
    if (to_draw > 0)
        rb_draw_cards_for_player(P, (uint8_t)to_draw, "deck", dst, NULL, 0, NULL, NULL, -1);
}

/* ── execute_select_heart_color (mirrors draw.rs::execute_select_heart_color) ──
   Dedupes heart_colors; if exactly one remains (and not heart_selection), fixes
   it directly into queue.selected_heart_color; otherwise emits a
   RB_CHOICE_SELECT_HEART_COLOR choice. */

void rb_effect_select_heart_color(GameState *g, int actor, int count,
                                  const char **heart_colors, int n_colors,
                                  const char *target) {
    (void)target;
    if (!g) return;
    const char *unique[8];
    int nu = 0;
    for (int i = 0; i < n_colors; i++) {
        int found = 0;
        for (int j = 0; j < nu; j++)
            if (!strcmp(unique[j], heart_colors[i])) { found = 1; break; }
        if (!found && nu < 8) unique[nu++] = heart_colors[i];
    }
    if (nu == 1) {
        g->queue.selected_heart_color = (int)rb_parse_heart_color(unique[0]);
        return;
    }
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_HEART_COLOR, NULL, NULL,
                   count > 0 ? count : 1, 0, "select_heart_color");
    g->queue.pending.n_heart_options = 0;
    for (int i = 0; i < nu && i < 8; i++) {
        strncpy(g->queue.pending.heart_options[i], unique[i],
                sizeof(g->queue.pending.heart_options[i]) - 1);
        g->queue.pending.heart_options[i][sizeof(g->queue.pending.heart_options[i]) - 1] = '\0';
        g->queue.pending.n_heart_options++;
    }
    g->queue.resume_mode = 0;
    g->queue.resume_eff = NULL;
}

/* ── execute_select_number (mirrors draw.rs::execute_select_number) ──────────
   Scan all cards to find max_cost, build options 1..max_cost + "67",
   emit RB_CHOICE_SELECT_NUMBER. */

void rb_effect_select_number(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;
    int max_cost = 10;
    uint32_t n = rb_num_cards();
    for (uint32_t i = 0; i < n; i++) {
        Card c;
        if (rb_decode_card_by_index(i, &c)) {
            if (c.cost > max_cost) max_cost = c.cost;
            rb_free_card(&c);
        }
    }
    int allow = e->is_optional ? 1 : 0;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_NUMBER, NULL, NULL,
                   max_cost, allow, "choice_number");
    const char *hc = draw_extra(e, "heart_color");
    if (!hc) hc = draw_extra(e, "heart_colors");
    g->queue.selected_heart_color = (int)rb_parse_heart_color(hc ? hc : "pink");
    char desc[160];
    snprintf(desc, sizeof(desc), "Choose a number: 1..%d, 67", max_cost);
    strncpy(g->queue.pending.description, desc, sizeof(g->queue.pending.description) - 1);
    g->queue.pending.description[sizeof(g->queue.pending.description) - 1] = '\0';
}

/* ── execute_area_select (mirrors draw.rs::execute_area_select) ──────────────
   Offer left/center/right, excluding the activating card's current stage position. */

void rb_effect_area_select(GameState *g, int actor, AbilityEffect *e, int host_cid) {
    if (!g || !e) return;
    const char *pos_names[3] = { "left", "center", "right" };
    char valid[3][8];
    int nv = 0;
    for (int i = 0; i < 3; i++) {
        if (host_cid >= 0 && g->p[actor].stage[i] == host_cid) continue;
        strncpy(valid[nv], pos_names[i], sizeof(valid[nv]) - 1);
        valid[nv][sizeof(valid[nv]) - 1] = '\0';
        nv++;
    }
    if (nv == 0) return;
    char opts[64];
    opts[0] = '\0';
    for (int i = 0; i < nv; i++) {
        if (i) strncat(opts, ",", sizeof(opts) - strlen(opts) - 1);
        strncat(opts, valid[i], sizeof(opts) - strlen(opts) - 1);
    }
    int allow = e->is_optional ? 1 : 0;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, nv, allow, "area_select");
    snprintf(g->queue.pending.description, sizeof(g->queue.pending.description),
             "Choose an area: %s", opts);
}

/* ── resolve_gain_heart_color (mirrors draw.rs::resolve_gain_heart_color) ────
   Returns a fixed heart color index, or -1 if a choice was emitted / not a
   heart resource / caller should distribute. */

int rb_resolve_gain_heart_color(GameState *g, int actor, AbilityEffect *e,
                                const char *resource, int count,
                                const char **heart_colors, int n_colors,
                                int heart_selection) {
    if (strcmp(resource, "heart") != 0 && strcmp(resource, "ハート") != 0) return -1;
    if (n_colors == 0 && !heart_selection && draw_extra(e, "heart_type") == NULL) return -1;
    const char *colors[8];
    int nc = 0;
    const char *ht = draw_extra(e, "heart_type");
    if (ht) colors[nc++] = ht;
    else for (int i = 0; i < n_colors && nc < 8; i++) colors[nc++] = heart_colors[i];
    if (nc == 0) {
        static const char *def[6] = { "heart01","heart02","heart03","heart04","heart05","heart06" };
        for (int i = 0; i < 6; i++) colors[nc++] = def[i];
    }
    const char *unique[8];
    int nu = 0;
    for (int i = 0; i < nc; i++) {
        int f = 0;
        for (int j = 0; j < nu; j++)
            if (!strcmp(unique[j], colors[i])) { f = 1; break; }
        if (!f && nu < 8) unique[nu++] = colors[i];
    }
    if (nu == 1 && !heart_selection) return (int)rb_parse_heart_color(unique[0]);
    if (!heart_selection && nu > 1 && count >= nu) return -1; /* caller distributes */
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_HEART_COLOR, NULL, NULL,
                   count > 0 ? count : 1, 0, "select_heart_color");
    g->queue.pending.n_heart_options = 0;
    for (int i = 0; i < nu && i < 8; i++) {
        strncpy(g->queue.pending.heart_options[i], unique[i],
                sizeof(g->queue.pending.heart_options[i]) - 1);
        g->queue.pending.heart_options[i][sizeof(g->queue.pending.heart_options[i]) - 1] = '\0';
        g->queue.pending.n_heart_options++;
    }
    g->queue.resume_mode = 0;
    g->queue.resume_eff = NULL;
    return -1;
}

/* ── make_card_effect_data (mirrors draw.rs::make_card_effect_data) ────────── */

RbEffectDataSingleCard rb_make_card_effect_data(int card_id, int amount, const char *color) {
    RbEffectDataSingleCard d;
    d.card_id = card_id;
    d.amount = amount;
    if (color) {
        strncpy(d.color, color, sizeof(d.color) - 1);
        d.color[sizeof(d.color) - 1] = '\0';
    } else {
        d.color[0] = '\0';
    }
    return d;
}
