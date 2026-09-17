use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn group_only_stage_and_energy_threshold_place_wait_energy_pl_sp_bp4_001_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp4-001-R");
    game.add_to_stage(MemberArea::Center, kanon);
    game.give_energy(7);
    fill_energy_deck(&mut game, 0, 3);

    let zone_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();

    fire_trigger(&mut game, kanon, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "one energy card placed from the energy deck"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before,
        "placed in WAIT state — ウェイト状態で置く"
    );
    assert_eq!(game.state.player1.energy_deck.cards.len(), 2, "deck −1");
}

#[test]
fn non_liella_stage_member_blocks_wait_energy_placement_pl_sp_bp4_001_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp4-001-R");
    game.add_to_stage(MemberArea::Center, kanon);
    game.state.player1.stage.stage[0] = game.new_id(FILLER);
    game.give_energy(7);
    fill_energy_deck(&mut game, 0, 3);

    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(&mut game, kanon, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "μ's member on stage → condition fails"
    );
}

#[test]
fn energy_threshold_includes_wait_state_for_placement_pl_sp_bp4_001_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp4-001-R");
    game.add_to_stage(MemberArea::Center, kanon);

    for _ in 0..6 {
        game.state
            .player1
            .energy_zone
            .cards
            .push(game.id("LL-E-001-SD"));
    }
    game.state.player1.energy_zone.set_active_count(5);
    fill_energy_deck(&mut game, 0, 2);
    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(&mut game, kanon, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "total 6 < 7 → blocked even though 5 are active"
    );

    game.state
        .player1
        .energy_zone
        .cards
        .push(game.new_id("LL-E-001-SD"));
    let zone_after_push = game.state.player1.energy_zone.cards.len();
    assert_eq!(zone_after_push, 7);
    fire_trigger(&mut game, kanon, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_after_push + 1,
        "mixed 5 active + 2 wait = 7 ≥ 7 → placement happens"
    );
}

#[test]
fn empty_energy_deck_skips_group_and_energy_gated_placement_pl_sp_bp4_001_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp4-001-R");
    game.add_to_stage(MemberArea::Center, kanon);
    game.give_energy(8);

    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(&mut game, kanon, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "nothing to place from an empty deck — must not crash or misbehave"
    );
}
