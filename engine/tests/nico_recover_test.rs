/// Tests for 矢澤にこ (PL!-bp5-009-R) — Activation ability:
///
/// 起動 ターン1回 手札を2枚控え室に置く：
/// 自分の控え室から必要ハートにheart06を3以上含むライブカードを1枚手札に加える。
///
/// Q209: The live card discarded as cost can be recovered by the effect,
/// since the cost resolves before the effect checks the discard.

mod helpers;
use helpers::*;

/// Q209: Discard a live card as cost, then recover it from discard via the effect.
#[test]
fn nico_q209_discard_then_recover_live_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nico = game.id("PL!-bp5-009-R");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::Center);

    // Hand after play: live_card + 2 filler = 3 cards
    game.dbg_all();
    game.activate_ability(nico);
    game.dbg_all();

    // Cost prompt: select 2 cards from hand to discard
    if game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }
    game.dbg_all();

    // Effect prompt: select 1 live card from discard to add to hand
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.dbg_all();

    assert!(game.state.player1.hand.cards.contains(&live_card),
        "Q209: Live card should be recoverable");
    assert_eq!(game.state.player1.hand.cards.len(), 2);
}

/// Edge: no live card in discard → effect does nothing gracefully.
#[test]
fn nico_q209_no_live_card_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nico = game.id("PL!-bp5-009-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::Center);

    // Hand after play: 3 filler = 3 cards (no live card)
    game.activate_ability(nico);

    if game.has_pending_choice() {
        // Discard 2 fillers
        game.select_indices(&[0, 1]);
    }

    // Effect should find 0 live cards in discard → no-op, no choice
    assert!(!game.has_pending_choice(),
        "No live card in discard → no selection prompt");
    // Hand: 3 - 2 = 1
    assert_eq!(game.state.player1.hand.cards.len(), 1,
        "No recovery when no live card in discard");
}
