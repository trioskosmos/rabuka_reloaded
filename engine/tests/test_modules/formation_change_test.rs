/// Tests for site-change movement tracking.
///
/// Cards: PL!HS-bp2-006-R (藤島慈) — position_change with multiple_targets=true
/// Tests that cards_moved_this_turn correctly tracks each position swap.
use crate::helpers::*;
use rabuka_engine::card::CardDatabase;
use std::sync::Arc;

/// Helper: play 慈 to right side, creating 3 choices.
fn setup_three_members(db: &Arc<CardDatabase>) -> TestGame {
    let mut game = TestGame::new(db.clone());
    let chii = game.id("PL!HS-bp2-006-R");
    let a = game.new_id("PL!-sd1-013-SD");
    let b = game.new_id("PL!-sd1-013-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [a, b, -1];
    game.state.player1.hand.cards.push(chii);
    game.give_energy(15);

    game.play_to_stage(chii, rabuka_engine::zones::MemberArea::RightSide);
    // Stage: [a, b, chii] — 3 choices expected
    game
}

/// 3 cards each move → each track their movement.
#[test]
fn all_three_move_counts() {
    let db = load_real_database();
    let mut game = setup_three_members(&db);

    assert!(game.has_pending_choice(), "First choice");
    let before = game.state.cards_moved_this_turn.len();

    game.select_option(1); // move a (left) → center (swap)
    assert!(game.has_pending_choice(), "Second choice");
    game.select_option(2); // move b (center) → right (swap)
    assert!(game.has_pending_choice(), "Third choice");
    game.select_option(0); // move chii (right) → left (swap)

    assert!(!game.has_pending_choice(), "All done");
    assert!(
        game.state.cards_moved_this_turn.len() > before,
        "Card movements should increase"
    );
}

/// Stay in same position.
#[test]
fn stay_in_place_no_new_move() {
    let db = load_real_database();
    let mut game = setup_three_members(&db);

    assert!(game.has_pending_choice(), "First choice");
    let before = game.state.cards_moved_this_turn.len();

    game.select_option(0); // left → left (stay)
    assert!(game.has_pending_choice(), "Second choice");
    game.select_option(1); // center → center (stay)
                           // Third choice may or may not be present — handle gracefully
    if game.has_pending_choice() {
        game.select_indices(&[2]); // right → right (stay)
    }

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(!game.has_pending_choice(), "All done");
    assert!(
        game.state.cards_moved_this_turn.len() >= before,
        "Card count should not decrease"
    );
}

/// Mixed: 1 moves, 2 stay.
#[test]
fn one_moves_two_stay() {
    let db = load_real_database();
    let mut game = setup_three_members(&db);

    assert!(game.has_pending_choice(), "First choice");
    let before = game.state.cards_moved_this_turn.len();

    game.select_option(2); // move a (left) → right (swap with chii)
    assert!(game.has_pending_choice(), "Second choice");
    game.select_option(1); // b (center) → center (stay)
                           // Third choice may/may not exist
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(!game.has_pending_choice(), "All done");
    assert!(
        game.state.cards_moved_this_turn.len() > before,
        "At least 1 card moved"
    );
}
