use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::game_setup::ActionType;

fn assert_live_success_reorder(order: [usize; 3]) {
    let mut game = TestGame::new(load_real_database());
    let nozomi = game.id("PL!-bp6-016-N");
    assert_eq!(
        game.state.card_database.get_card(nozomi).unwrap().card_no.as_ref(),
        "PL!-bp6-016-N"
    );
    let cards = [
        game.new_id("PL!-sd1-010-SD"),
        game.new_id("PL!S-sd1-001-SD"),
        game.new_id("PL!N-sd1-025-SD"),
        game.new_id("PL!-sd1-010-SD"),
        game.new_id("PL!S-sd1-001-SD"),
    ];
    let opponent = game.new_id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards = cards.to_vec().into();
    game.state.player2.main_deck.cards = vec![opponent].into();
    game.state.player1.stage.stage[1] = nozomi;

    fire_trigger(&mut game, nozomi, AbilityTrigger::LiveSuccess, "ライブ成功時");

    let mut remaining = cards[..3].to_vec();
    for &desired in &order[..2] {
        match game.get_pending_choice() {
            Choice::SelectTarget { target, allow_skip, .. } => {
                assert_eq!(target, "order");
                assert!(!allow_skip, "Returning all looked-at cards is mandatory");
            }
            choice => panic!("Expected mandatory reorder options, got {choice:?}"),
        }
        let option = remaining.iter().position(|&id| id == cards[desired]).unwrap();
        let actions = game.generated_actions();
        assert_eq!(actions.len(), remaining.len());
        assert!(actions.iter().all(|action| action.action_type == ActionType::ChoiceDecision));
        let offered = actions.iter().position(|action| {
            action.parameters.as_ref().and_then(|params| params.card_id) == Some(option as i16)
        }).expect("The desired remaining card must have an offered order action");
        game.select_generated(offered);
        remaining.remove(option);
    }

    assert!(!game.has_pending_choice(), "The final remaining card needs no choice");
    assert_eq!(
        game.state.player1.main_deck.cards.as_slice(),
        &[cards[order[0]], cards[order[1]], cards[order[2]], cards[3], cards[4]],
        "The chosen top-three order and untouched deck suffix must both be exact"
    );
    assert!(game.state.looked_at_cards.is_empty());
    assert!(game.state.player1.hand.cards.is_empty());
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert_eq!(game.state.player1.stage.stage, [-1, nozomi, -1]);
    assert_eq!(game.state.player2.main_deck.cards.as_slice(), &[opponent]);
}

#[test]
fn live_success_look_three_supports_every_changed_order() {
    for order in [[0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]] {
        assert_live_success_reorder(order);
    }
}

#[test]
fn live_success_look_three_can_keep_original_order_without_skipping() {
    assert_live_success_reorder([0, 1, 2]);
}

/// A deck holding FEWER than three cards.
///
/// 「自分のデッキの上からカードを3枚見る。それらを好きな順番でデッキの上に置く」 — the 3 is
/// how many are looked at, not how many must exist. With a 2-card deck the effect must still
/// run: look at both, let the order be chosen, and return exactly those two cards to the top
/// with the deck neither gaining nor losing a card.
///
/// The all-permutations test above always stages 5, and `live_success_look_three_deck_size_q36_test`
/// stages 15 but drives a DIFFERENT card's live — so a short deck reaches no test in this file.
/// A deck of exactly 0 and of exactly 1 are the two degenerate ends; 0 must be a no-op and 1
/// must need no order decision because there is nothing to reorder.
#[test]
fn live_success_look_three_handles_a_deck_shorter_than_three() {
    for deck_len in [0usize, 1, 2] {
        let mut game = TestGame::new(load_real_database());
        let nozomi = game.id("PL!-bp6-016-N");
        game.assert_card_identity(nozomi, "PL!-bp6-016-N");

        let cards: Vec<i16> = (0..deck_len)
            .map(|i| game.id(["PL!-sd1-010-SD", "PL!S-sd1-001-SD", "PL!N-sd1-025-SD"][i]))
            .collect();
        game.state.player1.main_deck.cards = cards.clone().into();
        game.state.player1.stage.stage[1] = nozomi;

        fire_trigger(&mut game, nozomi, AbilityTrigger::LiveSuccess, "ライブ成功時");

        // Put the looked-at cards back on top in REVERSE order, so a no-op implementation
        // cannot pass: the deck must end reversed, not unchanged. Taking the LAST card first
        // reverses the block — deck_len=2 picks index 1, deck_len=3 picks 2 then 1, and
        // deck_len 0 or 1 need no decision because there is nothing to reorder.
        let mut remaining = cards.clone();
        for desired in (1..deck_len).rev() {
            // 0 or 1 cards need no decision at all.
            assert!(
                game.has_pending_choice(),
                "deck_len={deck_len}: with {deck_len} card(s) looked at, {} order decision(s) \
                 remain and each must be offered",
                deck_len.saturating_sub(1)
            );            let option = remaining
                .iter()
                .position(|&id| id == cards[desired])
                .expect("the desired card is among the remaining ones");
            let actions = game.generated_actions();
            let offered = actions
                .iter()
                .position(|a| {
                    a.action_type == ActionType::ChoiceDecision
                        && a.parameters.as_ref().and_then(|p| p.card_id) == Some(option as i16)
                })
                .unwrap_or_else(|| {
                    panic!("deck_len={deck_len}: card {option} must be offered for ordering")
                });
            game.select_generated(offered);
            remaining.remove(option);
        }

        assert!(
            !game.has_pending_choice(),
            "deck_len={deck_len}: every order decision must be answered"
        );

        let mut expected: Vec<i16> = cards.iter().rev().copied().collect();
        expected.extend_from_slice(&[]);
        assert_eq!(
            game.state.player1.main_deck.cards.as_slice(),
            expected.as_slice(),
            "deck_len={deck_len}: the looked-at cards return to the top in the chosen order, \
             and the deck must hold exactly the same cards it started with"
        );
        assert!(
            game.state.looked_at_cards.is_empty(),
            "deck_len={deck_len}: nothing may be left revealed once they are put back"
        );
    }
}
