/* resolver.c — ability activation / choice resolution frontend.
   Mirror engine/src/ability/resolver.rs (resolver_get_pending_choice,
   can_activate_effect, resolve_ability, get_trigger_ability_infos,
   card_matches_type).

   STUBS: the authoritative activation/payment logic currently lives in
   engine.c. These wrappers mirror the Rust resolver names and will be
   filled in as the activation pipeline is ported. */

#include "rabuka.h"
#include <string.h>

/* Mirror resolver.rs:resolver_get_pending_choice — return the index of the
   effect currently awaiting an interactive choice, or -1. */
int rb_resolver_pending_choice(const GameState *g) {
    (void)g;
    return -1;
}

/* Mirror resolver.rs:can_activate_effect — is this effect activatable now?
   Default: yes (no timing/energy gating yet). */
int rb_can_activate_effect(const GameState *g, int actor, const AbilityEffect *eff) {
    (void)g; (void)actor; (void)eff;
    return 1;
}

/* Mirror resolver.rs:get_trigger_ability_infos — collect abilities whose
   trigger matches `trigger`. Fills out (cap max), returns count. */
int rb_resolver_trigger_infos(const GameState *g, int actor, const char *trigger,
                              AbilityInfo *out, int max) {
    (void)g; (void)actor; (void)trigger; (void)out; (void)max;
    return 0;
}

/* Mirror resolver.rs:resolve_ability — run a single ability's effects. */
int rb_resolve_ability(GameState *g, int actor, const AbilityEffect *eff, int *resolved) {
    return rb_compound_sequential(g, actor, &g->p[actor], eff, 1, resolved);
}

/* Mirror resolver.rs:card_matches_type — selector card-type filter.
   Delegates to util.c. */
int rb_resolver_card_matches_type(int cid, const char *filter) {
    return rb_card_matches_type(cid, filter);
}
