//! Shared scaffolding for the strategy bot generations (v1–v5).
//!
//! Decision logic lives in `strategy*.rs`; this module only holds plumbing
//! every generation used to duplicate: per-color heart accounting, rule
//! satisfaction checks, legal-action filtering, and the live-set / mulligan
//! select-deselect-confirm emission chains.
//!
//! Fallback policy: every generation used to end its emission chain with
//! `.expect("live set actions non-empty")` (and the mulligan equivalent).
//! An empty legal-action list is an engine edge case, not a bot decision —
//! instead of panicking we now `log::warn!` and return a synthetic skip/pass
//! action so the caller's `execute_action` reports the illegal move without
//! aborting the process. For all inputs where the old code did not panic,
//! decisions are byte-for-byte unchanged.

use crate::card::{CardDatabase, HeartColor, HeartMap};
use crate::game_setup::{Action, ActionType};
use crate::game_state::GameState;
use crate::player::Player;

/// Per-color heart accumulator. Index space matches `HeartColor` variant
/// order (Heart00..Heart06, BAll, Draw, Score, All).
///
/// `i16`, not `i32`: **44 bytes → 22**. A per-colour heart total, summed over
/// a stage of at most 3 members or a multi-turn supply ladder, never needs 32
/// bits — and `Acc` is carried by value in `LiveCandidate`, `HandContext`
/// (`board` + `best_need: Option<Acc>`), `Forward.hearts`, and several `Vec`s`,
/// so every holder halves.
///
/// It was `[i32; 11]` — and defined in FOUR other places besides this one
/// (`strategy_v2`, `strategy_v3`, `strategy_v4`, `bin/calibrate`), each with
/// its own byte-identical copy of `hc_index` and `acc_add`. Those four now
/// import from here, which is what makes the narrowing a one-line edit instead
/// of five, and removes four copies that could drift from this one.
pub type Acc = [i16; 11];

pub fn hc_index(c: HeartColor) -> usize {
    match c {
        HeartColor::Heart00 => 0,
        HeartColor::Heart01 => 1,
        HeartColor::Heart02 => 2,
        HeartColor::Heart03 => 3,
        HeartColor::Heart04 => 4,
        HeartColor::Heart05 => 5,
        HeartColor::Heart06 => 6,
        HeartColor::BAll => 7,
        HeartColor::Draw => 8,
        HeartColor::Score => 9,
        HeartColor::All => 10,
    }
}

pub fn acc_add(acc: &mut Acc, hearts: &HeartMap) {
    for (c, v) in hearts.iter() {
        acc[hc_index(*c)] = acc[hc_index(*c)].saturating_add(i16::from(*v));
    }
}

/// Sum of stage members' base hearts (all members; wait state only affects
/// blades per Q133).
pub fn stage_base_hearts(p: &Player, db: &CardDatabase) -> Acc {
    let mut acc = [0i16; 11];
    for &cid in p.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        if let Some(card) = db.get_card(cid) {
            if let Some(bh) = &card.base_heart {
                acc_add(&mut acc, &bh.hearts);
            }
        }
    }
    acc
}

/// Rule-faithful satisfaction check.
/// - Specific colors (Heart01-06) must be met exactly; `All` supply and
///   `BAll` blade-hearts are wildcards for specific colors.
/// - `Heart00` requirement is the "any color" total bucket (rule 2.1.1.2 /
///   2.11.3): filled by colorless hearts and any leftover hearts.
/// - Draw/Score icons are not heart requirements.
pub fn requirements_met(have: &Acc, need: &Acc) -> bool {
    let mut wildcard = have[10].saturating_add(have[7]); // All + BAll
    let mut specific_surplus = 0i16;
    for c in 1..=6 {
        let deficit = need[c] - have[c];
        if deficit > 0 {
            wildcard -= deficit;
            if wildcard < 0 {
                return false;
            }
        } else {
            specific_surplus += -deficit;
        }
    }
    // Heart00 bucket: colorless hearts, plus any leftover specific/wild hearts.
    let leftover = specific_surplus + wildcard.max(0) + have[0];
    leftover >= need[0]
}

/// Narrow a wide `i32` heart term to an [`Acc`] element.
///
/// Same clamping contract as `core::constants::score_to_i32`, one level down:
/// per-colour heart terms are single-digit units in every reachable state, and
/// clamping beats a wrapping `as` cast if one ever is not. Used by the call
/// sites that compute a term in `i32` (f64 truncation, summed modifiers) and
/// then fold it into an `i16` accumulator.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
#[inline]
pub fn narrow_hearts(v: i32) -> i16 {
    v.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

/// Graceful replacement for the old `.expect("… actions non-empty")`.
fn empty_action_fallback(context: &str, fallback_type: ActionType) -> Action {
    log::warn!(
        "bot strategy: no legal actions available at {}; returning {:?} fallback",
        context,
        fallback_type
    );
    Action {
        description: format!("fallback ({})", context),
        description_ja: None,
        action_type: fallback_type,
        parameters: None,
        selected: None,
    }
}

/// Find the toggle action selecting/deselecting a hand-indexed card during
/// Live Card Set.
pub fn find_live_select(
    actions: &[Action],
    hand_index: usize,
    want_selected: bool,
) -> Option<Action> {
    actions
        .iter()
        .find(|a| {
            a.action_type == ActionType::SelectLiveCard
                && a.selected == Some(want_selected)
                && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hand_index)
        })
        .cloned()
}

/// Find the toggle action selecting/deselecting a hand-indexed card during
/// Mulligan.
pub fn find_mulligan_select(
    actions: &[Action],
    hand_index: usize,
    want_selected: bool,
) -> Option<Action> {
    actions
        .iter()
        .find(|a| {
            a.action_type == ActionType::SelectMulligan
                && a.selected == Some(want_selected)
                && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hand_index)
        })
        .cloned()
}

pub fn live_selected_indices(gs: &GameState) -> Vec<usize> {
    gs.live_card_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect()
}

pub fn mulligan_selected_indices(gs: &GameState) -> Vec<usize> {
    gs.mulligan_selected_indices
        .iter()
        .map(|&i| i as usize)
        .collect()
}

/// Confirm-or-first terminal step of a Live Card Set emission chain.
pub fn confirm_live_set(actions: &[Action]) -> Action {
    actions
        .iter()
        .find(|a| a.action_type == ActionType::ConfirmLiveCardSet)
        .or_else(|| actions.first())
        .cloned()
        .unwrap_or_else(|| empty_action_fallback("live set", ActionType::Pass))
}

/// Emit ONE live-set action toward `desired`: select the first desired-but-
/// unselected card, else deselect any selected-but-not-desired card, else
/// confirm. Stable across calls within a phase.
pub fn emit_live_set(gs: &GameState, actions: &[Action], desired: &[usize]) -> Action {
    let selected = live_selected_indices(gs);
    for &hi in desired {
        if !selected.contains(&hi) {
            if let Some(a) = find_live_select(actions, hi, false) {
                return a;
            }
        }
    }
    for &hi in &selected {
        if !desired.contains(&hi) {
            if let Some(a) = find_live_select(actions, hi, true) {
                return a.clone();
            }
        }
    }
    confirm_live_set(actions)
}

/// Emit ONE mulligan action toward `discard`: select the first discard-but-
/// unselected card, else deselect any selected-but-not-discarded card, else
/// confirm/skip.
pub fn emit_mulligan(gs: &GameState, actions: &[Action], discard: &[usize]) -> Action {
    let selected = mulligan_selected_indices(gs);
    for &hi in discard {
        if !selected.contains(&hi) {
            if let Some(a) = find_mulligan_select(actions, hi, false) {
                return a;
            }
        }
    }
    for &hi in &selected {
        if !discard.contains(&hi) {
            if let Some(a) = find_mulligan_select(actions, hi, true) {
                return a.clone();
            }
        }
    }
    actions
        .iter()
        .find(|a| {
            matches!(
                a.action_type,
                ActionType::ConfirmMulligan | ActionType::SkipMulligan
            )
        })
        .or_else(|| actions.first())
        .cloned()
        .unwrap_or_else(|| empty_action_fallback("mulligan", ActionType::SkipMulligan))
}
