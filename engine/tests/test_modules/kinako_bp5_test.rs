/// Tests for PL!SP-bp5-006-R (桜小路きな子) ab#0 — Q234
///
/// Ability (起動[ターン1]):
///   デッキの上からカードを3枚控え室に置く：このメンバーはポジションチェンジする。
///
/// Q234: デッキが2枚以下の状態でも起動コストを支払えるか？
/// Answer: いいえ、3枚以上必要。
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

/// Kinako at LeftSide, another member at Center. Deck has 4 cards.
/// Activate → cost of 3 discarded → position change creates choice.
/// Pick RightSide → Kinako moves from LeftSide(0) to RightSide(2).
#[test]
fn kinako_bp5_q234_deck_4_activation_succeeds_position_changes() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kinako = game.id("PL!SP-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");
    let center_member = game.id("PL!-sd1-013-SD");

    // Kinako at LeftSide, other member at Center
    game.state.player1.stage.stage = [kinako, center_member, -1];

    // 4 cards in deck (enough for cost of 3)
    for _ in 0..4 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.activate_ability(kinako);

    // Cost: 3 cards discarded from deck
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        1,
        "3 cards should be discarded as activation cost from 4-card deck"
    );

    // Position change creates a choice (source_position unspecified → choose dest)
    assert!(
        game.has_pending_choice(),
        "Position change should create a destination choice after cost payment"
    );

    // Choose RightSide as destination
    game.select_option(2);

    // Kinako moves from LeftSide(0) to empty RightSide(2)
    assert_eq!(
        game.state.player1.stage.stage[0], -1,
        "LeftSide should be empty after Kinako moved"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], kinako,
        "Q234: Kinako should be at RightSide after position change"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], center_member,
        "Center member unchanged"
    );
}

/// Deck has 2 cards (< 3 needed). Activation fails → no cost paid,
/// no position change, no pending choice.
#[test]
fn kinako_bp5_q234_deck_2_activation_fails_no_movement() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kinako = game.id("PL!SP-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");

    // Kinako at Center, so position would be visible
    game.state.player1.stage.stage = [-1, kinako, -1];

    // Only 2 cards in deck (need 3+)
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);

    // Activate — cost fails (logs "Cannot pay cost: deck_top has only 2...")
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kinako),
        None,
        None,
        None,
    )
    .expect("activation request returns Ok even when cost fails");

    // Q234: No cost was paid
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        2,
        "Q234: No cards should be discarded when deck < 3"
    );

    // No pending choice (no position change triggered)
    assert!(
        !game.has_pending_choice(),
        "Q234: No position change choice when activation fails"
    );

    // Kinako position unchanged
    assert_eq!(
        game.state.player1.stage.stage[1], kinako,
        "Q234: Kinako should remain at original position"
    );
}
