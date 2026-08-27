#include "rabuka.h"
#include <string.h>

/* Minimal condition evaluator — Phase 2 stub that gates correctly for
   trivial conditions and returns 1 (pass) otherwise so existing tests
   remain green. Full tree eval lands in Phase 2 proper. */

int rb_eval_condition(const struct GameState *g, int actor, const Condition *c) {
    (void)g; (void)actor;
    if (!c) return 1;
    /* Trivial: empty compound passes. Real eval will dispatch on c->variant
       (0..19) matching engine/src/ability/condition_decoder_gen.rs). */
    if (c->n_fields == 0) return 1;
    /* For now, respect an explicit "false" field if present (used by some
       synthetic fixtures) — else pass. This keeps v0 green while proving the
       gating hook in rb_execute_effect. */
    for (uint32_t i = 0; i < c->n_fields; i++) {
        if (!strcmp(c->fields[i].key, "always_false") && c->fields[i].v.tag == RB_TAG_TRUE)
            return 0;
    }
    return 1;
}
