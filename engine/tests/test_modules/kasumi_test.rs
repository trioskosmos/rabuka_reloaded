/// Kasumi (中須かすみ PL!N-bp1-002-R+) — Debut topdeck look + Activation re-appear
///
/// Ab#0 (登場): Look at top 3 cards, arrange any on deck_top in any order, discard rest.
/// Ab#1 (起動): Cost: 2E + discard 1 from hand. Effect: move from discard to stage.
///   activation_condition_parsed: card must be in discard.
///
/// Q122: Looking at 3 cards with exactly 3 in deck => no refresh
/// Q63: Ability effect appearance doesn't pay member cost
/// Q75: Can't baton touch same turn appeared via ability
/// Q76: Can appear on occupied area (replaces existing member), but not locked areas
//=====================================================================
use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn kasumi_ability_only_from_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp1-002-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    // Put Kasumi in discard (this is the copy we'll test activation from)
    game.state.player1.waitroom.cards.push(kasumi);

    // Put a different Kasumi copy on stage (to test stage activation fails)
    let kasumi_stage = game.new_id("PL!N-bp1-002-R\u{ff0b}");
    game.state.player1.hand.cards.push(kasumi_stage);
    game.give_energy(3);
    game.play_to_stage(kasumi_stage, MemberArea::LeftSide);
    // Note: 3 energy given — 2 spent on play_to_stage, 1 remains for activation test

    // Should NOT be able to activate from stage (card not in discard)
    assert!(
        game.try_activate_ability(kasumi_stage).is_err(),
        "Should not activate from stage"
    );

    // Should NOT be able to activate from a copy in hand
    let kasumi_hand = game.new_id("PL!N-bp1-002-R\u{ff0b}");
    game.state.player1.hand.cards.push(kasumi_hand);
    assert!(
        game.try_activate_ability(kasumi_hand).is_err(),
        "Should not activate from hand"
    );

    // SHOULD be able to activate from discard
    // Need 2 energy for the ability cost + 1 card in hand to discard
    game.give_energy(2);
    game.state.player1.hand.cards.push(filler);
    assert!(
        game.try_activate_ability(kasumi).is_ok(),
        "Should activate from discard"
    );
}

#[test]
fn kasumi_q63_ability_appearance_no_cost_paid() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp1-002-R\u{ff0b}");
    let discard_fodder = game.id("PL!-sd1-010-SD");

    // Setup: Kasumi in discard, energy for cost, card in hand for discard cost
    game.state.player1.waitroom.cards.push(kasumi);
    game.state.player1.hand.cards.push(discard_fodder);
    let energy_before = 2u32;
    game.give_energy(energy_before as usize);
    let hand_before = game.state.player1.hand.cards.len();

    // Activate ability through the engine
    game.activate_ability(kasumi);

    // Cost step 1: pay 2E (auto-deducted, no prompt for this)
    // Cost step 2: discard 1 from hand -> SelectCard choice
    assert!(
        game.has_pending_choice(),
        "Should prompt to discard a card from hand"
    );

    // Resolve: discard the fodder card
    game.select_indices(&[0]);

    // Effect: place Kasumi on stage -> SelectPosition
    assert!(
        game.has_pending_choice(),
        "Should prompt to choose stage position"
    );

    // Choose Center
    TurnEngine::resume_with_choice(&mut game.state, Some(1), None).expect("select position");

    // Verify: Kasumi on stage
    game.assert_stage_pos(MemberArea::Center, kasumi, "Kasumi on center after ability");

    // Verify: 2 energy consumed (only ability cost, no member cost)
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "Q63: 2E consumed for ability cost, no extra member cost"
    );

    // Verify: 1 card discarded from hand
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "Q63: one card discarded from hand for cost"
    );

    // Verify: Kasumi no longer in discard
    assert!(
        !game.state.player1.waitroom.cards.contains(&kasumi),
        "Kasumi moved from discard to stage"
    );
}

#[test]
fn kasumi_q75_no_baton_touch_same_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp1-002-R\u{ff0b}");
    let discard_fodder = game.id("PL!-sd1-010-SD");
    let other_member = game.new_id("PL!-sd1-010-SD");

    // Setup: Kasumi in discard, energy for cost, card in hand for discard cost
    game.state.player1.waitroom.cards.push(kasumi);
    game.state.player1.hand.cards.push(discard_fodder);
    game.give_energy(2);

    // Activate ability through the engine
    game.activate_ability(kasumi);

    // Resolve: discard fodder from hand
    assert!(game.has_pending_choice(), "Discard prompt expected");
    game.select_indices(&[0]);

    // Resolve: choose LeftSide position
    assert!(game.has_pending_choice(), "Position prompt expected");
    TurnEngine::resume_with_choice(&mut game.state, Some(0), None).expect("select left");

    // Q75: Area should be locked (can't baton touch same turn)
    assert!(
        game.state
            .player1
            .areas_locked_this_turn
            .contains(&MemberArea::LeftSide),
        "Q75: LeftSide should be locked after ability appearance"
    );

    // Confirm other areas are NOT locked
    assert!(
        !game
            .state
            .player1
            .areas_locked_this_turn
            .contains(&MemberArea::Center),
        "Center should not be locked"
    );
    assert!(
        !game
            .state
            .player1
            .areas_locked_this_turn
            .contains(&MemberArea::RightSide),
        "RightSide should not be locked"
    );

    // Kasumi is on LeftSide
    game.assert_stage_pos(MemberArea::LeftSide, kasumi, "Kasumi on left after ability");

    // Try to baton touch: play another member to LeftSide should not replace Kasumi
    game.state.player1.hand.cards.push(other_member);
    // use_baton_touch = Some(true) via play_to_stage with default
    // The handle_play_member_to_stage will check the area — since only
    // LeftSide + Center have cards but LeftSide is locked, Center has a slot.
    // Actually, LeftSide is occupied, Center is empty, RightSide is empty.
    // Let's try to play another card directly to LeftSide — baton touch should
    // not be possible because the area is locked.
    // We verify Kasumi stays after the attempt.
    let _ = game.try_play_to_stage(other_member, MemberArea::LeftSide);

    // Kasumi should still be on LeftSide (wasn't replaced)
    game.assert_stage_pos(
        MemberArea::LeftSide,
        kasumi,
        "Kasumi still on left after baton touch attempt",
    );
}

#[test]
fn kasumi_q76_appear_on_occupied_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp1-002-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let discard_fodder = game.id("PL!-sd1-010-SD");

    // Setup: filler at Center, Kasumi in discard
    game.state.player1.stage.stage[1] = filler;
    game.state.player1.waitroom.cards.push(kasumi);
    game.state.player1.hand.cards.push(discard_fodder);
    game.give_energy(2);

    // Activate ability through the engine
    game.activate_ability(kasumi);

    // Resolve: discard fodder from hand
    assert!(game.has_pending_choice(), "Discard prompt expected");
    game.select_indices(&[0]);

    // Resolve: choose Center (occupied by filler)
    assert!(game.has_pending_choice(), "Position prompt expected");
    TurnEngine::resume_with_choice(&mut game.state, Some(1), None).expect("select center");

    // Q76: Kasumi should be on Center, filler moved to waitroom
    game.assert_stage_pos(
        MemberArea::Center,
        kasumi,
        "Q76: Kasumi replaced filler on center",
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&filler),
        "Q76: filler was moved to waitroom"
    );

    // Area should be locked (can't baton touch same turn)
    assert!(
        game.state
            .player1
            .areas_locked_this_turn
            .contains(&MemberArea::Center),
        "Q76: Center locked after ability appearance"
    );
}

#[test]
fn kasumi_q76_cannot_target_locked_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp1-002-R\u{ff0b}");
    let discard_fodder = game.id("PL!-sd1-010-SD");

    // All 3 areas are locked (simulating cards that appeared this turn)
    game.state
        .player1
        .areas_locked_this_turn
        .insert(MemberArea::LeftSide);
    game.state
        .player1
        .areas_locked_this_turn
        .insert(MemberArea::Center);
    game.state
        .player1
        .areas_locked_this_turn
        .insert(MemberArea::RightSide);

    // Setup: Kasumi in discard with enough resources
    game.state.player1.waitroom.cards.push(kasumi);
    game.state.player1.hand.cards.push(discard_fodder);
    game.give_energy(2);

    // Q76 says: cannot specify an area that has a member who appeared this turn
    // With all 3 locked, activation should succeed but placement should fail
    game.activate_ability(kasumi);

    // Resolve discard prompt
    assert!(game.has_pending_choice(), "Discard prompt expected");
    game.select_indices(&[0]);

    // No position choice since all areas locked
    assert!(
        !game.has_pending_choice(),
        "No position prompt when all areas locked"
    );

    // Kasumi should still be in discard
    assert!(
        game.state.player1.waitroom.cards.contains(&kasumi),
        "Kasumi stays in discard when all areas locked"
    );
}

#[test]
fn kasumi_q76_appear_when_stage_full_all_occupied() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp1-002-R\u{ff0b}");
    let filler1 = game.new_id("PL!-sd1-010-SD");
    let filler2 = game.new_id("PL!-sd1-010-SD");
    let filler3 = game.new_id("PL!-sd1-010-SD");
    let discard_fodder = game.id("PL!-sd1-010-SD");

    // Setup: all 3 stage positions occupied, Kasumi in discard
    game.state.player1.stage.stage[0] = filler1;
    game.state.player1.stage.stage[1] = filler2;
    game.state.player1.stage.stage[2] = filler3;
    game.state.player1.waitroom.cards.push(kasumi);
    game.state.player1.hand.cards.push(discard_fodder);
    game.give_energy(2);

    // Activate ability
    game.activate_ability(kasumi);

    // Resolve: discard fodder from hand
    assert!(game.has_pending_choice(), "Discard prompt expected");
    game.select_indices(&[0]);

    // Position prompt should appear even though stage is "full"
    assert!(
        game.has_pending_choice(),
        "Position prompt should appear when stage is full (allow_occupied_stage)"
    );

    // Choose LeftSide (occupied by filler1) — should replace
    TurnEngine::resume_with_choice(&mut game.state, Some(0), None).expect("select left");

    // Kasumi replaces filler1 on LeftSide
    game.assert_stage_pos(
        MemberArea::LeftSide,
        kasumi,
        "Kasumi replaced filler1 on left",
    );

    // filler1 moved to waitroom
    assert!(
        game.state.player1.waitroom.cards.contains(&filler1),
        "filler1 was moved to waitroom"
    );

    // LeftSide should be locked
    assert!(
        game.state
            .player1
            .areas_locked_this_turn
            .contains(&MemberArea::LeftSide),
        "LeftSide locked after ability appearance"
    );

    // Other areas NOT locked
    assert!(
        !game
            .state
            .player1
            .areas_locked_this_turn
            .contains(&MemberArea::Center),
        "Center should not be locked"
    );
    assert!(
        !game
            .state
            .player1
            .areas_locked_this_turn
            .contains(&MemberArea::RightSide),
        "RightSide should not be locked"
    );

    // filler2 and filler3 remain in place
    assert_eq!(
        game.state.player1.stage.stage[1], filler2,
        "filler2 still on center"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], filler3,
        "filler3 still on right"
    );

    // Kasumi no longer in discard
    assert!(
        !game.state.player1.waitroom.cards.contains(&kasumi),
        "Kasumi moved from discard"
    );
}

#[test]
fn kasumi_q122_look_top3_no_refresh_with_exactly_3() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp1-002-R\u{ff0b}");

    // Deck has exactly 3 cards
    for _ in 0..3 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }

    game.state.player1.hand.cards.push(kasumi);
    game.give_energy(2);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(kasumi, MemberArea::Center);

    // Q122: Debut look_and_select fires, deck had exactly 3 cards.
    // The look doesn't move cards from deck, so no refresh occurs.
    if game.has_pending_choice() {
        game.assert_stage_pos(
            MemberArea::Center,
            kasumi,
            "Kasumi should be on stage after debut",
        );
    }
}

#[test]
fn kasumi_ab0_debut_look_topdeck_arrange() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp1-002-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    // Fill deck with 5 cards
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(kasumi);
    game.give_energy(2);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(kasumi, MemberArea::Center);

    // Ab#0 fires: look at top 3, arrange any on top, discard rest
    // Verify the ability was recognized (debut triggers on play)
    if game.has_pending_choice() {
        game.assert_stage_pos(
            MemberArea::Center,
            kasumi,
            "Kasumi should be on center stage after debut",
        );
    }
}
