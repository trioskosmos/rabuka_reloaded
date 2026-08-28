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

static int h_gain_resource(GameState *g, int actor, const AbilityEffect *e) {
    /* Mirror misc.rs:execute_gain_resource — add `count` energy to the
       actor's active energy pool (capped at RB_ENERGY_CAP). */
    int n = e->count > 0 ? e->count : 1;
    RbPlayer *P = &g->p[actor];
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
static int h_restriction(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor; (void)e;
    return 1; /* TODO: set play restriction flag */
}
static int h_choice(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor; (void)e;
    return 1; /* TODO: open interactive choice */
}
static int h_position_change(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor; (void)e;
    return 1; /* TODO: move a member to a new position */
}
static int h_rotation(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor; (void)e;
    return 1; /* TODO: rotate orientation */
}
static int h_place_energy_under_member(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor; (void)e;
    return 1; /* TODO: tuck energy under a member */
}
static int h_play_baton_touch(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor; (void)e;
    return 1; /* TODO: baton touch redirect */
}
static int h_re_yell(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor; (void)e;
    return 1; /* TODO: re-yell trigger */
}
static int h_perform_yell(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor; (void)e;
    return 1; /* TODO: perform yell */
}
static int h_shuffle(GameState *g, int actor, const AbilityEffect *e) {
    (void)g; (void)actor; (void)e;
    return 1; /* TODO: shuffle zone */
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
