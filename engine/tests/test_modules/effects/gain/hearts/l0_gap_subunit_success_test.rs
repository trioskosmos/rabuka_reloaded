/// L0 gap coverage: Printemps / lilywhite / BiBi success-zone conditional
/// heart abilities (PL!-bp6-012-N, PL!-bp6-014-N, PL!-bp6-015-N).
///
/// The 成功ライブカード置き場 (success live card zone) only ever contains
/// LIVE cards in a real game, so each test seeds it with a live card of the
/// matching subunit and asserts both the positive and negative case.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

const SUBUNIT_LIVE_CASES: &[(&str, &str, HeartColor, &str)] = &[
    (
        "PL!-bp6-012-N",
        "PL!-pb1-028-L", // WAO-WAO Powerful day! (Printemps)
        HeartColor::Heart03,
        "Printemps live in success zone → exactly +1 heart03",
    ),
    (
        "PL!-bp6-014-N",
        "PL!-pb1-029-L", // lilywhite live
        HeartColor::Heart01,
        "lilywhite live in success zone → exactly +1 heart01",
    ),
    (
        "PL!-bp6-015-N",
        "PL!-pb1-030-L", // BiBi live
        HeartColor::Heart06,
        "BiBi live in success zone → exactly +1 heart06",
    ),
];

fn assert_success_zone_group_heart(member_no: &str, live_no: &str, color: HeartColor, msg: &str) {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id(member_no);
    game.state.player1.stage.stage = [-1, member, -1];

    // Negative first: empty success zone → no heart.
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(member, color), 0, "{msg} (negative case)");

    // Positive: a matching-subunit LIVE card in the success zone.
    let live = game.id(live_no);
    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(member, color), 1, "{msg}");

    // Non-matching subunit live card must NOT count: swap in a GuiltyKiss
    // live and confirm the modifier drops back to 0.
    *game.state.player1.success_live_card_zone.cards.last_mut().unwrap() =
        game.new_id("PL!S-pb1-021-L");
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(member, color), 0, "{msg} (wrong subunit)");
}

#[test]
fn bp6_012_printemps_in_success_grants_heart03() {
    let (m, l, c, msg) = SUBUNIT_LIVE_CASES[0];
    assert_success_zone_group_heart(m, l, c, msg);
}

#[test]
fn bp6_014_lilywhite_in_success_grants_heart01() {
    let (m, l, c, msg) = SUBUNIT_LIVE_CASES[1];
    assert_success_zone_group_heart(m, l, c, msg);
}

#[test]
fn bp6_015_bibi_in_success_grants_heart06() {
    let (m, l, c, msg) = SUBUNIT_LIVE_CASES[2];
    assert_success_zone_group_heart(m, l, c, msg);
}
