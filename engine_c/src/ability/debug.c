#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdatomic.h>

static atomic_bool g_ability_debug = ATOMIC_VAR_INIT(false);

int rb_ability_debug_enabled(void){ return atomic_load(&g_ability_debug); }
void rb_ability_debug_set(int enabled){ atomic_store(&g_ability_debug, enabled); }

typedef struct {
    int indent;
} RbAbDebug;

void rb_abdebug_init(RbAbDebug *d){ if(d) d->indent=0; }
void rb_abdebug_p(RbAbDebug *d, const char *tag, const char *msg){
    if(!rb_ability_debug_enabled()) return;
    if(!d||!tag||!msg) return;
    char pad[64]={0};
    for(int i=0;i<d->indent && i<32;i++) pad[i*2]=' ', pad[i*2+1]=' ';
    fprintf(stderr,"[AB]%s%s %s\n", pad, tag, msg);
}

/* ── Ported from engine/src/ability/debug.rs ───────────────────────────────────
    AbDebug methods: flush_to_rule_log, flush_to_structured_log, ability,
    condition, cost_pay, effect. The C port uses a simplified debug model
    where these are no-ops (logging infrastructure not available). */

/* Mirror AbDebug::flush_to_rule_log — flush debug buffer to rule log. No-op in C. */
void rb_abdebug_flush_to_rule_log(RbAbDebug *d) {
    (void)d;
}

/* Mirror AbDebug::flush_to_structured_log — flush debug buffer to structured log. No-op in C. */
void rb_abdebug_flush_to_structured_log(RbAbDebug *d) {
    (void)d;
}

/* Mirror AbDebug::ability — log ability info. No-op in C. */
void rb_abdebug_ability(RbAbDebug *d, const char *card_name, const char *card_no,
                        const char *card_id, const Ability *ability) {
    (void)d; (void)card_name; (void)card_no; (void)card_id; (void)ability;
}

/* Mirror AbDebug::condition — log condition evaluation. No-op in C. */
void rb_abdebug_condition(RbAbDebug *d, const Condition *cond, int actual,
                          int threshold, int passed) {
    (void)d; (void)cond; (void)actual; (void)threshold; (void)passed;
}

/* Mirror AbDebug::cost_pay — log cost payment. No-op in C. */
void rb_abdebug_cost_pay(RbAbDebug *d, const AbilityEffect *cost, int ok) {
    (void)d; (void)cost; (void)ok;
}

/* Mirror AbDebug::effect — log effect execution. No-op in C. */
void rb_abdebug_effect(RbAbDebug *d, const AbilityEffect *effect) {
    (void)d; (void)effect;
}
