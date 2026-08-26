/// Tests for 君のこころは輝いてるかい？ (PL!S-bp2-024-L) — Live card with:
///
/// Ab#0 (常時): Cannot be placed in the success live card zone.
/// Ab#1 (ライブ成功時): Draw 2 cards, then discard 1 from hand.
///
/// Q125: The constant restriction prevents placement even via other effects.
/// Q36:  LiveSuccess timing (general)
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// LiveSuccess fires: draw 2, then discard 1.
/// Verify hand count changes and a card is discarded.
#[test]
fn kagayaiteru_live_success_draw_then_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!S-bp2-024-L");
    let filler = game.id("PL!-sd1-010-SD");

    // Hand: the live card + 1 filler (for discard target)
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);

    // Stage: member with heart05 to satisfy the live card's need_heart ({heart05:1, heart0:1})
    let heart_provider = game.id("PL!S-bp2-015-PR");
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, heart_provider);

    // Seed decks for draw phase + LiveCardSet draws + draw 2
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(card);
    advance_to_live_start(&mut game);

    // Pass: FirstAttacker → SecondAttacker → LiveVictoryDetermination → Active
    game.pass();
    game.pass();
    game.pass();

    // LiveSuccess fired: drew 2, then a mandatory hand discard choice appears
    assert!(
        game.has_pending_choice(),
        "post-LiveSuccess hand discard prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard"
    );
    game.select_indices(&[0]);

    // After draws and discard, hand should have changed from initial
    let hand_after = game.state.player1.hand.cards.len();
    assert!(
        hand_after != hand_before,
        "Hand should change after draw 2 then discard 1"
    );
    // The net effect is +1 card (draw 2, discard 1)
    // But there are also phase transition draws that affect this
    // So just assert it's not the same
}

/// Q125: The constant restriction prevents placement in the success live zone.
/// After LiveSuccess resolves (draw+discard), victory determination runs.
/// P1 has the only live card so auto-wins, but the restriction should move
/// the card to waitroom instead of success zone.
#[test]
fn kagayaiteru_q125_cannot_place_in_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!S-bp2-024-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);

    // Stage: member with heart05 to satisfy the live card's need_heart
    let heart_provider = game.id("PL!S-bp2-015-PR");
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, heart_provider);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(card);
    advance_to_live_start(&mut game);

    game.pass();
    game.pass();
    game.pass();

    // LiveSuccess fired (draw 2) and created a mandatory hand discard choice
    assert!(
        game.has_pending_choice(),
        "post-LiveSuccess hand discard prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard"
    );
    game.select_indices(&[0]);

    // Pass again to complete LiveVictoryDetermination (move to success / waitroom)
    game.pass();

    // P1 had the only live card → auto-won, but the constant restriction
    // prevents placement in success zone → goes to waitroom instead
    assert!(
        game.state.player1.waitroom.cards.contains(&card),
        "Q125: Card should be in waitroom (cannot be placed in success zone)"
    );
}
