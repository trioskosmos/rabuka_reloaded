use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn mei_bp7_018_n_paid_live_discard_looks_five_and_takes_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.new_id("PL!SP-bp7-018-N");
    let live_cost = game.new_id("PL!-sd1-019-SD");
    game.add_to_hand(me);
    game.add_to_hand(live_cost);

    let a = game.new_id("PL!S-sd1-001-SD");
    let b = game.new_id("PL!-sd1-019-SD");
    game.give_energy(20);
    game.state.player1.main_deck.cards.insert(0, b);
    game.state.player1.main_deck.cards.insert(0, a);

    game.play_to_stage(me, MemberArea::LeftSide);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&live_cost),
        "cost live card was discarded to the waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&a),
        "one looked card added to hand"
    );
}

#[test]
fn mei_bp7_018_n_declined_live_discard_preserves_cost_card_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.new_id("PL!SP-bp7-018-N");
    let live_cost = game.new_id("PL!-sd1-019-SD");
    game.add_to_hand(me);
    game.add_to_hand(live_cost);
    game.give_energy(20);

    game.play_to_stage(me, MemberArea::LeftSide);
    game.select_indices(&[]);

    assert!(
        game.state.player1.hand.cards.contains(&live_cost),
        "declined: live card stays in hand"
    );
}
