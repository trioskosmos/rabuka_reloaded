/// Tests for 常時 (constant) abilities — continuous effects that don't trigger.
use crate::helpers::*;

/// PL!-bp3-002-R (絢瀬絵里) ab#1 Q144: 「常時」相手のステージにいる
/// ウェイト状態のメンバー1人につき、ブレードを得る。
///
/// For each wait member on the OPPONENT's stage, gain 1 blade.
/// Test: recalculate constant modifiers → blade is added.
#[test]
fn constant_blade_per_waited_opponent_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let eri = game.id("PL!-bp3-002-R");
    let opp1 = game.new_id("PL!-sd1-010-SD");

    // Stage: eri center (p1), one waited opponent member (p2)
    game.state.player1.stage.stage = [-1, eri, -1];
    game.state.player2.stage.stage = [opp1, -1, -1];
    game.state.mods.add_orientation_modifier(opp1, "wait");

    // Recalculate constant blade modifiers
    game.state.recalculate_constants();

    // The constant ability should have added a blade modifier: +1 per wait member
    let blade_mod = game.state.mods.get_blade_modifier(eri);
    assert_eq!(
        blade_mod, 1,
        "Constant ability: 1 wait member → exactly 1 blade, got {}",
        blade_mod
    );
}
