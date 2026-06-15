use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Negative tests for conditions that are NOT met.
// This ensures that abilities do not trigger or apply effects
// when their conditions evaluate to false.
// ====================================================================

/// Negative test for `card_count_condition` (Heart requirement not met)
/// Target: PL!HS-PR-019-PR (Ginko)
/// "登場: 自分のデッキの上からカードを3枚控え室に置く。それらがすべてheart04を持つメンバーカードの場合..."
#[test]
fn card_count_condition_negative_wrong_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ginko = game.id("PL!HS-PR-019-PR"); // Needs 3 heart04 cards discarded

    // Set up deck with 2 heart04 and 1 heart01 (condition requires ALL 3 to be heart04)
    let h04_card1 = game.id("PL!HS-PR-019-RM"); // Has heart04
    let h04_card2 = game.id("PL!HS-PR-019-RM"); // Has heart04
    let h01_card = game.id("PL!HS-PR-021-RM"); // Has heart01

    game.state.player1.main_deck.cards = vec![h04_card1, h04_card2, h01_card].into();
    game.state.player1.hand.cards.push(ginko);

    game.give_energy(10);

    // Play Ginko
    game.play_to_stage(ginko, MemberArea::Center);

    // Condition NOT met because one card was heart01. She should NOT gain the heart04 bonus.
    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(ginko, HeartColor::Heart04);
    assert_eq!(
        heart_mod, 0,
        "Ginko should not gain heart04 bonus because one of the 3 discarded cards was heart01"
    );
}

/// Negative test for `comparison_condition` (Score requirement not met)
/// Target: PL!N-bp4-012-P (Umi)
/// "常時: 相手のサクセスライブカード置場のライブカードの合計スコアが6以上の場合、スコアを＋1。"
#[test]
fn comparison_condition_negative_insufficient_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let umi = game.id("PL!N-bp4-012-P"); // Needs opponent success live score >= 6
    let opp_live1 = game.id("PL!-sd1-019-SD"); // Score 1
    let opp_live2 = game.id("PL!-sd1-019-SD"); // Score 1 (Total score = 2, < 6)

    game.add_to_stage(MemberArea::Center, umi);

    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(opp_live1);
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(opp_live2);

    game.state.recalculate_constants();

    let score_mod = game.state.mods.get_score_modifier(umi);
    assert_eq!(
        score_mod, 0,
        "Umi should not get score +1 because opponent score is 2 (< 6)"
    );
}

/// Negative test for `location_condition` with exact count threshold.
/// Using a hypothetical or similar condition where exactly X cards are required.
#[test]
fn location_condition_negative_too_few_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // PL!N-bp1-008-R Emma has "起動 ターン1回 手札のメンバーカードを1枚控え室に置く：..."
    // We will test if we can activate it without a member card in hand.
    let emma = game.id("PL!N-bp1-008-R");

    game.add_to_stage(MemberArea::Center, emma);
    game.state.player1.hand.cards.clear(); // Empty hand
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!N-bp1-001-R")); // Valid target in discard

    // Attempt to activate ability
    let _ = game.try_activate_ability(emma);

    // Hand should still be empty because the ability cost failed
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "Should not add card to hand since cost could not be paid"
    );
}
