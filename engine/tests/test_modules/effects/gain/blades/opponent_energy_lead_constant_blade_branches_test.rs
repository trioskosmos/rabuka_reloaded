use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Target: PL!S-pb1-005-R (You)
// 常時: 相手のエネルギーが自分より多い場合、ブレードを3得る。
// Condition: comparison_condition, resource_type=energy, operator=>
//            target=opponent, comparison_target=self
// Previously broken: comparison_target="self" with resource_type="energy"
// returned 0 instead of self's energy count.
// ====================================================================

#[allow(dead_code)]
fn has_positive_blade_modifier(gs: &rabuka_engine::game_state::GameState, cid: i16) -> bool {
    gs.mods.get_blade_modifier(cid) > 0
}

fn setup_energy_comparison(game: &mut TestGame, p1_energy: usize, p2_energy: usize) -> i16 {
    let you = game.id("PL!S-pb1-005-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_stage(MemberArea::Center, you);
    for _ in 0..p1_energy {
        game.state.player1.energy_zone.cards.push(filler);
    }
    game.state
        .player1
        .energy_zone
        .set_active_count(p1_energy as u8);
    for _ in 0..p2_energy {
        game.state.player2.energy_zone.cards.push(filler);
    }
    game.state
        .player2
        .energy_zone
        .set_active_count(p2_energy as u8);
    you
}

#[test]
fn opponent_more_energy_grants_three_constant_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let you = setup_energy_comparison(&mut game, 3, 7);
    game.state.recalculate_constants();
    assert!(
        has_positive_blade_modifier(&game.state, you),
        "You: opponent 7 > self 3 → should gain blade"
    );
    assert_eq!(game.state.mods.get_blade_modifier(you), 3);
}

#[test]
fn equal_energy_has_no_positive_constant_blade_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let you = setup_energy_comparison(&mut game, 5, 5);
    game.state.recalculate_constants();
    assert!(
        !has_positive_blade_modifier(&game.state, you),
        "You: opponent 5 == self 5 → no blade"
    );
}

#[test]
fn opponent_less_energy_has_no_positive_constant_blade_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let you = setup_energy_comparison(&mut game, 8, 3);
    game.state.recalculate_constants();
    assert!(
        !has_positive_blade_modifier(&game.state, you),
        "You: opponent 3 < self 8 → no blade"
    );
}

#[test]
fn energy_lead_changes_toggle_positive_constant_blade_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let filler = game.id("PL!-sd1-010-SD");
    let you = setup_energy_comparison(&mut game, 3, 7);

    // Opponent has more → blade
    game.state.recalculate_constants();
    assert!(has_positive_blade_modifier(&game.state, you));

    // Now P1 gains more energy to surpass opponent → blade removed
    for _ in 0..10 {
        game.state.player1.energy_zone.cards.push(filler);
    }
    game.state.player1.energy_zone.set_active_count(13);
    game.state.recalculate_constants();
    assert!(
        !has_positive_blade_modifier(&game.state, you),
        "You: self 13 > opponent 7 → blade should be removed"
    );

    // Later opponent gains even more energy → blade returns
    for _ in 0..10 {
        game.state.player2.energy_zone.cards.push(filler);
    }
    game.state.player2.energy_zone.set_active_count(17);
    game.state.recalculate_constants();
    assert!(
        has_positive_blade_modifier(&game.state, you),
        "You: opponent 17 > self 13 → blade should return"
    );
    assert_eq!(game.state.mods.get_blade_modifier(you), 3);
}
