use crate::helpers::*;

#[test]
fn pl_hs_bp2_002_r_plus_three_blades_track_higher_cost_member_presence() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp2-002-R＋");
    let big = game.id("PL!S-bp5-009-R");
    game.state.player1.stage.stage[1] = sayaka;
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(sayaka), 0);
    game.state.player1.stage.stage[0] = big;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(sayaka),
        3,
        "cost-15 member > her 13 → ブレード3つ"
    );
    game.state.player1.stage.stage[0] = -1;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(sayaka),
        0,
        "no bigger member → blades gone"
    );
}
