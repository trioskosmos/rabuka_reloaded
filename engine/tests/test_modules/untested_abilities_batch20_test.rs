/// Untested-abilities batch 20 — look_and_select debuts:
/// - PL!N-bp3-012-R: optional discard 1 -> look 4, may reveal a 虹ヶ咲 card to hand
/// - PL!N-pb1-028-N: optional discard 1 -> look 2, add 1 to hand, rest to waitroom
/// - PL!HS-bp1-011-PR: optional discard 1 -> look 5, may reveal a live card to hand
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

// ====================================================================
// PL!N-bp3-012-R (登場):
// 「手札を1枚控え室に置いてもよい：自分のデッキの上からカードを4枚見る。
//   その中で『虹ヶ咲』のカードを1枚公開して手札に加えてもよい。残りを控え室に置く。」
// ====================================================================

#[test]
fn bp3012_look4_reveals_niji_card_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-bp3-012-R");
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);

    let niji = game.id("PL!N-bp3-004-R"); // 虹ヶ咲 member
    stock_deck(&mut game, &[niji]);

    game.give_energy(15);
    game.play_to_stage(me, MemberArea::Center);
    assert!(game.has_pending_choice(), "optional cost offered");
    game.select_indices(&[0]); // pay: discard fodder
    if game.has_pending_choice() {
        game.select_indices(&[0]); // select the 虹ヶ咲 card (top of the looked four)
    }

    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "虹ヶ咲 card revealed from the looked four to hand"
    );
}

// ====================================================================
// PL!N-pb1-028-N (登場):
// 「手札を1枚控え室に置いてもよい：自分のデッキの上からカードを2枚見る。
//   その中で1枚手札に加え、残りを控え室に置く。」
// ====================================================================

#[test]
fn pb1028_look2_adds_one_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-pb1-028-N");
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);

    let prize = game.new_id(FILLER);
    stock_deck(&mut game, &[prize]);

    game.give_energy(15);
    game.play_to_stage(me, MemberArea::Center);
    if game.has_pending_choice() {
        game.select_indices(&[0]); // pay cost
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]); // add first looked card to hand
    }

    assert!(
        game.state.player1.hand.cards.contains(&prize),
        "the top deck card was added to hand via look"
    );
}

// ====================================================================
// PL!HS-bp1-011-PR (登場):
// 「手札を1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。
//   その中からライブカードを1枚公開して手札に加えてもよい。残りを控え室に置く。」
// ====================================================================

#[test]
fn bp1011_look5_reveals_live_card_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-bp1-011-PR");
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);

    let live_card = game.new_id("PL!HS-sd1-020-SD"); // Hasunosora live card
    stock_deck(&mut game, &[live_card]);

    game.give_energy(15);
    game.play_to_stage(me, MemberArea::Center);
    if game.has_pending_choice() {
        game.select_indices(&[0]); // pay cost
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]); // select the live card
    }

    assert!(
        game.state.player1.hand.cards.contains(&live_card),
        "live card revealed from the looked five to hand"
    );
}
