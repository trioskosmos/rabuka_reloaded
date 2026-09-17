use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn setsuna_pl_n_sd2_007_p_extra_draw_when_opponent_also_succeeded() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = game.id("PL!N-sd2-007-P");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.add_to_hand(setsuna);
    game.add_to_hand(game.new_id("PL!-sd1-010-SD"));

    game.state.p2_live_success_this_turn = true;
    game.state.p2_live_success_no_excess = true;

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(
        &mut game,
        setsuna,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert!(game.has_pending_choice(), "hand discard prompt expected");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard"
    );
    game.select_indices(&[0]);

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        2,
        "opponent succeeded too -> base draw + extra draw"
    );
}

#[test]
fn setsuna_pl_n_sd2_007_p_single_draw_without_opponent_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let setsuna = game.id("PL!N-sd2-007-P");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.add_to_hand(setsuna);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(
        &mut game,
        setsuna,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "opponent did not succeed -> only the base draw"
    );
}
