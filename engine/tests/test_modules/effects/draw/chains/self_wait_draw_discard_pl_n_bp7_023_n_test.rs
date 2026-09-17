use crate::helpers::*;

#[test]
fn pl_n_bp7_023_n_activation_waits_self_draws_two_discards_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let mia = game.id("PL!N-bp7-023-N");
    game.state.player1.stage.stage[1] = mia;

    let d1 = game.new_id("PL!-sd1-001-SD");
    let d2 = game.new_id("PL!S-sd1-001-SD");
    game.state.player1.main_deck.cards.insert(0, d2);
    game.state.player1.main_deck.cards.insert(0, d1);
    let h1 = game.new_id("PL!-sd1-007-SD");
    let h2 = game.new_id("PL!-sd1-004-SD");
    game.add_to_hand(h1);
    game.add_to_hand(h2);

    game.activate_ability(mia);
    assert_eq!(
        game.state.mods.get_orientation_modifier(mia),
        Some("wait"),
        "activation cost waits this member"
    );

    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&d1)
            && game.state.player1.hand.cards.contains(&d2),
        "drawn cards reached the hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&h1)
            && game.state.player1.waitroom.cards.contains(&h2),
        "two hand cards discarded to the waitroom"
    );
}
