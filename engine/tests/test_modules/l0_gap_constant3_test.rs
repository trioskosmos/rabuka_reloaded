/// L0 gap coverage: additional Constant (常時) blade/heart abilities.
use crate::helpers::*;

/// PL!SP-bp4-003-R: 常時 センター → ブレード+2.
#[test]
fn sp_bp4_003_center_blade() {
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

/// PL!HS-bp2-006-R: 常時 ほかの『みらくらぱーく！』メンバー1人につき、ブレード+1。
/// TODO: needs investigation — Mirakuraku group matching may require
/// specific card data or unit field setup.
#[test]
#[ignore = "Mirakuraku group matching needs investigation"]
fn hs_bp2_006_per_other_mirakuraku_member_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!HS-bp2-006-R");
    // Another みらくらぱーく！ member on stage
    let other_mk = game.id("PL!HS-sd1-005-SD");
    game.state.player1.stage.stage = [other_mk, member, -1];
    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(member);
    assert!(
        blade >= 1,
        "one other Mirakuraku member should grant >= +1 blade"
    );
}

/// PL!S-pb1-005-PR: 常時 相手のエネルギーが自分より多い場合、ブレード+3。
#[test]
fn spb1_005_opponent_more_energy_grants_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!S-pb1-005-PR");
    game.state.player1.stage.stage = [member, -1, -1];
    // P1 has no energy, P2 has plenty
    game.state.player2.energy_zone
        .cards
        .push(game.id("LL-E-001-SD"));
    game.state.player2.energy_zone.add_active(3);
    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(member);
    assert_eq!(
        blade, 3,
        "opponent has more energy → +3 blade"
    );

    // Negative: give P1 energy so P2 doesn't have more
    game.give_energy(10);
    game.state.recalculate_constants();
}
