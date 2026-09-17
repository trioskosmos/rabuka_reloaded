use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

fn stock_group_look_deck(game: &mut TestGame, top: &[i16]) {
    for &cid in top {
        game.state.player1.main_deck.cards.push(cid);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        let f = game.new_id(FILLER);
        game.state.player1.main_deck.cards.push(f);
    }
}

fn assert_paid_discard_group_look_reveals_to_hand(
    db: std::sync::Arc<rabuka_engine::card::CardDatabase>,
    me_no: &str,
    target_no: &str,
) {
    let mut game = TestGame::new(db);
    let me = game.id(me_no);
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);

    let target = game.id(target_no);
    stock_group_look_deck(&mut game, &[target]);
    game.give_energy(15);

    game.play_to_stage(me, MemberArea::Center);
    assert!(
        game.has_pending_choice(),
        "optional discard cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard cost"
    );
    game.select_indices(&[0]);

    assert!(
        game.has_pending_choice(),
        "looked_at reveal prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the looked_at pick"
    );
    game.select_indices(&[0]);

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "{} should be revealed from the looked cards to hand",
        target_no
    );
}

#[test]
fn pl_sp_pb1_015_n_paid_discard_looks_five_takes_catchu_card() {
    let db = load_real_database();
    assert_paid_discard_group_look_reveals_to_hand(db, "PL!SP-pb1-015-N", "PL!SP-bp1-004-PR");
}

#[test]
fn pl_sp_pb1_016_n_paid_discard_looks_five_takes_kaleidoscore_card() {
    let db = load_real_database();
    assert_paid_discard_group_look_reveals_to_hand(db, "PL!SP-pb1-016-N", "PL!SP-bp1-013-PR");
}

#[test]
fn pl_n_sd2_012_sd2_paid_discard_looks_three_takes_nijigasaki_card() {
    let db = load_real_database();
    assert_paid_discard_group_look_reveals_to_hand(db, "PL!N-sd2-012-SD2", "PL!N-bp3-004-R");
}
