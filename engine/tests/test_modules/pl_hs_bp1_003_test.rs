use crate::helpers::*;
use rabuka_engine::ability::util;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Target: PL!HS-bp1-003-R+ (乙宗梢) ab#0
// 常時: 自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、
//       かつ名前が異なる場合、「常時ライブの合計スコアを＋１する。」を得る。
//
// Tests that the multi-name card 上原歩夢&澁谷かのん&日野下花帆 (LL-bp1-001-R＋)
// is correctly recognized as a 蓮ノ空 member via its series.
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
/// with distinct names → condition met → +1 score.
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

    // Debug: check group match at runtime
    let stage_ids = game.state.player1.stage.stage;
    for (i, &cid) in stage_ids.iter().enumerate() {
        if cid == -1 {
            continue;
        }
        eprintln!(
            "Slot {}: id={} matches 蓮ノ空={}",
            i,
            cid,
            util::card_matches_group_str(&game.db, cid, Some("蓮ノ空"))
        );
    }
    eprintln!("stage array: {:?}", stage_ids);

    game.state.recalculate_constants();

    let score_mod = game.state.mods.get_score_modifier(kozue);
    assert!(
        score_mod > 0,
        "乙宗梢 should gain +1 score, got {}",
        score_mod
    );
}

/// Non-蓮ノ空 member on one area → condition fails → no score.
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
        game.state.mods.get_score_modifier(kozue),
        0,
        "Non-蓮ノ空 member → no score mod"
    );
}

/// Empty area → all_areas condition fails → no score.
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
        game.state.mods.get_score_modifier(kozue),
        0,
        "Empty area → no score mod"
    );
}

/// Duplicate names → distinct condition fails → no score.
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
        game.state.mods.get_score_modifier(kozue),
        0,
        "Duplicate names → no score mod"
    );
}
