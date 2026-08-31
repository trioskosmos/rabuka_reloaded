/* ═══════════════════════════════════════════════════════════════════════════
    Missing functions ported from engine/src/ability/move_cards.rs.
    These mirror the unmatched Rust ability methods that were not yet
    present in engine_c/src/ability/effects/move.c.
    ═══════════════════════════════════════════════════════════════════════════ */

/* ── 1. resolve_cards_from_source ──────────────────────────────────────── */
int rb_move_resolve_cards_from_source(GameState *g, int actor, AbilityEffect *e,
                                       int count, int *out_ids, int max) {
    if (!g || !out_ids || !e) return 0;
    if (count <= 0) count = 1;

    const char *source = e->source ? e->source : "";
    const char *destination = e->destination ? e->destination : "";
    const char *target = e->target ? e->target : "self";
    int use_p2 = (target && !strcmp(target, "opponent")) ? 1 : 0;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];

    const char *source_str = (!source || !*source) ? "discard" : source;

    /* selected_cards: drain from g->selected_cards */
    if (g->n_selected_cards > 0 && !strcmp(source_str, "selected_cards")) {
        int n = 0;
        for (int i = 0; i < g->n_selected_cards && n < max; i++) {
            int cid = g->selected_cards[i];
            int last_vacated = -1;
            remove_card_from_any_zone(P, &last_vacated >= 0 ? &last_vacated : NULL, cid);
            if (last_vacated >= 0)
                g->baton_last_vacated_area[pl] = last_vacated;
            out_ids[n++] = cid;
        }
        g->n_selected_cards = 0;
        return n;
    }

    /* recently_moved relay */
    if (!strcmp(source_str, "recently_moved")) {
        return rb_move_resolve_from_recently_moved(g, use_p2,
            cmf_extra(e, "card_type"), cmf_extra(e, "group_names"),
            out_ids, max);
    }

    /* preceding_moved: cards this sequential step already moved */
    if (!strcmp(source_str, "preceding_moved")) {
        int n = 0;
        const char *chars = cmf_extra(e, "characters");
        for (int i = 0; i < g->n_recently_moved && n < max; i++) {
            int cid = g->recently_moved[i];
            if (cid == -1) continue;
            if (chars && *chars) {
                const char *names[1] = { chars };
                if (!rb_card_matches_characters(cid, names, 1)) continue;
            }
            int last_vacated = -1;
            remove_card_from_any_zone(P, &last_vacated >= 0 ? &last_vacated : NULL, cid);
            if (last_vacated >= 0)
                g->baton_last_vacated_area[pl] = last_vacated;
            out_ids[n++] = cid;
        }
        return n;
    }

    /* looked_at_remaining */
    if (!strcmp(source_str, "looked_at_remaining")) {
        return rb_move_resolve_source_looked_at(g, actor, e, use_p2, count, out_ids, max);
    }

    /* revealed_cards */
    if (!strcmp(source_str, "revealed_cards")) {
        return rb_move_resolve_from_revealed_cards(g, actor, e, count,
            extra_true(e, "all"), extra_true(e, "max"), out_ids, max);
    }

    /* those_cards: resolve to trigger_moved_cards or own moved_cards */
    if (!strcmp(source_str, "those_cards")) {
        int tc_out[RB_MAX_RECENTLY_MOVED];
        int tc_n = 0;
        int fell_through = 0;
        int r = rb_move_resolve_from_those_cards(g, actor, e, use_p2, count,
            tc_out, RB_MAX_RECENTLY_MOVED, &fell_through);
        if (!fell_through) {
            for (int i = 0; i < r && i < max; i++) out_ids[i] = tc_out[i];
            return r;
        }
        /* fell through -> treat as discard */
        source_str = "discard";
    }

    const char *effective_source = source_str;
    return rb_move_resolve_from_zone(g, actor, effective_source, e, use_p2, count, out_ids, max);
}

/* ── 2. resolve_from_revealed_cards ────────────────────────────────────── */
int rb_move_resolve_from_revealed_cards(GameState *g, int actor, AbilityEffect *e,
                                         int count, int is_all, int is_max,
                                         int *out_ids, int max) {
    if (!g || !out_ids || !e) return 0;
    int take_count = is_all ? g->n_revealed : (count < g->n_revealed ? count : g->n_revealed);
    int can_skip = is_max || e->is_optional;
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    const char *cl = cmf_extra(e, "cost_limit");
    int cost_limit = cl ? atoi(cl) : -1;
    const char *clop = cmf_extra(e, "cost_operator");
    const char *cp = cmf_extra(e, "card_property");
    const char *neg = cmf_extra(e, "negation");
    int is_neg = neg && (!strcmp(neg, "true") || !strcmp(neg, "1"));

    /* Collect matching indices */
    int matching[RB_MAX_RECENTLY_MOVED];
    int nm = 0;
    for (int i = 0; i < g->n_revealed && nm < RB_MAX_RECENTLY_MOVED; i++) {
        int cid = g->revealed_cards[i];
        if (ctype && !rb_card_matches_type(cid, ctype)) continue;
        if (gn && !rb_card_matches_group_str(cid, gn)) continue;
        if (cost_limit >= 0 && !rb_card_matches_cost_limit(cid, cost_limit, clop)) continue;
        if (cp && *cp) {
            Card card;
            int has_prop = 0;
            if (rb_decode_card_by_index((uint32_t)cid, &card)) {
                if (!strcmp(cp, "has_blade_heart")) has_prop = rb_card_has_blade_heart(&card);
                else if (!strcmp(cp, "has_score_icon")) has_prop = rb_card_has_score_icon(&card);
                rb_free_card(&card);
            }
            if (is_neg) { if (has_prop) continue; } else { if (!has_prop) continue; }
        }
        matching[nm++] = i;
    }

    if (nm == 0) return 0;
    if (take_count < nm || can_skip) {
        rb_move_prompt_card_selection(g, actor, "revealed_cards", take_count, can_skip, e);
        return -1; /* choice prompted */
    }

    int actual = take_count < nm ? take_count : nm;
    /* Remove in reverse order so indices stay valid */
    int sorted_idx[RB_MAX_RECENTLY_MOVED];
    for (int i = 0; i < actual; i++) sorted_idx[i] = matching[i];
    for (int i = 0; i < actual - 1; i++)
        for (int j = i + 1; j < actual; j++)
            if (sorted_idx[j] > sorted_idx[i]) { int t = sorted_idx[i]; sorted_idx[i] = sorted_idx[j]; sorted_idx[j] = t; }

    int nout = 0;
    for (int k = 0; k < actual; k++) {
        int idx = sorted_idx[k];
        int cid = g->revealed_cards[idx];
        for (int j = idx; j < g->n_revealed - 1; j++)
            g->revealed_cards[j] = g->revealed_cards[j + 1];
        g->n_revealed--;
        /* Remove from physical zones to prevent duplication */
        for (int p = 0; p < 2; p++) {
            RbPlayer *P = &g->p[p];
            for (int i = 0; i < P->discard.n; i++) {
                if (P->discard.cards[i] == cid) {
                    for (int j = i; j < P->discard.n - 1; j++) P->discard.cards[j] = P->discard.cards[j + 1];
                    P->discard.n--; break;
                }
            }
            for (int i = 0; i < P->deck.n; i++) {
                if (P->deck.cards[i] == cid) {
                    for (int j = i; j < P->deck.n - 1; j++) P->deck.cards[j] = P->deck.cards[j + 1];
                    P->deck.n--; break;
                }
            }
        }
        if (nout < max) out_ids[nout++] = cid;
    }
    return nout;
}

/* ── 3. resolve_from_those_cards ───────────────────────────────────────── */
int rb_move_resolve_from_those_cards(GameState *g, int actor, AbilityEffect *e,
                                      int use_p2, int count, int *out_ids, int max,
                                      int *out_fell_through) {
    if (!g || !out_ids || !e) { if (out_fell_through) *out_fell_through = 1; return 0; }
    if (count <= 0) count = 1;

    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    const char *destination = e->destination ? e->destination : "";
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");

    /* Get trigger_moved_cards from queue entry, fall back to recently_moved */
    int trigger_n = 0;
    const int *trigger_cards = rb_entry_trigger_moved_cards(g, &trigger_n);
    int pool[RB_MAX_RECENTLY_MOVED];
    int pool_n = 0;

    if (trigger_cards && trigger_n > 0) {
        for (int i = 0; i < trigger_n && pool_n < RB_MAX_RECENTLY_MOVED; i++)
            pool[pool_n++] = trigger_cards[i];
    } else {
        /* Fall back to own moved cards (recently_moved) */
        for (int i = 0; i < g->n_recently_moved && pool_n < RB_MAX_RECENTLY_MOVED; i++)
            pool[pool_n++] = g->recently_moved[i];
    }

    if (pool_n == 0) {
        if (out_fell_through) *out_fell_through = 1;
        return 0;
    }

    /* Filter by card_type / group */
    int matching[RB_MAX_RECENTLY_MOVED];
    int nm = 0;
    for (int i = 0; i < pool_n; i++) {
        int cid = pool[i];
        if (ctype && !rb_card_matches_type(cid, ctype)) continue;
        if (gn && !rb_card_matches_group_str(cid, gn)) continue;
        matching[nm++] = cid;
    }

    if (nm == 0) {
        if (out_fell_through) *out_fell_through = 0;
        return 0; /* matched nothing -> empty result, NOT fall-through */
    }

    if (nm <= count && (strcmp(destination, "deck_top_or_bottom") == 0 || !e->is_optional)) {
        /* Direct take */
        int take = nm < count ? nm : count;
        if (!e->is_optional) {
            for (int i = 0; i < take; i++) {
                int cid = matching[i];
                for (int j = 0; j < P->discard.n; j++) {
                    if (P->discard.cards[j] == cid) {
                        for (int k = j; k < P->discard.n - 1; k++) P->discard.cards[k] = P->discard.cards[k + 1];
                        P->discard.n--; break;
                    }
                }
            }
        }
        for (int i = 0; i < take && i < max; i++) out_ids[i] = matching[i];
        if (out_fell_through) *out_fell_through = 0;
        return take;
    }

    if (!strcmp(destination, "deck_top_or_bottom")) {
        /* Q252: pick-one choice restricted to waitroom positions */
        int filtered[RB_MAX_RECENTLY_MOVED];
        int nf = 0;
        for (int i = 0; i < nm; i++) {
            int cid = matching[i];
            for (int j = 0; j < P->discard.n; j++) {
                if (P->discard.cards[j] == cid) {
                    int dup = 0;
                    for (int k = 0; k < nf; k++) if (filtered[k] == j) { dup = 1; break; }
                    if (!dup) filtered[nf++] = j;
                }
            }
        }
        char desc[128];
        if (gn && *gn) snprintf(desc, sizeof(desc), "Select 1 %s card to place on deck", gn);
        else snprintf(desc, sizeof(desc), "Select 1 card to place on deck");
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "discard", ctype, 1, 0, NULL);
        rb_choice_set_description(&g->queue.pending, desc);
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
        g->queue.resume_mode = 2;
        g->queue.resume_actor = actor;
        g->queue.resume_host = -1;
        if (out_fell_through) *out_fell_through = 0;
        return -1; /* choice prompted */
    }

    /* Generic: more matching than count -> player chooses */
    int filtered[RB_MAX_RECENTLY_MOVED];
    int nf = 0;
    for (int i = 0; i < nm; i++) {
        int cid = matching[i];
        for (int j = 0; j < P->discard.n; j++) {
            if (P->discard.cards[j] == cid) {
                int dup = 0;
                for (int k = 0; k < nf; k++) if (filtered[k] == j) { dup = 1; break; }
                if (!dup) filtered[nf++] = j;
            }
        }
    }
    char desc[128];
    if (gn && *gn) snprintf(desc, sizeof(desc), "Select %d %s card(s)", count, gn);
    else snprintf(desc, sizeof(desc), "Select card(s)");
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "discard", ctype, count, e->is_optional, NULL);
    rb_choice_set_description(&g->queue.pending, desc);
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
    g->queue.resume_mode = 2;
    g->queue.resume_actor = actor;
    g->queue.resume_host = -1;
    if (out_fell_through) *out_fell_through = 0;
    return -1; /* choice prompted */
}

/* ── 4. ask_optional_move_gate ─────────────────────────────────────────── */
int rb_move_ask_optional_move_gate(GameState *g, int actor, AbilityEffect *e,
                                    const char *source_zone_str,
                                    const char *desc_en, const char *desc_ja) {
    if (!g || !e || !source_zone_str) return 0;
    if (!e->is_optional) return 0;
    if (!rb_move_optional_gate_source(source_zone_str)) return 0;

    /* Check if already decided (conditional_choice present on queue entry) */
    if (g->queue.cur >= 0 && g->queue.cur < RB_QUEUE_DEPTH) {
        /* decided marker: resume_mode != 0 means we already asked */
        if (g->queue.resume_mode == 5) return 0;
    }

    /* Check if source zone has any cards */
    int pl = actor;
    if (e->target && !strcmp(e->target, "opponent")) pl = actor ^ 1;
    RbPlayer *P = &g->p[pl];
    int available = 0;
    if (!strcmp(source_zone_str, "energy_deck")) available = P->energy_deck.n;
    else if (!strcmp(source_zone_str, "deck") || !strcmp(source_zone_str, "deck_top") || !strcmp(source_zone_str, "deck_bottom")) available = P->deck.n;
    else if (!strcmp(source_zone_str, "energy")) available = P->energy.n;
    else available = 1;

    if (available == 0) return 0;

    (void)desc_ja;
    rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, 1, "pay_optional_cost");
    rb_choice_set_description(&g->queue.pending, desc_en);
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_OPTIONAL_COST);
    g->queue.resume_mode = 5;
    g->queue.resume_actor = actor;
    return 1;
}

/* ── 5. resolve_from_standard_zone ─────────────────────────────────────── */
int rb_move_resolve_from_standard_zone(GameState *g, int actor, AbilityEffect *e,
                                        int use_p2, int count,
                                        int *out_ids, int max) {
    if (!g || !out_ids || !e) return 0;
    int pl = use_p2 ? 1 : actor;
    const char *actual_zone = e->source ? e->source : "hand";
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    const char *cl = cmf_extra(e, "cost_limit");
    int cost_limit = cl ? atoi(cl) : -1;
    const char *clop = cmf_extra(e, "cost_operator");
    int is_all = extra_true(e, "all");
    int is_max = extra_true(e, "max");
    int can_skip = e->is_optional || is_max;

    /* Determine can_skip per zone */
    if (!strcmp(actual_zone, "discard")) can_skip = is_max || e->is_optional;
    else if (!strcmp(actual_zone, "hand")) can_skip = e->is_optional || cmf_extra(e, "any_number") != NULL;
    else if (!strcmp(actual_zone, "success_live_zone")) can_skip = e->is_optional;

    return rb_move_take_cards_from_standard_zone(g, actor, actual_zone, e,
        count, is_all, can_skip, out_ids, max);
}

/* ── 6. resolve_from_selected_cards ────────────────────────────────────── */
int rb_move_resolve_from_selected_cards(GameState *g, int actor, AbilityEffect *e,
                                         int use_p2, int count,
                                         int *out_ids, int max) {
    if (!g || !out_ids || !e) return 0;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    int is_all = extra_true(e, "all");

    if (g->n_selected_cards == 0) return 0;

    /* Classify selection: exact if count >= selected, prompt if more */
    int n_sel = g->n_selected_cards;
    if (count >= n_sel || is_all) {
        /* Exact: take all selected */
        int n = 0;
        for (int i = 0; i < n_sel && n < max; i++) {
            int cid = g->selected_cards[i];
            int last_vacated = -1;
            remove_card_from_any_zone(P, &last_vacated >= 0 ? &last_vacated : NULL, cid);
            if (last_vacated >= 0)
                g->baton_last_vacated_area[pl] = last_vacated;
            out_ids[n++] = cid;
        }
        g->n_selected_cards = 0;
        return n;
    }

    /* Prompt: player chooses which of the selected cards */
    rb_move_prompt_card_selection(g, actor, "selected_cards", count, 0, e);
    return -1;
}

/* ── 7. resolve_source_revealed_cards ──────────────────────────────────── */
int rb_move_resolve_source_revealed_cards(GameState *g, int actor, AbilityEffect *e,
                                           int use_p2, int count,
                                           int *out_ids, int max) {
    if (!g || !out_ids || !e) return 0;
    int pl = use_p2 ? 1 : actor;
    RbPlayer *P = &g->p[pl];
    const char *ctype = cmf_extra(e, "card_type");
    const char *gn = cmf_extra(e, "group_names");
    const char *cl = cmf_extra(e, "cost_limit");
    int cost_limit = cl ? atoi(cl) : -1;
    const char *clop = cmf_extra(e, "cost_operator");

    /* Collect cards owned by target player from revealed_cards */
    int owned[RB_MAX_RECENTLY_MOVED];
    int n_owned = 0;
    for (int i = 0; i < g->n_revealed && n_owned < RB_MAX_RECENTLY_MOVED; i++) {
        int cid = g->revealed_cards[i];
        /* Check ownership: card is in one of this player's zones */
        int is_owned = 0;
        for (int j = 0; j < P->hand.n; j++) if (P->hand.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->discard.n; j++) if (P->discard.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < RB_STAGE_SIZE; j++) if (P->stage[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->deck.n; j++) if (P->deck.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->energy.n; j++) if (P->energy.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->energy_deck.n; j++) if (P->energy_deck.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->live.n; j++) if (P->live.cards[j] == cid) { is_owned = 1; break; }
        if (!is_owned) for (int j = 0; j < P->success.n; j++) if (P->success.cards[j] == cid) { is_owned = 1; break; }
        if (is_owned) {
            /* Remove from revealed_cards */
            for (int j = i; j < g->n_revealed - 1; j++) g->revealed_cards[j] = g->revealed_cards[j + 1];
            g->n_revealed--;
            i--;
            owned[n_owned++] = cid;
        }
    }

    if (n_owned == 0) return 0;

    if (n_owned > count) {
        /* Put back and prompt */
        for (int i = 0; i < n_owned; i++) {
            if (g->n_revealed < RB_MAX_ZONE)
                g->revealed_cards[g->n_revealed++] = owned[i];
        }
        rb_move_prompt_card_selection(g, actor, "revealed_cards", count, e->is_optional, e);
        return -1;
    }

    /* Take all owned cards */
    int nout = 0;
    for (int i = 0; i < n_owned && nout < max; i++) {
        int cid = owned[i];
        /* Remove from hand (active player) */
        RbPlayer *A = &g->p[g->active];
        for (int j = 0; j < A->hand.n; j++) {
            if (A->hand.cards[j] == cid) {
                for (int k = j; k < A->hand.n - 1; k++) A->hand.cards[k] = A->hand.cards[k + 1];
                A->hand.n--; break;
            }
        }
        out_ids[nout++] = cid;
    }
    return nout;
}

/* ── 8. maybe_prompt_success_replacement ────────────────────────────────── */
int rb_move_maybe_prompt_success_replacement(GameState *g, int actor, int card_id,
                                              const char *dest, const char *target) {
    if (!g || !dest || !target) return 0;
    if (strcmp(dest, "success_live_zone") != 0) return 0;

    int pl = rb_resolve_target_player(g, target);
    if (pl < 0) pl = actor;
    RbPlayer *P = &g->p[pl];

    /* Check if there are valid live cards in waitroom matching replacement groups.
       The Rust code calls TurnEngine::get_success_replacement_info; in the C port
       we approximate by checking for any live card in the waitroom. */
    int has_live = 0;
    for (int i = 0; i < P->discard.n; i++) {
        int cid = P->discard.cards[i];
        if (rb_card_is_live(cid)) { has_live = 1; break; }
    }
    if (!has_live) return 0;

    rb_emit_choice(g, actor, RB_CHOICE_SELECT_CARD, "discard", "live_card", 1, 1, NULL);
    rb_choice_set_description(&g->queue.pending,
        "Choose a live card from discard to place in your success zone (or skip to place the original card)");
    rb_choice_set_route(&g->queue.pending, RB_ROUTE_SELECT_CARDS);
    g->queue.resume_mode = 2;
    g->queue.resume_actor = actor;
    g->queue.resume_host = card_id;
    return 1;
}

/* ── 9. execute_move_cards ─────────────────────────────────────────────── */
void rb_move_execute_move_cards(GameState *g, int actor, AbilityEffect *e) {
    if (!g || !e) return;

    /* Resolve count */
    int count = 1;
    if (e->count >= 0) count = e->count;
    else {
        int dc = rb_effect_count(g, actor, -1, e, g->last_draw_count);
        count = dc > 0 ? dc : 1;
    }
    if (count <= 0) count = 1;

    const char *source = e->source ? e->source : "";
    const char *destination = e->destination ? e->destination : "discard";
    const char *target = e->target ? e->target : "self";
    int is_max = e->max ? 1 : 0;
    int is_all = e->all_any ? 1 : 0;
    int is_self_cost = 0;
    const char *self_cost_str = cmf_extra(e, "self_cost");
    if (self_cost_str && (!strcmp(self_cost_str, "true") || !strcmp(self_cost_str, "1"))) is_self_cost = 1;

    int use_p2 = (target && !strcmp(target, "opponent")) ? 1 : 0;
    int pl = use_p2 ? 1 : actor;

    /* Handle or_card_types: let player pick which type */
    const char *ctype = cmf_extra(e, "card_type");
    const char *or_types = cmf_extra(e, "or_card_types");
    if (or_types && *or_types) {
        /* Emit a choice for the player to pick a type */
        char desc[128];
        snprintf(desc, sizeof(desc), "Pick card type: %s", or_types);
        rb_emit_choice(g, actor, RB_CHOICE_SELECT_TARGET, NULL, NULL, 1, 0, "choice_string");
        rb_choice_set_description(&g->queue.pending, desc);
        rb_choice_set_route(&g->queue.pending, RB_ROUTE_CONDITIONAL_CHOICE);
        g->queue.resume_mode = 6;
        g->queue.resume_actor = actor;
        g->queue.resume_host = -1;
        return;
    }

    /* Skip selection prompt if stage is full for empty_area destination */
    if (!strcmp(destination, "empty_area") || !strcmp(destination, "stage")) {
        RbPlayer *P = &g->p[pl];
        int has_empty = 0;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] < 0) { has_empty = 1; break; }
        if (!has_empty) return;
    }

    /* Resolve cards from source */
    int taken[RB_MAX_ZONE];
    int n_taken = rb_move_resolve_cards_from_source(g, actor, e, count, taken, RB_MAX_ZONE);

    if (n_taken < 0) return; /* choice prompted */
    if (n_taken == 0) return;

    /* Check for stage-full: return cards to waitroom */
    RbPlayer *P = &g->p[pl];
    int stage_full = (!strcmp(destination, "stage") && !e->allow_occupied_stage_any);
    if (stage_full) {
        int all_occupied = 1;
        for (int i = 0; i < RB_STAGE_SIZE; i++) if (P->stage[i] < 0) { all_occupied = 0; break; }
        if (all_occupied) {
            for (int i = 0; i < n_taken; i++) rb_waitroom_add(P, taken[i]);
            for (int i = 0; i < n_taken; i++) mc_record_movement(g, taken[i]);
            rb_recalc_constants(g);
            return;
        }
    }

    /* Place each card in destination */
    int moved[RB_MAX_ZONE];
    int n_moved = 0;
    for (int i = 0; i < n_taken; i++) {
        int cid = taken[i];

        /* Success zone replacement check */
        if (rb_move_maybe_prompt_success_replacement(g, actor, cid, destination, target))
            return;

        if (!strcmp(destination, "deck_top_or_bottom")) {
            rb_move_prompt_deck_top_or_bottom(g, actor, cid, target, source, e->is_optional);
            return;
        }

        int placed = rb_move_place_card_with_stage_choice(g, actor, -1, target, cid,
            destination, -1, is_max, count, NULL, -1, source,
            e->allow_occupied_stage_any, 0);
        if (placed == 1) return; /* choice prompted */
        if (placed == 0) {
            moved[n_moved++] = cid;
            rb_move_fire_debut_side_effects(g, actor, cid, target, NULL);
        }
    }

    /* Finalize movement */
    for (int i = 0; i < n_moved; i++) mc_record_movement(g, moved[i]);
    rb_recalc_constants(g);
}
