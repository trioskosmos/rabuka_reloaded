use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn advance_to_turn2(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
    }
}

/// Single baton to a non-protected zone succeeds when a
/// cannot_baton_touch card occupies a DIFFERENT area.
#[test]
fn single_baton_to_non_protected_zone_succeeds() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let protected = game.id("LL-bp2-001-R\u{ff0b}");
    let filler_stage = game.id("PL!-sd1-001-SD");
    let arriving = game.id("PL!-sd1-010-SD");

    // Protected card on Left, unprotected filler on Center
    game.state.player1.stage.stage = [protected, filler_stage, -1];
    game.state.player1.hand.cards.push(arriving);
    game.give_energy(10);

    // Play arriving to Center (area 1) with baton touch — should succeed
    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(arriving),
        None,
        Some(MemberArea::Center),
        Some(true),
    );
    assert!(
        result.is_ok(),
        "Single baton to non-protected zone should succeed: {:?}",
        result
    );
    // Protected card should still be on Left
    assert_eq!(game.state.player1.stage.stage[0], protected);
    // Arriving card should be on Center (replaced filler)
    assert_eq!(game.state.player1.stage.stage[1], arriving);
}

/// Double baton fails when one of the two target areas contains
/// a member with cannot_baton_touch protection.
#[test]
fn double_baton_to_protected_zone_rejected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let protected = game.id("LL-bp2-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-001-SD");
    let sumire = game.id("PL!SP-bp4-004-R\u{ff0b}");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    // Protected on Left, filler on Center — double baton targets BOTH
    game.state.player1.stage.stage = [protected, filler, -1];
    game.state.player1.hand.cards.push(sumire);
    game.give_energy(30);

    advance_to_turn2(&mut game);

    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(sumire),
        Some(vec![0, 1]),
        Some(MemberArea::Center),
        Some(true),
    );
    assert!(
        result.is_err(),
        "Double baton should be rejected when one target has cannot_baton_touch"
    );
}

/// Double baton succeeds when NEITHER of the two target areas
/// contains a member with cannot_baton_touch, even if a protected
/// member occupies a third (untargeted) area.
#[test]
fn double_baton_to_non_protected_zones_succeeds() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let protected = game.id("LL-bp2-001-R\u{ff0b}");
    let filler1 = game.id("PL!-sd1-010-SD");
    let filler2 = game.id("PL!-sd1-001-SD");
    let sumire = game.id("PL!SP-bp4-004-R\u{ff0b}");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler1);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler1);
    }
    // Protected on Right (area 2), two fillers on Left & Center
    game.state.player1.stage.stage = [filler1, filler2, protected];
    game.state.player1.hand.cards.push(sumire);
    game.give_energy(30);

    advance_to_turn2(&mut game);

    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(sumire),
        Some(vec![0, 1]),
        Some(MemberArea::Center),
        Some(true),
    );
    assert!(
        result.is_ok(),
        "Double baton to non-protected zones should succeed: {:?}",
        result
    );
    // 2 baton touches recorded
    assert_eq!(game.state.baton_touch_count_p1, 2);
    // Protected card on Right should be untouched
    assert_eq!(game.state.player1.stage.stage[2], protected);
}

/// Opponent can baton touch freely during their own turn even when
/// player1 has a cannot_baton_touch card on the field.
#[test]
fn opponent_baton_touch_unaffected_by_protected_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let protected = game.id("LL-bp2-001-R\u{ff0b}");
    let filler1 = game.id("PL!-sd1-010-SD");
    let filler2 = game.id("PL!-sd1-010-SD");
    let arriving = game.id("PL!-sd1-010-SD");

    // Player1: protected card on Left, filler on Center
    game.state.player1.stage.stage = [protected, filler1, -1];
    game.state.player1.hand.cards.push(filler2);
    game.give_energy(10);

    // Player2: filler (cost 4) on Left, arriving (cost 4) in hand
    // Baton touch cost = 4 - 4 = 0
    game.state.player2.stage.stage = [filler1, -1, -1];
    game.state.player2.hand.cards.push(arriving);
    for _ in 0..10 {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.add_active(10);

    // Advance past player1's main phase to player2's main phase
    for _ in 0..3 {
        game.pass();
    }

    // Player2's turn — baton touch filler on their Left
    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(arriving),
        None,
        Some(MemberArea::LeftSide),
        Some(true),
    );
    assert!(
        result.is_ok(),
        "Opponent baton touch should not be affected by player1's protected card: {:?}",
        result
    );
    assert_eq!(game.state.player2.stage.stage[0], arriving);
}
