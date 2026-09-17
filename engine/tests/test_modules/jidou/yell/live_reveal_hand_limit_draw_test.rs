use crate::helpers::*;

fn setup_live_reveal_draw(game: &mut TestGame, revealed_ids: &[i16]) {
    let source = game.id("PL!S-bp2-007-R\u{ff0b}");
    game.state.player1.stage.stage = [-1, source, -1];
    for &id in revealed_ids {
        game.state.revealed_cards.push(id);
        game.state.player1.waitroom.cards.push(id);
    }
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
}

#[test]
fn live_revealed_in_yell_with_empty_hand_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp3-026-L");
    setup_live_reveal_draw(&mut game, &[live]);

    let hand_before = game.state.player1.hand.cards.len();
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let hand_after = game.state.player1.hand.cards.len();

    assert_eq!(
        hand_after,
        hand_before + 1,
        "Live reveal draws 1 with hand at most 7"
    );
}

#[test]
fn live_revealed_in_yell_with_eight_hand_cards_does_not_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp3-026-L");
    setup_live_reveal_draw(&mut game, &[live]);

    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..8 {
        game.state.player1.hand.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let hand_after = game.state.player1.hand.cards.len();

    assert_eq!(
        hand_after, hand_before,
        "Live reveal must not draw when hand exceeds 7"
    );
}

#[test]
fn yell_without_live_card_does_not_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    setup_live_reveal_draw(&mut game, &[filler]);

    let hand_before = game.state.player1.hand.cards.len();
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    let hand_after = game.state.player1.hand.cards.len();

    assert_eq!(
        hand_after, hand_before,
        "No live card in yell must not draw"
    );
}
