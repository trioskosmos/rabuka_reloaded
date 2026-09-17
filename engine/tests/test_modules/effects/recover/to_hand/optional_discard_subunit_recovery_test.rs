use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member — never matches sub-unit filters

fn play_and_retrieve(db: std::sync::Arc<rabuka_engine::card::CardDatabase>, me_no: &str, retrieved: &str) {
    let mut game = TestGame::new(db);
    let me = game.id(me_no);
    game.state.player1.hand.cards.push(me);
    game.give_energy(20);

    // The card to retrieve sits in the waitroom.
    let target = game.id(retrieved);
    game.state.player1.waitroom.cards.push(target);

    // A fodder card for the optional discard cost.
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(fodder);

    game.play_to_stage(me, MemberArea::Center);
    assert!(game.has_pending_choice(), "optional discard cost offered");
    game.select_indices(&[0]); // discard the fodder

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "{} retrieved to hand",
        retrieved
    );
}

#[test]
fn sp_pb2_015_debut_discard_recovers_catchu_card() {
    let db = load_real_database();
    play_and_retrieve(db, "PL!SP-pb2-015-R", "PL!SP-bp1-004-PR"); // 澁谷かのん (CatChu!)
}

#[test]
fn sp_pb2_019_debut_discard_recovers_5yncri5e_card() {
    let db = load_real_database();
    play_and_retrieve(db, "PL!SP-pb2-019-R", "PL!SP-pb1-014-PR"); // 5yncri5e! member
}

#[test]
fn sp_pb2_021_debut_discard_recovers_kaleidoscore_card() {
    let db = load_real_database();
    play_and_retrieve(db, "PL!SP-pb2-021-R", "PL!SP-bp1-013-PR"); // 葉月 凛 (KALEIDOSCORE)
}
