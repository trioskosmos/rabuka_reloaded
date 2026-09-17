use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Target: PL!SP-bp1-001-P (Kanon)
// 常時: If no other members on your stage → cannot live.
// So: restriction with condition: location_condition(negation, exclude_self)
// ====================================================================

/// Edge case: Kanon is alone on stage → cannot_live restriction.
#[test]
fn alone_on_stage_sets_cannot_live_restriction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id("PL!SP-bp1-001-P");

    // Kanon alone on stage (no other members)
    game.add_to_stage(MemberArea::Center, kanon);

    game.state.recalculate_constants();

    assert!(
        game.state.is_action_prohibited("cannot_live"),
        "Kanon alone: cannot_live restriction should be active"
    );
}

/// Edge case: Kanon with another member → no restriction.
#[test]
fn another_stage_member_prevents_cannot_live_restriction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kanon = game.id("PL!SP-bp1-001-P");
    let other = game.id("PL!-sd1-002-SD");

    game.add_to_stage(MemberArea::Center, kanon);
    game.add_to_stage(MemberArea::LeftSide, other);

    game.state.recalculate_constants();

    assert!(
        !game.state.is_action_prohibited("cannot_live"),
        "Kanon with other: no restriction expected"
    );
}
