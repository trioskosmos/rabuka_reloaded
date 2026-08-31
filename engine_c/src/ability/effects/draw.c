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

/* ── Helpers (mirror effect-field readers used by draw.rs::execute_select_effect) ── */
static const char *draw_extra(const AbilityEffect *e, const char *k) {
    if (!e) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], k)) return e->extra_v[i];
    return NULL;
}
static int draw_heart_colors(const AbilityEffect *e, const char **out, int max) {
    int n = 0;
    if (!e) return 0;
    for (int i = 0; i < e->n_extra && n < max; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], "heart_colors") && e->extra_v[i])
            out[n++] = e->extra_v[i];
    return n;
}

/* Mirror draw.rs:execute_select_heart_color — dedupe colors; if exactly one color
    remains (and not a heart_selection), fix it directly into queue.selected_heart_color;
    otherwise emit a SelectHeartColor choice. */
void rb_effect_select_heart_color(GameState *g, int actor, int count,
                                  const char **heart_colors, int n_colors, const char *target) {
    (void)target;
    if (!g) return;
    const char *unique[8]; int nu = 0;
    for (int i = 0; i < n_colors; i++) {
        int found = 0;
        for (int j = 0; j < nu; j++) if (!strcmp(unique[j], heart_colors[i])) { found = 1; break; }
        if (!found && nu < 8) unique[nu++] = heart_colors[i];
    }
    if (nu == 1) {
        g->queue.selected_heart_color = (int)rb_parse_heart_color(unique[0]);
        return;
    }
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_HEART_COLOR, NULL, NULL, count > 0 ? count : 1, 0, "select_heart_color");
    g->queue.pending.n_heart_options = 0;
    for (int i = 0; i < nu && i < 8; i++) {
        strncpy(g->queue.pending.heart_options[i], unique[i], sizeof(g->queue.pending.heart_options[i]) - 1);
        g->queue.pending.n_heart_options++;
    }
    g->queue.resume_mode = 0;
    g->queue.resume_eff = NULL;
}

/* Mirror draw.rs:execute_select_number — offer 1..max_cost plus the sentinel "67". */
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
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_NUMBER, NULL, NULL, max_cost, allow, "choice_number");
    const char *hc = draw_extra(e, "heart_color");
    if (!hc) hc = draw_extra(e, "heart_colors");
    g->queue.selected_heart_color = (int)rb_parse_heart_color(hc ? hc : "pink");
    char desc[160];
    snprintf(desc, sizeof(desc), "Choose a number: 1..%d, 67", max_cost);
    strncpy(g->queue.pending.description, desc, sizeof(g->queue.pending.description) - 1);
}

/* Mirror draw.rs:execute_area_select — offer left/center/right, excluding the
    activating card's current stage position. */
void rb_effect_area_select(GameState *g, int actor, AbilityEffect *e, int host_cid) {
    if (!g || !e) return;
    const char *pos_names[3] = { "left", "center", "right" };
    char valid[3][8]; int nv = 0;
    for (int i = 0; i < 3; i++) {
        if (host_cid >= 0 && g->p[actor].stage[i] == host_cid) continue;
        strncpy(valid[nv], pos_names[i], sizeof(valid[nv]) - 1);
        nv++;
    }
    if (nv == 0) return;
    char opts[64]; opts[0] = 0;
    for (int i = 0; i < nv; i++) {
        if (i) strncat(opts, ",", sizeof(opts) - strlen(opts) - 1);
        strncat(opts, valid[i], sizeof(opts) - strlen(opts) - 1);
    }
    int allow = e->is_optional ? 1 : 0;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, nv, allow, "area_select");
    snprintf(g->queue.pending.description, sizeof(g->queue.pending.description),
             "Choose an area: %s", opts);
}

/* Mirror draw.rs:resolve_gain_heart_color — return a fixed heart color idx, or -1
    if a choice was emitted (or this is not a heart resource). */
int rb_resolve_gain_heart_color(GameState *g, int actor, AbilityEffect *e,
                                const char *resource, int count,
                                const char **heart_colors, int n_colors, int heart_selection) {
    if (strcmp(resource, "heart") != 0 && strcmp(resource, "ハート") != 0) return -1;
    if (n_colors == 0 && !heart_selection && draw_extra(e, "heart_type") == NULL) return -1;
    const char *colors[8]; int nc = 0;
    const char *ht = draw_extra(e, "heart_type");
    if (ht) colors[nc++] = ht;
    else for (int i = 0; i < n_colors && nc < 8; i++) colors[nc++] = heart_colors[i];
    if (nc == 0) {
        const char *def[6] = { "heart01","heart02","heart03","heart04","heart05","heart06" };
        for (int i = 0; i < 6; i++) colors[nc++] = def[i];
    }
    const char *unique[8]; int nu = 0;
    for (int i = 0; i < nc; i++) {
        int f = 0;
        for (int j = 0; j < nu; j++) if (!strcmp(unique[j], colors[i])) { f = 1; break; }
        if (!f && nu < 8) unique[nu++] = colors[i];
    }
    if (nu == 1 && !heart_selection) return (int)rb_parse_heart_color(unique[0]);
    if (!heart_selection && nu > 1 && count >= nu) return -1; /* caller distributes */
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_HEART_COLOR, NULL, NULL, count > 0 ? count : 1, 0, "select_heart_color");
    g->queue.pending.n_heart_options = 0;
    for (int i = 0; i < nu && i < 8; i++) {
        strncpy(g->queue.pending.heart_options[i], unique[i], sizeof(g->queue.pending.heart_options[i]) - 1);
        g->queue.pending.n_heart_options++;
    }
    g->queue.resume_mode = 0;
    g->queue.resume_eff = NULL;
    return -1;
}

/* Mirror draw.rs:execute_select_effect — route a `select` verb to the area /
    heart-color / C6 keep-shuffle / generic card-selection path. */
void rb_effect_select_effect(GameState *g, int actor, AbilityEffect *e, int host_cid) {
    if (!g || !e) return;
    const char *heart_colors[8]; int n_hc = draw_heart_colors(e, heart_colors, 8);
    int has_heart_colors = n_hc > 0;
    const char *src   = draw_extra(e, "source");
    const char *or_ct = draw_extra(e, "or_card_types");
    const char *chars = draw_extra(e, "characters");
    const char *group = draw_extra(e, "group_names");
    const char *ctype = draw_extra(e, "card_type");

    if (!src && n_hc == 0 && !or_ct && !chars && !group) {
        rb_effect_area_select(g, actor, e, host_cid);
        return;
    }
    if (!src && !ctype && has_heart_colors) {
        int count = e->count >= 0 ? e->count : 1;
        rb_effect_select_heart_color(g, actor, count, heart_colors, n_hc, e->target);
        return;
    }
    if (draw_extra(e, "keep_shuffle_under")) {
        rb_effect_both_hand_keep_shuffle_under(g, actor, e, host_cid);
        return;
    }
    /* generic execute_select: choose `count` cards from the resolved source zone */
    const char *source = (!ctype || strcmp(ctype, "member_card") != 0)
                         ? (src ? src : "hand")
                         : (src ? src : "stage");
    int cnt = e->count >= 0 ? e->count : 1;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, source, ctype, cnt, e->is_optional ? 1 : 0, NULL);
    g->queue.pending.filter_group[0] = 0;
    if (group) strncpy(g->queue.pending.filter_group, group, sizeof(g->queue.pending.filter_group) - 1);
    g->queue.pending.filter_heart = -1;
    if (n_hc) g->queue.pending.filter_heart = (int)rb_parse_heart_color(heart_colors[0]);
    strncpy(g->queue.resume_filter_group, g->queue.pending.filter_group, sizeof(g->queue.resume_filter_group) - 1);
    g->queue.resume_filter_heart = g->queue.pending.filter_heart;
    if (n_hc) {
        g->queue.resume_mode = 0; g->queue.resume_is_select = 0;
    } else {
        g->queue.resume_mode = 2; g->queue.resume_is_select = 1;
    }
    g->queue.resume_eff = e; g->queue.resume_actor = actor; g->queue.resume_host = actor;
}

/* ── C6 keep-N-shuffle-rest (draw.rs::execute_both_hand_keep_shuffle_under) ── */
void rb_effect_both_hand_keep_shuffle_under(GameState *g, int actor, AbilityEffect *e, int host_cid) {
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
        rb_emit_choice(g, pl, RB_CHOICE_SELECT_CARD, "hand", NULL, pick > 0 ? pick : 1, 1, "keep_shuffle_under");
        g->queue.resume_mode = 5; g->queue.resume_eff = e;
        g->queue.resume_actor = actor; g->queue.resume_host = actor;
        g->keep_shuffle_under_phase = 1;
        return;
    }
    if (phase == 1) {
        RbPlayer *Ps = &g->p[actor];
        int *snap = g->keep_shuffle_under_snapshot[0]; int ns = g->keep_shuffle_under_snapshot_n[0];
        int kept[RB_MAX_HAND]; int nk = 0;
        for (int i = 0; i < g->n_selected_cards && nk < RB_MAX_HAND; i++) kept[nk++] = g->selected_cards[i];
        for (int i = 0; i < ns; i++) {
            int is_kept = 0;
            for (int k = 0; k < nk; k++) if (kept[k] == snap[i]) { is_kept = 1; break; }
            if (!is_kept) {
                for (int p = 0; p < Ps->hand.n; p++) if (Ps->hand.cards[p] == snap[i]) {
                    for (int q = p; q < Ps->hand.n - 1; q++) Ps->hand.cards[q] = Ps->hand.cards[q + 1];
                    Ps->hand.n--; break;
                }
            }
        }
        int to_move[RB_MAX_HAND]; int nm = 0;
        for (int i = 0; i < ns; i++) {
            int is_kept = 0;
            for (int k = 0; k < nk; k++) if (kept[k] == snap[i]) { is_kept = 1; break; }
            if (!is_kept && nm < RB_MAX_HAND) to_move[nm++] = snap[i];
        }
        rb_shuffle(to_move, nm);
        for (int i = 0; i < nm; i++) if (Ps->deck.n < RB_MAX_ZONE) Ps->deck.cards[Ps->deck.n++] = to_move[i];
        /* snapshot opponent and prompt */
        int opp = actor ^ 1; RbPlayer *Po = &g->p[opp];
        int ns2 = 0;
        for (int i = 0; i < Po->hand.n && ns2 < RB_MAX_HAND; i++)
            g->keep_shuffle_under_snapshot[1][ns2++] = Po->hand.cards[i];
        g->keep_shuffle_under_snapshot_n[1] = ns2;
        int pick2 = count < Po->hand.n ? count : Po->hand.n;
        rb_emit_choice(g, opp, RB_CHOICE_SELECT_CARD, "hand", NULL, pick2 > 0 ? pick2 : 1, 1, "keep_shuffle_under");
        g->queue.resume_mode = 5; g->queue.resume_eff = e;
        g->queue.resume_actor = actor; g->queue.resume_host = actor;
        g->keep_shuffle_under_phase = 2;
        g->n_selected_cards = 0; /* fresh selection for opponent */
        return;
    }
    /* phase == 2: opponent's selection resolved */
    {
        int opp = actor ^ 1; RbPlayer *Po = &g->p[opp];
        int *snap = g->keep_shuffle_under_snapshot[1]; int ns = g->keep_shuffle_under_snapshot_n[1];
        int kept[RB_MAX_HAND]; int nk = 0;
        for (int i = 0; i < g->n_selected_cards && nk < RB_MAX_HAND; i++) kept[nk++] = g->selected_cards[i];
        for (int i = 0; i < ns; i++) {
            int is_kept = 0;
            for (int k = 0; k < nk; k++) if (kept[k] == snap[i]) { is_kept = 1; break; }
            if (!is_kept) {
                for (int p = 0; p < Po->hand.n; p++) if (Po->hand.cards[p] == snap[i]) {
                    for (int q = p; q < Po->hand.n - 1; q++) Po->hand.cards[q] = Po->hand.cards[q + 1];
                    Po->hand.n--; break;
                }
            }
        }
        int to_move[RB_MAX_HAND]; int nm = 0;
        for (int i = 0; i < ns; i++) {
            int is_kept = 0;
            for (int k = 0; k < nk; k++) if (kept[k] == snap[i]) { is_kept = 1; break; }
            if (!is_kept && nm < RB_MAX_HAND) to_move[nm++] = snap[i];
        }
        rb_shuffle(to_move, nm);
        for (int i = 0; i < nm; i++) if (Po->deck.n < RB_MAX_ZONE) Po->deck.cards[Po->deck.n++] = to_move[i];
    }
    g->keep_shuffle_under_phase = 0;
    g->keep_shuffle_under_count = 0;
    g->keep_shuffle_under_snapshot_n[0] = 0;
    g->keep_shuffle_under_snapshot_n[1] = 0;
    g->keep_shuffle_under_selected_n = 0;
    g->n_selected_cards = 0;
}

/* ── Draw until hand reaches target count (draw.rs::execute_draw_until_count) ──
   If the target player's hand has fewer cards than target_count, draw the
   difference from the deck. Mirrors the Rust logic:
   to_draw = target_count.saturating_sub(current_hand_count) ── */
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
    int who = (e->target && (!strcmp(e->target, "opponent") || !strcmp(e->target, "p2"))) ? actor ^ 1 : actor;
    RbPlayer *P = &g->p[who];
    int current = P->hand.n;
    if (current >= target_count) return;
    int to_draw = target_count - current;
    rb_draw_cards_for_player(P, (uint8_t)to_draw, "deck", "hand", NULL, 0, NULL, NULL, -1);
}

/* ── Make single-card effect data (draw.rs::make_card_effect_data) ──
   Builds an EffectData::SingleCard for a blade/heart resource grant.
   The C engine stores this as a flat struct for downstream consumers. ── */
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
