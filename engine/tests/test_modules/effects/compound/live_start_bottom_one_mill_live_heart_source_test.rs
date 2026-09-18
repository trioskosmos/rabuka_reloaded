use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn seed_deck_with_bottom(game: &mut TestGame) -> (i16, Vec<i16>) {
    let filler = game.id("PL!-sd1-010-SD");
    let a = game.id("PL!S-bp7-006-R"); // 津島善子 (top marker)
    let b1 = game.id("PL!SP-sd1-001-SD"); // 澁谷かのん
    let b2 = game.id("PL!SP-sd1-003-SD"); // 嵐千砂都
    let b3 = game.id("PL!SP-sd1-004-SD"); // 平安名すみれ
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(a); // index 0 = top
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.main_deck.cards.push(b1);
    game.state.player1.main_deck.cards.push(b2);
    game.state.player1.main_deck.cards.push(b3); // last = bottom
    (a, vec![b3, b2, b1])
}

#[test]
fn live_start_mills_bottom_one_preserving_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let yoshiko = game.id("PL!S-bp7-015-N");
    game.state.player1.stage.stage = [-1, yoshiko, -1];
    // Put a card in hand to prove source is NOT the hand.
    let hand_card = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(hand_card);

    game.give_energy(3);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    let (_top, bottoms) = seed_deck_with_bottom(&mut game);
    let bottom_most = bottoms[0];
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // Only the single bottom card moved.
    let p1_waitroom: Vec<i16> = game.state.player1.waitroom.cards.iter().copied().collect();
    assert_eq!(
        p1_waitroom.len(),
        1,
        "exactly 1 card discarded, got {}",
        p1_waitroom.len()
    );
    assert_eq!(
        p1_waitroom.first(),
        Some(&bottom_most),
        "the bottom-most card (last in deck) must be discarded, discard={:?}",
        p1_waitroom
    );
    // Hand card untouched.
    assert!(
        game.state.player1.hand.cards.contains(&hand_card),
        "the hand card must NOT be discarded"
    );
}
