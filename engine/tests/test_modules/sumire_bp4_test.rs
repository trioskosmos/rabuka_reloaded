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
        Some(vec![0, 1]),
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("play with double baton via card_indices");

    // After baton touch + debut: draw 2 cards (auto).
    // If multiple matching cards in discard, a card selection choice appears.
    // If exactly 1 match, card auto-selected (no choice).
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    // If multiple empty stage slots, choose deployment position.
    if game.has_pending_choice() {
        game.select_option(0);
    }

    // Q194: Double baton touch replaced 2 members
    assert_eq!(
        game.state.baton_touch_count_p1, 2,
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

    // Both occupied areas (Left & Center) should have their cards tracked as deployed this turn
    let on_stage_after = game.state.player1.stage.stage;
    assert!(
        game.state
            .player1
            .deployed_this_turn
            .contains(&on_stage_after[0]),
        "LeftSide card should be in deployed_this_turn"
    );
    assert!(
        game.state
            .player1
            .deployed_this_turn
            .contains(&on_stage_after[1]),
        "Center card should be in deployed_this_turn"
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

    // Lock LeftSide AFTER advance (which clears deployed_this_turn)
    // Simulate liella1 being deployed this turn by adding its card ID to the tracking set.
    if !game.state.player1.deployed_this_turn.contains(&liella1) {
        game.state.player1.deployed_this_turn.push(liella1);
    }

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
        game.state.baton_touch_count_p1, 1,
        "Q194: Only 1 baton touch — LeftSide had a member deployed this turn"
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
        game.state.baton_touch_count_p1, 1,
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
        Some(vec![0, 1]),
        Some(MemberArea::LeftSide),
        Some(true),
    )
    .expect("play with double baton via card_indices");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Double baton occurred (2 replacements)
    assert_eq!(game.state.baton_touch_count_p1, 2, "Double baton occurred");

    // Sumire placed at Left (non-Center) — ab#1 should NOT trigger (position check fails)
    // Hand should only have the filler card (1 card), no draw from ab#1
    let hand_len = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_len, 1,
        "No draw: ab#1 requires Center position, Sumire is at Left"
    );

    // Left (where Sumire was placed) should be locked — Sumire was deployed this turn.
    // Center was vacated by double baton but is now empty — no card to lock.
    assert!(
        game.state.player1.deployed_this_turn.contains(&sumire),
        "Sumire should be tracked as deployed this turn"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], -1,
        "Center is empty after double baton"
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

/// Single baton (area button with baton toggle) should NOT auto-promote to double.
/// With 2+ members on stage, only 1 replacement occurs, ab#1 does NOT trigger.
#[test]
fn sumire_single_baton_stays_single_no_auto_promote() {
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

    // Use regular area button path (no card_indices) — single baton only
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(sumire),
        None,
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("play with single baton");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Only 1 baton touch — no auto-promotion
    assert_eq!(
        game.state.baton_touch_count_p1, 1,
        "Single baton stays single — 1 baton touch, not 2"
    );

    // ab#1 requires min_baton_touch_count=2, so no draw
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Only filler in hand — no draw from ab#1"
    );

    // Only center was replaced
    let in_waitroom = &game.state.player1.waitroom.cards;
    let liella_in_waitroom = [liella1, liella2]
        .iter()
        .filter(|&&id| in_waitroom.contains(&id))
        .count();
    assert_eq!(
        liella_in_waitroom, 1,
        "Only 1 member replaced (center), left stays"
    );

    // Left still occupied
    assert_eq!(
        game.state.player1.stage.stage[0], liella1,
        "Left still has liella1"
    );
}

/// Helper that mimics the web server's action type parsing path.
/// This is the exact code path the button click goes through:
///   1. UI sends action_type as a string (e.g. "play_member_to_stage")
///   2. Web server parses it via FromStr
///   3. If parsing fails, defaults to Pass (skips turn)
///   4. Otherwise executes the action
fn execute_action_via_string_parsing(
    game: &mut TestGame,
    action_type_str: &str,
    card_id: Option<i16>,
    card_indices: Option<Vec<usize>>,
    stage_area: Option<MemberArea>,
    use_baton_touch: Option<bool>,
) -> Result<(), String> {
    // Same parsing logic as web_server.rs execute_action handler
    let action_type = action_type_str
        .parse::<ActionType>()
        .unwrap_or(ActionType::Pass);
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &action_type,
        card_id,
        card_indices,
        stage_area,
        use_baton_touch,
    )
}

/// Verify the string-parsing path works for the correct snake_case format
/// that the fixed button now sends.
#[test]
fn sumire_action_type_string_parsing_play_member_to_stage() {
    let parsed: Result<ActionType, String> = "play_member_to_stage".parse();
    assert_eq!(
        parsed.expect("snake_case should parse"),
        ActionType::PlayMemberToStage
    );

    let parsed: Result<ActionType, String> = "PlayMemberToStage".parse();
    assert!(parsed.is_err(), "PascalCase should fail to parse");

    // This is what the web server does on parse failure — defaults to Pass
    let fallback = "PlayMemberToStage"
        .parse::<ActionType>()
        .unwrap_or(ActionType::Pass);
    assert_eq!(
        fallback,
        ActionType::Pass,
        "Failed parse falls back to Pass → turn skips"
    );
}

/// Full integration test that exercises the button path end-to-end:
/// the action_type string goes through the same parsing logic as the web server.
#[test]
fn sumire_double_baton_integration_via_string_path() {
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

    // This is what the fixed double baton button sends:
    // action_type = "play_member_to_stage" (snake_case)
    // card_indices = [0, 1] (replace left & center)
    // stage_area = "center" (place Sumire in center)
    // use_baton_touch = true
    execute_action_via_string_parsing(
        &mut game,
        "play_member_to_stage",
        Some(sumire),
        Some(vec![0, 1]),
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("double baton via string-parsed action_type should succeed");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.baton_touch_count_p1, 2,
        "2 baton touches via button integration path"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], sumire,
        "Sumire occupies Center"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "Hand: filler + 2 draws from ab#1"
    );

    // Both occupied areas should have their cards tracked as deployed this turn
    let on_stage = game.state.player1.stage.stage;
    assert!(
        game.state.player1.deployed_this_turn.contains(&on_stage[0]),
        "LeftSide card should be in deployed_this_turn"
    );
    assert!(
        game.state.player1.deployed_this_turn.contains(&on_stage[1]),
        "Center card (Sumire) should be in deployed_this_turn"
    );
}

/// Explicit double baton via card_indices parameter (UI double-baton button path).
#[test]
fn sumire_explicit_double_baton_via_card_indices() {
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

    let energy_before = game.state.player1.energy_zone.active_count();
    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(sumire),
        Some(vec![0, 1]),
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("explicit double baton via card_indices");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(game.state.baton_touch_count_p1, 2, "2 baton touches");
    assert_eq!(
        game.state.baton_touch_arriving_card_id,
        Some(sumire),
        "baton_touch_arriving_card_id set"
    );

    let on_stage = game.state.player1.stage.stage;
    assert_eq!(
        on_stage[0], liella2,
        "Left has deployed liella2 (cost 2 <= 4)"
    );
    assert_eq!(on_stage[1], sumire, "Center has Sumire");
    assert_eq!(on_stage[2], -1, "Right empty");

    assert!(
        game.state.player1.waitroom.cards.contains(&liella1),
        "liella1 in waitroom (cost 15 > 4)"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&liella2),
        "liella2 deployed back to stage"
    );

    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "Hand: filler + 2 draws"
    );

    let expected_cost = 5u32;
    let energy_after = game.state.player1.energy_zone.active_count();
    assert_eq!(
        energy_after,
        energy_before.saturating_sub(expected_cost as usize),
        "Energy: paid {}, remaining {}",
        expected_cost,
        energy_after
    );

    // Both occupied areas should have their cards tracked as deployed this turn
    let on_stage = game.state.player1.stage.stage;
    assert!(
        game.state.player1.deployed_this_turn.contains(&on_stage[0]),
        "Left card (deployed liella2) should be in deployed_this_turn"
    );
    assert!(
        game.state.player1.deployed_this_turn.contains(&on_stage[1]),
        "Center card (Sumire) should be in deployed_this_turn"
    );
}

/// Explicit double baton to non-Center position should NOT trigger ab#1.
#[test]
fn sumire_explicit_double_baton_to_left_no_debut() {
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

    let energy_before = game.state.player1.energy_zone.active_count();
    advance_to_turn2(&mut game);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(sumire),
        Some(vec![0, 1]),
        Some(MemberArea::LeftSide),
        Some(true),
    )
    .expect("explicit double baton to Left");

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(game.state.baton_touch_count_p1, 2, "2 baton touches");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "No draw: center required"
    );

    let expected_cost = 5u32;
    let energy_after = game.state.player1.energy_zone.active_count();
    assert_eq!(
        energy_after,
        energy_before.saturating_sub(expected_cost as usize),
        "Energy: paid {}, remaining {}",
        expected_cost,
        energy_after
    );

    assert_eq!(game.state.player1.stage.stage[0], sumire, "Sumire at Left");
    assert_eq!(game.state.player1.stage.stage[1], -1, "Center empty");
    assert_eq!(game.state.player1.stage.stage[2], -1, "Right empty");

    // Only Sumire was deployed this turn (Center is empty, no card to lock)
    assert!(
        game.state.player1.deployed_this_turn.contains(&sumire),
        "Sumire should be in deployed_this_turn"
    );
    assert_eq!(
        game.state.player1.deployed_this_turn.len(),
        1,
        "Only Sumire deployed this turn"
    );
}
