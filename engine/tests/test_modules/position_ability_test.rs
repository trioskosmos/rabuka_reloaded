use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn setup_game() -> TestGame {
    let mut game = TestGame::new(load_real_database());
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [-1, -1, -1];
    game
}

fn hand_size(game: &TestGame) -> usize {
    game.state.player1.hand.cards.len()
}

fn coco_id(game: &TestGame) -> i16 {
    game.id("PL!SP-bp1-002-R\u{ff0b}")
}

fn chisato_id(game: &TestGame) -> i16 {
    game.id("PL!SP-bp4-003-R")
}

/// PL!SP-bp1-002-R+ (唐可可): 登場, 左サイド — draw 2 if on LEFT side.
#[test]
fn coco_left_side_draws() {
    let mut game = setup_game();
    let coco = coco_id(&game);
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(coco);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    let hand_before = hand_size(&game);
    game.play_to_stage(coco, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_option(1);
    }

    let hand_after = hand_size(&game);
    assert!(
        hand_after > hand_before - 1,
        "Coco on Left: should draw, hand {} -> {}",
        hand_before,
        hand_after
    );
}

#[test]
fn coco_center_does_not_draw() {
    let mut game = setup_game();
    let coco = coco_id(&game);
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(coco);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    let hand_before = hand_size(&game);
    game.play_to_stage(coco, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_option(1);
    }

    let hand_after = hand_size(&game);
    assert_eq!(
        hand_after,
        hand_before - 1,
        "Coco on Center: should NOT draw, hand {} -> {}",
        hand_before,
        hand_after
    );
}

#[test]
fn coco_right_side_does_not_draw() {
    let mut game = setup_game();
    let coco = coco_id(&game);
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(coco);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    let hand_before = hand_size(&game);
    game.play_to_stage(coco, MemberArea::RightSide);
    while game.has_pending_choice() {
        game.select_option(1);
    }

    let hand_after = hand_size(&game);
    assert_eq!(
        hand_after,
        hand_before - 1,
        "Coco on Right: should NOT draw, hand {} -> {}",
        hand_before,
        hand_after
    );
}

#[test]
fn chisato_left_side_draws_and_discards() {
    let mut game = setup_game();
    let chisato = chisato_id(&game);
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(chisato);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    let hand_before = hand_size(&game);
    game.play_to_stage(chisato, MemberArea::LeftSide);

    assert!(
        game.has_pending_choice(),
        "Chisato on Left: ability should have triggered (pending choice expected)"
    );

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let hand_after = hand_size(&game);
    assert_eq!(
        hand_after,
        hand_before - 1,
        "Chisato on Left: draw 2 + discard 2 should net hand_before - 1. hand {} -> {}",
        hand_before,
        hand_after
    );
}

#[test]
fn chisato_right_side_draws_and_discards() {
    let mut game = setup_game();
    let chisato = chisato_id(&game);
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(chisato);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    let hand_before = hand_size(&game);
    game.play_to_stage(chisato, MemberArea::RightSide);

    assert!(
        game.has_pending_choice(),
        "Chisato on Right: ability should have triggered (pending choice expected)"
    );

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let hand_after = hand_size(&game);
    assert_eq!(
        hand_after,
        hand_before - 1,
        "Chisato on Right: draw 2 + discard 2 should net hand_before - 1. hand {} -> {}",
        hand_before,
        hand_after
    );
}

#[test]
fn chisato_center_does_not_activate() {
    let mut game = setup_game();
    let chisato = chisato_id(&game);
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(chisato);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    let hand_before = hand_size(&game);
    game.play_to_stage(chisato, MemberArea::Center);

    while game.has_pending_choice() {
        let choice = game.get_pending_choice().clone();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectCard { count, zone, .. } => {
                panic!(
                    "Chisato on Center: ability should NOT have fired, but got SelectCard zone={} count={}",
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
        "Chisato on Center: should NOT activate, hand {} -> {}",
        hand_before,
        hand_after
    );
}
