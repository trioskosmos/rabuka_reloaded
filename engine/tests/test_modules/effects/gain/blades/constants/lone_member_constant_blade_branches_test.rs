use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn pl_hs_pb1_015_r_loses_three_blades_only_while_alone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let seras = game.id("PL!HS-pb1-015-R");
    assert_eq!(game.db.get_card(seras).unwrap().blade, 5);
    game.add_to_stage(MemberArea::Center, seras);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(seras),
        -3,
        "lone on stage → ブレードを３つ失う"
    );
    game.add_to_stage(MemberArea::LeftSide, game.id(FILLER));
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(seras),
        0,
        "ほかのメンバーがいる → no loss"
    );
    game.state.player1.stage.stage[0] = -1;
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(seras), -3);
}

#[test]
fn pl_hs_bp6_002_r_gains_two_blades_only_while_alone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp6-002-R");
    assert_eq!(game.db.get_card(sayaka).unwrap().blade, 2);
    game.add_to_stage(MemberArea::Center, sayaka);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(sayaka),
        2,
        "lone on stage → blade+2"
    );
    game.add_to_stage(MemberArea::RightSide, game.new_id(FILLER));
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(sayaka),
        0,
        "not alone anymore → bonus off"
    );
}
