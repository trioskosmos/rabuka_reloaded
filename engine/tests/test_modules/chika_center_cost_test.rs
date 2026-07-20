/// Tests for PL!S-bp3-001-R+ (高海千歌) — Center position enforcement.
///
/// chika_test.rs covers the success path from center and self-only target scope.
/// These verify activation is blocked from other positions.
use crate::helpers::*;

/// Failure: member on left side → cannot activate (center required)
#[test]
fn chika_center_cost_fails_in_left_side() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [chika, filler, -1];
    let result = game.try_activate_ability(chika);

    assert!(result.is_err(), "Should fail: requires center");
}

/// Failure: member on right side → cannot activate
#[test]
fn chika_center_cost_fails_in_right_side() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, filler, chika];
    let result = game.try_activate_ability(chika);

    assert!(result.is_err(), "Should fail: requires center");
}

/// Success from center (complementary to existing chika_test)
#[test]
fn chika_center_cost_succeeds_in_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");

    game.state.player1.stage.stage = [-1, chika, -1];
    game.activate_ability(chika);

    assert!(
        game.state.mods.get_orientation_modifier(chika) == Some("wait"),
        "Chika should be in wait state"
    );
}
