//! Mulligan replacement semantics - the regression tests for a bug found by
//! playing a side manually against v7.
//!
//! The bug: `handle_mulligan_confirmation` advanced the mulligan phase BEFORE
//! mutating a hand, so `active_player_mut()` had already flipped to the other
//! seat. A player's own mulligan therefore removed the selected cards from
//! their OPPONENT's hand and dealt the replacements to the opponent. Observed
//! directly, acting as player1 and selecting hand indices [3, 2]:
//!
//! ```text
//!   my_hand  [2547, 2563, 2573, 2576, 2537, 2533]  -> unchanged
//!   opp_hand [2651, 2611, 2599, 2615, 2639, 2636]  -> [2651, 2611, 2639, 2636, 2648, 2609]
//! ```
//!
//! Why it mattered well beyond one game: the arena ablation reported v8's
//! mulligan as "neutral", and in a mirror match a mulligan that hits the
//! opponent is exactly what a no-op looks like - both seats corrupt each other
//! so the effect largely cancels and the measurement reports no difference.
//! Against a human it would be actively harmful.
//!
//! These are unit tests rather than an integration test on purpose. Reaching a
//! mulligan phase end-to-end means driving RPS for both seats, and
//! `auto_advance_one` does not resolve the second seat's mulligan, so an
//! integration test spends its budget fighting the setup dance. The behaviour
//! under test is one function, and the owner rule is one method.

use super::TurnEngine;
use crate::card::CardDatabase;
use crate::core::types::Phase;
use crate::game_state::GameState;
use crate::player::Player;
use std::sync::Arc;

fn test_db() -> Arc<CardDatabase> {
    let cards = crate::card_loader::CardLoader::load_cards_from_strs(include_str!(
        "../../../cards/cards.json"
    ))
    .expect("cards");
    Arc::new(CardDatabase::load_or_create(cards))
}

/// A minimal state parked in a mulligan phase, with player1 as first attacker
/// so the phase owner is derivable.
fn state_in(phase: Phase, p1_first: bool) -> GameState {
    let p1 = Player::new("p1".into(), "P1".into(), true);
    let p2 = Player::new("p2".into(), "P2".into(), false);
    let mut gs = GameState::new(p1, p2, test_db());
    gs.player1.is_first_attacker = p1_first;
    gs.current_phase = phase;
    gs
}

fn fill(p: &mut Player, cards: &[i16]) {
    p.hand.cards.clear();
    for &c in cards {
        p.hand.add_card(c);
    }
}

/// Record the selection the way two SelectMulligan actions would leave it.
fn select(gs: &mut GameState, indices: &[u8]) {
    gs.mulligan_selected_indices.clear();
    for &i in indices {
        gs.mulligan_selected_indices.push(i);
    }
}

fn hand(gs: &GameState, index: usize) -> Vec<i16> {
    let src = if index == 0 { &gs.player1 } else { &gs.player2 };
    src.hand.cards.iter().copied().collect()
}

#[test]
fn mulligan_replaces_the_choosing_players_cards_not_the_opponents() {
    let mut gs = state_in(Phase::MulliganFirstAttacker, true);
    assert_eq!(
        gs.mulligan_owner_index(),
        Some(0),
        "player1 is the first attacker, so player1 owns the first mulligan"
    );

    fill(&mut gs.player1, &[101, 102, 103, 104, 105, 106]);
    fill(&mut gs.player2, &[201, 202, 203, 204, 205, 206]);
    let before_own = gs.player1.hand.cards.clone();
    let before_opp = gs.player2.hand.cards.clone();

    // What two SelectMulligan actions leave behind.
    select(&mut gs, &[3, 2]);
    TurnEngine::handle_mulligan_confirmation(&mut gs, None).expect("confirm");

    assert_eq!(
        gs.player1.hand.cards.len(),
        before_own.len(),
        "a mulligan redraws exactly as many cards as it removes"
    );
    assert_ne!(
        gs.player1.hand.cards, before_own,
        "the choosing player's hand is unchanged, so the mulligan never applied to them"
    );
    assert_eq!(
        gs.player2.hand.cards, before_opp,
        "the OPPONENT's hand was modified by player1's mulligan"
    );
    assert!(
        gs.mulligan_selected_indices.is_empty(),
        "confirm must clear the selection, got {:?}",
        gs.mulligan_selected_indices
    );
}

#[test]
fn the_second_mulligan_phase_belongs_to_the_second_seat() {
    let mut gs = state_in(Phase::MulliganSecondAttacker, true);
    assert_eq!(
        gs.mulligan_owner_index(),
        Some(1),
        "player1 is first attacker, so player2 owns the second mulligan"
    );

    fill(&mut gs.player1, &[101, 102, 103, 104, 105, 106]);
    fill(&mut gs.player2, &[201, 202, 203, 204, 205, 206]);
    let before_own = gs.player2.hand.cards.clone();
    let before_opp = gs.player1.hand.cards.clone();

    select(&mut gs, &[1]);
    TurnEngine::handle_mulligan_confirmation(&mut gs, None).expect("confirm");

    assert_ne!(
        gs.player2.hand.cards, before_own,
        "the second seat's own mulligan must apply to it"
    );
    assert_eq!(
        gs.player1.hand.cards, before_opp,
        "the first seat must not be touched by the second seat's mulligan"
    );
}

/// The phase owner must follow turn order, not always be player1.
#[test]
fn mulligan_owner_follows_turn_order() {
    assert_eq!(state_in(Phase::MulliganFirstAttacker, true).mulligan_owner_index(), Some(0));
    assert_eq!(state_in(Phase::MulliganFirstAttacker, false).mulligan_owner_index(), Some(1));
    assert_eq!(state_in(Phase::MulliganSecondAttacker, true).mulligan_owner_index(), Some(1));
    assert_eq!(state_in(Phase::MulliganSecondAttacker, false).mulligan_owner_index(), Some(0));
    // Outside a mulligan phase there is no owner, which is what stops the
    // original bug: the handler could ask `active_player()` and get an answer
    // that had already flipped to the wrong seat.
    assert_eq!(state_in(Phase::Main, true).mulligan_owner_index(), None);
}

#[test]
fn an_empty_confirmation_changes_nothing() {
    let mut gs = state_in(Phase::MulliganFirstAttacker, true);
    fill(&mut gs.player1, &[101, 102, 103]);
    fill(&mut gs.player2, &[201, 202, 203]);
    let before_own = hand(&gs, 0);
    let before_opp = hand(&gs, 1);

    TurnEngine::handle_mulligan_confirmation(&mut gs, None).expect("confirm");

    assert_eq!(hand(&gs, 0), before_own, "an empty mulligan changes nothing");
    assert_eq!(hand(&gs, 1), before_opp);
}

#[test]
fn a_single_selected_card_is_replaced() {
    let mut gs = state_in(Phase::MulliganFirstAttacker, true);
    fill(&mut gs.player1, &[101, 102, 103, 104]);
    fill(&mut gs.player2, &[201, 202, 203, 204]);
    let before_own = gs.player1.hand.cards.clone();
    let before_opp = gs.player2.hand.cards.clone();

    select(&mut gs, &[2]);
    TurnEngine::handle_mulligan_confirmation(&mut gs, None).expect("confirm");

    assert_eq!(gs.player1.hand.cards.len(), before_own.len());
    assert_ne!(gs.player1.hand.cards, before_own);
    assert_eq!(gs.player2.hand.cards, before_opp);
}
