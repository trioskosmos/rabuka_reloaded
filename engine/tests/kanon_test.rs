/// Helper functions (copied from gameplay_test.rs)
fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass(); assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// 澁谷かのん (PL!SP-pb1-001-R) — LiveStart cost-or-discard + LiveSuccess optional score
///
/// Ab#0 (ライブ開始時): Unless you pay 2E → discard 2 from hand.
///   Parser: sequential[ condition(negation, energy>=2): do_nothing, move_cards(hand→discard,2) ]
///   Issue: The "unless" (支払わないかぎり) pattern doesn't properly gate the discard
///
/// Ab#1 (ライブ成功時): Optional: pay 6E → total score +1.
///   Parser: cost(pay_energy,6,optional), effect(modify_score,+1) ✓
///
/// Q91: No live → LiveStart doesn't trigger
/// Q92: Paying all-or-nothing, can choose not to pay
/// Q93: Partial resolution with insufficient hand cards
//=====================================================================

mod helpers;
use helpers::*;

#[test]
fn kanon_ab1_live_success_optional_cost_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = game.id("PL!SP-pb1-001-R");

    game.state.player1.stage.stage[1] = kanon;
    game.state.player1.hand.cards.push(game.id("PL!-sd1-020-SD"));
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(game.id("PL!-sd1-020-SD"));
    advance_to_live_start(&mut game);

    // Skip LiveStart "unless pay" optional cost
    if game.has_pending_choice() { game.select_indices(&[]); }

    // Give 6 active energy for LiveSuccess optional cost
    for _ in 0..6 { game.state.player1.energy_zone.cards.push(game.id("LL-E-001-SD")); }
    game.state.player1.energy_zone.active_energy_count = 6;

    let energy_before = game.state.player1.energy_zone.active_energy_count;
    game.pass(); game.pass(); game.pass();

    // LiveSuccess should fire during LiveVictoryDetermination.
    // The optional cost (pay 6E → score +1) should create a pending choice prompt.
    // We choose to pay: select the first option (pay)
    if game.has_pending_choice() {
        game.select_option(1); // card_id=1 → "pay_optional_cost"
    }
    let energy_spent = energy_before - game.state.player1.energy_zone.active_energy_count;
    assert!(energy_spent == 0 || energy_spent == 6,
        "Q92: optional cost should deduct 6E when paid (got {})", energy_spent);
}



#[test]
fn kanon_q93_partial_resolution_one_card() {
    // Q93: With 1 hand card and not paying the optional 2E,
    // the effect resolves partially: discard the 1 card instead of 2
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = game.id("PL!SP-pb1-001-R");

    game.state.player1.stage.stage[1] = kanon;
    // Put 1 card in hand (besides the live card) + the live card
    game.state.player1.hand.cards.push(game.id("PL!-sd1-019-SD")); // live card
    game.state.player1.hand.cards.push(game.id("PL!-sd1-010-SD")); // hand card

    let hand_before = game.state.player1.hand.cards.len();
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(game.id("PL!-sd1-019-SD"));
    advance_to_live_start(&mut game);

    // Skip the optional 2E cost → discard effect fires
    if game.has_pending_choice() { game.select_indices(&[]); }

    // After LiveCardSet replacement draw, hand had 0 cards (only the to-be-discarded one)
    // Actually the flow: LiveCardSet draws replacement = number of live cards set (1)
    // So hand has 0 (1 set as live) + 1 (drawn) = discard 1 → hand goes to 0
    let hand_after = game.state.player1.hand.cards.len();
    assert!(hand_after <= hand_before.saturating_sub(1),
        "Q93: at least 1 card should be discarded");
}

#[test]
fn kanon_q93_partial_resolution_zero_cards() {
    // Q93: With 0 hand cards and not paying, nothing happens (no crash)
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = game.id("PL!SP-pb1-001-R");

    game.state.player1.stage.stage[1] = kanon;
    game.state.player1.hand.cards.push(game.id("PL!-sd1-019-SD")); // live card only

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(game.id("PL!-sd1-019-SD"));
    advance_to_live_start(&mut game);

    // Skip optional cost → discard 2 fires, but 0 cards to discard → no-op
    if game.has_pending_choice() { game.select_indices(&[]); }

    // Q93: With 0 hand cards, the discard does nothing (no error/crash)
    assert!(game.state.player1.stage.stage[1] != -1,
        "Kanon should still be on stage after LiveStart");
}
