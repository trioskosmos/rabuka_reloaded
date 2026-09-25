/// Tests for 葉月 恋 (PL!SP-bp4-005-R＋):
///
/// ab#0 (登場): If baton-touched from a Liella! member AND energy ≥ 7,
///   put 2 energy cards from energy deck in wait state.
///
/// Bug: The group check was missing — Ren gained energy even when baton-touched
/// from a non-Liella! member.
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn advance_to_turn2(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
    }
}

/// Ren's ab#0 should trigger when baton-touched from a Liella! member
/// and energy ≥ 7.
#[test]
fn ren_baton_touch_from_liella_places_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp4-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let liella = game.id("PL!SP-sd1-001-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(ren);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella, -1, -1];
    // Give 15 energy to the zone AND seed the energy deck for ab#0's effect
    game.give_energy(15);
    let energy_card = game.id("LL-E-001-SD");
    for _ in 0..5 {
        game.state.player1.energy_deck.cards.push(energy_card);
    }

    advance_to_turn2(&mut game);

    let energy_before = game.state.player1.energy_zone.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(ren),
        None,
        Some(MemberArea::LeftSide),
        Some(true),
    )
    .expect("play with baton touch");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // ab#0: 2 energy cards placed in wait state from energy deck
    assert!(
        game.state.player1.energy_zone.cards.len() >= energy_before + 2,
        "2 energy cards added when baton-touched from Liella! with ≥7 energy"
    );
    assert!(
        game.state.player1.stage.stage.contains(&ren),
        "Ren placed on stage"
    );
}

/// Ren's ab#0 should NOT trigger when baton-touched from a non-Liella! member,
/// even with sufficient energy. (Bug regression test — group_names check)
#[test]
fn ren_baton_touch_from_non_liella_no_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp4-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    // Use a character from a different series (not Liella!) — e.g., 高坂穂乃果 (μ's)
    let non_liella = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(ren);
    game.state.player1.hand.cards.push(filler);
    // non-Liella! member on stage (μ's series, not Liella!)
    game.state.player1.stage.stage = [non_liella, -1, -1];
    game.give_energy(15);

    advance_to_turn2(&mut game);

    let energy_before = game.state.player1.energy_zone.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(ren),
        None,
        Some(MemberArea::LeftSide),
        Some(true),
    )
    .expect("play with baton touch");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // ab#0 should NOT trigger because baton-touched from non-Liella! member
    // The group_names check in engine should reject this
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before,
        "No energy added when baton-touched from non-Liella! member"
    );
    assert!(
        game.state.player1.stage.stage.contains(&ren),
        "Ren placed on stage anyway"
    );
}

/// Ren's ab#0 should NOT trigger with insufficient energy (<7),
/// even from a Liella! baton touch.
#[test]
fn ren_baton_touch_liella_insufficient_energy_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp4-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let liella = game.id("PL!SP-sd1-001-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(ren);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella, -1, -1];
    // Give 10 energy. After baton touch (cost replaced by liella's 11):
    // Ren (15) - liella (11) = 4 cost, leaving 6 energy.
    // Ren's ab#0 condition requires ≥7 energy → should NOT trigger.
    game.give_energy(10);

    advance_to_turn2(&mut game);

    let energy_before = game.state.player1.energy_zone.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(ren),
        None,
        Some(MemberArea::LeftSide),
        Some(true),
    )
    .expect("play with baton touch");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before,
        "No energy added with only 6 energy"
    );
}
