/// Tests for PL!SP-bp5-002-R+ — Left-side position requirement enforced via
/// activation_position on the effect (from {{leftside.png|左サイド}} icon).
///
/// 起動/左サイド/ターン1回: このメンバーをウェイトにする：
///   カードを3枚引き、手札を2枚控え室に置く。
use crate::helpers::*;

/// Success: member on left side → ability activates
#[test]
fn sp_bp5_leftside_cost_success_in_left_side() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp5-002-R+");
    let f1 = game.id("PL!-sd1-010-SD");
    let f2 = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [card, f1, f2];
    game.activate_ability(card);

    assert!(
        game.state.mods.get_orientation_modifier(card) == Some(&"wait".to_string()),
        "Cost paid: card should be in wait state"
    );
}

/// Failure: member on center → cannot activate
#[test]
fn sp_bp5_leftside_cost_fails_in_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp5-002-R+");
    let f = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [f, card, -1];
    let stage_before = game.player().stage.stage.clone();
    let result = game.try_activate_ability(card);

    assert!(result.is_err(), "Should fail when not on left side");
    assert!(game.player().stage.stage == stage_before, "Stage unchanged");
}

/// Failure: member on right side → cannot activate
#[test]
fn sp_bp5_leftside_cost_fails_in_right_side() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp5-002-R+");
    let f = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [f, -1, card];
    let result = game.try_activate_ability(card);

    assert!(result.is_err(), "Should fail when not on left side");
}
