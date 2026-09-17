use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn keke_pl_sp_bp7_013_n_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let koko = game.id("PL!SP-bp7-013-N");
    game.state.player1.stage.stage[0] = koko;
    koko
}

#[test]
fn pl_sp_bp7_013_n_constant_three_kaleidoscore_grants_heart06_and_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let koko = keke_pl_sp_bp7_013_n_setup(&mut game);
    let k2 = game.id("PL!SP-bp1-013-PR");
    let k3 = game.id("PL!SP-PR-012-PR");
    game.state.player1.stage.stage[1] = k2;
    game.state.player1.stage.stage[2] = k3;

    game.state.recalculate_constants();

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(koko, HeartColor::Heart06),
        1,
        "3 KALEIDOSCORE members -> heart06 granted"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(koko),
        1,
        "3 KALEIDOSCORE members -> blade granted"
    );
}

#[test]
fn pl_sp_bp7_013_n_constant_two_kaleidoscore_grants_no_heart06_or_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let koko = keke_pl_sp_bp7_013_n_setup(&mut game);
    let k2 = game.id("PL!SP-bp1-013-PR");
    game.state.player1.stage.stage[1] = k2;
    let outsider = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[2] = outsider;

    game.state.recalculate_constants();

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(koko, HeartColor::Heart06),
        0,
        "only 2 KALEIDOSCORE -> no heart06"
    );
    assert_eq!(game.state.mods.get_blade_modifier(koko), 0);
}
