use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

fn stock_filtered_look_deck(game: &mut TestGame, top: &[i16]) {
    for &cid in top {
        game.state.player1.main_deck.cards.push(cid);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        let f = game.new_id(FILLER);
        game.state.player1.main_deck.cards.push(f);
    }
}

#[test]
fn pl_n_bp3_012_r_paid_discard_looks_four_reveals_nijigasaki_card_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-bp3-012-R");
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);

    let niji = game.id("PL!N-bp3-004-R");
    stock_filtered_look_deck(&mut game, &[niji]);

    game.give_energy(15);
    game.play_to_stage(me, MemberArea::Center);
    assert!(game.has_pending_choice(), "optional cost offered");
    game.select_indices(&[0]);
    assert!(
        game.has_pending_choice(),
        "look-at-four reveal prompt expected after paying the optional cost"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (zone=looked_at count=1 allow_skip=true group=虹ヶ咲)"
    );
    game.select_indices(&[0]);

    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "虹ヶ咲 card revealed from the looked four to hand"
    );
}

#[test]
fn pl_hs_bp1_011_pr_paid_discard_looks_five_reveals_live_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-bp1-011-PR");
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);

    let live_card = game.new_id("PL!HS-sd1-020-SD");
    stock_filtered_look_deck(&mut game, &[live_card]);

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
        "look-at-five reveal prompt expected after paying the cost"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (zone=looked_at count=1 allow_skip=true live_card)"
    );
    game.select_indices(&[0]);

    assert!(
        game.state.player1.hand.cards.contains(&live_card),
        "live card revealed from the looked five to hand"
    );
}
