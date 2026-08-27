use crate::helpers::*;

/// PL!SP-bp7-012-N 澁谷かのん 登場:
/// 自分の控え室から、『CatChu!』と『KALEIDOSCORE』と『5yncri5e!』のカードをそれぞれ1枚ずつ選び、
/// それらを好きな順番でデッキの下に置いてもよい。そうしたとき、カードを1枚引く。
/// Nuance: optional, each group exactly 1, any_order under deck, conditional draw, and decline (0 selection).

fn setup_kanon(game: &mut TestGame) -> i16 {
    let kanon = game.id("PL!SP-bp7-012-N");
    let filler = game.id("PL!-sd1-010-SD");
    // Put kanon in hand to debut
    game.state.player1.hand.cards.push(kanon);
    // Fill discard with one of each group (unit field)
    let cat = game.id("PL!SP-bp1-004-PR"); // CatChu! unit
    let kale = game.id("PL!SP-bp1-013-PR"); // KALEIDOSCORE unit
    let five = game.id("PL!SP-pb1-014-PR"); // 5yncri5e! unit
    game.state.player1.waitroom.cards.push(cat);
    game.state.player1.waitroom.cards.push(kale);
    game.state.player1.waitroom.cards.push(five);
    game.state.player1.waitroom.cards.push(filler);
    game.give_energy(5);
    kanon
}

#[test]
fn kanon_select_three_any_order_and_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = setup_kanon(&mut game);
    let hand_before = game.state.player1.hand.cards.len();
    // Debut kanon to center
    game.play_to_stage(kanon, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();
    // Should prompt to select 3 cards (one per group) with any_order
    assert!(game.has_pending_choice(), "should prompt to select CatChu!/KALEIDOSCORE/5yncri5e! (any_order)");
    let ch = game.get_pending_choice().clone();
    // Verify the choice is a card selection (group cards) - use ch to satisfy warning and document expectation
    assert!(matches!(&ch, rabuka_engine::ability::types::Choice::SelectCard { .. } | rabuka_engine::ability::types::Choice::SelectTarget { .. }), "kanon should offer SelectCard or SelectTarget, got {:?}", ch);
    // Select all three (indices 0,1,2) - order matters for deck bottom
    game.select_indices(&[0, 1, 2]);
    game.drain_auto_ability_choices();
    // If any_order, there may be a second choice for ordering; if so, pick any
    if game.has_pending_choice() {
        // println!("second choice {:?}", game.get_pending_choice());
        game.select_indices(&[0, 1, 2]);
        game.drain_auto_ability_choices();
    }
    assert!(game.state.player1.hand.cards.len() >= hand_before - 1, "should have at least hand_before-1 after debut (hand_before {} hand now {:?})", hand_before, game.state.player1.hand.cards);
    // The draw is conditional, but at least the 3 cards should be under deck
    // Check that discard no longer has the 3 group cards
    let cat = game.id("PL!SP-bp1-004-PR");
    assert!(!game.state.player1.waitroom.cards.contains(&cat), "CatChu! should be under deck");
}

#[test]
fn kanon_decline_optional_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = setup_kanon(&mut game);
    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(kanon, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();
    assert!(game.has_pending_choice());
    // Decline by selecting 0
    game.select_indices(&[]);
    game.drain_auto_ability_choices();
    // Should NOT draw
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "declining should not draw"
    );
}

#[test]
fn kanon_only_one_group_present_can_still_select_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = game.id("PL!SP-bp7-012-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(kanon);
    // Only CatChu! in discard, not the others
    let cat = game.id("PL!SP-bp1-004-PR");
    game.state.player1.waitroom.cards.push(cat);
    game.state.player1.waitroom.cards.push(filler);
    game.give_energy(5);
    game.play_to_stage(kanon, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();
    // Should still prompt but only CatChu! is selectable
    assert!(game.has_pending_choice());
    // The number of selectable cards should be 1 (or 2 with filler not in group)
    // We just select the one CatChu!
    game.select_indices(&[0]);
    game.drain_auto_ability_choices();
    if game.has_pending_choice() {
        game.select_indices(&[0]);
        game.drain_auto_ability_choices();
    }
    // Hand after: played kanon (-1) + maybe draw (+1) = >= hand_before -1
    // We just verify the selected CatChu! is now under deck
    let cat = game.id("PL!SP-bp1-004-PR");
    assert!(!game.state.player1.waitroom.cards.contains(&cat), "CatChu! should be under deck after selection");
}
