use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn drain_skippable_choices(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { allow_skip: true, .. } => game.select_indices(&[]),
            _ => break,
        }
    }
}

/// SENTIMENTAL StepS ライブ (PL!-pb1-032-L) ab#0:
///   ライブ成功時 自分の成功ライブカード置き場に『μ's』のカードがある場合、
///   カードを1枚引く。
///
/// These tests are a DIFFERENTIAL: one fixture, one live, and the only
/// difference is whether a μ's card or a Hasunosora card is in the success live
/// zone. The outcome is the deck size across the ライブ成功時 window.
///
/// They used to assert only `!has_pending_choice()`, which is true whether the
/// ability fired or not — a draw is not a prompt. All three passed, and two of
/// their premises were not reachable: PL!-pb1-032-L carries no `unit`, so a
/// SUCCEEDING live of this very card does not supply the μ's card the condition
/// asks for, and neither does the empty zone.
fn setup() -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-pb1-032-L");
    let filler = game.id_ref("PL!-sd1-010-SD");
    // Three DISTINCT members that print heart01/heart03/heart06 so the live's
    // requirement (h01 + h03 + h06 + h0×2) is MET and the live succeeds.
    let m = game.new_id("PL!-sd1-001-SD");
    let m2 = game.new_id("PL!-sd1-001-SD");
    let m3 = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [m, m2, m3];
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(live);
    (game, live, filler)
}

/// Cards the deck loses across the ライブ成功時 window, counted from AFTER the
/// yell.
///
/// The baseline is taken at LiveVictoryDetermination, not before the live runs:
/// the yell discards the cards it did not reveal, so a count that starts at the
/// live card set is 9-10 cards of noise (measured) and swamps the 1 card the
/// ability moves. ライブ成功時 then resolves on the steps inside
/// LiveVictoryDetermination, and the live closes on the → Active transition.
fn deck_delta_across_live_success(game: &mut TestGame) -> i32 {
    game.advance_to_phase(rabuka_engine::game_state::Phase::LiveVictoryDetermination);
    drain_skippable_choices(game);
    let before = game.state.player1.main_deck.cards.len() as i32;
    game.advance_to_phase(rabuka_engine::game_state::Phase::Active);
    drain_skippable_choices(game);
    game.state.player1.main_deck.cards.len() as i32 - before
}

/// Does the group matcher the ability uses consider this card a μ's card?
fn is_mu(game: &TestGame, id: i16) -> bool {
    let Some(card) = game.db.get_card(id) else {
        return false;
    };
    card.unit
        .as_deref()
        .is_some_and(|u| u.contains('\u{03bc}'))
        || card.name.contains('\u{03bc}')
}

#[test]
fn success_zone_group_draw_live_draws_one_with_mu_card_in_success_zone() {
    let (mut game, live, filler) = setup();
    // A μ's card in the success zone is the whole condition. PL!-sd1-001-SD is
    // 高坂穂乃果, a μ's member, so this card is a legal occupant of the zone.
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    let delta = deck_delta_across_live_success(&mut game);

    assert!(
        is_mu(&game, filler),
        "precondition: the card in the success zone really is a μ's card"
    );
    assert!(
        game.state.player1.success_live_card_zone.cards.contains(&live),
        "precondition: this live SUCCEEDED, so ライブ成功時 resolved at all"
    );
    assert_eq!(
        delta, -1,
        "「カードを1枚引く」: exactly one card leaves the deck"
    );
    assert!(
        !game.has_pending_choice(),
        "the draw is not a choice the player answers"
    );
}

#[test]
fn success_zone_group_draw_live_draws_nothing_with_only_non_mu_card_in_success_zone() {
    let (mut game, live, _filler) = setup();
    // Same fixture, same live, same success: the zone holds a スリーズブケ
    // member instead. Identical to the positive twin in every respect but the
    // group's name, so "0 cards drawn" is attributable to the filter and not to
    // the live failing or the window being wrong.
    let hasu = game.id("PL!HS-bp1-001-R");
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(hasu);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    let delta = deck_delta_across_live_success(&mut game);

    assert!(
        game.state.player1.success_live_card_zone.cards.contains(&live),
        "precondition: this live SUCCEEDED — the negative must be the FILTER, \
         not a failed live"
    );
    assert!(
        !game.state
            .player1
            .success_live_card_zone
            .cards
            .iter()
            .any(|c| is_mu(&game, *c)),
        "precondition: no μ's card is in the success zone"
    );
    assert_eq!(
        delta, 0,
        "a zone holding only non-μ's cards leaves the deck untouched"
    );
    assert!(!game.has_pending_choice());
}

#[test]
fn success_zone_group_draw_live_draws_nothing_with_empty_success_zone() {
    let (mut game, live, _filler) = setup();
    game.state
        .player1
        .success_live_card_zone
        .cards
        .clear();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    let delta = deck_delta_across_live_success(&mut game);

    assert_eq!(
        delta, 0,
        "『μ's』のカードがない: nothing is drawn"
    );
    assert!(!game.has_pending_choice());
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.advance_to_phase(rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker);
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}
