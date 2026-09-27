use crate::helpers::*;

#[test]
fn constant_blades_equal_under_member_energy_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let mia = game.id("PL!N-pb1-011-R");
    let energy = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [-1, mia, -1];
    game.state.player1.stage.under_cards[1].push(energy);
    game.state.player1.stage.under_cards[1].push(energy);

    // Verify the card is in the db and on stage
    let card = game.db.get_card(mia);
    assert!(card.is_some(), "Mia Taylor should be in db");
    assert_eq!(game.state.player1.stage.stage[1], mia, "Mia at center");

    // Verify the card has abilities
    let abilities: Vec<_> = card.unwrap().resolved_abilities().collect();
    assert!(
        !abilities.is_empty(),
        "Mia Taylor must resolve at least one ability for the per_under_member \
         blade bonus under test to be reachable"
    );

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(mia);
    assert_eq!(blade_mod, 2, "2 energy under → 2 blade, got {}", blade_mod);
}

#[test]
fn constant_blades_zero_without_under_member_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let mia = game.id("PL!N-pb1-011-R");

    game.state.player1.stage.stage = [-1, mia, -1];

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(mia);
    assert_eq!(blade_mod, 0, "0 energy under → 0 blade, got {}", blade_mod);
}
