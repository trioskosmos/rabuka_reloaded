/// Tests for PL!-bp4-007-R/P (東條希 / Nozomi) — 登場 aggregate condition
///
/// Ability (登場):
///   自分の成功ライブカード置き場にカードが1枚以上あり、かつスコアの合計が１以下の場合、
///   ライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。
///
/// The condition checks success_live_card_zone for ≥1 card AND total score ≤ 1.
/// Effect: gain_ability applies a +1 score modifier via `gs.mods.add_score_modifier`.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Both conditions met: 1 score-1 live card in success zone → gain +1 score modifier.
#[test]
fn nozomi_condition_met_gains_score_boost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id("PL!-bp4-007-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live1 = game.id("PL!-sd1-019-SD"); // score 1

    // Fill decks so play_to_stage doesn't fail on deck operations
    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..30 { game.state.player2.main_deck.cards.push(filler); }

    // 1 card in success zone with total score 1 ≤ 1
    game.state.player1.success_live_card_zone.cards.push(live1);
    // Nozomi in hand, stage empty
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(nozomi);
    game.give_energy(15);

    let score_before = game.state.mods.get_score_modifier(nozomi);

    // Natural debut: 登場 trigger fires during play_to_stage
    game.play_to_stage(nozomi, MemberArea::LeftSide);

    let score_after = game.state.mods.get_score_modifier(nozomi);
    assert!(score_after > score_before,
        "Condition met: score modifier should increase (before={}, after={})",
        score_before, score_after);
}

/// Score total > 1: 2 score-1 cards = total 2 > 1 → condition fails → no modifier.
#[test]
fn nozomi_score_too_high_condition_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id("PL!-bp4-007-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live1 = game.id("PL!-sd1-019-SD"); // score 1
    let live2 = game.new_id("PL!-sd1-019-SD"); // score 1, total = 2

    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..30 { game.state.player2.main_deck.cards.push(filler); }

    // 2 cards in success zone, total score = 2 > 1
    game.state.player1.success_live_card_zone.cards.push(live1);
    game.state.player1.success_live_card_zone.cards.push(live2);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(nozomi);
    game.give_energy(15);

    let score_before = game.state.mods.get_score_modifier(nozomi);
    game.play_to_stage(nozomi, MemberArea::LeftSide);
    let score_after = game.state.mods.get_score_modifier(nozomi);

    assert_eq!(score_after, score_before,
        "Total score 2 > 1 → condition fails → no modifier change");
}

/// Empty success zone → first condition fails (0 ≥ 1 is false) → no modifier.
#[test]
fn nozomi_empty_zone_condition_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id("PL!-bp4-007-R");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..30 { game.state.player2.main_deck.cards.push(filler); }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(nozomi);
    game.give_energy(15);

    let score_before = game.state.mods.get_score_modifier(nozomi);
    game.play_to_stage(nozomi, MemberArea::LeftSide);
    let score_after = game.state.mods.get_score_modifier(nozomi);

    assert_eq!(score_after, score_before,
        "Empty success zone → condition fails → no modifier change");
}

/// Verify P rarity works the same as R.
#[test]
fn nozomi_p_rarity_same_behavior() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id("PL!-bp4-007-P");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..30 { game.state.player2.main_deck.cards.push(filler); }

    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(nozomi);
    game.give_energy(15);

    let score_before = game.state.mods.get_score_modifier(nozomi);
    game.play_to_stage(nozomi, MemberArea::LeftSide);
    let score_after = game.state.mods.get_score_modifier(nozomi);

    assert!(score_after > score_before,
        "P rarity: condition passes, score modifier increases");
}
