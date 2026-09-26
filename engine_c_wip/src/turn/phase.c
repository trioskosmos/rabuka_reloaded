#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* Defined below (mirrors phases.rs:318 execute_performance_phase). Declared here
   because rb_advance_phase dispatches the two performance windows into it. */
void rb_execute_performance_phase(GameState *g, int is_first);

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
    for(int q=0;q<RB_STAGE_SIZE;q++) if(P->stage[q]!=RB_EMPTY_SLOT &&
        (P->stage_wait[q] || (rb_mods_get_orientation(&g->mods, P->stage[q]) &&
                             !strcmp(rb_mods_get_orientation(&g->mods, P->stage[q]), "wait")))) {
        if(rb_mods_is_delayed_cannot_active(&g->mods,P->stage[q])) continue;
        P->stage_wait[q]=0;
        rb_mods_set_orientation(&g->mods, P->stage[q], "active");
    }
    rb_mods_tick_delayed_for(&g->mods, owned, n_owned);
    int excluded_energy = 0;
    for (int i = 0; i < P->energy.n; i++)
        if (rb_mods_is_delayed_cannot_active(&g->mods, P->energy.cards[i])) excluded_energy++;
    P->energy_active = P->energy.n - excluded_energy;
    if(P->energy_active < 0) P->energy_active = 0;
}

void rb_advance_phase(GameState *g) {
    if(g->winner!=-1) return;
    if(rb_has_pending_choice(g)) return;
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
        RbPlayer *P = &g->p[g->active];
        int placed = P->live.n;
        int hand_before_refill = P->hand.n;
        for (int i = 0; i < placed; i++) rb_draw(g, g->active);
        fprintf(stderr, "[LIVE_ZONE_REFILL] pl=%d placed=%d hand_before=%d hand_after=%d\n",
                g->active, placed, hand_before_refill, P->hand.n);
        if (g->active == g->first_attacker) {
            g->active = g->second_attacker;
            return;
        }
        g->active = g->first_attacker;
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
        /* Two windows per live: the first attacker performs, then the second,
           then LiveVictoryDetermination. The window body — performer selection,
           the post-snapshot auto-ability sweeps and the phase transition — is
           rb_execute_performance_phase (mirrors phases.rs:256-265 dispatching
           FirstAttackerPerformance / SecondAttackerPerformance into
           execute_performance_phase(is_first)). live_batch_mode keeps the two
           windows from settling the round between them (mirrors the Rust pair
           of phases being separate TurnPhase values). */
        g->live_batch_mode = 1;
        rb_execute_performance_phase(g, g->active == g->first_attacker);
        return;
    }
    if(g->phase==RB_PHASE_VICTORY){
        if(g->live_victory_pending){
            rb_execute_live_victory_determination(g);
            fprintf(stderr, "[LIVE_VICTORY_PHASE] resume pending=%d\n", rb_has_pending_choice(g));
            if(rb_has_pending_choice(g)) return;
            g->live_victory_pending = 0;
        }
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
        /* Clear stage_arrived tracking (mirrors rb_turn's reset of baton arrival-ban).
            Without this, baton touch remains blocked on the next turn because the
            existing member's arrival flag is still set. */
        for(int p=0;p<2;p++) for(int i=0;i<RB_STAGE_SIZE;i++) g->stage_arrived[p][i]=0;
        g->active=g->first_attacker;
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
            if (rb_card_is_energy(c)) bag_push_local(&P->energy_deck, c);
            else                       bag_push_local(&P->discard, c);
            rb_record_card_movement(g, c, RB_ZONEID_LIVE_CARD_ZONE,
                                    rb_card_is_energy(c) ? RB_ZONEID_ENERGY_DECK : RB_ZONEID_WAITROOM,
                                    is_p1 ? 0 : 1, 0);
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
    rb_push_rps_log(g, p1, p2, winner == 0 ? "tie" : winner == 1 ? "p1" : "p2");
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

/* ───────────────────────────── play-time cost reduction ─────────────────────────────
   Faithful port of engine/src/turn/phases.rs:1317-1670:
     play_time_cost_reduction_hook        (phases.rs:1317)
     play_time_cost_reduction_amount      (phases.rs:1437)
     play_time_alt_cost_chars             (phases.rs:1472)
     normalize_member_name                (phases.rs:1505)
     has_play_time_alt_cost_hand_cards    (phases.rs:1509)
     can_assign_hand_for_alt_cost         (phases.rs:1518)
     build_alt_cost_candidates            (phases.rs:1529)
     has_distinct_assignment_k            (phases.rs:1557)
     find_distinct_assignment_k           (phases.rs:1561)
     discard_play_time_alt_cost           (phases.rs:1589)
     shuffle_waitroom_members_to_deck_bottom (phases.rs:1639)

   The C model has no Option<i8> / Option<Vec<String>>; "absent" is the
   RB_PTC_NONE sentinel and the character list is a CSV buffer, matching how
   the rest of the C port represents the decoded string extras. */

/* Rust: Option<i8>::None / Option<(Vec<String>, i16)>::None */
#define RB_PTC_NONE (-32768)
/* Upper bound on the named characters in one play-time alt-cost ability. The
   Rust code has no explicit cap (it is a Vec); RB_MAX_ZONE is the same bound
   the C port uses everywhere else for a decoded list. */
#define RB_PTC_MAX_CHARS 8
/* Name buffer per character slot (Rust: Vec<String>, unbounded). */
#define RB_PTC_NAME_MAX 64

static const char *ptc_effect_extra(const AbilityEffect *e, const char *key) {
    if (!e || !key) return NULL;
    for (int i = 0; i < e->n_extra; i++)
        if (e->extra_k[i] && !strcmp(e->extra_k[i], key)) return e->extra_v[i];
    return NULL;
}

static int ptc_effect_extra_int(const AbilityEffect *e, const char *key, int def) {
    const char *v = ptc_effect_extra(e, key);
    if (!v || !*v) return def;
    char *end = NULL;
    long n = strtol(v, &end, 10);
    if (end == v) return def;
    return (int)n;
}

/* Rust: effect.<flag>_any().unwrap_or(false) — a wire boolean that may also be
   spelled as the string "true" (the C decoder stringifies booleans). */
static int ptc_effect_flag(const AbilityEffect *e, const char *key) {
    const char *v = ptc_effect_extra(e, key);
    if (!v) {
        /* dedicated struct fields the C decoder fills directly */
        if (!strcmp(key, "optional"))     return e->is_optional;
        if (!strcmp(key, "is_further"))   return e->is_further;
        if (!strcmp(key, "conditional"))  return e->conditional_flag;
        if (!strcmp(key, "conditional_negation")) return e->conditional_negation;
        if (!strcmp(key, "per_unit"))     return e->per_unit != 0;
        if (!strcmp(key, "distinct"))     return e->distinct_flag != 0;
        return 0;
    }
    return !strcmp(v, "true");
}

/* Split a decoded CSV extra ("a,b,\"c,d\"") into slots. Returns the count. */
static int ptc_split_csv(const char *csv, char out[][RB_PTC_NAME_MAX], int max) {
    if (!csv || !*csv) return 0;
    int n = 0;
    const char *p = csv;
    while (*p && n < max) {
        char *w = out[n];
        int q = 0;
        while (*p == ' ' || *p == ',') p++;
        if (!*p) break;
        if (*p == '"') {
            p++;
            while (*p && *p != '"') { if (q < RB_PTC_NAME_MAX - 1) w[q++] = *p; p++; }
            if (*p == '"') p++;
        } else {
            while (*p && *p != ',') { if (q < RB_PTC_NAME_MAX - 1) w[q++] = *p; p++; }
        }
        w[q] = '\0';
        if (q) n++;
    }
    return n;
}

/* Condition field accessors. Rust reads these through
   Condition::get_location / get_all / get_card_type (engine/src/core/card.rs),
   which all project the flat "common" field set; the C Condition keeps the same
   flat field list, so a key scan is the equivalent read. */
static const char *ptc_cond_str(const Condition *c, const char *key) {
    if (!c || !key) return NULL;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        if (!c->fields[i].key || strcmp(c->fields[i].key, key) != 0) continue;
        if (c->fields[i].v.tag == RB_TAG_STR) return c->fields[i].v.s;
    }
    return NULL;
}

static int ptc_cond_bool(const Condition *c, const char *key) {
    if (!c || !key) return 0;
    for (uint32_t i = 0; i < c->n_fields; i++) {
        if (!c->fields[i].key || strcmp(c->fields[i].key, key) != 0) continue;
        if (c->fields[i].v.tag == RB_TAG_TRUE) return 1;
        if (c->fields[i].v.tag == RB_TAG_STR && c->fields[i].v.s &&
            !strcmp(c->fields[i].v.s, "true")) return 1;
    }
    return 0;
}

/* Rust: normalize_member_name (phases.rs:1505) — `s.replace([' ', '　'], "")`.
   Drops the ASCII space and the ideographic space U+3000 (E3 80 80). */
void rb_normalize_member_name(const char *src, char *out, size_t out_sz) {
    if (!out || !out_sz) return;
    out[0] = '\0';
    if (!src) return;
    size_t w = 0;
    for (const unsigned char *p = (const unsigned char *)src; *p; ) {
        if (*p == ' ') { p++; continue; }
        size_t n = 1;
        if (p[0] == 0xE3 && p[1] == 0x80 && p[2] == 0x80) n = 3;
        if (w + n >= out_sz) break;
        memcpy(out + w, p, n);
        w += n;
        p += n;
    }
    out[w] = '\0';
}

/* Rust: find_distinct_assignment_k::backtrack (phases.rs:1562-1579).
   Recursive search over the candidate matrix: slot `level` picks any candidate
   not already used; success when every slot is filled. `acc` accumulates the
   chosen card ids in slot order. Returns 1 on success, 0 on failure. */
int rb_backtrack(int cands[][RB_MAX_HAND], const int *cand_counts, int n_levels,
                 int level, int *used, int *n_used, int *acc, int *n_acc) {
    if (level >= n_levels) return 1;
    for (int i = 0; i < cand_counts[level]; i++) {
        int cid = cands[level][i];
        int is_used = 0;
        for (int j = 0; j < *n_used; j++) if (used[j] == cid) { is_used = 1; break; }
        if (is_used) continue;
        used[(*n_used)++] = cid;
        acc[(*n_acc)++] = cid;
        if (rb_backtrack(cands, cand_counts, n_levels, level + 1,
                         used, n_used, acc, n_acc))
            return 1;
        (*n_acc)--;
        (*n_used)--;
    }
    return 0;
}

/* Rust: build_alt_cost_candidates (phases.rs:1529-1553). One candidate list per
   required character name: every hand card that is a member and whose
   normalized name CONTAINS the normalized needle. `exclude_id` is the card
   being played (it is in hand but cannot pay for itself). Returns 0 when any
   slot has no candidate (Rust: None). */
int rb_build_alt_cost_candidates(const GameState *g, int pl, int exclude_id,
                                const char *const *chars, int n_chars,
                                int cands[][RB_MAX_HAND], int *cand_counts) {
    if (!g || pl < 0 || pl > 1 || !chars || !cands || !cand_counts) return 0;
    if (n_chars <= 0 || n_chars > RB_PTC_MAX_CHARS) return 0;
    for (int i = 0; i < n_chars; i++) cands[i][0] = 0, cand_counts[i] = 0;
    for (int i = 0; i < n_chars; i++) {
        char needle[RB_PTC_NAME_MAX];
        rb_normalize_member_name(chars[i], needle, sizeof(needle));
        for (int h = 0; h < g->p[pl].hand.n; h++) {
            int cid = g->p[pl].hand.cards[h];
            if (cid == exclude_id) continue;
            if (!rb_card_is_member(cid)) continue;
            Card c;
            if (!rb_card_get_card_by_id(cid, &c)) continue;
            char name[RB_PTC_NAME_MAX];
            rb_normalize_member_name(c.name ? c.name : "", name, sizeof(name));
            rb_free_card(&c);
            if (!strstr(name, needle)) continue;
            if (cand_counts[i] < RB_MAX_HAND) cands[i][cand_counts[i]++] = cid;
        }
        if (cand_counts[i] == 0) return 0;
    }
    return 1;
}

/* Rust: find_distinct_assignment_k (phases.rs:1561-1587). Writes the chosen
   card ids to `out` (slot order) and returns the number chosen, or 0 when no
   distinct assignment exists (Rust: None). */
int rb_find_distinct_assignment_k(const GameState *g, int pl, int exclude_id,
                                 const char *const *chars, int n_chars,
                                 int *out, int out_cap) {
    if (!g || !out || out_cap < n_chars) return 0;
    int cands[RB_PTC_MAX_CHARS][RB_MAX_HAND];
    int cand_counts[RB_PTC_MAX_CHARS];
    if (!rb_build_alt_cost_candidates(g, pl, exclude_id, chars, n_chars,
                                      cands, cand_counts))
        return 0;
    int used[RB_MAX_HAND];  int n_used = 0;
    int acc[RB_PTC_MAX_CHARS]; int n_acc = 0;
    if (!rb_backtrack(cands, cand_counts, n_chars, 0, used, &n_used, acc, &n_acc))
        return 0;
    for (int i = 0; i < n_acc && i < out_cap; i++) out[i] = acc[i];
    return n_acc;
}

/* Rust: has_distinct_assignment_k (phases.rs:1557) — existence only. */
int rb_has_distinct_assignment_k(const GameState *g, int pl, int exclude_id,
                                 const char *const *chars, int n_chars) {
    return rb_find_distinct_assignment_k(g, pl, exclude_id, chars, n_chars,
                                         NULL, 0) > 0;
}

/* Rust: can_assign_hand_for_alt_cost (phases.rs:1518-1527). */
int rb_can_assign_hand_for_alt_cost(const GameState *g, int pl, int exclude_id,
                                    const char *const *chars, int n_chars) {
    if (!g || pl < 0 || pl > 1) return 0;
    int cands[RB_PTC_MAX_CHARS][RB_MAX_HAND];
    int cand_counts[RB_PTC_MAX_CHARS];
    if (!rb_build_alt_cost_candidates(g, pl, exclude_id, chars, n_chars,
                                      cands, cand_counts))
        return 0;
    int used[RB_MAX_HAND];  int n_used = 0;
    int acc[RB_PTC_MAX_CHARS]; int n_acc = 0;
    return rb_backtrack(cands, cand_counts, n_chars, 0, used, &n_used, acc, &n_acc);
}

/* Locate the 常時 (Constant) modify_cost ability on `card_id` that plays the
   role of the play-time cost reduction, i.e. the one gated on "all member cards
   in the discard" (phases.rs:1448-1460). Requiring all + member_card excludes
   ordinary conditional play-cost abilities. Returns 1 and fills *out (caller
   frees with rb_free_ability) when such an ability exists. */
static int ptc_find_reduction_ability(int card_id, Ability *out) {
    if (card_id < 0) return 0;
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int keep = 0;
        if (rb_ability_has_trigger(&ab, RB_TK_CONSTANT) && ab.effect &&
            ab.effect->action && !strcmp(ab.effect->action, "modify_cost")) {
            const Condition *c = ab.effect->condition;
            const char *loc = ptc_cond_str(c, "location");
            const char *ct  = ptc_cond_str(c, "card_type");
            if (loc && !strcmp(loc, "discard") &&
                ptc_cond_bool(c, "all") &&
                ct && !strcmp(ct, "member_card"))
                keep = 1;
        }
        if (keep) { *out = ab; return 1; }
        rb_free_ability(&ab);
    }
    return 0;
}

/* Locate the generic play-time alternative cost on `card_id` (phases.rs:1472-1502):
   常時 modify_cost with operation=set, location=hand, optional, a `value`, and a
   non-empty named-character list. On success writes the names into `names` and
   the set-cost value into *set_value. Returns 1 on success. */
static int ptc_find_alt_cost_ability(int card_id, char names[][RB_PTC_NAME_MAX],
                                     int *n_names, int *set_value) {
    if (card_id < 0) return 0;
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int keep = 0;
        if (rb_ability_has_trigger(&ab, RB_TK_CONSTANT) && ab.effect &&
            ab.effect->action && !strcmp(ab.effect->action, "modify_cost")) {
            const AbilityEffect *e = ab.effect;
            const char *op = ptc_effect_extra(e, "operation");
            const char *loc = ptc_effect_extra(e, "location");
            const char *chars = ptc_effect_extra(e, "characters");
            int has_value = ptc_effect_extra(e, "value") != NULL;
            if (has_value && op && !strcmp(op, "set") &&
                loc && !strcmp(loc, "hand") && ptc_effect_flag(e, "optional") &&
                chars) {
                int k = ptc_split_csv(chars, names, RB_PTC_MAX_CHARS);
                if (k > 0) {
                    *n_names = k;
                    *set_value = ptc_effect_extra_int(e, "value", 0);
                    keep = 1;
                }
            }
        }
        if (keep) { rb_free_ability(&ab); return 1; }
        rb_free_ability(&ab);
    }
    return 0;
}

/* Rust: play_time_cost_reduction_amount (phases.rs:1437-1463). Returns the
   reduction amount, or RB_PTC_NONE when the card has no play-time reduction. */
int rb_play_time_cost_reduction_amount(const GameState *g, int card_id) {
    (void)g; /* Rust reads only the card database here too. */
    Ability ab;
    if (!ptc_find_reduction_ability(card_id, &ab)) return RB_PTC_NONE;
    int amt = ab.effect && ab.effect->count >= 0 ? ab.effect->count : 0;
    rb_free_ability(&ab);
    return amt;
}

/* Rust: play_time_alt_cost_chars (phases.rs:1472-1502). Writes the required
   character names as CSV into `chars` and returns the set-cost value, or
   RB_PTC_NONE when the card has no generic play-time alternative cost. */
int rb_play_time_alt_cost_chars(const GameState *g, int card_id, char *chars,
                                int chars_len) {
    (void)g;
    if (!chars || chars_len <= 0) return RB_PTC_NONE;
    chars[0] = '\0';
    char names[RB_PTC_MAX_CHARS][RB_PTC_NAME_MAX];
    int n_names = 0, set_value = 0;
    if (!ptc_find_alt_cost_ability(card_id, names, &n_names, &set_value))
        return RB_PTC_NONE;
    int w = 0;
    for (int i = 0; i < n_names; i++) {
        int k = snprintf(chars + w, (size_t)(chars_len - w), "%s%s",
                         i ? "," : "", names[i]);
        if (k < 0 || k >= chars_len - w) break;
        w += k;
    }
    return set_value;
}

/* Rust: has_play_time_alt_cost_hand_cards (phases.rs:1509-1516) — the ACTIVE
   player's hand must be able to cover every named character. */
int rb_has_play_time_alt_cost_hand_cards(const GameState *g, int card_id) {
    if (!g) return 0;
    char names[RB_PTC_MAX_CHARS][RB_PTC_NAME_MAX];
    int n_names = 0, set_value = 0;
    if (!ptc_find_alt_cost_ability(card_id, names, &n_names, &set_value)) return 0;
    const char *chars[RB_PTC_MAX_CHARS];
    for (int i = 0; i < n_names; i++) chars[i] = names[i];
    return rb_can_assign_hand_for_alt_cost(g, g->active, card_id, chars, n_names);
}

/* Rust: discard_play_time_alt_cost (phases.rs:1589-1636) — move one distinct
   hand card per named character to the waitroom, and record the moves.
   Returns 1 on success, 0 when the hand cannot cover the cost. */
int rb_discard_play_time_alt_cost(GameState *g, int pl, int played_card_id) {
    if (!g || pl < 0 || pl > 1) return 0;
    char names[RB_PTC_MAX_CHARS][RB_PTC_NAME_MAX];
    int n_names = 0, set_value = 0;
    if (!ptc_find_alt_cost_ability(played_card_id, names, &n_names, &set_value))
        return 0;
    const char *chars[RB_PTC_MAX_CHARS];
    for (int i = 0; i < n_names; i++) chars[i] = names[i];
    int to_discard[RB_PTC_MAX_CHARS];
    int n = rb_find_distinct_assignment_k(g, pl, played_card_id, chars, n_names,
                                          to_discard, RB_PTC_MAX_CHARS);
    if (n <= 0) return 0;
    RbPlayer *P = &g->p[pl];
    for (int i = 0; i < n; i++) {
        int idx = -1;
        for (int j = 0; j < P->hand.n; j++)
            if (P->hand.cards[j] == to_discard[i]) { idx = j; break; }
        if (idx < 0) continue;
        int cid = rb_hand_remove_card(P, idx);
        if (cid >= 0 && P->discard.n < RB_MAX_ZONE)
            P->discard.cards[P->discard.n++] = cid;
    }
    return 1;
}

/* Rust: shuffle_waitroom_members_to_deck_bottom (phases.rs:1639-1670).
   ONLY member cards leave the waitroom: they are shuffled among themselves and
   appended to the BOTTOM of the deck; every non-member waitroom card stays. */
void rb_shuffle_waitroom_members_to_deck_bottom(GameState *g, int pl) {
    if (!g || pl < 0 || pl > 1) return;
    RbPlayer *P = &g->p[pl];
    int members[RB_MAX_ZONE];
    int n_members = 0;
    int remaining[RB_MAX_ZONE];
    int n_remaining = 0;
    for (int i = 0; i < P->discard.n; i++) {
        int cid = P->discard.cards[i];
        if (rb_card_is_member(cid)) {
            if (n_members < RB_MAX_ZONE) members[n_members++] = cid;
        } else {
            if (n_remaining < RB_MAX_ZONE) remaining[n_remaining++] = cid;
        }
    }
    rb_shuffle(members, n_members);
    for (int i = 0; i < n_remaining; i++) P->discard.cards[i] = remaining[i];
    P->discard.n = n_remaining;
    for (int i = 0; i < n_members; i++) {
        if (P->deck.n < RB_MAX_ZONE) P->deck.cards[P->deck.n++] = members[i];
    }
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


/* Rust live.rs:2120 — `draw_effects_occurred` is true when any card revealed by
   the yell carried a draw icon. live.c computes the icon tallies inside
   do_yell; the predicate is re-derived here from the revealed pool (the same
   blade-heart / special-heart classification as live.c:79-106) because the
   phase machine needs it after rb_perform_live returns. */
static int ptc_yell_had_draw_icon(const GameState *g) {
    for (int i = 0; i < g->n_revealed; i++) {
        Card c;
        if (!rb_decode_card_by_index((uint32_t)g->revealed_cards[i], &c)) continue;
        int start = c.num_base;
        int end = start + c.num_blade;
        if (end > c.n_hearts) end = c.n_hearts;
        for (int h = start; h < end; h++)
            if (c.heart_color[h] == RB_HEART_DRAW && c.heart_count[h] > 0) {
                rb_free_card(&c);
                return 1;
            }
        int special_is_draw = c.has_special && c.special_color == RB_HEART_DRAW &&
                              c.special_count > 0;
        rb_free_card(&c);
        if (special_is_draw) return 1;
    }
    return 0;
}

/* ── execute_performance_phase (phases.rs:318-662) ─────────────────────────────────
   The C port splits this Rust function: the yell reveal, the 8.3.14-8.3.16 heart
   calculation, the success check and the snapshot are owned by
   src/turn/live.c:rb_perform_live (the Rust player_perform_live /
   check_live_success / build_snapshot trio). What belongs to the phase machine —
   and what this function owns — is the window sequencing, the post-snapshot
   auto-ability sweep for BOTH players, and the phase transition:

     phases.rs:335-346  performer = first_attacker (is_first) / second_attacker
     phases.rs:352      check_timing before the window opens
     phases.rs:382-527  player_perform_live + check_live_success  -> rb_perform_live
     phases.rs:630-641  build_snapshot + push_performance_snapshot -> rb_perform_live
     phases.rs:643-648  trigger + process the performer's auto abilities, then the
                        OPPONENT's (「相手Ponにライブされたとき」)
     phases.rs:649-654  when draw effects occurred, repeat both sweeps
     phases.rs:655-661  FirstAttackerPerformance -> SecondAttackerPerformance,
                        SecondAttackerPerformance -> LiveVictoryDetermination
   Rust selects the performer by player ID, where is_first means "the FIRST
   attacker's window"; the C model stores first_attacker/second_attacker as seat
   indices, so the same selection is `is_first ? g->first_attacker :
   g->second_attacker`. */
void rb_execute_performance_phase(GameState *g, int is_first) {
    if (!g) return;
    int performer = is_first ? g->first_attacker : g->second_attacker;
    if (performer < 0 || performer > 1) performer = g->active;

    /* phases.rs:240/352 — constants must be freshly registered before the
       window opens, or heart modifiers go stale (q127_wien). */
    rb_recalc_constants(g);
    rb_check_timing(g);

    /* phases.rs:346 — the live card zone is turned face up; the C RbBag has no
       face-up flag, the reveal is the yell inside rb_perform_live. */
    /* phases.rs:382-527 + 630-641 */
    g->active = performer;
    rb_perform_live(g, performer);
    if (g->performance_resume_pending || rb_has_pending_choice(g)) {
        /* Paused on a choice. rb_perform_live re-enters through
           performance_resume_* on the next pass, so this same window, the phase
           and the active seat all stay put — the Rust function simply has not
           reached its tail yet. */
        return;
    }

    /* phases.rs:643-648 — the performer's own auto abilities first, then the
       opponent's (「相手のプレイヤーがライブを行ったとき」). */
    rb_trigger_auto_abilities(g, performer, "自動");
    rb_process_pending_auto_abilities(g);
    int opponent = performer ^ 1;
    rb_trigger_auto_abilities(g, opponent, "自動");
    rb_process_pending_auto_abilities(g);

    /* phases.rs:649-654 — a draw effect revealed by the yell can arm further
       abilities, so both sweeps run a second time. */
    if (ptc_yell_had_draw_icon(g)) {
        rb_trigger_auto_abilities(g, performer, "自動");
        rb_process_pending_auto_abilities(g);
        rb_trigger_auto_abilities(g, opponent, "自動");
        rb_process_pending_auto_abilities(g);
    }

    /* phases.rs:655-661 */
    if (is_first) {
        g->active = g->second_attacker;
        g->phase = RB_PHASE_PERFORMANCE;
    } else {
        g->active = g->first_attacker;
        g->live_batch_mode = 0;
        rb_execute_live_victory_determination(g);
        g->live_victory_pending = rb_has_pending_choice(g);
        fprintf(stderr, "[LIVE_VICTORY_PHASE] initial pending=%d\n", g->live_victory_pending);
        g->phase = RB_PHASE_VICTORY;
    }
}

/* ── play_time_cost_reduction_hook (phases.rs:1317-1433) ───────────────────────────
   Two mutually exclusive shapes, detected off the card's 常時 modify_cost
   abilities:
     1. a plain reduction -> set the cost to (base - reduction) and shuffle the
        waitroom members to the deck bottom;
     2. a generic alt cost -> discard one named member per slot from hand and
        set the cost to the ability's value.
   First entry arms the optional choice; re-entry applies the stored answer. The
   C model keeps the pending play in the ptc_* fields (see the GameState comment:
   rb_play_member pauses there and rb_complete_play_with_cost finishes it), so
   this function is the detection + arming half and `ptc_set` is the answer
   (-1 = declined, >= 0 = accepted alt-cost value). */
int rb_play_time_cost_reduction_hook(GameState *g, int card_id) {
    if (!g || card_id < 0) return 0;

    /* phases.rs:1327-1383 — re-entry: the choice was offered and answered. */
    if (g->ptc_active && g->ptc_card == card_id) {
        g->ptc_active = 0;
        if (g->ptc_set >= 0) {
            /* phases.rs:1356-1378 — alternative cost accepted. */
            if (!rb_discard_play_time_alt_cost(g, g->active, card_id)) {
                /* phases.rs:1361-1364 */
                fprintf(stderr, "[PTC] alt-cost hand cards missing card=%d\n", card_id);
                return 0;
            }
            rb_mods_set_cost(&g->mods, card_id, g->ptc_set);
        } else if (g->ptc_base >= 0) {
            /* phases.rs:1332-1355 — plain reduction accepted. */
            int reduction = rb_play_time_cost_reduction_amount(g, card_id);
            if (reduction != RB_PTC_NONE) {
                rb_mods_set_cost(&g->mods, card_id, g->ptc_base - reduction);
                rb_shuffle_waitroom_members_to_deck_bottom(g, g->active);
            }
        }
        return 1;
    }

    Card c;
    int base = 0;
    if (rb_card_get_card_by_id(card_id, &c)) { base = c.cost; rb_free_card(&c); }

    /* phases.rs:1385-1401 — first entry: arm the optional reduction. */
    if (rb_play_time_cost_reduction_amount(g, card_id) != RB_PTC_NONE) {
        g->ptc_active = 1;
        g->ptc_card = card_id;
        g->ptc_set = -1;
        g->ptc_base = base;
        return 1;
    }

    /* phases.rs:1402-1430 — first entry: arm the alternative cost, but only
       when the hand can actually pay it. */
    if (rb_has_play_time_alt_cost_hand_cards(g, card_id)) {
        char chars[RB_PTC_NAME_MAX * RB_PTC_MAX_CHARS];
        int set_value = rb_play_time_alt_cost_chars(g, card_id, chars, (int)sizeof(chars));
        if (set_value != RB_PTC_NONE) {
            g->ptc_active = 1;
            g->ptc_card = card_id;
            g->ptc_set = set_value;
            g->ptc_base = base;
            return 1;
        }
    }
    return 0;
}

/* -- rps_choice_name -- */
const char *rb_rps_choice_name(int choice) {
    switch (choice) {
        case 0: return "グー";
        case 1: return "パー";
        case 2: return "チョキ";
        default: return "?";
    }
}

/* -- push_rps_log -- */
void rb_push_rps_log(GameState *g, int p1, int p2, const char *winner_str) {
    if (!g) return;
    fprintf(stderr, "[RPS] p1=%s p2=%s winner=%s\n",
            rb_rps_choice_name(p1), rb_rps_choice_name(p2),
            winner_str ? winner_str : "none");
}

static RbGeneratedArea rb_action_areas[RB_MAX_HAND * RB_STAGE_SIZE][RB_STAGE_SIZE];

static RbGeneratedAction *rb_actions_push(RbGeneratedAction *actions, int *count, int *capacity,
                                 RbGeneratedAction action) {
    if (*count >= *capacity) {
        int next_capacity = *capacity > 0 ? *capacity * 2 : 16;
        RbGeneratedAction *next = (RbGeneratedAction *)rb_malloc((size_t)next_capacity * sizeof(*next));
        if (!next) return NULL;
        if (*count > 0) memcpy(next, actions, (size_t)(*count) * sizeof(*next));
        rb_free(actions);
        actions = next;
        *capacity = next_capacity;
    }
    actions[(*count)++] = action;
    return actions;
}

RbGeneratedActionList rb_generate_action_candidates(const GameState *state) {
    RbGeneratedActionList result = { NULL, 0 };
    if (!state || state->phase != RB_PHASE_MAIN) return result;

    int capacity = 1 + state->p[state->active].hand.n * RB_STAGE_SIZE;
    RbGeneratedAction *actions = (RbGeneratedAction *)rb_malloc((size_t)capacity * sizeof(*actions));
    if (!actions) return result;

    RbGeneratedAction pass;
    memset(&pass, 0, sizeof(pass));
    pass.action_type = 1;
    actions[result.count++] = pass;
    if (rb_is_action_prohibited(state, "play_member")) {
        result.actions = actions;
        return result;
    }

    int actor = state->active;
    const RbPlayer *player = &state->p[actor];
    int area_slot = 0;
    for (int hand_index = 0; hand_index < player->hand.n; hand_index++) {
        int card_id = player->hand.cards[hand_index];
        if (!rb_card_is_member(card_id)) continue;

        Card card;
        if (!rb_decode_card_by_index((uint32_t)card_id, &card)) continue;
        int cost_modifier = rb_mods_get_cost((RbMods *)&state->mods, card_id);
        int reduction = rb_calculate_play_cost_reduction(state, actor,
                                                        player->hand.n, card_id);
        int effective_cost = card.cost + cost_modifier - reduction;
        if (effective_cost < 0) effective_cost = 0;
        int available_count = 0;

        for (int area = 0; area < RB_STAGE_SIZE; area++) {
            RbGeneratedArea *info = &rb_action_areas[area_slot][area];
            info->available = 0;
            info->is_baton_touch = 0;
            int existing_id = player->stage[area];
            if (existing_id == RB_EMPTY_SLOT) {
                if (player->energy_active >= effective_cost) {
                    info->available = 1;
                    available_count++;
                }
            } else if (!rb_card_arrived_this_turn(state, actor, existing_id) &&
                       !rb_card_has_restriction(state, card_id, existing_id,
                                                "cannot_baton_touch")) {
                Card existing;
                if (rb_decode_card_by_index((uint32_t)existing_id, &existing)) {
                    int existing_modifier = rb_mods_get_cost((RbMods *)&state->mods,
                                                             existing_id);
                    int existing_cost = existing.cost + existing_modifier;
                    int baton_cost = effective_cost - existing_cost;
                    if (baton_cost < 0) baton_cost = 0;
                    if (player->energy_active >= baton_cost) {
                        info->available = 1;
                        info->is_baton_touch = 1;
                        available_count++;
                    }
                    rb_free_card(&existing);
                }
            }
        }

        if (available_count > 0) {
            for (int area = 0; area < RB_STAGE_SIZE; area++) {
                if (!rb_action_areas[area_slot][area].available) continue;
                RbGeneratedAction action;
                memset(&action, 0, sizeof(action));
                action.action_type = 14;
                action.has_parameters = 1;
                action.parameters.card_id = card_id;
                action.parameters.available_areas = rb_action_areas[area_slot];
                action.parameters.n_available_areas = RB_STAGE_SIZE;
                RbGeneratedAction *next = rb_actions_push(actions, &result.count,
                                                &capacity, action);
                if (!next) {
                    rb_free(actions);
                    result.actions = NULL;
                    result.count = 0;
                    rb_free_card(&card);
                    return result;
                }
                actions = next;
            }
        }
        area_slot++;
        rb_free_card(&card);
    }

    result.actions = actions;
    return result;
}

typedef struct {
    int available;
    int is_baton_touch;
} RbBatonAvailableArea;

typedef struct {
    int card_id;
    const RbBatonAvailableArea *available_areas;
    int n_available_areas;
} RbBatonActionParameters;

typedef struct {
    int action_type;
    RbBatonActionParameters parameters;
    int has_parameters;
} RbBatonAction;

typedef struct {
    RbBatonAction *actions;
    int count;
} RbBatonActionList;

RbBatonActionList rb_generate_possible_actions(const GameState *state) {
    RbGeneratedActionList generated = rb_generate_action_candidates(state);
    RbBatonActionList result;
    result.actions = (RbBatonAction *)generated.actions;
    result.count = generated.count;
    return result;
}
