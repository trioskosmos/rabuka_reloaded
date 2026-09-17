/// Tests for look_and_select with optional hand-discard cost (reveal-to-hand shape):
/// 手札を1枚控え室に置いてもよい：3枚見る → 1枚を手札に加え、残りを控え室に置く。
/// Branches: cost paid (take to hand, remainder incl. cost card to waitroom),
/// cost declined (no look at all).
///
/// Card: 絢瀬絵里 (PL!-sd1-011-SD)
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// 絢瀬 絵里 (PL!-sd1-011-SD) — debut look_and_select with optional
// hand-discard cost. Text: 登場 手札を1枚控え室に置いてもよい：
// 自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、
// 残りを控え室に置く。
// ====================================================================

/// Accepted-cost branch: exactly 3 cards leave the deck, one is taken to
/// hand, the other two land in the waitroom. Pinned by card identity, not
/// by counts.
#[test]
fn eli_cost_look_three_takes_one_to_hand() {
    let mut game = TestGame::new(load_real_database());
    let a = game.id("PL!-sd1-010-SD");
    let b = game.id("PL!-sd1-020-SD");
    let c = game.id("PL!-sd1-014-SD");
    let d = game.id("PL!-sd1-015-SD");
    game.state.player1.main_deck.cards = vec![a, b, c, d].into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let eli = game.id("PL!-sd1-011-SD");
    game.add_to_hand(eli);
    let cost = game.id("PL!-sd1-020-SD");
    game.add_to_hand(cost);
    game.give_energy(12);
    game.play_to_stage(eli, MemberArea::Center);
    // Cost prompt first: discard 1 from hand (only the cost card is in hand).
    game.assert_select_card("hand", 1, true);
    game.select_indices(&[0]);
    assert!(
        game.state.player1.stage.stage.contains(&eli),
        "eli stays on stage (only the cost card leaves)"
    );
    assert!(!game.state.player1.hand.cards.contains(&cost));
    // Look prompt over the top three.
    game.assert_select_card("looked_at", 1, false);
    assert_eq!(game.state.looked_at_cards.as_slice(), &[a, b, c]);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert!(
        game.state.player1.hand.cards.contains(&a),
        "selected card added to hand"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.as_slice(),
        &[cost, b, c],
        "cost card + unselected remainder hit the waitroom in order"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.as_slice(),
        &[d],
        "deck untouched below the looked-at three"
    );
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

/// Declined-cost branch: skipping 「てもよい」 means NO look at all — deck,
/// hand and waitroom stay untouched.
#[test]
fn eli_declined_cost_skips_look_entirely() {
    let mut game = TestGame::new(load_real_database());
    let cards: [i16; 4] = std::array::from_fn(|_| game.id("PL!-sd1-010-SD"));
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let eli = game.id("PL!-sd1-011-SD");
    game.add_to_hand(eli);
    let cost = game.id("PL!-sd1-020-SD");
    game.add_to_hand(cost);
    game.give_energy(12);
    game.play_to_stage(eli, MemberArea::Center);
    game.assert_select_card("hand", 1, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice(), "declined cost → effect never starts");
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[cost]);
    assert!(
        game.state.player1.stage.stage.contains(&eli),
        "eli stays on stage; nothing else moved"
    );
    assert!(game.state.player1.waitroom.cards.is_empty(), "declined cost discards nothing");
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &cards);
    assert!(game.state.looked_at_cards.is_empty());
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}
