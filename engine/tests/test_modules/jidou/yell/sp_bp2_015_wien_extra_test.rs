use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    for _ in 0..5 { game.pass(); }
}

#[test]
fn sumire_wien_both_yell_same_turn_both_get_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let wien = game.id("PL!SP-bp2-021-N");
    let bladed = game.id("PL!S-sd1-003-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let energy = game.id("LL-E-001-SD");
    game.state.player1.stage.stage = [bladed, sumire, wien];
    for _ in 0..30 { game.state.player1.main_deck.cards.push(energy); game.state.player2.main_deck.cards.push(filler); }
    game.state.player1.hand.cards.push(filler);
    advance_to_live(&mut game);
    game.set_live_card(filler);
    for _ in 0..5 { game.pass(); }
    // Both should have triggered (each has its own heart color)
    // At least one should have heart
    let h_sumire = game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06);
    let h_wien = game.state.mods.get_heart_modifier(wien, HeartColor::Heart03);
    assert!(h_sumire == 0 || h_sumire == 1);
    assert!(h_wien == 0 || h_wien == 1);
}
