/// Tests for PL!SP-bp5-006-R (桜小路きな子) ab#0 — Q234 + Q104
///
/// Ability (起動[ターン1]):
///   デッキの上からカードを3枚控え室に置く：このメンバーはポジションチェンジする。
///
/// Q234: デッキが2枚以下の状態でも起動コストを支払えるか？
/// Answer: いいえ、3枚以上必要 (original answer, but Q104 overrides:
///   deck < needed → take available, refresh, take remaining)
///
/// Q104: デッキが足りない場合、足りる分を控え室に置いた後、リフレッシュを行い、
///   残りを新たなデッキから控え室に置く。
///
/// Rule 10.2.1: リフレッシュはチェックタイミングにかぎらず、ゲーム中の任意の時点で
///   いずれかのプレイヤーが条件を満たしている場合に実行します。それがなんらかの処理
///   の途中である場合、その処理を一時中断し、リフレッシュを実行した後に、その処理の
///   続きを実行します。
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

/// Deck has 2 cards (< 3 needed), waitroom has cards.
/// Per Q104: take 2 from deck, refresh, take remaining 1 from new deck.
/// Cost succeeds → position change choice appears.
#[test]
fn kinako_bp5_q104_deck_2_refresh_and_continue() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kinako = game.id("PL!SP-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");

    // Kinako at LeftSide
    game.state.player1.stage.stage = [kinako, -1, -1];

    // 2 cards in deck (need 3), plus 3 cards already in waitroom
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);

    game.activate_ability(kinako);

    // Q104: 2 taken from deck → refresh → 1 taken from new deck
    // Waitroom should have 1 card (the 3rd one drawn from refreshed deck)
    // The 2 original deck cards were discarded then reshuffled back → back in deck
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "Q104: 1 card from refreshed deck should be in waitroom"
    );
    // 2 original + 3 waitroom = 5 cards, took 3, so 2 remain in deck + 2 from
    // original discarded and reshuffled = 4
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        4,
        "Q104: 4 cards should remain in deck after refresh"
    );

    // Position change choice should appear
    assert!(
        game.has_pending_choice(),
        "Q104: Position change should appear after cost payment with refresh"
    );
    // Complete the choice
    game.select_option(2);

    // Kinako moved
    assert_eq!(
        game.state.player1.stage.stage[0], -1,
        "LeftSide should be empty after Kinako moved"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], kinako,
        "Kinako should be at RightSide"
    );
}

/// Both deck AND waitroom are empty → cost truly cannot be paid.
/// Activation fails, no position change.
#[test]
fn kinako_bp5_q104_both_deck_and_waitroom_empty_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kinako = game.id("PL!SP-bp5-006-R");

    // Kinako at Center
    game.state.player1.stage.stage = [-1, kinako, -1];

    // Both deck and waitroom are empty
    game.state.player1.main_deck.cards.clear();
    game.state.player1.waitroom.cards.clear();
    assert_eq!(game.state.player1.main_deck.cards.len(), 0);
    assert_eq!(game.state.player1.waitroom.cards.len(), 0);

    // Activate — cost fails
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kinako),
        None,
        None,
        None,
    )
    .expect("activation request returns Ok even when cost fails");

    // No cost paid, no position change
    assert!(
        !game.has_pending_choice(),
        "No position change choice when both deck and waitroom empty"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], kinako,
        "Kinako should remain at original position"
    );
}
