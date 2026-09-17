/// Tests for look_and_select with any_number=true (reorder-to-top shape):
/// 見るN枚 → 好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。
///
/// Card: 百生 吟子 (PL!HS-bp2-016-N) ab#0
/// Text: 登場 自分のデッキの上からカードを2枚見る。
///       その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。
///
/// Bug: any_number=true ended after first selection instead of batch selecting
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn debut_ginko() -> (TestGame, [i16; 4]) {
    let mut game = TestGame::new(load_real_database());
    let cards = std::array::from_fn(|_| game.id("PL!-sd1-010-SD"));
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent_card = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent_card].into();
    let ginko = game.id("PL!HS-bp2-016-N");
    game.add_to_hand(ginko);
    game.give_energy(4);
    game.play_to_stage(ginko, MemberArea::Center);
    assert!(game.state.player1.stage.stage.contains(&ginko));
    assert!(game.state.player1.hand.cards.is_empty());
    game.assert_select_card("looked_at", 2, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &cards[..2]);
    (game, cards)
}
#[test]
fn look_and_select_any_number_partial_selection() {
    let (mut game, [a, b, c, d]) = debut_ginko();
    game.select_indices(&[1]);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[b, c, d]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a]);
    assert!(game.state.player1.hand.cards.is_empty());
}

/// Select 2 out of 2 looked-at cards (full batch) with any_number=true.
/// Both go to deck top, none to discard.
#[test]
fn look_and_select_any_number_full_selection() {
    let (mut game, [a, b, c, d]) = debut_ginko();
    game.select_indices(&[0]);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[b, a, c, d]);
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert!(game.state.player1.hand.cards.is_empty());
}

/// Select 0 out of 2 looked-at cards with any_number=true.
/// Both go to discard.
#[test]
fn look_and_select_any_number_skip_all() {
    let (mut game, [a, b, c, d]) = debut_ginko();
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[c, d]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[a, b]);
    assert!(game.state.player1.hand.cards.is_empty());
}

#[test]
fn ginko_inspection_refresh_preserves_existing_top_and_recovers_waitroom() {
    let mut game = TestGame::new(load_real_database());
    let top = game.id("PL!-sd1-010-SD");
    let recycled = game.id("PL!-sd1-010-SD");
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards = vec![top].into();
    game.add_to_discard(recycled);
    game.state.player2.main_deck.cards = vec![opponent].into();
    let ginko = game.id("PL!HS-bp2-016-N");
    game.add_to_hand(ginko);
    game.give_energy(4);
    game.play_to_stage(ginko, MemberArea::Center);
    game.assert_select_card("looked_at", 2, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &[top, recycled]);
    assert!(game.state.player1.waitroom.cards.is_empty());
    game.select_indices(&[0]);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[top]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[recycled]);
    assert!(game.state.player1.hand.cards.is_empty());
    assert!(game.state.player1.stage.stage.contains(&ginko));
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

#[test]
fn ginko_inspecting_exact_deck_size_does_not_refresh_waitroom() {
    let mut game = TestGame::new(load_real_database());
    let a = game.id("PL!-sd1-010-SD");
    let b = game.id("PL!-sd1-010-SD");
    let waiting = game.id("PL!-sd1-010-SD");
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards = vec![a, b].into();
    game.add_to_discard(waiting);
    game.state.player2.main_deck.cards = vec![opponent].into();
    let ginko = game.id("PL!HS-bp2-016-N");
    game.add_to_hand(ginko);
    game.give_energy(4);
    game.play_to_stage(ginko, MemberArea::Center);
    game.assert_select_card("looked_at", 2, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &[a, b]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[waiting]);
    game.select_indices(&[1]);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[a, b]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[waiting]);
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}
