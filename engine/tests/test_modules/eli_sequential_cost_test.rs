/// Tests for PL!-bp5-002-R (Ayase Eli) — sequential_cost with wait + optional discard
use crate::helpers::*;

/// Sequential cost: put to wait (optional), discard 1 (optional) → look at 5 from deck,
/// select cost≥9 μ's member to hand, discard rest.
#[test]
fn eli_bp5_sequential_wait_then_discard_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = game.id("PL!-bp5-002-R");
    let filler = game.id("PL!-sd1-010-SD");
    let mus_high_cost = game.id("PL!-bp3-002-R"); // Eli, cost 9, μ's member

    game.state.player1.hand.cards.push(eli);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);

    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(mus_high_cost);
    }
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(eli, rabuka_engine::zones::MemberArea::Center);

    // Choice 1: wait is auto-paid (binary cost), discard prompt shows directly
    assert!(game.has_pending_choice(), "Discard prompt");
    assert_eq!(
        game.state
            .get_pending_choice_json()
            .as_ref()
            .and_then(|v| v.get("zone"))
            .and_then(|v| v.as_str()),
        Some("hand"),
        "Should be hand discard, got {:?}",
        game.state.get_pending_choice_json()
    );
    game.select_indices(&[0]);

    // Remaining: looked_at selection — the first 5 are μ's cost 9 → match
    // Pick 1 to hand
    assert!(
        game.has_pending_choice(),
        "looked_at selection prompt expected after hand discard"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (looked_at)"
    );
    game.select_indices(&[0]);

    // Hand had 2 cards after play, cost discarded 1 → hand = 1,
    // then looked_at selection added 1 → hand = 2
    assert_eq!(game.state.player1.hand.cards.len(), 2);
    let w = game.state.mods.get_orientation_modifier(eli);
    assert!(w.is_some());
    assert_eq!(w.unwrap(), "wait");
}

#[test]
fn eli_bp5_skip_both_costs_still_looks() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = game.id("PL!-bp5-002-R");
    let filler = game.id("PL!-sd1-010-SD");
    let mus_high = game.id("PL!-bp3-002-R");
    game.state.player1.hand.cards.push(eli);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(mus_high); }
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(eli, rabuka_engine::zones::MemberArea::Center);
    assert!(game.has_pending_choice(), "discard prompt expected");
    // Skip discard (empty selection) — ability is optional, should still continue to look phase
    game.select_indices(&[]);
    // Drain any remaining choices: the look phase is optional (count 1 optional true), so skipping it should just discard
    let mut safety = 0;
    while game.has_pending_choice() && safety < 5 {
        safety += 1;
        game.select_indices(&[]);
    }
    // After playing, hand should be 0 or 1; just verify no panic and look completed
    assert!(game.state.player1.hand.cards.len() <= 1, "hand should be 0 or 1 after skipping");
    // Look phase completed (no pending choice remains)
    assert!(!game.has_pending_choice(), "no pending choice after draining");
}

#[test]
fn eli_bp5_no_eligible_look_discards_all() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = game.id("PL!-bp5-002-R");
    let filler = game.id("PL!-sd1-010-SD"); // filler is not μ's cost 9, so no eligible
    game.state.player1.hand.cards.push(eli);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    let wait_before = game.state.player1.waitroom.cards.len();
    game.play_to_stage(eli, rabuka_engine::zones::MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]); // discard filler
    // Drain look choices (with no eligible, engine may auto-discard)
    let mut safety = 0;
    while game.has_pending_choice() && safety < 5 {
        safety += 1;
        game.select_indices(&[]);
    }
    assert!(game.state.player1.hand.cards.len() <= 1, "hand should be 0 or 1");
    assert!(game.state.player1.waitroom.cards.len() >= wait_before + 5, "at least 5 looked discarded");
}



#[test]
fn eli_bp5_look_select_optional_skip_keeps_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = game.id("PL!-bp5-002-R");
    let filler = game.id("PL!-sd1-010-SD");
    let mus_high = game.id("PL!-bp3-002-R");
    game.state.player1.hand.cards.push(eli);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);
    // Top 5: 2 eligible, 3 filler
    for _ in 0..2 { game.state.player1.main_deck.cards.push(mus_high); }
    for _ in 0..3 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(eli, rabuka_engine::zones::MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    // Drain look choices, skip optional select even though eligible exists
    let mut safety = 0;
    while game.has_pending_choice() && safety < 5 {
        safety += 1;
        game.select_indices(&[]);
    }
    assert!(game.state.player1.hand.cards.len() <= 1, "skip must not add beyond 1");
}
