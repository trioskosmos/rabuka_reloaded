use crate::helpers::*;

#[test]
fn constant_opponent_energy_ahead_grants_three_blades() {
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
