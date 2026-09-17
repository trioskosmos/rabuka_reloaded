use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn revealed_live_places_wait_energy_pl_s_pb1_007_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-pb1-007-R");
    game.add_to_stage(MemberArea::Center, hanamaru);
    fill_energy_deck(&mut game, 0, 2);
    game.state.revealed_cards.push(game.id("PL!-sd1-019-SD"));

    let zone_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();

    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "live card among revealed cards → place 1 energy"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before,
        "energy enters in WAIT state"
    );
}

#[test]
fn member_only_or_empty_reveal_places_no_energy_pl_s_pb1_007_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-pb1-007-R");
    game.add_to_stage(MemberArea::Center, hanamaru);
    fill_energy_deck(&mut game, 0, 2);

    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "no reveal → no placement"
    );

    game.state.revealed_cards.push(game.new_id(FILLER));
    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "member-only reveal → ライブカードが1枚以上 fails"
    );
}

#[test]
fn multiple_revealed_lives_still_place_only_one_energy_pl_s_pb1_007_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-pb1-007-R");
    game.add_to_stage(MemberArea::Center, hanamaru);
    fill_energy_deck(&mut game, 0, 3);

    game.state.revealed_cards.push(game.id("PL!-sd1-019-SD"));
    game.state.revealed_cards.push(game.new_id(FILLER));
    game.state.revealed_cards.push(game.new_id("PL!-sd1-021-SD"));

    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "two revealed lives + members → STILL exactly 1 energy"
    );
}

#[test]
fn empty_energy_deck_skips_revealed_live_placement_pl_s_pb1_007_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanamaru = game.id("PL!S-pb1-007-R");
    game.add_to_stage(MemberArea::Center, hanamaru);
    game.state
        .revealed_cards
        .push(game.id("PL!-sd1-019-SD"));

    let zone_before = game.state.player1.energy_zone.cards.len();
    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "empty energy deck → nothing placed, no crash"
    );
}
