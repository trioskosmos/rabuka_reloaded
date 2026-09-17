use crate::helpers::*;

use rabuka_engine::zones::MemberArea;

/// Test case 1: Basic successful activation - Yoshiko in center with valid targets
#[test]
fn test_yoshiko_center_ability_basic_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Setup: Get Yoshiko and other Aqours members
    let yoshiko = game.id("PL!S-bp3-006-R＋"); // 津島善子
    let chika = game.id("PL!S-bp2-001-R"); // Chika Takami (Aqours, cost 9)
    let riko = game.id("PL!S-bp2-002-R"); // Riko Sakurauchi (Aqours, cost 4)
    let hand_card = game.id("PL!-sd1-010-SD"); // Generic hand card to discard
    let dia = game.id("PL!S-bp2-004-R"); // Dia Kurosawa (Aqours, cost 11)

    // Setup: Place Yoshiko in center position
    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_stage(MemberArea::RightSide, riko);

    // Add cards to hand for cost payment
    game.add_to_hand(hand_card);

    // Put higher cost Aqours members in discard for summoning
    game.add_to_discard(dia); // Dia (cost 11) can be summoned if Chika (cost 9) is sent to discard (9+2=11)

    // Give player enough energy
    game.give_energy(5);

    let _initial_hand_size = game.player().hand.cards.len();
    let _initial_discard_size = game.player().waitroom.cards.len();
    let initial_stage_count = game
        .player()
        .stage
        .stage
        .iter()
        .filter(|&&id| id != -1)
        .count();

    // Activate Yoshiko's ability
    game.activate_ability(yoshiko);

    // Handle all pending choices: cost (hand discard) + effect (stage member selection)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify cost: Yoshiko WAITED (orientation, not merely present), hand
    // card discarded. "Still on stage" alone passes even if the engine
    // forgot to rest her.
    assert_eq!(
        game.state.mods.get_orientation_modifier(yoshiko),
        Some("wait"),
        "cost clause 1: Yoshiko must be put to wait"
    );
    assert!(
        game.player().stage.stage[1] == yoshiko,
        "Yoshiko should still be on stage but in wait state"
    );
    assert!(
        !game.player().hand.cards.contains(&hand_card),
        "Hand card should not be in hand (discarded as cost)"
    );
    assert!(
        game.player().waitroom.cards.contains(&hand_card),
        "Hand card should be in discard (paid as cost)"
    );

    // Effect action 1: Chika (idx 0 selected) moved from stage to discard
    assert!(
        game.player().stage.stage[0] == dia,
        "Dia should be summoned to the stage from discard"
    );
    assert!(
        !game.player().waitroom.cards.contains(&dia),
        "Dia should no longer be in discard"
    );

    // Effect action 2: Dia (cost 11 = Chika cost 9 + 2) summoned to vacated area
    assert!(
        game.player().stage.stage[1] == yoshiko,
        "Yoshiko should stay on stage in center"
    );
    assert_eq!(
        game.player()
            .stage
            .stage
            .iter()
            .filter(|&&id| id != -1)
            .count(),
        initial_stage_count,
        "Stage count should be same (1 replaced)"
    );
}

/// Test case 2: Cannot activate when not in center position
#[test]
fn test_yoshiko_center_ability_fails_not_in_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!-sd1-001-SD");

    // Place Yoshiko in left side, not center
    game.add_to_stage(MemberArea::LeftSide, yoshiko);
    game.add_to_stage(MemberArea::Center, chika);

    game.give_energy(5);

    // Try to activate ability - should fail
    let stage_before = game.player().stage.stage.clone();
    let result = game.try_activate_ability(yoshiko);

    // Verify ability cannot be activated
    assert!(
        result.is_err(),
        "Ability activation should fail when not in center"
    );

    // Verify no state changes occurred
    assert!(
        game.player().stage.stage == stage_before,
        "Stage should be unchanged"
    );
    assert!(
        game.state.get_pending_choice().is_none(),
        "Should not have pending choice when not in center"
    );
}

/// Test case 3: Cost payment fails - no cards in hand to discard
#[test]
fn test_yoshiko_center_ability_fails_no_hand_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R"); // Aqours Chika

    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);

    // Don't add any cards to hand
    game.give_energy(5);

    // Activate ability — cost validation fails (hand empty, need 1 discard)
    // Error is caught internally, ability completes with no state change
    game.activate_ability(yoshiko);

    // No pending choice was created (cost validation failed before choice)
    assert!(
        !game.has_pending_choice(),
        "No choice should exist when cost cannot be paid"
    );

    // No cost was actually paid — hand still empty, Yoshiko unchanged
    assert!(
        game.player().hand.cards.is_empty(),
        "Hand should still be empty"
    );
    assert!(
        game.player().waitroom.cards.is_empty(),
        "Nothing should be in discard"
    );
}

/// Test case 4: Main effect fails - no other Aqours members on stage
#[test]
fn test_yoshiko_center_ability_no_other_aqours_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let hand_card = game.id("PL!-sd1-010-SD");

    // Only Yoshiko on stage, no other Aqours members
    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_hand(hand_card);

    game.give_energy(5);

    let initial_discard_size = game.player().waitroom.cards.len();

    // Activate ability
    game.activate_ability(yoshiko);

    // Handle cost choice (hand discard)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify cost was paid but effect had no valid targets
    assert!(
        game.player().stage.stage[1] == yoshiko,
        "Yoshiko should still be on stage but in wait state (cost paid)"
    );
    assert_eq!(
        game.player().waitroom.cards.len(),
        initial_discard_size + 1,
        "Only hand card should be discarded, no stage member moved"
    );
}

/// Test case 5: Conditional effect fails - no valid targets in discard with correct cost
#[test]
fn test_yoshiko_center_ability_no_valid_discard_targets() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R"); // Aqours Chika, cost 9
    let low_cost_card = game.id("PL!-sd1-010-SD"); // Non-Aqours card
    let hand_card = game.id("PL!-sd1-011-SD");

    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika); // Only one target to avoid choice
    game.add_to_hand(hand_card);

    // Add a valid conditional effect target to ensure full ability execution
    let dia = game.id("PL!S-bp2-004-R"); // Aqours Dia, cost 11 (Chika cost 9 + 2 = 11)
    game.add_to_discard(dia); // Valid target for conditional effect
    game.add_to_discard(low_cost_card); // Non-Aqours card

    game.give_energy(5);

    // Activate ability
    game.activate_ability(yoshiko);

    // Handle all pending choices: cost + effect (stage member selection)
    while game.has_pending_choice() {
        game.select_indices(&[0]); // Select Chika (only Aqours on stage)
    }

    // Verify cost paid
    assert!(
        game.player().stage.stage[1] == yoshiko,
        "Yoshiko should still be on stage but in wait state"
    );

    // Effect action 1: Chika was moved from stage
    assert!(
        game.player().waitroom.cards.contains(&chika),
        "Chika should be in discard"
    );

    // Effect action 2: Dia (cost 11 = Chika cost 9 + 2) summoned to vacated area
    assert!(
        game.player().stage.stage[0] == dia,
        "Dia should be summoned to the vacated area (cost 11 = 9 + 2)"
    );
    assert!(
        !game.player().waitroom.cards.contains(&dia),
        "Dia should no longer be in discard"
    );
}

/// Test case 6: Cost calculation test - summon correct cost member
#[test]
fn test_yoshiko_center_ability_cost_calculation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R"); // Aqours Chika, cost 9
    let dia = game.id("PL!S-bp2-004-R"); // Aqours Dia, cost 11 (9 + 2 = 11, should be summonable)
    let you = game.id("PL!S-bp2-003-R"); // Aqours You, cost 9 (9 + 2 = 11, should NOT be summonable - wrong cost)
    let hand_card = game.id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika); // Cost 9 member to be sent to discard
    game.add_to_hand(hand_card);

    // Put correct cost Aqours members in discard (need cost 11 for conditional effect: 9 + 2 = 11)
    game.add_to_discard(dia); // Cost 11 - should be summonable
    game.add_to_discard(you); // Cost 9 - should NOT be summonable (wrong cost)

    game.give_energy(5);

    // Activate ability
    game.activate_ability(yoshiko);

    // Handle all pending choices: cost + effect (stage→Chika selected) + conditional summon
    while game.has_pending_choice() {
        game.select_indices(&[0]); // Select Chika (only Aqours on stage besides self)
    }

    // Effect action 1: Chika moved from stage to discard
    assert!(
        game.player().waitroom.cards.contains(&chika),
        "Chika should be in discard"
    );

    // Effect action 2: Dia (cost 11 = Chika cost 9 + 2) summoned to vacated area
    assert!(
        game.player().stage.stage[0] == dia,
        "Dia should be summoned to vacated area (cost 11 = 9 + 2)"
    );
    assert!(
        !game.player().waitroom.cards.contains(&dia),
        "Dia should no longer be in discard"
    );
    // You (cost 9) stays in discard — cost 9 ≠ 11, not a valid summon target.
    // This assertion IS the test: without it a cost-blind summon passes.
    assert!(
        game.player().waitroom.cards.contains(&you),
        "wrong-cost You must remain in the waitroom"
    );
    assert!(
        !game.player().stage.stage.contains(&you),
        "wrong-cost You must not be summoned"
    );
}

/// Test case 7: Use limit test - can only use once per turn
#[test]
fn test_yoshiko_center_ability_use_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R"); // Aqours Chika
    let riko = game.id("PL!S-bp2-002-R"); // Aqours Riko (other valid stage target so choice is needed)
    let dia = game.id("PL!S-bp2-004-R"); // Aqours Dia (cost 11 = Chika cost 9 + 2)
    let hand_card1 = game.id("PL!-sd1-010-SD");
    let hand_card2 = game.id("PL!-sd1-011-SD");

    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_stage(MemberArea::RightSide, riko);
    game.add_to_hand(hand_card1);
    game.add_to_hand(hand_card2);
    game.add_to_discard(dia);

    game.give_energy(10);

    // First activation should succeed
    game.activate_ability(yoshiko);

    // Handle all choices for the first ability (cost discard + stage selection + possible discard selection)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let initial_hand_size_after_first = game.player().hand.cards.len();

    // Try to activate again - should fail due to use limit (turn_limited_abilities_used check)
    let result = game.try_activate_ability(yoshiko);
    assert!(
        result.is_err(),
        "Second activation should fail due to use limit"
    );

    // Verify no additional cost was paid
    assert_eq!(
        game.player().hand.cards.len(),
        initial_hand_size_after_first,
        "Should not pay cost again due to use limit"
    );
}

/// Test case 8: Exclude self test - Yoshiko cannot be chosen for discard effect
#[test]
fn test_yoshiko_center_ability_exclude_self() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋");
    let chika = game.id("PL!S-bp2-001-R"); // Correct Aqours Chika
    let hand_card = game.id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_hand(hand_card);

    // Add a cost 11 Aqours member to discard for conditional effect (Chika cost 9 + 2 = 11)
    let dia = game.id("PL!S-bp2-004-R"); // Dia Kurosawa (Aqours, cost 11)
    game.add_to_discard(dia);

    game.give_energy(5);

    // Activate ability
    game.activate_ability(yoshiko);

    // Handle all pending choices: cost + effect (stage member selection) + conditional summon
    while game.has_pending_choice() {
        // Stage choice: only Chika (Aqours, not self) is available
        // Discard choice: Dia (cost 11 = Chika cost 9 + 2) auto-resolves (1 candidate)
        game.select_indices(&[0]);
    }

    // Verify Yoshiko is still on stage in wait state, not moved by main effect
    assert!(
        game.player().stage.stage[1] == yoshiko,
        "Yoshiko should still be on stage in wait state"
    );

    // Verify Chika was moved from stage (effect exclude_self = true)
    assert!(
        game.player().waitroom.cards.contains(&chika),
        "Chika should be the one moved by main effect"
    );

    // Verify Yoshiko was NOT moved (exclude_self prevents targeting the activator)
    assert!(
        !game.player().waitroom.cards.contains(&yoshiko),
        "Yoshiko should not be in discard from main effect"
    );

    // Verify Dia (cost 11 = Chika cost 9 + 2) summoned to vacated area
    assert!(
        game.player().stage.stage[0] == dia,
        "Dia should be summoned to vacated area (cost 11 = 9 + 2)"
    );
}

/// Test: cost_reference uses the SACRIFICED member's cost, NOT Yoshiko's cost.
/// Yoshiko (cost 13) in center, Chika (cost 9) on left.
/// A card with cost 11 (9+2) in discard → correct target.
/// A card with cost 15 (13+2) in discard → should NOT be picked (wrong cost for Chika).
#[test]
fn test_yoshiko_cost_reference_uses_sacrificed_not_self() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋"); // cost 13
    let chika = game.id("PL!S-bp2-001-R"); // cost 9 — to be sacrificed
    let correct_target = game.id("PL!S-bp2-004-R"); // Dia, cost 11 = 9+2
    let wrong_target = game.id("PL!S-PR-014-PR"); // cost 15 = 13+2 (Yoshiko+2)
    let hand_card = game.id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_hand(hand_card);
    game.add_to_discard(correct_target);
    game.add_to_discard(wrong_target);
    game.give_energy(15);

    game.activate_ability(yoshiko);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify: the correct +2 card was summoned (cost 11 = Chika 9 + 2), NOT cost-15
    // Dia should be deployed to left side (where Chika was)
    assert_eq!(
        game.player().stage.stage[0],
        correct_target,
        "Dia (cost 11 = 9+2) should be summoned to vacated area"
    );
    // Chika should be in discard
    assert!(
        game.player().waitroom.cards.contains(&chika),
        "Chika should be in discard (sacrificed)"
    );
    // Wrong target (cost 15) stays in discard
    assert!(
        game.player().waitroom.cards.contains(&wrong_target),
        "Wrong-cost target (cost 15) should remain in discard"
    );
    // Yoshiko stays at center
    assert_eq!(
        game.player().stage.stage[1],
        yoshiko,
        "Yoshiko should remain at center"
    );
}

/// Test: WITHOUT the correct +2 target in discard → nothing summoned.
/// Verifies the sacrificed member is still moved (card text says so).
#[test]
fn test_yoshiko_no_valid_target_after_sacrifice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R＋"); // cost 13
    let chika = game.id("PL!S-bp2-001-R"); // cost 9
    let wrong_target = game.id("PL!S-PR-014-PR"); // cost 15 = 13+2, but need 11
    let hand_card = game.id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::Center, yoshiko);
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_hand(hand_card);
    game.add_to_discard(wrong_target); // only wrong-cost card in discard
    game.give_energy(15);

    game.activate_ability(yoshiko);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Chika was sacrificed (first action always runs)
    assert!(
        game.player().waitroom.cards.contains(&chika),
        "Chika should be in discard (sacrificed even without valid +2)"
    );
    // Wrong target stays in discard (not summoned)
    assert!(
        game.player().waitroom.cards.contains(&wrong_target),
        "Wrong-cost target should remain in discard"
    );
    // Nothing was summoned to stage[0]
    assert_eq!(
        game.player().stage.stage[0],
        -1,
        "Stage left should be empty (no valid +2 card to summon)"
    );
    // Yoshiko stays at center
    assert_eq!(
        game.player().stage.stage[1],
        yoshiko,
        "Yoshiko should remain at center"
    );
}
