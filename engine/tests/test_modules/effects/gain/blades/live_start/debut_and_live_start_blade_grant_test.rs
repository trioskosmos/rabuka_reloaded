use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const AQOURS_MEMBER: &str = "PL!S-sd1-001-SD";

#[test]
fn hs_cl1_006_debut_grants_three_blades_until_live_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-cl1-006-CL");
    game.state.player1.stage.stage[1] = me;

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        3,
        "debut grants +3 blades until live end"
    );
}

#[test]
fn s_sd1_022_live_start_grants_blades_only_to_aqours_stage_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-sd1-022-SD");
    game.state.player1.live_card_zone.cards.push(live);

    let a1 = game.id(AQOURS_MEMBER); // Aqours
    let a2 = game.new_id("PL!S-sd1-002-SD"); // Aqours, second copy
    let non_aqours = game.id("PL!HS-bp5-004-R"); // スリーズブーケ — must NOT gain
    game.state.player1.stage.stage[0] = a1;
    game.state.player1.stage.stage[1] = a2;
    game.state.player1.stage.stage[2] = non_aqours;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert!(
        game.state.mods.get_blade_modifier(a1) >= 1,
        "Aqours member 1 gains a blade"
    );
    assert!(
        game.state.mods.get_blade_modifier(a2) >= 1,
        "Aqours member 2 gains a blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(non_aqours),
        0,
        "non-Aqours member must not gain"
    );
}
