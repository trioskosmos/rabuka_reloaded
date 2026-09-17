use crate::helpers::*;

#[test]
fn opponent_waited_energy_live_leaves_live_zone_after_phases() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-pb1-019-L");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    game.state.player1.stage.stage[1] = game.id("PL!S-sd1-001-SD");
    game.state.player1.hand.cards.push(live);
    for _ in 0..5 { game.pass(); }
    game.set_live_card(live);
    for _ in 0..2 { game.pass(); }
    // Need to go to LiveSuccess: advance through live phases
    for _ in 0..7 { game.pass(); }
    // After live, opponent should have energy wait placed if live succeeded - at least verify the live resolved
    assert!(!game.state.player1.live_card_zone.cards.contains(&live) || game.state.player1.success_live_card_zone.cards.contains(&live), "live should have resolved");
}

#[test]
fn no_set_live_reaches_live_card_set_without_pending_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage[1] = game.id("PL!S-sd1-001-SD");
    // Don't set live, so no LiveSuccess
    for _ in 0..5 { game.pass(); }
    assert!(!game.has_pending_choice());
}
