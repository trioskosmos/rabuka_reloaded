/// Edge-case tests for untested 常時 (constant/always-on) abilities with complex
/// conditions. Constants are conditionally-granted modifiers — the most error-prone
/// path because they must dynamically update as game state changes.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

const FILLER: &str = "PL!-sd1-010-SD";

fn blade(game: &TestGame, cid: i16) -> i32 {
    game.state.mods.get_blade_modifier(cid)
}

fn heart(game: &TestGame, cid: i16, hc: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(cid, hc)
}

// ===================================================================
// PL!S-PR-037-PR: 常時 自分のステージにいるメンバーがちょうど2人である
//   かぎり、heart05＋ブレード×1を得る。
//   Parser bug: heart_colors leaked from effect into condition, making the
//   count filter only match members with heart05 (wrong). Fixed by stripping
//   heart_colors from conditions whose text has no {{heart_ icons.
// ===================================================================

#[test]
fn exactly_two_members_grants() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!S-PR-037-PR");
    let a = game.id("PL!-sd1-001-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }

    // 1 member (card only): no gain
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, card);
    game.state.recalculate_constants();
    assert_eq!(blade(&game, card), 0, "1 stage member → no blade");
    assert_eq!(heart(&game, card, HeartColor::Heart05), 0, "1 stage member → no heart05");

    // 2 members: gain heart05 + blade
    game.add_to_stage(rabuka_engine::zones::MemberArea::LeftSide, a);
    game.state.recalculate_constants();
    assert!(blade(&game, card) >= 1, "2 stage members → blade, got {}", blade(&game, card));
    assert!(heart(&game, card, HeartColor::Heart05) >= 1, "2 stage members → heart05");

    // 3 members: no gain (exactly 2)
    game.add_to_stage(rabuka_engine::zones::MemberArea::RightSide, game.id("PL!-sd1-002-SD"));
    game.state.recalculate_constants();
    assert_eq!(blade(&game, card), 0, "3 stage members → no blade");
}

// ===================================================================
// PL!HS-pb1-007-R: 常時 自分のステージにメンバーがちょうど2人おり、
//   かつ相手のステージにメンバーが3人以上いるかぎり、heart06を得る。
//   Compound condition with self_count == 2 AND opp_count >= 3.
// ===================================================================

#[test]
fn compound_condition_two_self_three_opp() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-pb1-007-R");
    let a = game.id("PL!-sd1-001-SD");

    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, card);
    game.add_to_stage(rabuka_engine::zones::MemberArea::LeftSide, a);

    // self=2, opp=2: NOT met (opp < 3)
    game.state.player2.stage.stage[0] = game.id("PL!-sd1-001-SD");
    game.state.player2.stage.stage[1] = game.id("PL!-sd1-002-SD");
    game.state.recalculate_constants();
    assert_eq!(heart(&game, card, HeartColor::Heart06), 0, "self=2, opp=2 → no heart06");

    // self=2, opp=3: MET → gain heart06
    game.state.player2.stage.stage[2] = game.id("PL!-sd1-003-SD");
    game.state.recalculate_constants();
    assert!(
        heart(&game, card, HeartColor::Heart06) >= 1,
        "self=2, opp=3 → should gain heart06"
    );

    // self=3: NOT met (not exactly 2)
    game.add_to_stage(rabuka_engine::zones::MemberArea::RightSide, game.id("PL!-sd1-004-SD"));
    game.state.recalculate_constants();
    assert_eq!(heart(&game, card, HeartColor::Heart06), 0, "self=3 → no heart06");
}

// ===================================================================
// PL!HS-pb1-022-N: 常時 自分のステージに「大沢瑠璃乃」がいるかぎり、
//   heart01×2を得る。藤島慈がいるかぎり、ブレード×2を得る。
//   Member-name constants on one card.
// ===================================================================

#[test]
fn member_name_constants_selective_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-pb1-022-N");

    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, card);

    // Neither on stage: no gain
    game.state.recalculate_constants();
    assert_eq!(heart(&game, card, HeartColor::Heart01), 0, "no named members → no heart01");
    assert_eq!(blade(&game, card), 0, "no named members → no blade");

    // 藤島慈 (PL!HS-bp2-015-N) on stage: blade×2, but NOT heart01
    let toko = game.id("PL!HS-bp2-015-N");
    game.add_to_stage(rabuka_engine::zones::MemberArea::LeftSide, toko);
    game.state.recalculate_constants();
    assert_eq!(
        heart(&game, card, HeartColor::Heart01), 0,
        "only 藤島慈 → no heart01"
    );
    assert!(blade(&game, card) >= 2, "藤島慈 on stage → blade×2");

    // 大沢瑠璃乃 (PL!HS-bp6-019-N) on stage: heart01×2
    let rurino = game.id("PL!HS-bp6-019-N");
    game.add_to_stage(rabuka_engine::zones::MemberArea::RightSide, rurino);
    game.state.recalculate_constants();
    assert!(
        heart(&game, card, HeartColor::Heart01) >= 2,
        "大沢瑠璃乃 on stage → heart01×2"
    );

    // Remove 藤島慈: blade goes away, heart01 stays
    game.state.player1.stage.stage[0] = -1;
    game.state.recalculate_constants();
    assert_eq!(blade(&game, card), 0, "藤島慈 removed → blade gone");
    assert!(
        heart(&game, card, HeartColor::Heart01) >= 2,
        "藤島慈 removed but 大沢 still there → heart01×2 remains"
    );
}
