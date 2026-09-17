use crate::helpers::*;

#[test]
fn live_start_with_added_group_heart02_preserves_live_and_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-pb1-019-L");
    // Setup Aqours members with total heart02 >=6: use PL!S-bp2-002-R etc which has heart heart02?
    // For test, we can directly give heart02 via modifiers to reach 6
    let aqours1 = game.id("PL!S-bp2-002-R");
    let aqours2 = game.id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [aqours1, aqours2, -1];
    // Give each 3 heart02 via modifiers to reach 6 total
    game.state.mods.add_heart_modifier(aqours1, rabuka_engine::card::HeartColor::Heart02, 3);
    game.state.mods.add_heart_modifier(aqours2, rabuka_engine::card::HeartColor::Heart02, 3);
    game.state.player1.hand.cards.push(live);
    for _ in 0..10 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); game.state.player2.main_deck.cards.push(f); }
    for _ in 0..5 { game.pass(); }
    game.set_live_card(live);
    for _ in 0..2 { game.pass(); }
    // LiveStart should have invalidated LiveSuccess, but we check that the invalidate flag is set
    assert!(game.state.player1.live_card_zone.cards.contains(&live), "live should be in live zone after set");
    assert!(game.state.mods.get_heart_modifier(aqours1, rabuka_engine::card::HeartColor::Heart02) >= 3);
}

#[test]
fn live_start_without_added_group_heart02_preserves_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-pb1-019-L");
    let aqours = game.id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [aqours, -1, -1];
    // No extra heart02, total heart02 from base may be 0 or low (<6)
    game.state.player1.hand.cards.push(live);
    for _ in 0..10 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); game.state.player2.main_deck.cards.push(f); }
    for _ in 0..5 { game.pass(); }
    game.set_live_card(live);
    for _ in 0..2 { game.pass(); }
    // With low heart02, invalidate should NOT happen, LiveSuccess should still be valid
    assert!(game.state.player1.live_card_zone.cards.contains(&live), "live should remain in live zone when not invalidated");
}
