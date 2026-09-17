//! Regression: `execute_action` must forward `ability_index` so that a
//! member with two simultaneously activatable abilities executes the
//! REQUESTED one, not the first eligible one (both bots' simulation and
//! real arena play route through this wrapper).
//!
//! Fixture: PL!N-bp7-006-SEC (近江彼方) has two 起動 abilities:
//! - ab#0: pay 1 energy → look at top 4, put them back on deck (no zone loss)
//! - ab#1: mill top 3 cards to waitroom (no energy) → conditional choice
//! The zone deltas of each are mutually exclusive fingerprints.

use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};
use rabuka_engine::game_state::Phase;

fn setup_dual_activation() -> (TestGame, i16) {
    let mut game = TestGame::new(load_real_database());
    let kanata = game.id("PL!N-bp7-006-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [kanata, -1, -1];
    game.give_energy(10);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.current_phase = Phase::Main;
    (game, kanata)
}

fn find_use_ability(
    actions: &[rabuka_engine::game_setup::Action],
    kanata: i16,
    idx: usize,
) -> usize {
    actions
        .iter()
        .position(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(kanata)
                && a.parameters.as_ref().and_then(|p| p.ability_index) == Some(idx)
        })
        .unwrap_or_else(|| panic!("UseAbility action for ability_index {idx} not offered"))
}

#[test]
fn execute_action_honors_ability_index_for_dual_activation_member() {
    // ab#1 action must mill 3 and leave energy untouched. Before the fix,
    // the wrapper dropped the index and executed ab#0 instead (energy -1,
    // no mill) — exactly what these assertions catch.
    let (mut game, kanata) = setup_dual_activation();
    let actions = game_setup::generate_possible_actions(&game.state);
    let deck_before = game.state.player1.main_deck.cards.len();
    let wr_before = game.state.player1.waitroom.cards.len();
    let energy_before = game.state.player1.energy_zone.active_count();

    game_setup::execute_action(&mut game.state, &actions[find_use_ability(&actions, kanata, 1)])
        .expect("ab#1 executes");

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "ab#1 must mill 3 deck-top cards"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        wr_before + 3,
        "ab#1 milled cards land in the waitroom"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before,
        "ab#1 costs no energy (its cost is the mill)"
    );

    // ab#0 action must pay 1 energy and never mill.
    let (mut game2, kanata2) = setup_dual_activation();
    let actions2 = game_setup::generate_possible_actions(&game2.state);
    let deck_before2 = game2.state.player1.main_deck.cards.len();
    let wr_before2 = game2.state.player1.waitroom.cards.len();
    let energy_before2 = game2.state.player1.energy_zone.active_count();

    game_setup::execute_action(&mut game2.state, &actions2[find_use_ability(&actions2, kanata2, 0)])
        .expect("ab#0 executes");
    // look_at leaves an "choose order" pending choice; the 4 cards return
    // to the deck only after resolving it.
    while game2.has_pending_choice() {
        game2.select_indices(&[0]);
    }

    assert_eq!(
        game2.state.player1.energy_zone.active_count(),
        energy_before2 - 1,
        "ab#0 pays exactly 1 energy"
    );
    assert_eq!(
        game2.state.player1.waitroom.cards.len(),
        wr_before2,
        "ab#0 must not mill to the waitroom"
    );
    assert_eq!(
        game2.state.player1.main_deck.cards.len(),
        deck_before2,
        "ab#0 looks at 4 but returns them to the deck"
    );
}
