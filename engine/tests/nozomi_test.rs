/// Tests for 東條 希 (PL!-bp5-007-R) — Debut ability via baton touch:
///
/// このメンバーがコストが低いメンバーからバトンタッチして登場した場合、
/// 自分と相手はそれぞれ自身の手札の枚数が3枚になるまで手札を控え室に置き、
/// その後、自分と相手はそれぞれカードを3枚引く。
///
/// Q229: If a player has 3 or fewer cards in hand at debut,
/// do they still draw 3 cards? A: Yes — discard step skipped, draw still happens.

mod helpers;
use helpers::*;

fn give_p2_hand(game: &mut TestGame, card_id: i16, count: usize) {
    for _ in 0..count {
        game.state.player2.hand.cards.push(card_id);
    }
}

/// Q229: Baton touch debut fires discard-to-3 then draw-3.
/// P1 has 5 cards (discard 2→3 then draw 3 = 6).
/// P2 has 2 cards (skip discard, draw 3 = 5).
#[test]
fn nozomi_q229_baton_touch_triggers_discard_then_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nozomi = game.id("PL!-bp5-007-R");
    let cheap = game.id("PL!SP-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nozomi);
    for _ in 0..5 { game.state.player1.hand.cards.push(filler); }
    give_p2_hand(&mut game, filler, 2);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage[1] = cheap;
    game.give_energy(13);
    game.play_to_stage(nozomi, rabuka_engine::zones::MemberArea::Center);

    if game.has_pending_choice() { game.select_indices(&[0, 1]); }

    assert_eq!(game.state.player1.hand.cards.len(), 6,
        "P1: 6 cards after discard 2 + draw 3");
    assert_eq!(game.state.player2.hand.cards.len(), 5,
        "P2: 2 + draw 3 = 5");
    assert!(game.state.player1.stage.stage.contains(&nozomi));
}

/// Q229: Both already at 3 hand cards → no discard, both draw 3.
#[test]
fn nozomi_q229_both_at_3_no_discard_draw_3() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nozomi = game.id("PL!-bp5-007-R");
    let cheap = game.id("PL!SP-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nozomi);
    for _ in 0..3 { game.state.player1.hand.cards.push(filler); }
    give_p2_hand(&mut game, filler, 3);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage[1] = cheap;
    game.give_energy(13);
    game.play_to_stage(nozomi, rabuka_engine::zones::MemberArea::Center);

    assert_eq!(game.state.player1.hand.cards.len(), 6,
        "P1: 4-1+3 = 6");
    assert_eq!(game.state.player2.hand.cards.len(), 6,
        "P2: 3+3 = 6");
}

/// Equal-cost baton touch: replacing a card with the SAME cost.
/// The condition has operator: "<" (replaced cost < new card cost).
/// Equal cost should NOT satisfy "<" → ability does NOT fire.
#[test]
fn nozomi_edge_equal_cost_baton_touch_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nozomi = game.id("PL!-bp5-007-R");
    // 東條希 costs 13. Use a cost-13 card on stage (any cost-13 member).
    let same_cost = game.id("PL!SP-sd1-009-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nozomi);
    for _ in 0..3 { game.state.player1.hand.cards.push(filler); }
    game.give_energy(13);

    // Stage has a cost-13 card at Center
    game.state.player1.stage.stage[1] = same_cost;
    game.play_to_stage(nozomi, rabuka_engine::zones::MemberArea::Center);

    // Baton touch occurs (equal cost → cost_paid = 13-13 = 0)
    // But the ability's operator is "<" (replaced < new)
    // 13 < 13 is false → condition should fail → no draw
    assert_eq!(game.state.player1.hand.cards.len(), 3,
        "4 hand -1 played = 3, equal-cost baton touch should not trigger ability");
}

/// Play to empty area (no baton touch) → ability condition fails → no draw.
#[test]
fn nozomi_edge_play_to_empty_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nozomi = game.id("PL!-bp5-007-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nozomi);
    for _ in 0..3 { game.state.player1.hand.cards.push(filler); }
    game.give_energy(13);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(nozomi, rabuka_engine::zones::MemberArea::Center);

    assert_eq!(game.state.player1.hand.cards.len(), 3,
        "4 hand - 1 played = 3, no draw from ability");
}
