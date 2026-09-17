use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn n_bp7_024_n_debut_gains_heart01_with_three_r3birth_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-bp7-024-N");
    let r1 = game.id("PL!N-bp1-023-PRproteinbar");
    let r2 = game.id("PL!N-bp1-024-PR");
    let r3 = game.id("PL!N-PR-012-PR");
    game.state.player1.stage.stage = [me, r1, r2];
    game.state.player2.stage.stage[0] = r3;

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    const H01: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart01;
    assert!(
        game.state.mods.get_heart_modifier(me, H01) > 0,
        "3 R3BIRTH members on stage -> heart01 until live end"
    );
}

#[test]
fn n_bp7_024_n_debut_gains_no_heart01_with_two_r3birth_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-bp7-024-N");
    let r1 = game.id("PL!N-bp1-023-PRproteinbar");
    let other = game.new_id(FILLER);
    game.state.player1.stage.stage = [me, r1, other];

    fire_trigger(&mut game, me, AbilityTrigger::Debut, "登場");

    const H01: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart01;
    assert_eq!(
        game.state.mods.get_heart_modifier(me, H01),
        0,
        "only 2 R3BIRTH members -> no heart01"
    );
}
