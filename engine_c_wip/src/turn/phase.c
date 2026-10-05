#include "rabuka.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/* Defined below (mirrors phases.rs:318 execute_performance_phase). Declared here
   because rb_advance_phase dispatches the two performance windows into it. */
void rb_execute_performance_phase(GameState *g, int is_first);

/* ── E4's PRODUCER SIDE, capture half. Rust phases.rs:320-333 ────────────────
 *
 *     let nhm_flat: Vec<(i16, HeartColor, ModifierEntry)> = game_state
 *         .mods
 *         .need_heart_modifiers
 *         .iter()
 *         .flat_map(|(&card_id, colors)| {
 *             colors.iter().map(move |(&color, entry)| (card_id, color, *entry))
 *         })
 *         .collect();
 *
 * A WHOLESALE FLATTENING of the whole need_heart map, not a delta and not a
 * per-card list: every (card_id, colour, entry) triple that exists is captured,
 * because rb_restore_performance_need_heart_modifiers (modifiers.rs:2159-2182)
 * re-adds them from the snapshots after E3's clear(). Only what survives into a
 * snapshot can come back, which is precisely the "preserve live_start and other
 * non-constant sources" contract in E4's doc comment.
 *
 * WHY A MODULE-SCOPE BUFFER AND NOT A GameState FIELD: sizeof(GameState) is
 * pinned at 1 090 264 B and tests/test_harness_self.c §A4 pins it; appending a
 * field turns that red by design. RbLiveSnapshot.performance_need_heart_modifiers
 * / n_performance_need_heart_modifiers (rabuka.h:1375-1376) already exist for the
 * store, so the capture is the only piece with nowhere else to live.
 *
 * WHY IT IS A BUFFER RATHER THAN A DIRECT READ AT THE PUSH: Rust reads the map
 * at the head of the window, BEFORE rb_recalc_constants (phases.rs:352) and
 * before the yell and its 自動 sweep. Reading it at the push instead would
 * capture post-yell state. The buffer keeps the Rust position.
 *
 * THE ONE PLACE THIS DIVERGES, STATED PLAINLY: when a choice pauses
 * rb_perform_live, this C port returns from rb_execute_performance_phase and the
 * phase machine re-enters it from the top (phase.c:335), so the capture is taken
 * again on the resume pass. Rust's single blocking call keeps the original
 * nhm_flat. That is inherent to the C's resumable phase machine and already
 * applies to the whole function, rb_recalc_constants (:1662) included -- it is not
 * introduced by this capture. */
static RbPerfNeedHeartMod perf_nhm[RB_MAX_PERF_NEED_HEART_MODS];
static int perf_nhm_n = 0;

int rb_perf_nhm_count(void) { return perf_nhm_n; }
const RbPerfNeedHeartMod *rb_perf_nhm_at(int i) { return &perf_nhm[i]; }

static void rb_capture_performance_need_heart(GameState *g) {
    perf_nhm_n = 0;
    if (!g) return;
    for (int cid = 0; cid < RB_MAX_CARD_IDS; cid++) {
        for (int c = 0; c < 8; c++) {
            const RbModifierEntry *e = &g->mods.need_heart[cid][c];
            /* Rust iterates the MAP, so it sees exactly the entries that were
             * created. The C's need_heart[cid][color] is a dense array and cannot
             * distinguish "absent" from "present and zero" -- the same
             * representational gap src/core/modifiers.c:1990 states for the
             * success_zone_* ledgers ("In C, 'present in the map' is 'the entry
             * is non-zero'"). A zero-valued Rust entry contributes nothing to the
             * restore anyway: `target.additive += 0` and the `set` arm is guarded
             * by `entry.set != 0`, so skipping it is exact. */
            if (e->add == 0 && e->set == 0) continue;
            if (perf_nhm_n >= RB_MAX_PERF_NEED_HEART_MODS) return;
            RbPerfNeedHeartMod *d = &perf_nhm[perf_nhm_n++];
            d->card_id = (int16_t)cid;
            d->color   = (int8_t)c;
            d->entry    = *e;
        }
    }
}

/* phases.rs:333 / 339 handle_rps_choice_p1 / _p2 — defined further down this
   file; rb_advance_phase's pre-game arm calls them. */
int rb_handle_rps_choice_p1(GameState *g, int choice);
int rb_handle_rps_choice_p2(GameState *g, int choice);

/* src/game/game_setup.c — the faithful ports of the pre-game ceremony:
     rb_setup_choose_attacker        turn/actions/mod.rs:230 handle_choose_attacker
     rb_setup_handle_mulligan_skip   turn/phases.rs:801  handle_mulligan_skip
   rb_advance_phase's headless auto-pass routes into these rather than
   re-implementing them, so there is exactly one implementation of each.
   CROSS-FILE REQUEST: neither is declared in include/rabuka.h, so the externs
   are spelled out here (game_setup.c is not part of this file's ownership). */
int rb_setup_choose_attacker(GameState *g, int p1_first);
int rb_setup_handle_mulligan_skip(GameState *g);

/* ── CROSS-FILE REQUESTS for include/rabuka.h (that file is not this file's) ──
 *
 * rb_clear_play_scoped_baton_touch — src/core/tracking.c:80, the faithful twin of
 *   Rust's GameState::clear_play_scoped_baton_touch
 *   (engine/src/core/game_state/modifiers.rs:1769-1774), which phases.rs:1144
 *   makes the FIRST statement of every handle_play_member_to_stage. It had zero
 *   callers AND no prototype in any header. Called from
 *   rb_handle_play_member_to_stage below.
 *
 * rb_play_member_ex / rb_play_double_baton_ex — src/engine.c, the ports of
 *   Player::move_card_from_hand_to_stage (core/player.rs:344, reached from
 *   phases.rs:1260-1271) and TurnPhases::play_double_baton (phases.rs:936,
 *   reached from phases.rs:1227-1231). Both take the request parameters
 *   handle_play_member_to_stage's Rust signature carries and rb_play_member's
 *   does not. rb_play_member (the public 4-arg entry) is unchanged and forwards
 *   here with use_baton_touch = 0. */
void rb_clear_play_scoped_baton_touch(GameState *g);
int  rb_play_member_ex(GameState *g, int pl, int hand_idx, int stage_pos,
                       int use_baton_touch);
int  rb_play_double_baton_ex(GameState *g, int pl, int hand_idx, int a1, int a2,
                             int place);

/* src/game/game_setup.c — the Main-phase ability OFFER scan, the C twin of
   game_setup.rs:2078-2348 (the stage scan and the 控え室 scan). It is a
   faithful, complete port with no callers of its own, so rb_generate_action_
   candidates below could never offer an activated ability: an activation was
   resolvable (rb_activate_card) but unreachable through the action list, which
   is why several suites report an ability "is never offered".
   rb_setup_for_each_ability_candidate is the bridge that hands the offer
   table over; the gate rules all live on the game_setup.c side. */
int rb_setup_for_each_ability_candidate(
    const GameState *g,
    int (*sink)(void *ctx, int card_id, int ability_index, int from_stage,
                int stage_slot, int base_cost, int effective_cost),
    void *ctx);

/* ───────────────────────── XREQ: the three dead setup builders ─────────────
 * game_setup.c ports the rules content of Rust's three remaining row builders
 * faithfully and completely. All three had ZERO callers because this file
 * re-implemented the play-member half inline and refused every phase except
 * RB_PHASE_MAIN, so nothing in the C tree could ever ask them a question:
 *
 *   rb_setup_mulligan_candidates    game_setup.rs:1467 generate_mulligan_actions
 *   rb_setup_play_candidates        game_setup.rs:1690 generate_main_phase_actions
 *   rb_setup_live_card_candidates   game_setup.rs:2353 generate_live_card_set_actions
 *
 * They are called from ONE place in Rust — game_setup.rs:647 / :651 / :654, all
 * inside generate_possible_actions (:634) — so they are called from one place
 * here too: rb_generate_action_candidates below, which is that function's C twin.
 *
 * Unlike rb_setup_for_each_ability_candidate, these three take a caller-supplied
 * ROW ARRAY rather than a sink.
 *
 * The four row types used to be MIRRORED here, field for field, because they were
 * file-private in game_setup.c and include/rabuka.h was not this file's to edit.
 * The header owner has since moved RbSetupAreaCandidate,
 * RbSetupDoubleBatonOption, RbSetupPlayCandidate, RbSetupHandCandidate and
 * RB_SETUP_MAX_DOUBLE_BATON into include/rabuka.h, so the mirror is DELETED — the
 * loud duplicate-typedef failure it was designed to produce is the one that fired,
 * and it is the intended outcome. The prototypes below are still needed because
 * the header does not declare these three yet; they move here unchanged. */
int rb_setup_mulligan_candidates(const GameState *g, RbSetupHandCandidate *out, int max);
int rb_setup_play_candidates(const GameState *g, RbSetupPlayCandidate *out, int max);
int rb_setup_live_card_candidates(const GameState *g, RbSetupHandCandidate *out, int max);

/* Turn phase machine — mirrors engine/src/turn/phases.rs:advance_phase
   Two TurnPhases per round: FirstAttackerNormal / SecondAttackerNormal / Live.
   For portability we keep it linear: RPS → Active → Energy → Draw → Main
   executed twice (first then second attacker) before LiveSet → Performance
   → Victory → rollover. Hosts that don't need mulligan can skip it. */

/* Defined at phase.c:735; activate_wait_members below needs it first. */
static const char *ptc_effect_extra(const AbilityEffect *e, const char *key);

/* DEFECT FIX — the missing `constant_cannot_activate_members` filter.
 *
 * Rust actions/mod.rs:125-131, the Active-phase `to_activate` filter in
 * phases.rs:
 *     if game_state.constant_cannot_activate_members.iter()
 *         .any(|x| x.parse::<i16>() == Ok(cid)) { return None; }
 * That set is built by the CONSTANT scan in
 * core/game_state/modifiers.rs:1081-1092: for every CONSTANT ability effect with
 * restriction_type "cannot_activate",
 *     let tgt = effect.target_any().unwrap_or("self");
 *     if rt == "cannot_activate" {
 *         if tgt == "self" { self.constant_cannot_activate_members
 *                                 .push(card_id.to_string()); }
 *         else { self.constant_cannot_activate_members
 *                     .push(resolve_target_player(tgt).id.to_string()); }
 *     }
 * and it is CLEARED at the top of every scan (modifiers.rs:671).
 *
 * The C had neither the GameState field nor the filter, so a member under a
 * 「このメンバーは自分のアクティブフェイズにアクティブにしない」 constant
 * restriction was flipped to "active" by activate_wait_members on its
 * controller's Active phase — the exact card text says it must not activate.
 *
 * Only the SELF-target half is observable: a non-self target pushes a PLAYER id
 * ("player1"/"player2"), and `"player1".parse::<i16>()` fails, so the
 * `== Ok(cid)` comparison at phases.rs:126 never matches it. That half is a
 * no-op in Rust too and is not reimplemented.
 *
 * The predicate is recomputed locally here instead of stored, because the set is
 * filled ONLY by recalculate_constants (modifiers.c, not this file's) and
 * include/rabuka.h — which has no such field — is not this file's to edit. The
 * recomputation is exact: same trigger kind (RB_TK_CONSTANT), same action name,
 * same restriction_type, same target default. The effect's own `condition`
 * (both corpus abilities carry a temporal_condition on phase=active_phase) is
 * deliberately NOT evaluated, because modifiers.rs:1081-1092 never evaluates it
 * either. */
static int has_constant_cannot_activate(int card_id) {
    if (card_id < 0) return 0;
    int n = rb_card_num_abilities((uint32_t)card_id);
    for (int i = 0; i < n; i++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)card_id, i, &ab)) continue;
        int keep = 0;
        if (rb_ability_has_trigger(&ab, RB_TK_CONSTANT) && ab.effect &&
            ab.effect->action && !strcmp(ab.effect->action, "restriction")) {
            const char *rt = ptc_effect_extra(ab.effect, "restriction_type");
            if (!rt) rt = ptc_effect_extra(ab.effect, "type");
            const char *tg = ab.effect->target;
            if (!tg) tg = ptc_effect_extra(ab.effect, "target");
            if (rt && !strcmp(rt, "cannot_activate") &&
                (!tg || !strcmp(tg, "self")))
                keep = 1;
        }
        if (keep) { rb_free_ability(&ab); return 1; }
        rb_free_ability(&ab);
    }
    return 0;
}

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
        /* phases.rs:125-131 — a card the constant scan put in
           constant_cannot_activate_members keeps its wait orientation. See
           has_constant_cannot_activate above for why this is computed here. */
        if(has_constant_cannot_activate(P->stage[q])) continue;
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
    /* ── Pre-game ceremony ────────────────────────────────────────────────────
       Rust's TurnEngine::advance_phase has NO arm for RockPaperScissors,
       ChooseFirstAttacker or either Mulligan phase: those are HOST-driven
       (game_setup.rs:359-364 setup_game parks on RockPaperScissors, then the
       host drives RPS -> ChooseFirstAttacker -> Mulligan -> Active through
       execute_main_phase_action). The C rb_advance_phase is also the headless
       harness's auto-pass, so it has to be able to walk the ceremony itself or
       a headless game deadlocks on a phase nothing can move.

       The split is: if the host has already answered, its answer wins and this
       arm is not reached at all (the host routes RPS through
       rb_setup_execute_action, whose RPS resolve has already parked the phase
       on RB_PHASE_OPENING). Only when NO RPS choice has been recorded does this
       arm stand in for the two missing players. */
    if(g->phase==RB_PHASE_RPS){
        if(g->player1_rps_choice<0 && g->player2_rps_choice<0){
            /* HEADLESS STAND-IN for the host's two Rock/Paper/Scissors actions.
               Rust never guesses these; the C has no host on this path. Same
               seeded draw as the retired rb_game_init code, so replay fixtures
               that seed the RNG still produce the same seat order. */
            int a=(int)rb_rand_range(3), b;
            do { b=(int)rb_rand_range(3); } while(b==a);
            rb_handle_rps_choice_p1(g, a);
            rb_handle_rps_choice_p2(g, b);
        }
        /* resolve_rps_if_both_chosen parks on RB_PHASE_OPENING; a tie clears
           both choices and stays here for another pass. */
        if(g->phase==RB_PHASE_RPS) return;
    }
    /* phases.rs:229-238 ChooseFirstAttacker — set the seats, draw 6 each, enter
       MulliganFirstAttacker. Faithful port lives in src/game/game_setup.c
       (rb_setup_choose_attacker, turn/actions/mod.rs:230 handle_choose_attacker);
       this is the headless auto-pass routing into it. */
    if(g->phase==RB_PHASE_OPENING){
        rb_setup_choose_attacker(g, g->rps_winner != 2);
        return;
    }
    /* phases.rs:698-711 advance_mulligan_phase. Headless auto-pass takes the
       SKIP path (phases.rs:801 handle_mulligan_skip): no replacement, just clear
       the selection and move on. A host that wants a replacement drives
       SelectMulligan/ConfirmMulligan through rb_setup_execute_action instead. */
    if(g->phase==RB_PHASE_MULLIGAN_FIRST || g->phase==RB_PHASE_MULLIGAN_SECOND){
        rb_setup_handle_mulligan_skip(g);
        return;
    }
if(g->phase==RB_PHASE_ACTIVE){
        /* ── THE MISSING ACTIVE-ROLLOVER RESET ──────────────────────────────
           Rust engine/src/turn/phases.rs:98, the FIRST statement of the
           `Phase::Active` arm of `advance_phase`:

               game_state.reset_keyword_tracking();
               tdbg!("PHASE_ACTIVE:1 reset_keyword_tracking OK");

           Its body is engine/src/core/game_state/tracking.rs:6-36. The C's
           fold of it is src/core/tracking.c:38 `rb_reset_keyword_tracking`,
           which until now had ZERO production callers -- the exact state
           src/core/tracking.c:27-30 admitted in its own comment ("The C turn
           rollover in src/turn/phase.c has no equivalent call"). That comment
           is now WRONG and this is the call it was asking for.

           WHY IT IS HERE AND NOT IN THE VICTORY ROLLOVER BELOW: Rust runs it
           on ENTRY TO EACH PLAYER'S ACTIVE PHASE, i.e. TWICE PER ROUND, and
           this arm is the C's entry to the Active phase -- the `Phase::Active`
           arm of `rb_advance_phase`, the same frame the C reaches at :325 when
           it flips to the second attacker. The C also reaches it from :677
           (the victory arm's tail), from the interactive loop (src/main.c:97/:102
           call rb_advance_phase directly and never rb_turn), from the batch
           loop (src/main.c:134 rb_turn, now a thin driver over rb_advance_phase)
           and from the WEB path (src/web_server.c -> engine.c:1550
           rb_advance_phase), so the placement covers every production driver.
           Placing it in the victory rollover instead would clear these trackers
           ONCE PER ROUND, which is a different, weaker scope than Rust's.

           It goes FIRST, before activate_wait_members, because Rust's reset is
           at phases.rs:98 and the wait-activation filter that follows is at
           phases.rs:109-138 -- the same order, and order matters: the reset
           clears `position_change_occurred_this_turn` (via
           rb_reset_change_flags) and the activation filter reads
           constant_cannot_activate_members / is_delayed_cannot_active.

           WHAT IT CLEARS, AND WHY THE BLAST RADIUS IS 40 OF 985 ABILITIES --
           six keyword-scoped trackers that previously latched for the whole
           game because nothing reset them:
             p[].deck_refreshed_this_turn          -> condition.c:542 and :2155
                                                       read it; 7 production
                                                       producers, 0 resets
             player1/2_cheer_blade_heart_count      -> condition.c:2566
             live_success[0..1]                     -> condition.c:2676/:2681,
                                                       triggers.c:462
             n_turn1/2_abilities_played             -> no producer anywhere
             n_auto_ability_trigger_counts          -> no producer anywhere
             n_areas_placed_this_turn               -> no producer anywhere
             last_resolution_cards_p1/p2            -> no producer anywhere
           plus, through rb_reset_change_flags / rb_reset_loop_detection /
           rb_clear_baton_touch_tracking, the change-of-state flags, the
           loop-detection history and the baton-touch history.

           The sharpest single divergence it removes, with its card number:
             PL!S-bp2-022-L -- 「{{ライブ成功時}}このターン、自分のデッキが
             リフレッシュしていた場合、このカードのスコアを＋２する。」
           `deck_refreshed_this_turn` had 7 producers and no reset, so that card
           paid its +2 from the FIRST deck refresh of the game onward, on every
           turn where nothing was refreshed. The string 「このターン」 scopes 40 of
           the 985 unique corpus abilities (69 of 2116 pairs, 65 of 2639 cards).
           See engine_c_wip/TEST_ONLY_AUDIT.md §4/O1 and PARITY_CHECKLIST.md.

           NOT DONE HERE, ON PURPOSE, AND IT IS NOT AN OVERSIGHT:
             rb_reset_keyword_tracking does NOT clear debut counts or the
             card-appearance / movement trackers (tracking.rs:9-17 says so, and
             src/core/tracking.c:32-37 explains why: it runs twice per round
             while 「このターン」 spans the whole round, so clearing them here
             truncated first-attacker-window facts before live-start /
             live-success conditions could read them -- PL!N-pb1-037-L Q203,
             PL!N-bp3-005-R＋, PL!HS-bp2-021-L). They are cleared together in
             the victory rollover below.
             It also clears `g->game_state_history` (loop detection). That is
             Rust's line too (tracking.rs:27), and `loop_detected` has NO
             production reader anywhere in src/ -- it is a flag nothing acts on
             -- so weakening the window across an Active boundary cannot hang a
             batch run. */
        rb_reset_keyword_tracking(g);
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
    /* phases.rs:224-225 - the FIRST live-set window hands over to the second,
       which is a DISTINCT value in Rust's Phase enum
       (LiveCardSetSecondAttacker). It is stored here so g->phase holds what
       Rust's Phase holds; from this point the window is identified by the
       phase value, not by which seat happens to be active. */
    if(g->phase==RB_PHASE_LIVE_SET || g->phase==RB_PHASE_LIVE_SET_SECOND){
        RbPlayer *P = &g->p[g->active];
        int placed = P->live.n;
        int hand_before_refill = P->hand.n;
        for (int i = 0; i < placed; i++) rb_draw(g, g->active);
        fprintf(stderr, "[LIVE_ZONE_REFILL] pl=%d placed=%d hand_before=%d hand_after=%d\n",
                g->active, placed, hand_before_refill, P->hand.n);
if (g->phase == RB_PHASE_LIVE_SET) {
            g->active = g->second_attacker;
            /* phases.rs:225 — the window is done; the SECOND window is its
               own Phase and is now STORED, not implied by `active`. */
            g->phase = RB_PHASE_LIVE_SET_SECOND;
            return;
        }
g->active = g->first_attacker;
        /* DEFECT FIX - phases.rs:232-233, the FIRST two statements of the
           LiveCardSetSecondAttacker arm, before its check_timing at :240:
             game_state.player1.live_card_set_limit_reduction = 0;
             game_state.player2.live_card_set_limit_reduction = 0;
           execute_reduce_live_card_set_limit (ability/effects/restriction.rs
           :217-234) does `player.live_card_set_limit_reduction += count`, and the
           two corpus abilities that reach it say "the NEXT live-card-set phase
           can set one fewer card". The C incremented
           g->live_set_limit_reduction at both writers (ability/effects/rules.c
           :1952 and ability/effects/state.c:1296) and NEVER cleared it, so once
           either card fired the player's live-card-set limit was permanently
           shrunk for the rest of the game. Rust honours "next"; the C honoured
           "forever". */
        g->live_set_limit_reduction[0] = 0;
        g->live_set_limit_reduction[1] = 0;
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
    if(g->phase==RB_PHASE_PERFORMANCE || g->phase==RB_PHASE_PERFORMANCE_SECOND){
        /* Two windows per live: the first attacker performs, then the second,
           then LiveVictoryDetermination. The window body — performer selection,
           the post-snapshot auto-ability sweeps and the phase transition — is
           rb_execute_performance_phase (mirrors phases.rs:256-265 dispatching
           FirstAttackerPerformance / SecondAttackerPerformance into
           execute_performance_phase(is_first)). live_batch_mode keeps the two
           windows from settling the round between them (mirrors the Rust pair
           of phases being separate TurnPhase values). */
        /* Which window this is is read off the STORED phase, not off `active`
           (types.rs:137 / phases.rs:256-265 name four distinct live values:
           LiveCardSetFirstAttacker / LiveCardSetSecondAttacker /
           FirstAttackerPerformance / SecondAttackerPerformance). */
        g->live_batch_mode = 1;
        rb_execute_performance_phase(g, g->phase == RB_PHASE_PERFORMANCE);
        return;
    }
    if(g->phase==RB_PHASE_VICTORY){
        if(g->live_victory_pending){
            rb_execute_live_victory_determination(g);
            fprintf(stderr, "[LIVE_VICTORY_PHASE] resume pending=%d\n", rb_has_pending_choice(g));
            if(rb_has_pending_choice(g)) return;
            g->live_victory_pending = 0;
        }
        /* victory check + rollover.
           The result fields are a pure re-read of what
           rb_execute_live_victory_determination already recorded
           (live.c:1603-1605), so they are not re-decided here.

           The first/second-attacker seats are deliberately NOT elected in this
           branch. Rust has exactly two writers of `is_first_attacker`:
           actions/mod.rs:230-231 (RPS) and live.rs:1283-1289, the latter inside
           move_to_success_and_update_attacker — which runs from
           rb_execute_live_victory_determination, i.e. BEFORE the VICTORY branch
           of advance_phase (phases.rs:265-311, the Rust counterpart of this
           block, writes no seat at all). That election keys off the SUCCESS-ZONE
           DELTA (p1_added / p2_added), not off who won the live.

           The election this branch used to do was winner-based and had no Rust
           counterpart: it flipped the seats whenever exactly one seat won, even
           when that seat placed nothing into the success zone (e.g. the
           p1_must_skip / p2_must_skip path in
           rb_move_live_to_success_and_handle_wins, where the winner sends its
           live card to the waitroom instead), and it declined to flip when both
           seats won even though only one of them actually placed. live.c owns the
           election; do not overwrite it here. */
        int p1_won=0, p2_won=0;
        rb_determine_live_winners(g, &p1_won, &p2_won);
        g->p1_live_won = p1_won; g->p2_live_won = p2_won;
        /* No seat election here — see the block comment above. An election keyed only on
           WHO won flipped `first_attacker` even when the winner placed nothing into the
           success zone, which Rule 8.4.13 forbids ("a win that placed nothing does NOT
           re-elect the winner"). live.c's rb_move_to_success_and_update_attacker performs
           the single, correct election and keys it off the placement itself. */

        for(int pl=0;pl<2;pl++){
            if(g->p[pl].success.n >= RB_VICTORY_CARD_COUNT) g->winner=pl;
            else if(g->p[pl].score >= RB_SCORE_WIN) g->winner=pl;
        }
        if(g->p[0].success.n>=RB_VICTORY_CARD_COUNT && g->p[1].success.n>=RB_VICTORY_CARD_COUNT) g->winner=2;
        if(g->winner!=-1){ g->phase=RB_PHASE_DONE; return; }
        g->turn++;
        /* Mirror engine/src/turn/phases.rs:309 — the last statement of the
           victory / turn-rollover arm of advance_phase:

               game_state.clear_card_movement_tracking();

           Rust's clear_card_movement_tracking (modifiers.rs:1920-1942) is what
           turns the whole 「このターン中に…」 family over, and its appearance
           half is exactly the two lists this block left alone:
           `self.cards_appeared_this_turn.clear();`  (:1923) and
           `self.card_appearance_source.clear();`    (:1937).
           rb_clear_card_appearance_tracking is the C fold of precisely that
           pair, so calling it here is the same clear at the same point.

           The call was MISSING, which left every turn-scoped appearance fact
           alive across the rollover: the hand-rolled memset below cleared
           moved_this_turn and the debut counters, but never
           cards_appeared_this_turn or card_appearance_source.

           That was invisible while rb_record_card_appearance had no callers;
           it became a real, silent, cross-turn leak the moment the play choke
           point started recording debuts. A 「このターン中に登場した場合」 gate
           stayed satisfiable from the turn the card debuted, forever, and —
           because rb_get_card_appearance_source takes the FIRST match — a
           控え室から登場 gate could never again be answered by a genuinely
           different later debut.

           NOTE: this rollover is DUPLICATED in the C. The other one lives in
           rb_end_turn (engine.c:1510-1528) and already made this exact call;
           the tests reach THIS one.

           FAITHFULNESS NOTE for the rabuka.h owner: Rust's single call here
           also drops cards_activated_this_turn, turn_movements and
           mods.opp_cause_fired_keys. The C's single fold of all four is
           rb_clear_card_movement_tracking (modifiers.c:548), which ALSO drops
           n_batch_movements — so calling THAT here would be the exact Rust
           line. It has no prototype in include/rabuka.h, hence the narrower
           call. See the field-addition request in the handover report. */
        rb_clear_card_appearance_tracking(g);
        /* The OTHER THREE fields Rust's single `clear_card_movement_tracking`
           call drops, so `phases.rs:309` is now fully ported at this site.
           `rb_clear_card_movement_tracking` (modifiers.c:932) does clear all
           three -- but it ALSO zeroes `n_batch_movements`, which Rust does not
           do here: `batch_movements` is per-ABILITY state that Rust turns over
           in `clear_effect_tracking` (mod.rs:978-984) and only there. Calling
           the broad function from the rollover would clear it a turn early, so
           the three field clears are written out instead. That is why this is
           not BLOCKED-ON `include/rabuka.h` after all: the prototype
           (`void rb_clear_card_movement_tracking(GameState *g);`, to sit beside
           `rb_clear_card_appearance_tracking` at rabuka.h:2842) is still worth
           landing, but nothing here waits for it.

             modifiers.rs:2027  cards_paid_cost_this_turn.clear()
                 「コストを支払う」 is per-turn; uncleared, a cost paid on turn N
                 keeps firing PL!S-bp3-021-N on turn N+1, forever.
             modifiers.rs:2038  turn_area_movements.clear()
             modifiers.rs:2041  mods.opp_cause_fired_keys.clear()
                 The opponent-cause watcher dedupe is turn-scoped by Rust's own
                 comment: "a given move can arm a marked watcher once, and the
                 set resets with the movement data". The set is still 0W/0R in
                 the C (the live dedupe is game_state_abilities.c's turn-pruned
                 `s_opp_cause_fired[]` side table), so these two lines are the
                 half-port being closed, not a new behaviour. */
        g->n_cards_paid_cost_this_turn = 0;
        g->n_turn_area_movements = 0;
        g->mods.n_opp_cause_fired_keys = 0;
        /* Clear per-turn temporal-condition tracking (mirrors GameState reset of
            moved_this_turn / debut_count_this_turn / position_change_occurred_this_turn). */
        for(int i=0;i<RB_MAX_CARD_IDS;i++) g->moved_this_turn[i]=0;
        g->debut_count_this_turn[0]=g->debut_count_this_turn[1]=0;
        g->position_change_occurred_this_turn=0;
        /* Clear stage_arrived tracking (mirrors rb_turn's reset of baton arrival-ban).
            Without this, baton touch remains blocked on the next turn because the
            existing member's arrival flag is still set. */
        for(int p=0;p<2;p++) for(int i=0;i<RB_STAGE_SIZE;i++) g->stage_arrived[p][i]=0;

        /* ── engine/src/turn/phases.rs:277-303 — the rest of the SAME rollover ──
           Rust's LiveVictoryDetermination arm clears eight pieces of turn state
           before it bumps the turn counter. The block above covers
           appearance tracking (:309), deployed_this_turn (:286-287) and
           debut_count_this_turn (:290-291); the lines below are the four that
           had NO C clear site on the tested path, so every fact they hold stayed
           satisfiable for the rest of the game.

cannot_activate_members.clear()      phases.rs:279
                C fold: GameState.player_cannot_activate[2] (the Rust list is
                per-seat, not per-card) plus GameState.cannot_active_cards /
                n_cannot_active_cards, which is the C fold of the per-MEMBER half
                of the same Rust set (written by effects/misc.c:913-920, read by
                rb_card_is_cannot_active). Both writers were already in place and
                NEITHER was ever cleared outside engine.c:1199-1201 — which only
                rb_turn reaches, i.e. not the interactive loop and not the web
                path, and not any test. That is why
                tests/test_parity_restriction.c hand-zeroed the field by hand.
                CORRECTION to engine.c:1464-1467, which claims that test "no
                longer has to zero the field by hand": it still does, at
                test_parity_restriction.c:465-466, :923-924 and :1006-1007. Those
                are per-sub-test isolation (each starts from a fresh
                test_game_new board and drives a phase-gate probe), not the
                rollover they are usually read as — the lines the claim points at
                (:947-948) are a `board_begin` call and a blank line. tests/ is
                not this task's to edit, and no test is weakened either way.

             turn_state_changes.clear()          phases.rs:303
               C fold: turn_state_changes / n_turn_state_changes. It is a 64-row
               ring appended by effects/state.c:447/648/1870 with NO reader-side
               rollover clear anywhere in the tree, so once 64 state changes had
               happened the buffer was permanently full and every later
               「このターン…状態が変化した」 style condition stopped being able to
               record anything at all.

             clear_revealed_cards()              phases.rs:277
               NOT DONE, and it is not an oversight. The C folds Rust's pending
               reveal pool and the yell's own reveal record into ONE field
               (GameState.revealed_cards / n_revealed): the draw-icon sweep in
               rb_execute_performance_phase below reads it as "what the yell just
               revealed", and
               tests/test_parity_rules_phases.c:836 (B7) and its D2 twin assert
               `n_revealed > 0` as the PREMISE that the reveal happened — after
               perform_live has already run this rollover. Clearing it here is
               the Rust line and measurably costs exactly those two premise
               checks (parity_rules_phases 28 -> 30 fail, A/B'd), because the C
               cannot tell the two uses apart. The other five sites that clear
               n_revealed (live.c, choice.c, misc.c:942, rules.c:2047,
               modifiers.c:593) all clear it as the POOL, at the point the choice
               consumes it — which is the Rust behaviour for the pool. REPORTED
               as a field-separation request, not silently half-done.

baton_touch_used — C-LOCAL, no Rust clear site. Rust passes
                baton_touch_used PER PLAY ACTION as an argument
                (phases.rs:1076 -> trigger_scan.rs:73-83), so it is a stack value
                that cannot outlive the action; the C keeps it in GameState and
                sets it in engine.c:485 (single baton) and :622 (double baton).
                Nothing in the TREE ever cleared it again outside rb_turn, so from
                the first baton touch of the game onward it was latched at 1 for
                both seats, and src/turn/triggers.c:243
                `if (!baton_touch_used && baton_touch_trigger)` suppressed every
                    「バトンタッチされたとき」 ability for the rest of the match.
                The rollover is the widest frame the C phase machine has, and the
                one Rust's own turn machine is closest to, so that is where it is
                cleared. It is still coarser than Rust's per-action scope (a turn
                has two Main phases), but coarser is correct and never-latched is
                not; making it per-action needs a play-action frame, which is
                engine.c's and rb_play_member's, not this file's.
                VERIFIED CLEARED, and every caller of the phase machine reaches
                this arm: the interactive loop (src/main.c:97 / :102 call
                rb_advance_phase directly and never rb_turn), the batch loop
                (src/main.c:134 rb_turn, which is now a thin driver over
                rb_advance_phase), the WEB path
                (src/web_server.c:666 rb_execute_main_phase_action(.., 1, ..) ->
                engine.c:1550 rb_advance_phase, and rb_setup_settle_single_
                player_state at web_server.c:394/:567/:591), and every test.

            NOT CLEARED HERE — one of them is THIS FILE'S and one is not:
              cannot_activate_members.clear()      phases.rs:279 — DONE, the two
                clears below. See the fold note above; both writers
                (effects/misc.c:913-920, effects/rules.c:1869-1872) now have a
                rollover clear, so the field no longer outlives its turn.
                NOTE the fold is DUPLICATED in rules.c: `RbRulesState::
                cannot_activate_members[2]` is a second copy of the same Rust set,
                written at rules.c:1870 beside the GameState write, and its only
                reader `rb_rules_cannot_activate_player` (rules.c:2274) has ZERO
                callers tree-wide, so that copy is dead-but-set and CANNOT be
                cleared from here (`rb_rules_state` is static in rules.c). That
                half is BLOCKED-ON, together with cannot_live_players below.

cannot_live_players.clear()         phases.rs:280
                BLOCKED-ON src/ability/effects/rules.c (not this file's, and not
                under src/ability ownership this task holds). It lives in the
                file-static RbRulesState at rules.c:72; `rb_rules_state` is
                `static` there, so there is no accessor and no way to clear it
                from here, or from live.c.
                EXACT SYMBOL NEEDED —

                    void rb_rules_clear_turn_state(GameState *g);

                ⚠ THE DECLARATION IS DELIBERATELY ABSENT FROM BOTH FILES THAT
                USED TO CARRY IT. src/turn/phase.c and src/turn/live.c each
                spelled this prototype out inside a comment block, so the symbol
                had TWO declarations, NO definition anywhere in the tree, and NO
                caller — and checklist item I2 still claimed `[x]`. Any future
                call site would have been a LINK ERROR with no compiler warning
                along the way, because both declarations were inside comments
                and therefore only ever documentation. So neither comment
                declares it now: this tombstone is the single record, and the
                definition has to arrive with the prototype, in rules.c, in one
                edit. Do NOT re-add the prototype here before the body exists.

                clearing `cannot_live_players[2]` and `cannot_activate_members[2]`
                in the same frame (Rust clears them adjacently, phases.rs:279-280),
                called from the two clears below next to
                player_cannot_activate. `rb_rules_forget` (rules.c:140) is NOT a
                substitute: it is 0-callers too and drops EVERY side table for the
                state (delayed_prohibition, wait_immune, conditional_choice,
                moved_cards, …), none of which Rust clears at the rollover.
                Until it exists, live.c's `discard_live_on_cannot_live`
                (live.rs:1832) cannot be wired either — see PARITY_CHECKLIST.md
                I2, where the whole reader/writer/clear chain is written out.
                ⚠ `cannot_live_players[2]` (rules.c:72, written rules.c:1878) is
                STILL never cleared: it latches 1 on the first restriction that
                forbids a live ("ライブできない", the `cannot_live` arm of
                r_apply_restriction, rules.c:1876-1878) and no rollover ever
                zeroes it. That is the
                concrete reason this item is still open.

wait_immune_members.clear()         phases.rs:281
                NOW DONE — see the rb_rules_clear_turn_state call below. It was
                skipped because the list had no writer anywhere in the tree (the
                header said so at rabuka.h:3280-3289), so it was always empty and
                clearing it was a no-op. That stopped being true when the wait_immune
                producer was lifted into the live h_restriction (O3): the reader
                rb_rules_is_wait_immune carries no turn stamp, so without this
                clear the list would latch for the rest of the match.
               turn_limited_abilities_used.clear() phases.rs:278
                already self-clearing in C: the use-limit table is keyed by turn
                (ability_queue.c:1138/1147 `if (q->use_turn != cur_turn)`), which
                is the same guarantee Rust gets from the explicit clear. */
        g->player_cannot_activate[0] = 0;
        g->player_cannot_activate[1] = 0;
        g->n_cannot_active_cards = 0;
        /* Rust phases.rs:279-281 empties three adjacent per-turn sets; the two
           GameState folds above are the first of them, and all three live in
           rules.c's `static` RbRulesState, which is why this call is the only way
           to reach the other two from here. This is the symbol the tombstone
           above has been asking for since 2026-10-03 — it now has a definition
           (rules.c), so the two declarations that used to live inside comment
           blocks in this file and in live.c are correctly absent.
           Now declared in include/rabuka.h, beside rb_advance_phase. It used
           to be declared HERE, mid-function: legal C, but it left the symbol
           invisible to every other translation unit, and before the body
           existed in rules.c it was spelled only inside COMMENT blocks in this
           file and in live.c -- so checklist item I2 could read `[x]` for a
           symbol with no definition and no caller. The header also records what
           it clears against Rust turn/phases.rs:313-317. */
        rb_rules_clear_turn_state(g);
        /* ── Rust's SECOND clear of the same three sets, and the one piece of it
           that was genuinely missing. `clear_live_only_state`
           (engine/src/core/game_state/abilities.rs:3201-3209) is called from
           `check_expired_effects` at :3241, which `advance_phase` invokes at
           phases.rs:310 — i.e. 30 lines AFTER the explicit clears at :279-281 and
           AFTER `current_turn_phase` has already been moved off Live at :306-307,
           so its `if current_turn_phase == Live { return }` guard does not fire.
           Consequence for the fold: the second site is a strict NO-OP for
           `cannot_live_players` and `wait_immune_members` (already empty) and
           therefore `rb_rules_clear_turn_state` above covers BOTH Rust sites,
           not one of two. Its only effect Rust gets here and the C does not is
           `prohibition_effects.clear()`, and that set has NO clear anywhere in
           the C: the one writer (game_state_abilities.c:1418) appends, the three
           readers (game_state_abilities.c:594, core/tracking.c:197,
           turn/triggers.c:1414) only ever COMPACT it. So a zone
           prohibition recorded in turn 1 stayed answerable for the rest of the
           match. This line is the Rust :3206 statement. */
        g->n_prohibition_effects = 0;
        g->n_turn_state_changes = 0;
        g->baton_touch_used[0] = 0;
        g->baton_touch_used[1] = 0;
        /* n_prohibition / n_selected_cards — C-local with no Rust analogue
         * (engine.c's rb_turn table), so this is a scope decision, not a Rust
         * line. Both are PER-ACTION buffers that only ever grew:
         *
         *   n_prohibition      C-local prohibition notes. compound.c:766-773
         *                      appends two rows per compound effect; the
         *                      deduplicating writer (game_state_abilities.c:1314
         *                      RB_PUSH) and the two readers
         *                      (triggers.c:1231/1507, game_state_abilities.c:571)
         *                      only ever COMPACT them down, never zero them. The
         *                      64-row table therefore filled once and stayed full
         *                      for the rest of the match, so every later
         *                      prohibition note was dropped on the floor.
         *   n_selected_cards   C-store cross-effect selection buffer. Written by
         *                      choice.c:432/1699/1710/1791/1828/2129/2148, cost.c
         *                      :1292-1295 and this file's own
         *                      rb_handle_live_card_selection; cleared on consume
         *                      (choice.c:1569, compound.c:686) and by the live-set
         *                      handlers below. A live-set window that is ANSWERED
         *                      with an explicit index list rather than by toggling
         *                      leaves the buffer holding the previous window's
         *                      indices, which is then read as this window's
         *                      selection.
         *
         * The only clear was rb_turn's per-round prologue, and rb_turn is
         * reachable only from src/main.c:134's batch loop — NOT from the
         * interactive loop (src/main.c:29-105), NOT from web_server.c, and NOT
         * from any test. So on every path that matters these two counters
         * accumulated for the whole game. The rollover is the widest frame the C
         * phase machine has and the one Rust's own turn machine is closest to
         * (phases.rs:265-311 clears seven other pieces of turn state there), so
         * that is where they go; rb_turn's now-duplicate copies are gone
         * (engine.c, rb_turn). */
        g->n_prohibition = 0;
        g->n_selected_cards = 0;
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
        /* Rust Player::refresh (core/player.rs:634-648) shuffles the waitroom and
           pushes the WHOLE of it under the deck; main_deck.cards is a Vec, so it
           is unbounded. The C RbBag is capped at RB_MAX_ZONE, so the surplus has
           to STAY in the waitroom. The old body copied the entire waitroom into
           deck.cards[] with no cap expression at all and then cleared discard.n
           unconditionally — the "a card leaves one zone and arrives in none"
           shape zones.c:102-119 already fixed for its twin rb_zone_refresh.
           Still latent (the deck.n == 0 guard makes discard.n the only
           contribution, and every waitroom push is capped), but one guard
           removal away, so it is bounded here rather than relied upon.

           Shuffling the WAITROOM before the copy rather than the deck after it is
           byte-equivalent whenever everything crosses — deck.n == 0 here, so
           both orderings run the same number of draws over the same set. */
        rb_shuffle(P->discard.cards, P->discard.n);
        int moved = 0;
        while (moved < P->discard.n && P->deck.n < RB_MAX_ZONE)
            P->deck.cards[P->deck.n++] = P->discard.cards[moved++];
        if (moved < P->discard.n) {
            for (int i = moved; i < P->discard.n; i++)
                P->discard.cards[i - moved] = P->discard.cards[i];
            P->discard.n -= moved;
        } else {
            P->discard.n = 0;
        }
        P->deck_refreshed_this_turn = 1;
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
    /* THE FACE-UP GATE (was missing; Rust turn/actions/mod.rs:1732-1736).
         fn check_invalid_live_cards(game_state: &mut GameState, is_p1: bool) {
             let seat = if is_p1 { 0 } else { 1 };
             if !game_state.seat_player(seat).live_card_zone.face_up { return; }

       The eviction is only legal while that seat's live-card zone is REVEALED.
       The C had no gate, so it evicted a wrongly-set non-live card from the
       live zone during the live-card-SET phase, where Rust does not -- which is
       the "Rust evicts a wrongly-set member AFTER the live, the C evicts BEFORE"
       divergence. Reachable, not a no-op: `face_up` starts false
       (core/zones.rs:514), is set false again by every set_live_card /
       live-card confirmation (phases.rs:832/:887) and true only at the
       performance (phases.rs:343), so between a seat's live-card set and its
       own performance Rust's early return fires and the C's body used to run.

       WHY THE INDEX IS THE SEAT AND NOT A SLOT. The field is declared
       `uint8_t live_face_up[RB_MAX_LIVE_CARDS]` (include/rabuka.h:1801) with a
       comment calling it a per-CARD parallel array beside `live`, but Rust's
       `LiveCardZone.face_up` (core/zones.rs:501) is ONE flag for the whole
       zone, and `LiveCardZone::add_card(&mut self, card_id, _face_up: bool)`
       (zones.rs:1038) IGNORES its flag (hence the underscore) -- so no per-card
       value ever exists to store and a per-card array cannot represent it. The
       two seat flags are written at live_face_up[0] and [1] here. If the header
       owner reshapes the declaration to `uint8_t live_face_up[2];` (the exact
       patch is reported) this code compiles and behaves identically; the header
       COMMENT is what needs correcting, not the storage. */
    if (!g->live_face_up[is_p1 ? 0 : 1]) return;
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
                /* DEFECT FIX. Rust actions/mod.rs:1851-1856 (check_orphaned_
                   under_cards) sends an orphaned ENERGY card to `player.energy_
                   deck`, NOT to the player's active energy zone:
                       if card_db.get_card(cid).is_some_and(|c| c.is_energy()) {
                           player.energy_deck.cards.push(cid);
                       } else {
                           player.waitroom.cards.push(cid);
                       }
                   The C pushed it into `P->energy`, i.e. the ACTIVE ENERGY ZONE,
                   which makes the card immediately usable as energy and never
                   returns it to the deck. Rule 10.5.3 (member leaves the stage,
                   card under it is orphaned) and Rule 10.5.4 (energy card in
                   waitroom -> energy deck) are the same rule: the energy card
                   goes back to the energy DECK, so it must be re-drawn to be
                   used. Same routing as rb_check_invalid_live_cards above. */
                if (rb_card_is_energy(cid)) bag_push_local(&P->energy_deck, cid);
                else                        bag_push_local(&P->discard, cid);
            }
        }
    }
}

void rb_check_invalid_resolution_zone(GameState *g) {
    if (g->resolution.n == 0) { g->n_resolution_owners = 0; return; }
    /* Rust actions/mod.rs:1864-1895 (check_invalid_resolution_zone):
           let cards  = core::mem::take(&mut game_state.resolution_zone.cards);
           let owners = core::mem::take(&mut game_state.resolution_zone.owners);
           for (index, card_id) in cards.into_iter().enumerate() {
               let owner = owners.get(index).copied().unwrap_or(active);
               let owner_player = game_state.seat_player_mut(owner);
               if card_db.get_card(cid).is_some_and(|c| c.is_energy()) {
                   owner_player.energy_deck.cards.push(card_id);
               } else {
                   owner_player.waitroom.cards.push(card_id);
               }
           }

       BOTH halves now land. The ENERGY half was closed earlier (it used to push
       every resolved card into the active player's discard, so an energy card
       pulled in by a yell went to the DISCARD and died there). The OWNER half
       was structurally absent until include/rabuka.h:1617-1618 added
       `resolution_owners[]` + `n_resolution_owners` and
       rb_resolution_add_for_owner (zones.c) added the writer; this drain is the
       reader. Each card goes back to the seat that YELLED it, not to the
       active seat, which is the difference the non-active performer's revealed
       yell cards are measured on (tests/test_parity_rules_extra.c
       test_c_rule_5_8_returns_to_owner_not_active_seat).

       `unwrap_or(active)` is reproduced, not approximated: an entry with no
       owner stamp -- written straight into g->resolution.cards[] rather than
       through rb_resolution_add(_for_owner), which several suites do -- falls
       back to g->active, and the 0xFF sentinel that
       rb_resolution_add_for_owner stores for "owner unknown" lands in the same
       branch (see RB_RESOLUTION_OWNER_UNKNOWN in src/core/zones.c). Iterating
       from the end keeps every `i` equal to the index Rust's forward
       enumerate() saw, which is what pairs a card with its owner. */
    for (int i = g->resolution.n - 1; i >= 0; i--) {
        int owner = (i < g->n_resolution_owners) ? (int)g->resolution_owners[i] : -1;
        if (owner < 0 || owner > 1) owner = g->active;   /* unwrap_or(active) */
        RbPlayer *OP = &g->p[owner];
        int cid = bag_remove_at_local(&g->resolution, i);
        if (rb_card_is_energy(cid)) bag_push_local(&OP->energy_deck, cid);
        else                        bag_push_local(&OP->discard, cid);
    }
    g->n_resolution_owners = 0;
}

void rb_check_timing(GameState *g) {
    /* DEFECT FIX. Rust actions/mod.rs:1632-1638 (check_timing) refreshes in
       FIRST-ATTACKER order, and Player::refresh (core/player.rs:634-648) draws
       from the global RNG (`crate::rng::shuffle_slice`) whenever a deck is empty
       and the waitroom is not:
           if game_state.player1.is_first_attacker {
               game_state.player1.refresh();
               game_state.player2.refresh();
           } else {
               game_state.player2.refresh();
               game_state.player1.refresh();
           }
       `player1.is_first_attacker` is exactly `g->first_attacker == 0` in the C
       (g->first_attacker holds the SEAT index; rb_game_init seeds 0/1). The C
       refreshed p1 then p2 unconditionally, so whenever player 2 was the first
       attacker AND both players had an empty deck with a non-empty waitroom, the
       two shuffles consumed the shared RNG stream in the opposite order and both
       deck orders came out different from Rust's. */
    if (g->first_attacker == 0) { rb_player_refresh(g, 0); rb_player_refresh(g, 1); }
    else                         { rb_player_refresh(g, 1); rb_player_refresh(g, 0); }
    rb_check_victory_condition(g);
    /* actions/mod.rs:1646-1648 — once check_victory_condition has ended the game
       Rust RETURNS and every step below is skipped. The C has no `game_ended`
       field; g->winner is the equivalent the phase machine already gates on
       (rb_advance_phase:118 returns on g->winner!=-1), so this early-out is
       redundant here rather than divergent. Left as-is; see AUDIT §4-7. */
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
/* Mirror phases.rs:1780-1809 resolve_rps_if_both_chosen — rock-paper-scissors
   resolution. Choices: 0=グー, 1=パー, 2=チョキ. (0,2)|(1,0)|(2,1) → P1 wins.

   DEFECT FIX. The old body only stamped g->rps_winner and returned, so the RPS
   phase never moved on: Rust sets
       game_state.rps_winner = Some(rps_winner);
       game_state.current_phase = Phase::ChooseFirstAttacker;
   and on a TIE it logs 引き分け　再選択, clears BOTH player1_rps_choice and
   player2_rps_choice back to None and returns WITHOUT publishing a winner —
   the re-roll. The old C published rps_winner = 0 ("tie") as if it were an
   outcome and left both choices set, so a re-asked round could never resolve
   and the host could never see ChooseFirstAttacker. Both are restored here;
   RB_PHASE_OPENING is the C spelling of Rust's ChooseFirstAttacker
   (src/core/game_state_core.c:242-247 already reads it that way). */
int rb_resolve_rps_if_both_chosen(GameState *g) {
    if (!g) return 0;
    if (g->player1_rps_choice < 0 || g->player2_rps_choice < 0) return 0;
    int p1 = g->player1_rps_choice, p2 = g->player2_rps_choice;
    int winner = 0; /* 0=tie, 1=P1, 2=P2 */
    if ((p1 == 0 && p2 == 2) || (p1 == 1 && p2 == 0) || (p1 == 2 && p2 == 1)) winner = 1;
    else if (p1 != p2) winner = 2;
    if (winner == 0) {
        /* 引き分け　再選択 — clear both choices and re-ask. rps_winner is NOT
           written: Rust leaves the Option untouched across a tie. */
        rb_push_rps_log(g, p1, p2, "tie");
        g->player1_rps_choice = -1;
        g->player2_rps_choice = -1;
        return 0;
    }
    g->rps_winner = winner;
    rb_push_rps_log(g, p1, p2, winner == 1 ? "p1" : "p2");
    /* ChooseFirstAttacker — the host picks the seats next. */
    g->phase = RB_PHASE_OPENING;
    return 1;
}

/* The three mulligan handlers that used to live here
   (rb_handle_mulligan_selection / _confirmation / _skip) were STUBS: they only
   flipped mulligan_selecting[] / mulligan_done[] and never touched a hand, so
   the exact bug engine/src/turn/mulligan_tests.rs exists for — the replacement
   landing on the OPPONENT — was unreachable in C because there was no
   replacement at all. They had ZERO callers in the tree.

   They are DELETED, not re-pointed. The live implementation is the faithful
   port in src/game/game_setup.c (rb_setup_handle_mulligan_selection /
   _confirmation / _skip), which is what rb_setup_execute_action dispatches to
   and what owner-captures the mulligan seat BEFORE advancing the phase. Two
   live copies on one path is exactly what the brief forbids, and the
   rb_setup_* family is the one wired into the action dispatcher. */

/* ───────────────────────────── play-time cost reduction ─────────────────────────────
   Faithful port of engine/src/turn/phases.rs:1317-1670:
     play_time_cost_reduction_hook        (phases.rs:1401)
     play_time_cost_reduction_amount      (phases.rs:1521)
     play_time_alt_cost_chars             (phases.rs:1556)
     normalize_member_name                (phases.rs:1589)
     has_play_time_alt_cost_hand_cards    (phases.rs:1593)
     can_assign_hand_for_alt_cost         (phases.rs:1602)
     build_alt_cost_candidates            (phases.rs:1613)
     has_distinct_assignment_k            (phases.rs:1641)
     find_distinct_assignment_k           (phases.rs:1645)
     discard_play_time_alt_cost           (phases.rs:1673)
     shuffle_waitroom_members_to_deck_bottom (phases.rs:1723)

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

/* Rust: normalize_member_name (phases.rs:1589) — `s.replace([' ', '　'], "")`.
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

/* Rust: find_distinct_assignment_k::backtrack (phases.rs:1646-1663).
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

/* Rust: build_alt_cost_candidates (phases.rs:1613-1637). One candidate list per
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

/* Rust: find_distinct_assignment_k (phases.rs:1645-1671). Writes the chosen
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

/* Rust: has_distinct_assignment_k (phases.rs:1641) — existence only. */
int rb_has_distinct_assignment_k(const GameState *g, int pl, int exclude_id,
                                 const char *const *chars, int n_chars) {
    return rb_find_distinct_assignment_k(g, pl, exclude_id, chars, n_chars,
                                         NULL, 0) > 0;
}

/* Rust: can_assign_hand_for_alt_cost (phases.rs:1602-1611). */
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

/* Rust: play_time_cost_reduction_amount (phases.rs:1521-1547). Returns the
   reduction amount, or RB_PTC_NONE when the card has no play-time reduction. */
int rb_play_time_cost_reduction_amount(const GameState *g, int card_id) {
    (void)g; /* Rust reads only the card database here too. */
    Ability ab;
    if (!ptc_find_reduction_ability(card_id, &ab)) return RB_PTC_NONE;
    int amt = ab.effect && ab.effect->count >= 0 ? ab.effect->count : 0;
    rb_free_ability(&ab);
    return amt;
}

/* Rust: play_time_alt_cost_chars (phases.rs:1556-1586). Writes the required
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

/* Rust: has_play_time_alt_cost_hand_cards (phases.rs:1593-1600) — the ACTIVE
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

/* Rust: discard_play_time_alt_cost (phases.rs:1673-1720) — move one distinct
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

/* Rust: shuffle_waitroom_members_to_deck_bottom (phases.rs:1723-1754).
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
        case RB_PHASE_LIVE_SET_SECOND: return "LiveCardSetSecond";
        case RB_PHASE_PERFORMANCE:    return "Performance";
        case RB_PHASE_PERFORMANCE_SECOND: return "PerformanceSecond";
        case RB_PHASE_VICTORY:        return "Victory";
        case RB_PHASE_DONE:           return "Done";
        case RB_PHASE_MULLIGAN_FIRST: return "MulliganFirst";
        case RB_PHASE_MULLIGAN_SECOND: return "MulliganSecond";
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
    /* phases.rs:830-832 — the card is taken out of the hand face DOWN:
         let card = player.hand.cards.remove(idx);
         player.live_card_zone.face_up = false;
       which re-closes the gate rb_check_invalid_live_cards reads. Noted, not
       fixed: Rust takes this write BEFORE the `live_cards.len() >= MAX_LIVE_CARDS`
       refusal at :834 and therefore also consumes the hand card on that
       refusal, while the C refuses at :1288 with the card still in hand. Moving
       the capacity check would change hand contents, which is outside this
       item; the face-up value agrees on every path both engines reach. */
    g->live_face_up[g->active] = 0;
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
    /* phases.rs:886-887 — the confirmed set goes down face DOWN, the same
       single statement the set_live_card path takes, so a seat that confirms an
       empty selection still re-closes the gate (Rust writes it before looking
       at `placed` at all). */
    g->live_face_up[g->active] = 0;
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
   Rust: engine/src/turn/phases.rs:1132-1250, dispatched from
   engine/src/turn/actions/mod.rs:309-318 (PlayMemberToStage, which forwards the
   action's own card_id / card_indices / stage_area / use_baton_touch).

   The previous body was a "simplified port" that opened with
   `(void)indices; (void)n_indices; (void)use_baton_touch;` and delegated
   straight to rb_play_member. Three of the six Rust parameters were therefore
   DISCARDED and two more were unreachable; the remaining three are reproduced
   below, in Rust's order, because two of them change the RESULT:

     :1139  use_baton_touch.unwrap_or(false)      — the C carries an int, so the
              absent case is 0. Forwarded to rb_play_member_ex, which reads it as
              the LEFT side of player.rs:394-395's `||`.
     :1144  clear_play_scoped_baton_touch()        — the FIRST statement of every
              member play. Was zero-caller in the whole tree.
     :1166-1177  card_id -> hand index. An explicit id is looked up and is an
              error if absent (`:1168-1169`); None takes the FIRST MEMBER in hand
              (`:1170-1177`, `card.is_member()`), which is NOT "the first card" —
              a live card in hand is a member too and Rust's is_member() does not
              exclude it, so this is exactly Rust's filter, not a stricter one.
     :1182-1192  card_indices with len == 2 -> the two areas to replace. Any other
              length is None, i.e. the single-baton path.
     :1194-1196  double baton: the landing area is `stage_area.unwrap_or(db[0])`,
              so the host picks WHICH of the two vacated areas the arrival takes.
     :1198-1208  single: `stage_area.unwrap_or(first empty, else ALL[0])`.
     :1227-1231  the double baton replaces ALL named areas BEFORE placing.

   The double-baton PLACEMENT (pay, protection check, replace both, land, fire
   debut/auto) is engine.c's rb_play_double_baton_ex, the port of Rust's
   play_double_baton (phases.rs:978-1129). The single-baton placement is
   rb_play_member_ex, the port of Player::move_card_from_hand_to_stage
   (core/player.rs:344-...).

   ORDERING NOTE — the play-time cost hook is deliberately NOT called here.
   Rust runs `play_time_cost_reduction_hook` at :1219-1222 and RETURNS from the
   function when it arms a play, before the double-baton arm at :1227. The C's
   equivalent hook is rb_detect_alt_cost, which lives inside rb_play_member_ex
   because it has to re-enter the same placement to pay; so for the double-baton
   path the hook is reached through rb_complete_play_with_cost's re-dispatch of
   the latched pair instead of at :1219. The observable order (hook first, then
   replacement) is preserved; the two are not the same code path because the C
   has no Rust stack frame to return through. */
int rb_handle_play_member_to_stage(GameState *g, int card_id, const int *indices, int n_indices, int stage_area, int use_baton_touch) {
    if (!g) return 0;
    RbPlayer *P = &g->p[g->active];

    /* phases.rs:1139 — `use_baton_touch.unwrap_or(false)`. The C's int carries
     * the unwrapped value; a negative value is treated as absent so a caller
     * that forwards Rust's `None` (game_setup.c's `int` from the action struct)
     * lands on false rather than on "demand a baton". */
    if (use_baton_touch < 0) use_baton_touch = 0;

    /* phases.rs:1141-1144 — the FIRST statement of the function, before the card
     * is even resolved. This is Rust's own word: "Clear stale PLAY-scoped baton
     * touch state from any previous action this turn. Turn-scoped history
     * (arriving ids, per-player counts) survives so same-turn baton plays
     * accumulate." It is NOT the same clear as rb_clear_baton_touch_tracking
     * (the turn-scoped one); the C's rb_clear_play_scoped_baton_touch
     * (src/core/tracking.c:80, the twin of modifiers.rs:1769-1774) drops exactly
     * the four play-scoped fields and nothing else. It had ZERO callers and no
     * prototype in any header, so the replaced-member identity and cost, the
     * zero-cost flag and the last arriving id survived every play. */
    rb_clear_play_scoped_baton_touch(g);

    /* phases.rs:1165-1177 */
    int idx;
    if (card_id >= 0) {
        idx = -1;
        for (int i = 0; i < P->hand.n; i++) {
            if (P->hand.cards[i] == card_id) { idx = i; break; }
        }
        if (idx < 0) return 0;   /* Err("Card with id {} not found in hand") */
    } else {
        idx = -1;
        for (int i = 0; i < P->hand.n; i++) {
            if (rb_card_is_member(P->hand.cards[i])) { idx = i; break; }
        }
        if (idx < 0) return 0;   /* Err("No member cards in hand") */
    }
    int card = P->hand.cards[idx];

    /* phases.rs:1182-1192 — `card_indices.as_ref().and_then(|v| if v.len() == 2
     * { Some([ALL[v[0]], ALL[v[1]]]) } else { None })`. The indices are AREA
     * INDICES into MemberArea::ALL, which is [LeftSide, Center, RightSide]
     * (core/zones.rs:68-72) — the same order as the C's RB_STAGE_SIZE slots, so
     * they are used directly. A short/long list is None, NOT a single baton. */
    int pair[2] = { -1, -1 };
    if (indices && n_indices == 2 &&
        indices[0] >= 0 && indices[0] < RB_STAGE_SIZE &&
        indices[1] >= 0 && indices[1] < RB_STAGE_SIZE) {
        pair[0] = indices[0];
        pair[1] = indices[1];
    }

    /* phases.rs:1194-1208 */
    int area;
    if (pair[1] >= 0) {
        /* Double baton: `stage_area.unwrap_or(db_areas[0])` — the host names
         * which of the two vacated areas the arrival lands in. */
        area = (stage_area >= 0 && stage_area < RB_STAGE_SIZE) ? stage_area : pair[0];
    } else {
        /* `stage_area.unwrap_or_else(|| first empty, else ALL[0])` */
        area = (stage_area >= 0 && stage_area < RB_STAGE_SIZE)
                   ? stage_area
                   : rb_stage_first_empty(P->stage);
        if (area < 0) area = 0;   /* MemberArea::ALL[0] */
    }

    /* phases.rs:1227-1231 */
    if (pair[1] >= 0)
        return rb_play_double_baton_ex(g, g->active, idx, pair[0], pair[1], area);

    /* phases.rs:1260-1271 — move_card_from_hand_to_stage(idx, area,
     * use_baton_touch, ...) is rb_play_member_ex. `card` is unused below only
     * because the placement re-reads the hand at `idx`; it is read here so a
     * card that cannot be decoded is refused before any request is latched. */
    Card probe;
    if (!rb_decode_card_by_index((uint32_t)card, &probe)) return 0;
    rb_free_card(&probe);
    return rb_play_member_ex(g, g->active, idx, area, use_baton_touch);
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


/* ── execute_performance_phase (phases.rs:353-697) ─────────────────────────────────
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

    /* E4's PRODUCER SIDE, capture call. phases.rs:320-333 is the FIRST thing
     * execute_performance_phase does -- before player_id (:334), before performer
     * (:339), before `performer.live_card_zone.face_up = true` (:343) and long
     * before rb_recalc_constants below (:352). It has to be: the snapshot this
     * window pushes is what rb_restore_performance_need_heart_modifiers replays
     * after E3 wipes mods.need_heart at victory determination, so a capture taken
     * after any of those steps would be a different set of entries than Rust's. */
    rb_capture_performance_need_heart(g);

    int performer = is_first ? g->first_attacker : g->second_attacker;
    if (performer < 0 || performer > 1) performer = g->active;

/* phases.rs:343 — `performer.live_card_zone.face_up = true;`, the FIRST
       statement of the performance window and the reason
       check_invalid_live_cards may evict at all. ORDER IS LOAD-BEARING: Rust
       writes it at :343 and calls Self::check_timing(game_state) at :349, so the
       gate is already OPEN for the timing pass that opens the window. Writing it
       after the check_timing below is the bug this comment exists to prevent —
       it leaves the eviction to the NEXT timing pass, which is one phase later
       and not the same board, so a non-live card forced into the live slot
       survives ライブ開始時 (measured: tests/test_parity_jidou_watch.c Q213
       ハナムスビ). The old comment here claimed the reveal "is the yell inside
       rb_perform_live"; that is the YELL, not the face-up flag, and the flag is
       separate state the C had no home for until include/rabuka.h:1801. */
    g->live_face_up[performer] = 1;

    /* phases.rs:240/352 — constants must be freshly registered before the
       window opens, or heart modifiers go stale (q127_wien). */
    rb_recalc_constants(g);
    rb_check_timing(g);

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
       abilities, so both sweeps run a second time.
       Rust reads `perf_data.draw_effects_occurred`, which live.rs:2192 sets as
       `yell_cards.iter().any(|yc| yc.draw_icons > 0)` — i.e. the draw tally
       produced by process_yell_revealed_card_icons (live.rs:112) for each card
       the yell revealed. The C has no stored flag for it (do_yell keeps
       draw_icons in a local), so this call site walks the same revealed pool
       (live.c:606-607 fills g->revealed_cards with exactly the yell cards) and
       asks the ONE canonical decode for that card's draw tally, which is what
       `ptc_yell_had_draw_icon` used to re-derive with its own copy of the
       blade-heart / special-heart classification.

       override_color is passed as -1 (no recolour) on purpose: the only thing a
       recolour can change here is whether a BLADE heart becomes a draw icon, and
       the recolour source is a blade type, which rb_blade_color_to_heart maps
       only to 1..6 / All / pink (live.c:1006-1010) — never to
       RB_HEART_DRAW (8) or RB_HEART_SCORE (9), which the canonical decode
       passes through unchanged (live.c:97-104). So the tally is the same either
       way, and reading it without a re-derived override keeps this file from
       growing a fourth copy of do_yell's override scan. */
    int draw_effects_occurred = 0;
    for (int i = 0; i < g->n_revealed; i++) {
        int total[8], card_hearts[8], note_icons = 0, draw_icons = 0, cheer = 0;
        rb_process_yell_revealed_card_icons(g, g->revealed_cards[i], -1,
                                            total, &cheer, card_hearts,
                                            &note_icons, &draw_icons);
        if (draw_icons > 0) { draw_effects_occurred = 1; break; }
    }
    if (draw_effects_occurred) {
        rb_trigger_auto_abilities(g, performer, "自動");
        rb_process_pending_auto_abilities(g);
        rb_trigger_auto_abilities(g, opponent, "自動");
        rb_process_pending_auto_abilities(g);
    }

    /* phases.rs:655-661 */
    if (is_first) {
        g->active = g->second_attacker;
        /* phases.rs:648 — SecondAttackerPerformance is its own Phase value and
           is now STORED, so the second window is observable as itself. */
        g->phase = RB_PHASE_PERFORMANCE_SECOND;
    } else {
        g->active = g->first_attacker;
        g->live_batch_mode = 0;
        rb_execute_live_victory_determination(g);
        g->live_victory_pending = rb_has_pending_choice(g);
        fprintf(stderr, "[LIVE_VICTORY_PHASE] initial pending=%d\n", g->live_victory_pending);
        g->phase = RB_PHASE_VICTORY;
    }
}

/* ── play_time_cost_reduction_hook (phases.rs:1401-1517) ───────────────────────────
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

/* One activation offer from rb_setup_for_each_ability_candidate, appended to
 * the same growable action list the play-member rows use.
 *
 * The action_type is the one the whole C tree already reads an activation
 * offer through: RB_ACTION_ACTIVATE_ABILITY (23). src/main.c, src/web_server.c
 * and tests/test_parity_draw_flat.c / _ability_watch.c / _wave1c.c all
 * discriminate offers with that constant, and 1 / 14 in this same function are
 * the pass and play-member tags. Using it here is what makes an activated
 * ability visible to every existing reader of the list.
 *
 * `ability_index` is taken for completeness but NOT serialised:
 * RbGeneratedActionParameters carries only card_id / available_areas, and
 * include/rabuka.h is not this file's to edit. The Rust row carries
 * ability_index so a host can name WHICH ability it is activating; without it
 * rb_activate_card resolves the card's activatable slot itself. Recorded here
 * because it is the one decision datum the offer row drops. */
typedef struct {
    RbGeneratedAction **actions;
    int *count;
    int *capacity;
    int failed;
} RbAbilityOfferSink;

static int rb_ability_offer_sink(void *ctx, int card_id, int ability_index,
                                 int from_stage, int stage_slot,
                                 int base_cost, int effective_cost) {
    RbAbilityOfferSink *sink = (RbAbilityOfferSink *)ctx;
    RbGeneratedAction action;
    memset(&action, 0, sizeof(action));
    action.action_type = RB_ACTION_ACTIVATE_ABILITY;
    action.has_parameters = 1;
    action.parameters.card_id = card_id;
    /* available_areas stays NULL / 0: a UseAbility row is not a stage play, and
     * Rust leaves available_areas unset on it. The static rb_action_areas rows
     * belong to the play-member loop and must not be aliased here. */
    action.parameters.available_areas = NULL;
    action.parameters.n_available_areas = 0;
    (void)ability_index; (void)from_stage; (void)stage_slot;
    (void)base_cost; (void)effective_cost;
    RbGeneratedAction *next = rb_actions_push(*sink->actions, sink->count,
                                              sink->capacity, action);
    if (!next) { sink->failed = 1; return 1; }
    *sink->actions = next;
    return 0;
}

/* Push one bare row: an action_type plus a card_id and nothing else. Used for
 * the mulligan / live-set / pass rows, none of which carries an area list. */
static int rb_push_card_action(RbGeneratedAction **actions, int *count, int *capacity,
                               int action_type, int card_id) {
    RbGeneratedAction action;
    memset(&action, 0, sizeof(action));
    action.action_type = action_type;
    action.has_parameters = 1;
    action.parameters.card_id = card_id;
    action.parameters.available_areas = NULL;
    action.parameters.n_available_areas = 0;
    RbGeneratedAction *next = rb_actions_push(*actions, count, capacity, action);
    if (!next) return 0;
    *actions = next;
    return 1;
}

/* game_setup.rs:1467 generate_mulligan_actions, non-label content. Rust pushes
 * a ConfirmMulligan header and then one SelectMulligan row per card of the
 * mulligan OWNER's hand — the loop at :1489-1532 has no limit and no gating, so
 * every hand card gets a row whether or not it is currently selected. */
static int rb_push_mulligan_rows(const GameState *state, RbGeneratedAction **actions,
                                 int *count, int *capacity) {
    RbSetupHandCandidate rows[RB_MAX_HAND];
    int n = rb_setup_mulligan_candidates(state, rows, RB_MAX_HAND);
    if (!rb_push_card_action(actions, count, capacity, RB_SETUP_ACT_CONFIRM_MULLIGAN, 0))
        return 0;
    for (int i = 0; i < n; i++)
        if (!rb_push_card_action(actions, count, capacity, RB_SETUP_ACT_SELECT_MULLIGAN,
                                 rows[i].card_id))
            return 0;
    return 1;
}

/* game_setup.rs:2353 generate_live_card_set_actions, non-label content. A
 * ConfirmLiveCardSet header, then one SelectLiveCard row per hand card of the
 * ACTIVE player; rows past the set limit are already withheld inside the
 * builder (game_setup.rs:2385-2388, mirrored at game_setup.c:1392). */
static int rb_push_live_set_rows(const GameState *state, RbGeneratedAction **actions,
                                 int *count, int *capacity) {
    RbSetupHandCandidate rows[RB_MAX_HAND];
    int n = rb_setup_live_card_candidates(state, rows, RB_MAX_HAND);
    if (!rb_push_card_action(actions, count, capacity, RB_SETUP_ACT_CONFIRM_LIVE_CARD_SET, 0))
        return 0;
    for (int i = 0; i < n; i++)
        if (!rb_push_card_action(actions, count, capacity, RB_SETUP_ACT_SELECT_LIVE_CARD,
                                 rows[i].card_id))
            return 0;
    return 1;
}

RbGeneratedActionList rb_generate_action_candidates(const GameState *state) {
    RbGeneratedActionList result = { NULL, 0 };
    if (!state) return result;

    /* ── Which phases generate rows at all — game_setup.rs:641-659 ────────────
       Rust's generate_possible_actions matches on current_phase and only five
       groups build a non-empty list:
         RockPaperScissors                          -> generate_rps_actions
         ChooseFirstAttacker                        -> 2 constant rows
         MulliganFirstAttacker|MulliganSecondAttacker-> generate_mulligan_actions
         Main                                       -> generate_main_phase_actions
         LiveCardSetFirstAttacker|LiveCardSetSecond…  -> generate_live_card_set_actions
       and returns Vec::new() for Active, Energy, Draw, FirstAttackerPerformance,
       SecondAttackerPerformance and LiveVictoryDetermination (:648-650, :656-658).
       The C guard used to be a bare `phase != RB_PHASE_MAIN -> {}`, so the
       mulligan and live-set windows were never offered at all and
       rb_setup_mulligan_candidates / rb_setup_live_card_candidates had no
       consumer. The same table, minus the two constant groups (below).

       NOT BUILT, DELIBERATELY — RockPaperScissors and ChooseFirstAttacker.
       Both are pure constant rows in Rust (game_setup.rs:1446-1465: three RPS
       rows; two attacker-choice rows), so they need no builder, and neither
       ActionType has a C reader: rb_setup_execute_action dispatches them, but
       IT has no caller either, and the C tree's own action_type numbering is
       NOT Rust's ActionType ordinals — phase.c tags the pass row 1 and
       src/main.c:45/69 and src/web_server.c:156-159 both hard-code `== 1` for
       pass, which is RbSetupActionType RB_SETUP_ACT_ROCK_CHOICE. Emitting RPS
       rows under Rust's ordinals would make a Rock row indistinguishable from
       a pass row for every reader in the tree. The two groups are therefore
       reported rather than faked; unifying the numbering needs the header
       owner plus main.c/web_server.c.

       NOT BUILT: the pending-choice arm. Rust checks get_pending_choice() FIRST
       (:637-639) and routes to generate_pending_choice_actions, which is a
       500-line menu builder with no C port behind it (the C prompts through
       rb_has_pending_choice / rb_resume_with_choice instead). There is nothing
       here to call, so nothing is invented. */
    switch (state->phase) {
        case RB_PHASE_MULLIGAN_FIRST:
        case RB_PHASE_MULLIGAN_SECOND:
        case RB_PHASE_MAIN:
        case RB_PHASE_LIVE_SET:
        case RB_PHASE_LIVE_SET_SECOND:
            break;
        default:
            return result;
    }

    int capacity = 1 + state->p[state->active].hand.n * RB_STAGE_SIZE;
    RbGeneratedAction *actions = (RbGeneratedAction *)rb_malloc((size_t)capacity * sizeof(*actions));
    if (!actions) return result;

    /* ── Mulligan windows (game_setup.rs:647 -> :1467) ─────────────────────── */
    if (state->phase == RB_PHASE_MULLIGAN_FIRST || state->phase == RB_PHASE_MULLIGAN_SECOND) {
        if (!rb_push_mulligan_rows(state, &actions, &result.count, &capacity)) {
            rb_free(actions);
            result.actions = NULL;
            result.count = 0;
            return result;
        }
        result.actions = actions;
        return result;
    }

    /* ── Live-card-set windows (game_setup.rs:654 -> :2353) ────────────────── */
    if (state->phase == RB_PHASE_LIVE_SET || state->phase == RB_PHASE_LIVE_SET_SECOND) {
        if (!rb_push_live_set_rows(state, &actions, &result.count, &capacity)) {
            rb_free(actions);
            result.actions = NULL;
            result.count = 0;
            return result;
        }
        result.actions = actions;
        return result;
    }

    /* ── Main phase (game_setup.rs:651 -> :1690) ──────────────────────────── */
    RbGeneratedAction pass;
    memset(&pass, 0, sizeof(pass));
    /* ActionType::Pass. NOTE the C tree spells "pass" as 1, not Rust's ordinal 0
       (see the switch comment above); src/main.c:45/69 and src/web_server.c:156
       hard-code 1, so the existing value is load-bearing and is not changed
       here. RB_SETUP_ACT_PLAY_MEMBER_TO_STAGE below, by contrast, is 14 in both
       vocabularies. */
    pass.action_type = 1;
    actions[result.count++] = pass;

    /* Rule 7.7.2.2 — 「プレイ Pamplona」 withholds the PLAY-MEMBER offers only.
     * In Rust the guard at game_setup.rs:1707 wraps the per-hand-card block and
     * CLOSES at :2076; the ability scans at :2078 and :2254 sit OUTSIDE it, so
     * an ability stays offerable while member plays are prohibited. The
     * play-member half now reads its rows from rb_setup_play_candidates, which
     * applies the same rule internally (game_setup.c:1090) — the guard is kept
     * because it is what makes the ability half below unconditional. */
    if (!rb_is_action_prohibited(state, "play_member")) {
    /* The play-member scan is rb_setup_play_candidates (game_setup.rs:1766-2075,
     * the whole per-hand-card body, rules only). The hand-rolled loop that used
     * to live here disagreed with it — and with Rust — three ways, all of which
     * disappear by calling it:
     *   1. effective cost. The old line read
     *        effective_cost = card.cost + cost_modifier - reduction
     *      folding the CONSTANT cost modifier into the play price. Rust does not:
     *      game_setup.rs:1769 takes card.cost.unwrap_or(0) and :1783 computes
     *      card_cost.saturating_sub(reduction) — the modifier is not in the play
     *      cost at all (it only prices a BATON of that member, via
     *      stage_play_cost at :1674/:1546). The builder does the same
     *      (game_setup.c:1107).
     *   2. baton price of the member being replaced. The old line read
     *        existing_cost = existing.cost + existing_modifier
     *      with no floor. Rust prices that member with stage_play_cost
     *      (game_setup.rs:1674) = floored_cost(base, modifier) =
     *      saturate_u8(base + modifier).max(1) (game_setup.rs:1546 +
     *      core/constants.rs:116), so a net-zero or negative modifier floors to
     *      1, never 0. The builder calls rb_setup_stage_play_cost, which floors
     *      (game_setup.c:942 / :1058).
     *   3. LIVE cards. The old loop filtered on rb_card_is_member only, so a
     *      live card in hand (every live card is a member) was offered as a
     *      play-member row. Rust requires card.is_member() && !card.is_live()
     *      (game_setup.rs:1768) and the builder repeats it (game_setup.c:1095).
     * Rust emits one Action per AVAILABLE AREA (game_setup.rs:1883-1990) and
     * each carries the full three-area table, which is the row shape kept here. */
    RbSetupPlayCandidate rows[RB_MAX_HAND];
    int n_rows = rb_setup_play_candidates(state, rows, RB_MAX_HAND);
    for (int r = 0; r < n_rows; r++) {
        if (!rows[r].any_area_available) continue;
        for (int area = 0; area < RB_STAGE_SIZE; area++) {
            rb_action_areas[r][area].available = rows[r].areas[area].available;
            rb_action_areas[r][area].is_baton_touch = rows[r].areas[area].is_baton_touch;
        }
        for (int area = 0; area < RB_STAGE_SIZE; area++) {
            if (!rb_action_areas[r][area].available) continue;
            RbGeneratedAction action;
            memset(&action, 0, sizeof(action));
            action.action_type = RB_SETUP_ACT_PLAY_MEMBER_TO_STAGE;
            action.has_parameters = 1;
            action.parameters.card_id = rows[r].card_id;
            action.parameters.available_areas = rb_action_areas[r];
            action.parameters.n_available_areas = RB_STAGE_SIZE;
            RbGeneratedAction *next = rb_actions_push(actions, &result.count,
                                            &capacity, action);
            if (!next) {
                rb_free(actions);
                result.actions = NULL;
                result.count = 0;
                return result;
            }
            actions = next;
        }
    }
    } /* end !is_action_prohibited("play_member") */

    /* ── Main-phase activation offers (game_setup.rs:2078-2348) ────────────
     * Rust appends these AFTER the play-member block: the stage scan first
     * (three slots, printed AND runtime-gained abilities), then the 控え室
     * scan. Every gate — 「…場合のみ起動できる」, trigger/effect position,
     * discard-only abilities, the per-turn use limit, the mandatory-cost
     * payability rules and the optional-payment exemption — is applied on the
     * game_setup.c side; this loop only turns each surviving offer into one
     * action row, exactly as Rust pushes one Action per offered ability. */
    {
        RbAbilityOfferSink sink;
        sink.actions = &actions;
        sink.count = &result.count;
        sink.capacity = &capacity;
        sink.failed = 0;
        rb_setup_for_each_ability_candidate(state, rb_ability_offer_sink, &sink);
        if (sink.failed) {
            /* rb_actions_push ran out of memory mid-append; report no list
             * rather than a truncated one, matching the play-member path. */
            rb_free(actions);
            result.actions = NULL;
            result.count = 0;
            return result;
        }
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

