/// Tests for 宮下 愛 (PL!N-bp3-005) auto ability and live start ability.
///
/// Ab#0 (自動): When 3+ members have debuted on your stage this turn,
///              draw until hand has 5 cards.
/// Ab#1 (ライブ開始時): If 2+ members debuted this turn, gain
///              "Always: +1 total live score" until live end.
///
/// QA entries: Q160, Q161, Q162

mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;

/// Q162: If 2 members have already debuted this turn, then this card is
/// played (3rd debut), the auto ability should trigger and draw until hand
/// has 5 cards.
/// Q161: This card's own debut counts toward the 3 — this test validates
/// that the count reaches 3 when Ai debuts as the 3rd member.
#[test]
fn miyashita_ai_q162_q161_three_debuts_triggers_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ai = game.id("PL!N-bp3-005-P");
    let filler = game.id("PL!-sd1-010-SD");

    // Energy for 2 filler (cost 4 each) + Ai (cost 15) = 23 total
    game.give_energy(25);

    // Hand: 2 filler cards + Ai
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(ai);

    // Populate deck so draw_until_count has cards to draw
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Debut #1: filler to Left
    game.play_to_stage(filler, MemberArea::LeftSide);

    // Debut #2: filler to Center
    game.play_to_stage(filler, MemberArea::Center);

    // Debut #3: Ai to Right
    game.play_to_stage(ai, MemberArea::RightSide);

    assert_eq!(game.state.player1.hand.cards.len(), 5,
        "Q162: auto ability should draw until hand has 5 cards after 3 debuts");

    assert_eq!(game.state.player1.debut_count_this_turn, 3,
        "Q161: debut count should be 3 and include Ai's own debut");
}

/// Q160: Members that debuted this turn but later left the stage (e.g.,
/// via area displacement) still count toward the debut threshold.
#[test]
fn miyashita_ai_q160_displaced_debuts_still_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ai = game.id("PL!N-bp3-005-P");
    let filler = game.id("PL!-sd1-010-SD");

    // Energy sufficient for all plays
    game.give_energy(25);

    // Hand
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(ai);

    // Populate deck
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Debut #1: filler to Left (stage: [filler, empty, empty])
    game.play_to_stage(filler, MemberArea::LeftSide);
    assert_eq!(game.state.player1.debut_count_this_turn, 1);

    // Debut #2: filler to Center (stage: [filler, filler, empty])
    game.play_to_stage(filler, MemberArea::Center);
    assert_eq!(game.state.player1.debut_count_this_turn, 2);

    // Debut #3: Ai to Left (occupied) — existing filler displaced to waitroom
    game.play_to_stage(ai, MemberArea::LeftSide);
    assert_eq!(game.state.player1.debut_count_this_turn, 3,
        "Q160: debut count should still be 3 even though filler was displaced");

    // The displaced filler should now be in waitroom
    let waitroom_has_filler = game.state.player1.waitroom.cards.contains(&filler);
    assert!(waitroom_has_filler,
        "Q160: displaced filler should be in waitroom");

    // Ai should be on stage
    assert!(game.state.player1.stage.stage.contains(&ai),
        "Ai should be on stage");

    // Triggered auto ability should have drawn until 5 cards in hand
    assert_eq!(game.state.player1.hand.cards.len(), 5,
        "Q160: auto ability should trigger and draw to 5 cards");
}

/// Negative test: Only 1 member debuted before Ai → count = 2 < 3
/// Auto ability should NOT trigger.
#[test]
fn miyashita_ai_only_two_debuts_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ai = game.id("PL!N-bp3-005-P");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(20);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(ai);

    // Debut #1: filler
    game.play_to_stage(filler, MemberArea::LeftSide);
    // Debut #2: Ai (only 2 total debuts)
    game.play_to_stage(ai, MemberArea::Center);

    // With only 2 debuts (< 3), the auto ability condition should fail
    // Hand was [filler, ai] and both played, so hand = []
    assert_eq!(game.state.player1.hand.cards.len(), 0,
        "Auto ability should NOT trigger with only 2 debuts");
}
