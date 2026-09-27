//! Shared helpers used by the standalone engine binaries (engine/src/bin/*.rs).
//!
//! These are thin orchestration conveniences: they wrap engine primitives
//! (`build_two_decks`, `execute_action`, `settle_single_player_state`, ...)
//! that were previously inlined (with slight variations) into each bin.

use std::path::Path;
use std::sync::Arc;

use crate::card::CardDatabase;
use crate::card_loader;
use crate::deck_builder::Deck;
use crate::deck_parser::DeckParser;
use crate::game_setup;
use crate::game_state::GameState;
use crate::player::Player;

/// Load the card database from `cards/cards.json`.
///
/// The path is relative to the `engine/` directory, which is where every tool in
/// `src/bin/` is run from.
pub fn fresh_database() -> Arc<CardDatabase> {
    let cards_path = Path::new("../cards/cards.json");
    let cards = card_loader::CardLoader::load_cards_from_file(cards_path).expect("load cards");
    Arc::new(CardDatabase::load_or_create(cards))
}

/// Read `web_ui/decks/<name>.txt` as a list of card numbers.
///
/// A missing deck is a hard error, not a fallback. Synthesizing a 60-card list
/// for an unknown name produces a plausible-looking game on a deck nobody asked
/// for, so every number measured from it silently describes the wrong match.
pub fn load_deck(name: &str) -> Vec<String> {
    let deck_path = Path::new("../web_ui/decks").join(format!("{name}.txt"));
    if !deck_path.exists() {
        panic!(
            "deck not found: {}\n\
             A missing deck is a hard error, not a fallback: every win rate measured \
             against a synthesized list describes a different game than the one you asked for.",
            deck_path.display()
        );
    }
    let deck = DeckParser::parse_deck_file(&deck_path).expect("parse deck");
    DeckParser::deck_list_to_card_numbers(&deck)
}

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

/// [`deal_game`] with the default `"p1"`/`"p2"` ids and `"P1"`/`"P2"` names,
/// which is what every offline tool and replay harness wants.
pub fn deal_default_game(db: &Arc<CardDatabase>, t1: &Deck, t2: &Deck) -> GameState {
    deal_game(db, t1, t2, "p1", "P1", "p2", "P2")
}

/// Execute an action extracted from `action.parameters`, then settle all
/// automatic phases. Mirrors the per-bin inline block. Returns the engine
/// result (the existing per-bin blocks discard it with `let _`).
pub fn execute_and_settle(gs: &mut GameState, action: &game_setup::Action) -> Result<(), String> {
    let _timer = crate::timer::Timer::start("bin_common::execute_and_settle");
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