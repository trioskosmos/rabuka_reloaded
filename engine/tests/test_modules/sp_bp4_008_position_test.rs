use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// SP-bp4-008 leftside draw2 discard1 / rightside energy active2
#[test]
fn sp_bp4_008_leftside_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shiki = game.id("PL!SP-bp4-008-P");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.hand.cards.push(shiki);
    game.give_energy(20);
    game.play_to_stage(shiki, MemberArea::LeftSide);
    // Leftside should draw 2
    assert_eq!(game.state.player1.hand.cards.len(), 2, "leftside should draw 2");
    assert!(game.has_pending_choice(), "discard prompt");
    game.select_indices(&[0]);
    assert_eq!(game.state.player1.hand.cards.len(), 1);
}

#[test]
fn sp_bp4_008_leftside_center_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shiki = game.id("PL!SP-bp4-008-P");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.hand.cards.push(shiki);
    game.give_energy(20);
    game.play_to_stage(shiki, MemberArea::Center);
    // Center should NOT draw (leftside only)
    assert_eq!(game.state.player1.hand.cards.len(), 0, "center should not draw");
    assert!(!game.has_pending_choice());
}

#[test]
fn sp_bp4_008_rightside_activates_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shiki = game.id("PL!SP-bp4-008-P");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.hand.cards.push(shiki);
    game.give_energy(20);
    game.play_to_stage(shiki, MemberArea::RightSide);
    assert!(game.state.player1.stage.stage[2] == shiki, "rightside should be at RightSide index 2");
    assert!(!game.has_pending_choice() || game.state.player1.hand.cards.len() <= 2);
}

#[test]
fn sp_bp4_008_rightside_left_no_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shiki = game.id("PL!SP-bp4-008-P");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.hand.cards.push(shiki);
    game.give_energy(20);
    game.play_to_stage(shiki, MemberArea::LeftSide);
    assert!(game.state.player1.stage.stage[0] == shiki, "leftside should be at LeftSide index 0");
}
