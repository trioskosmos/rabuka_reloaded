/// L0 gap coverage: Printemps / lilywhite / BiBi success-zone conditional
/// heart abilities (PL!-bp6-012-N, PL!-bp6-014-N, PL!-bp6-015-N).
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// PL!-bp6-012-N: 常時 成功ライブカード置き場に『Printemps』のカードがある場合、
/// heart03+1。
/// TODO: needs investigation — success zone condition may require
/// an actual live resolution rather than direct state injection.
#[test]
#[ignore = "success zone condition needs live flow"]
fn bp6_012_printemps_in_success_grants_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-bp6-012-N");
    game.state.player1.stage.stage = [-1, member, -1];
    // Put a Printemps card in the success zone
    let printemps_card = game.id("PL!-sd1-001-SD");
    game.state.player1.success_live_card_zone.cards.push(printemps_card);
    game.state.recalculate_constants();

    let h03 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart03);
    assert!(h03 >= 1, "Printemps in success zone → >= +1 heart03");
}

/// PL!-bp6-014-N: 常時 成功ライブカード置き場に『lilywhite』のカードがある場合、
/// heart01+1。
#[ignore = "success zone condition needs live flow"]
fn bp6_014_lilywhite_in_success_grants_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-bp6-014-N");
    game.state.player1.stage.stage = [-1, member, -1];
    let lilywhite_card = game.id("PL!-sd1-005-SD");
    game.state.player1.success_live_card_zone.cards.push(lilywhite_card);
    game.state.recalculate_constants();

    let h01 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart01);
    assert!(h01 >= 1, "lilywhite in success zone → >= +1 heart01");
}

/// PL!-bp6-015-N: 常時 成功ライブカード置き場に『BiBi』のカードがある場合、
/// heart06+1。
#[ignore = "success zone condition needs live flow"]
fn bp6_015_bibi_in_success_grants_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-bp6-015-N");
    game.state.player1.stage.stage = [-1, member, -1];
    let bibi_card = game.id("PL!-sd1-002-SD");
    game.state.player1.success_live_card_zone.cards.push(bibi_card);
    game.state.recalculate_constants();

    let h06 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart06);
    assert!(h06 >= 1, "BiBi in success zone → >= +1 heart06");
}
