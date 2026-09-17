use crate::helpers::*;

#[test]
fn yohane_opponent_discards_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yohane = game.id("PL!S-pb1-006-R");
    let live_hand = game.id("PL!-sd1-019-SD");
    game.state.player1.stage.stage = [yohane, -1, -1];
    game.state.player1.hand.cards.push(live_hand);
    game.state.player2.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.state.player2.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.give_energy(15);
    game.activate_ability(yohane);
    // Cost is reveal live from hand, but the pending we get is opponent discard
    // (the reveal cost is auto-handled as part of cost payment)
    assert!(game.has_pending_choice(), "opponent should be prompted to discard, got {:?}", game.get_pending_choice());
    game.select_indices(&[0]); // opponent discards
    game.drain_auto_ability_choices();
    assert_eq!(game.state.mods.get_blade_modifier(yohane), 0, "opponent discarded -> no blade");
}

#[test]
fn yohane_opponent_declines_gets_blade4() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yohane = game.id("PL!S-pb1-006-R");
    let live_hand = game.id("PL!-sd1-019-SD");
    game.state.player1.stage.stage = [yohane, -1, -1];
    game.state.player1.hand.cards.push(live_hand);
    game.state.player2.hand.cards.push(game.id("PL!-sd1-010-SD"));
    game.give_energy(15);
    game.activate_ability(yohane);
    assert!(game.has_pending_choice(), "opponent should be prompted, got {:?}", game.get_pending_choice());
    game.select_indices(&[]); // opponent declines
    game.drain_auto_ability_choices();
    assert_eq!(game.state.mods.get_blade_modifier(yohane), 4, "opponent declined -> blade+4");
}
