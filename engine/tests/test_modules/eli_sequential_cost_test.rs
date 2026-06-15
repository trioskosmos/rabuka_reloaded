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

    game.state.player1.hand.cards.push(eli);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(9);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(eli, rabuka_engine::zones::MemberArea::Center);

    // Choice 1: wait cost — pay
    assert!(game.has_pending_choice(), "Wait cost prompt");
    game.select_option(1);

    // Choice 2: discard — verify it's a hand SelectCard, not a SelectTarget
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

    // Remaining: looked_at selection — skip
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(game.state.player1.hand.cards.len(), 1);
    let w = game.state.mods.get_orientation_modifier(eli);
    assert!(w.is_some());
    assert_eq!(w.unwrap().as_str(), "wait");
}
