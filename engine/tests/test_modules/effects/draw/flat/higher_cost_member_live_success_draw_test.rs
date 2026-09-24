use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::turn::TurnEngine;

const FILLER: &str = "PL!-sd1-010-SD";
const HIGHER_COST_MEMBER: &str = "PL!HS-bp5-004-R";
const LOWER_COST_MEMBER: &str = "PL!SP-PR-007-PR";
const EQUAL_COST_MEMBER: &str = "PL!-sd1-014-SD";
const KOSUZU_DECK_CARDS: [&str; 4] = [
    "PL!-sd1-010-SD",
    "PL!S-bp2-011-N",
    "PL!S-bp2-012-N",
    "PL!S-bp2-013-N",
];

#[test]
fn pb1013_higher_cost_member_on_stage_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-pb1-013-R"); // cost 9
    let big = game.id(HIGHER_COST_MEMBER); // cost 15
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = big;
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "a member costing more than 9 is on stage -> draw 1"
    );
    assert!(
        game.state.player1.hand.cards.contains(&drawn),
        "the drawn card is the one stocked on the deck"
    );
}

#[test]
fn pb1013_equal_or_lower_cost_members_do_not_draw() {
    for other_card_no in [LOWER_COST_MEMBER, EQUAL_COST_MEMBER] {
        let db = load_real_database();
        let mut game = TestGame::new(db);
        let me = game.id("PL!HS-pb1-013-R");
        let other = game.id(other_card_no);
        let undrawn = game.new_id(FILLER);
        game.state.player1.stage.stage[0] = me;
        game.state.player1.stage.stage[1] = other;
        game.state.player1.main_deck.cards.push(undrawn);
        let hand_before = game.state.player1.hand.cards.len();
        let deck_before = game.state.player1.main_deck.cards.len();

        fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

        assert_eq!(game.state.player1.hand.cards.len(), hand_before);
        assert_eq!(game.state.player1.main_deck.cards.len(), deck_before);
        assert!(game.state.player1.main_deck.cards.contains(&undrawn));
    }
}

fn trigger_kosuzu_live_start() -> (TestGame, [i16; 4]) {
    let mut game = TestGame::new(load_real_database());
    let kosuzu = game.id("PL!HS-pb1-013-R");
    assert_eq!(game.db.get_card(kosuzu).unwrap().card_no, "PL!HS-pb1-013-R");
    let cards: [i16; 4] = std::array::from_fn(|index| game.new_id(KOSUZU_DECK_CARDS[index]));
    let opponent_card = game.id("PL!-sd1-014-SD");
    game.state.player1.main_deck.cards = cards.to_vec().into();
    game.state.player2.main_deck.cards = vec![opponent_card].into();
    game.state.player1.stage.stage[1] = kosuzu;

    TurnEngine::trigger_live_start_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");
    game.assert_select_card("looked_at", 2, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &cards[..2]);
    (game, cards)
}

#[test]
fn kosuzu_live_start_can_keep_zero_cards() {
    let (mut game, [a, b, c, d]) = trigger_kosuzu_live_start();
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[c, d]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a, b]);
}

#[test]
fn kosuzu_live_start_can_keep_one_card() {
    let (mut game, [a, b, c, d]) = trigger_kosuzu_live_start();
    game.select_indices(&[1]);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[b, c, d]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a]);
}

#[test]
fn kosuzu_live_start_can_reorder_both_cards() {
    let (mut game, [a, b, c, d]) = trigger_kosuzu_live_start();
    game.select_indices(&[0]);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[b, a, c, d]);
    assert!(game.state.player1.waitroom.cards.is_empty());
}
