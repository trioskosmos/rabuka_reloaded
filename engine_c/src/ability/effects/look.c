#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* Look/select pools — mirrors engine/src/ability/look.rs
   For portable core we keep two pools per player:
   looked_at (cards revealed by look_at) and selected (choice result).
   The host's SELECT_CARD choice picks from looked_at; looked_at_remaining
   goes to destination (usually discard or deck). Full keep_shuffle_under
   2-phase lands with the 20-fixture harness. */

#define MAX_LOOKED 64
typedef struct {
    int cards[MAX_LOOKED];
    int n;
    int from_deck; /* 1 if pool came from deck top */
    int owner;     /* player whose cards are in the pool */
} LookPool;

static LookPool g_look[2]; /* per-player */

void rb_look_clear(int pl){ g_look[pl].n=0; g_look[pl].from_deck=0; g_look[pl].owner=pl; }

/* Expose the looked_at pool for relay references (move_cards source
   "looked_at" / "looked_at_remaining", mirroring engine/src/ability/look.rs
   looked_at relay pool). Returns the count, fills out_ids (cap max). */
int rb_looked_at_pool(int pl, int *out_ids, int max){
    LookPool *lp=&g_look[pl];
    int n = lp->n < max ? lp->n : max;
    for(int i=0;i<n;i++) out_ids[i]=lp->cards[i];
    return n;
}

void rb_effect_look_at(GameState *g, int actor, AbilityEffect *e){
    int cnt = e->count>=0? e->count:1;
    int who = actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    RbPlayer *P=&g->p[who];
    LookPool *lp=&g_look[who];
    lp->n=0; lp->from_deck=0; lp->owner=who;
    const char *src = e->source ? e->source : "deck";
    int from_deck = !strcmp(src,"deck")||!strcmp(src,"deck_top");
    lp->from_deck = from_deck;
    /* reveal_per_group (look.rs::execute_reveal_per_group): when a group filter is
        present, reveal from the deck top until a card of that group is found,
        populating the looked_at pool — instead of revealing a fixed count. */
    const char *group=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"group_names")) group=e->extra_v[i];
    if(group){
        /* reveal_per_group (look.rs::execute_reveal_per_group): reveal from the
            source (deck top OR hand top) until a card of that group is found,
            populating the looked_at pool — instead of revealing a fixed count.
            Mirrors Rust for both deck and hand sources. */
        while(lp->n<MAX_LOOKED){
            int cid=-1;
            if(from_deck && P->deck.n>0) cid=P->deck.cards[--P->deck.n];
            else if(!from_deck && P->hand.n>0) cid=P->hand.cards[--P->hand.n];
            else break;
            if(cid<0) break;
            lp->cards[lp->n++]=cid;
            if(rb_card_matches_group_str(cid, group)) break;
        }
    } else {
        for(int i=0;i<cnt && lp->n < MAX_LOOKED;i++){
            int cid=-1;
            if(from_deck && P->deck.n>0) cid=P->deck.cards[--P->deck.n];
            else if(P->hand.n>0) cid=P->hand.cards[--P->hand.n];
            else break;
            lp->cards[lp->n++]=cid;
        }
    }
    /* Mirror Rust reveal → gs.revealed_cards: every looked/revealed card also lands
        in the shared revealed_cards pool consumed by the all_revealed_match_heart_color
        condition (ability/condition.c eval_all_revealed). */
    for(int i=0;i<lp->n;i++){
        if(g->n_revealed < RB_MAX_RECENTLY_MOVED)
            g->revealed_cards[g->n_revealed++]=lp->cards[i];
    }
    /* Surface as SELECT_CARD choice on looked_at zone */
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "looked_at", NULL, 1, e->is_optional?1:0, NULL);
    g->queue.resume_mode = 2; g->queue.resume_eff = e; g->queue.resume_is_select = 0;
    g->queue.resume_actor = actor; g->queue.resume_host = actor;
}

void rb_effect_select_cards(GameState *g, int actor, AbilityEffect *e){
    /* If we have a looked_at pool, the choice is from it; otherwise from hand */
    int who = actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    LookPool *lp=&g_look[who];
    const char *zone = lp->n>0 ? "looked_at" : (e->source ? e->source : "hand");
    const char *ctype=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"card_type")) ctype=e->extra_v[i];
    int cnt=e->count>=0?e->count:1;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, zone, ctype, cnt, e->is_optional?1:0, NULL);
    /* SelectionContext filter (ability/choice.rs): narrow the valid pool to a
        group and/or heart color so a host UI / test picks a legal card. */
    g->queue.pending.filter_group[0] = 0;
    g->queue.pending.filter_heart = -1;
    int has_heart_color = 0;
    for(int i=0;i<e->n_extra;i++){
        if(e->extra_k[i] && !strcmp(e->extra_k[i],"group_names") && e->extra_v[i])
            strncpy(g->queue.pending.filter_group, e->extra_v[i], sizeof(g->queue.pending.filter_group)-1);
        else if(e->extra_k[i] && !strcmp(e->extra_k[i],"heart_colors") && e->extra_v[i]){
            const char *hc=e->extra_v[i];
            int col=-1;
            if(!strcmp(hc,"pink")||!strcmp(hc,"heart00")) col=0;
            else if(!strcmp(hc,"red")) col=1; else if(!strcmp(hc,"yellow")) col=2;
            else if(!strcmp(hc,"green")) col=3; else if(!strcmp(hc,"blue")) col=4;
            else if(!strcmp(hc,"purple")) col=5; else if(!strcmp(hc,"orange")) col=6;
            else if(!strcmp(hc,"all")) col=7;
            g->queue.pending.filter_heart = col;
        }
        if (e->extra_k[i] && (!strcmp(e->extra_k[i], "heart_color") ||
                              !strcmp(e->extra_k[i], "heart_colors")))
            has_heart_color = 1;
    }
    /* snapshot the filter so rb_look_resume can validate after the pending choice is cleared */
    strncpy(g->queue.resume_filter_group, g->queue.pending.filter_group, sizeof(g->queue.resume_filter_group)-1);
    g->queue.resume_filter_heart = g->queue.pending.filter_heart;
    /* Heart-color selection (mirrors Rust execute_choice → conditional_choice =
        Str(color)). A select with a heart_color extra is a "pick a heart color"
        prompt; stash the chosen color so the following gain_resource applies it.
        Route it through the default resume branch (mode 0) so the parent's later
        siblings (the gain) run after the choice resolves — NOT the card-select
        look/keep path (mode 2). */
    if (has_heart_color) {
        g->queue.selected_heart_color = -1;
        for (int i = 0; i < e->n_extra; i++) {
            if (e->extra_k[i] && (!strcmp(e->extra_k[i], "heart_color") ||
                                  !strcmp(e->extra_k[i], "heart_colors")) && e->extra_v[i]) {
                int col = -1;
                if (!strcmp(e->extra_v[i], "pink") || !strcmp(e->extra_v[i], "heart00")) col = 0;
                else if (!strcmp(e->extra_v[i], "red") || !strcmp(e->extra_v[i], "heart01")) col = 1;
                else if (!strcmp(e->extra_v[i], "yellow") || !strcmp(e->extra_v[i], "heart02")) col = 2;
                else if (!strcmp(e->extra_v[i], "green") || !strcmp(e->extra_v[i], "heart03")) col = 3;
                else if (!strcmp(e->extra_v[i], "blue") || !strcmp(e->extra_v[i], "heart04")) col = 4;
                else if (!strcmp(e->extra_v[i], "purple") || !strcmp(e->extra_v[i], "heart05")) col = 5;
                else if (!strcmp(e->extra_v[i], "orange") || !strcmp(e->extra_v[i], "heart06")) col = 6;
                else if (!strcmp(e->extra_v[i], "all") || !strcmp(e->extra_v[i], "heart07") || !strcmp(e->extra_v[i], "b_all")) col = 7;
                if (col >= 0) g->queue.selected_heart_color = col;
            }
        }
        g->queue.resume_mode = 0; g->queue.resume_is_select = 0;
        g->queue.resume_eff = e; g->queue.resume_actor = actor; g->queue.resume_host = actor;
    } else {
        g->queue.resume_mode = 2; g->queue.resume_eff = e; g->queue.resume_is_select = 1;
        g->queue.resume_actor = actor; g->queue.resume_host = actor;
    }
}

/* Called when host resumes SELECT_CARD — move chosen card to destination.
   Mirrors engine/src/ability/look.rs keep_shuffle_under: cards that came
   from the deck and were NOT kept are shuffled back into the owner's deck
   (under), not discarded; hand-sourced cards return to hand; the rest go to
   the destination / discard. */
void rb_look_resume(GameState *g, int actor, int selected_idx, const char *destination, int is_select){
    LookPool *lp=&g_look[actor];
    int who = lp->owner;
    RbPlayer *P=&g->p[who];
    if(selected_idx<0 || selected_idx>=lp->n){
        /* skip: every looked card returns to its origin (deck→shuffle under,
           hand→hand) */
        if(lp->from_deck){
            for(int i=0;i<lp->n;i++) if(P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++]=lp->cards[i];
            rb_shuffle(P->deck.cards, P->deck.n);
        } else {
            for(int i=0;i<lp->n;i++) if(P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++]=lp->cards[i];
        }
        lp->n=0; return;
    }
    int chosen=lp->cards[selected_idx];
    /* SelectionContext filter (ability/choice.rs): if a group/heart filter was
        specified, the kept card must satisfy it; otherwise treat as a skip so a
        mismatched pick never silently enters selected_cards. */
    if(is_select && (g->queue.resume_filter_group[0] || g->queue.resume_filter_heart >= 0)) {
        int ok = 1;
        if (g->queue.resume_filter_group[0] &&
            !rb_card_matches_group_str(chosen, g->queue.resume_filter_group)) ok = 0;
        if (ok && g->queue.resume_filter_heart >= 0) {
            Card fc; if (rb_decode_card_by_index((uint32_t)chosen, &fc)) {
                int has = 0;
                for (int h = 0; h < fc.n_hearts; h++)
                    if (fc.heart_color[h] == g->queue.resume_filter_heart) has = 1;
                rb_free_card(&fc);
                if (!has) ok = 0;
            }
        }
        if (!ok) {
            if (lp->from_deck) {
                for (int i = 0; i < lp->n; i++) if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = lp->cards[i];
                rb_shuffle(P->deck.cards, P->deck.n);
            } else {
                for (int i = 0; i < lp->n; i++) if (P->hand.n < RB_MAX_ZONE) P->hand.cards[P->hand.n++] = lp->cards[i];
            }
            lp->n = 0; return;
        }
    }
    if(is_select){
        if(g->n_selected_cards < RB_MAX_RECENTLY_MOVED) g->selected_cards[g->n_selected_cards++]=chosen;
    }
    RbZone dst=RB_ZONE_HAND;
    if(destination) rb_zone_of_str(destination,&dst);
    RbBag *db=NULL;
    if(dst==RB_ZONE_HAND) db=&P->hand;
    else if(dst==RB_ZONE_DISCARD) db=&P->discard;
    else if(dst==RB_ZONE_DECK) db=&P->deck;
    else if(dst==RB_ZONE_STAGE) db=NULL; /* stage placement handled via play */
    if(db && db->n < RB_MAX_ZONE) db->cards[db->n++]=chosen;
    /* remaining: deck-sourced shuffle back under; otherwise to discard */
    if(lp->from_deck){
        for(int i=0;i<lp->n;i++) if(i!=selected_idx){
            if(P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++]=lp->cards[i];
        }
        rb_shuffle(P->deck.cards, P->deck.n);
    } else {
        for(int i=0;i<lp->n;i++) if(i!=selected_idx){
            if(P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++]=lp->cards[i];
        }
    }
    lp->n=0;
}

/* Reveal cards from the target's deck top into the looked_at pool until
   `pred` returns true (or the deck empties). Mirrors engine/src/ability/look.rs
   reveal_until — the matched card and everything revealed above it become the
   looked_at pool (gs.looked_at_cards = all_revealed). */
static int reveal_until(GameState *g, int who, int (*pred)(const GameState*, int cid)){
    RbPlayer *P=&g->p[who];
    LookPool *lp=&g_look[who];
    lp->n=0; lp->from_deck=1; lp->owner=who;
    while(P->deck.n>0 && lp->n<MAX_LOOKED){
        int cid=P->deck.cards[--P->deck.n];
        lp->cards[lp->n++]=cid;
        if(g->n_revealed < RB_MAX_RECENTLY_MOVED)
            g->revealed_cards[g->n_revealed++]=cid;
        if(pred(g,cid)) return 1; /* matched */
    }
    return 0; /* deck exhausted without a match */
}

static int card_is_live_pred(const GameState *g, int cid){
    Card c; if(!rb_decode_card_by_index((uint32_t)cid,&c)) return 0;
    int r = (c.type_flags & 0x3) == 1; rb_free_card(&c); return r;
}

/* Adapter so card_matches_card_type_filter (int,int) can be used as a
   reveal_until predicate (const GameState*,int). */
static const char *g_reveal_ctype;
static int g_reveal_cost_limit = -1;
static const char *g_reveal_cost_op;
static int card_type_pred(const GameState *g, int cid){
    (void)g;
    return card_matches_card_type_filter(cid, g_reveal_ctype);
}

/* Mirror look.rs::execute_reveal_until_target — reveal from the deck until a card
    matching card_type (and, for member_card, cost_limit op) is found. The predicate
    layers the optional cost gate on top of the card_type gate. */
static int card_type_cost_pred(const GameState *g, int cid){
    (void)g;
    if (!g_reveal_ctype) return 0;
    if (!card_matches_card_type_filter(cid, g_reveal_ctype)) return 0;
    /* cost_limit only applies when the selected card_type is member_card. */
    if (g_reveal_cost_limit >= 0 && !strcmp(g_reveal_ctype, "member_card"))
        if (!rb_card_matches_cost_limit(cid, g_reveal_cost_limit, g_reveal_cost_op)) return 0;
    return 1;
}

void rb_effect_reveal_until_live_card(GameState *g, int actor, AbilityEffect *e){
    int who = actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    reveal_until(g, who, card_is_live_pred);
    /* surface the looked_at pool as a SELECT_CARD choice so host/tests can read it */
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "looked_at", NULL, 1, e->is_optional?1:0, NULL);
    g->queue.resume_mode = 2; g->queue.resume_eff = e; g->queue.resume_is_select = 0;
    g->queue.resume_actor = actor; g->queue.resume_host = actor;
}

void rb_effect_reveal_until_chosen_card(GameState *g, int actor, AbilityEffect *e){
    int who = actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    const char *ctype=NULL;
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],"card_type")) ctype=e->extra_v[i];
    g_reveal_ctype = ctype ? ctype : "live_card";
    reveal_until(g, who, ctype ? card_type_pred : card_is_live_pred);
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "looked_at", ctype, 1, e->is_optional?1:0, NULL);
    g->queue.resume_mode = 2; g->queue.resume_eff = e; g->queue.resume_is_select = 0;
    g->queue.resume_actor = actor; g->queue.resume_host = actor;
}

/* Mirror look.rs::execute_reveal_until_target — reveal from the deck until a card
    matching card_type (and the member_card cost_limit gate) is found. On a match
    the matched card is moved to the FRONT of the looked_at pool; on no match the
    pool is cleared, mirroring Rust's matched_idx reordering. */
void rb_effect_reveal_until_target(GameState *g, int actor, AbilityEffect *e){
    int who = actor;
    if(e->target && !strcmp(e->target,"opponent")) who=actor^1;
    const char *ctype=NULL, *clim=NULL, *cop=NULL;
    for(int i=0;i<e->n_extra;i++){
        if(!e->extra_k[i]) continue;
        if(!strcmp(e->extra_k[i],"card_type")) ctype=e->extra_v[i];
        else if(!strcmp(e->extra_k[i],"cost_limit")) clim=e->extra_v[i];
        else if(!strcmp(e->extra_k[i],"cost_limit_operator")) cop=e->extra_v[i];
    }
    g_reveal_ctype = ctype ? ctype : "live_card";
    g_reveal_cost_limit = clim ? atoi(clim) : -1;
    g_reveal_cost_op = cop ? cop : NULL;
    LookPool *lp=&g_look[who];
    int matched = reveal_until(g, who, card_type_cost_pred);
    if(matched && lp->n>1){
        /* reveal_until pushes the matched card last; move it to the pool front. */
        int m = lp->cards[lp->n-1];
        for(int i=lp->n-1;i>0;i--) lp->cards[i]=lp->cards[i-1];
        lp->cards[0]=m;
    } else if(!matched){
        lp->n=0; /* no match → clear pool (mirrors Rust looked_at_cards.clear()) */
    }
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "looked_at", ctype, 1, e->is_optional?1:0, NULL);
    g->queue.resume_mode = 2; g->queue.resume_eff = e; g->queue.resume_is_select = 0;
    g->queue.resume_actor = actor; g->queue.resume_host = actor;
}
