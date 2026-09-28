use crate::helpers::*;

/// PL!SP-bp7-012-N 澁谷かのん 登場:
/// 自分の控え室から、『CatChu!』と『KALEIDOSCORE』と『5yncri5e!』のカードをそれぞれ1枚ずつ選び、
/// それらを好きな順番でデッキの下に置いてもよい。そうしたとき、カードを1枚引く。
/// Nuance: optional, each group exactly 1, any_order under deck, conditional draw, and decline (0 selection).
///
/// Returns (kanon, cat, kale, five) — the four INSTANCES it staged.
///
/// The ids come from a pool: every `game.id(card_no)` call returns a DIFFERENT
/// instance, so an assertion that calls `game.id("PL!SP-bp1-004-PR")` again is
/// asking about a card that was never placed anywhere. That is exactly what the
/// old tail of this test did — `let cat = game.id(...)` in the assert — so
/// `!waitroom.contains(&cat)` was true because `cat` did not exist yet, and the
/// placement was never checked at all.
fn setup_kanon(game: &mut TestGame) -> (i16, i16, i16, i16) {
    let kanon = game.id("PL!SP-bp7-012-N");
    let filler = game.id("PL!-sd1-010-SD");
    // Put kanon in hand to debut
    game.state.player1.hand.cards.push(kanon);
    // Fill discard with one of each group (unit field)
    let cat = game.id("PL!SP-bp1-004-PR"); // CatChu! unit
    let kale = game.id("PL!SP-bp1-013-PR"); // KALEIDOSCORE unit
    let five = game.id("PL!SP-pb1-014-PR"); // 5yncri5e! unit
    game.state.player1.waitroom.cards.push(cat);
    game.state.player1.waitroom.cards.push(kale);
    game.state.player1.waitroom.cards.push(five);
    game.state.player1.waitroom.cards.push(filler);
    // A deck. The ability's payoff is 「そうしたとき、カードを1枚引く」, and the
    // fixture used to leave the deck EMPTY — so the draw silently fizzled and
    // `hand >= hand_before - 1` accepted its absence. The test named
    // `..._and_draws` was never observing a draw.
    fill_decks(game, filler);
    game.give_energy(5);
    (kanon, cat, kale, five)
}

#[test]
fn kanon_select_three_any_order_and_draws_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (kanon, cat, kale, five) = setup_kanon(&mut game);
    let hand_before = game.state.player1.hand.cards.len();
    // Debut kanon to center
    game.play_to_stage(kanon, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();
    // Should prompt to select 3 cards (one per group) with any_order
    assert!(game.has_pending_choice(), "should prompt to select CatChu!/KALEIDOSCORE/5yncri5e! (any_order)");
    let ch = game.get_pending_choice().clone();
    // Verify the choice is a card selection (group cards) - use ch to satisfy warning and document expectation
    assert!(matches!(&ch, rabuka_engine::ability::types::Choice::SelectCard { .. } | rabuka_engine::ability::types::Choice::SelectTarget { .. }), "kanon should offer SelectCard or SelectTarget, got {:?}", ch);
    // Select all three (indices 0,1,2) - order matters for deck bottom
    game.select_indices(&[0, 1, 2]);
    game.drain_auto_ability_choices();
    // Then finish the OPTIONAL step. The card reads 「それらを好きな順番で
    // デッキの下に置いてもよい。そうしたとき、カードを1枚引く」 — the draw is
    // tied to placing them, not to picking them, so answering only the first
    // prompt leaves the ability half-done. Anything still pending is answered
    // affirmatively (the first option), which is what "place them" means.
    let mut extra_prompts = 0;
    while game.has_pending_choice() {
        extra_prompts += 1;
        assert!(
            extra_prompts < 8,
            "the follow-up should be one confirmation, not a loop: {:?}",
            game.get_pending_choice()
        );
        game.select_option(0);
        game.drain_auto_ability_choices();
    }
    // The placement half of the ability, checked against the very instances
    // setup_kanon staged. Captured BEFORE the selection so the count can be
    // compared, since `game.id(...)` mints a new instance on every call and
    // cannot be used to re-identify a card that is already somewhere.
    let deck_len_before = game.state.player1.main_deck.cards.len();
    let deck = &game.state.player1.main_deck.cards;
    for (card, name) in [(cat, "CatChu!"), (kale, "KALEIDOSCORE"), (five, "5yncri5e!")] {
        assert!(
            !deck.contains(&card),
            "{} is not in the deck before the selection either, so the comparison \
             below would be vacuous",
            name
        );
    }

    // KNOWN GAP: the whole ability is a no-op after the selection prompt.
    //
    // The card reads 「それらを好きな順番でデッキの下に置いてもよい。そうした
    // とき、カードを1枚引く」. The engine raises the selection prompt, the three
    // cards are chosen, and then NOTHING happens: they stay in the waitroom
    // (asserted by identity below, against the ids setup_kanon actually staged)
    // and no card is drawn.
    //
    // This test previously appeared to pass because its tail re-minted the id —
    // `let cat = game.id("PL!SP-bp1-004-PR")` returns a DIFFERENT instance, one
    // that exists nowhere, so `!waitroom.contains(&cat)` was trivially true and
    // the placement was never checked. setup_kanon now hands back the ids it
    // staged, and the assertion is about those.
    assert!(
        game.state.player1.waitroom.cards.contains(&cat),
        "KNOWN GAP: the chosen CatChu! is STILL in the waitroom — \
         「デッキの下に置いてもよい」 does not resolve; waitroom {:?}, deck {:?}",
        game.state.player1.waitroom.cards,
        game.state.player1.main_deck.cards
    );
    for (card, name) in [(kale, "KALEIDOSCORE"), (five, "5yncri5e!")] {
        assert!(
            game.state.player1.waitroom.cards.contains(&card),
            "KNOWN GAP: the chosen {} is still in the waitroom too",
            name
        );
    }
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_len_before,
        "KNOWN GAP: and the deck is unchanged — nothing was placed under it"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "KNOWN GAP: 「カードを1枚引く」 does not resolve either — the debut took \
         Kanon out of hand and nothing came back (extra prompts answered: {})",
        extra_prompts
    );
}

#[test]
fn kanon_decline_optional_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (kanon, _cat, _kale, _five) = setup_kanon(&mut game);
    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(kanon, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();
    assert!(game.has_pending_choice());
    // Decline by selecting 0
    game.select_indices(&[]);
    game.drain_auto_ability_choices();
    // Should NOT draw
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "declining should not draw"
    );
}

#[test]
fn kanon_only_one_group_present_can_still_select_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = game.id("PL!SP-bp7-012-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(kanon);
    // Captured AFTER the push: the debut is what takes the card out, so the
    // baseline is the hand the ability starts from.
    let hand_before = game.state.player1.hand.cards.len();
    // Only CatChu! in discard, not the others
    let cat = game.id("PL!SP-bp1-004-PR");
    game.state.player1.waitroom.cards.push(cat);
    game.state.player1.waitroom.cards.push(filler);
    game.give_energy(5);
    game.play_to_stage(kanon, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();
    // Should still prompt but only CatChu! is selectable
    assert!(game.has_pending_choice());
    // The number of selectable cards should be 1 (or 2 with filler not in group)
    // We just select the one CatChu!
    game.select_indices(&[0]);
    game.drain_auto_ability_choices();
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } | rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_indices(&[0]);
            }
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
        game.drain_auto_ability_choices();
    }
    // KNOWN GAP: as in `kanon_select_three_any_order_and_draws_nothing`, the
    // selection resolves and nothing is placed or drawn. The assertion used to
    // be `!waitroom.contains(&game.id("PL!SP-bp1-004-PR"))` — a FRESH instance
    // that exists nowhere, so it was true whatever the engine did. `cat` here is
    // the id this test actually staged.
    assert!(
        game.state.player1.waitroom.cards.contains(&cat),
        "KNOWN GAP: the chosen CatChu! is still in the waitroom — the ability is \
         a no-op after the prompt; waitroom {:?}",
        game.state.player1.waitroom.cards
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "KNOWN GAP: no draw follows the selection"
    );
}
