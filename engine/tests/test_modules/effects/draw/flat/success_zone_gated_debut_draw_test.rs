use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn pb1_005_debut_draws_with_cards_in_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-pb1-005-R"); // cost 2
    let filler = game.id("PL!-sd1-010-SD");
    let won_live = game.id("PL!-sd1-020-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(won_live);

    game.state.player1.hand.cards.push(card);
    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Debut: card left hand (-1), condition met → drew 1 (+1).
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "play (-1) + conditional draw (+1) → hand count unchanged"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&card),
        "card itself must be on stage, not back in hand"
    );
}

#[test]
fn pb1_005_debut_no_draw_without_success_zone_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-pb1-005-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);
    assert!(
        game.state.player1.success_live_card_zone.cards.is_empty(),
        "precondition: empty success zone"
    );

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // No condition, no draw: hand went 1 → 0.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "without success-zone cards the debut must not draw"
    );
}
