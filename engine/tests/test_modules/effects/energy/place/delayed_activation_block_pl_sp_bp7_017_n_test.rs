use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_sp_bp7_017_n_energy_placement_sets_delayed_activation_block() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kinako = game.id("PL!SP-bp7-017-N");
    game.add_to_stage(MemberArea::Center, kinako);
    fill_energy_deck(&mut game, 0, 2);
    fire_trigger(&mut game, kinako, AbilityTrigger::Debut, "登場");
    assert_eq!(game.state.player1.energy_zone.cards.len(), 1, "one energy placed from the energy deck");
    let placed = *game.state.player1.energy_zone.cards.last().unwrap();
    assert!(game.state.mods.is_delayed_cannot_active(placed), "Q280-family: the placed energy must NOT activate next turn");
    assert_eq!(game.state.player1.energy_deck.cards.len(), 1, "energy deck shrank by one");
}
