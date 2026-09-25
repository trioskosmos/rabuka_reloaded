use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Target: PL!-bp5-003-R+ (Honoka)
// 常時: If 3+ members with DISTINCT names on your stage → gain heart03.
// So: location_condition with distinct=true, count=3, operator=>=
// ====================================================================

/// Edge case: Exactly 3 members with distinct names → gain 1 heart03.
#[test]
fn three_distinct_stage_names_grant_one_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-bp5-003-R+");
    let member2 = game.id("PL!-sd1-001-SD");
    let member3 = game.id("PL!-sd1-002-SD");

    // Fill all three stage areas with different-named members
    game.add_to_stage(MemberArea::LeftSide, honoka);
    game.add_to_stage(MemberArea::Center, member2);
    game.add_to_stage(MemberArea::RightSide, member3);

    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(honoka, HeartColor::Heart03);
    assert_eq!(
        heart_mod, 1,
        "Honoka: 3 distinct-named members → +1 heart03, got {}",
        heart_mod
    );
}

/// Edge case: Only 2 distinct members → gain 0.
#[test]
fn two_distinct_stage_names_grant_no_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-bp5-003-R+");
    let member2 = game.id("PL!-sd1-002-SD");

    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::RightSide, member2);

    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(honoka, HeartColor::Heart03);
    assert_eq!(
        heart_mod, 0,
        "Honoka: only 2 distinct members → no heart, got {}",
        heart_mod
    );
}

/// Edge case: 3 members but 2 have the same name (not distinct) → gain 0.
#[test]
fn duplicate_stage_name_prevents_three_name_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-bp5-003-R+");
    let same_as_honoka = game.new_id("PL!-bp5-003-R+");
    let member3 = game.id("PL!-sd1-002-SD");

    game.add_to_stage(MemberArea::LeftSide, honoka);
    game.add_to_stage(MemberArea::Center, same_as_honoka);
    game.add_to_stage(MemberArea::RightSide, member3);

    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(honoka, HeartColor::Heart03);
    assert_eq!(
        heart_mod, 0,
        "Honoka: 2 copies of same member → not 3 distinct names, got {}",
        heart_mod
    );
}

/// Edge case: Stage becomes empty → recalculate from 3→0 members.
#[test]
fn three_name_heart03_clears_when_source_leaves_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-bp5-003-R+");
    let member2 = game.id("PL!-sd1-001-SD");
    let member3 = game.id("PL!-sd1-002-SD");

    game.add_to_stage(MemberArea::LeftSide, honoka);
    game.add_to_stage(MemberArea::Center, member2);
    game.add_to_stage(MemberArea::RightSide, member3);

    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(honoka, HeartColor::Heart03),
        1
    );

    // Remove one member → only 2 distinct
    game.state.player1.stage.stage[0] = -1;
    game.state.recalculate_constants();

    let heart_mod = game
        .state
        .mods
        .get_heart_modifier(honoka, HeartColor::Heart03);
    assert_eq!(
        heart_mod, 0,
        "Honoka: one member removed → only 2 distinct, no heart, got {}",
        heart_mod
    );
}
