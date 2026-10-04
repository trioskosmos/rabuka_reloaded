//! Strategy bot v8 — outcome-model search.
//!
//! v8 is not a retune of v7. v7's structure is a chain of per-branch
//! heuristics: a ten-weight one-ply Main aggregate, a live-set policy built
//! from seven mutually exclusive special cases (free win, ceiling fold,
//! minimum-win, strict closeout, desperation life, junk fill, role gates),
//! each with its own floor constant, plus an inherited v4 mulligan. Every
//! measurement in docs/BOT_STRATEGY.md against that structure came back
//! "neutral or noise": a flat baton bonus cost 18pp, a Pass tax exploded
//! draws, energy weights moved nothing, and a reachable-curve mulligan lost
//! because it preserved a hand SHAPE whose value a one-ply eval already
//! captured. That is the signature of an architecture whose parameters are
//! not connected to the objective.
//!
//! v8 replaces the architecture with three things, all in this module's
//! siblings:
//!
//! - [`v8_model`] — ONE model of a live check, uncertainty-aware and
//!   rule-exact, with the opponent modelled from public information only. It
//!   is also the forward development model (the guides' 4→9→13 cost ladder).
//! - [`v8_live`] — the live set is a single argmax over every zone the hand
//!   can legally form. Free wins, conceding, junk-only sets and desperation
//!   gambles are no longer branches; they are the argmax in the states where
//!   they really are the best play.
//! - [`v8_main`] — the Main phase is a bounded two-ply search whose leaves are
//!   expected PLACEMENT ADVANTAGE, so a deploy is priced by the check it
//!   enables, not by a counter delta it happens to move.
//!
//! The mulligan lives here and uses the same currency: it enumerates every
//! legal replacement and scores the kept hand by a three-turn forward
//! simulation, so it evaluates the opening a hand actually produces instead of
//! matching a curve pattern.
//!
//! Fairness: v8 reads own hand / deck / stage / energy / waitroom plus the
//! opponent's PUBLIC zones (stage, success count, live-zone SIZE). It never
//! reads the opponent's hand, deck, energy deck or set contents. Own-deck
//! composition (blade-heart density, live count) is fair information per
//! docs/BOT_STRATEGY.md §9.
//!
//! Experiment switches. Each isolates ONE doctrine component, per the
//! validation protocol in docs/BOT_STRATEGY.md: never change two at once, and
//! never believe a result without an interval.
//!
//! - `V8_NO_INITIATIVE` / `V8_INITIATIVE_WEIGHT` - the turn-order term.
//! - `V8_NODES` (default 96), `V8_FOLLOWUP` (default 6) - search budget.
//!   `V8_NODES=1` collapses the Main phase to the one-ply leaf.
//! - `V8_MULLIGAN_V4` - fall back to the inherited v4 mulligan.
//! - `V8_MAIN_V7` - delegate the Main phase to v7, keeping v8's live set.
//!   Isolates "is the new Main phase helping or hurting".
//! - `V8_LIVE_V7` - delegate the live set to v7, keeping v8's Main phase.
//!   Isolates the live-set decision, which is where the placement rate lives.
//! - `V8_NO_FOLD` - never choose an empty live zone while holding a life.
//!   The floor v7 needed as a hand-tuned constant; if v8 only needs this to
//!   keep up with v7, the outcome model is not doing its job and should not
//!   ship.
//! - `V8_DEBUG` - decision table on stderr (UNTRACED runs only; logging
//!   perturbs allocation layout and changes outcomes, section 8.4).

use crate::bot::strategy_common::{emit_mulligan, Acc};
use crate::card::{CardDatabase, CardType};
use crate::game_setup::Action;
use crate::game_state::GameState;

use super::v8_model::{self, PLACEMENT_CREDIT};

const SCALE: f64 = 1000.0;

fn flag(name: &str) -> bool {
    std::env::var_os(name).is_some()
}

// ── Main phase ───────────────────────────────────────────────────────────

pub fn choose_action_v8_entry(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if flag("V8_MAIN_V7") {
        return crate::bot::strategy_v7::choose_action_v7(gs, actions, me);
    }
    // Turn order, rule 8.4.13. Investigated by playing the side against v7 and
    // comparing: over 30 games v7 and v8 disagreed on the RPS turn-order pick
    // 30 out of 30, v7 taking first and v8 second. A perfect split reads like
    // one bot picking option 0 and the other option 1 rather than a judgement.
    //
    // It is NOT fixed here, and the reason is worth recording. Intercepting
    // `ChooseFirstAttacker` in this entry point to take v7's option produced a
    // byte-identical 800-game paired result (0 discordant games, p = 1.0),
    // because `bot_arena` does not route the turn-order phase through this
    // entry - v8's Main-phase chooser handles it and already takes first.
    // So the divergence is specific to the harness path and is not a v8 defect
    // on the measured path. Reverted rather than shipped as a change that
    // cannot be shown to do anything.
    probe_main_against_v7(gs, actions, me);
    super::v8_main::choose_action_v8(gs, actions, me)
}

/// `V8_PROBE_V7=1`: run BOTH Main-phase policies on the same state and log every
/// disagreement.
///
/// This exists because eleven leaf-term experiments and a crossover told me
/// *that* v8's Main phase is worse and roughly *how much*, and nothing at all
/// about *which decisions* are wrong. Aggregate win rates cannot localise a
/// defect: the same +9 point gap is consistent with v8 losing many small
/// decisions, a few catastrophic ones, or one systematic rule, and those need
/// different fixes. Every one of the earlier experiments was chosen by
/// hypothesising which of those it was.
///
/// Each line carries the state features the two policies actually read, so a
/// disagreement can be attributed rather than merely counted:
///
/// ```text
/// PROBE t=3 me=0 bl=4 cost=9 h=7 ammo=2 hand=4 opp:bl=3 h=6 | v8=Play(5) v7=Play(6)
/// ```
///
/// UNTRACED by construction — it is an `eprintln!` in the hot path, and
/// `BOT_STRATEGY.md` section 8.4 records that logging changes outcomes. Diagnosis
/// only; never a reported number.
pub fn probe_main_against_v7(gs: &GameState, actions: &[Action], me: u8) {
    if !flag("V8_PROBE_V7") {
        return;
    }
    // Only the states both policies are actually asked about.
    if gs.current_phase != crate::types::Phase::Main || gs.has_pending_choice() {
        return;
    }
    let db = &gs.card_database;
    // Both choosers are pure — they return an `Action` and touch neither the
    // state nor the RNG (each seeds its own scratch) — so the same state can be
    // put to both policies and the two answers compared directly. Comparing
    // their SCORE LABELS instead was the first version and it was useless: v8
    // calls Pass "end-main" and v7 calls it "end-main=0", so every pass in the
    // game logged as a disagreement.
    let ours = super::v8_main::choose_action_v8(gs, actions, me);
    let theirs = crate::bot::strategy_v7::choose_action_v7(gs, actions, me);
    // `Action` carries no top-level `card_id`; the target lives in
    // `parameters`, whose rendering is not stable across the two bots, so the
    // comparison is on `action_type` plus the engine's own `description`, which
    // both share because it is produced by the action generator rather than by
    // either policy.
    if ours.action_type == theirs.action_type && ours.description == theirs.description {
        return;
    }
    let describe = |x: &Action| format!("{:?} {}", x.action_type, x.description);
    let blades = super::v8_model::active_blades(gs, me, db);
    let opp = 1 - me;
    eprintln!(
        "PROBE t={} me={} bl={} cost={} h={} ammo={} hand={} deck={} | opp: bl={} h={} | \
         v8={} v7={}",
        gs.turn_number,
        me,
        blades,
        super::v8_model::stage_cost(gs, me, db),
        super::v8_model::supply_hearts(gs, me, db),
        super::v8_model::lives_in_hand(gs.seat_player(me), db),
        gs.seat_player(me).hand.cards.len(),
        gs.seat_player(me).main_deck.cards.len(),
        super::v8_model::active_blades(gs, opp, db),
        super::v8_model::supply_hearts(gs, opp, db),
        describe(&ours),
        describe(&theirs),
    );
}

pub fn score_actions_v8(gs: &GameState, actions: &[Action], me: u8) -> Vec<(f64, String)> {
    super::v8_main::score_actions_v8(gs, actions, me)
}

// ── Live set ─────────────────────────────────────────────────────────────

pub fn choose_live_set_v8_entry(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    if flag("V8_LIVE_V7") {
        return crate::bot::strategy_v7::choose_live_set_v7(gs, actions, db);
    }
    super::v8_live::choose_live_set_v8(gs, actions, db)
}

/// v8's live set with the set-width cap raised to two lives. This is the only
/// difference from `choose_live_set_v8_entry`, and `v8_live` carries the
/// measurement that justifies it.
pub fn choose_live_set_v8_wide_entry(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    super::v8_live::choose_live_set_v8_wide(gs, actions, db)
}

// ── Mulligan: a three-turn forward evaluation of every legal opening ─────

/// `P(we place at the next check)` for the best life in the kept hand, using
/// the same closed-form estimate as the Main phase.
fn opening_place(
    opening: &v8_model::Forward,
    gs: &GameState,
    me: u8,
    lives: &[(i32, Acc)],
) -> f64 {
    let db = &gs.card_database;
    let opp = super::v8_model::OppModel::build(gs, me, db);
    let yell = v8_model::expected_yell_score(gs, me, db, opening.blades);
    lives
        .iter()
        .map(|(score, need)| {
            let p_pass = v8_model::pass_estimate_from_supply(
                &opening.hearts,
                opening.blades,
                opening.density,
                need,
            );
            v8_model::check_outcome(p_pass, score + yell, 1, 0, &opp).p_place
        })
        .fold(0.0f64, f64::max)
}

/// Forward value of one opening, in the same placement units the Main phase
/// uses, so the opening and the play that follows it are measured the same way.
///
/// `discarded` matters as much as `keep`: a replacement puts a card from our
/// own deck into the hand before turn 1. Modelling a replacement as a pure loss
/// - which the first draft did - makes "keep every card" the argmax of every
///   search, and v8 replaced 1.24 cards against v4's 2.00. Our own deck is fair
///   information (section 9), so the redraw is taken from its top in order.
///
/// The walk itself is [`v8_model::Forward`], the same one the Main-phase leaf
/// runs. It used to be a second copy written next to it, and the two copies
/// disagreed about baton touches: this one overwrote the stage slot without
/// moving the sent member's blades out of the active set or its hearts into the
/// pool, so it over-counted blades and under-counted hearts by one member for
/// every step of the ladder, and the error grew with the depth of the walk this
/// function takes. One walk, one answer.
fn opening_value(
    db: &CardDatabase,
    gs: &GameState,
    me: u8,
    keep: &[i16],
    discarded: &[i16],
    energy: i32,
    density: f64,
) -> f64 {
    let mut opening = v8_model::Forward::from_opening(
        db,
        energy,
        gs.seat_player(me).main_deck.cards.to_vec(),
        keep,
        discarded,
    );
    opening.density = density;
    // A kept life and a REDRAWN life are both in hand at turn 1, so both are
    // settable ammunition and both belong in the check the opening is scored
    // against. A replacement that draws a life is worth a replacement.
    let lives: Vec<(i32, Acc)> = keep
        .iter()
        .chain(opening.redraw.iter())
        .filter_map(|&cid| {
            let card = db.get_card(cid)?;
            if card.card_type != CardType::Live {
                return None;
            }
            let (score, need) = (v8_model::printed_score(db, cid), v8_model::life_need(gs, cid));
            if v8_model::has_unpassable_icon(&need) {
                return None;
            }
            Some((score, need))
        })
        .collect();

    // Five snapshots: the opening board plus one per Main phase of T1..T4.
    let mut snapshots: Vec<f64> = Vec::with_capacity(5);
    for _ in 0..5 {
        snapshots.push(opening_place(&opening, gs, me, &lives));
        opening.step(db);
    }
    // Placement by T3, plus the gain the following turn's development unlocks,
    // plus the ammunition we are keeping. Same shape as
    // `v8_main::leaf_value`, so the opening and the play agree on units.
    let place_3 = snapshots[3];
    let place_4 = snapshots[4];
    SCALE * (place_3 + (place_4 - place_3))
        + SCALE * PLACEMENT_CREDIT * opening.lives as f64
        - blade_budget_penalty(db, gs, me, keep, discarded)
}

/// Cards in the deck carrying no blade-heart: the growth/convertibility budget.
///
/// This is the resource competitive play turns on, and nothing in `bot/` counts
/// it. The community write-up is explicit about the trade: a deck grows fast by
/// running high-cost members, those are the ones with no blade-heart, and every
/// one of them lowers the check rate. Their worked numbers for having 6 / 7 / 8
/// of them and still drawing the cost-9 by turn 2 are **89% / 93% / 95%** — so
/// the sixth is worth +4 points and the eighth only +2, and the marginal card is
/// already nearly dead by the time there are seven.
///
/// `opening_place` prices the *check*, so the opening is currently scored with
/// no view of what it costs in growth fuel. That is a one-sided evaluation: it
/// can see why a hand is good and never why it is unaffordable. A hand that
/// keeps two more no-blade-heart members than the budget allows is being paid
/// for a curve the deck cannot actually cash.
///
/// ## Why the penalty is on the post-mulligan deck
///
/// A discarded member goes back and is redrawn from the top, so what matters is
/// the count in the deck the game is played with, not the count in hand. Both
/// are counted here, because a no-blade-heart member kept in hand is a card
/// that will be played; one discarded is a card that will be drawn again later
/// and can wait. Penalising both is the conservative reading and the switch
/// exists so the two can be separated if the first measurement disagrees.
fn blade_budget_penalty(
    db: &CardDatabase,
    gs: &GameState,
    me: u8,
    keep: &[i16],
    discarded: &[i16],
) -> f64 {
    let weight: f64 = std::env::var("V8_MULL_BH")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|w: &f64| w.is_finite() && *w >= 0.0)
        .unwrap_or(0.0);
    if weight == 0.0 {
        return 0.0;
    }
    let budget: i32 = std::env::var("V8_MULL_BH_BUDGET")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7);
    let no_blade = |cid: i16| {
        db.get_card(cid)
            .is_some_and(|c| c.card_type == CardType::Member && c.blade_heart.is_none())
    };
    let in_deck = gs
        .seat_player(me)
        .main_deck
        .cards
        .iter()
        .filter(|&&cid| no_blade(cid))
        .count() as i32;
    // Kept fuel is spent early; discarded fuel is still drawable, so it counts
    // at half. The hand is where the cost is actually paid.
    let kept = keep.iter().filter(|&&cid| no_blade(cid)).count() as i32;
    let drawn = discarded.iter().filter(|&&cid| no_blade(cid)).count() as i32;
    let total = in_deck + kept + drawn / 2;
    weight * (total - budget).max(0) as f64
}

/// Which hand indices to DISCARD. The engine permits zero through six
/// replacements; v4 always replaced up to three and v7's curve variant only
/// fired on ~17% of hands. v8 scores every legal discard set with the
/// three-turn forward simulation and takes the best, so the decision always
/// fires and always has an out (keeping the whole hand scores too).
pub fn choose_mulligan_v8(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    if std::env::var_os("V8_MULLIGAN_V4").is_some() {
        return crate::bot::strategy_v4::choose_mulligan_v4(gs, actions, db);
    }
    let me = gs.active_player_index();
    let hand = gs.seat_player(me).hand.cards.to_vec();
    if hand.is_empty() {
        return crate::bot::strategy_v4::choose_mulligan_v4(gs, actions, db);
    }
    // Blade-heart density of our own deck, for the closed-form flip estimate.
    let deck = gs.seat_player(me).main_deck.cards.clone();
    let deck_len = deck.len().max(1) as f64;
    let density = deck
        .iter()
        .filter(|&&cid| {
            db.get_card(cid)
                .is_some_and(|c| c.blade_heart.is_some())
        })
        .count() as f64
        / deck_len;
    let energy = gs.seat_player(me).energy_zone.active_count() as i32;

    // Engine rule: replacements are capped, and the cap is the number of cards
    // we were actually offered. Anything above the hand size is illegal.
    let max_replacements = hand.len().min(6);
    let mut best_discard: Vec<usize> = Vec::new();
    let mut best_value = f64::NEG_INFINITY;
    let subsets = 1usize << hand.len().min(16);
    for mask in 0..subsets {
        let discard: Vec<usize> = (0..hand.len()).filter(|i| mask & (1 << i) != 0).collect();
        if discard.len() > max_replacements {
            continue;
        }
        let keep: Vec<i16> = hand
            .iter()
            .enumerate()
            .filter(|(i, _)| !discard.contains(i))
            .map(|(_, &cid)| cid)
            .collect();
        let discarded: Vec<i16> = discard.iter().map(|&i| hand[i]).collect();
        let value = opening_value(db, gs, me, &keep, &discarded, energy, density);
        // Ties prefer keeping more cards: a replacement is only worth it if the
        // redraw is strictly better, which is the discipline the rejected v7
        // reachable-mulligan experiment lacked.
        let better = value > best_value + 1e-9
            || ((value - best_value).abs() <= 1e-9 && discard.len() < best_discard.len());
        if better {
            best_value = value;
            best_discard = discard;
        }
    }
    log::debug!(
        "v8 mulligan t{} me{} hand={} discard={:?} value={:.2}",
        gs.turn_number,
        me,
        hand.len(),
        best_discard,
        best_value
    );
    if std::env::var_os("V8_DEBUG").is_some() {
        eprintln!(
            "V8MG t{} me{} hand={} discard={:?} value={:.2}",
            gs.turn_number,
            me,
            hand.len(),
            best_discard,
            best_value
        );
    }
    emit_mulligan(gs, actions, &best_discard)
}

// ── Re-exports with generic names for bot/registry.rs ────────────────────

pub use choose_action_v8_entry as choose_action;
pub use choose_live_set_v8_entry as choose_live_set;
pub use choose_live_set_v8_wide_entry as choose_live_set_wide;

/// v8's live set with rule 8.4.13's turn-order term priced at `V8_TURNORDER`.
/// This is v12's whole difference from stock `choose_live_set`.
pub use super::v8_live::choose_live_set_v8_order as choose_live_set_order;
pub use super::v8_live::choose_live_set_v8_counter as choose_live_set_counter;
pub use super::v8_live::choose_live_set_v8_br as choose_live_set_br;

/// v9's Main phase with `V7_CHECK_PRESSURE` / `V7_BLADE_ARITH` installed for
/// THIS seat only. This is v16's whole difference from stock v9.
///
/// The guard is why this is a function rather than an env var: the two weights
/// are process-global, so setting them from a switch would change both seats of
/// a head-to-head and the matchup would measure nothing.
pub fn choose_action_v16(
    gs: &GameState,
    actions: &[Action],
    me: u8,
) -> Action {
    let num = |key: &str| {
        std::env::var(key)
            .ok()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v: &f64| v.is_finite() && *v >= 0.0)
            .unwrap_or(0.0)
    };
    let _guard = crate::bot::v7_main::LookaheadGuard::new(
        num("V7_CHECK_PRESSURE"),
        num("V7_BLADE_ARITH"),
    );
    crate::bot::strategy_v7::choose_action(gs, actions, me)
}
pub use choose_mulligan_v8 as choose_mulligan;
pub use score_actions_v8 as score_actions;

pub use super::v8_live::LivePrediction;

/// The live-set decision's own numbers, for offline calibration against engine
/// outcomes. Recomputes the argmax from the same inputs and reads no hidden
/// state, so calling it cannot change a game.
pub fn predict_live_set_v8(
    gs: &GameState,
    db: &CardDatabase,
) -> Option<LivePrediction> {
    super::v8_live::predict_live_set(gs, db)
}
