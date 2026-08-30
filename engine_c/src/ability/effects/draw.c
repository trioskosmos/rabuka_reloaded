#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* Ported from engine/src/ability/effects/draw.rs
    draw_cards_for_player: draw `count` cards from source deck to destination,
    handling card_type filter, distinct, deck refresh (rule 10.2.1) and
    place_card_in_zone. card_type_filter is a string match on
    rb_card_is_live / is_energy; distinct ignored for now. */

int rb_draw_cards_for_player(RbPlayer *player, uint8_t count, const char *source,
                             const char *destination, const char *card_type_filter,
                             int is_any_number, void *distinct, void *card_db, int self_target_id){
    (void)distinct; (void)card_db;
    if(is_any_number) return 0;
    int drawn = 0;
    while(drawn < count){
        int card = -1;
        int from_deck = source && (!strcmp(source,"deck")||!strcmp(source,"main_deck")||!strcmp(source,"deck_top")||!strcmp(source,"deck_bottom"));
        int deck_bottom = source && !strcmp(source,"deck_bottom");
        if(from_deck){
            if(player->deck.n>0){
                if(deck_bottom){ card = player->deck.cards[0]; for(int i=1;i<player->deck.n;i++) player->deck.cards[i-1]=player->deck.cards[i]; player->deck.n--; }
                else card = player->deck.cards[--player->deck.n];
            } else {
                /* Q104 / Rule 10.2.1: deck empty mid-draw -> refresh from waitroom */
                if(player->discard.n>0){
                    for(int i=0;i<player->discard.n;i++) player->deck.cards[player->deck.n++] = player->discard.cards[i];
                    player->discard.n = 0;
                    rb_shuffle(player->deck.cards, player->deck.n);
                    player->deck_refreshed_this_turn = 1;
                    continue;
                }
                break;
            }
        } else if(source && (!strcmp(source,"discard")||!strcmp(source,"waitroom"))){
            if(player->discard.n>0) card = player->discard.cards[--player->discard.n];
            else break;
        } else if(source && !strcmp(source,"hand")){
            if(player->hand.n>0) card = player->hand.cards[--player->hand.n];
            else break;
        } else if(source && !strcmp(source,"energy")){
            if(player->energy.n>0) card = player->energy.cards[--player->energy.n];
            else break;
        } else if(source && (!strcmp(source,"success")||!strcmp(source,"success_zone")||!strcmp(source,"success_live_zone")||!strcmp(source,"success_live_card_zone"))){
            if(player->success.n>0) card = player->success.cards[--player->success.n];
            else break;
        } else if(source && (!strcmp(source,"staged")||!strcmp(source,"stage"))){
            for(int i=0;i<RB_STAGE_SIZE;i++) if(player->stage[i]!=RB_EMPTY_SLOT){ card=player->stage[i]; player->stage[i]=RB_EMPTY_SLOT; break; }
            if(card==-1) break;
        } else {
            /* resolution_zone / revealed_cards / unknown: not ported (revealed pool
                lives in GameState, not RbPlayer; resolution not tracked) */
            break;
        }
        if(card==-1) break;
        /* card_type filter */
        int matches = 1;
        if(card_type_filter){
            if(!strcmp(card_type_filter,"live_card")) matches = rb_card_is_live(card);
            else if(!strcmp(card_type_filter,"member_card")) matches = !rb_card_is_live(card) && !rb_card_is_energy(card);
            else if(!strcmp(card_type_filter,"energy_card")) matches = rb_card_is_energy(card);
            /* exclude self */
            if(self_target_id!=-1 && card==self_target_id) matches = 0;
        }
        if(matches){
            const char *dst = destination ? destination : "hand";
            RbZone z;
            if(rb_zone_of_str(dst,&z)==0){
                /* fallback to hand */
                if(player->hand.n < RB_MAX_ZONE) player->hand.cards[player->hand.n++] = card;
            } else {
                /* place in zone */
                if(z==RB_ZONE_HAND && player->hand.n < RB_MAX_ZONE) player->hand.cards[player->hand.n++] = card;
                else if(z==RB_ZONE_DISCARD && player->discard.n < RB_MAX_ZONE) player->discard.cards[player->discard.n++] = card;
                else if(z==RB_ZONE_DECK && player->deck.n < RB_MAX_ZONE) player->deck.cards[player->deck.n++] = card;
                else if(z==RB_ZONE_ENERGY && player->energy.n < RB_MAX_ZONE) player->energy.cards[player->energy.n++] = card;
                else if(z==RB_ZONE_LIVE && player->live.n < RB_MAX_ZONE) player->live.cards[player->live.n++] = card;
                else if(z==RB_ZONE_SUCCESS && player->success.n < RB_MAX_ZONE) player->success.cards[player->success.n++] = card;
                else if(player->hand.n < RB_MAX_ZONE) player->hand.cards[player->hand.n++] = card;
            }
            drawn++;
        } else {
            /* not matching -> put back on the source pile bottom */
            if(from_deck){
                for(int i=player->deck.n;i>0;i--) player->deck.cards[i]=player->deck.cards[i-1];
                player->deck.cards[0]=card; player->deck.n++;
            } else if(source && (!strcmp(source,"discard")||!strcmp(source,"waitroom"))){
                if(player->discard.n<RB_MAX_ZONE) player->discard.cards[player->discard.n++]=card;
            } else if(source && !strcmp(source,"hand")){
                if(player->hand.n<RB_MAX_ZONE) player->hand.cards[player->hand.n++]=card;
            } else if(source && !strcmp(source,"energy")){
                if(player->energy.n<RB_MAX_ZONE) player->energy.cards[player->energy.n++]=card;
            } else if(source && (!strcmp(source,"success")||!strcmp(source,"success_zone")||!strcmp(source,"success_live_zone")||!strcmp(source,"success_live_card_zone"))){
                if(player->success.n<RB_MAX_ZONE) player->success.cards[player->success.n++]=card;
            }
            /* stage source: a non-matching staged draw simply returns to its slot */
        }
    }
    return drawn;
}

/* Mirror engine.c:target_player  Eresolve the effect's target player index. */
static int draw_target_player(const AbilityEffect *e, int actor) {
    if (e && e->target) {
        if (!strcmp(e->target, "opponent")) return actor ^ 1;
        if (!strcmp(e->target, "both") || !strcmp(e->target, "either")) return actor;
    }
    return actor;
}

/* Mirror draw.rs:execute_draw_wrapper  Eresolve the count (static / dynamic /
    zero-special), then run the draw via execute_draw semantics. */
int rb_effect_draw_card(GameState *g, int actor, AbilityEffect *e, int host_cid) {
    if (!g || !e) return 0;
    const char *act = e->action;

    /* pull extras */
    int is_any_number = 0, is_self_target = 0, per_unit = 0, per_unit_count = 1;
    const char *per_unit_type = NULL, *per_unit_source = NULL;
    const char *source = e->source, *destination = e->destination, *card_type = NULL;
    for (int i = 0; i < e->n_extra; i++) {
        const char *k = e->extra_k[i], *v = e->extra_v[i];
        if (!k) continue;
        if (!strcmp(k, "any_number") && v && !strcmp(v, "true")) is_any_number = 1;
        else if (!strcmp(k, "self_target") && v && !strcmp(v, "true")) is_self_target = 1;
        else if (!strcmp(k, "per_unit") && v && !strcmp(v, "true")) per_unit = 1;
        else if (!strcmp(k, "per_unit_count")) per_unit_count = v ? atoi(v) : 1;
        else if (!strcmp(k, "per_unit_type")) per_unit_type = v;
        else if (!strcmp(k, "per_unit_source")) per_unit_source = v;
        else if (!strcmp(k, "source")) source = v;
        else if (!strcmp(k, "destination")) destination = v;
        else if (!strcmp(k, "card_type")) card_type = v;
    }
    if (!source) {
        /* Rust: card_type==Member && source none => Stage; else Deck */
        if (card_type && !strcmp(card_type, "member_card")) source = "stage";
        else source = "deck_top";
    }
    if (!destination) destination = "hand";

    /* count resolution (mirror execute_draw_wrapper) */
    int final_count;
    if (e->count < 0) {
        final_count = rb_effect_count(g, actor, host_cid, e, g->last_draw_count);
    } else if (e->count == 0) {
        /* Rust: when count is 0, draw = moved_cards (then recently_moved, then
            last_cost_discard_count). C tracks both recently_moved and
            mods.last_cost_discard_count. */
        final_count = g->n_recently_moved > 0 ? g->n_recently_moved
                     : g->mods.last_cost_discard_count;
    } else {
        final_count = e->count;
    }

    /* per_unit multiplier (mirror execute_draw::final_count) */
    if (per_unit) {
        int multiplier = 1;
        if (per_unit_type && !strcmp(per_unit_type, "discard")) {
            /* Rust: discard_count / per_unit_count over tracked moves. C tracks
                recently_moved; divide that (best-effort) by per_unit_count. */
            int disc = g->n_recently_moved;
            multiplier = per_unit_count > 0 ? disc / per_unit_count : disc;
        } else if (per_unit_source && !strcmp(per_unit_source, "this_cost_waited")) {
            /* Rust counts members waited by this ability's own cost. C has no
                per-cost waited tracking; approximate with 1. */
            multiplier = 1;
        } else {
            multiplier = 1;
        }
        final_count = final_count * multiplier * (per_unit_count > 0 ? per_unit_count : 1);
    }

    /* draw_until_count: draw up to target_count (hand-based) */
    if (act && !strcmp(act, "draw_until_count")) {
        int pl = draw_target_player(e, actor);
        int have = g->p[pl].hand.n;
        int to_draw = final_count - have;
        if (to_draw < 0) to_draw = 0;
        int n = rb_draw_cards_for_player(&g->p[pl], (uint8_t)to_draw, source, destination,
                                         card_type, 0, NULL, NULL, -1);
        g->last_draw_count = n;
        return n;
    }

    /* optional draw: emit a pay/skip gate (mirror emit_pay_skip_gate). The draw
        is performed on resume, not by re-executing the effect. */
    if (e->is_optional) {
        int tgt = draw_target_player(e, actor);
        if (act && !strcmp(act, "draw_until_count")) tgt = draw_target_player(e, actor);
        g->queue.resume_draw_count = final_count;
        g->queue.resume_draw_target = (e->target && !strcmp(e->target, "both")) ? 2 : tgt;
        strncpy(g->queue.resume_draw_source, source ? source : "deck_top", sizeof(g->queue.resume_draw_source)-1);
        strncpy(g->queue.resume_draw_dest, destination, sizeof(g->queue.resume_draw_dest)-1);
        strncpy(g->queue.resume_draw_ctype, card_type ? card_type : "", sizeof(g->queue.resume_draw_ctype)-1);
        g->queue.resume_draw_self_id = is_self_target ? host_cid : -1;
        /* resume_parent / resume_child are stashed by rb_execute_effect_ex (mirrors
            the optional-cost gate) so remaining sibling effects continue on resume. */
        g->queue.resume_mode = 4;
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, final_count, 1, "draw:skip");
        return 0;
    }

    /* any_number: Rust emits a choice; headless draws everything available. */
    if (is_any_number) {
        int tgt = draw_target_player(e, actor);
        int n = rb_draw_cards_for_player(&g->p[tgt], 99, source, destination, card_type, 0, NULL, NULL, -1);
        g->last_draw_count = n;
        return n;
    }

    /* target resolution (mirror execute_draw) */
    if (e->target && !strcmp(e->target, "both")) {
        int n0 = rb_draw_cards_for_player(&g->p[0], (uint8_t)final_count, source, destination, card_type, 0, NULL, NULL, -1);
        int n1 = rb_draw_cards_for_player(&g->p[1], (uint8_t)final_count, source, destination, card_type, 0, NULL, NULL, -1);
        g->last_draw_count = n0 + n1;
        return g->last_draw_count;
    }

    int target = draw_target_player(e, actor);

    /* self_target: activating card must be on target's stage */
    if (is_self_target) {
        if (host_cid >= 0) {
            int on_stage = 0;
            for (int s = 0; s < RB_STAGE_SIZE; s++)
                if (g->p[target].stage[s] == host_cid) { on_stage = 1; break; }
            if (!on_stage) return 0; /* Rust: Err -> no draw */
            int n = rb_draw_cards_for_player(&g->p[target], (uint8_t)final_count, source, destination,
                                             card_type, 0, NULL, NULL, host_cid);
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

/* Mirror draw.rs:execute_draw_wrapper  Ethin resolver-facing entry point. */
int rb_execute_draw_wrapper(GameState *g, int actor, AbilityEffect *e, int host_cid) {
    return rb_effect_draw_card(g, actor, e, host_cid);
}
