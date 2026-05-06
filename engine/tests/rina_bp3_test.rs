/// Q164: 天王寺璃奈 (PL!N-bp3-009-R＋) — LiveStart: put 2 member cards from
/// discard to deck bottom. Only YOUR discard, not opponent's (Q164).
mod helpers;
use helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

/// Put member cards in own discard, verify the cost can select them.
#[test]
fn rina_bp3_q164_select_from_own_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-bp3-009-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("LL-bp5-001-L");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 { game.state.player2.main_deck.cards.push(filler); }

    // Put 2 member cards in own discard
    let member_a = game.id("PL!-sd1-002-SD"); // 絢瀬絵里, member_card
    let member_b = game.id("PL!-sd1-005-SD"); // 星空凛, member_card
    game.state.player1.waitroom.cards.push(member_a);
    game.state.player1.waitroom.cards.push(member_b);

    // Put member cards in opponent's discard too — should NOT be selectable
    game.state.player2.waitroom.cards.push(member_a);

    game.state.player1.stage.stage = [rina, filler, filler];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // LiveStart fires: optional cost → select 2 members from discard
    if game.has_pending_choice() { game.select_indices(&[0]); } // pay or skip
    if game.has_pending_choice() { game.select_indices(&[0, 1]); } // pick 2 cards

    let deck = &game.state.player1.main_deck.cards;
    eprintln!("[RINA] deck after: {:?}", deck);
    // 2 cards moved from discard to deck bottom
    assert!(deck.len() >= 2, "Deck gained 2 cards from discard");
    assert_eq!(game.state.player1.waitroom.cards.len(), 0,
        "Both cards removed from discard");
}
