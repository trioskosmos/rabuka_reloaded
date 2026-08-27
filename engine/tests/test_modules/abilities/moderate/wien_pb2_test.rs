/// Tests for ウィーン・マルガレーテ (PL!SP-pb2-010-R/PP) — LiveStart ability:
///
/// ライブ開始時: 手札を1枚控え室に置かないかぎり、自分のエネルギー1枚をエネルギーデッキに置く。
///
/// Q262: When hand is empty at live start, you cannot discard, so you must
/// place energy from your field to the energy deck.
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Q262: 0 cards in hand (no discard possible) → must place energy to energy deck.
#[test]
fn wien_q262_empty_hand_triggers_energy_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-pb2-010-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Wien on center stage, give player energy on field
    game.state.player1.stage.stage[1] = wien;
    game.give_energy(5);

    // Hand starts empty — no cards to discard
    // Need a live card but hand is empty, so we add it temporarily
    game.state.player1.hand.cards.push(live_card);

    advance_to_live_card_set_p1(&mut game);

    // Hand should now have only the live card at this point
    // Set it as the live card — after this hand is empty
    game.set_live_card(live_card);

    let energy_zone_before = game.state.player1.energy_zone.active_count();
    let energy_deck_before = game.state.player1.energy_deck.cards.len();

    advance_to_live_start(&mut game);

    // Q262: Engine presents Skip/Pay choice for the optional discard.
    assert!(
        game.has_pending_choice(),
        "Q262: Engine should present Skip/Pay choice"
    );
    game.select_option(0); // Option 0 = Skip → queues move_cards effect

    // Q262: Energy zone→energy_deck movement auto-takes (fungible cards,
    // no per-card selection needed after the optional discard skip).

    // Q262: Energy card should have moved from energy zone to energy deck
    assert!(
        game.state.player1.energy_zone.active_count() < energy_zone_before,
        "Q262: An energy card should have left the energy zone"
    );
    assert!(
        game.state.player1.energy_deck.cards.len() > energy_deck_before,
        "Q262: Energy deck should have gained a card"
    );
}

/// When hand has a discardable card and you discard, energy must NOT move (negation).
#[test]
fn wien_q262_discard_prevents_energy_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wien = game.id("PL!SP-pb2-010-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[1] = wien;
    game.give_energy(5);
    // Hand has 2 cards: live + filler discardable
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    advance_to_live_card_set_p1(&mut game);
    // Set live card — hand still has filler
    game.set_live_card(live_card);
    assert_eq!(game.state.player1.hand.cards.len(), 1, "hand should have filler after live set");
    let energy_zone_before = game.state.player1.energy_zone.active_count();
    let energy_deck_before = game.state.player1.energy_deck.cards.len();
    advance_to_live_start(&mut game);
    assert!(game.has_pending_choice(), "should present discard choice");
    // Choose to discard (option 1 is usually Pay/Discard)
    game.select_option(1);
    // If engine presents a follow-up SelectCard for which card to discard, handle it
    let mut safety = 0;
    while game.has_pending_choice() && safety < 5 {
        safety += 1;
        let ct = game.pending_choice_type().unwrap_or_default();
        if ct == "SelectCard" {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
    }
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_zone_before,
        "discarding should prevent energy move"
    );
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        energy_deck_before,
        "energy deck unchanged when discard taken"
    );
}

#[test]
fn wien_live_success_choice_draw_via_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wien = game.id("PL!SP-pb2-010-R");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[1] = wien;
    let hand_before = game.state.player1.hand.cards.len();
    // Fire LiveSuccess directly (choice: draw 2 vs place energy wait)
    crate::helpers::fire_trigger(
        &mut game,
        wien,
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert!(game.has_pending_choice(), "LiveSuccess should present choice");
    // Option 0 = draw 2
    game.select_option(0);
    // Drain any follow-up (draw is immediate)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 2,
        "draw 2 should increase hand by 2"
    );
}

#[test]
fn wien_live_success_choice_energy_via_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wien = game.id("PL!SP-pb2-010-R");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[1] = wien;
    game.give_energy(3);
    let hand_before = game.state.player1.hand.cards.len();
    crate::helpers::fire_trigger(
        &mut game,
        wien,
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert!(game.has_pending_choice(), "LiveSuccess should present choice");
    // Option 1 = place energy wait (second option) — should NOT draw
    game.select_option(1);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "energy choice should not draw cards"
    );
    assert!(!game.has_pending_choice(), "choice should be resolved");
}


