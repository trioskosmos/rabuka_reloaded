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
fn sp_bp2_013_debut_places_waitroom_card_on_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp2-013-N"); // cost 9
    let filler = game.id("PL!-sd1-010-SD");
    let marker = game.new_id("PL!-sd1-019-SD");

    game.give_energy(12);
    fill_decks(&mut game, filler);
    game.add_to_discard(marker);

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);

    // Choose the waitroom card ("1枚まで" → SelectCard with skip allowed).
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard to pick a waitroom card"
    );
    game.select_indices(&[0]);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&marker),
        "chosen card must sit on top of the deck (index 0)"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&marker),
        "chosen card must have left the waitroom"
    );
}

#[test]
fn sp_bp2_013_debut_can_place_zero_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp2-013-N");
    let filler = game.id("PL!-sd1-010-SD");
    let bystander = game.new_id("PL!-sd1-019-SD");

    game.give_energy(12);
    fill_decks(&mut game, filler);
    game.add_to_discard(bystander);

    let deck_before = game.state.player1.main_deck.cards.len();

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    // Decline: take nothing from the waitroom.
    game.select_indices(&[]);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declined placement: deck untouched"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&bystander),
        "bystander must remain in the waitroom"
    );
}

#[test]
fn n_bp4_021_debut_accept_optional_move_puts_waitroom_card_on_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp4-021-N");
    let recycled = game.id("PL!N-sd1-025-SD");
    let other = game.new_id("PL!-sd1-010-SD");

    { let f = game.new_id("PL!-sd1-010-SD"); crate::helpers::fill_decks(&mut game, f); }
    game.add_to_hand(kasumi);
    game.add_to_discard(recycled);
    game.add_to_discard(other);
    game.give_energy(12);

    game.play_to_stage(kasumi, MemberArea::Center);

    assert!(game.has_pending_choice(), "optional move must be prompted");
    game.select_indices(&[0]);

    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&recycled),
        "accepted -> waitroom card sits on deck top"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&recycled),
        "accepted -> card left the waitroom"
    );
}

#[test]
fn n_bp4_021_debut_skip_optional_move_keeps_card_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp4-021-N");
    let recycled = game.id("PL!N-sd1-025-SD");

    { let f = game.new_id("PL!-sd1-010-SD"); crate::helpers::fill_decks(&mut game, f); }
    game.add_to_hand(kasumi);
    game.add_to_discard(recycled);
    let other = game.new_id("PL!-sd1-010-SD");
    game.add_to_discard(other);
    game.give_energy(12);

    game.play_to_stage(kasumi, MemberArea::Center);

    assert!(game.has_pending_choice(), "optional move must be prompted");
    game.select_indices(&[]);

    assert!(
        game.state.player1.waitroom.cards.contains(&recycled),
        "declined -> card stays in the waitroom"
    );
}
