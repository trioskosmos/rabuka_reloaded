use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn hs_bp6_012_debut_with_other_cerise_bouquet_member_activates_one_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-bp6-012-R");
    let mate = game.id("PL!HS-bp1-012-PR"); // スリーズブーケ member
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = mate;

    // One WAIT energy (card pushed without add_active).
    let energy = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(energy);
    assert_eq!(game.state.player1.energy_zone.active_count(), 0);

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        1,
        "teammate present -> 1 energy activated"
    );
}
