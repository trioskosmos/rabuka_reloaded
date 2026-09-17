use crate::helpers::*;

fn assert_live_success_reveal_score(revealed_print: &str, expected_bonus: u32) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let source = game.id("PL!-bp6-007-R+");
    let revealed = game.id(revealed_print);
    let unrelated_member = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, source, -1];
    game.state.player1.hand.cards.extend([live, unrelated_member]);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.main_deck.cards.insert(5, revealed);

    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.set_live_card(live);
    for _ in 0..5 {
        game.pass();
        assert!(!game.has_pending_choice(), "reveal-to-hand has no choice");
    }

    assert!(game.state.player1.success_live_card_zone.cards.contains(&live));
    assert!(game.state.player1.hand.cards.contains(&revealed));
    assert!(game.state.player1.hand.cards.contains(&unrelated_member));
    assert!(!game.state.player1.main_deck.cards.contains(&revealed));
    assert_eq!(game.state.player1.main_deck.cards.first(), Some(&filler));
    let snapshot = game
        .state
        .performance_snapshots
        .iter()
        .find(|snapshot| snapshot.player_id == "p1")
        .unwrap();
    assert!(snapshot.success);
    assert_eq!(snapshot.lives.len(), 1);
    assert_eq!(snapshot.total_score as u32, 2 + expected_bonus);
}

#[test]
fn live_success_reveals_member_without_blade_heart_to_hand_and_adds_total_score() {
    assert_live_success_reveal_score("PL!-sd1-001-SD", 1);
}

#[test]
fn live_success_reveals_blade_heart_member_without_score_despite_qualifying_hand_member() {
    assert_live_success_reveal_score("PL!-sd1-010-SD", 0);
}

#[test]
fn live_success_reveals_live_without_blade_heart_without_score_despite_qualifying_hand_member() {
    assert_live_success_reveal_score("PL!-sd1-019-SD", 0);
}

#[test]
fn live_success_revealed_total_score_applies_once_with_two_set_lives() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let source = game.id("PL!-bp6-007-R+");
    let stage_member_a = game.id("PL!-sd1-001-SD"); // h01=1 h03=2 h06=1, no blade-heart
    let stage_member_b = game.id("PL!-sd1-001-SD"); // second copy
    let unrelated_member = game.id("PL!-sd1-001-SD");
    let live_a = game.id("PL!-sd1-020-SD"); // score 2
    let live_b = game.id("PL!-sd1-021-SD"); // score 3
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [stage_member_a, source, stage_member_b];
    game.state
        .player1
        .hand
        .cards
        .extend([live_a, live_b, unrelated_member]);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    // Every deck card is the qualifying blade-heart-less member, so whatever
    // the LiveSuccess reveals after the draw/yell window is always a match.
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(stage_member_a);
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.set_live_card(live_a);
    game.set_live_card(live_b);
    for _ in 0..5 {
        game.pass();
        while let Some(choice) = game.pending_choice_type() {
            assert!(
                choice == "SelectLiveSuccess" || choice == "SelectCard",
                "unexpected prompt {} in two-live flow",
                choice
            );
            game.select_indices(&[0]);
        }
    }

    assert!(game.state.player1.hand.cards.len() >= 4);
    let snapshot = game
        .state
        .performance_snapshots
        .iter()
        .find(|snapshot| snapshot.player_id == "p1")
        .unwrap();
    assert_eq!(snapshot.lives.len(), 2);
    assert!(
        snapshot.lives.iter().all(|live| live.passed),
        "both lives must pass so the print's single +1 is observable"
    );
    assert_eq!(
        snapshot.total_score as u32, 6,
        "score 2 + score 3 + printed single +1; per-live multiplication would show 7"
    );
}
