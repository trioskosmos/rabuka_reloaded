use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn bp3_001_activation_waits_self_draw_then_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp3-001-R"); // cost 13
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.activate_ability(card);

    // Cost paid: self waited.
    assert_eq!(
        game.state.mods.get_orientation_modifier(card),
        Some("wait"),
        "cost: member should be waited"
    );
    // Effect: drew 1, now must discard 1 from hand. The discard is
    // mandatory ("手札を1枚控え室に置く" — no もよい), so allow_skip=false.
    match game.get_pending_choice() {
        Choice::SelectCard {
            zone, allow_skip, ..
        } => {
            assert_eq!(zone, "hand", "discard must come from hand");
            assert!(!allow_skip, "mandatory discard must not be skippable");
        }
        _other => panic!(
            "expected SelectCard for the hand discard, got {}",
            game.pending_choice_summary()
        ),
    }
    let hand_at_prompt = game.state.player1.hand.cards.len();
    let discard_target = game.state.player1.hand.cards[0];
    game.select_indices(&[0]);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_at_prompt - 1,
        "resolve discard: prompt-time hand minus the discarded card"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&discard_target),
        "selected card must be in the waitroom"
    );

    // {{ターン1回}} — a second press in the same turn is rejected outright:
    // the member is waited (activations need an active member), so the
    // engine refuses before even reaching the use-limit bookkeeping.
    let err = game.try_activate_ability(card).unwrap_err();
    assert!(
        err.contains("No activatable") || err.contains("turn") || err.contains("already"),
        "expected second activation to be rejected, got: {}",
        err
    );
}
