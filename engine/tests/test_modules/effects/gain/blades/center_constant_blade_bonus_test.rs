use crate::helpers::*;

#[test]
fn center_constant_grants_at_least_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m = game.id("PL!SP-bp4-003-R");
    game.state.player1.stage.stage = [-1, m, -1];
    game.state.recalculate_constants();
    assert!(
        game.state.mods.get_blade_modifier(m) >= 2,
        "center constant should grant +2 blade"
    );
}
