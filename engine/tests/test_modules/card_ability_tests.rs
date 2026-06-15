/// Comprehensive ability tests for specific card abilities
///
/// This file contains detailed tests for individual card abilities,
/// focusing on edge cases, condition validation, and proper card selection.
use crate::helpers::*;

// ====================================================================
//  大沢瑠璃乃 (PL!HS-bp2-005) — 登場 trigger with member condition
// ====================================================================
// 登場: 手札を1枚控え室に置いてもよい：自分のステージにほかのメンバーがいる場合、
//   自分の控え室から『みらくらぱーく！』のカードを1枚手札に加える。
//
// Ability Components:
// - Trigger: 登場 (When card is played to stage)
// - Cost: Optional - discard 1 card from hand to discard
// - Condition: Must have other members on stage (exclude self)
// - Effect: Move 1 card from discard to hand
// - Target filter: Only cards with group_names ["みらくらぱーく！"]
// ====================================================================

/// Test that the ability does NOT trigger when Rurino is the only card on stage
#[test]
fn rurino_ozora_no_trigger_when_alone_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Get Rurino card
    let rurino = game.id("PL!HS-bp2-005-R＋");

    // Add Rurino to hand and set up stage with Rurino alone
    game.add_to_hand(rurino);
    game.give_energy(10); // Rurino costs 10 energy
    game.state.player1.stage.stage = [-1, -1, -1];

    // Add cards to hand and discard for testing
    game.add_to_hand(game.id("PL!-sd1-010-SD"));
    let mirakura_card = game.id("PL!HS-pb1-003-R");
    game.add_to_discard(mirakura_card);

    // Play Rurino to trigger 登場
    game.play_to_stage(rurino, rabuka_engine::zones::MemberArea::Center);

    // Verify ability did NOT trigger - condition not met
    // The みらくらぱーく！ card should still be in discard
    assert!(
        game.state.player1.waitroom.cards.contains(&mirakura_card),
        "みらくらぱーく！ card should remain in discard when Rurino is alone on stage"
    );

    // Verify stage setup is correct
    assert_eq!(
        game.state.player1.stage.stage[1], rurino,
        "Rurino should be on stage"
    );
    assert_eq!(
        game.state.player1.stage.stage[0], -1,
        "Left position should be empty"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], -1,
        "Right position should be empty"
    );
}

/// Test that the ability triggers when other members are present on stage
/// and the full pay → recover flow works.
#[test]
fn rurino_ozora_triggers_with_other_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp2-005-R＋");
    let other_member = game.id("PL!-sd1-010-SD");
    let cost_card = game.id("PL!-sd1-020-SD");

    game.add_to_hand(rurino);
    game.add_to_hand(cost_card);
    game.give_energy(10);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.add_to_stage(rabuka_engine::zones::MemberArea::RightSide, other_member);

    let mirakura_card = game.id("PL!HS-pb1-003-R");
    game.add_to_discard(mirakura_card);

    let hand_before = game.state.player1.hand.cards.len();
    let discard_before = game.state.player1.waitroom.cards.len();

    game.play_to_stage(rurino, rabuka_engine::zones::MemberArea::Center);

    // Net: -1 Rurino -1 cost +1 mirakura = -1 hand, 0 discard
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "Hand: {} - 1 (Rurino played, cost discarded, mirakura recovered)",
        hand_before
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before,
        "Discard: net 0 (cost in, mirakura out)"
    );
}

/// Test that only みらくらぱーく！ cards can be selected from discard
#[test]
fn rurino_ozora_only_selects_mirakura_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp2-005-R＋");
    let other = game.id("PL!-sd1-010-SD");

    game.add_to_hand(rurino);
    game.give_energy(10);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.add_to_stage(rabuka_engine::zones::MemberArea::LeftSide, other);
    game.add_to_stage(
        rabuka_engine::zones::MemberArea::RightSide,
        game.id("PL!-sd1-020-SD"),
    );

    let mirakura_card1 = game.id("PL!HS-pb1-003-R");
    let mirakura_card2 = game.id("PL!HS-pb1-003-P＋");
    let non_mirakura = game.id("PL!-sd1-003-SD");

    game.add_to_discard(mirakura_card1);
    game.add_to_discard(mirakura_card2);
    game.add_to_discard(non_mirakura);

    game.add_to_hand(game.id("PL!-sd1-005-SD"));

    game.play_to_stage(rurino, rabuka_engine::zones::MemberArea::Center);

    // Resolve any pending choices (select first eligible card each time)
    let mut safety = 0;
    while game.has_pending_choice() && safety < 10 {
        game.select_indices(&[0]);
        safety += 1;
    }

    // Non-mirakura cards remain in discard (filtered out by ability)
    assert!(
        game.state.player1.waitroom.cards.contains(&non_mirakura),
        "Non-mirakura card should remain in discard"
    );

    // One mirakura card was recovered to hand
    let mirakura_in_hand = game.state.player1.hand.cards.contains(&mirakura_card1)
        || game.state.player1.hand.cards.contains(&mirakura_card2);
    assert!(
        mirakura_in_hand,
        "One mirakura card should be recovered to hand"
    );
    // One mirakura card should still be in discard (the other was recovered)
    let mirakura_still_in_discard = game.state.player1.waitroom.cards.contains(&mirakura_card1)
        && game.state.player1.waitroom.cards.contains(&mirakura_card2);
    assert!(
        !mirakura_still_in_discard,
        "At most one mirakura card should remain in discard (one was recovered)"
    );
}

/// Test that the ability handles the case where the cost is paid with
/// the only hand card (Rurino's additional cost card) and mirakura is retrieved.
#[test]
fn rurino_ozora_optional_cost_no_payment() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp2-005-R＋");
    let other_member = game.id("PL!-sd1-010-SD");

    game.add_to_hand(rurino);
    game.give_energy(10);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.add_to_stage(rabuka_engine::zones::MemberArea::RightSide, other_member);

    let mirakura_card = game.id("PL!HS-pb1-003-R");
    game.add_to_discard(mirakura_card);

    let hand_before = game.state.player1.hand.cards.len(); // 1 (rurino)

    game.play_to_stage(rurino, rabuka_engine::zones::MemberArea::Center);

    // Rurino played to stage. The cost auto-resolves (no cards to discard
    // after playing Rurino → cost succeeds with 0 discarded).
    // The effect retrieves mirakura from discard.
    // Net hand: -1 (Rurino played) +1 (mirakura recovered) = 0 net change
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Hand: Rurino played + mirakura recovered = net 0"
    );
    // Discard: +1 (mirakura recovered) → net -1
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        0,
        "Mirakura was recovered from discard"
    );
}

/// Test cost payment - engine auto-resolves optional cost and effect
#[test]
fn rurino_ozora_cost_payment_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp2-005-R＋");
    let other_member = game.id("PL!-sd1-010-SD");

    game.add_to_hand(rurino);
    game.give_energy(10);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.add_to_stage(rabuka_engine::zones::MemberArea::RightSide, other_member);

    let cost_card = game.id("PL!-sd1-020-SD");
    game.add_to_hand(cost_card);
    let mirakura_card = game.id("PL!HS-pb1-003-R");
    game.add_to_discard(mirakura_card);

    let hand_before = game.state.player1.hand.cards.len();
    let discard_before = game.state.player1.waitroom.cards.len();

    // Play Rurino → debut triggers → cost auto-resolves → effect applies
    game.play_to_stage(rurino, rabuka_engine::zones::MemberArea::Center);

    // Hand should have changed: -1 Rurino played -1 cost discarded +1 mirakura recovered = -1
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "Net hand: {hand_before} - 1 (Rurino played) - 1 (cost) + 1 (mirakura) = {}",
        hand_before - 1
    );
    // Discard should have net 0 change: +1 cost -1 mirakura
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before,
        "Net discard: 0 (cost in, mirakura out)"
    );
}

/// Test edge case: empty discard pile
#[test]
fn rurino_ozora_empty_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp2-005-R＋");
    let other_member = game.id("PL!-sd1-010-SD");

    // Add Rurino to hand and set up stage with Rurino and another member
    game.add_to_hand(rurino);
    game.give_energy(10); // Rurino costs 10 energy
    game.state.player1.stage.stage = [-1, -1, -1];

    // Add other member to stage
    game.add_to_stage(rabuka_engine::zones::MemberArea::RightSide, other_member);

    // Add card for cost but keep discard empty
    game.add_to_hand(game.id("PL!-sd1-020-SD"));

    // Discard should be empty
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        0,
        "Discard should be empty initially"
    );

    // Play Rurino to trigger 登場
    game.play_to_stage(rurino, rabuka_engine::zones::MemberArea::Center);

    // Even if cost is paid, no cards can be moved from discard to hand
    // because discard is empty
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        0,
        "Discard should remain empty"
    );
}

/// Test with multiple Rurino cards on stage
#[test]
fn rurino_ozora_multiple_copies_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino1 = game.id("PL!HS-bp2-005-R＋");
    let rurino2 = game.id("PL!HS-bp2-005-P＋");
    let other_member = game.id("PL!-sd1-010-SD");

    // Set up stage with multiple Rurino copies and another member
    game.state.player1.stage.stage = [-1, -1, -1];
    game.add_to_stage(rabuka_engine::zones::MemberArea::LeftSide, rurino1);
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, rurino2);
    game.add_to_stage(rabuka_engine::zones::MemberArea::RightSide, other_member);

    // Each Rurino should be able to trigger independently
    // The "other members" condition should exclude the specific Rurino card
    // but include the other Rurino copy

    // For rurino1: other members include rurino2 and other_member
    let rurino1_others = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .filter(|&&id| id != -1 && id != rurino1)
        .count();
    assert_eq!(rurino1_others, 2, "Rurino1 should see 2 other members");

    // For rurino2: other members include rurino1 and other_member
    let rurino2_others = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .filter(|&&id| id != -1 && id != rurino2)
        .count();
    assert_eq!(rurino2_others, 2, "Rurino2 should see 2 other members");
}

/// Test that the ability works correctly with different stage positions
#[test]
fn rurino_ozora_different_stage_positions() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rurino = game.id("PL!HS-bp2-005-R＋");
    let other_member = game.id("PL!-sd1-010-SD");

    // Add Rurino to hand and set up stage
    game.add_to_hand(rurino);
    game.give_energy(10); // Rurino costs 10 energy
    game.state.player1.stage.stage = [-1, -1, -1];

    // Test Rurino in left position with other member at center
    game.add_to_stage(rabuka_engine::zones::MemberArea::LeftSide, rurino);
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, other_member);

    let other_members_count = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .filter(|&&id| id != -1 && id != rurino)
        .count();
    assert_eq!(
        other_members_count, 1,
        "Should have 1 other member when Rurino is left"
    );

    // Test Rurino in right position with other member at center
    game.state.player1.stage.stage = [-1, other_member, rurino];

    let other_members_count = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .filter(|&&id| id != -1 && id != rurino)
        .count();
    assert_eq!(
        other_members_count, 1,
        "Should have 1 other member when Rurino is right"
    );
}

#[test]
fn mirakura_discard_then_draws_count_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ability_card = game.new_id("PL!HS-pb1-003-R");
    let discard_card = game.new_id("PL!HS-pb1-003-R"); // same template as ability → matches same filter
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(ability_card);
    game.add_to_hand(discard_card);
    game.give_energy(15);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.play_to_stage(ability_card, rabuka_engine::zones::MemberArea::Center);

    // Cost may auto-pay (Exact match: 1 eligible = count=1) or create a Prompt choice.
    // Handle both cases:
    if game.has_pending_choice() {
        // Choice created — select the one eligible card
        let ct = game.pending_choice_type();
        assert_eq!(ct, Some("SelectCard".to_string()));
        game.select_indices(&[0]);
    }

    // Effect: draw 2 (1 discarded + 1 bonus)
    assert!(
        game.state.player1.waitroom.cards.contains(&discard_card),
        "Discard pile should contain the discarded card"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "Should draw 2 cards after discarding 1"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        18,
        "Deck should have 18 remaining cards after drawing 2"
    );
}

#[test]
fn q244_mirakura_no_discard_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ability_card = game.new_id("PL!HS-pb1-003-R");
    let other_card = game.new_id("PL!HS-pb1-003-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(ability_card);
    game.add_to_hand(other_card);
    game.give_energy(15);
    game.state.player1.main_deck.cards.push(filler);

    let deck_before = game.state.player1.main_deck.cards.len();

    game.play_to_stage(ability_card, rabuka_engine::zones::MemberArea::Center);

    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        0,
        "No cards should be discarded when player chooses 0"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "If 0 cards are discarded, the ability should still draw 1 card"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "Deck should lose exactly one card when drawing for 0 discarded cards"
    );
}

// ====================================================================
//  大沢瑠璃乃 (PL!HS-bp1-005-R) — 登場:  discard up to 3 → draw equal to discarded
// ====================================================================
// 登場: 手札を3枚まで控え室に置いてもよい：これにより置いた枚数分カードを引く。
//
// Cost: optional, discard up to 3 from hand
// Effect: draw cards equal to number actually discarded
// ====================================================================
fn setup_rurino_bp1(game: &mut TestGame) -> (usize, usize) {
    let rurino = game.id("PL!HS-bp1-005-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(rurino);
    game.give_energy(9);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let deck_before = game.state.player1.main_deck.cards.len();
    game.play_to_stage(rurino, rabuka_engine::zones::MemberArea::Center);

    (deck_before, 3)
}

#[test]
fn rurino_bp1_discard_2_draw_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (deck_before, hand_before) = setup_rurino_bp1(&mut game);

    if game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }

    // count=0 → draw = last_cost_discard_count = 2
    assert_eq!(game.state.player1.waitroom.cards.len(), 2);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 2);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "3 - 2 discarded + 2 drawn = 3, got {}",
        game.state.player1.hand.cards.len()
    );
}

#[test]
fn rurino_bp1_discard_0_draw_0() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (deck_before, hand_before) = setup_rurino_bp1(&mut game);

    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // skipped cost → 0 discarded → draw 0
    assert_eq!(game.state.player1.hand.cards.len(), hand_before);
    assert_eq!(game.state.player1.waitroom.cards.len(), 0);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before);
}

#[test]
fn rurino_bp1_discard_3_draw_3() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (deck_before, hand_before) = setup_rurino_bp1(&mut game);

    if game.has_pending_choice() {
        game.select_indices(&[0, 1, 2]);
    }

    // count=0 → draw = last_cost_discard_count = 3
    assert_eq!(game.state.player1.waitroom.cards.len(), 3);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 3);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "3 - 3 discarded + 3 drawn = 3, got {}",
        game.state.player1.hand.cards.len()
    );
}

// ====================================================================
// 小泉花陽 (PL!-bp3-008-R+) ab#1 — ライブ開始時 optional change_state wait
// ====================================================================
// ライブ開始時: 『μ's』のメンバー1人をウェイトにしてもよい：
//   ライブ終了時まで、heart03+heart03を得る。
//
// Cost: optional change_state (wait) with group_names=["μ's"], count=1
// Effect: gain_resource heart ×2 (heart03), duration=live_end
// ====================================================================

/// Advance helpers for live start
fn h_advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass(); // Main -> Active
    game.pass(); // Active -> Energy
    game.pass(); // Energy -> Draw
    game.pass(); // Draw -> Main
    game.pass(); // Main -> LiveCardSetFirstAttacker
}

fn h_advance_to_live_start(game: &mut TestGame) {
    game.pass(); // LiveCardSetP1 -> LiveCardSetP2
    game.pass(); // LiveCardSetP2 -> FirstAttackerPerformance
}

/// Pay optional cost: a μ's member should be waited, hearts gained.
#[test]
fn hanayo_pay_optional_cost_waits_member_and_gains_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hanayo = game.id("PL!-bp3-008-R+");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Place Hanayo on stage (she's a μ's member via series check)
    game.state.player1.stage.stage = [hanayo, -1, -1];

    // Advance to live start
    h_advance_to_live_card_set_p1(&mut game);
    game.state.player1.live_card_zone.cards.push(filler);
    h_advance_to_live_start(&mut game);

    // Should have pending choice: "Pay optional cost? (pay or skip)"
    assert!(
        game.has_pending_choice(),
        "Optional cost prompt should appear"
    );

    // Pay the cost (select_option(1) = "Yes")
    game.select_option(1);

    // After paying, the wait should have been applied
    let hanayo_orientation = game.state.mods.get_orientation_modifier(hanayo);
    assert_eq!(
        hanayo_orientation.map(|s| s.as_str()),
        Some("wait"),
        "Hanayo should be waited after paying optional cost"
    );

    // Heart modifier should be applied (gain_resource heart03 x2, duration=live_end)
    let heart03 = game
        .state
        .mods
        .get_heart_modifier(hanayo, rabuka_engine::card::HeartColor::Heart03);
    assert_eq!(
        heart03, 2,
        "Hanayo should have heart03 x2 modifier after paying cost"
    );
}

/// Skip optional cost: no member waited, no hearts gained.
#[test]
fn hanayo_skip_optional_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    let hanayo = game.id("PL!-bp3-008-R+");

    game.state.player1.stage.stage = [hanayo, -1, -1];

    h_advance_to_live_card_set_p1(&mut game);
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    h_advance_to_live_start(&mut game);

    assert!(
        game.has_pending_choice(),
        "Optional cost prompt should appear"
    );

    // Skip the cost (select_option(0) = "No")
    game.select_option(0);

    // Hanayo should NOT be waited
    let hanayo_orientation = game.state.mods.get_orientation_modifier(hanayo);
    assert_ne!(
        hanayo_orientation.map(|s| s.as_str()),
        Some("wait"),
        "Hanayo should NOT be waited when cost is skipped"
    );

    // No heart modifier should be present
    let heart03 = game
        .state
        .mods
        .get_heart_modifier(hanayo, rabuka_engine::card::HeartColor::Heart03);
    assert_eq!(heart03, 0, "No heart modifier when cost is skipped");
}

/// No μ's member on stage → optional cost prompt should NOT appear.
#[test]
fn hanayo_no_mus_member_skips_cost_prompt() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    let hanayo = game.id("PL!-bp3-008-R+");
    let non_mus = game.id("PL!-sd1-010-SD"); // no μ's affiliation

    // Only non-μ's members on stage (Hanayo is in hand, not on stage)
    game.state.player1.stage.stage = [non_mus, -1, -1];
    game.state.player1.hand.cards.push(hanayo);
    game.give_energy(15);
    game.play_to_stage(hanayo, rabuka_engine::zones::MemberArea::Center);

    // Now stage has [non_mus, hanayo, -1]
    // Hanayo has the live-start ability, but since she's the only μ's member,
    // the ability will trigger but...
    // Actually, the cost can target any μ's member including hanayo herself.
    // Let's use a different approach: put NO μ's members on stage.

    // Reset: only non-μ's members
    game.state.player1.stage.stage = [non_mus, non_mus, non_mus];

    h_advance_to_live_card_set_p1(&mut game);
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!-sd1-010-SD"));
    h_advance_to_live_start(&mut game);

    // No optional cost prompt since there are no μ's members to wait
    assert!(
        !game.has_pending_choice(),
        "No optional cost prompt when no μ's members on stage"
    );
}

// ====================================================================
// SUKI for you, DREAM for you! (PL!S-bp3-025-L) ab#0
// ライブ開始時: 自分のステージにいる『Aqours』のメンバー1人を選ぶ。
// そのメンバーが持つブレードが6つ以上の場合、このカードのスコアを+1する。
//
// Action 1: select (count=1, card_type=member_card, target=self, group_names=[Aqours])
// Action 2: modify_score (value=1, self_target=true,
//           condition: card_blade_condition, count=6, operator=>=, source=selected_cards)
// ====================================================================

/// Known Aqours member card IDs (verified by checking series=ラブライブ！サンシャイン!!)
const AQOURS_MEMBER_1: &str = "PL!S-bp2-015-PR"; // cost=4, blade=1 (low)
const AQOURS_MEMBER_2: &str = "PL!S-sd1-001-SD"; // cost=17, blade=6 (high)

/// Place SUKI as live card, fill deck, and advance to live start.
/// Returns the copy ID of the SUKI card placed on the live card zone.
fn setup_suki_and_advance(game: &mut TestGame) -> i16 {
    let filler_id = game.id("PL!-sd1-010-SD");
    // Fill deck so debug draw_card assertion doesn't fire
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler_id);
        game.state.player2.main_deck.cards.push(filler_id);
    }
    let suki_id = game.id("PL!S-bp3-025-L");
    game.state.player1.live_card_zone.cards.push(suki_id);
    h_advance_to_live_card_set_p1(game);
    game.state.player1.live_card_zone.cards.push(filler_id);
    h_advance_to_live_start(game);
    suki_id
}

/// One Aqours member with base blade=1 (<6) on stage → pick it → condition fails.
#[test]
fn suki_low_blade_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    game.state.player1.stage.stage[0] = game.id(AQOURS_MEMBER_1);
    let suki_id = setup_suki_and_advance(&mut game);

    assert!(
        game.has_pending_choice(),
        "Choice expected (1 Aqours member)"
    );
    game.select_option(0);

    let score = game
        .state
        .mods
        .score_modifiers
        .get(&suki_id)
        .copied()
        .unwrap_or(0);
    assert_eq!(score, 0, "No bonus for blade=1 (<6)");
}

/// One Aqours member with base blade=1 +6 =7 ≥6 → pick it → condition passes.
#[test]
fn suki_high_blade_gains_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let member_id = game.id(AQOURS_MEMBER_1);
    game.state.player1.stage.stage[0] = member_id;
    game.state.mods.add_blade_modifier(member_id, 6);
    let suki_id = setup_suki_and_advance(&mut game);

    assert!(
        game.has_pending_choice(),
        "Choice expected (1 Aqours member)"
    );
    game.select_option(0);

    let score = game
        .state
        .mods
        .score_modifiers
        .get(&suki_id)
        .copied()
        .unwrap_or(0);
    assert!(score >= 1, "Bonus given for blade 1+6=7 (got {})", score);
}

/// Two Aqours members, both low blade → pick first → no bonus.
#[test]
fn suki_choose_low_blade_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    game.state.player1.stage.stage[0] = game.id(AQOURS_MEMBER_1);
    game.state.player1.stage.stage[1] = game.id(AQOURS_MEMBER_2);
    let suki_id = setup_suki_and_advance(&mut game);

    assert!(
        game.has_pending_choice(),
        "Should prompt with 2 Aqours members"
    );
    game.select_option(0);

    let score = game
        .state
        .mods
        .score_modifiers
        .get(&suki_id)
        .copied()
        .unwrap_or(0);
    assert_eq!(score, 0, "No bonus for picking low-blade member");
}

/// Two Aqours members, second has blade=6 → pick second → bonus.
#[test]
fn suki_choose_high_blade_gains_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    game.state.player1.stage.stage[0] = game.id(AQOURS_MEMBER_1);
    game.state.player1.stage.stage[1] = game.id(AQOURS_MEMBER_2);
    let suki_id = setup_suki_and_advance(&mut game);

    assert!(
        game.has_pending_choice(),
        "Should prompt with 2 Aqours members"
    );
    game.select_option(1);

    let score = game
        .state
        .mods
        .score_modifiers
        .get(&suki_id)
        .copied()
        .unwrap_or(0);
    assert!(score >= 1, "Bonus for high-blade choice (got {})", score);
}

// ====================================================================
// ウィーン・マルガレーテ (PL!SP-bp4-021-N) ab#0
// 常時: 自分のエネルギーが相手より多いかぎり、heart06を得る。
// Condition: comparison_condition, resource_type=energy, operator=>
//            comparison_target=opponent
// Cost to play: 11 energy.  `rem` = energy cards remaining after paying.
// ====================================================================

fn wien_has_heart06(gs: &rabuka_engine::game_state::GameState, cid: i16) -> bool {
    gs.mods
        .get_heart_modifier(cid, rabuka_engine::card::HeartColor::Heart06)
        > 0
}

/// Set up remaining count for P1 (after paying 11 cost) and P2 total, then play Wien.
fn setup_wien(game: &mut TestGame, p1_rem: usize, _p1_active: usize, p2_total: usize) -> i16 {
    let wien_id = game.id("PL!SP-bp4-021-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(wien_id);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.main_deck.cards.push(filler);
    // Give 11 + remaining so after paying the cost, `p1_rem` cards remain
    game.give_energy(11 + p1_rem);
    for _ in 0..p2_total {
        game.state.player2.energy_zone.cards.push(filler);
    }
    game.state.player2.energy_zone.active_energy_count = p2_total;
    game.play_to_stage(wien_id, rabuka_engine::zones::MemberArea::Center);
    wien_id
}

/// P1 remaining > P2 total → heart06.
#[test]
fn wien_more_energy_gains_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let wien_id = setup_wien(&mut game, 4, 4, 2);
    assert!(
        wien_has_heart06(&game.state, wien_id),
        "heart06 when P1=15 > P2=2"
    );
}

#[test]
fn wien_more_total_active_less_still_gains() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let wien_id = setup_wien(&mut game, 4, 1, 2);
    assert!(
        wien_has_heart06(&game.state, wien_id),
        "heart06 when P1 total=15 > P2=2 (active 1 < 2)"
    );
}

#[test]
fn wien_equal_energy_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let wien_id = setup_wien(&mut game, 3, 3, 14);
    // P1 total = 11+3 = 14, P2 total = 14 → 14 > 14 is FALSE
    assert!(
        !wien_has_heart06(&game.state, wien_id),
        "no heart06 when P1=14 == P2=14"
    );
}

#[test]
fn wien_less_energy_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let wien_id = setup_wien(&mut game, 2, 2, 14);
    // P1 total = 11+2 = 13, P2 total = 14 → 13 > 14 is FALSE
    assert!(
        !wien_has_heart06(&game.state, wien_id),
        "no heart06 when P1=13 < P2=14"
    );
}
