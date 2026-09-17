use crate::helpers::*;

#[test]
fn live_success_reorder_does_not_trigger_on_debut() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id("PL!-bp6-016-N");
    let a = game.new_id("PL!-sd1-010-SD");
    let b = game.new_id("PL!S-sd1-001-SD");
    let c = game.new_id("PL!N-sd1-025-SD");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.add_to_hand(nozomi);
    game.state.player1.main_deck.cards.insert(0, c);
    game.state.player1.main_deck.cards.insert(0, b);
    game.state.player1.main_deck.cards.insert(0, a);
    game.give_energy(6);

    let deck_before = game.state.player1.main_deck.cards.clone();
    game.play_to_stage(nozomi, rabuka_engine::zones::MemberArea::Center);

    assert!(game.state.player1.stage.stage.contains(&nozomi));
    assert!(!game.has_pending_choice(), "LiveSuccess must not fire on debut");
    assert!(game.state.looked_at_cards.is_empty());
    assert_eq!(game.state.player1.main_deck.cards, deck_before);
}
