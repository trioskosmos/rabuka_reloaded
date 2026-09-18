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
fn live_start_optional_bottom_mill_removes_bottom_card_from_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let mari = game.id("PL!S-bp7-008-R");
    game.state.player1.stage.stage = [-1, mari, -1];

    game.give_energy(3);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);
    advance_to_live_card_set_p1(&mut game);
    // Seed the deck AFTER the live-card-set passes (which draw from the top),
    // so the top/bottom markers survive intact until the live-start trigger.
    let (_top, bottoms) = seed_deck_with_bottom(&mut game);
    let bottom_most = bottoms[0];
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // Mari ab#1: デッキの一番下のカードを控え室に置いてもよい → optional discard.
    // The optional-cost choice is ["No", "Yes"] — accept with option 1.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 4 {
        game.select_option(1);
        guard += 1;
    }

    // CLEAN-G1 claim: the BOTTOM card must have left the deck (source=deck_bottom,
    // not hand/top). Note: mari's follow-up (CLEAN-G5, a separate defect) may move
    // the discarded card to hand, and live-start draws consume the deck top, so
    // assert only that the bottom-most card is gone from the deck.
    let deck: Vec<i16> = game.state.player1.main_deck.cards.iter().copied().collect();
    assert!(
        !deck.contains(&bottom_most),
        "the bottom-most card must have left the deck, deck={:?}",
        deck
    );
}
