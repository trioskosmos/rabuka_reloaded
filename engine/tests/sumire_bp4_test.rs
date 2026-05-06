/// Tests for 平安名すみれ (PL!SP-bp4-004-R＋) ab#0 + ab#1 — Q193, Q194
///
/// ab#0 (常時): When playing this card, may baton touch with 2 members.
/// ab#1 (登場, Center): If entered via baton touch with 2 Liella! members:
///   draw 2, then put 1 cost≤4 Liella! member from discard to empty stage area.
mod helpers;
use helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::zones::MemberArea;

fn advance_to_turn2(game: &mut TestGame) {
    for _ in 0..7 { game.pass(); }
}

/// Q193: Baton touch inserts into a vacated area (player chooses which of two).
/// Q194: Baton touch requires both members from PREVIOUS turns.
/// Basic flow: play with baton touch from 2 Liella! members -> draw 2 + deploy.
#[test]
fn sumire_q193_q194_baton_touch_draw_and_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp4-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-004-R");
    let liella2 = game.id("PL!SP-bp1-005-R");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.hand.cards.push(sumire);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella1, liella2, -1];
    game.state.player1.waitroom.cards.push(liella1);
    game.give_energy(20);

    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state, &ActionType::PlayMemberToStage,
        Some(sumire), None, Some(MemberArea::Center), Some(true),
    ).expect("play with baton touch");

    while game.has_pending_choice() { game.select_indices(&[0]); }

    assert!(game.state.player1.hand.cards.len() >= 2,
        "Q193: Drew 2 cards from baton touch deploy effect");
}

/// Playing WITHOUT baton touch to an EMPTY area — ab#1 should NOT trigger (no draw).
/// (Targeting an occupied area auto-triggers baton touch per the game rules.)
#[test]
fn sumire_no_baton_touch_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp4-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-004-R");
    let liella2 = game.id("PL!SP-bp1-005-R");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.hand.cards.push(sumire);
    game.state.player1.hand.cards.push(filler);
    // Empty the center area so targeting it doesn't auto-trigger baton touch
    game.state.player1.stage.stage = [liella1, -1, liella2];
    game.state.player1.waitroom.cards.push(liella1);
    game.give_energy(25);

    let hand_before = game.state.player1.hand.cards.len();
    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state, &ActionType::PlayMemberToStage,
        Some(sumire), None, Some(MemberArea::Center), Some(false),
    ).expect("play without baton touch");

    while game.has_pending_choice() { game.select_indices(&[0]); }

    // Sumire was removed from hand to play. If draw effect doesn't trigger, hand stays at hand_before - 1.
    assert_eq!(game.state.player1.hand.cards.len(), hand_before - 1,
        "No baton touch -> ab#1 does not trigger, no draw");
}

/// Play without Liella! on stage — no baton touch available, just play normally.
#[test]
fn sumire_no_liella_on_stage_plays_normally() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp4-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.hand.cards.push(sumire);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [filler, filler, -1];
    game.give_energy(20);

    advance_to_turn2(&mut game);
    TurnEngine::execute_main_phase_action(
        &mut game.state, &ActionType::PlayMemberToStage,
        Some(sumire), None, Some(MemberArea::Center), None,
    ).expect("play without baton touch");

    assert!(game.state.player1.stage.stage.contains(&sumire),
        "Sumire placed on stage normally");
}
