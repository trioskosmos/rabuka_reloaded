use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn surplus_reorder(surplus: u8) -> (TestGame, [i16; 4]) {
    let mut game = TestGame::new(load_real_database());
    let live = game.id("PL!HS-bp6-028-L");
    assert_eq!(game.db.get_card(live).unwrap().card_no, "PL!HS-bp6-028-L");
    let cards = std::array::from_fn(|_| game.id("PL!-sd1-010-SD"));
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards.push(opponent);
    game.state.player1.live_card_zone.cards.push(live);
    game.state.live_surplus_ready_this_turn = true;
    game.state.self_live_surplus_count = surplus;
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    (game, cards)
}

#[test]
fn surplus_one_reorder_partial_routes_remainder_to_waitroom() {
    let (mut game, [a, b, c, d]) = surplus_reorder(1);
    game.assert_select_card("looked_at", 2, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &[a, b]);
    game.select_indices(&[1]);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[b, c, d]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a]);
    assert!(game.state.player1.hand.cards.is_empty());
}

#[test]
fn surplus_one_reorder_full_supports_both_orders() {
    for first in 0..2 {
        let (mut game, [a, b, c, d]) = surplus_reorder(1);
        game.assert_select_card("looked_at", 2, true);
        game.select_indices(&[first]);
        game.assert_select_card("looked_at", 1, true);
        game.select_indices(&[0]);
        assert!(!game.has_pending_choice());
        let expected = if first == 0 { [b, a, c, d] } else { [a, b, c, d] };
        assert_eq!(game.state.player1.main_deck.cards.as_slice(), &expected);
        assert!(game.state.player1.waitroom.cards.is_empty());
        assert!(game.state.player1.hand.cards.is_empty());
    }
}

#[test]
fn surplus_one_reorder_skip_routes_both_to_waitroom() {
    let (mut game, [a, b, c, d]) = surplus_reorder(1);
    game.assert_select_card("looked_at", 2, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[c, d]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a, b]);
    assert!(game.state.player1.hand.cards.is_empty());
}

#[test]
fn surplus_zero_does_not_look_or_reorder() {
    let (game, cards) = surplus_reorder(0);
    assert!(!game.has_pending_choice());
    assert!(game.state.looked_at_cards.is_empty());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &cards);
    assert!(game.state.player1.waitroom.cards.is_empty());
}
