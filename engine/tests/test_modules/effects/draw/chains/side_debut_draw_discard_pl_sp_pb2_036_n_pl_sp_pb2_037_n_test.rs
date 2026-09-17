use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_sp_pb2_036_n_right_side_debut_drawn_cards_reach_hand_without_blade_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.new_id("PL!SP-pb2-036-N");
    let keep_a = game.new_id("PL!S-sd1-001-SD");
    let keep_b = game.new_id("PL!N-bp3-006-R");
    game.add_to_hand(me);
    game.add_to_hand(keep_a);
    game.add_to_hand(keep_b);
    let d1 = game.new_id("PL!-sd1-007-SD");
    let d2 = game.new_id("PL!-sd1-001-SD");
    game.state.player1.main_deck.cards.insert(0, d2);
    game.state.player1.main_deck.cards.insert(0, d1);
    game.give_energy(20);

    game.play_to_stage(me, MemberArea::RightSide);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "sanity: no stray modifiers"
    );
    assert!(
        game.state.player1.hand.cards.contains(&d1)
            && game.state.player1.hand.cards.contains(&d2),
        "both drawn cards reached the hand"
    );
}

#[test]
fn pl_sp_pb2_037_n_left_side_debut_draws_two_leaving_one_known_hand_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.new_id("PL!SP-pb2-037-N");
    game.add_to_hand(me);
    let keep = game.new_id("PL!-sd1-007-SD");
    game.add_to_hand(keep);
    let d1 = game.new_id("PL!-sd1-001-SD");
    let d2 = game.new_id("PL!-sd1-004-SD");
    game.state.player1.main_deck.cards.insert(0, d2);
    game.state.player1.main_deck.cards.insert(0, d1);
    game.give_energy(20);
    let filler_deck_len = game.state.player1.main_deck.cards.len();

    game.play_to_stage(me, MemberArea::LeftSide);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        filler_deck_len - 2,
        "drew exactly 2"
    );
    let survivors = [&keep, &d1, &d2]
        .iter()
        .filter(|c| game.state.player1.hand.cards.contains(c))
        .count();
    assert_eq!(survivors, 1, "exactly one of the three remains in hand");
}
