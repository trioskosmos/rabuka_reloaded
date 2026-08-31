/* engine_c/src/ability/choice_frag_13.c
 *
 * Port fragment of engine/src/ability/choice.rs (~lines 3278-3381):
 *   - handle_heart_color_selection  -> rb_resolver_handle_heart_color_selection
 *   - handle_choice_condition        -> rb_resolver_handle_choice_condition
 *   - handle_heart_selection         -> rb_resolver_handle_heart_selection
 *
 * Rust path notes:
 *   - The Rust methods live on `AbilityResolver` (`self`). `self.gs` becomes a
 *     `GameState*` here; resolver-local fields with no C struct (sub_choice_created,
 *     pending_choice) are mirrored with module-scope statics (one resolver instance
 *     per process, matching choice_frag_14.c).
 *   - `gs.ability_queue.current_entry_mut()` maps to
 *     `&g->queue.entries[g->queue.cur]`; `gs.activating_card` maps to
 *     `g->queue.entries[g->queue.cur].card_id`. The C `RbQueueEntry` has no
 *     `conditional_choice` field, so that bookkeeping is mirrored by the
 *     `selected_heart_color` field (rabuka.h:562) which gain_resource consumes.
 *   - `gs.prohibition_effects.push(...)` mirrors into `g->prohibition[]` /
 *     `g->n_prohibition`.
 *   - `self.pending_choice = None` maps to `rb_clear_pending_choice(g)`.
 *   - `clear_choice_state + resume_pending_actions` and `finalize_choice` are
 *     delegated to rb_resolver_clear_choice_state_and_resume (owned elsewhere);
 *     `clear_choice_state + Ok(())` is delegated to rb_resolver_handle_selection_epilogue.
 *     Both are forward-declared below and NOT defined in this fragment.
 */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* ── Resolver-local state (mirrors AbilityResolver fields with no C struct) ── */
static int          g_sub_choice_created = 0;   /* AbilityResolver::sub_choice_created */
static AbilityEffect *g_pending_choice = NULL;  /* AbilityResolver::pending_choice (None == NULL) */

/* ── Forward-declared helpers owned by other translation units (do not define) ── */
void rb_resolver_clear_choice_state_and_resume(GameState *g);
int  rb_resolver_handle_selection_epilogue(GameState *g);
AbilityEffect *rb_resolver_entry_cost(GameState *g);            /* gs.entry_cost() */
const char   *rb_player_prefix(const GameState *g);            /* gs.player_prefix() */
void rb_log_entry(GameState *g, const char *text, const char *prefix,
                  int card_id, const char *act_name, const char *kind);
void rb_record_ability_application(GameState *g, int card_id, const char *text,
                                   const char *kind, int host_cid,
                                   int heart_color, int16_t amount);

/* ── Prototypes for the functions defined in this fragment ── */
int rb_resolver_handle_heart_color_selection(GameState *g, const char *selected);
int rb_resolver_handle_choice_condition(GameState *g, const char *selected);
int rb_resolver_handle_heart_selection(GameState *g, int count,
                                       const char *const *colors, int n_colors);

/* Mirror gs.prohibition_effects.push(s): append a prohibition note string. */
static void rb_push_prohibition(GameState *g, const char *s) {
    if (!g || !s) return;
    if (g->n_prohibition < 64) {
        strncpy(g->prohibition[g->n_prohibition], s, sizeof(g->prohibition[0]) - 1);
        g->prohibition[g->n_prohibition][sizeof(g->prohibition[0]) - 1] = '\0';
        g->n_prohibition++;
    }
}

/* Mirror options[idx].text.split("}}").last().unwrap_or(text).trim(): take the
   substring after the final "}}" token (the human-readable choice label). */
static void rb_choice_condition_label(const char *text, char *out, size_t n) {
    const char *label = text ? text : "";
    if (text) {
        const char *p = strstr(label, "}}");
        while (p) { label = p + 2; p = strstr(label, "}}"); }
        while (*label == ' ' || *label == '\t') label++;
    }
    if (out && n > 0) {
        strncpy(out, label, n - 1);
        out[n - 1] = '\0';
    }
}

/* handle_heart_color_selection — Rust: choice.rs:handle_heart_color_selection
   (mirrors the SELECT_HEART_COLOR answer: record the chosen heart color, then
   clear choice state and resume the pending ability). Returns 0 on success. */
int rb_resolver_handle_heart_color_selection(GameState *g, const char *selected) {
    /* Rust: const HEART_VALS: [&str; 7] = [...] */
    static const char *HEART_VALS[7] = {
        "heart00", "heart01", "heart02", "heart03",
        "heart04", "heart05", "heart06"
    };
    if (!g || !selected) return 1;

    /* Rust: let Ok(idx) = selected.parse::<usize>() else { warn; return Err } */
    char *end = NULL;
    long idx = strtol(selected, &end, 10);
    if (end == selected || *end != '\0') {
        /* log::warn!("[HEART_COLOR_SEL] non-numeric index {:?}; rejecting selection") */
        return 1;   /* invalid selection */
    }

    if (idx >= 0 && (size_t)idx < 7) {
        const char *color = HEART_VALS[idx];
        /* Rust: gs.prohibition_effects.push(format!("selected_heart_color:{}", color)) */
        char buf[64];
        snprintf(buf, sizeof buf, "selected_heart_color:%s", color);
        rb_push_prohibition(g, buf);
        /* Rust: if let Some(entry) = gs.ability_queue.current_entry_mut() {
                       entry.conditional_choice = Some(ConditionalChoice::Str(color)) }
           C mirror: queue.selected_heart_color (consumed by gain_resource). */
        g->queue.selected_heart_color = (int)rb_parse_heart_color(color);
    }

    /* Rust: self.clear_choice_state(gs); self.resume_pending_actions(gs) */
    rb_resolver_clear_choice_state_and_resume(g);
    return 0;
}

/* handle_choice_condition — Rust: choice.rs:handle_choice_condition
   (mirrors a ChoiceCondition cost option being picked: pay the chosen cost
   action, preserving any sub-choice it may create). Returns 0 on success. */
int rb_resolver_handle_choice_condition(GameState *g, const char *selected) {
    if (!g || !selected) return 1;

    /* Rust: let Ok(idx) = selected.parse::<usize>() else { warn; return Err } */
    char *end = NULL;
    long idx = strtol(selected, &end, 10);
    if (end == selected || *end != '\0') {
        /* log::warn!("[CHOICE_COND] non-numeric index {:?}; rejecting selection") */
        return 1;   /* invalid selection */
    }

    /* Rust: if let Some(options) = gs.entry_cost().and_then(|c| c.compound.actions.clone()) */
    AbilityEffect *cost = rb_resolver_entry_cost(g);
    if (cost && idx >= 0 && idx < cost->n_child) {
        AbilityEffect *opt = cost->child[idx];

        /* Rust: label = options[idx].text.split("}}").last().unwrap_or(text).trim() */
        char label[256];
        rb_choice_condition_label(opt ? opt->text : NULL, label, sizeof label);

        /* Rust: let pp = gs.player_prefix(); */
        const char *pp = rb_player_prefix(g);

        /* Rust: let act_name = gs.activating_card
                             .and_then(|id| gs.card_database.get_card(id))
                             .map(|c| c.name.to_string()); */
        int cid = g->queue.entries[g->queue.cur].card_id;
        const char *act_name = NULL;
        Card c;
        if (rb_decode_card_by_index((uint32_t)cid, &c)) {
            act_name = c.name;   /* borrowed until rb_free_card */
        }

        /* Rust: gs.log_entry(format!("{pp} {}: [choice] {} ✓", ...), &pp,
                              gs.activating_card, act_name, "choice"); */
        char msg[512];
        snprintf(msg, sizeof msg, "%s %s: [choice] %s ✓",
                 pp ? pp : "", act_name ? act_name : "", label);
        rb_log_entry(g, msg, pp, cid, act_name, "choice");
        if (act_name) rb_free_card(&c);

        /* Rust: take the pending SelectTarget so we can detect if pay_cost
           creates a new sub-choice (e.g. SelectCard for discard from hand). */
        AbilityEffect *old_choice = g_pending_choice;
        g_pending_choice = NULL;

        /* Rust: self.pay_cost(gs, &options[idx])?; */
        rb_pay_cost(g, g->queue.actor, opt);

        /* Rust: if self.pending_choice.is_some() { self.sub_choice_created = true }
                  else { self.pending_choice = old_choice } */
        if (g_pending_choice != NULL) {
            g_sub_choice_created = 1;
        } else {
            g_pending_choice = old_choice;
        }
    }

    /* Rust: self.clear_choice_state(gs); Ok(()) */
    rb_resolver_handle_selection_epilogue(g);
    return 0;
}

/* handle_heart_selection — Rust: choice.rs:handle_heart_selection
   (mirrors a "treat hearts as <color> x N" choice: apply the heart override,
   record the application, then finalize the choice). Returns 0 on success. */
int rb_resolver_handle_heart_selection(GameState *g, int count,
                                       const char *const *colors, int n_colors) {
    if (!g) return 1;

    /* Rust: if let Some(chosen) = colors.first() { ... } */
    if (n_colors > 0 && colors && colors[0]) {
        const char *chosen = colors[0];
        /* Rust: let color = crate::card::parse_heart_color(chosen); */
        RbHeartColor color = rb_parse_heart_color(chosen);

        /* Rust: if let Some(card_id) = gs.activating_card { ... } */
        int cid = g->queue.entries[g->queue.cur].card_id;
        if (cid >= 0) {
            int eff_count = (count < 1) ? 1 : count;   /* Rust: count.max(1) */
            /* Rust: gs.set_heart_override(card_id, color, count.max(1), "live_end") */
            rb_mods_set_heart_override(&g->mods, cid, (int)color);
            /* Rust: gs.record_ability_application(card_id,
                           format!("Treat hearts as {} ×{}", chosen, count.max(1)),
                           "heart_override", card_id, Some(color.index() as u8),
                           count.max(1) as i16); */
            rb_record_ability_application(g, cid,
                /* text */ NULL, "heart_override", cid,
                (int)color, (int16_t)eff_count);
        }

        /* Rust: if let Some(entry) = gs.ability_queue.current_entry_mut() {
                       entry.conditional_choice = Some(ConditionalChoice::Str(chosen.clone())) }
           C mirror: queue.selected_heart_color. */
        g->queue.selected_heart_color = (int)color;
    }

    /* Rust: self.pending_choice = None; self.finalize_choice(gs, &self.execution_context.clone()) */
    rb_clear_pending_choice(g);                       /* self.pending_choice = None */
    rb_resolver_clear_choice_state_and_resume(g);    /* self.finalize_choice(...) */
    return 0;
}
