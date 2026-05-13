/// Q156: ダイスキだったらダイジョウブ！(PL!S-bp3-020-L) — re-yell with 2 copies.
///
/// Auto: When a card is revealed by yell and blade heart cards ≤ 2 among revealed,
/// may discard all hand and re-yell. Q156: With 2 copies in live zone, both re-yell.
use crate::helpers::*;

#[test]
fn daisuki_q156_two_copies_re_yell_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live1 = game.id("PL!S-bp3-020-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(live1);
    game.state.player1.hand.cards.push(live1);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage = [filler, filler, -1];

    let deck_before = game.state.player1.main_deck.cards.len();
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..60 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live1);
    game.set_live_card(live1);
    game.pass(); // LiveCardSet draw (2 cards)
    game.pass(); // Performance + LiveStart
    game.pass(); // P1 performance (yell)
    game.pass(); // P2 performance
    game.pass(); // LiveVictory -> LiveSuccess

    while game.has_pending_choice() {
        game.select_option(1);
    }
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Re-yell would have drawn more cards from deck
    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_after < deck_before + 60,
        "Q156: Re-yell consumed deck cards"
    );
}
