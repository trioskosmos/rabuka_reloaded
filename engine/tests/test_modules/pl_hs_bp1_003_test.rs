use crate::helpers::*;
use rabuka_engine::ability::util;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
fn advance_to_live_victory(game: &mut TestGame) {
    for _ in 0..3 {
        game.pass();
    }
}

// ====================================================================
// Target: PL!HS-bp1-003-R+ (乙宗梢) ab#0
// 常時: 自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、
//       かつ名前が異なる場合、「常時ライブの合計スコアを＋１する。」を得る。
//
// The +1 score is stored in p1_constant_total_score_bonus /
// p2_constant_total_score_bonus (per-player global bonus), NOT as a
// per-card score modifier. The bonus is added directly to the total
// live score in calculate_live_score.
// ====================================================================

/// Verify the multi-name card matches 蓮ノ空 via group check.
#[test]
fn multiname_card_matches_hasunosora_group() {
    let db = load_real_database();

    let multiname_id = card_id(&db, "LL-bp1-001-R\u{ff0b}");
    assert!(
        util::card_matches_group_str(&db, multiname_id, Some("蓮ノ空")),
        "LL-bp1-001-R+ should match 蓮ノ空 (series includes 蓮ノ空女学院)"
    );

    let kozue_id = card_id(&db, "PL!HS-bp1-003-R\u{ff0b}");
    assert!(
        util::card_matches_group_str(&db, kozue_id, Some("蓮ノ空")),
        "PL!HS-bp1-003-R+ should match 蓮ノ空"
    );

    let filler_id = card_id(&db, "PL!-sd1-010-SD");
    assert!(
        !util::card_matches_group_str(&db, filler_id, Some("蓮ノ空")),
        "Filler should NOT match 蓮ノ空"
    );
}

/// All 3 stage areas filled with 蓮ノ空 members (including multi-name card)
/// with distinct names → condition met → P1 gets +1 global score bonus, P2 gets 0.
#[test]
fn all_areas_hasunosora_with_multiname_gains_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kozue = game.id("PL!HS-bp1-003-R\u{ff0b}");
    let multiname = game.id("LL-bp1-001-R\u{ff0b}");
    let sayaka = game.id("PL!HS-bp1-002-R");

    game.add_to_stage(MemberArea::LeftSide, kozue);
    game.add_to_stage(MemberArea::Center, multiname);
    game.add_to_stage(MemberArea::RightSide, sayaka);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "Condition met → P1 bonus = +1"
    );
    assert_eq!(
        game.state.mods.p2_constant_total_score_bonus, 0,
        "P2 has no matching cards → P2 bonus = 0"
    );
}

/// Non-蓮ノ空 member on one area → condition fails → no bonus for either player.
#[test]
fn missing_hasunosora_member_fails_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kozue = game.id("PL!HS-bp1-003-R\u{ff0b}");
    let multiname = game.id("LL-bp1-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::LeftSide, kozue);
    game.add_to_stage(MemberArea::Center, multiname);
    game.add_to_stage(MemberArea::RightSide, filler);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "Non-蓮ノ空 member → no bonus"
    );
}

/// Empty area → all_areas condition fails → no bonus.
#[test]
fn empty_area_fails_all_areas_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kozue = game.id("PL!HS-bp1-003-R\u{ff0b}");
    let multiname = game.id("LL-bp1-001-R\u{ff0b}");

    game.add_to_stage(MemberArea::LeftSide, kozue);
    game.add_to_stage(MemberArea::Center, multiname);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "Empty area → no bonus"
    );
}

/// Duplicate names → distinct condition fails → no bonus.
#[test]
fn duplicate_names_fails_distinct_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kozue = game.id("PL!HS-bp1-003-R\u{ff0b}");
    let kozue2 = game.new_id("PL!HS-bp1-003-R\u{ff0b}");
    let sayaka = game.id("PL!HS-bp1-002-R");

    game.add_to_stage(MemberArea::LeftSide, kozue);
    game.add_to_stage(MemberArea::Center, kozue2);
    game.add_to_stage(MemberArea::RightSide, sayaka);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "Duplicate names → no bonus"
    );
}

/// End-to-end live score: condition met → P1 gets +1 bonus → calculate_live_score
/// returns base(1) + bonus(1) = 2. P2 gets no bonus → score = base(1) + 0 = 1.
#[test]
fn live_score_includes_global_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kozue = game.id("PL!HS-bp1-003-R\u{ff0b}");
    let multiname = game.id("LL-bp1-001-R\u{ff0b}");
    let sayaka = game.id("PL!HS-bp1-002-R");

    game.add_to_stage(MemberArea::LeftSide, kozue);
    game.add_to_stage(MemberArea::Center, multiname);
    game.add_to_stage(MemberArea::RightSide, sayaka);

    // Add a live card (score=1, need_heart heart0:4)
    let live = game.id("PL!HS-bp1-019-L");
    game.state.player1.live_card_zone.cards.push(live);

    // Provide stage hearts to satisfy need_heart
    let mut h = HeartMap::new();
    h.insert(HeartColor::Heart00, 4);
    let hearts = BaseHeart { hearts: h };
    game.state.player1.stage_hearts = Some(hearts.clone());

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "Condition met → +1 P1 bonus"
    );
    assert_eq!(
        game.state.mods.p2_constant_total_score_bonus, 0,
        "P2 has no bonus"
    );

    let p1_score = game.state.player1.live_card_zone.calculate_live_score(
        &game.state.card_database,
        0,
        game.state.player1.stage_hearts.as_ref(),
        None,
        None,
        game.state.mods.p1_constant_total_score_bonus,
    );
    assert_eq!(p1_score, 2, "P1 score = base(1) + bonus(1) = 2");

    let p2_score = game.state.player2.live_card_zone.calculate_live_score(
        &game.state.card_database,
        0,
        game.state.player2.stage_hearts.as_ref(),
        None,
        None,
        game.state.mods.p2_constant_total_score_bonus,
    );
    assert_eq!(p2_score, 0, "P2 has no live card → score = 0");
}

/// Full turn-phase end-to-end: P1 has condition met (+1 bonus), P2 does not.
/// Both have live card `PL!-bp3-019-L` (score=0, need_heart={heart01:1, heart06:1}).
/// P1's stage provides heart01:5, heart06:3; P2's stage provides heart01:4, heart06:3.
/// Both hearts satisfied → P1 score=1, P2 score=0 → P1 wins.
#[test]
fn live_victory_p1_wins_with_global_bonus_p2_without() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kozue = game.id("PL!HS-bp1-003-R\u{ff0b}");
    let multiname = game.id("LL-bp1-001-R\u{ff0b}");
    let sayaka = game.id("PL!HS-bp1-002-R");
    let live = game.id("PL!-bp3-019-L");
    let filler = game.id("PL!-sd1-010-SD");

    // P1: 3 蓮ノ空 members (condition met → +1 bonus)
    game.add_to_stage(MemberArea::LeftSide, kozue);
    game.add_to_stage(MemberArea::Center, multiname);
    game.add_to_stage(MemberArea::RightSide, sayaka);
    game.state.player1.hand.cards.push(live);

    // P2: only 2 members (condition not met → no bonus)
    let p2_kozue = game.new_id("PL!HS-bp1-003-R\u{ff0b}");
    let p2_multiname = game.new_id("LL-bp1-001-R\u{ff0b}");
    let p2_live = game.new_id("PL!-bp3-019-L");
    game.state.player2.stage.stage = [p2_kozue, p2_multiname, -1];
    game.state.player2.hand.cards.push(p2_live);

    // Fill both decks
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Advance through phases to live victory
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    advance_to_live_victory(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.pass();

    // P1 wins (score=1 > P2 score=0)
    let p1_success = game.state.player1.success_live_card_zone.cards.len();
    let p2_success = game.state.player2.success_live_card_zone.cards.len();
    assert_eq!(
        p1_success, 1,
        "P1 has +1 bonus → higher score → P1's live card moves to success zone"
    );
    assert_eq!(
        p2_success, 0,
        "P2 has no bonus → lower score → P2's live card stays in live zone"
    );
}

/// Negative live score: condition not met → no bonus → score = base(1) only.
#[test]
fn live_score_no_bonus_when_condition_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kozue = game.id("PL!HS-bp1-003-R\u{ff0b}");
    let multiname = game.id("LL-bp1-001-R\u{ff0b}");

    // Only 2 stage slots filled → all_areas condition fails
    game.add_to_stage(MemberArea::LeftSide, kozue);
    game.add_to_stage(MemberArea::Center, multiname);

    let live = game.id("PL!HS-bp1-019-L");
    game.state.player1.live_card_zone.cards.push(live);

    let mut h = HeartMap::new();
    h.insert(HeartColor::Heart00, 4);
    let hearts = BaseHeart { hearts: h };
    game.state.player1.stage_hearts = Some(hearts.clone());

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "Condition not met → no bonus"
    );

    let score = game.state.player1.live_card_zone.calculate_live_score(
        &game.state.card_database,
        0,
        game.state.player1.stage_hearts.as_ref(),
        None,
        None,
        game.state.mods.p1_constant_total_score_bonus,
    );
    assert_eq!(score, 1, "Score = base(1) + 0 = 1");
}
