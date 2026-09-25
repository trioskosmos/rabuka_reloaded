/// Debut cost-9 group search family (reveal-to-hand shape):
/// このメンバーをウェイトにし、手札を1枚控え室に置いてもよい：
/// 自分のデッキの上からカードを5枚見る。その中からコスト9以上の『X』の
/// メンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。
/// X = μ's (PL!-bp5-002-R, covered in eli_bp5_cost9_search.rs) /
/// Aqours (PL!S-bp5-006-R) / 虹ヶ咲 (PL!N-bp5-009-R) /
/// Liella! (PL!SP-bp5-008-R) / 蓮ノ空 (PL!HS-bp5-008-R, in izumi_bp5_test.rs).
///
/// Branch matrix per card: take / decline-pick / no-eligible auto-skip /
/// decline-cost. This file pins all four for the three non-μ's siblings.
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
    assert!(
        game.has_pending_choice(),
        "looked-at reveal prompt expected after paying the cost"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (looked_at, count=1)"
    );
    game.select_indices(&[0]); // select the qualifying member (deck top)

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

// --------------------------------------------------------------------
// Branch matrix: declined pick / no-eligible / declined cost, per card.
// Setup stocks `top_nos` as the top of a 40-card deck and returns
// (game, cost-fodder id, looked-at ids).
// --------------------------------------------------------------------

fn setup_cost9(me_no: &str, top_nos: &[&str]) -> (TestGame, i16, Vec<i16>) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(me_no);
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);
    let mut looked = Vec::new();
    for no in top_nos {
        let id = game.new_id(no);
        game.state.player1.main_deck.cards.push(id);
        looked.push(id);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        let f = game.new_id(FILLER);
        game.state.player1.main_deck.cards.push(f);
    }
    game.give_energy(15);
    game.play_to_stage(me, MemberArea::Center);
    (game, fodder, looked)
}

fn sorted(mut v: Vec<i16>) -> Vec<i16> {
    v.sort_unstable();
    v
}

/// Pay the discard cost, then decline the optional pick: nothing is taken,
/// all five looked-at cards plus the cost fodder land in the waitroom.
fn decline_pick_sends_all_to_waitroom(me_no: &str, target_no: &str) {
    let top = [target_no, FILLER, FILLER, FILLER, FILLER];
    let (mut game, fodder, looked) = setup_cost9(me_no, &top);
    game.assert_select_card("hand", 1, true);
    game.select_indices(&[0]); // pay: discard fodder
    game.assert_select_card("looked_at", 1, true);
    assert_eq!(game.state.looked_at_cards.as_slice(), &looked);
    game.select_indices(&[]); // decline the optional pick
    assert!(!game.has_pending_choice());
    assert!(game.state.player1.hand.cards.is_empty());
    let mut expected = looked.clone();
    expected.push(fodder);
    assert_eq!(
        sorted(game.state.player1.waitroom.cards.to_vec()),
        sorted(expected),
        "declined pick routes 残り + cost card to the waitroom"
    );
    assert_eq!(game.state.player1.main_deck.cards.len(), 35);
}

/// No cost-9 group member among the five: auto-skip, all five to waitroom.
fn no_eligible_auto_skips(me_no: &str) {
    let top = [FILLER, FILLER, FILLER, FILLER, FILLER];
    let (mut game, fodder, looked) = setup_cost9(me_no, &top);
    game.assert_select_card("hand", 1, true);
    game.select_indices(&[0]); // pay: discard fodder
    assert!(
        !game.has_pending_choice(),
        "no eligible card → look auto-skips without prompt"
    );
    assert!(game.state.player1.hand.cards.is_empty());
    let mut expected = looked.clone();
    expected.push(fodder);
    assert_eq!(
        sorted(game.state.player1.waitroom.cards.to_vec()),
        sorted(expected),
        "all five looked-at non-matching cards go to the waitroom"
    );
    assert_eq!(game.state.player1.main_deck.cards.len(), 35);
}

/// Decline the optional bundled cost (rest-self + discard): the ability
/// completes with no look — member stays active, fodder stays in hand,
/// deck and waitroom untouched. Same for all five template siblings.
fn declined_cost_skips_look(me_no: &str) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(me_no);
    let fodder = game.new_id(FILLER);
    game.state.player1.hand.cards.push(me);
    game.state.player1.hand.cards.push(fodder);
    while game.state.player1.main_deck.cards.len() < 40 {
        let f = game.new_id(FILLER);
        game.state.player1.main_deck.cards.push(f);
    }
    game.give_energy(15);
    let deck_before = game.state.player1.main_deck.cards.len();
    game.play_to_stage(me, MemberArea::Center);
    game.assert_select_card("hand", 1, true);
    game.select_indices(&[]); // skip the bundled rest + discard cost
    assert!(
        !game.has_pending_choice(),
        "skipped cost → effect never starts, no look prompt"
    );
    assert_eq!(game.state.player1.hand.cards.as_slice(), &[fodder]);
    assert!(
        game.state.mods.get_orientation_modifier(me).is_none(),
        "skipped rest cost → member stays active"
    );
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "deck untouched: no cards looked at"
    );
    assert!(game.state.looked_at_cards.is_empty());
}

#[test]
fn aqours_decline_pick() {
    decline_pick_sends_all_to_waitroom("PL!S-bp5-006-R", "PL!S-pb1-005-PR");
}

#[test]
fn aqours_no_eligible() {
    no_eligible_auto_skips("PL!S-bp5-006-R");
}

#[test]
fn aqours_declined_cost() {
    declined_cost_skips_look("PL!S-bp5-006-R");
}

#[test]
fn nijigasaki_decline_pick() {
    decline_pick_sends_all_to_waitroom("PL!N-bp5-009-R", "PL!N-sd1-005-PRproteinbar");
}

#[test]
fn nijigasaki_no_eligible() {
    no_eligible_auto_skips("PL!N-bp5-009-R");
}

#[test]
fn nijigasaki_declined_cost() {
    declined_cost_skips_look("PL!N-bp5-009-R");
}

#[test]
fn liella_decline_pick() {
    decline_pick_sends_all_to_waitroom("PL!SP-bp5-008-R", "PL!SP-bp1-013-PR");
}

#[test]
fn liella_no_eligible() {
    no_eligible_auto_skips("PL!SP-bp5-008-R");
}

#[test]
fn liella_declined_cost() {
    declined_cost_skips_look("PL!SP-bp5-008-R");
}
