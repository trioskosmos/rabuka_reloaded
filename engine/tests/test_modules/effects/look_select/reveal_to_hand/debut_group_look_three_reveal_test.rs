use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn debut_group_look(target: Option<&str>) -> (TestGame, [i16; 4]) {
    let mut game = TestGame::new(load_real_database());
    let member = game.id("PL!N-sd2-009-SD2");
    assert_eq!(game.db.get_card(member).unwrap().card_no, "PL!N-sd2-009-SD2");
    let cards = [
        game.id("PL!-sd1-010-SD"),
        game.id(target.unwrap_or("PL!-sd1-010-SD")),
        game.id("PL!S-sd1-001-SD"),
        game.id("PL!-sd1-010-SD"),
    ];
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards.push(opponent);
    game.add_to_hand(member);
    game.give_energy(11);
    game.play_to_stage(member, MemberArea::Center);
    assert_eq!(game.state.player1.stage.stage[1], member);
    (game, cards)
}

#[test]
fn debut_look_three_reveals_group_member_and_discards_other_cards() {
    let (mut game, [a, b, c, d]) = debut_group_look(Some("PL!N-bp1-004-R"));
    game.assert_select_card("looked_at", 1, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &[a, b, c]);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[b]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a, c]);
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[d]);
}

#[test]
fn debut_look_three_can_reveal_group_live_not_only_member() {
    let (mut game, [a, b, c, d]) = debut_group_look(Some("PL!N-sd1-019-SD"));
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[b]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a, c]);
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[d]);
}

#[test]
fn debut_look_three_decline_discards_all_looked_cards() {
    let (mut game, [a, b, c, d]) = debut_group_look(Some("PL!N-bp1-004-R"));
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a, b, c]);
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[d]);
}

#[test]
fn debut_look_three_no_group_card_discards_all_without_pick() {
    let (game, [a, b, c, d]) = debut_group_look(None);
    assert!(!game.has_pending_choice());
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a, b, c]);
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[d]);
}
