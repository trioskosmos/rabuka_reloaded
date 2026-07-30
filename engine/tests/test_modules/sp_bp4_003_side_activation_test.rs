/// Tests for PL!SP-bp4-003-R (嵐千砂都) — Left/Right side position activation.
///
/// Ability: 登場/左サイド/右サイド: カードを2枚引き、手札を2枚控え室に置く
///   (この能力は左サイドエリアか右サイドエリアに登場した場合のみ発動する。)
///
/// The ability should fire when played to left or right side, but NOT center.
/// Effect: draw 2 cards, then discard 2 cards from hand.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn setup_game() -> TestGame {
    let mut game = TestGame::new(load_real_database());
    let filler = game.id("PL!-sd1-010-SD");
    // Fill both decks so draw 2 has enough cards
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [-1, -1, -1];
    game
}

fn chisato_id(game: &TestGame) -> i16 {
    game.id("PL!SP-bp4-003-R")
}

fn hand_size(game: &TestGame) -> usize {
    game.state.player1.hand.cards.len()
}

/// Left side: ability fires → draw 2, discard 2 → net hand change = 0
#[test]
fn sp_bp4_003_left_side_draws_and_discards() {
    let mut game = setup_game();
    let chisato = chisato_id(&game);
    let filler = game.id("PL!-sd1-010-SD");

    // Put chisato + 2 fillers in hand (need fillers for discard cost)
    game.state.player1.hand.cards.push(chisato);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    let hand_before = hand_size(&game); // 3
    game.play_to_stage(chisato, MemberArea::LeftSide);

    // Resolve any pending choices (draw 2 → discard 2)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let hand_after = hand_size(&game);
    // Started with 3, played 1 to stage (-1), drew 2 (+2), discarded 2 (-2) = 2
    assert_eq!(
        hand_after,
        hand_before - 1,
        "Left side: draw 2 + discard 2 should net to hand_before - 1 (card on stage). hand {} -> {}",
        hand_before,
        hand_after
    );
}

/// Right side: ability fires → draw 2, discard 2 → net hand change = 0
#[test]
fn sp_bp4_003_right_side_draws_and_discards() {
    let mut game = setup_game();
    let chisato = chisato_id(&game);
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(chisato);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    let hand_before = hand_size(&game);
    game.play_to_stage(chisato, MemberArea::RightSide);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let hand_after = hand_size(&game);
    assert_eq!(
        hand_after,
        hand_before - 1,
        "Right side: draw 2 + discard 2 should net to hand_before - 1. hand {} -> {}",
        hand_before,
        hand_after
    );
}

/// Center: ability does NOT fire → just the play-to-stage cost
#[test]
fn sp_bp4_003_center_does_not_activate() {
    let mut game = setup_game();
    let chisato = chisato_id(&game);
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(chisato);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    let hand_before = hand_size(&game); // 2
    game.play_to_stage(chisato, MemberArea::Center);

    // Drain any auto-ability prompts that are NOT the draw/discard
    while game.has_pending_choice() {
        // If it's a SelectCard with count 2 (draw), the ability fired unexpectedly
        let choice = game.get_pending_choice().clone();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectCard { count, zone, .. } => {
                panic!(
                    "Center: ability should NOT have fired, but got SelectCard zone={} count={}",
                    zone, count
                );
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }

    let hand_after = hand_size(&game);
    assert_eq!(
        hand_after,
        hand_before - 1,
        "Center: should NOT activate, hand {} -> {}",
        hand_before,
        hand_after
    );
}
