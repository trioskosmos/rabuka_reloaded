use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const MARI: &str = "PL!S-bp7-008-R";

fn debut() -> (TestGame, [i16; 5]) {
    let mut game = TestGame::new(load_real_database());
    let cards = std::array::from_fn(|_| game.id("PL!-sd1-010-SD"));
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let mari = game.id(MARI);
    game.add_to_hand(mari);
    game.give_energy(20);
    game.play_to_stage(mari, MemberArea::Center);
    assert!(game.state.player1.stage.stage.contains(&mari));
    assert!(!game.state.player1.hand.cards.contains(&mari));
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    assert_eq!(game.state.looked_at_cards.as_slice(), &cards[..3]);
    (game, cards)
}

#[test]
fn mari_keeps_one_on_top_rest_to_deck_bottom() {
    let (mut game, [a, b, c, d, e]) = debut();
    game.select_indices(&[1]);
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[b, d, e, a, c]);
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert!(game.state.player1.hand.cards.is_empty());
}

#[test]
fn mari_keeps_two_on_top_one_to_deck_bottom() {
    let (mut game, [a, b, c, d, e]) = debut();
    game.select_indices(&[0]);
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[0]);
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[b, a, d, e, c]);
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert!(game.state.player1.hand.cards.is_empty());
}
