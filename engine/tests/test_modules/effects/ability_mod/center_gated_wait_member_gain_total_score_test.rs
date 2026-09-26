/// Tests for PL!S-bp3-001-R+ (高海千歌) — Center position enforcement.
///
/// chika_test.rs covers the success path from center and self-only target scope.
/// These verify activation is blocked from other positions.
use crate::helpers::*;

/// 高海千歌 PL!S-bp3-001-R＋ is センター限定, and her 起動 cost waits herself.
/// A wrong-area refusal is therefore cheap to confuse with a cost refusal, so
/// these assert the specific thing that must NOT have happened: she was not
/// waited, and the attempt recorded no use.
fn assert_refused_outside_center(game: &TestGame, chika: i16, area: &str) {
    assert_ne!(
        game.state.mods.get_orientation_modifier(chika),
        Some("wait"),
        "{area}: the センター cost must not be charged when the ability is \
         refused for being out of position"
    );
    game.assert_use_not_recorded(chika, 0, &format!("refused in {area}"));
}

/// Failure: member on left side → cannot activate (center required)
#[test]
fn wait_member_gain_total_score_activation_fails_in_left_side() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [chika, filler, -1];
    let result = game.try_activate_ability(chika);

    assert!(result.is_err(), "Should fail: requires center");
    assert_refused_outside_center(&game, chika, "left side");
}

/// Failure: member on right side → cannot activate
#[test]
fn wait_member_gain_total_score_activation_fails_in_right_side() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, filler, chika];
    let result = game.try_activate_ability(chika);

    assert!(result.is_err(), "Should fail: requires center");
    assert_refused_outside_center(&game, chika, "right side");
}

/// Success from center (complementary to existing chika_test)
#[test]
fn wait_member_gain_total_score_activation_succeeds_in_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");

    game.state.player1.stage.stage = [-1, chika, -1];
    game.activate_ability(chika);

    assert!(
        game.state.mods.get_orientation_modifier(chika) == Some("wait"),
        "Chika should be in wait state"
    );
    // The centre case really did consume the use, so the two negatives above
    // are failing for position and not for a missing ability.
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(chika, 0, game.state.turn_number)),
        "activating from the centre must record the use"
    );
}
