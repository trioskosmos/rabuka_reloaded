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
