use crate::helpers::*;

const KOTORI: &str = "PL!-bp5-003-R\u{ff0b}";
const FILLER: &str = "PL!-sd1-010-SD";

/// ab#0: 3 distinct names on stage → heart03 gained
#[test]
fn three_distinct_stage_names_grant_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    let filler_a = game.id(FILLER);
    let filler_b = game.id("PL!-sd1-014-SD"); // different card

    game.state.player1.stage.stage = [kotori, filler_a, filler_b];
    game.state.recalculate_constants();

    let heart03 = game.state.mods.get_heart_modifier(
        kotori,
        rabuka_engine::core::card::HeartColor::Heart03,
    );
    assert!(
        heart03 > 0,
        "ab#0 should grant heart03 with 3 distinct names on stage"
    );
}

/// ab#0: only 2 distinct names → no heart03
#[test]
fn two_distinct_stage_names_do_not_grant_heart03() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    let filler = game.id(FILLER);

    game.state.player1.stage.stage = [kotori, filler, -1];
    game.state.recalculate_constants();

    let heart03 = game.state.mods.get_heart_modifier(
        kotori,
        rabuka_engine::core::card::HeartColor::Heart03,
    );
    assert_eq!(
        heart03, 0,
        "ab#0 should NOT grant heart03 with only 2 distinct names"
    );
}
