use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn pl_sp_sd1_026_sd_srl_nine_energy_grants_one_score() {
    let db = load_real_database();
    for variant in ["PL!SP-sd1-026-SD", "PL!SP-sd1-026-SRL"] {
        let mut game = TestGame::new(db.clone());
        let live = game.id(variant);
        game.state.player1.live_card_zone.cards.push(live);
        game.give_energy(9);

        fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

        assert_eq!(
            game.state.mods.get_score_modifier(live),
            1,
            "{}: 9 energy -> score +1",
            variant
        );
    }
}

#[test]
fn pl_sp_sd1_026_sd_eight_energy_grants_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-sd1-026-SD");
    game.state.player1.live_card_zone.cards.push(live);
    game.give_energy(8);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "8 energy -> no score"
    );
}
