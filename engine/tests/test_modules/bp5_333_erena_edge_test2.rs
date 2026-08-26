use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn erena_p_variant_wait_gains_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let erena = game.id("PL!-bp5-333-P＋");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [erena, -1, -1];
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..40 { game.state.player2.main_deck.cards.push(filler); }
    game.state.mods.add_orientation_modifier(erena, "wait");
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(erena, HeartColor::Heart05), 1);
}

#[test]
fn erena_wait_then_active_loses_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let erena = game.id("PL!-bp5-333-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [erena, -1, -1];
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..40 { game.state.player2.main_deck.cards.push(filler); }
    game.state.mods.add_orientation_modifier(erena, "wait");
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(erena, HeartColor::Heart05), 1);
    game.state.mods.add_orientation_modifier(erena, "active");
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(erena, HeartColor::Heart05), 0);
}
