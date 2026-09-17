use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn pl_s_bp7_010_n_accept_bottom_card_to_fourth_from_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp7-010-N");
    game.add_to_stage(MemberArea::Center, chika);
    let x = game.id("PL!S-sd1-001-SD");
    let mut deck_cards: Vec<i16> = Vec::new();
    for _ in 0..4 {
        let f = game.new_id(FILLER);
        deck_cards.push(f);
        game.state.player1.main_deck.cards.push(f);
    }
    game.state.player1.main_deck.cards.push(x);
    fire_trigger(&mut game, chika, AbilityTrigger::Debut, "登場");
    if !game.has_pending_choice() {
        panic!("expected the optional move-to-4th choice");
    }
    eprintln!("[CHIKA] choice: {}", game.pending_choice_summary());
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(1),
        Some("SelectCard") => game.select_indices(&[0]),
        other => panic!("unexpected {other:?}"),
    }
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert_eq!(game.state.player1.main_deck.cards.get(3), Some(&x), "X now sits 4th from the TOP (index 3)");
    assert_ne!(game.state.player1.main_deck.cards.last(), Some(&x), "X left the bottom");
}

#[test]
fn pl_s_bp7_010_n_decline_keeps_bottom_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp7-010-N");
    game.add_to_stage(MemberArea::Center, chika);
    let x = game.id("PL!S-sd1-001-SD");
    game.state.player1.main_deck.cards.push(x);
    fire_trigger(&mut game, chika, AbilityTrigger::Debut, "登場");
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[]),
            _ => break,
        }
    }
    assert!(game.state.player1.main_deck.cards.contains(&x), "declined → X must STAY in the deck (5.7.1: 見る only informs)");
    assert_eq!(game.state.player1.main_deck.cards.last(), Some(&x), "declined → X stays at the BOTTOM");
}
