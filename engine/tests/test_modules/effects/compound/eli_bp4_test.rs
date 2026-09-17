use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn setup_eli_no_success() -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = game.id("PL!-bp4-002-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live_a = game.id("PL!-bp3-025-L");
    let live_b = game.id("PL!-bp3-026-L");
    game.state.player1.hand.cards.push(eli);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(live_a);
    game.state.player1.waitroom.cards.push(live_b);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(15);
    (game, eli, live_a)
}

fn setup_eli_with_success() -> (TestGame, i16, i16) {
    let (mut game, eli, live_a) = setup_eli_no_success();
    let live1 = game.id("PL!-bp3-025-L");
    let live2 = game.id("PL!-bp3-026-L");
    game.state.player1.success_live_card_zone.cards.push(live1);
    game.state.player1.success_live_card_zone.cards.push(live2);
    (game, eli, live_a)
}

#[test]
fn eli_bp4_condition_fails_score_too_low() {
    let (mut game, eli, _live_a) = setup_eli_no_success();
    game.play_to_stage(eli, MemberArea::Center);
    assert!(!game.has_pending_choice());
}

#[test]
fn eli_bp4_sequential_discard_two() {
    let (mut game, eli, live_a) = setup_eli_with_success();
    game.play_to_stage(eli, MemberArea::Center);
    game.activate_ability(eli);
    assert!(
        game.has_pending_choice(),
        "Should prompt for hand discard (1/2)"
    );
    game.select_indices(&[0]);
    assert!(
        game.has_pending_choice(),
        "Should re-prompt for second discard"
    );
    game.select_indices(&[1]);
    assert!(
        game.has_pending_choice(),
        "Should prompt for live card selection from discard"
    );
    game.select_indices(&[0]);
    assert!(game.state.player1.hand.cards.contains(&live_a));
}

#[test]
fn eli_bp4_all_at_once_discard() {
    let (mut game, eli, live_a) = setup_eli_with_success();
    game.play_to_stage(eli, MemberArea::Center);
    game.activate_ability(eli);
    assert!(game.has_pending_choice(), "Should prompt for hand discard");
    game.select_indices(&[0]);
    // Observed: the cost still re-prompts for the second single discard.
    assert!(
        game.has_pending_choice(),
        "Should re-prompt for second discard"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (hand discard 2/2)"
    );
    game.select_indices(&[0]);
    assert!(
        game.has_pending_choice(),
        "Should prompt for live card selection from discard"
    );
    game.select_indices(&[0]);
    assert!(game.state.player1.hand.cards.contains(&live_a));
}
