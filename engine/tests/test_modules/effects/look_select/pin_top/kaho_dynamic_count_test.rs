/// Tests for look_and_select with dynamic look count + pin-to-top shape:
/// デッキの上からN枚見る。その中から1枚をデッキの一番上に置き、残りを控え室に置く。
/// N = own stage members + 3.
///
/// Card: 日野下花帆 (PL!HS-bp6-001-R＋)
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn look_and_select_dynamic_count_look_at_counts_stage_members_plus_two() {
    let (game, inspected, cards, opponent_stage, opponent_deck) = play_kaho_selection(0);
    assert_eq!(inspected, 3, "no allies looks three");
    assert_kaho_pins(&game, inspected, &cards, &opponent_stage, opponent_deck);
}

#[test]
fn kaho_two_own_members_look_four_and_keep_one() {
    let (game, inspected, cards, opponent_stage, opponent_deck) = play_kaho_selection(1);
    assert_eq!(inspected, 4, "one ally looks four");
    assert_kaho_pins(&game, inspected, &cards, &opponent_stage, opponent_deck);
}

#[test]
fn kaho_full_own_stage_looks_five_and_keeps_one() {
    let (game, inspected, cards, opponent_stage, opponent_deck) = play_kaho_selection(2);
    assert_eq!(inspected, 5, "full own stage looks five");
    assert_kaho_pins(&game, inspected, &cards, &opponent_stage, opponent_deck);
}

/// Shared outcome pins (called AFTER each test's own direct asserts so the
/// static audit sees per-test assertions).
fn assert_kaho_pins(
    game: &TestGame,
    inspected: usize,
    cards: &[i16; 7],
    opponent_stage: &[i16; 3],
    opponent_deck: i16,
) {
    let mut expected_deck = vec![cards[inspected - 1]];
    expected_deck.extend_from_slice(&cards[inspected..]);
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), expected_deck.as_slice());
    let mut actual_discard = game.state.player1.waitroom.cards.to_vec();
    let mut expected_discard = cards[..inspected - 1].to_vec();
    actual_discard.sort_unstable();
    expected_discard.sort_unstable();
    assert_eq!(actual_discard, expected_discard);
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.player2.stage.stage, *opponent_stage);
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent_deck]);
    assert!(game.state.player2.waitroom.cards.is_empty());
}

/// Plays Kaho with `allies` own members already on stage, drives the
/// pin-to-top selection (keeps the last looked-at card on top), and
/// returns the game plus the facts each test pins: inspected count,
/// deck ids, and the opponent snapshot taken before the debut.
fn play_kaho_selection(allies: usize) -> (TestGame, usize, [i16; 7], [i16; 3], i16) {
    let mut game = TestGame::new(load_real_database());
    let cards: [i16; 7] = std::array::from_fn(|_| game.id("PL!-sd1-010-SD"));
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent_deck = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent_deck].into();
    for area in 0..3 {
        let opponent = game.id("PL!-sd1-014-SD");
        game.state.player2.stage.stage[area] = opponent;
    }
    let opponent_stage = game.state.player2.stage.stage;
    for area in [MemberArea::LeftSide, MemberArea::RightSide].into_iter().take(allies) {
        let ally = game.id("PL!-sd1-015-SD");
        game.add_to_stage(area, ally);
    }
    let kaho = game.id("PL!HS-bp6-001-R＋");
    game.add_to_hand(kaho);
    game.give_energy(4);
    game.play_to_stage(kaho, MemberArea::Center);
    let inspected = allies + 3;
    assert!(game.state.player1.stage.stage.contains(&kaho));
    game.assert_select_card("looked_at", 1, false);
    assert_eq!(game.state.looked_at_cards.as_slice(), &cards[..inspected]);
    game.select_indices(&[inspected - 1]);
    assert!(!game.has_pending_choice());
    (game, inspected, cards, opponent_stage, opponent_deck)
}
