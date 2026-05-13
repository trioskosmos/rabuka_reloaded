/// Tests for 天王寺璃奈 (PL!N-bp5-021-N) — Debut: discard top 2 from deck,
/// then place a live card from discard at 4th-from-top of deck.
///
/// Q226: When deck has 2 cards, "4th from top" clamps to bottom (index 2).
use crate::helpers::*;

/// Deck starts with 4 cards. Debut discards top 2 → deck=2.
/// Then places live card at "4th from top" which clamps to index 2 (bottom).
#[test]
fn rina_q226_position_clamps_to_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-bp5-021-N");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("LL-bp5-001-L"); // a live card for discard

    game.state.player1.main_deck.cards.clear();
    // 4 cards: [filler, filler, filler, filler]
    for _ in 0..4 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Put the live card in discard so the ability can find it
    game.state.player1.waitroom.cards.push(live);

    game.state.player1.hand.cards.push(rina);
    game.state.player1.hand.cards.push(filler);

    game.give_energy(4);
    game.play_to_stage(rina, rabuka_engine::zones::MemberArea::LeftSide);

    // Debut triggers. Cost: discard top 2 (auto: deck 4→2).
    // Then optional: place live card from discard at 4th-from-top.
    // Deck has 2 cards, position 4 clamps to 2 (bottom).
    if game.has_pending_choice() {
        game.select_option(0); // pay the optional cost
    }
    // Select the live card from discard
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Deck should have 3 cards: 1 (the live card placed at bottom) + 2 (remaining)
    // Actually: initial 4, discards top 2 → deck=2. Places live at 4th from top → clamped to index 2 → deck[2] = live
    // But deck has 2 cards (indices 0,1). Clamping to index 2 means at the end → deck=3
    let deck = &game.state.player1.main_deck.cards;
    eprintln!("[RINA] deck final: {:?}", deck);
    assert_eq!(deck.len(), 3, "Deck = 2 remaining + 1 placed = 3");
    // The last card should be the live card (placed at clamped bottom position)
    assert_eq!(deck[2], live, "Live card is at bottom of deck");
}
