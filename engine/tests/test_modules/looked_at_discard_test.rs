use crate::helpers::*;

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
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

    advance_to_live_start(&mut game);
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

    assert!(
        game.state.player1.waitroom.cards.contains(&looked_at_id),
        "Discarded card should be in waitroom"
    );
    assert!(
        game.state.player1.main_deck.cards.first().copied() != Some(looked_at_id),
        "Discarded card should not be on deck top"
    );
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

    advance_to_live_start(&mut game);
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

    assert!(
        game.state.player1.main_deck.cards.first().copied() == Some(looked_at_id),
        "Skipped card should be on deck top, got {:?}",
        game.state.player1.main_deck.cards.first()
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&looked_at_id),
        "Skipped card should not be in waitroom"
    );
}
