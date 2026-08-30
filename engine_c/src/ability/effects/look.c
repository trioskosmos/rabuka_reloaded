#include "rabuka.h"
#include <string.h>

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
    for(int i=0;i<cnt && lp->n < MAX_LOOKED;i++){
        int cid=-1;
        if(from_deck && P->deck.n>0) cid=P->deck.cards[--P->deck.n];
        else if(P->hand.n>0) cid=P->hand.cards[--P->hand.n];
        else break;
        lp->cards[lp->n++]=cid;
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
    g->queue.resume_mode = 2; g->queue.resume_eff = e; g->queue.resume_is_select = 1;
    g->queue.resume_actor = actor; g->queue.resume_host = actor;
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
static int card_type_pred(const GameState *g, int cid){
    (void)g;
    return card_matches_card_type_filter(cid, g_reveal_ctype);
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
