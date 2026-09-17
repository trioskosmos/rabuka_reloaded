use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn muse_bp6_009_r_center_with_two_original_blade_side_members_adds_one_live_total_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!-bp6-009-R");
    game.add_to_stage(MemberArea::Center, me);

    let l = game.id("PL!-sd1-006-SD");
    let r = game.new_id("PL!-sd1-006-SD");
    game.add_to_stage(MemberArea::LeftSide, l);
    game.add_to_stage(MemberArea::RightSide, r);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "blade-2 members in both side areas -> live total +1"
    );
}
