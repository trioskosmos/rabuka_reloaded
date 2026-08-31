/* engine_c/src/ability/choice_frag_08.c
 *
 * Port of engine/src/ability/choice.rs (lines ~2363-2583):
 *   - handle_selection_epilogue  -> rb_resolver_handle_selection_epilogue
 *   - handle_select_target       -> rb_resolver_handle_select_target
 *
 * The Rust `AbilityResolver` (self) is modelled as `RbResolver *rs`; its
 * `gs` field maps directly to the `GameState *` argument. Helper methods that
 * are part of the same resolver port are called by `rb_resolver_*` name
 * (forward-declared below and defined in sibling translation units). Three
 * helpers are mandated by the port spec and must NOT be defined here:
 *   rb_resolver_clear_choice_state_and_resume
 *   rb_set_chosen_target
 *   rb_resolver_handle_heart_selection
 *
 * C11, no dependencies beyond rabuka.h.
 */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdarg.h>

/* ── Resolver-local state (mirrors AbilityResolver fields used by these two
 *    methods: pending_choice, spawn_context.target, current_effect,
 *    moved_cards, pending_reprompt_choice). ── */
typedef struct RbResolver {
    int           has_pending_choice;            /* self.pending_choice.is_some() */
    char          spawn_target[32];              /* self.spawn_context.target */
    AbilityEffect *current_effect;               /* self.current_effect */
    int           moved_cards[RB_MAX_RECENTLY_MOVED]; /* self.moved_cards */
    int           n_moved_cards;
    int           pending_reprompt;              /* self.pending_reprompt_choice.is_some() */
    char          reprompt_target[32];
    char          reprompt_desc[256];
    char          reprompt_desc_en[256];
    char          reprompt_desc_ja[256];
    int           reprompt_allow_skip;
} RbResolver;

/* ExecutionContext passed to finalize_choice (opaque at this layer). */
typedef struct RbExecutionContext RbExecutionContext;

/* SelectTargetKind — mirrors engine/src/ability/enums::SelectTargetKind. */
typedef enum {
    RB_STK_NONE = 0,
    RB_STK_CHOICE,
    RB_STK_CHOICE_STRING,
    RB_STK_PAY_OPTIONAL_COST_SKIP_OPTIONAL_COST,
    RB_STK_PAY_COST_ALL_DISCARD,
    RB_STK_DOUBLE_BATON_TOUCH,
    RB_STK_PRIMARY_ALTERNATIVE,
    RB_STK_APPLY_REPLACEMENT,
    RB_STK_CHOOSE_REQUIRED_HEARTS,
    RB_STK_POSITION_DESTINATION,
    RB_STK_HEART_COLOR,
    RB_STK_CHOICE_TYPE,
    RB_STK_CHOICE_CONDITION,
    RB_STK_CONDITIONAL_OPTIONAL,
    RB_STK_DRAW_ANY_NUMBER,
    RB_STK_ORDER,
    RB_STK_SELF_OR_OPPONENT
} RbSelectTargetKind;

/* ConditionalChoice payload tag — mirrors engine/src/ability/types::ConditionalChoice. */
typedef enum {
    RB_CC_NONE = 0,
    RB_CC_EFFECTS
} RbConditionalChoiceTag;

/* ── Forward prototypes (in-fragment forward use) ── */

/* Mandated external helpers (defined elsewhere — do NOT define here). */
int  rb_resolver_clear_choice_state_and_resume(GameState *g, RbResolver *rs);
void rb_set_chosen_target(AbilityEffect *eff, const char *chosen);
int  rb_resolver_handle_heart_selection(RbResolver *rs, GameState *g, const char *selected);

/* Other self.* resolver methods (defined in sibling port units). */
int  rb_resolver_handle_choice_string_selection(RbResolver *rs, GameState *g,
                                                const char *selected, RbConditionalChoiceTag cond_tag);
int  rb_resolver_handle_position_change_choice(RbResolver *rs, GameState *g,
                                               const char *route, const char *selected);
int  rb_resolver_handle_choice_string_store(RbResolver *rs, GameState *g,
                                            const char *selected, RbConditionalChoiceTag cond_tag);
int  rb_resolver_handle_optional_cost_payment(RbResolver *rs, GameState *g, const char *selected);
int  rb_resolver_handle_pay_cost_all_discard(RbResolver *rs, GameState *g, const char *selected);
int  rb_resolver_handle_double_baton_touch(RbResolver *rs, GameState *g, const char *selected);
int  rb_resolver_handle_primary_alternative(RbResolver *rs, GameState *g, const char *selected);
int  rb_resolver_handle_position_destination(RbResolver *rs, GameState *g, const char *selected);
int  rb_resolver_handle_choice_condition(RbResolver *rs, GameState *g, const char *selected);
int  rb_resolver_handle_conditional_optional(RbResolver *rs, GameState *g, const char *selected);
int  rb_resolver_handle_draw_any_number(RbResolver *rs, GameState *g, const char *selected);
int  rb_resolver_handle_order_selection(RbResolver *rs, GameState *g, const char *selected);
void rb_resolver_clear_choice_state(GameState *g, RbResolver *rs);
int  rb_resolver_resume_pending_actions(GameState *g, RbResolver *rs);
int  rb_resolver_finalize_choice(GameState *g, RbResolver *rs, const RbExecutionContext *ctx);

/* GameState entry accessors (defined elsewhere). */
const char *rb_entry_choice_card_no(const GameState *g);   /* route name: "Choice"/"ChoiceString"/"Raw:..."/"", choice.rs entry_choice_card_no */
RbConditionalChoiceTag rb_entry_conditional_choice(const GameState *g,
                                                   const AbilityEffect *const **out_effects, int *out_n);
int  rb_queue_has_pending_actions(const GameState *g);      /* choice.rs gs.ability_queue.has_pending_actions */
int  rb_entry_effect_any_number_any(const GameState *g);    /* entry_effect().any_number_any() */
int  rb_entry_effect_has_alt_cond(const GameState *g);      /* entry_effect().compound.alternative_condition.is_some() */
const Condition *rb_entry_effect_alt_cond(const GameState *g);
const char *rb_entry_effect_alt_count_type(const GameState *g);
int  rb_eval_condition_with_moved(GameState *g, RbResolver *rs, const Condition *c);
void rb_queue_current_entry_set_conditional_choice_effects(GameState *g,
                                                           const AbilityEffect *const *effects, int n);
int  rb_resolver_set_pending_actions(GameState *g, const AbilityEffect *const *cmds, int n);
void rb_push_prohibition(GameState *g, const char *s);      /* gs.prohibition_effects.push */
int  rb_select_target_kind_from_str(const char *target, RbSelectTargetKind *out);
AbilityEffect *rb_effect_steps_first(const AbilityEffect *e);   /* current.effect_steps.first() */
AbilityEffect *rb_effect_clone(const AbilityEffect *e);
void rb_effect_free(AbilityEffect *e);
void rb_effect_answers_str(const AbilityEffect *e, char *out, size_t n); /* o.answers_any().join(", ") */
const char *rb_effect_text(const AbilityEffect *e);
int  rb_effect_has_look_action(const AbilityEffect *e);
int  rb_effect_has_select_action(const AbilityEffect *e);

/* local debug logging (mirrors log::debug! in the Rust source) */
static void rb_dbgf(const char *fmt, ...) {
    (void)fmt;
    /* fragment: logging intentionally a no-op; kept for parity with Rust log::debug!. */
}

/* ─────────────────────────────────────────────────────────────────────────
 * handle_selection_epilogue (choice.rs:2363)
 * ───────────────────────────────────────────────────────────────────────── */
int rb_resolver_handle_selection_epilogue(RbResolver *rs, GameState *g,
                                          const RbExecutionContext *ctx) {
    /* choice.rs:2368 — if queue still has pending actions and we are not
       sitting on a stale pending_choice, clear state and resume. */
    if (rb_queue_has_pending_actions(g) && !rs->has_pending_choice) {
        rb_resolver_clear_choice_state(g, rs);
        return rb_resolver_resume_pending_actions(g, rs);
    }
    /* choice.rs:2372 — otherwise finalize the choice. */
    return rb_resolver_finalize_choice(g, rs, ctx);
}

/* ─────────────────────────────────────────────────────────────────────────
 * handle_select_target (choice.rs:2375)
 * ───────────────────────────────────────────────────────────────────────── */
int rb_resolver_handle_select_target(RbResolver *rs, GameState *g,
                                     const char *target, const char *selected) {
    const char *choice_card_no = rb_entry_choice_card_no(g);  /* gs.entry_choice_card_no() */
    RbConditionalChoiceTag cond_tag =
        rb_entry_conditional_choice(g, NULL, NULL);          /* gs.entry_conditional_choice() */

    rb_dbgf("[HST] target=%s selected=%s choice_card_no=%s",
            target, selected, choice_card_no);

    /* choice.rs:2392 — choice_card_no-based routing */
    if (strcmp(choice_card_no, "Choice") == 0) {
        const AbilityEffect *const *all_options = NULL;
        int n_options = 0;
        if (cond_tag == RB_CC_EFFECTS) {
            rb_entry_conditional_choice(g, &all_options, &n_options);
        }
        if (cond_tag == RB_CC_EFFECTS && all_options && n_options > 0) {
            /* choice.rs:2395 — parse numeric option index. */
            char *end = NULL;
            long idx = strtol(selected, &end, 10);
            if (end == selected || *end != '\0' || idx < 0) {
                rb_dbgf("[HST] non-numeric option index %s; rejecting selection", selected);
                return -1; /* Err(format!("invalid selection: ...")) */
            }
            if (idx < n_options) {
                const AbilityEffect *selected_effect = all_options[idx];
                /* remaining = all_options without idx */
                const AbilityEffect *const *remaining = NULL;
                int n_rem = 0;
                /* build remaining list (clone of all_options minus selected) */
                {
                    const AbilityEffect **buf = (const AbilityEffect **)
                        rb_malloc((size_t)(n_options - 1) * sizeof(const AbilityEffect *));
                    int k = 0;
                    for (int i = 0; i < n_options; i++) {
                        if (i != (int)idx) buf[k++] = all_options[i];
                    }
                    remaining = (const AbilityEffect *const *)buf;
                    n_rem = k;
                }

                /* choice.rs:2411 — determine whether to re-prompt after the
                   selected effect completes. */
                int wants_re_prompt = 0;
                if (n_rem > 0) {
                    if (rb_entry_effect_any_number_any(g)) {
                        wants_re_prompt = 1;
                    } else if (rb_entry_effect_has_alt_cond(g)) {
                        const Condition *alt = rb_entry_effect_alt_cond(g);
                        if (rb_eval_condition_with_moved(g, rs, alt) &&
                            strcmp(rb_entry_effect_alt_count_type(g), "any_number") == 0) {
                            wants_re_prompt = 1;
                        }
                    }
                }

                if (wants_re_prompt) {
                    /* choice.rs:2427 — build remaining description from answers. */
                    char desc[512];
                    desc[0] = '\0';
                    for (int i = 0; i < n_rem; i++) {
                        char a[256];
                        rb_effect_answers_str(remaining[i], a, sizeof(a));
                        if (i > 0) strncat(desc, " / ", sizeof(desc) - strlen(desc) - 1);
                        strncat(desc, a[0] ? a : rb_effect_text(remaining[i]),
                                sizeof(desc) - strlen(desc) - 1);
                    }
                    /* choice.rs:2436 — update current queue entry's conditional_choice. */
                    rb_queue_current_entry_set_conditional_choice_effects(g, remaining, n_rem);
                    /* choice.rs:2440 — stage the re-prompt SelectTarget choice. */
                    rs->pending_reprompt = 1;
                    strncpy(rs->reprompt_target, "choice", sizeof(rs->reprompt_target) - 1);
                    strncpy(rs->reprompt_desc, desc, sizeof(rs->reprompt_desc) - 1);
                    strncpy(rs->reprompt_desc_en, desc, sizeof(rs->reprompt_desc_en) - 1);
                    strncpy(rs->reprompt_desc_ja, desc, sizeof(rs->reprompt_desc_ja) - 1);
                    rs->reprompt_allow_skip = 1;
                }

                /* choice.rs:2449 — schedule the selected effect as a pending command. */
                {
                    const AbilityEffect *cmds[1] = { selected_effect };
                    rb_resolver_set_pending_actions(g, cmds, 1);
                }
                rs->has_pending_choice = 0; /* clear stale */
                return rb_resolver_resume_pending_actions(g, rs);
            }
        } else if (rs->has_pending_choice) {
            /* choice.rs:2453 — no effects payload; clear stale pending_choice. */
            rs->has_pending_choice = 0;
        }
        return rb_resolver_resume_pending_actions(g, rs);
    }

    if (strcmp(choice_card_no, "ChoiceString") == 0) {
        /* choice.rs:2459 — route to handle_choice_string_selection. */
        return rb_resolver_handle_choice_string_selection(rs, g, selected, cond_tag);
    }

    if (strncmp(choice_card_no, "Raw:", 4) == 0 &&
        strstr(choice_card_no, "position_change") != NULL) {
        /* choice.rs:2461 — Raw("position_change...") routing. */
        rb_dbgf("[HPCC_MATCH] routing to handle_position_change_choice: s=%s selected=%s",
                choice_card_no, selected);
        return rb_resolver_handle_position_change_choice(rs, g, choice_card_no, selected);
    }

    /* choice.rs:2474 — target-based routing via typed enum. */
    RbSelectTargetKind kind = RB_STK_NONE;
    if (!rb_select_target_kind_from_str(target, &kind)) {
        /* choice.rs:2478 — SelectTargetKind::Choice */
        if (kind == RB_STK_CHOICE) {
            rb_resolver_clear_choice_state(g, rs);
            return 0; /* Ok(()) */
        }
        /* choice.rs:2479 — SelectTargetKind::ChoiceString */
        if (kind == RB_STK_CHOICE_STRING) {
            return rb_resolver_handle_choice_string_store(rs, g, selected, cond_tag);
        }
        /* choice.rs:2482 */
        if (kind == RB_STK_PAY_OPTIONAL_COST_SKIP_OPTIONAL_COST) {
            return rb_resolver_handle_optional_cost_payment(rs, g, selected);
        }
        /* choice.rs:2485 */
        if (kind == RB_STK_PAY_COST_ALL_DISCARD) {
            return rb_resolver_handle_pay_cost_all_discard(rs, g, selected);
        }
        /* choice.rs:2488 */
        if (kind == RB_STK_DOUBLE_BATON_TOUCH) {
            return rb_resolver_handle_double_baton_touch(rs, g, selected);
        }
        /* choice.rs:2491 */
        if (kind == RB_STK_PRIMARY_ALTERNATIVE) {
            return rb_resolver_handle_primary_alternative(rs, g, selected);
        }
        /* choice.rs:2494 */
        if (kind == RB_STK_APPLY_REPLACEMENT) {
            rb_resolver_clear_choice_state(g, rs);
            return 0; /* Ok(()) */
        }
        /* choice.rs:2498 — ChooseRequiredHearts */
        if (kind == RB_STK_CHOOSE_REQUIRED_HEARTS) {
            char buf[128];
            snprintf(buf, sizeof(buf), "chosen_required_hearts:%s", selected);
            rb_push_prohibition(g, buf); /* gs.prohibition_effects.push(...) */
            rb_resolver_clear_choice_state(g, rs);
            return 0; /* Ok(()) */
        }
        /* choice.rs:2504 */
        if (kind == RB_STK_POSITION_DESTINATION) {
            return rb_resolver_handle_position_destination(rs, g, selected);
        }
        /* choice.rs:2507 */
        if (kind == RB_STK_HEART_COLOR) {
            return rb_resolver_handle_heart_selection(rs, g, selected);
        }
        /* choice.rs:2510 */
        if (kind == RB_STK_CHOICE_TYPE) {
            rb_resolver_clear_choice_state(g, rs);
            return 0; /* Ok(()) */
        }
        /* choice.rs:2514 */
        if (kind == RB_STK_CHOICE_CONDITION) {
            return rb_resolver_handle_choice_condition(rs, g, selected);
        }
        /* choice.rs:2517 */
        if (kind == RB_STK_CONDITIONAL_OPTIONAL) {
            return rb_resolver_handle_conditional_optional(rs, g, selected);
        }
        /* choice.rs:2520 */
        if (kind == RB_STK_DRAW_ANY_NUMBER) {
            return rb_resolver_handle_draw_any_number(rs, g, selected);
        }
        /* choice.rs:2523 */
        if (kind == RB_STK_ORDER) {
            return rb_resolver_handle_order_selection(rs, g, selected);
        }
        /* choice.rs:2526 — SelfOrOpponent */
        if (kind == RB_STK_SELF_OR_OPPONENT) {
            const char *chosen;
            if (strcmp(selected, "自分") == 0)      chosen = "self";
            else if (strcmp(selected, "相手") == 0) chosen = "opponent";
            else return -1; /* Err("Invalid choice for SelfOrOpponent") */
            rb_dbgf("[SELFOR] chosen=%s", chosen);
            strncpy(rs->spawn_target, chosen, sizeof(rs->spawn_target) - 1);
            if (rs->current_effect) {
                rb_dbgf("[SELFOR] current.action=%s",
                        rs->current_effect->action ? rs->current_effect->action : "");
                AbilityEffect *inner = rb_effect_steps_first(rs->current_effect);
                if (inner) {
                    AbilityEffect *modified = rb_effect_clone(inner);
                    rb_dbgf("[SELFOR] inner.action=%s la=%d sa=%d",
                            modified->action ? modified->action : "",
                            rb_effect_has_look_action(modified),
                            rb_effect_has_select_action(modified));
                    /* choice.rs:2548 — set the chosen target on the inner effect. */
                    rb_set_chosen_target(modified, chosen);
                    rs->has_pending_choice = 0; /* clear stale */
                    /* choice.rs:2556 — schedule the modified effect. */
                    {
                        const AbilityEffect *cmds[1] = { modified };
                        rb_resolver_set_pending_actions(g, cmds, 1);
                    }
                    int res = rb_resolver_resume_pending_actions(g, rs);
                    rb_dbgf("[SELFOR] after resume: pending=%d res=%d", rs->has_pending_choice, res);
                    if (res == 0) {
                        rb_effect_free(modified);
                        return 0; /* Ok(()) */
                    }
                    rb_dbgf("[SELFOR] inner effect failed");
                    rb_resolver_clear_choice_state(g, rs);
                    rb_effect_free(modified);
                    return 0; /* Ok(()) */
                }
            }
            rb_dbgf("[SELFOR] no inner effect found");
            rb_resolver_clear_choice_state(g, rs);
            return 0; /* Ok(()) */
        }
    }

    /* choice.rs:2581 — default fallback. */
    rb_resolver_clear_choice_state(g, rs);
    return 0; /* Ok(()) */
}
