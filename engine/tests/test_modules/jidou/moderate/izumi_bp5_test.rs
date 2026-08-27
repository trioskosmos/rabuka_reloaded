/// Tests for PL!HS-bp5-008-R (桂城 泉 ab#0) — sequential_cost
/// "このメンバーをウェイトにし、手札を1枚控え室に置いてもよい" =
///   wait (optional, self_cost) + discard 1 from hand (optional).
/// The combined prompt shows once. Confirm → both execute. Skip → neither.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Pay the sequential cost: wait + discard. Both should apply.
#[test]
fn izumi_bp5_pay_cost_waits_and_discards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(izumi);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4); // izumi costs 4 energy to play

    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(izumi, MemberArea::Center);

    // Combined prompt: wait + discard 1 (or skip)
    assert!(
        game.has_pending_choice(),
        "Combined cost prompt should appear"
    );
    let json = game.state.get_pending_choice_json();
    assert_eq!(
        json.as_ref()
            .and_then(|v| v.get("zone"))
            .and_then(|v| v.as_str()),
        Some("hand"),
        "Should be hand discard prompt"
    );

    // Confirm: select the filler card from hand
    game.select_indices(&[0]);

    // Wait should now be paid
    let w = game.state.mods.get_orientation_modifier(izumi);
    assert!(w.is_some(), "Wait should be applied after paying cost");
    assert_eq!(w.unwrap(), "wait");

    // After discard, hand goes from [filler] to []
    // Then look_and_select fires → no matching cards (filler ≠ 蓮ノ空 + cost≥9) → auto-skip
    // No pending choice should remain
    assert!(
        !game.has_pending_choice(),
        "No pending choice after auto-skip"
    );

    assert_eq!(game.state.player1.hand.cards.len(), 0);
}

/// Skip the sequential cost: neither wait nor discard should apply.
#[test]
fn izumi_bp5_skip_cost_no_wait_no_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(izumi);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4); // izumi costs 4 energy to play

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(izumi, MemberArea::Center);

    // Combined prompt: skip it (empty indices = skip for SelectCard)
    assert!(
        game.has_pending_choice(),
        "Combined cost prompt should appear"
    );
    game.select_indices(&[]);

    // Wait should NOT have been paid
    let w = game.state.mods.get_orientation_modifier(izumi);
    assert!(
        w.is_none(),
        "Wait should NOT be applied when cost is skipped"
    );

    // Hand should be unchanged (still has filler, izumi was played to stage)
    assert_eq!(game.state.player1.hand.cards.len(), 1);
}

#[test]
fn izumi_bp5_look_with_eligible_hasu_selects() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");
    let hasu9 = game.id("PL!HS-sd1-001-SD"); // cost 9 Hasunosora eligible
    game.state.player1.hand.cards.push(izumi);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);
    // Deck top 5: 1 eligible + 4 filler (ensure eligible in first 5)
    for _ in 0..4 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.main_deck.cards.push(hasu9);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(izumi, MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]); // pay cost: discard filler, wait
    assert!(game.has_pending_choice(), "look select should appear with eligible");
    assert_eq!(game.pending_choice_type().as_deref(), Some("SelectCard"));
    // Select the eligible Hasunosora
    game.select_indices(&[0]);
    assert!(game.state.player1.hand.cards.contains(&hasu9), "Hasunosora cost9 should be in hand");
    assert!(!game.has_pending_choice(), "no pending after select");
}

#[test]
fn izumi_bp5_look_with_eligible_skip_keeps_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");
    let hasu9 = game.id("PL!HS-sd1-001-SD");
    game.state.player1.hand.cards.push(izumi);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);
    for _ in 0..4 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.main_deck.cards.push(hasu9);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(izumi, MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    // Skip optional select even though eligible exists
    game.select_indices(&[]);
    assert!(!game.state.player1.hand.cards.contains(&hasu9), "skipped select should not add Hasunosora");
    assert!(!game.has_pending_choice());
}

#[test]
fn izumi_bp5_look_no_eligible_auto_discards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp5-008-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(izumi);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [-1, -1, -1];
    let wait_before = game.state.player1.waitroom.cards.len();
    game.play_to_stage(izumi, MemberArea::Center);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    // No eligible in top 5 (all filler), should auto-discard 5 to waitroom and end
    let mut safety = 0;
    while game.has_pending_choice() && safety < 5 {
        safety += 1;
        game.select_indices(&[]);
    }
    assert!(!game.has_pending_choice(), "no eligible should auto end");
    assert!(game.state.player1.waitroom.cards.len() >= wait_before + 6, "5 looked + 1 discard to waitroom");
}
