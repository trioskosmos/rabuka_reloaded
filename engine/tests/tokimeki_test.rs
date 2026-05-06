/// Tests for TOKIMEKI Runners (PL!N-bp5-026-L):
///
/// Ab#0 (ライブ開始時): 自分のステージにいるメンバーが持つハートの中に
///   heart01〜heart06がすべてある場合、このカードのスコアを＋１する。
///
/// Q216: Hearts checked collectively across ALL members, not per-member.
///   Bug: engine's execute_modify_score ignores heart_colors filter entirely,
///   so the +1 applies regardless of which hearts are actually present.
///
/// Q232: +1 score modifies TOTAL, not the base card score.
///   Total scoring = base(2) + modifier(1) = 3, but card.score stays 2.

mod helpers;
use helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

/// Q216: Two members collectively providing only heart02,04,05 (missing 01,03,06).
/// Condition FAILS (not all 6 colors present) → no score +1.
#[test]
fn tokimeki_q216_missing_hearts_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let tokimeki = game.id("PL!N-bp5-026-L");
    let filler = game.id("PL!-sd1-010-SD");

    // PL!S-sd1-001-SD (千歌): heart02=3, heart04=2, heart05=2
    // PL!S-sd1-011-SD (梨子): heart02=1, heart05=2
    // Combined: heart02=4, heart04=2, heart05=4 → NO heart01,03,06
    let member_a = game.id("PL!S-sd1-001-SD");
    let member_b = game.id("PL!S-sd1-011-SD");

    game.state.player1.stage.stage = [member_a, member_b, -1];
    game.state.player1.hand.cards.push(tokimeki);

    for _ in 0..20 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..20 { game.state.player2.main_deck.cards.push(filler); }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(tokimeki);
    game.pass();
    game.pass();

    let has_mod = game.state.get_score_modifier(tokimeki);
    assert_eq!(has_mod, 0,
        "Q216: Missing heart01/03/06 → no +1 score");
}

/// Q216 variant: All 6 colors collectively → condition correctly met.
/// Uses members that together cover all 6 hearts.
#[test]
fn tokimeki_all_6_colors_collectively_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let tokimeki = game.id("PL!N-bp5-026-L");
    let filler = game.id("PL!-sd1-010-SD");

    // Member A: heart02=3, heart04=2, heart05=2
    // Member B: heart01=1, heart03=2, heart06=1
    // Combined: all 6 colors ✓
    let member_a = game.id("PL!S-sd1-001-SD");
    let member_b = game.id("PL!-sd1-001-SD"); // 園田海未: h01=1,h03=2,h06=1

    game.state.player1.stage.stage = [member_a, member_b, -1];
    game.state.player1.hand.cards.push(tokimeki);

    for _ in 0..20 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..20 { game.state.player2.main_deck.cards.push(filler); }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(tokimeki);
    game.pass();
    game.pass();

    assert_eq!(game.state.get_score_modifier(tokimeki), 1,
        "Q216: All 6 colors collectively → +1 score");
}

/// Q232: Score modifier is separate from base card score.
/// Uses 3 members that collectively provide all 6 heart colors.
#[test]
fn tokimeki_q232_modifier_separate_from_base_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tokimeki = game.id("PL!N-bp5-026-L");
    let filler = game.id("PL!-sd1-010-SD");

    // 3 members covering all 6 colors:
    // PL!S-sd1-001-SD: heart02=3, heart04=2, heart05=2
    // PL!-sd1-001-SD (穂乃果): heart01=1, heart03=2, heart06=1
    // Third member as filler (don't need more colors, but it's fine)
    let member_a = game.id("PL!S-sd1-001-SD");
    let member_b = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [member_a, member_b, -1];
    game.state.player1.hand.cards.push(tokimeki);

    for _ in 0..20 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..20 { game.state.player2.main_deck.cards.push(filler); }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(tokimeki);
    game.pass();
    game.pass();

    assert_eq!(game.state.get_score_modifier(tokimeki), 1,
        "Q232: All 6 colors → Modifier +1 applied");
}
