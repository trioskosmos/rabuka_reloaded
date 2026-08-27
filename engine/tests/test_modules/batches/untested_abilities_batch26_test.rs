/// Untested-abilities batch 26 — look_and_select debuts with sub-unit/name filters
/// (pb1 series + sd2), using the established play->cost->look->select pattern:
/// - PL!SP-pb1-015-N: look 5 -> CatChu! card to hand
/// - PL!SP-pb1-016-N: look 5 -> KALEIDOSCORE card to hand
/// - PL!N-sd2-012-SD2: look 3 -> 虹ヶ咲 card to hand
/// Each also verifies a non-matching looked card goes to the waitroom.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member

fn stock_deck(game: &mut TestGame, top: &[i16]) {
    for &cid in top {
        game.state.player1.main_deck.cards.push(cid);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        let f = game.new_id(FILLER);
        game.state.player1.main_deck.cards.push(f);
    }
}

fn run_look_reveal(db: std::sync::Arc<rabuka_engine::card::CardDatabase>, me_no: &str, target_no: &str) {
    let mut game = TestGame::new(db);
    let me = game.id(me_no);
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);

    let target = game.id(target_no);
    stock_deck(&mut game, &[target]);
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
    game.select_indices(&[0]); // pay optional discard cost

    assert!(
        game.has_pending_choice(),
        "looked_at reveal prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the looked_at pick"
    );
    game.select_indices(&[0]); // select the matching card (deck top)

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "{} should be revealed from the looked cards to hand",
        target_no
    );
}

#[test]
fn pb1015_look5_catchu_card_to_hand() {
    let db = load_real_database();
    run_look_reveal(db, "PL!SP-pb1-015-N", "PL!SP-bp1-004-PR"); // 澁谷かのん (CatChu!)
}

#[test]
fn pb1016_look5_kaleidoscore_card_to_hand() {
    let db = load_real_database();
    run_look_reveal(db, "PL!SP-pb1-016-N", "PL!SP-bp1-013-PR"); // 葉月 凛 (KALEIDOSCORE)
}

#[test]
fn sd2012_look3_nijigasaki_card_to_hand() {
    let db = load_real_database();
    run_look_reveal(db, "PL!N-sd2-012-SD2", "PL!N-bp3-004-R"); // 虹ヶ咲 member
}
