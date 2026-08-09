//! Shared helpers used by the standalone engine binaries (engine/src/bin/*.rs).
//!
//! These are thin orchestration conveniences: they wrap engine primitives
//! (`build_two_decks`, `execute_action`, `settle_single_player_state`, ...)
//! that were previously inlined (with slight variations) into each bin.

use std::sync::Arc;

use crate::card::CardDatabase;
use crate::deck_builder::Deck;
use crate::game_setup;
use crate::game_state::GameState;
use crate::player::Player;

/// Shuffle fresh copies of the two template decks, build two players, and set up
/// a new game. `p1_id`/`p2_id` are the load-bearing player ids (e.g. `"p1"` vs
/// `"player1"` differ across bins), `p1_name`/`p2_name` are display names.
pub fn deal_game(
    db: &Arc<CardDatabase>,
    t1: &Deck,
    t2: &Deck,
    p1_id: &str,
    p1_name: &str,
    p2_id: &str,
    p2_name: &str,
) -> GameState {
    let mut d1 = t1.clone();
    d1.shuffle_main_deck();
    d1.shuffle_energy_deck();
    let mut d2 = t2.clone();
    d2.shuffle_main_deck();
    d2.shuffle_energy_deck();

    let mut p1 = Player::new(p1_id.to_string(), p1_name.to_string(), true);
    let mut p2 = Player::new(p2_id.to_string(), p2_name.to_string(), false);
    p1.set_main_deck(d1.main_deck);
    p1.set_energy_deck(d1.energy_deck);
    p2.set_main_deck(d2.main_deck);
    p2.set_energy_deck(d2.energy_deck);

    let mut gs = GameState::new(p1, p2, Arc::clone(db));
    game_setup::setup_game(&mut gs);
    gs
}

/// Execute an action extracted from `action.parameters`, then settle all
/// automatic phases. Mirrors the per-bin inline block. Returns the engine
/// result (the existing per-bin blocks discard it with `let _`).
pub fn execute_and_settle(gs: &mut GameState, action: &game_setup::Action) -> Result<(), String> {
    let res = game_setup::execute_action(gs, action);
    game_setup::settle_single_player_state(gs);
    res
}

/// Win/loss outcome derived from the success zones + engine game result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOutcome {
    P1Win,
    P2Win,
    Draw,
    Stuck,
}

/// The `p1z>=3 && p2z<=2 => P1` / mirror P2 / draw rule that several bins inline.
pub fn classify_winner(gs: &GameState) -> GameOutcome {
    let p1z = gs.player1.success_live_card_zone.cards.len();
    let p2z = gs.player2.success_live_card_zone.cards.len();
    if p1z >= 3 && p2z <= 2 {
        GameOutcome::P1Win
    } else if p2z >= 3 && p1z <= 2 {
        GameOutcome::P2Win
    } else {
        GameOutcome::Draw
    }
}