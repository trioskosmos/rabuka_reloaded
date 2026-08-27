use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

#[test]
fn looked_at_discard_player_discards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!HS-cl1-001-CL");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = card;
    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..6 {
        game.pass();
    }
    game.state
        .player1
        .main_deck
        .cards
        .insert(0, game.new_id("PL!-sd1-010-SD"));
    game.pass();

    assert!(game.has_pending_choice());
    game.assert_select_card("looked_at", 1, true);

    let looked_at_id = game.state.looked_at_cards[0];
    game.select_indices(&[0]);
    game.drain_auto_ability_choices();

    assert!(game.state.player1.waitroom.cards.contains(&looked_at_id));
    assert!(game.state.player1.main_deck.cards.first().copied() != Some(looked_at_id));
}

#[test]
fn looked_at_discard_player_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!HS-cl1-001-CL");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = card;
    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..6 {
        game.pass();
    }
    game.state
        .player1
        .main_deck
        .cards
        .insert(0, game.new_id("PL!-sd1-010-SD"));
    game.pass();

    assert!(game.has_pending_choice());
    game.assert_select_card("looked_at", 1, true);

    let looked_at_id = game.state.looked_at_cards[0];
    game.select_indices(&[]);
    game.drain_auto_ability_choices();

    assert!(game.state.player1.main_deck.cards.first().copied() == Some(looked_at_id));
    assert!(!game.state.player1.waitroom.cards.contains(&looked_at_id));
}

/// Card PL!HS-pb1-027-L (ユメワズライ):
///   ライブ成功時：自分のステージに『スリーズブーケ』のメンバーがいる場合、
///   自分のデッキの上からカードを4枚控え室に置いてもよい。
///
/// Verifies: group_names does NOT leak from condition onto action.
/// The discard has no group filter (deck_top is an independent source).
#[test]
fn live_success_discard_4_from_deck_unconditionally() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let yumewazurai = game.id("PL!HS-pb1-027-L");
    let member = game.id("PL!HS-PR-007-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = member;
    game.state.player1.stage.stage[2] = member;
    game.state.player1.stage.stage[0] = member;
    game.state.player1.hand.cards.push(yumewazurai);
    game.state.player1.hand.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Follow exactly the awake_test phase pattern
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(yumewazurai);
    advance_to_live_start(&mut game);
    game.pass(); // Performance
    let deck_before = game.state.player1.main_deck.cards.len();
    let wr_before = game.state.player1.waitroom.cards.len();
    game.pass(); // SecondAttackerPerformance → LiveVictoryDetermination
    game.pass(); // LiveVictoryDetermination processing (LiveSuccess fires, choice created)

    // Answer "Yes" to the optional discard before advancing further
    if let Some(rabuka_engine::ability::types::Choice::SelectTarget { ref target, .. }) =
        game.state.get_pending_choice()
    {
        if target == "pay_optional_cost:skip_optional_cost" {
            rabuka_engine::turn::TurnEngine::resume_with_choice(&mut game.state, Some(1), None)
                .expect("accept optional discard");
        }
    }
    game.pass(); // post-LiveVictory

    // Exactly 4 cards discarded from deck top to waitroom
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 4,
        "Deck lost exactly 4 cards (was {}, now {})",
        deck_before,
        game.state.player1.main_deck.cards.len()
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        wr_before + 4,
        "Waitroom gained exactly 4 cards (was {}, now {})",
        wr_before,
        game.state.player1.waitroom.cards.len()
    );
}
