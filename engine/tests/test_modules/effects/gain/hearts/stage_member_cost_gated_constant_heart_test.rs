use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member, cost 4

#[test]
fn sp_sd2_008_constant_cost13_or_higher_stage_member_grants_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-sd2-008-SD2");
    let big = game.id("PL!HS-bp5-004-R"); // cost 15
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = big;

    game.state.recalculate_constants();

    const H03: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart03;
    let h03 = game.state.mods.get_heart_modifier(me, H03);
    assert!(h03 > 0, "cost-15 member on stage -> self gains heart03 (got {h03})");
}

#[test]
fn sp_sd2_008_constant_no_cost13_or_higher_stage_member_grants_no_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-sd2-008-SD2");
    let small = game.id(FILLER); // cost 4
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = small;

    game.state.recalculate_constants();

    const H03: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart03;
    let h03 = game.state.mods.get_heart_modifier(me, H03);
    assert_eq!(
        h03, 0,
        "no cost-13+ member -> no heart03 granted"
    );
}
