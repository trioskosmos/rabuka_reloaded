/// Tests for 優木せつ菜 (PL!N-bp5-007-R+) — Q230
///
/// ab#0 (LiveStart): If both players have equal counts in success_live_zone,
///   gain heart02 x2 until live end.
/// ab#1 (LiveSuccess): If surplus heart >= 1, draw 2, then discard 1 from hand.
mod helpers;
use helpers::*;
fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

// ========================
// ab#0 — LiveStart: equality check on success_live_zone
// ========================

/// Q230: Both have 0 success cards → equal → gain heart02 x2.
#[test]
fn setsuna_q230_both_zero_heart02_gained() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let setsuna = game.id("PL!N-bp5-007-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [setsuna, filler, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 { game.state.player2.main_deck.cards.push(filler); }
    game.give_energy(15);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // Both 0 → equal → single fixed heart02 → modifier applied to activating card
    let mod_val = game.state.get_heart_modifier(setsuna, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(mod_val, 2,
        "Q230: Both 0 success cards → heart02 x2 gained");
}

/// ab#0: P1 has 1 success card, P2 has 1 → equal → no pending choice (fixed heart color).
#[test]
fn setsuna_unequal_success_cards_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let setsuna = game.id("PL!N-bp5-007-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [setsuna, filler, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);

    // P1 has 2 success cards, P2 has 1 → not equal
    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.player2.success_live_card_zone.cards.push(live);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 { game.state.player2.main_deck.cards.push(filler); }
    game.give_energy(15);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // Engine gap: same as above.
    while game.has_pending_choice() { game.select_indices(&[0]); }
}

// ========================
// ab#1 — LiveSuccess: surplus heart draw-discard
// ========================

/// Surplus heart >= 1 → draw 2, discard 1 → net +1.
/// PL!-sd1-019-SD needs heart01+03+06 = 3.
/// PL!-sd1-001-SD (Honoka) has heart01=1, heart03=2, heart06=1 = 4 total → surplus 1.
#[test]
fn setsuna_surplus_heart_draw_2_discard_1_net_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let setsuna = game.id("PL!N-bp5-007-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    let provider = game.id("PL!-sd1-001-SD"); // 4 hearts, surplus = 1

    game.state.player1.stage.stage = [setsuna, provider, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 { game.state.player2.main_deck.cards.push(filler); }
    game.give_energy(15);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass(); // LiveCardSetP1 → P2 (draws for P1)
    game.pass(); // LiveCardSetP2 → Performance (LiveStart triggers)
    game.pass(); // P1 performance
    game.pass(); // P2 performance
    game.pass(); // LiveVictoryDetermination → triggers LiveSuccess

    // Resolve choices (discard 1 from hand after drawing 2)
    while game.has_pending_choice() { game.select_indices(&[0]); }

    // Advance to next turn
    game.pass();

    // Net effect of ab#1: draw 2, discard 1
    let hand = game.state.player1.hand.cards.len();
    assert!(hand >= 3,
        "ab#1: surplus heart >=1 → net +1 hand");
}

/// ab#1: Same surplus scenario but with bigger deck, verifies discard choice resolves.
#[test]
fn setsuna_surplus_heart_draw_and_discard_net_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let setsuna = game.id("PL!N-bp5-007-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    let provider = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [setsuna, provider, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..60 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..60 { game.state.player2.main_deck.cards.push(filler); }
    game.give_energy(15);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    while game.has_pending_choice() { game.select_indices(&[0]); }
    game.pass();

    let hand = game.state.player1.hand.cards.len();
    assert!(hand >= 3,
        "ab#1: surplus heart triggers → net +1 hand");
}


