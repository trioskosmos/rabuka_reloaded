//! Strategy bot v7 — currently a v6 alias (see experimental note below).
//!
//! # Experimental outcome (2026-08-27, cross-deck arena)
//!
//! Two intuitively-sound improvements over v6 were tried and BOTH regressed:
//!
//! 1. **More aggressive, match-point-aware live sets** (lower binomial floors
//!    + mandatory 1-life attempt at match point). On `fade deck` v7 fell to
//!    70-100 vs v6 (v6 was 142-39 vs v5) with stalls rising 17→24. Cause: every
//!    all-or-nothing failure *wastes a life* without placing; lives are not
//!    infinitely recycled mid-game (refresh only fires on deck-out, which these
//!    games never reach). So forcing attempts just depleted ammo and caused more
//!    stalls. This re-confirms the V3_WHY_IT_SUCKS correction: "folding is
//!    usually worse than swinging" is FALSE when ammo is effectively finite.
//!
//! 2. **Color-aware Main development** (bonus for playing members whose
//!    `base_heart` colors match our own hand lives' `need_heart`). v7 then lost
//!    to v6 on 6 of 8 decks (e.g. liella 58-84, fade 47-81, muse 59-78,
//!    5CP3Z 64-79) and only won on hasunosora. Steering development toward
//!    *specific* life colors is suboptimal: what matters for the yell is total
//!    hearts+blades (the flips supply the needed colors stochastically), so
//!    color-matching trades board power for a false target.
//!
//! Net: **v6 is at a heuristic plateau.** Marginal term surgery — the doc's
//! own conclusion in §8.5 / "v5 sits on a plateau" — does not move win rate,
//! and aggressive variants actively hurt. v7 is therefore kept as a v6 alias so
//! it never regresses; the genuine path to >v6 is the structural fix the doc
//! prescribes: simulation/ISMCTS-backed live-set (and main) decisions using the
//! existing `ismcts.rs` + `DeterminizationSampler` + `PublicObservation`
//! infrastructure. Fairness is unchanged (own hand/deck + public board only).
//!
//! To make a real v7, replace the bodies below with an ISMCTS/rollout search
//! (see `engine/src/bot/ismcts.rs`) rather than more scalar terms.

use crate::card::CardDatabase;
use crate::game_setup::Action;
use crate::game_state::GameState;

pub fn choose_action_v7(gs: &GameState, actions: &[Action], me: u8) -> Action {
    crate::bot::strategy_v6::choose_action_v6(gs, actions, me)
}

pub fn choose_live_set_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    crate::bot::strategy_v6::choose_live_set_v6(gs, actions, db)
}

pub fn choose_mulligan_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    crate::bot::strategy_v4::choose_mulligan_v4(gs, actions, db)
}
