use crate::helpers::*;

#[test]
fn nozomi_pl_bp6_016_n_debut_preserves_top_three_membership() {
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

    game.play_to_stage(nozomi, rabuka_engine::zones::MemberArea::Center);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    let top3: Vec<i16> = game.state.player1.main_deck.cards[..3].to_vec();
    let mut sorted = top3.clone();
    sorted.sort();
    let mut expected = vec![a, b, c];
    expected.sort();
    assert_eq!(
        sorted, expected,
        "all three looked-at cards return to the deck top (any order)"
    );
}
