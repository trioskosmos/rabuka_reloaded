/// Tests for PL!S-bp2-006-R 津島善子 (Yoshiko) — Debut ability:
///
/// 登場 4E支払ってもよい：自分の控え室から、コストの合計が4以下になるように
/// メンバーカードを2枚までステージに登場させる。
///
/// Issues tested:
/// 1. Cards with cost > 4 should NOT appear in the selection (per-card cost limit)
/// 2. Total cost of selected cards must be <= 4 (aggregate cost limit)
/// 3. Each placed card gets a position choice when >1 empty slots
/// 4. 登場 (Debut) trigger fires for each card placed on stage
/// 5. Selection ends when 2 cards selected or user stops (max=2)
/// 6. Cards place on stage, not in hand
/// 7. filterred_indices on initial prompt only includes cards with cost ≤ 4 (Bug 1 fix)
/// 8. Second card selection correctly places card with debut (Bug 2 fix)
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

fn get_cost(game: &TestGame, card_id: i16) -> u8 {
    game.state
        .card_database
        .get_card(card_id)
        .and_then(|c| c.cost)
        .unwrap_or(99)
}

fn waitroom_contains(game: &TestGame, card_id: i16) -> bool {
    game.state.player1.waitroom.cards.contains(&card_id)
}

fn setup_game() -> (TestGame, i16, Vec<i16>, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp2-006-R");

    // μ's SD cards with known costs
    let cost_2 = game.id("PL!-sd1-002-SD"); // cost 2 (Eli)
    let cost_2b = game.id("PL!-sd1-005-SD"); // cost 2 (Honoka)
    let cost_4 = game.id("PL!-sd1-008-SD"); // cost 4 (Nozomi)
    let cost_high = game.id("PL!-sd1-001-SD"); // cost 11 (> 4)
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.waitroom.cards.push(cost_2); // index 0
    game.state.player1.waitroom.cards.push(cost_4); // index 1
    game.state.player1.waitroom.cards.push(cost_2b); // index 2
    game.state.player1.waitroom.cards.push(cost_high); // index 3 (cost > 4)

    game.state.player1.hand.cards.push(yoshiko);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(16);
    game.state.player1.stage.stage = [-1, -1, -1];

    (
        game,
        yoshiko,
        vec![cost_2, cost_4, cost_2b, cost_high],
        filler,
    )
}

/// Assert the pending choice is a SelectPosition and choose a position.
/// position_idx maps: 0=left, 1=center, 2=right (raw stage index).
fn select_stage_position(game: &mut TestGame, position_idx: i16) {
    assert!(game.has_pending_choice(), "Should have position choice");
    let choice = game.get_pending_choice();
    assert!(
        matches!(choice, Choice::SelectPosition { .. }),
        "Expected SelectPosition, got {:?}",
        choice
    );
    game.select_option(position_idx);
}

/// Assert stage matches expected card IDs at each position.
fn assert_stage(game: &TestGame, expected: [i16; 3]) {
    assert_eq!(
        game.state.player1.stage.stage,
        expected,
        "Stage mismatch: left={}, center={}, right={}",
        game.state.player1.stage.stage[0],
        game.state.player1.stage.stage[1],
        game.state.player1.stage.stage[2],
    );
}

/// Test: Cards with cost > 4 filtered out. Sequential pick → position → debut
/// for each placed card.
#[test]
fn yoshiko_debut_over_cost_filtered() {
    let (mut game, yoshiko, cards, _filler) = setup_game();
    let cost_high = cards[3];
    assert!(get_cost(&game, cost_high) > 4);

    game.play_to_stage(yoshiko, MemberArea::Center);
    assert_eq!(game.state.player1.debut_count_this_turn, 1);
    assert_stage(&game, [-1, yoshiko, -1]);

    assert!(
        game.has_pending_choice(),
        "Should prompt for optional cost payment"
    );
    game.select_option(1); // pay 4E

    assert!(
        game.has_pending_choice(),
        "Should have discard selection choice"
    );

    // Verify initial prompt's filtered_indices only includes cards with cost ≤ 4 (Bug 1 fix)
    if let Choice::SelectCard {
        cost_total,
        filtered_indices,
        ..
    } = game.get_pending_choice()
    {
        assert_eq!(
            *cost_total,
            Some(4),
            "cost_total should be 4 on initial prompt"
        );
        assert!(
            filtered_indices.is_some(),
            "filtered_indices should be Some on initial prompt"
        );
        let fi = filtered_indices.as_ref().unwrap();
        assert!(
            !fi.contains(&3),
            "cost_high (index 3) should NOT be in filtered_indices (cost > 4)"
        );
        assert!(
            fi.contains(&0),
            "cost_2 (index 0) should be in filtered_indices"
        );
        assert!(
            fi.contains(&1),
            "cost_4 (index 1) should be in filtered_indices"
        );
        assert!(
            fi.contains(&2),
            "cost_2b (index 2) should be in filtered_indices"
        );
        assert_eq!(
            fi.len(),
            3,
            "filtered_indices should have exactly 3 entries (cost ≤ 4)"
        );
    }

    // Select first card (cost_2) from discard
    assert!(game.has_pending_choice());
    game.select_waitroom_card_filtered(cards[0]); // cost_2

    // Position choice for cost_2: stage=[-1, yoshi, -1], empty=[0,2]
    select_stage_position(&mut game, 0); // left
    assert_eq!(
        game.state.player1.debut_count_this_turn, 2,
        "Debut should fire for cost_2"
    );
    assert_stage(&game, [cards[0], yoshiko, -1]);

    // Re-prompt for second card
    assert!(
        game.has_pending_choice(),
        "Re-prompt for second card should appear"
    );
    // Verify re-prompt's filtered_indices: remaining budget = 2, only cost_2b (index 0 in current waitroom) fits
    if let Choice::SelectCard {
        cost_total,
        filtered_indices,
        ..
    } = game.get_pending_choice()
    {
        assert_eq!(
            *cost_total,
            Some(2),
            "cost_total should be 2 (remaining budget)"
        );
        let fi = filtered_indices.as_ref().unwrap();
        assert_eq!(fi.len(), 1, "only one card should fit remaining budget");
        let cid = game.state.player1.waitroom.cards[fi[0]];
        assert_eq!(
            get_cost(&game, cid),
            2,
            "only cost-2 card should be in filtered_indices"
        );
    }
    game.select_waitroom_card_filtered(cards[2]); // cost_2b

    // cost_2b placed directly at right (only empty slot), no position choice
    assert_stage(&game, [cards[0], yoshiko, cards[2]]);
    assert_eq!(
        game.state.player1.debut_count_this_turn, 3,
        "Debut should fire for cost_2b"
    );

    // cost_high (>4) should remain in discard
    assert!(
        waitroom_contains(&game, cost_high),
        "cost_high should remain in discard (cost > 4)"
    );
    // Selected cards removed from discard
    assert!(
        !waitroom_contains(&game, cards[0]),
        "card[0] should be removed from discard"
    );
    assert!(
        !waitroom_contains(&game, cards[2]),
        "card[2] should be removed from discard"
    );
}

/// Test: Two cost-2 cards (total=4) both placed with correct positions and debuts.
#[test]
fn yoshiko_debit_two_cost2_ok() {
    let (mut game, yoshiko, cards, _filler) = setup_game();
    let cost_2a = cards[0];
    let cost_2b = game.id("PL!-sd1-002-SD");
    game.state.player1.waitroom.cards.push(cost_2b);
    assert!(get_cost(&game, cost_2a) + get_cost(&game, cost_2b) <= 4);

    game.play_to_stage(yoshiko, MemberArea::Center);
    assert_eq!(game.state.player1.debut_count_this_turn, 1);
    assert_stage(&game, [-1, yoshiko, -1]);

    game.select_option(1); // pay

    // Pick cost_2a → position choice → left
    assert!(game.has_pending_choice());
    game.select_waitroom_card_filtered(cost_2a);
    select_stage_position(&mut game, 0); // left
    assert_eq!(game.state.player1.debut_count_this_turn, 2);
    assert_stage(&game, [cost_2a, yoshiko, -1]);

    // Re-prompt → pick cost_2b → direct to right (only empty slot)
    assert!(game.has_pending_choice(), "Re-prompt for second card");
    game.select_waitroom_card_filtered(cost_2b);
    assert_stage(&game, [cost_2a, yoshiko, cost_2b]);
    assert_eq!(game.state.player1.debut_count_this_turn, 3);

    assert!(!waitroom_contains(&game, cost_2a));
    assert!(!waitroom_contains(&game, cost_2b));
}

/// Test: Single cost-4 card → position choice → placed → skip remaining.
#[test]
fn yoshiko_debit_single_cost4_ok() {
    let (mut game, yoshiko, cards, _filler) = setup_game();
    let cost_4a = cards[1];
    assert_eq!(get_cost(&game, cost_4a), 4);

    game.play_to_stage(yoshiko, MemberArea::Center);
    assert_eq!(game.state.player1.debut_count_this_turn, 1);
    assert_stage(&game, [-1, yoshiko, -1]);

    game.select_option(1); // pay

    // Select cost-4 from discard using filtered index
    assert!(game.has_pending_choice());
    game.select_waitroom_card_filtered(cost_4a);

    // Position choice for cost_4: empty=[0,2] → choose left
    select_stage_position(&mut game, 0); // left
    assert_eq!(game.state.player1.debut_count_this_turn, 2);
    assert_stage(&game, [cost_4a, yoshiko, -1]);

    // Re-prompt: remaining budget=0, skip
    assert!(game.has_pending_choice(), "Skip re-prompt should appear");
    if let Choice::SelectCard {
        cost_total,
        filtered_indices,
        ..
    } = game.get_pending_choice()
    {
        assert_eq!(
            *cost_total,
            Some(0),
            "cost_total should be 0 (budget exhausted)"
        );
        assert!(
            filtered_indices.as_ref().map_or(false, |fi| fi.is_empty()),
            "filtered_indices should be empty (no cards fit remaining budget)"
        );
    }
    game.select_indices(&[]); // skip

    assert!(!waitroom_contains(&game, cost_4a));
    assert!(waitroom_contains(&game, cards[0]));
    assert!(waitroom_contains(&game, cards[2]));
    assert_eq!(game.state.player1.debut_count_this_turn, 2);
    assert_stage(&game, [cost_4a, yoshiko, -1]);
}

/// Test: After selecting cost_2 (budget 2/4), cost_4 exceeds remaining → skip.
/// Also verifies the re-prompt choice has correct cost_total and budget filtering.
#[test]
fn yoshiko_debit_cost2_then_cost4_exceeds() {
    let (mut game, yoshiko, cards, _filler) = setup_game();
    assert_eq!(get_cost(&game, cards[0]), 2);
    assert_eq!(get_cost(&game, cards[1]), 4);

    game.play_to_stage(yoshiko, MemberArea::Center);
    assert_eq!(game.state.player1.debut_count_this_turn, 1);
    assert_stage(&game, [-1, yoshiko, -1]);

    game.select_option(1); // pay

    // Pick cost_2 → position choice → left
    assert!(game.has_pending_choice());
    game.select_waitroom_card_filtered(cards[0]);
    select_stage_position(&mut game, 0); // left
    assert!(!waitroom_contains(&game, cards[0]), "cost_2 moved to stage");
    assert_eq!(game.state.player1.debut_count_this_turn, 2);
    assert_stage(&game, [cards[0], yoshiko, -1]);

    // Re-prompt: remaining budget=2, cost_4=4 > 2
    assert!(game.has_pending_choice(), "Skip re-prompt should appear");
    // Inspect the re-prompt choice to verify cost_total and filtered_indices
    if let Choice::SelectCard {
        cost_total,
        cost_total_operator,
        filtered_indices,
        ..
    } = game.get_pending_choice()
    {
        assert_eq!(
            *cost_total,
            Some(2),
            "cost_total should be 2 (remaining budget)"
        );
        assert_eq!(
            cost_total_operator.as_deref(),
            Some("<="),
            "operator should be <="
        );
        assert!(
            filtered_indices.is_some(),
            "filtered_indices should be Some"
        );
        let indices = filtered_indices.as_ref().unwrap();
        // After cost_2 removed, waitroom = [cost_4(idx 0), cost_2b(idx 1), cost_high(idx 2)]
        // Only cost_2b (cost=2) fits remaining budget of 2
        assert_eq!(
            indices.len(),
            1,
            "Only one card should fit remaining budget"
        );
        let idx = indices[0];
        assert!(
            idx < game.state.player1.waitroom.cards.len(),
            "filtered index {} out of bounds for waitroom len {}",
            idx,
            game.state.player1.waitroom.cards.len()
        );
        let cid = game.state.player1.waitroom.cards[idx];
        assert_eq!(
            get_cost(&game, cid),
            2,
            "Only cost-2 card should be in filtered_indices, got cost {}",
            get_cost(&game, cid)
        );
        // The cost_4 card (now at new index 0) must NOT be in filtered_indices
        assert!(
            !indices.contains(&0),
            "cost-4 card at waitroom[0] should NOT be in filtered_indices"
        );
    }
    game.select_indices(&[]); // skip

    assert!(
        waitroom_contains(&game, cards[1]),
        "cost_4 should stay in discard"
    );
    assert_eq!(game.state.player1.debut_count_this_turn, 2);
    assert_stage(&game, [cards[0], yoshiko, -1]);
}

/// Test: After selecting cost_2, selecting via filtered index [0] correctly maps
/// to the only budget-fitting card (cost_2b at waitroom[1]), not cost_4.
/// With filtered_indices fix, the frontend's filtered-relative index [0]
/// maps through filtered_indices [1] → waitroom[1] = cost_2b, which is
/// within the remaining budget of 2.
#[test]
fn yoshiko_debit_cost2_redirects_to_budget_card() {
    let (mut game, yoshiko, cards, _filler) = setup_game();
    let cost_2b = cards[2];
    assert_eq!(get_cost(&game, cards[0]), 2);
    assert_eq!(get_cost(&game, cards[1]), 4);

    game.play_to_stage(yoshiko, MemberArea::Center);
    game.select_option(1); // pay

    // Pick cost_2 → position → left
    assert!(game.has_pending_choice());
    game.select_waitroom_card_filtered(cards[0]);
    select_stage_position(&mut game, 0);
    assert_stage(&game, [cards[0], yoshiko, -1]);

    // Re-prompt with cost_total=Some(2) (remaining budget)
    assert!(game.has_pending_choice(), "Re-prompt should appear");

    // filtered_indices = [1] (only cost_2b at waitroom[1] fits budget).
    // Send filtered index [0] → maps through fi[0]=1 → waitroom[1] = cost_2b.
    // cost_2b (cost=2) is within remaining budget (2) → accepted.
    game.select_indices(&[0]);

    // cost_2b should be placed on stage
    assert!(
        !waitroom_contains(&game, cost_2b),
        "cost_2b should be removed from discard (selected via filtered index)"
    );
    // Stage should now have cost_2b at right
    assert_stage(&game, [cards[0], yoshiko, cost_2b]);
    // cost_4 should still be in discard (not selected)
    assert!(
        waitroom_contains(&game, cards[1]),
        "cost_4 should remain in discard"
    );
    // cost_2 + cost_2b debuts
    assert_eq!(game.state.player1.debut_count_this_turn, 3);
}

/// Test: High-cost (>4) never in selection, stays in discard.
#[test]
fn yoshiko_debut_high_cost_not_selectable() {
    let (mut game, yoshiko, cards, _filler) = setup_game();
    let cost_high = cards[3];
    assert!(get_cost(&game, cost_high) > 4);

    game.play_to_stage(yoshiko, MemberArea::Center);
    assert_eq!(game.state.player1.debut_count_this_turn, 1);
    assert_stage(&game, [-1, yoshiko, -1]);

    game.select_option(1); // pay

    // Pick cost_2 → position → left
    assert!(game.has_pending_choice());
    game.select_waitroom_card_filtered(cards[0]);
    select_stage_position(&mut game, 0);
    assert_eq!(game.state.player1.debut_count_this_turn, 2);

    // Re-prompt → pick cost_2b → direct to right
    assert!(game.has_pending_choice(), "Re-prompt for second card");
    game.select_waitroom_card_filtered(cards[2]);
    assert_stage(&game, [cards[0], yoshiko, cards[2]]);
    assert_eq!(game.state.player1.debut_count_this_turn, 3);

    assert!(waitroom_contains(&game, cost_high));
    assert!(!waitroom_contains(&game, cards[0]));
    assert!(!waitroom_contains(&game, cards[2]));
}

/// Test: Full stage → cards stay in discard, no placement, no extra debut.
#[test]
fn yoshiko_debit_stage_full_graceful() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yoshiko = game.id("PL!S-bp2-006-R");
    let cost_2a = game.id("PL!-sd1-002-SD");
    let cost_2b = game.id("PL!-sd1-002-SD");
    let filler_m = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler_m, filler_m, filler_m]; // full
    game.state.player1.waitroom.cards.push(cost_2a);
    game.state.player1.waitroom.cards.push(cost_2b);
    game.state.player1.hand.cards.push(yoshiko);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(16);

    game.play_to_stage(yoshiko, MemberArea::Center);

    if game.has_pending_choice() {
        game.select_option(1);
    }
    if game.has_pending_choice() {
        game.select_waitroom_card_filtered(cost_2a);
    }
    if game.has_pending_choice() {
        game.select_waitroom_card_filtered(cost_2b);
    }

    // Stage full → cards stay in discard, NOT moved to hand
    assert!(
        waitroom_contains(&game, cost_2a),
        "cost_2a should stay in discard when stage is full"
    );
    assert!(
        waitroom_contains(&game, cost_2b),
        "cost_2b should stay in discard when stage is full"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&cost_2a),
        "Should NOT route to hand"
    );
    // Stage unchanged (yoshiko replaced center filler at play_to_stage)
    assert_eq!(
        game.state.player1.stage.stage[1], yoshiko,
        "Yoshiko should be at center"
    );
    // Only yoshiko's debut fired; placed cards couldn't appear
    assert_eq!(
        game.state.player1.debut_count_this_turn, 1,
        "No extra debut triggers when stage is full"
    );
}
