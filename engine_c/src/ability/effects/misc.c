/* effects/misc.c — miscellaneous ability-effect handlers.
   Mirror engine/src/ability/effects/misc.rs (execute_gain_resource,
   play_baton_touch, place_energy_under_member, position_change, rotation,
   choice, pay_energy, discard_until_count, restriction, re_yell,
   perform_yell, shuffle, ...).

   STUBS: each handler mirrors its Rust counterpart's signature and returns
   the permissive default. The dispatch rb_execute_misc_effect routes by
   effect name so callers (engine.c) can delegate unknown effect types here
   without touching the main switch. Fill handlers in one by one. */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>

static int h_gain_resource(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs:execute_gain_resource — add `count` energy to the
       actor's energy zone AND activate it (capped at RB_ENERGY_CAP). The
       C model tracks energy as a count, so both the zone size and the
       active count advance together to keep zone/active consistent. */
    int n = e->count > 0 ? e->count : 1;
    RbPlayer *P = &g->p[actor];
    P->energy.n += n;
    if (P->energy.n > RB_ENERGY_CAP) P->energy.n = RB_ENERGY_CAP;
    P->energy_active += n;
    if (P->energy_active > RB_ENERGY_CAP) P->energy_active = RB_ENERGY_CAP;
    return 1;
}
static int h_pay_energy(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs pay_energy — spend `count` active energy. */
    int n = e->count > 0 ? e->count : 1;
    RbPlayer *P = &g->p[actor];
    P->energy_active -= n;
    if (P->energy_active < 0) P->energy_active = 0;
    return 1;
}
static int h_discard_until_count(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs discard_until_count — discard from hand until hand
       size reaches `count`. */
    int target = e->count > 0 ? e->count : 0;
    RbPlayer *P = &g->p[actor];
    while (P->hand.n > target && P->hand.n > 0) {
        int card = P->hand.cards[--P->hand.n]; /* drop from end */
        if (P->discard.n < RB_MAX_ZONE) P->discard.cards[P->discard.n++] = card;
    }
    return 1;
}
static const char *eff_extra(const AbilityEffect *e, const char *k){
    for(int i=0;i<e->n_extra;i++) if(e->extra_k[i] && !strcmp(e->extra_k[i],k)) return e->extra_v[i];
    return NULL;
}
static int h_restriction(GameState *g, int actor, const AbilityEffect *e) {
    const char *rtype = eff_extra(e, "restriction_type");
    const char *rdest = eff_extra(e, "restricted_destination");
    if(!rtype) rtype = eff_extra(e, "type");
    if(!rdest && e->destination) rdest = e->destination;
    int delayed = 0;
    const char *dstr = eff_extra(e, "delayed");
    if(dstr && (!strcmp(dstr,"true")||!strcmp(dstr,"1"))) delayed = 1;

    /* Record the prohibition note (mirrors gs.prohibition_effects). */
    if(g->n_prohibition < 64){
        char *b = g->prohibition[g->n_prohibition];
        int bi = 0;
        const char *a = rtype?rtype:"unknown";
        const char *d = rdest?rdest:"";
        for(const char *p=a; *p && bi<46; ) b[bi++]=*p++;
        if(bi<47) b[bi++]=':';
        for(const char *p=d; *p && bi<47; ) b[bi++]=*p++;
        b[bi]=0;
        g->n_prohibition++;
    }

    /* cannot_activate / cannot_active → block ability activation. */
    int is_cannot = rtype && (!strcmp(rtype,"cannot_activate_by_effect") ||
                             !strcmp(rtype,"cannot_active") || !strcmp(rtype,"cannot_activate"));
    if(is_cannot){
        int tgt = actor;
        if(e->target && !strcmp(e->target,"opponent")) tgt = actor^1;
        if(delayed){
            /* Key the ban on the cards this ability just moved, else the target
               player's staged members (next-turn-only activation lockout). */
            for(int i=0;i<g->n_recently_moved && g->n_cannot_active_cards<RB_MAX_ZONE;i++)
                g->cannot_active_cards[g->n_cannot_active_cards++]=g->recently_moved[i];
            for(int q=0;q<RB_STAGE_SIZE;q++)
                if(g->p[tgt].stage[q]>=0 && g->n_cannot_active_cards<RB_MAX_ZONE)
                    g->cannot_active_cards[g->n_cannot_active_cards++]=g->p[tgt].stage[q];
        } else {
            g->player_cannot_activate[tgt] = 1;
        }
    }
    return 1;
}
static int h_choice(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor; (void)e;
    return 1; /* TODO: open interactive choice */
}
static int h_position_change(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs position_change — move a member from a source area to a
       destination area on the actor's stage. Source defaults to center,
       destination from e->destination (else e->target). */
    RbPlayer *P = &g->p[actor];
    int src = 1, dst = 1;
    if (e->source && *e->source) src = rb_pos_to_area(e->source);
    const char *dest = e->destination && *e->destination ? e->destination : e->target;
    if (dest && *dest) dst = rb_pos_to_area(dest);
    if (src < 0 || src >= RB_STAGE_SIZE) src = 1;
    if (dst < 0 || dst >= RB_STAGE_SIZE) dst = 1;
    if (src == dst) return 1;
    if (P->stage[src] < 0) return 0;        /* nothing to move */
    if (P->stage[dst] >= 0) return 0;        /* destination occupied */
    int card = P->stage[src];
    P->stage[src] = -1; P->stage_wait[src] = 0;
    P->stage[dst] = card; P->stage_wait[dst] = 0;
    return 1;
}
static int h_rotation(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs rotation — flip the orientation (active<->wait) of the
       targeted member. Target area defaults to center. */
    RbPlayer *P = &g->p[actor];
    int area = 1; /* center */
    if (e->target && *e->target) area = rb_pos_to_area(e->target);
    if (area < 0 || area >= RB_STAGE_SIZE) area = 1;
    if (P->stage[area] >= 0) { P->stage_wait[area] = !P->stage_wait[area]; return 1; }
    return 0;
}
static int h_place_energy_under_member(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs place_energy_under_member — tuck `count` energy cards
       under a stage member (under_cards[area]). They leave the energy zone. */
    RbPlayer *P = &g->p[actor];
    int area = 1; /* center */
    const char *dest = e->destination && *e->destination ? e->destination : e->target;
    if (dest && *dest) area = rb_pos_to_area(dest);
    if (area < 0 || area >= RB_STAGE_SIZE) area = 1;
    if (P->stage[area] < 0) return 0; /* no member to tuck under */
    int n = e->count > 0 ? e->count : 1;
    int moved = 0;
    while (moved < n && P->energy.n > 0) {
        int cid = P->energy.cards[--P->energy.n];
        if (P->under_cards[area].n < RB_MAX_ZONE)
            P->under_cards[area].cards[P->under_cards[area].n++] = cid;
        moved++;
    }
    return 1;
}
static int h_play_baton_touch(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor; (void)e;
    /* Mirror misc.rs play_baton_touch — interactive baton-redirect gate.
       Headless auto-play has no opponent to redirect to, so this is a
       permissive no-op (the redirected play is simply allowed). */
    return 1;
}
static int h_re_yell(GameState *g, int actor, const AbilityEffect *e) {
    (void)actor; (void)e;
    /* Mirror misc.rs re_yell — re-run the live yell pool. Signals live.c's
       two-pass rebuild (g->re_yell_occurred) so hearts harvested by
       perform_yell are re-applied to the success check. */
    g->re_yell_occurred = 1;
    return 1;
}
static int h_perform_yell(GameState *g, int actor, const AbilityEffect *e) {
    (void)e;
    /* Mirror misc.rs perform_yell — finalize the current yell, harvesting the
       yelled member's blade into the live pool. The yelled cards are the
       actor's currently-staged live cards; sum their effective blade into the
       re_yell harvest that live.c's two-pass rebuild re-applies. */
    RbPlayer *P = &g->p[actor];
    for (int i = 0; i < P->live.n; i++) {
        Card c; if (rb_decode_card_by_index((uint32_t)P->live.cards[i], &c)) {
            int blade = (int)c.blade + rb_mods_get_blade(&g->mods, P->live.cards[i]);
            if (blade > 0) g->re_yell_blade_hearts[RB_HEART_PINK] += blade;
            rb_free_card(&c);
        }
    }
    g->re_yell_occurred = 1;
    return 1;
}
static int h_shuffle(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs shuffle — Fisher-Yates shuffle of the named zone
       (default: deck). */
    (void)actor;
    const char *zone = e->target && *e->target ? e->target : "deck";
    RbBag *b = NULL;
    if (!strcmp(zone, "hand")) b = &g->p[actor].hand;
    else if (!strcmp(zone, "deck")) b = &g->p[actor].deck;
    else if (!strcmp(zone, "energy")) b = &g->p[actor].energy;
    else if (!strcmp(zone, "discard")) b = &g->p[actor].discard;
    if (!b || b->n < 2) return 1;
    for (int i = b->n - 1; i > 0; i--) {
        int j = rand() % (i + 1);
        int t = b->cards[i]; b->cards[i] = b->cards[j]; b->cards[j] = t;
    }
    return 1;
}

/* Dispatch an "misc" effect by its name string. Returns 1 on handled. */
int rb_execute_misc_effect(GameState *g, int actor, const RbPlayer *self,
                           const AbilityEffect *e, int *resolved) {
    if (!e) return 0;
    const char *name = e->action;
    int r = 1;
    if (name) {
        if      (!strcmp(name, "gain_resource"))          r = h_gain_resource(g, actor, e);
        else if (!strcmp(name, "pay_energy"))             r = h_pay_energy(g, actor, e);
        else if (!strcmp(name, "discard_until_count"))     r = h_discard_until_count(g, actor, e);
        else if (!strcmp(name, "restriction"))            r = h_restriction(g, actor, e);
        else if (!strcmp(name, "choice"))                 r = h_choice(g, actor, e);
        else if (!strcmp(name, "position_change"))        r = h_position_change(g, actor, e);
        else if (!strcmp(name, "rotation"))               r = h_rotation(g, actor, e);
        else if (!strcmp(name, "place_energy_under_member")) r = h_place_energy_under_member(g, actor, e);
        else if (!strcmp(name, "play_baton_touch"))       r = h_play_baton_touch(g, actor, e);
        else if (!strcmp(name, "re_yell"))                r = h_re_yell(g, actor, e);
        else if (!strcmp(name, "perform_yell"))           r = h_perform_yell(g, actor, e);
        else if (!strcmp(name, "shuffle"))                r = h_shuffle(g, actor, e);
        else r = 0; /* unknown misc effect */
    }
    (void)self;
    if (resolved) *resolved = r;
    return r;
}
