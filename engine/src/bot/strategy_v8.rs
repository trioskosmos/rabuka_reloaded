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
//! Experiment switches (each isolates ONE doctrine component, per the
//! validation protocol in docs/BOT_STRATEGY.md):
//! - `V8_NO_INITIATIVE` / `V8_INITIATIVE_WEIGHT` — the turn-order term.
//! - `V8_NODES` (default 96), `V8_FOLLOWUP` (default 6) — search budget.
//! - `V8_MULLIGAN_V4` — fall back to the inherited v4 mulligan.
//! - `V8_DEBUG` — decision table on stderr (UNTRACED runs only; logging
//!   perturbs allocation layout and changes outcomes, §8.4).

use crate::bot::strategy_common::{acc_add, emit_mulligan, Acc};
use crate::card::{CardDatabase, CardType};
use crate::game_setup::Action;
use crate::game_state::GameState;

use super::v8_model::{self, PLACEMENT_CREDIT};

const SCALE: f64 = 1000.0;

// ── Main phase ───────────────────────────────────────────────────────────

pub fn choose_action_v8_entry(gs: &GameState, actions: &[Action], me: u8) -> Action {
    super::v8_main::choose_action_v8(gs, actions, me)
}

pub fn score_actions_v8(gs: &GameState, actions: &[Action], me: u8) -> Vec<(f64, String)> {
    super::v8_main::score_actions_v8(gs, actions, me)
}

// ── Live set ─────────────────────────────────────────────────────────────

pub fn choose_live_set_v8_entry(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    super::v8_live::choose_live_set_v8(gs, actions, db)
}

// ── Mulligan: a three-turn forward evaluation of every legal opening ─────

/// Hypothetical board after a prefix of our own Main phases.
struct Opening {
    hearts: Acc,
    blades: i32,
    stage: [i32; 3],
    budget: i32,
    hand: Vec<i16>,
    deck: Vec<i16>,
    deck_cursor: usize,
    lives_kept: usize,
}

impl Opening {
    fn new(me_energy: i32) -> Self {
        Self {
            hearts: [0; 11],
            blades: 0,
            stage: [0; 3],
            budget: me_energy,
            hand: Vec::new(),
            deck: Vec::new(),
            deck_cursor: 0,
            lives_kept: 0,
        }
    }

    fn add_kept(&mut self, db: &CardDatabase, cid: i16) {
        let Some(card) = db.get_card(cid) else {
            return;
        };
        match card.card_type {
            CardType::Member => {
                self.hand.push(cid);
            }
            CardType::Energy => {
                // Energy is a budget unit; +1 is exactly what it buys.
                self.budget += 1;
            }
            CardType::Live => {
                self.lives_kept += 1;
            }
        }
    }

    /// One of our Main phases: energy phase, draw phase, best affordable
    /// member deploy.
    fn step(&mut self, db: &CardDatabase) {
        self.budget += 1; // rule 7.5
        if let Some(cid) = self.deck.get(self.deck_cursor).copied() {
            self.deck_cursor += 1;
            if db
                .get_card(cid)
                .is_some_and(|c| c.card_type == CardType::Member)
            {
                self.hand.push(cid);
            }
        }
        let discount = self.stage.iter().copied().max().unwrap_or(0);
        let has_occupied = discount > 0;
        let baton_slot = self
            .stage
            .iter()
            .enumerate()
            .filter(|(_, c)| **c > 0)
            .min_by_key(|(_, c)| **c)
            .map(|(i, _)| i);
        let free_slot = self.stage.iter().position(|c| *c == 0);

        let mut choice: Option<(usize, i32)> = None;
        for (i, &cid) in self.hand.iter().enumerate() {
            let Some(card) = db.get_card(cid) else {
                continue;
            };
            let cost = i32::from(card.cost.unwrap_or(0));
            if cost <= 0 {
                continue;
            }
            let effective = if has_occupied {
                cost.saturating_sub(discount)
            } else if free_slot.is_some() {
                cost
            } else {
                continue;
            };
            if effective > self.budget {
                continue;
            }
            if choice.is_none_or(|(_, best)| cost > best) {
                choice = Some((i, cost));
            }
        }
        let Some((index, cost)) = choice else {
            return;
        };
        let cid = self.hand.remove(index);
        if let Some(card) = db.get_card(cid) {
            if let Some(base) = &card.base_heart {
                acc_add(&mut self.hearts, &base.hearts);
            }
            self.blades += i32::from(card.blade);
        }
        let slot = if has_occupied {
            baton_slot.unwrap_or(0)
        } else {
            free_slot.unwrap_or(0)
        };
        if let Some(cell) = self.stage.get_mut(slot) {
            *cell = cost;
        }
        self.budget -= if has_occupied {
            cost.saturating_sub(discount)
        } else {
            cost
        };
    }
}

/// `P(we place at the next check)` for the best life in the kept hand, using
/// the same closed-form estimate as the Main phase.
fn opening_place(
    opening: &Opening,
    gs: &GameState,
    me: u8,
    lives: &[(i32, Acc)],
    density: f64,
) -> f64 {
    let db = &gs.card_database;
    let opp = super::v8_model::OppModel::build(gs, me, db);
    let yell = v8_model::expected_yell_score(gs, me, db, opening.blades);
    lives
        .iter()
        .map(|(score, need)| {
            let p_pass =
                v8_model::pass_estimate_from_supply(&opening.hearts, opening.blades, density, need);
            v8_model::check_outcome(p_pass, score + yell, 1, 0, &opp).p_place
        })
        .fold(0.0f64, f64::max)
}

/// Forward value of one kept hand, in the same placement units the Main phase
/// uses, so the opening and the play that follows it are measured the same way.
fn opening_value(
    db: &CardDatabase,
    gs: &GameState,
    me: u8,
    keep: &[i16],
    energy: i32,
    density: f64,
) -> f64 {
    let mut opening = Opening::new(energy);
    // Our own deck is fair information; assume the draws come from it in the
    // order the guides' curve wants (best affordable first).
    opening.deck = gs.seat_player(me).main_deck.cards.to_vec();
    for &cid in keep {
        opening.add_kept(db, cid);
    }
    let lives: Vec<(i32, Acc)> = keep
        .iter()
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
        snapshots.push(opening_place(&opening, gs, me, &lives, density));
        opening.step(db);
    }
    // Placement by T3, plus the gain the following turn's development unlocks,
    // plus the ammunition we are keeping. Same shape as
    // `v8_main::leaf_value`, so the opening and the play agree on units.
    let place_3 = snapshots[3];
    let place_4 = snapshots[4];
    SCALE * (place_3 + (place_4 - place_3)) + SCALE * PLACEMENT_CREDIT * opening.lives_kept as f64
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
        let value = opening_value(db, gs, me, &keep, energy, density);
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
pub use choose_mulligan_v8 as choose_mulligan;
pub use score_actions_v8 as score_actions;
