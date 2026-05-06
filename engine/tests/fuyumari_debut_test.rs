/// Tests for PL!SP-bp2-011-R (鬼塚冬毬) ab#0 — Q118
///
/// Ability (登場):
///   自分の控え室から、カード名の異なるライブカードを2枚選ぶ。
///   選択した場合、相手はそのカードのうち1枚を選ぶ。
///   相手に選ばれたカードを自分の手札に加える。
///
/// Q118: 1枚しか選べない場合、相手が選んで手札に加えられるか？
/// Answer: いいえ。2枚選べないと効果は不発。

mod helpers;
use helpers::*;

/// 2 different-named live cards in discard. Debut fires → player selects 2 →
/// opponent picks 1 → that card goes to hand.
#[test]
fn fuyumari_q118_two_distinct_live_cards_opponent_chooses() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let fuyumari = game.id("PL!SP-bp2-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    // Two different-named live cards
    let live_a = game.id("PL!-sd1-019-SD");  // START:DASH!!
    let live_b = game.id("PL!-sd1-020-SD");  // other live card

    // Fuyumari in hand
    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.hand.cards.push(filler);

    // Two different-named live cards in discard
    game.state.player1.waitroom.cards.push(live_a);
    game.state.player1.waitroom.cards.push(live_b);

    game.give_energy(11);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::LeftSide);

    // Debut fires: select 2 distinct live cards from discard
    if game.has_pending_choice() {
        game.select_indices(&[0, 1]);  // select both live cards
    }

    // Opponent chooses 1 of the 2
    if game.has_pending_choice() {
        game.select_option(0);  // opponent picks first card
    }

    // One live card should be in hand (the one opponent chose)
    let in_hand = game.state.player1.hand.cards.contains(&live_a)
        || game.state.player1.hand.cards.contains(&live_b);
    assert!(in_hand,
        "Q118: Opponent-chosen live card should be in P1 hand");
}

/// Only 1 live card in discard → can't select 2 → effect doesn't trigger.
/// No card added to hand.
#[test]
fn fuyumari_q118_only_one_live_card_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let fuyumari = game.id("PL!SP-bp2-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    game.state.player1.hand.cards.push(fuyumari);
    game.state.player1.hand.cards.push(filler);
    // Only 1 live card in discard
    game.state.player1.waitroom.cards.push(live);
    // Also a non-live card in discard (won't match card_type filter)
    game.state.player1.waitroom.cards.push(filler);

    game.give_energy(11);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(fuyumari, rabuka_engine::zones::MemberArea::LeftSide);

    // With only 1 live card, ability tries to select 2 but can't.
    // Q118: No card added when fewer than 2 available.
    // Handle any pending choice from the select attempt
    if game.has_pending_choice() {
        game.select_indices(&[]);  // dismiss the choice
    }
    assert!(!game.state.player1.hand.cards.contains(&live),
        "Q118: No live card should be added when <2 available");
}
