use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn ren_pl_sp_pb2_027_n_energy_setup(game: &mut TestGame, energy: usize) -> i16 {
    let ren = game.id("PL!SP-pb2-027-N");
    game.state.player1.stage.stage[1] = ren;
    game.give_energy(energy);
    game.state.recalculate_constants();
    ren
}

#[test]
fn ren_pl_sp_pb2_027_n_five_energy_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = ren_pl_sp_pb2_027_n_energy_setup(&mut game, 5);
    assert_eq!(
        game.state.mods.get_heart_modifier(ren, HeartColor::Heart03),
        0,
        "5 energy (<6) -> no heart03"
    );
}

#[test]
fn ren_pl_sp_pb2_027_n_six_energy_one_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = ren_pl_sp_pb2_027_n_energy_setup(&mut game, 6);
    assert_eq!(
        game.state.mods.get_heart_modifier(ren, HeartColor::Heart03),
        1,
        "6 energy (>=6, <8) -> exactly one heart03"
    );
}

#[test]
fn ren_pl_sp_pb2_027_n_seven_energy_still_one_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = ren_pl_sp_pb2_027_n_energy_setup(&mut game, 7);
    assert_eq!(
        game.state.mods.get_heart_modifier(ren, HeartColor::Heart03),
        1,
        "7 energy (<8) -> still one heart03"
    );
}

#[test]
fn ren_pl_sp_pb2_027_n_eight_energy_two_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = ren_pl_sp_pb2_027_n_energy_setup(&mut game, 8);
    assert_eq!(
        game.state.mods.get_heart_modifier(ren, HeartColor::Heart03),
        2,
        "8 energy (>=8) -> both heart03"
    );
}
