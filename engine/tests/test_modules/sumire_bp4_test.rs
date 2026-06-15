/// Tests for 平安名すみれ (PL!SP-bp4-004-R＋) ab#0 + ab#1 — Q193, Q194
///
/// ab#0 (常時): When playing this card, may baton touch with 2 members.
/// ab#1 (登場, Center): If entered via baton touch with 2 Liella! members:
///   draw 2, then put 1 cost≤4 Liella! member from discard to empty stage area.
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn advance_to_turn2(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
    }
}

/// Q193: Baton touch inserts into a vacated area (player chooses which of two).
/// Q194: Baton touch requires both members from PREVIOUS turns.
/// Basic flow: play with baton touch -> 2 members replaced, draw 2 + deploy.
#[test]
fn sumire_q193_q194_baton_touch_draw_and_deploy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp4-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-004-R");
    let liella2 = game.id("PL!SP-bp1-005-R");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(sumire);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella1, liella2, -1];
    game.give_energy(20);

    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(sumire),
        None,
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("play with baton touch");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Q194: Double baton touch replaced 2 members
    assert_eq!(
        game.state.baton_touch_count, 2,
        "Q194: 2 baton touches recorded (double baton)"
    );

    // The ab#1 deploys one Liella member from discard to the other baton-passed position.
    // Stage was [liella1(0/Left), liella2(1/Center), -1(2/Right)]
    // Double baton replaces both Left(0) and Center(1), Sumire goes to Center(1).
    // The deployed card should go to Left(0) — the other vacated position, not the first empty slot.
    let on_stage = game.state.player1.stage.stage;
    let in_waitroom = &game.state.player1.waitroom.cards;
    let liella_ids = [liella1, liella2];
    let liella_on_stage = liella_ids
        .iter()
        .filter(|&&id| on_stage.contains(&id))
        .count();
    let liella_in_waitroom = liella_ids
        .iter()
        .filter(|&&id| in_waitroom.contains(&id))
        .count();
    assert_eq!(liella_on_stage, 1, "Q193: One Liella member deployed");
    assert!(
        liella_in_waitroom >= 1,
        "Q193: Other Liella stays in waitroom"
    );
    assert_eq!(on_stage[1], sumire, "Q193: Sumire occupies Center");
    assert_eq!(on_stage[2], -1, "Q193: Right area stays empty");
    // Deployed card goes to Left (the other baton-passed position, not Sumire's Center)
    assert_ne!(on_stage[0], -1, "Q193: Deployed card at Left");
    assert_eq!(on_stage[1], sumire, "Q193: Sumire occupies Center");
    assert_eq!(on_stage[2], -1, "Q193: Right area stays empty");
    assert!(
        in_waitroom.contains(&liella1) || in_waitroom.contains(&liella2),
        "Q193: At least one Liella member stays in waitroom"
    );
    assert_eq!(on_stage[1], sumire, "Q193: Sumire occupies Center");
    assert_eq!(on_stage[2], -1, "Q193: Right area stays empty");

    // 2 baton touches -> ab#1 condition passes -> draw 2 cards
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "Q193: Drew 2 cards = hand len 3"
    );
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
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(sumire);
    game.state.player1.hand.cards.push(filler);
    // Empty the center area so targeting it doesn't auto-trigger baton touch
    game.state.player1.stage.stage = [liella1, -1, liella2];
    game.give_energy(25);

    let hand_before = game.state.player1.hand.cards.len();
    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(sumire),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("play without baton touch");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Sumire was removed from hand to play. If draw effect doesn't trigger, hand stays at hand_before - 1.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "No baton touch -> ab#1 does not trigger, no draw"
    );
}

/// Q194: If the second member debuted this turn (area locked), double baton touch
/// should NOT replace the locked member. Only 1 baton touch occurs → ab#1 does NOT trigger.
#[test]
fn sumire_q194_locked_member_excluded_from_double_baton() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp4-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-004-R");
    let liella2 = game.id("PL!SP-bp1-005-R");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(sumire);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella1, liella2, -1];
    game.give_energy(20);

    advance_to_turn2(&mut game);

    // Lock LeftSide AFTER advance (which clears areas_locked_this_turn)
    game.state
        .player1
        .areas_locked_this_turn
        .insert(MemberArea::LeftSide);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(sumire),
        None,
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("play with baton touch");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Only 1 baton touch (center unlocked). LeftSide was locked → only 1 member replaced.
    // ab#1 should NOT trigger because min_baton_touch_count=2
    let _on_stage = game.state.player1.stage.stage;
    assert_eq!(
        game.state.baton_touch_count, 1,
        "Q194: Only 1 baton touch — LeftSide was locked (debuted this turn)"
    );
    // Only 1 Liella member in waitroom (the replaced center one)
    let in_waitroom = &game.state.player1.waitroom.cards;
    let liella_in_waitroom = [liella1, liella2]
        .iter()
        .filter(|&&id| in_waitroom.contains(&id))
        .count();
    assert_eq!(
        liella_in_waitroom, 1,
        "Q194: Only center member replaced; left member stays"
    );
    // No draw because ab#1 didn't trigger
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Q194: Only 1 card in hand (filler) — no draw from ab#1"
    );
}

/// Only 1 occupied area plus Sumire's target area = only 1 member is replaced.
/// ab#1 should NOT trigger because min_baton_touch_count=2.
#[test]
fn sumire_only_one_occupied_area_triggers_single_baton_touch() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp4-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-004-R");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(sumire);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella1, -1, -1];
    game.give_energy(20);

    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(sumire),
        None,
        Some(MemberArea::LeftSide),
        Some(true),
    )
    .expect("play with baton touch");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.baton_touch_count, 1,
        "Only 1 baton touch (only 1 occupied area)"
    );
    // ab#1 should NOT trigger (only 1 baton touch, needs 2)
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "No draw from ab#1 — only 1 baton touch occurred"
    );
}

/// Double baton to Left (non-Center) should NOT activate ab#1.
/// ab#1 requires Center position.
#[test]
fn sumire_double_baton_to_left_does_not_activate_debut() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp4-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let liella1 = game.id("PL!SP-bp1-004-R");
    let liella2 = game.id("PL!SP-bp1-005-R");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(sumire);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [liella1, liella2, -1];
    game.give_energy(20);

    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(sumire),
        None,
        Some(MemberArea::LeftSide),
        Some(true),
    )
    .expect("play with baton touch");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Double baton occurred (2 replacements)
    assert_eq!(game.state.baton_touch_count, 2, "Double baton occurred");

    // Sumire placed at Left (non-Center) — ab#1 should NOT trigger (position check fails)
    // Hand should only have the filler card (1 card), no draw from ab#1
    let hand_len = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_len, 1,
        "No draw: ab#1 requires Center position, Sumire is at Left"
    );
}

/// Play without Liella! on stage — no baton touch available, just play normally.
#[test]
fn sumire_no_liella_on_stage_plays_normally() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sumire = game.id("PL!SP-bp4-004-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(sumire);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [filler, filler, -1];
    game.give_energy(20);

    advance_to_turn2(&mut game);
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(sumire),
        None,
        Some(MemberArea::Center),
        None,
    )
    .expect("play without baton touch");

    assert!(
        game.state.player1.stage.stage.contains(&sumire),
        "Sumire placed on stage normally"
    );
}
