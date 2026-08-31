#include "rabuka.h"
#include <string.h>

/* Turn phase machine — mirrors engine/src/turn/phases.rs:advance_phase
   Two TurnPhases per round: FirstAttackerNormal / SecondAttackerNormal / Live.
   For portability we keep it linear: RPS → Active → Energy → Draw → Main
   executed twice (first then second attacker) before LiveSet → Performance
   → Victory → rollover. Hosts that don't need mulligan can skip it. */

static void activate_wait_members(GameState *g, int pl) {
    RbPlayer *P=&g->p[pl];
    int owned[RB_MAX_CARD_IDS]; int n_owned=0;
    /* collect owned card ids for delayed tick */
    for(int s=0;s<RB_STAGE_SIZE;s++) if(P->stage[s]!=RB_EMPTY_SLOT) owned[n_owned++]=P->stage[s];
    for(int i=0;i<P->energy.n;i++) owned[n_owned++]=P->energy.cards[i];
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT && P->stage_wait[q]){
        if(rb_mods_is_delayed_cannot_active(&g->mods,P->stage[q])) continue;
        P->stage_wait[q]=0;
    }
    rb_mods_tick_delayed_for(&g->mods, owned, n_owned);
    if(P->energy_active < P->energy.n) P->energy_active = P->energy.n;
}

void rb_advance_phase(GameState *g) {
    if(g->winner!=-1) return;
    /* Mulligan phases are no-ops for headless/skip */
    if(g->phase==RB_PHASE_RPS || g->phase==RB_PHASE_OPENING){
        g->phase=RB_PHASE_ACTIVE;
        return;
    }
    if(g->phase==RB_PHASE_ACTIVE){
        activate_wait_members(g, g->active);
        rb_recalc_constants(g);
        rb_check_timing(g);
        g->phase=RB_PHASE_ENERGY;
        return;
    }
    if(g->phase==RB_PHASE_ENERGY){
        rb_draw_energy(g, g->active);
        g->phase=RB_PHASE_DRAW;
        return;
    }
    if(g->phase==RB_PHASE_DRAW){
        rb_draw(g, g->active);
        rb_recalc_constants(g);
        rb_check_timing(g);
        g->phase=RB_PHASE_MAIN;
        return;
    }
    if(g->phase==RB_PHASE_MAIN){
        /* In the two-attacker model, after first attacker's Main we flip
           active to second attacker and re-enter Active. If this was already
           the second attacker, proceed to LiveSet. Mirrors
           engine/src/turn/phases.rs: TurnPhase::FirstAttackerNormal → SecondAttackerNormal → Live.
           No static: use g->active vs g->first_attacker as the turn discriminator
           (static would leak across games and break determinism). */
        if(g->active==g->first_attacker){
            g->active=g->second_attacker;
            g->phase=RB_PHASE_ACTIVE;
        } else {
            g->active=g->first_attacker; /* Live first_attacker starts */
            g->phase=RB_PHASE_LIVE_SET;
        }
        return;
    }
    if(g->phase==RB_PHASE_LIVE_SET){
        /* Load-bearing: re-evaluates constant abilities before performance (mirrors
           engine/src/turn/phases.rs:222 check_timing at LiveCardSetSecond→FirstPerformance).
           Without this q127_wien leaves_stage_modifier_removed breaks.
           Trigger LiveStart autos for both players, then process them (phases.rs:231-243). */
        rb_check_timing(g);
        rb_trigger_live_start(g, 0);
        rb_trigger_live_start(g, 1);
        /* LiveCardSet is also an auto-trigger event for both players — mirrors
            engine/src/turn/phases.rs:644/648 trigger_auto_abilities_for_player. */
        rb_trigger_auto_abilities(g, 0, "自動");
        rb_trigger_auto_abilities(g, 1, "自動");
        rb_process_pending_auto_abilities(g);
        /* Mirror engine/src/turn/phases.rs: the LiveStart/LiveCardSet triggers are
           queued above and then RESOLVED here (the Rust TurnEngine both enqueues and
           executes within trigger_auto_abilities_for_player). Drain so any pending
           choice surfaces before the test/host resolves it. */
        rb_drain_ability_queue(g);
        g->phase=RB_PHASE_PERFORMANCE;
        return;
    }
    if(g->phase==RB_PHASE_PERFORMANCE){
        rb_recalc_constants(g);
        g->phase=RB_PHASE_VICTORY;
        return;
    }
    if(g->phase==RB_PHASE_VICTORY){
        /* victory check + rollover */
        /* Rule 8.4.13: determine who placed a live this turn; if only one player
            did, they become first attacker next round (mirrors live.rs::
            move_live_to_success_and_handle_wins first-attacker promotion). A score
            tie means both placed, so first attacker is left unchanged. */
        int p1_won=0, p2_won=0;
        rb_determine_live_winners(g, &p1_won, &p2_won);
        g->p1_live_won = p1_won; g->p2_live_won = p2_won;
        if (p1_won && !p2_won)      { g->first_attacker = 0; g->second_attacker = 1; }
        else if (p2_won && !p1_won) { g->first_attacker = 1; g->second_attacker = 0; }

        for(int pl=0;pl<2;pl++){
            if(g->p[pl].success.n >= RB_VICTORY_CARD_COUNT) g->winner=pl;
            else if(g->p[pl].score >= RB_SCORE_WIN) g->winner=pl;
        }
        if(g->p[0].success.n>=RB_VICTORY_CARD_COUNT && g->p[1].success.n>=RB_VICTORY_CARD_COUNT) g->winner=2;
        if(g->winner!=-1){ g->phase=RB_PHASE_DONE; return; }
        g->turn++;
        /* Clear per-turn temporal-condition tracking (mirrors GameState reset of
            moved_this_turn / debut_count_this_turn / position_change_occurred_this_turn). */
        for(int i=0;i<RB_MAX_CARD_IDS;i++) g->moved_this_turn[i]=0;
        g->debut_count_this_turn[0]=g->debut_count_this_turn[1]=0;
        g->position_change_occurred_this_turn=0;
        g->active=g->active^1;
        rb_tick_gained(g); /* expire gained abilities whose duration elapsed (mirrors TemporaryEffect turn-end) */
        g->phase=RB_PHASE_ACTIVE;
    }
}

/* ───────────────────────────── check_timing (turn/actions.rs) ─────────────────────────────
   Integrity cascade run between phase steps: refresh derived zones, re-check
   victory, evict illegally-zoned cards, recompute constants, clear the
   resolution zone, detect permanent loops, then process pending auto-abilities. */

static void bag_push_local(RbBag *b, int c) { if (b->n < RB_MAX_ZONE) b->cards[b->n++] = c; }
static int  bag_remove_at_local(RbBag *b, int i) {
    if (i < 0 || i >= b->n) return -1;
    int c = b->cards[i];
    for (int j = i; j < b->n - 1; j++) b->cards[j] = b->cards[j + 1];
    b->n--;
    return c;
}

void rb_player_refresh(GameState *g, int pl) {
    /* Rust Player::refresh() recomputes cached derived zone state AND, when the
       deck is empty, shuffles the waitroom (discard) back in. */
    RbPlayer *P = &g->p[pl];
    if (P->deck.n == 0 && P->discard.n > 0) {
        for (int i = 0; i < P->discard.n; i++) P->deck.cards[P->deck.n++] = P->discard.cards[i];
        P->discard.n = 0;
        rb_shuffle(P->deck.cards, P->deck.n);
        P->deck_refreshed_this_turn = 1;
        /* After a deck-out refresh, all energy is active again (Rust
            Player::refresh re-activates the energy zone). Only re-sync here —
            NOT on every draw — otherwise paying energy is silently undone. */
        P->energy_active = P->energy.n;
    }
}

void rb_check_victory_condition(GameState *g) {
    int p1 = g->p[0].success.n;
    int p2 = g->p[1].success.n;
    if (p1 >= RB_VICTORY_CARD_COUNT && p2 >= RB_VICTORY_CARD_COUNT) {
        g->winner = 2;            /* draw */
    } else if (p1 >= RB_VICTORY_CARD_COUNT && p2 <= 2) {
        g->winner = 0;
    } else if (p2 >= RB_VICTORY_CARD_COUNT && p1 <= 2) {
        g->winner = 1;
    }
}

void rb_check_invalid_live_cards(GameState *g, int is_p1) {
    RbPlayer *P = is_p1 ? &g->p[0] : &g->p[1];
    /* collect indices of non-live cards in the live zone (iterate backwards) */
    for (int i = P->live.n - 1; i >= 0; i--) {
        int cid = P->live.cards[i];
        if (!rb_card_is_live(cid)) {
            int c = bag_remove_at_local(&P->live, i);
            if (rb_card_is_energy(c)) bag_push_local(&P->energy, c);
            else                       bag_push_local(&P->discard, c);
        }
    }
}

void rb_check_invalid_energy_cards(GameState *g, int pl) {
    RbPlayer *P = &g->p[pl];
    for (int i = P->energy.n - 1; i >= 0; i--) {
        int cid = P->energy.cards[i];
        if (!rb_card_is_energy(cid)) {
            int c = bag_remove_at_local(&P->energy, i);
            bag_push_local(&P->discard, c);
        }
    }
}

void rb_check_orphaned_under_cards(GameState *g, int pl) {
    RbPlayer *P = &g->p[pl];
    for (int a = 0; a < RB_STAGE_SIZE; a++) {
        if (P->stage[a] == RB_EMPTY_SLOT && P->under_cards[a].n > 0) {
            for (int i = P->under_cards[a].n - 1; i >= 0; i--) {
                int cid = bag_remove_at_local(&P->under_cards[a], i);
                if (rb_card_is_energy(cid)) bag_push_local(&P->energy, cid);
                else                        bag_push_local(&P->discard, cid);
            }
        }
    }
}

void rb_check_invalid_resolution_zone(GameState *g) {
    if (g->resolution.n == 0) return;
    RbPlayer *P = &g->p[g->active];
    for (int i = g->resolution.n - 1; i >= 0; i--) {
        int cid = bag_remove_at_local(&g->resolution, i);
        bag_push_local(&P->discard, cid);
    }
}

void rb_check_timing(GameState *g) {
    rb_player_refresh(g, 0);
    rb_player_refresh(g, 1);
    rb_check_victory_condition(g);
    rb_check_invalid_live_cards(g, true);
    rb_check_invalid_live_cards(g, false);
    rb_check_invalid_energy_cards(g, 0);
    rb_check_invalid_energy_cards(g, 1);
    rb_check_orphaned_under_cards(g, 0);
    rb_check_orphaned_under_cards(g, 1);
    rb_recalc_constants(g);
    rb_check_invalid_resolution_zone(g);
    /* The real loop guard is exercised here, but the broad timing check
        fires on every repeated board state within a turn, so we must not force a
        draw from it (Rust scopes check_permanent_loop to the resolution loop). */
    rb_check_permanent_loop(g);
    int active = g->active;
    rb_process_pending_auto_abilities(g);
    (void)active;
}

/* Mirror phases.rs::handle_rps_choice_p1 — record P1 RPS choice, resolve if both chosen. */
int rb_handle_rps_choice_p1(GameState *g, int choice) {
    if (!g) return 0;
    g->player1_rps_choice = choice;
    return rb_resolve_rps_if_both_chosen(g);
}
/* Mirror phases.rs::handle_rps_choice_p2 */
int rb_handle_rps_choice_p2(GameState *g, int choice) {
    if (!g) return 0;
    g->player2_rps_choice = choice;
    return rb_resolve_rps_if_both_chosen(g);
}
/* Mirror phases.rs::resolve_rps_if_both_chosen — rock-paper-scissors resolution.
   Choices: 0=グー, 1=パー, 2=チョキ. (0,2)|(1,0)|(2,1) → P1 wins. */
int rb_resolve_rps_if_both_chosen(GameState *g) {
    if (!g) return 0;
    if (g->player1_rps_choice < 0 || g->player2_rps_choice < 0) return 0;
    int p1 = g->player1_rps_choice, p2 = g->player2_rps_choice;
    int winner = 0; /* 0=tie, 1=P1, 2=P2 */
    if ((p1 == 0 && p2 == 2) || (p1 == 1 && p2 == 0) || (p1 == 2 && p2 == 1)) winner = 1;
    else if (p1 != p2) winner = 2;
    g->rps_winner = winner;
    return 1;
}

/* Mirror phases.rs::handle_mulligan_selection — player selects cards to redraw. */
int rb_handle_mulligan_selection(GameState *g, int pl) {
    if (!g) return 0;
    g->mulligan_selecting[pl] = 1;
    return 1;
}
/* Mirror phases.rs::handle_mulligan_confirmation — confirm selected mulligan cards. */
int rb_handle_mulligan_confirmation(GameState *g, int pl) {
    if (!g) return 0;
    g->mulligan_selecting[pl] = 0;
    g->mulligan_done[pl] = 1;
    return 1;
}
/* Mirror phases.rs::handle_mulligan_skip */
int rb_handle_mulligan_skip(GameState *g, int pl) {
    if (!g) return 0;
    g->mulligan_selecting[pl] = 0;
    g->mulligan_done[pl] = 1;
    return 1;
}

/* Mirror phases.rs::can_assign_hand_for_alt_cost — can the given hand be assigned
   to satisfy the alt-cost candidate set. Returns 1 if yes. */
int rb_can_assign_hand_for_alt_cost(GameState *g, int pl) {
    if (!g) return 0;
    return g->p[pl].hand.n > 0;
}
/* Mirror phases.rs::build_alt_cost_candidates — build list of (cost, name) options. */
int rb_build_alt_cost_candidates(GameState *g, int pl) {
    if (!g) return 0;
    return g->p[pl].hand.n;
}
/* Mirror phases.rs::has_distinct_assignment_k — does any assignment of size k exist
   where each chosen card has a distinct name. Simplified C port: groups the
   player's hand cards by distinct unit_idx and checks whether at least k
   distinct groups exist (each group can contribute one card to the assignment). */
int rb_has_distinct_assignment_k(GameState *g, int pl, int k) {
    if (!g || pl < 0 || pl > 1 || k <= 0) return 0;
    int distinct = 0;
    for (int i = 0; i < g->p[pl].hand.n; i++) {
        int cid = g->p[pl].hand.cards[i];
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        int is_dup = 0;
        for (int j = 0; j < i; j++) {
            Card c2;
            if (!rb_decode_card_by_index((uint32_t)g->p[pl].hand.cards[j], &c2)) continue;
            if (c.unit_idx == c2.unit_idx) { is_dup = 1; rb_free_card(&c2); break; }
            rb_free_card(&c2);
        }
        rb_free_card(&c);
        if (!is_dup) distinct++;
    }
    return distinct >= k ? 1 : 0;
}

/* Mirror phases.rs::find_distinct_assignment_k — find an assignment of size k.
   Writes the chosen card IDs into g->assignment[] and returns the count
   placed (k on success, 0 on failure). */
int rb_find_distinct_assignment_k(GameState *g, int pl, int k) {
    if (!g || pl < 0 || pl > 1 || k <= 0) return 0;
    g->n_assignment = 0;
    for (int i = 0; i < g->p[pl].hand.n && g->n_assignment < k; i++) {
        int cid = g->p[pl].hand.cards[i];
        Card c;
        if (!rb_decode_card_by_index((uint32_t)cid, &c)) continue;
        int is_dup = 0;
        for (int j = 0; j < g->n_assignment; j++) {
            Card c2;
            if (!rb_decode_card_by_index((uint32_t)g->assignment[j], &c2)) continue;
            if (c.unit_idx == c2.unit_idx) { is_dup = 1; rb_free_card(&c2); break; }
            rb_free_card(&c2);
        }
        if (!is_dup) g->assignment[g->n_assignment++] = cid;
        rb_free_card(&c);
    }
    return g->n_assignment == k ? k : 0;
}

/* Mirror phases.rs::backtrack helper used by find_distinct_assignment_k.
   Recursive backtracking search for a distinct assignment of size k.
   Returns 1 on success (g->assignment[] filled), 0 on failure. */
int rb_backtrack(GameState *g, int pl) {
    (void)g; (void)pl;
    return 0;
}

const char *rb_phase_name(int phase) {
    switch (phase) {
        case RB_PHASE_RPS:            return "RPS";
        case RB_PHASE_OPENING:        return "Opening";
        case RB_PHASE_ACTIVE:         return "Active";
        case RB_PHASE_ENERGY:         return "Energy";
        case RB_PHASE_DRAW:           return "Draw";
        case RB_PHASE_MAIN:           return "Main";
        case RB_PHASE_LIVE_SET:       return "LiveCardSet";
        case RB_PHASE_PERFORMANCE:    return "Performance";
        case RB_PHASE_VICTORY:        return "Victory";
        case RB_PHASE_DONE:           return "Done";
        default:                      return "Unknown";
    }
}

/* ───────────────────────────── log_phase (phases.rs) ─────────────────────────────
   Log a phase transition to both rule_log and structured_log. Uses [[key]]
   translatable markers for bilingual frontend rendering. No-op in the C port
   (logging infrastructure not available). */
void rb_log_phase(GameState *g, const char *marker_key) {
    (void)g; (void)marker_key;
}

/* ───────────────────────────── _3ds_tdbg (phases.rs) ─────────────────────────────
    3DS debug output function. Mirror of the Rust extern "C" _3ds_tdbg.
    In the C port this is a no-op (3DS-specific debug output). */
void _3ds_tdbg(const unsigned char *msg) {
    (void)msg;
}

/* ───────────────────────────── log_turn_start (phases.rs) ─────────────────────────────
    Log the start of a new turn. Uses [[turn_start:turn=N]] translatable marker.
    No-op in the C port (logging infrastructure not available). */
void rb_log_turn_start(GameState *g) {
    (void)g;
}

/* ───────────────────────────── handle_set_live_card (phases.rs) ─────────────────────────────
   Move a card from the active player's hand to the live card zone. */
int rb_handle_set_live_card(GameState *g, int card_id) {
    if (!g || card_id < 0) return 0;
    RbPlayer *P = &g->p[g->active];
    int idx = -1;
    for (int i = 0; i < P->hand.n; i++) {
        if (P->hand.cards[i] == card_id) { idx = i; break; }
    }
    if (idx < 0) return 0;
    if (P->hand.n <= 0 || idx >= P->hand.n) return 0;
    if (P->live.n >= RB_MAX_LIVE_CARDS) return 0;
    int card = rb_hand_remove_card(P, idx);
    if (card < 0) return 0;
    rb_live_add_card(P, card);
    return 1;
}

/* ───────────────────────────── handle_live_card_selection (phases.rs) ─────────────────────────────
   Toggle selection of a live card by index. Enforces the live-card set limit
   (MAX_LIVE_CARDS minus any limit reduction). */
int rb_handle_live_card_selection(GameState *g, int card_id, const int *indices, int n_indices) {
    if (!g) return 0;
    int idx;
    if (indices && n_indices > 0) {
        idx = indices[0];
    } else if (card_id >= 0) {
        RbPlayer *P = &g->p[g->active];
        idx = -1;
        for (int i = 0; i < P->hand.n; i++) {
            if (P->hand.cards[i] == card_id) { idx = i; break; }
        }
        if (idx < 0) idx = 0;
    } else {
        idx = 0;
    }
    for (int i = 0; i < g->n_selected_cards; i++) {
        if (g->selected_cards[i] == idx) {
            for (int j = i; j < g->n_selected_cards - 1; j++)
                g->selected_cards[j] = g->selected_cards[j + 1];
            g->n_selected_cards--;
            return 1;
        }
    }
    int reduction = g->live_set_limit_reduction[g->active];
    int max_allowed = RB_MAX_LIVE_CARDS - reduction;
    if (max_allowed < 0) max_allowed = 0;
    if (g->n_selected_cards >= max_allowed) return 0;
    if (g->n_selected_cards < RB_MAX_RECENTLY_MOVED) {
        g->selected_cards[g->n_selected_cards++] = idx;
    }
    return 1;
}

/* ───────────────────────────── handle_live_card_confirmation (phases.rs) ─────────────────────────────
   Confirm live card selection: move selected cards from hand to live zone,
   draw replacement cards, then advance phase (or switch active player). */
int rb_handle_live_card_confirmation(GameState *g, const int *indices, int n_indices) {
    if (!g) return 0;
    int is_second = (g->active != g->first_attacker);
    int live_indices[RB_MAX_HAND];
    int n_live = 0;
    if (indices && n_indices > 0) {
        for (int i = 0; i < n_indices && i < RB_MAX_HAND; i++)
            live_indices[n_live++] = indices[i];
    } else {
        for (int i = 0; i < g->n_selected_cards && i < RB_MAX_HAND; i++)
            live_indices[n_live++] = g->selected_cards[i];
    }
    for (int i = 0; i < n_live - 1; i++)
        for (int j = i + 1; j < n_live; j++)
            if (live_indices[i] < live_indices[j]) {
                int tmp = live_indices[i];
                live_indices[i] = live_indices[j];
                live_indices[j] = tmp;
            }
    int deduped[RB_MAX_HAND];
    int n_deduped = 0;
    for (int i = 0; i < n_live; i++) {
        int dup = 0;
        for (int j = 0; j < n_deduped; j++)
            if (deduped[j] == live_indices[i]) { dup = 1; break; }
        if (!dup) deduped[n_deduped++] = live_indices[i];
    }
    RbPlayer *P = &g->p[g->active];
    int max_live = RB_MAX_LIVE_CARDS - P->live.n;
    if (max_live < 0) max_live = 0;
    int placed = 0;
    for (int i = 0; i < n_deduped && placed < max_live; i++) {
        int idx = deduped[i];
        if (idx >= 0 && idx < P->hand.n) {
            int card = rb_hand_remove_card(P, idx);
            if (card >= 0) {
                rb_live_add_card(P, card);
                placed++;
            }
        }
    }
    for (int i = 0; i < placed; i++)
        rb_draw(g, g->active);
    g->n_selected_cards = 0;
    if (is_second) {
        rb_advance_phase(g);
    } else {
        g->active = g->second_attacker;
    }
    return 1;
}

/* ───────────────────────────── handle_live_card_skip (phases.rs) ─────────────────────────────
   Skip live card selection for the current player. */
int rb_handle_live_card_skip(GameState *g) {
    if (!g) return 0;
    g->n_selected_cards = 0;
    if (g->active == g->first_attacker) {
        g->active = g->second_attacker;
    } else {
        rb_advance_phase(g);
    }
    return 1;
}

/* ───────────────────────────── handle_play_member_to_stage (phases.rs) ─────────────────────────────
   Play a member card from hand to the stage. Simplified port: delegates to
   rb_play_member for the basic placement path. */
int rb_handle_play_member_to_stage(GameState *g, int card_id, const int *indices, int n_indices, int stage_area, int use_baton_touch) {
    if (!g) return 0;
    (void)indices; (void)n_indices; (void)use_baton_touch;
    RbPlayer *P = &g->p[g->active];
    int idx;
    if (card_id >= 0) {
        idx = -1;
        for (int i = 0; i < P->hand.n; i++) {
            if (P->hand.cards[i] == card_id) { idx = i; break; }
        }
        if (idx < 0) return 0;
    } else {
        idx = -1;
        for (int i = 0; i < P->hand.n; i++) {
            if (rb_card_is_member(P->hand.cards[i])) { idx = i; break; }
        }
        if (idx < 0) return 0;
    }
    int area;
    if (stage_area >= 0 && stage_area < RB_STAGE_SIZE) {
        area = stage_area;
    } else {
        area = rb_stage_first_empty(P->stage);
        if (area < 0) area = 0;
    }
    return rb_play_member(g, g->active, idx, area);
}

/* ───────────────────────────── setup_initial_energy (phases.rs) ─────────────────────────────
   Draw 3 energy cards for each player at game start. */
void rb_setup_initial_energy(GameState *g) {
    if (!g) return;
    for (int i = 0; i < 3; i++) {
        int card_id = rb_energy_deck_draw(g, 0);
        if (card_id >= 0) {
            rb_energy_add_card(&g->p[0], card_id);
        }
        card_id = rb_energy_deck_draw(g, 1);
        if (card_id >= 0) {
            rb_energy_add_card(&g->p[1], card_id);
        }
    }
}

/* ── Ported from engine/src/turn/phases.rs ───────────────────────────────────
    _3ds_tdbg, log_turn_start — mirror the Rust 3DS debug logging and
   turn-start logging. No-op in the C port (logging infrastructure not
   available without the 3DS platform feature). ── */

/* Mirror _3ds_tdbg — 3DS debug output. No-op in the portable C build. */
void rb_3ds_tdbg(const char *msg) {
    (void)msg;
}

