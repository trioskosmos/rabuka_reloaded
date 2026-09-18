use crate::helpers::*;

#[test]
fn live_success_optional_three_energy_pay_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-pb1-004-R");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    game.state.player1.stage.stage[1] = sumire;
    game.give_energy(5);
    let hand_before = game.state.player1.hand.cards.len();
    crate::helpers::fire_trigger(&mut game, sumire, rabuka_engine::core::types::AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice(), "LiveSuccess should present pay 3E choice");
    game.select_option(1); // Pay is option 1
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    assert_eq!(game.state.player1.hand.cards.len(), hand_before + 1, "should draw 1 on pay");
    assert_eq!(game.state.player1.energy_zone.active_count(), 2, "5-3=2 active left");
}

#[test]
fn live_success_optional_three_energy_skip_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-pb1-004-R");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    game.state.player1.stage.stage[1] = sumire;
    game.give_energy(5);
    let hand_before = game.state.player1.hand.cards.len();
    crate::helpers::fire_trigger(&mut game, sumire, rabuka_engine::core::types::AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice());
    game.select_option(0); // Skip is option 0
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    assert_eq!(game.state.player1.hand.cards.len(), hand_before, "skip should not draw");
    assert_eq!(game.state.player1.energy_zone.active_count(), 5, "skip should not pay");
}
