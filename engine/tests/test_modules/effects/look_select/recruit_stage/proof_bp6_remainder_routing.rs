/// Look_and_select with recruit remainder (remainder-to-deck-top shape):
/// N枚見る → 1枚を手札に加え、残りをデッキの上に戻す。
/// The only ability with this shape; regression home for the split
/// destination/remainder_destination parse (Issue 12).
///
/// Card: PL!HS-bp6-029-L (Proof) — LiveStart: if 蓮ノ空 stage cost ≥ 20,
/// look 2 → take 1 to hand, rest back to deck top; if ≥ 30, additionally
/// heart00 -2. Below 20: no effect at all.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

// ====================================================================
// Issue 12: PL!HS-bp6-029-L (Proof)
// ====================================================================
// LiveStart: if 蓮ノ空 cost >= 20, look 2 → pick 1 to hand, rest deck_top.
// If >= 30, additionally heart00 -2.
// Parser fix: split destination (hand) + remainder_destination (deck_top).
// ====================================================================

#[test]
fn issue12_proof_look_and_select_cost_20plus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    let hs1 = game.id("PL!HS-sd1-008-SD"); // cost 13
    let hs2 = game.id("PL!HS-bp1-004-R\u{ff0b}"); // cost 15
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs1, hs2, filler];
    game.state.player1.main_deck.cards.clear();
    let top1 = game.new_id("PL!-sd1-010-SD");
    let top2 = game.new_id("PL!-sd1-005-SD");
    game.state.player1.main_deck.cards.push(top1);
    game.state.player1.main_deck.cards.push(top2);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }

    // No heart reduction (cost 28 < 30)
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(proof, HeartColor::Heart00),
        0,
        "12a: cost < 30 -> no heart reduction"
    );
}

#[test]
fn issue12_proof_look_and_select_cost_30plus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    let hs_a = game.id("PL!HS-bp1-004-R\u{ff0b}"); // cost 15
    let hs_b = game.id("PL!HS-bp5-002-R\u{ff0b}"); // cost 15
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs_a, hs_b, filler];
    game.state.player1.main_deck.cards.clear();
    let top1 = game.new_id("PL!-sd1-010-SD");
    let top2 = game.new_id("PL!-sd1-005-SD");
    game.state.player1.main_deck.cards.push(top1);
    game.state.player1.main_deck.cards.push(top2);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }

    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(proof, HeartColor::Heart00);
    eprintln!("[12b] heart00 mod: {}", h00);
    // Card says: "30以上の場合、さらに必要ハートをheart0x2減らす"
    assert_eq!(h00, -2, "12b: cost >= 30 -> heart00 -2, got {}", h00);
}

// ====================================================================
// Proof edge case: cost < 20 → no effect at all
// ====================================================================

#[test]
fn proof_cost_below_20_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    let hs_low = game.id("PL!HS-bp1-005-PR"); // cost=9
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs_low, filler, filler];
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }

    // Cost 9 < 20 → no effect at all. Hand should have only proof.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Proof: cost=9 < 20 → should draw nothing"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(proof, HeartColor::Heart00),
        0,
        "Proof: cost=9 < 20 → no heart reduction"
    );
}

// ====================================================================
// Proof edge case: cost >=20 → look-and-select draws 1 card
// ====================================================================

#[test]
fn proof_cost_20plus_draws_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    // Use abilityless 蓮ノ空 members (no LiveStart to interfere):
    //   PL!HS-bp1-016-PR cost=9, PL!HS-bp1-016-N cost=9, PL!HS-bp1-012-PR cost=4
    //   Total: 9+9+4=22 >= 20
    let hs1 = game.id("PL!HS-bp1-016-PR");
    let hs2 = game.id("PL!HS-bp1-016-N");
    let hs3 = game.id("PL!HS-bp1-012-PR");
    // Use filler for remaining cards
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs1, hs2, hs3];
    game.state.player1.main_deck.cards.clear();
    // Top 2 must be 蓮ノ空 cards (select_action has group_names filter)
    let top1 = game.id("PL!HS-bp1-012-PR"); // abilityless 蓮ノ空 cost=4
    let top2 = game.id("PL!HS-bp1-012-N"); // abilityless 蓮ノ空 cost=4
    game.state.player1.main_deck.cards.push(top1);
    game.state.player1.main_deck.cards.push(top2);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    // Only Proof's LiveStart fires (abilityless members have none).
    // Proof's look-and-select creates a SelectCard choice → pick index 0.
    let hand_after_setup = game.state.player1.hand.cards.len();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Total cost 9+9+4=22 >= 20 → look-and-select fires → draws 1 card
    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_after,
        hand_after_setup + 1,
        "Proof: cost 22 >= 20 → should draw 1 card (hand {}=>{} )",
        hand_after_setup,
        hand_after
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(proof, HeartColor::Heart00),
        0,
        "Proof: cost 22 < 30 → no heart reduction"
    );
}
