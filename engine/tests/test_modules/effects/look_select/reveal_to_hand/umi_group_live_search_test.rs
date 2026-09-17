/// Tests for look_and_select with group filter (reveal-to-hand shape):
/// 5枚見る → 『μ's』のライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。
/// Branches: no-eligible auto-skip, eligible take, declined pick (残り still discarded).
///
/// Card: 園田海未 (PL!-sd1-004-SD)
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Debut look_and_select with group filter — 園田海未 (PL!-sd1-004-SD):
/// 「デッキの上から5枚見る。その中から『μ's』のライブカードを1枚公開して
///  手札に加えてもよい。残りを控え室に置く。」
/// No eligible μ's live card among the looked-at five → auto-skip (no prompt),
/// and ALL five go to the waitroom.
#[test]
fn look_and_select_no_eligible_cards_auto_skips() {
    let mut game = TestGame::new(load_real_database());
    let cards: [i16; 7] = std::array::from_fn(|_| game.id("PL!-sd1-010-SD"));
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let umi = game.id("PL!-sd1-004-SD");
    game.add_to_hand(umi);
    game.give_energy(11);
    game.play_to_stage(umi, MemberArea::Center);
    assert!(!game.has_pending_choice(), "no eligible card → auto-skip");
    assert_eq!(
        game.state.player1.waitroom.cards.as_slice(),
        &cards[..5],
        "all five looked-at non-matching cards go to the waitroom"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.as_slice(),
        &cards[5..],
        "deck untouched below the looked-at five"
    );
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

/// Same printed ability, eligible branch: a μ's live card among the five is
/// taken to hand (公開 = revealed selection), the rest to the waitroom.
#[test]
fn look_and_select_takes_group_live_card_to_hand() {
    let mut game = TestGame::new(load_real_database());
    let live = game.id("PL!-sd1-020-SD");
    let mut cards: Vec<i16> = (0..5).map(|_| game.id("PL!-sd1-010-SD")).collect();
    cards.insert(2, live);
    game.state.player1.main_deck.cards = cards.clone().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let umi = game.id("PL!-sd1-004-SD");
    game.add_to_hand(umi);
    game.give_energy(11);
    game.play_to_stage(umi, MemberArea::Center);
    game.assert_select_card("looked_at", 1, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &cards[..5]);
    // Frontend protocol: indices are positions WITHIN filtered_indices
    // (only the μ's live card passes the group filter → position 0).
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[live]);
    let mut wr = game.state.player1.waitroom.cards.to_vec();
    let mut expected = [&cards[..2], &cards[3..5]].concat();
    wr.sort_unstable();
    expected.sort_unstable();
    assert_eq!(wr, expected);
    assert_eq!(
        game.state.player1.main_deck.cards.as_slice(),
        &cards[5..],
        "deck untouched below the looked-at five"
    );
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

/// Declined branch of the same printed ability: skip the 「てもよい」 pick →
/// nothing returns, ALL five looked-at cards go to the waitroom (残りを控え室に置く
/// applies even on decline — the remainder directive is explicit).
#[test]
fn look_and_select_skip_still_discards_remainder_to_waitroom() {
    let mut game = TestGame::new(load_real_database());
    let live = game.id("PL!-sd1-020-SD");
    let mut cards: Vec<i16> = (0..5).map(|_| game.id("PL!-sd1-010-SD")).collect();
    cards.insert(2, live);
    game.state.player1.main_deck.cards = cards.clone().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let umi = game.id("PL!-sd1-004-SD");
    game.add_to_hand(umi);
    game.give_energy(11);
    game.play_to_stage(umi, MemberArea::Center);
    game.assert_select_card("looked_at", 1, true);
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert!(game.state.player1.hand.cards.is_empty());
    let mut wr = game.state.player1.waitroom.cards.to_vec();
    let mut expected = cards.clone();
    expected.truncate(5);
    wr.sort_unstable();
    expected.sort_unstable();
    assert_eq!(wr, expected, "declined pick still sends 残り to the waitroom");
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &cards[5..]);
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}
