use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame) {
    for _ in 0..10 {
        let f = game.new_id("PL!-sd1-010-SD");
        game.state.player1.main_deck.cards.push(f);
        let f = game.new_id("PL!-sd1-010-SD");
        game.state.player2.main_deck.cards.push(f);
    }
}

#[test]
fn shioriko_bp4_basic_swap_one_niji_one_niji() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = game.id("PL!N-bp4-010-R＋");
    let niji_a = game.new_id("PL!N-sd1-025-SD");
    let niji_b = game.new_id("PL!N-sd1-025-SD");

    fill_decks(&mut game);
    game.add_to_hand(shioriko);
    game.give_energy(10);

    game.state.player1.success_live_card_zone.cards.push(niji_a);
    game.state.player1.waitroom.cards.push(niji_b);

    game.play_to_stage(shioriko, MemberArea::Center);

    game.select_indices(&[0]);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&niji_b),
        "niji_b should be in success zone after swap"
    );
    assert!(
        !game
            .state
            .player1
            .success_live_card_zone
            .cards
            .contains(&niji_a),
        "niji_a should not be in success zone after swap"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&niji_a),
        "niji_a should be in waitroom after swap"
    );
}

#[test]
fn shioriko_bp4_skip_optional_nothing_moves() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = game.id("PL!N-bp4-010-R＋");
    let niji = game.new_id("PL!N-sd1-025-SD");

    fill_decks(&mut game);
    game.add_to_hand(shioriko);
    game.give_energy(10);

    game.state.player1.success_live_card_zone.cards.push(niji);

    game.play_to_stage(shioriko, MemberArea::Center);

    game.select_indices(&[]);

    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&niji),
        "Niji card should remain in success zone after skip"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&niji),
        "Niji card should not be in waitroom after skip"
    );
}

#[test]
fn shioriko_bp4_two_niji_in_success_choose_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = game.id("PL!N-bp4-010-R＋");
    let niji_a = game.new_id("PL!N-sd1-025-SD");
    let niji_b = game.new_id("PL!N-sd1-025-SD");
    let niji_c = game.new_id("PL!N-sd1-025-SD");

    fill_decks(&mut game);
    game.add_to_hand(shioriko);
    game.give_energy(10);

    game.state.player1.success_live_card_zone.cards.push(niji_a);
    game.state.player1.success_live_card_zone.cards.push(niji_b);
    game.state.player1.waitroom.cards.push(niji_c);

    game.play_to_stage(shioriko, MemberArea::Center);

    game.select_indices(&[0]);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&niji_a),
        "niji_a (chosen to move) should be in waitroom"
    );
    assert!(
        !game
            .state
            .player1
            .success_live_card_zone
            .cards
            .contains(&niji_a),
        "niji_a should not be in success zone"
    );
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&niji_b),
        "niji_b (the second Niji) should remain in success zone"
    );
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&niji_c),
        "niji_c should be in success zone (retrieved from waitroom)"
    );
    assert_eq!(
        game.state.player1.success_live_card_zone.cards.len(),
        2,
        "Success zone should have exactly 2 cards (niji_b + niji_c)"
    );
}

#[test]
fn shioriko_bp4_non_niji_in_success_not_offered() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = game.id("PL!N-bp4-010-R＋");
    let niji = game.new_id("PL!N-sd1-025-SD");
    let non_niji = game.new_id("PL!SP-sd1-023-SD");
    let niji_b = game.new_id("PL!N-sd1-025-SD");

    fill_decks(&mut game);
    game.add_to_hand(shioriko);
    game.give_energy(10);

    game.state.player1.success_live_card_zone.cards.push(niji);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(non_niji);
    game.state.player1.waitroom.cards.push(niji_b);

    game.play_to_stage(shioriko, MemberArea::Center);

    game.select_indices(&[0]);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !game
            .state
            .player1
            .success_live_card_zone
            .cards
            .contains(&niji),
        "Niji card should have been moved out of success zone"
    );
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&non_niji),
        "Non-Niji card should remain untouched in success zone"
    );
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&niji_b),
        "Retrieved Niji should be in success zone"
    );
}

#[test]
fn shioriko_bp4_non_niji_in_discard_not_offered() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = game.id("PL!N-bp4-010-R＋");
    let niji = game.new_id("PL!N-sd1-025-SD");
    let niji_b = game.new_id("PL!N-sd1-025-SD");
    let non_niji = game.new_id("PL!SP-sd1-023-SD");

    fill_decks(&mut game);
    game.add_to_hand(shioriko);
    game.give_energy(10);

    game.state.player1.success_live_card_zone.cards.push(niji);
    game.state.player1.waitroom.cards.push(non_niji);
    game.state.player1.waitroom.cards.push(niji_b);

    game.play_to_stage(shioriko, MemberArea::Center);

    game.select_indices(&[0]);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&niji_b),
        "Niji_b should be in success zone (retrieved from waitroom)"
    );
    assert!(
        !game
            .state
            .player1
            .success_live_card_zone
            .cards
            .contains(&niji),
        "Original niji should have moved out of success zone"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_niji),
        "Non-Niji card should remain untouched in waitroom"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&niji),
        "Original niji should be in waitroom after swap"
    );
}

#[test]
fn shioriko_bp4_no_niji_in_success_skips_entirely() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = game.id("PL!N-bp4-010-R＋");
    let non_niji = game.new_id("PL!SP-sd1-023-SD");
    let niji = game.new_id("PL!N-sd1-025-SD");

    fill_decks(&mut game);
    game.add_to_hand(shioriko);
    game.give_energy(10);

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(non_niji);
    game.state.player1.waitroom.cards.push(niji);

    game.play_to_stage(shioriko, MemberArea::Center);

    assert!(
        !game.has_pending_choice(),
        "No choice should be pending - no Nijigasaki in success zone to swap out"
    );

    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&non_niji),
        "Non-Niji card should remain in success zone"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&niji),
        "Niji card should remain in waitroom (conditional never satisfied)"
    );
}

#[test]
fn shioriko_bp4_no_niji_in_discard_after_move_swaps_same_card_back() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = game.id("PL!N-bp4-010-R＋");
    let niji = game.new_id("PL!N-sd1-025-SD");

    fill_decks(&mut game);
    game.add_to_hand(shioriko);
    game.give_energy(10);

    game.state.player1.success_live_card_zone.cards.push(niji);

    game.play_to_stage(shioriko, MemberArea::Center);

    game.select_indices(&[0]);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&niji),
        "Niji card should end up back in success zone (only Niji available)"
    );
    assert_eq!(
        game.state.player1.success_live_card_zone.cards.len(),
        1,
        "Success zone should still have exactly 1 card"
    );
}

#[test]
fn shioriko_bp4_both_zones_empty_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let shioriko = game.id("PL!N-bp4-010-R＋");

    fill_decks(&mut game);
    game.add_to_hand(shioriko);
    game.give_energy(10);

    game.play_to_stage(shioriko, MemberArea::Center);

    assert!(
        !game.has_pending_choice(),
        "No choice should be pending - both zones have no Nijigasaki live cards"
    );
}
