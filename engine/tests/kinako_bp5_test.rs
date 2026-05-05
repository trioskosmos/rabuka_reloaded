/// Tests for PL!SP-bp5-006-R (桜小路きな子) ab#0 — Q234
///
/// Ability (起動[ターン1]):
///   デッキの上からカードを3枚控え室に置く：このメンバーはポジションチェンジする。
///
/// Q234: デッキが2枚以下の状態でも起動コストを支払えるか？
/// Answer: いいえ、3枚以上必要。

mod helpers;
use helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

/// Kinako on stage with only 2 cards in deck → activation should fail.
#[test]
fn kinako_bp5_q234_activation_fails_with_2_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kinako = game.id("PL!SP-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");

    // Kinako on stage
    game.state.player1.stage.stage[1] = kinako;

    // Only 2 cards in deck (need 3+)
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);

    // Q234: activation should fail with < 3 cards in deck
    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kinako), None, None, None,
    );
    assert!(result.is_err(), "Q234: Should fail when deck has < 3 cards");
    assert!(result.unwrap_err().contains("deck") || result.unwrap_err().contains("card"),
        "Error should mention insufficient deck cards");
}

/// Kinako on stage with 4 cards in deck → activation should succeed,
/// discarding 3 cards and triggering position change.
#[test]
fn kinako_bp5_q234_activation_succeeds_with_4_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kinako = game.id("PL!SP-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");

    // Kinako on stage
    game.state.player1.stage.stage[1] = kinako;

    // 4 cards in deck (enough for cost of 3)
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);

    let deck_before = game.state.player1.main_deck.cards.len();

    game.activate_ability(kinako);

    // 3 cards should be discarded as cost
    let deck_after = game.state.player1.main_deck.cards.len();
    assert_eq!(deck_before - deck_after, 3,
        "Activation cost should discard 3 cards from deck");
}

/// Verify the parser extracted activation ability with deck discard cost.
#[test]
fn kinako_bp5_q234_parser_fields() {
    let db = load_real_database();
    let card = db.get_card_by_no("PL!SP-bp5-006-R").expect("Kinako exists");
    let ab = &card.abilities[0];

    assert_eq!(ab.triggers.as_deref(), Some("起動"));
    if let Some(ref effect) = ab.effect {
        assert_eq!(effect.action, "position_change");
    }
    if let Some(ref cost) = ab.cost {
        assert_eq!(cost.cost_type.as_deref(), Some("move_cards"));
        assert_eq!(cost.source.as_deref(), Some("deck_top"));
        assert_eq!(cost.count, Some(3));
        assert_eq!(cost.destination.as_deref(), Some("discard"));
    }
}
