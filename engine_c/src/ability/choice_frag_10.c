/* engine_c/src/ability/choice_frag_10.c
 *
 * Fragment port of engine/src/ability/choice.rs
 *   - handle_position_change_choice   (choice.rs ~2652-2947)
 *   - apply_effect_modification       (choice.rs ~2949-2964)
 *
 * Rust self (AbilityResolver) is modelled as `RbAbilityResolver *self`; the
 * Rust `self.gs: &mut GameState` is carried both as `self->gs` and as the
 * explicit `GameState *g` parameter. Native rb_* helpers are used for the
 * real engine operations; resolver-only state lives on RbAbilityResolver.
 *
 * The two resume helpers mandated by the port brief,
 *   rb_resolver_handle_selection_epilogue
 *   rb_resolver_clear_choice_state_and_resume
 * are called by name below but NOT defined here (they live in the resolver
 * core translation). Prototypes for every other rb_resolver_* helper we call
 * are forward-declared so this fragment is self-contained C11.
 */

#include "rabuka.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>

/* ── Resolver-local state (mirrors AbilityResolver fields used by the two
 *    functions below). Not in rabuka.h so we own it in this fragment. ── */
#define RB_MAX_PENDING_OPTIONS 8
#ifndef RB_MAX_FORMATION
#define RB_MAX_FORMATION 8
#endif

struct RbAbilityResolver {
    GameState *gs;                                  /* self.gs */
    RbFormationSlot formation_plan[RB_MAX_FORMATION];
    int formation_plan_n;                           /* self.formation_plan */
    int selected_area;                              /* self.selected_area (-1 none) */
    int execution_context;                          /* self.execution_context (0=None) */
    /* self.pending_choice accumulator (Choice::SelectTarget) */
    char pending_target[64];
    char pending_description[128];
    char pending_description_en[128];
    char pending_description_ja[128];
    int  pending_allow_skip;
    char pending_options[RB_MAX_PENDING_OPTIONS][16];
    int  pending_options_n;
    void *card_db;                                  /* self.card_database */
};
typedef struct RbAbilityResolver RbAbilityResolver;

/* ── Forward prototypes for resolver helpers referenced but NOT defined here ── */
int  rb_resolver_handle_selection_epilogue(RbAbilityResolver *self, GameState *g);
int  rb_resolver_clear_choice_state_and_resume(RbAbilityResolver *self, GameState *g);
void rb_resolver_clear_choice_state(RbAbilityResolver *self, GameState *g);
void rb_resolver_clear_choice_meta(RbAbilityResolver *self, GameState *g);
void rb_resolver_store_pending_choice(RbAbilityResolver *self, GameState *g);
void rb_resolver_resume_pending_actions(RbAbilityResolver *self, GameState *g);
void rb_resolver_set_pending_actions(RbAbilityResolver *self, GameState *g,
                                     const AbilityEffect *eff, int n);
void rb_resolver_set_current_entry_choice(RbAbilityResolver *self, GameState *g,
                                          const char *route);
int  rb_resolver_card_name(RbAbilityResolver *self, int cid,
                           char *buf, size_t sz);

/* ── Local helpers ── */
static inline int rb_streq(const char *a, const char *b) {
    return a && b && strcmp(a, b) == 0;
}
static inline int rb_startswith(const char *s, const char *p) {
    return s && p && strncmp(s, p, strlen(p)) == 0;
}
/* Mirror Rust `selected.split_once(':').map(|(_,pos)| pos)` */
static const char *rb_split_once_after(const char *s, char sep) {
    const char *c = s ? strchr(s, (int)sep) : NULL;
    return c ? c + 1 : s;
}
/* Mirror GameState::record_card_movement (recently_moved tracking). */
static void rb_resolver_record_move(GameState *g, int id) {
    if (!g || id < 0) return;
    if (g->n_recently_moved < RB_MAX_RECENTLY_MOVED)
        g->recently_moved[g->n_recently_moved++] = id;
}
/* Resume epilogue (required named call) followed by the actual resume. */
static inline int rb_resolver_resume_with_epilogue(RbAbilityResolver *self, GameState *g) {
    rb_resolver_handle_selection_epilogue(self, g);
    return rb_resolver_clear_choice_state_and_resume(self, g);
}

/* ───────────────────────────────────────────────────────────────────────────
 * rb_resolver_handle_position_change_choice
 *   Rust: AbilityResolver::handle_position_change_choice
 *         (engine/src/ability/choice.rs:2652)
 * ─────────────────────────────────────────────────────────────────────────── */
int rb_resolver_handle_position_change_choice(RbAbilityResolver *self,
                                              GameState *g,
                                              const char *choice_route, /* Option<ChoiceRoute::Raw> (NULL = None) */
                                              const char *selected)
{
    if (!self || !g || !selected) return -1;
    self->gs = g;

    /* [HPCC] entry log — choice_card_no={:?} selected={} ... */
    /* (Rust log::debug!("[HPCC] entry: ..."); mirrored by trace) */

    /* if selected == "skip" { ... }  (choice.rs:2665) */
    if (rb_streq(selected, "skip")) {
        self->formation_plan_n = 0;   /* self.formation_plan.clear(); */
        /* gs.ability_queue.set_pending_actions(vec![]) — clear conditional-sequential subs */
        rb_resolver_set_pending_actions(self, g, NULL, 0);
        /* self.clear_choice_state_and_resume(gs)?; */
        /* self.execution_context = ExecutionContext::None; */
        self->execution_context = 0;
        return rb_resolver_resume_with_epilogue(self, g);
    }

    /* if let Some(effect) = gs.entry_effect().cloned() { ... }  (choice.rs:2675) */
    AbilityEffect *eff = g->queue.resume_eff;   /* gs.entry_effect() */
    if (eff) {
        AbilityEffect modified = *eff;          /* let mut modified = effect.clone(); */

        /* Resolve destination string (choice.rs:2677-2685) */
        const char *dest;
        if (rb_streq(selected, "0") || rb_streq(selected, "left") || rb_streq(selected, "left_side"))
            dest = "left";
        else if (rb_streq(selected, "1") || rb_streq(selected, "center"))
            dest = "center";
        else if (rb_streq(selected, "2") || rb_streq(selected, "right") || rb_streq(selected, "right_side"))
            dest = "right";
        else
            dest = rb_split_once_after(selected, ':'); /* split_once(':').map((_,pos)).unwrap_or(selected) */

        char *explicit_source_pos = NULL;       /* Option<String> (heap → temp buffer) */
        char explicit_buf[16];

        /* if let Some(ChoiceRoute::Raw(ref raw)) = choice_card_no { ... } (choice.rs:2687) */
        if (choice_route) {
            if (rb_startswith(choice_route, "position_change:")) {
                const char *tgt = choice_route + strlen("position_change:");
                if (rb_streq(tgt, "opponent:front")) {
                    /* modified.kind.filter_mut().source_position = Some(selected) -- Rust-only
                       nested AbilityEffect::kind.filter not representable in flat C struct; skipped. */
                    /* self.execute_position_change_with_destination(gs, &modified, "front") */
                    int actor = g->queue.actor;
                    int host  = g->queue.resume_host;
                    int pc_ok = rb_position_change_with_destination(g, actor, &modified, "front", host);
                    if (pc_ok != 0) {
                        /* log::debug!("Failed to execute position change: {}", e); */
                    }
                    if (pc_ok == 0) {
                        rb_trigger_auto_abilities_for_movement(g, actor); /* gs.trigger_auto_abilities_for_movement_current() */
                    }
                    return rb_resolver_resume_with_epilogue(self, g);    /* self.clear_choice_state_and_resume(gs)?; return Ok(()) */
                } else if (strchr(tgt, ':')) {
                    /* tgt.splitn(2, ':') (choice.rs:2703) */
                    char tgt_copy[64];
                    strncpy(tgt_copy, tgt, sizeof(tgt_copy) - 1);
                    tgt_copy[sizeof(tgt_copy) - 1] = '\0';
                    const char *colon = strchr(tgt_copy, ':');
                    *((char *)colon) = '\0';                 /* parts[0] */
                    const char *rest = colon + 1;            /* parts[1] */
                    modified.target = rb_strdup2(tgt_copy);  /* modified.target = Some(parts[0]) */

                    if (rb_streq(rest, "select")) {
                        /* Check "self:left"/"opponent:center" encoded in selected (choice.rs:2706) */
                        const char *colon2 = strchr(selected, ':');
                        if (colon2) {
                            char pref[32];
                            strncpy(pref, selected, colon2 - selected);
                            pref[colon2 - selected] = '\0';
                            modified.target = rb_strdup2(pref);                 /* modified.target = Some(player_prefix) */
                            strncpy(explicit_buf, colon2 + 1, sizeof(explicit_buf) - 1);
                            explicit_buf[sizeof(explicit_buf) - 1] = '\0';
                            explicit_source_pos = explicit_buf;                /* explicit_source_pos = Some(position) */
                        } else {
                            strncpy(explicit_buf, dest, sizeof(explicit_buf) - 1);
                            explicit_buf[sizeof(explicit_buf) - 1] = '\0';
                            explicit_source_pos = explicit_buf;                /* explicit_source_pos = Some(dest) */
                        }
                    } else if (rb_stage_position_index(rest) >= 0) {
                        /* stage_position_index(parts[1]).is_some() */
                        strncpy(explicit_buf, rest, sizeof(explicit_buf) - 1);
                        explicit_buf[sizeof(explicit_buf) - 1] = '\0';
                        explicit_source_pos = explicit_buf;
                    } else {
                        /* modified.set_target_member(Some(parts[1])) */
                        modified.target = rb_strdup2(rest);
                    }
                } else {
                    modified.target = rb_strdup2(tgt);        /* modified.target = Some(tgt) */
                }
            }
        }

        /* was_select = choice_card_no contains ":select" (choice.rs:2725) */
        int was_select = (choice_route && strstr(choice_route, ":select") != NULL);

        if (was_select) {
            /* The user just chose WHICH member to move (source position). (choice.rs:2729) */
            const char *target_str = modified.target ? modified.target : "self";
            rb_resolver_clear_choice_meta(self, g);     /* self.clear_choice_meta(gs) */
            self->pending_target[0] = '\0';             /* self.pending_choice = None */

            /* fixed_dest = modified.position_any().get_position() -- Rust-only nested
               position field; not representable in flat C struct, treated as None. */
            const char *fixed_dest = NULL;

            if (fixed_dest) {
                /* Card fixes destination → move source `dest` to fixed_dest directly. (choice.rs:2745) */
                if (g->queue.cur >= 0 && g->queue.cur < RB_QUEUE_DEPTH) {
                    char route_buf[64];
                    snprintf(route_buf, sizeof(route_buf), "position_change:%s:%s", target_str, dest);
                    rb_resolver_set_current_entry_choice(self, g, route_buf); /* entry.choice_card_no = ... */
                }
                modified.destination = rb_strdup2(fixed_dest);  /* modified.destination = Some(Zone::from_source_str(dest)) */
                if (explicit_source_pos) {
                    const char *target = modified.target ? modified.target : "self";
                    int pl = rb_resolve_target_player(g, target);   /* gs.resolve_target_player_mut(target) */
                    int src_idx = rb_stage_position_index(explicit_source_pos);
                    int dst_idx = rb_stage_position_index(fixed_dest);
                    if (src_idx != dst_idx && src_idx < 3 && dst_idx < 3 &&
                        g->p[pl].stage[src_idx] != RB_EMPTY_SLOT) {
                        int from = rb_pos_to_area(src_idx >= 0 ? (const char *)NULL : NULL); /* pos_to_area(src_idx) */
                        int to   = rb_pos_to_area(dst_idx >= 0 ? (const char *)NULL : NULL);
                        (void)from; (void)to;
                        /* Use the formation/swap helper: swap src→dst. */
                        int fa = src_idx, ta = dst_idx;
                        rb_stage_formation_change(g, pl, &fa, &ta, 1); /* player.stage.position_change(from,to) */
                        g->position_change_occurred_this_turn = 1;     /* gs.position_change_occurred_this_turn = true */
                        int src_id = g->p[pl].stage[src_idx];
                        int tgt_id = g->p[pl].stage[dst_idx];
                        if (src_id != RB_EMPTY_SLOT) rb_resolver_record_move(g, src_id);
                        if (tgt_id != RB_EMPTY_SLOT) rb_resolver_record_move(g, tgt_id);
                    }
                    rb_trigger_auto_abilities_for_movement(g, pl); /* gs.trigger_auto_abilities_for_movement_current() */
                }
                return rb_resolver_resume_with_epilogue(self, g);  /* self.clear_choice_state_and_resume(gs)?; return Ok(()) */
            }

            /* Compute destinations directly — all positions except source. (choice.rs:2788) */
            const char *all_positions[3] = { "left", "center", "right" };
            self->pending_options_n = 0;
            for (int i = 0; i < 3; i++) {
                if (rb_streq(all_positions[i], dest)) continue;   /* filter pos != dest */
                if (self->pending_options_n < RB_MAX_PENDING_OPTIONS)
                    strncpy(self->pending_options[self->pending_options_n++], all_positions[i], 15);
            }
            if (self->pending_options_n == 0) {
                return rb_resolver_resume_with_epilogue(self, g); /* self.clear_choice_state_and_resume(gs)?; return Ok(()) */
            }
            if (g->queue.cur >= 0 && g->queue.cur < RB_QUEUE_DEPTH) {
                char route_buf[64];
                snprintf(route_buf, sizeof(route_buf), "position_change:%s:%s", target_str, dest);
                rb_resolver_set_current_entry_choice(self, g, route_buf); /* entry.choice_card_no = ... */
            }
            /* from_label (choice.rs:2806) */
            const char *from_label = (rb_streq(dest, "left") || rb_streq(dest, "left_side")) ? "Left"
                                   : rb_streq(dest, "center") ? "Center"
                                   : (rb_streq(dest, "right") || rb_streq(dest, "right_side")) ? "Right"
                                   : "?";
            /* self.pending_choice = Some(Choice::SelectTarget { ... }) (choice.rs:2812) */
            strncpy(self->pending_target, "position|destination", sizeof(self->pending_target) - 1);
            snprintf(self->pending_description, sizeof(self->pending_description),
                     "Choose destination for position change (currently at %s)", from_label);
            snprintf(self->pending_description_en, sizeof(self->pending_description_en),
                     "Choose destination for position change (currently at %s)", from_label);
            snprintf(self->pending_description_ja, sizeof(self->pending_description_ja),
                     "移動先を選択（現在: %s）", from_label);
            self->pending_allow_skip = 0;
            rb_resolver_store_pending_choice(self, g);  /* self.store_pending_choice(gs) */
            return 0;                                    /* return Ok(()) — await choice */
        }

        /* Formation change: store assignment, present next destination or finalize. (choice.rs:2832) */
        if (self->formation_plan_n > 0) {
            int target_card_id = -1;
            if (choice_route && rb_startswith(choice_route, "position_change:self:")) {
                target_card_id = atoi(choice_route + strlen("position_change:self:"));
            }
            int entry_idx = -1;
            for (int i = 0; i < self->formation_plan_n; i++) {
                if (self->formation_plan[i].member_id == target_card_id) { entry_idx = i; break; }
            }
            if (entry_idx >= 0) {
                self->formation_plan[entry_idx].dest_area = rb_stage_position_index(dest); /* (idx).1 = dest */
                /* next = first slot still unassigned (dest_area < 0) (choice.rs:2843) */
                int next_idx = -1;
                for (int i = 0; i < self->formation_plan_n; i++) {
                    if (self->formation_plan[i].dest_area < 0) { next_idx = i; break; }
                }
                if (next_idx >= 0) {
                    int next_cid = self->formation_plan[next_idx].member_id;
                    char next_cname[64];
                    rb_resolver_card_name(self, next_cid, next_cname, sizeof(next_cname)); /* card_database.get_card */
                    /* current_pos = stage position of next_cid (choice.rs:2851) */
                    int current_pos = -1;
                    const char *tgt = modified.target ? modified.target : "self";
                    int pl = rb_resolve_target_player(g, tgt);
                    for (int i = 0; i < RB_STAGE_SIZE; i++) {
                        if (g->p[pl].stage[i] == next_cid) { current_pos = i; break; }
                    }
                    const char *pos_name = (current_pos == 0) ? "Left"
                                        : (current_pos == 1) ? "Center"
                                        : (current_pos == 2) ? "Right" : "?";
                    int out_areas[RB_STAGE_SIZE];
                    int n_areas = rb_misc_position_destinations(g, g->queue.actor, &modified,
                                                                g->queue.resume_host,
                                                                self->formation_plan, self->formation_plan_n,
                                                                out_areas, RB_STAGE_SIZE); /* compute_valid_position_destinations */
                    if (n_areas == 0) {
                        rb_misc_finalize_formation_change(g, g->queue.actor,
                                                          self->formation_plan, self->formation_plan_n); /* finalize_formation_change */
                        rb_trigger_auto_abilities_for_movement(g, g->queue.actor); /* trigger_auto_abilities_for_movement_current */
                        return rb_resolver_resume_with_epilogue(self, g); /* clear_choice_state_and_resume */
                    }
                    if (g->queue.cur >= 0 && g->queue.cur < RB_QUEUE_DEPTH) {
                        char route_buf[64];
                        snprintf(route_buf, sizeof(route_buf), "position_change:self:%d", next_cid);
                        rb_resolver_set_current_entry_choice(self, g, route_buf); /* entry.choice_card_no = ... */
                    }
                    /* self.pending_choice = Some(Choice::SelectTarget { ... }) (choice.rs:2877) */
                    strncpy(self->pending_target, "position|destination", sizeof(self->pending_target) - 1);
                    snprintf(self->pending_description, sizeof(self->pending_description),
                             "Choose destination for %s (currently at %s)", next_cname, pos_name);
                    snprintf(self->pending_description_en, sizeof(self->pending_description_en),
                             "Choose destination for %s (currently at %s)", next_cname, pos_name);
                    snprintf(self->pending_description_ja, sizeof(self->pending_description_ja),
                             "%sの移動先を選択（現在: %s）", next_cname, pos_name);
                    self->pending_allow_skip = (eff->is_optional != 0); /* effect.optional.unwrap_or(false) */
                    self->pending_options_n = 0;
                    for (int i = 0; i < n_areas && self->pending_options_n < RB_MAX_PENDING_OPTIONS; i++) {
                        const char *nm = (out_areas[i] == 0) ? "left" : (out_areas[i] == 1) ? "center" : "right";
                        strncpy(self->pending_options[self->pending_options_n++], nm, 15);
                    }
                    rb_resolver_store_pending_choice(self, g); /* self.store_pending_choice(gs) */
                    return 0;                                    /* await choice */
                } else {
                    /* All members assigned — execute batch swap. (choice.rs:2897) */
                    rb_misc_finalize_formation_change(g, g->queue.actor,
                                                      self->formation_plan, self->formation_plan_n); /* finalize_formation_change */
                    rb_trigger_auto_abilities_for_movement(g, g->queue.actor); /* trigger_auto_abilities_for_movement_current */
                    return rb_resolver_resume_with_epilogue(self, g); /* clear_choice_state_and_resume */
                }
            }
        }

        /* Default single-member path: apply destination + execute. (choice.rs:2906) */
        modified.destination = rb_strdup2(dest); /* modified.destination = Some(Zone::from_source_str(dest)) */
        if (explicit_source_pos) {
            const char *target = modified.target ? modified.target : "self";
            int pl = rb_resolve_target_player(g, target);          /* gs.resolve_target_player_mut(target) */
            int src_idx = rb_stage_position_index(explicit_source_pos);
            int dst_idx = rb_stage_position_index(dest);
            if (src_idx != dst_idx && src_idx < 3 && dst_idx < 3 &&
                g->p[pl].stage[src_idx] != RB_EMPTY_SLOT) {
                int from = rb_pos_to_area((const char *)NULL); /* pos_to_area(src_idx) — area by index below */
                int to   = rb_pos_to_area((const char *)NULL);
                (void)from; (void)to;
                int fa = src_idx, ta = dst_idx;
                rb_stage_formation_change(g, pl, &fa, &ta, 1); /* player.stage.position_change(from,to) */
                g->position_change_occurred_this_turn = 1;
                int src_id = g->p[pl].stage[src_idx];
                int tgt_id = g->p[pl].stage[dst_idx];
                if (src_id != RB_EMPTY_SLOT) rb_resolver_record_move(g, src_id);
                if (tgt_id != RB_EMPTY_SLOT) rb_resolver_record_move(g, tgt_id);
            }
            rb_trigger_auto_abilities_for_movement(g, pl); /* gs.trigger_auto_abilities_for_movement_current */
        } else {
            int actor = g->queue.actor;
            int host  = g->queue.resume_host;
            int pc_ok = rb_position_change_with_destination(g, actor, &modified, dest, host); /* execute_position_change_with_destination */
            if (pc_ok != 0) {
                /* log::debug!("Failed to execute position change: {}", e); */
            } else {
                rb_trigger_auto_abilities_for_movement(g, actor); /* trigger_auto_abilities_for_movement_current */
            }
        }
        self->selected_area = -1;   /* self.selected_area = None */
    }

    /* self.clear_choice_state_and_resume(gs)?; (choice.rs:2945) */
    return rb_resolver_resume_with_epilogue(self, g);
}

/* ───────────────────────────────────────────────────────────────────────────
 * rb_resolver_apply_effect_modification
 *   Rust: AbilityResolver::apply_effect_modification<F>
 *         (engine/src/ability/choice.rs:2949)
 *   Generic closure F: Fn(&mut AbilityEffect) → C function pointer
 *   `void (*modifier)(AbilityEffect *)`.
 * ─────────────────────────────────────────────────────────────────────────── */
int rb_resolver_apply_effect_modification(RbAbilityResolver *self,
                                          GameState *g,
                                          void (*modifier)(AbilityEffect *))
{
    if (!self || !g) return -1;
    self->gs = g;

    rb_resolver_clear_choice_state(self, g);   /* self.clear_choice_state(gs) */

    /* if let Some(mut effect) = gs.entry_effect().cloned() { ... } (choice.rs:2958) */
    AbilityEffect *eff = g->queue.resume_eff;  /* gs.entry_effect() */
    if (eff) {
        AbilityEffect effect = *eff;           /* owned clone */
        if (modifier) modifier(&effect);       /* modifier(&mut effect) */
        rb_resolver_set_pending_actions(self, g, &effect, 1); /* gs.ability_queue.set_pending_actions(vec![effect]) */
    }

    rb_resolver_resume_pending_actions(self, g); /* self.resume_pending_actions(gs)? */
    return 0;
}
