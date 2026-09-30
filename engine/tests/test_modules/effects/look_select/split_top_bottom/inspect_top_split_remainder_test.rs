use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const MARI: &str = "PL!S-bp7-008-R";

fn debut() -> (TestGame, [i16; 5]) {
    let mut game = TestGame::new(load_real_database());
    let cards = std::array::from_fn(|_| game.id("PL!-sd1-010-SD"));
    game.state.player1.main_deck.cards = cards.to_vec().into();
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards = vec![opponent].into();
    let mari = game.id(MARI);
    game.add_to_hand(mari);
    game.give_energy(20);
    game.play_to_stage(mari, MemberArea::Center);
    assert!(game.state.player1.stage.stage.contains(&mari));
    assert!(!game.state.player1.hand.cards.contains(&mari));
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    assert_eq!(game.state.looked_at_cards.as_slice(), &cards[..3]);
    (game, cards)
}

#[test]
fn mari_keeps_one_on_top_rest_to_deck_bottom() {
    let (mut game, [a, b, c, d, e]) = debut();
    game.select_indices(&[1]);
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[b, d, e, a, c]);
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert!(game.state.player1.hand.cards.is_empty());
}

#[test]
fn mari_keeps_two_on_top_one_to_deck_bottom() {
    let (mut game, [a, b, c, d, e]) = debut();
    game.select_indices(&[0]);
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[0]);
    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    game.select_indices(&[]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.main_deck.cards.as_slice(), &[b, a, d, e, c]);
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert!(game.state.player1.hand.cards.is_empty());
}

/// 「好きな枚数」 includes ZERO, and only the two tests above exercise it — they pick 1 and 2.
///
/// The third outcome, and the one nothing covered. Declining the "好きな枚数" prompt does
/// **not** cancel the ability and does **not** offer an ordering choice for the remainder: the
/// engine treats zero-to-the-top as "no preference expressed" and applies
/// 残りを好きな順番でデッキの下に置く to all three at once, in the order they were looked at.
///
/// So the printed split still happens — nothing goes on top — but all three land underneath
/// the two cards that were never part of the look window. An implementation that abandoned the
/// ability on a decline (leaving the deck as `[d, e]` with the looked-at pool discarded), or
/// that asked for a bottom ordering it then ignored, fails here and passes both existing tests.
#[test]
fn mari_taking_none_to_the_top_sends_all_three_straight_to_the_bottom() {
    let (mut game, [a, b, c, d, e]) = debut();

    // Precondition, stated from what the look has actually done rather than assumed: the top
    // three are lifted OUT into the looked-at pool, so the deck holds only the two that were
    // never part of the choice.
    assert_eq!(
        game.state.looked_at_cards.as_slice(),
        &[a, b, c],
        "the top three are looked at"
    );
    let deck_during_ability: Vec<i16> = game.state.player1.main_deck.cards.to_vec();
    assert_eq!(
        deck_during_ability,
        vec![d, e],
        "the deck holds only the two cards below the look window"
    );

    // 好きな枚数 = 0: decline the "how many to the top" prompt.
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectCard".to_string()),
        "precondition: the skippable 好きな枚数 prompt is open"
    );
    game.select_indices(&[]);

    assert!(
        !game.has_pending_choice(),
        "declining 好きな枚数 applies 残り…デッキの下に置く to all three immediately — there is \
         no ordering prompt when nothing was placed on top"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.to_vec(),
        vec![d, e, a, b, c],
        "好きな枚数=0 — nothing to the top, the two cards below the look window stay on top, \
         and all three looked-at cards go underneath in the order they were looked at"
    );
    assert!(
        game.state.player1.waitroom.cards.is_empty(),
        "nothing is banked (waitroom={:?})",
        game.state.player1.waitroom.cards
    );
    assert!(game.state.player1.hand.cards.is_empty());
    assert!(
        game.state.looked_at_cards.is_empty(),
        "and the looked-at pool is emptied when the ability ends"
    );
}
