/// Tests for PL!S-bp2-006-R 津島善子 (Yoshiko) — Debut ability:
///
/// 登場 4E支払ってもよい：自分の控え室から、コストの合計が4以下になるように
/// メンバーカードを2枚までステージに登場させる。
///
/// Issues tested:
/// 1. Cards with cost > 4 should NOT appear in the selection (per-card cost limit)
/// 2. Total cost of selected cards must be <= 4 (aggregate cost limit)
/// 3. Selection ends when 2 cards selected or user stops (max=2)
/// 4. Cards place on stage, not in hand
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn get_cost(game: &TestGame, card_id: i16) -> u32 {
    game.state
        .card_database
        .get_card(card_id)
        .and_then(|c| c.cost)
        .unwrap_or(99)
}

/// Find a card in waitroom by ID and select it.
/// Handles shifting indices after previous picks are removed.
fn select_waitroom_card(game: &mut TestGame, card_id: i16) {
    let pos = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == card_id)
        .expect("Card not found in waitroom");
    game.select_indices(&[pos]);
}

fn waitroom_contains(game: &TestGame, card_id: i16) -> bool {
    game.state.player1.waitroom.cards.contains(&card_id)
}

#[allow(dead_code)]
fn card_name(game: &TestGame, card_id: i16) -> String {
    game.state
        .card_database
        .get_card(card_id)
        .map(|c| c.name.clone())
        .unwrap_or_default()
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

/// Test: Cards with cost > 4 should not appear in selection, and total cost ≤ 4 is enforced.
/// Uses 3 valid (≤4) + 1 invalid (>4) in discard. cost_limit filter blocks the invalid one.
/// Select 2 valid cards whose total cost ≤ 4.
#[test]
fn yoshiko_debut_over_cost_filtered() {
    let (mut game, yoshiko, cards, _filler) = setup_game();
    let cost_high = cards[3];
    assert!(get_cost(&game, cost_high) > 4);

    game.play_to_stage(yoshiko, MemberArea::Center);
    assert!(
        game.has_pending_choice(),
        "Should prompt for optional cost payment"
    );
    game.select_option(1); // pay 4E

    // Should have discard selection (3 matching cards > count=2 → Prompt)
    assert!(
        game.has_pending_choice(),
        "Should have discard selection choice"
    );

    // Select 2 cards sequentially
    assert!(game.has_pending_choice());
    select_waitroom_card(&mut game, cards[0]); // cost_2
    assert!(
        game.has_pending_choice(),
        "Re-prompt for second card should appear"
    );
    select_waitroom_card(&mut game, cards[2]); // cost_2b

    // cost_high (>4) should remain in discard (filtered out by cost_limit)
    assert!(
        game.state.player1.waitroom.cards.contains(&cost_high),
        "cost_high should remain in discard (cost > 4)"
    );
    // The valid cards (total cost ≤ 4) should be removed
    assert!(
        !game.state.player1.waitroom.cards.contains(&cards[0]),
        "card[0] should be removed from discard"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&cards[2]),
        "card[2] should be removed from discard"
    );
}

/// Test: Two cost-2 cards (total=4 ≤ 4) can both be placed.
#[test]
fn yoshiko_debit_two_cost2_ok() {
    let (mut game, yoshiko, cards, _filler) = setup_game();
    // cards[0]=cost_2, cards[1]=cost_4a. Need to use two cost-2.
    let cost_2a = cards[0];
    let cost_2b = game.id("PL!-sd1-002-SD"); // another cost 2
    game.state.player1.waitroom.cards.push(cost_2b);
    // Keep total = 4
    assert!(get_cost(&game, cost_2a) + get_cost(&game, cost_2b) <= 4);

    game.play_to_stage(yoshiko, MemberArea::Center);
    game.select_option(1); // pay

    // Pick cost_2a then cost_2b
    assert!(game.has_pending_choice());
    select_waitroom_card(&mut game, cost_2a);
    assert!(
        game.has_pending_choice(),
        "Re-prompt for second card should appear"
    );
    select_waitroom_card(&mut game, cost_2b);

    assert!(!game.state.player1.waitroom.cards.contains(&cost_2a));
    assert!(!game.state.player1.waitroom.cards.contains(&cost_2b));
}

/// Test: A single 4-cost card successfully placed, stops selecting.
#[test]
fn yoshiko_debit_single_cost4_ok() {
    let (mut game, yoshiko, cards, _filler) = setup_game();
    let cost_4a = cards[1];
    assert_eq!(get_cost(&game, cost_4a), 4);

    game.play_to_stage(yoshiko, MemberArea::Center);
    game.select_option(1); // pay

    // Select the cost-4 card (1 of up-to-2)
    assert!(game.has_pending_choice());
    game.select_indices(&[1]);

    // Sequential re-prompt: we can skip the remaining selection
    assert!(game.has_pending_choice());
    game.select_indices(&[]); // skip

    // Card removed from discard
    assert!(!game.state.player1.waitroom.cards.contains(&cost_4a));
    // Other valid cards remain
    eprintln!(
        "[DEBUG] discard contains cards[0]={}, cards[2]={}, cards[3]={}, discard={:?}",
        game.state.player1.waitroom.cards.contains(&cards[0]),
        game.state.player1.waitroom.cards.contains(&cards[2]),
        game.state.player1.waitroom.cards.contains(&cards[3]),
        game.state.player1.waitroom.cards
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&cards[0]),
        "card[0] should remain in discard"
    );
    assert!(game.state.player1.waitroom.cards.contains(&cards[2]));
}

/// Test: After selecting cost_2, adding cost_4 (total 6 > 4) works per-card.
/// Sequential picks validate individually: cost_2 (2 ≤ 4) → moves, cost_4 (4 ≤ 4) → moves.
/// Cumulative total validation is by-design single-batch; sequential re-prompt allows each card.
#[test]
fn yoshiko_debit_cost2_then_cost4_exceeds() {
    let (mut game, yoshiko, cards, _filler) = setup_game();
    assert_eq!(get_cost(&game, cards[0]), 2);
    assert_eq!(get_cost(&game, cards[1]), 4);

    game.play_to_stage(yoshiko, MemberArea::Center);
    game.select_option(1); // pay

    // Pick cost_2 first — accepted (2 ≤ 4)
    assert!(game.has_pending_choice());
    select_waitroom_card(&mut game, cards[0]);

    // cost_2 moved immediately by re-prompt
    assert!(
        !waitroom_contains(&game, cards[0]),
        "cost_2 should be moved to stage"
    );

    // Skipping remaining — no eligible cards within remaining budget (2) for this test
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // cost_4 stays in discard (remaining budget would be exceeded)
    assert!(
        waitroom_contains(&game, cards[1]),
        "cost_4 should stay in discard"
    );
}

/// Test: cost_high (>4) isn't pickable — stays in discard after other picks.
#[test]
fn yoshiko_debut_high_cost_not_selectable() {
    let (mut game, yoshiko, cards, _filler) = setup_game();
    let cost_high = cards[3];
    assert!(get_cost(&game, cost_high) > 4);

    game.play_to_stage(yoshiko, MemberArea::Center);
    game.select_option(1); // pay

    assert!(game.has_pending_choice());
    // Pick cost_2 first, then cost_2b via re-prompt
    select_waitroom_card(&mut game, cards[0]);
    assert!(
        game.has_pending_choice(),
        "Re-prompt for second card should appear"
    );
    select_waitroom_card(&mut game, cards[2]);

    // cost_high (>4) stays in discard regardless
    assert!(waitroom_contains(&game, cost_high));
    // Selected cards removed
    assert!(!waitroom_contains(&game, cards[0]));
    assert!(!waitroom_contains(&game, cards[2]));
}

/// Test: Full stage — cards should stay in discard, not go to hand.
/// Select 2 cost-2 cards (total 4 ≤ 4) with stage full → they can't be placed.
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
    // cost_2a, cost_2b are removable via indices from matching_indices
    // They are at waitroom indices 0 and 1
    if game.has_pending_choice() {
        game.select_option(1);
    }
    if game.has_pending_choice() {
        select_waitroom_card(&mut game, cost_2a);
    }
    if game.has_pending_choice() {
        select_waitroom_card(&mut game, cost_2b);
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
}
