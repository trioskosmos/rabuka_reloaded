/// Untested-abilities batch 22 — the bp5 cost-9 look_and_select trio:
/// rest self + optional discard 1 -> look 5, reveal a member whose COST is
/// 9+ (per set: Aqours / 虹ヶ咲 / Liella!) to hand, rest to waitroom.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member, cost 4 — never qualifies

/// Runs the full flow for one card and asserts `target` lands in hand.
fn look5_reveal_cost9_member(db: std::sync::Arc<rabuka_engine::card::CardDatabase>, me_no: &str, target_no: &str) {
    let mut game = TestGame::new(db);
    let me = game.id(me_no);
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);

    let target = game.id(target_no);
    stock_deck(&mut game, &[target]);
    game.give_energy(15);

    game.play_to_stage(me, MemberArea::Center);
    // me rests itself as part of the cost
    assert!(game.has_pending_choice(), "optional discard cost offered");
    game.select_indices(&[0]); // discard fodder
    if game.has_pending_choice() {
        game.select_indices(&[0]); // select the qualifying member (deck top)
    }

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "{} should be revealed from the looked five to hand",
        target_no
    );
}

fn stock_deck(game: &mut TestGame, top: &[i16]) {
    for &cid in top {
        game.state.player1.main_deck.cards.push(cid);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        let f = game.new_id(FILLER);
        game.state.player1.main_deck.cards.push(f);
    }
}

#[test]
fn bp5006_aqours_variant() {
    let db = load_real_database();
    look5_reveal_cost9_member(db, "PL!S-bp5-006-R", "PL!S-pb1-005-PR"); // Aqours cost 15
}

#[test]
fn bp5009_nijigasaki_variant() {
    let db = load_real_database();
    look5_reveal_cost9_member(db, "PL!N-bp5-009-R", "PL!N-sd1-005-PRproteinbar"); // 虹ヶ咲 cost 11
}

#[test]
fn bp5010_liella_variant() {
    let db = load_real_database();
    look5_reveal_cost9_member(db, "PL!SP-bp5-008-R", "PL!SP-bp1-013-PR"); // Liella! cost 9
}
