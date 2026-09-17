use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

fn stock_pl_n_pb1_028_n_look_deck(game: &mut TestGame, top: &[i16]) {
    for &cid in top {
        game.state.player1.main_deck.cards.push(cid);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        let f = game.new_id(FILLER);
        game.state.player1.main_deck.cards.push(f);
    }
}

#[test]
fn pl_n_pb1_028_n_paid_discard_looks_two_adds_one_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-pb1-028-N");
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);

    let prize = game.new_id(FILLER);
    stock_pl_n_pb1_028_n_look_deck(&mut game, &[prize]);

    game.give_energy(15);
    game.play_to_stage(me, MemberArea::Center);
    assert!(
        game.has_pending_choice(),
        "optional discard-1 cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (zone=hand count=1 allow_skip=true) for the cost"
    );
    game.select_indices(&[0]);
    assert!(
        game.has_pending_choice(),
        "look-at-two add-to-hand prompt expected after paying the cost"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (zone=looked_at count=1 allow_skip=false)"
    );
    game.select_indices(&[0]);

    assert!(
        game.state.player1.hand.cards.contains(&prize),
        "the top deck card was added to hand via look"
    );
}
