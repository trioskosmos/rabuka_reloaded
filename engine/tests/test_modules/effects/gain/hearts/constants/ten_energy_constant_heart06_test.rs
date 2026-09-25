use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn ten_energy_constant_grants_two_heart06_with_twelve_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!SP-bp5-016-N"); // 葉月恋: energy≥10 → heart06×2
    game.state.player1.stage.stage = [-1, member, -1];
    let fid = game.id_ref("PL!-sd1-010-SD");
    fill_decks(&mut game, fid);
    game.give_energy(12);
    game.state.recalculate_constants();

    let h06 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart06);
    assert_eq!(h06, 2, "12 energy ≥ 10 → +2 heart06");

    // Negative: drop below threshold
    game.state.player1.energy_zone.sub_active(5);
    game.state.recalculate_constants();
}
