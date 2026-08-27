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
    // Then a single optional prompt: place live card from discard (allow_skip).
    assert!(
        game.has_pending_choice(),
        "optional live-card placement prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the placement (zone=discard, allow_skip)"
    );
    game.select_indices(&[0]); // place the live card from the waitroom top

    // Placement is single-step; no further prompt may appear.
    assert!(
        !game.has_pending_choice(),
        "placement resolves in one step; no second prompt expected"
    );

    // Deck should have 3 cards: 1 (the live card placed at bottom) + 2 (remaining)
    // Actually: initial 4, discards top 2 → deck=2. Places live at 4th from top → clamped to index 2 → deck[2] = live
    // But deck has 2 cards (indices 0,1). Clamping to index 2 means at the end → deck=3
    let deck = &game.state.player1.main_deck.cards;
    eprintln!("[RINA] deck final: {:?}", deck);
    assert_eq!(deck.len(), 3, "Deck = 2 remaining + 1 placed = 3");
    // The last card should be the live card (placed at clamped bottom position)
    assert_eq!(deck[2], live, "Live card is at bottom of deck");
}

#[test]
fn rina_q226_skip_optional_placement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rina = game.id("PL!N-bp5-021-N");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("LL-bp5-001-L");
    game.state.player1.main_deck.cards.clear();
    for _ in 0..4 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.waitroom.cards.push(live);
    game.state.player1.hand.cards.push(rina);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);
    game.play_to_stage(rina, rabuka_engine::zones::MemberArea::Center);
    assert!(game.has_pending_choice(), "optional placement expected");
    // Skip the optional placement
    game.select_indices(&[]);
    assert!(!game.has_pending_choice(), "skipping should end with no pending");
    // Deck should have 2 cards (4 -2 discarded, no placement)
    assert_eq!(game.state.player1.main_deck.cards.len(), 2, "skipped placement keeps deck at 2");
    // Live should remain in discard
    assert!(game.state.player1.waitroom.cards.contains(&live), "live stays in discard when skipped");
}

#[test]
fn rina_q226_no_live_in_discard_no_prompt() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rina = game.id("PL!N-bp5-021-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    for _ in 0..4 { game.state.player1.main_deck.cards.push(filler); }
    // Waitroom has no live card
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.hand.cards.push(rina);
    game.give_energy(4);
    game.play_to_stage(rina, rabuka_engine::zones::MemberArea::Center);
    // With no live in discard, the optional SelectCard should either not appear or be skippable with 0 options
    // Drain any pending (if appears, skip)
    let mut safety = 0;
    while game.has_pending_choice() && safety < 5 {
        safety += 1;
        game.select_indices(&[]);
    }
    assert!(!game.has_pending_choice(), "no live in discard should end cleanly");
    assert_eq!(game.state.player1.main_deck.cards.len(), 2, "deck 4-2 =2");
}
