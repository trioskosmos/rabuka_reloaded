/// Tests for 矢澤にこ (PL!-bp5-009-R) — Activation ability:
///
/// 起動 ターン1回 手札を2枚控え室に置く：
/// 自分の控え室から必要ハートにheart06を3以上含むライブカードを1枚手札に加える。
///
/// Q209: Live card discarded as cost can be recovered by the same ability.

mod helpers;
use helpers::*;

/// Q209: Cost creates a choice (3 cards in hand, need to pick 2).
/// After paying cost, effect recovers the same live card from discard.
#[test]
fn nico_q209_cost_choice_then_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nico = game.id("PL!-bp5-009-R");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Hand: nico + live_card + 2x filler = 4 cards
    // After play_to_stage: hand = live_card + 2 filler = 3 cards
    // Cost (discard 2) with 3 cards → choice created to pick which 2
    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::Center);

    // Hand: live_card + filler + filler = 3 cards
    game.activate_ability(nico);

    // Cost prompt: select 2 cards from hand to discard
    if game.has_pending_choice() {
        game.select_indices(&[0, 1]); // discard live_card + 1 filler
    }

    // Effect should recover the live card from discard
    assert!(game.state.player1.hand.cards.contains(&live_card),
        "Q209: Live card discarded as cost should be recoverable");
    // Hand: 3 - 2 + 1 = 2
    assert_eq!(game.state.player1.hand.cards.len(), 2,
        "Q209: Net 2 cards in hand");
}

/// Edge: no live card in discard → effect skips gracefully.
#[test]
fn nico_q209_no_live_card_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nico = game.id("PL!-bp5-009-R");
    let filler = game.id("PL!-sd1-010-SD");

    // Hand: nico + 3 filler = 4. After play: 3 filler = 3 cards
    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.state.player1.stage.stage[1] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::Center);

    game.activate_ability(nico);

    if game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }

    // No live card in discard → no recovery
    assert_eq!(game.state.player1.hand.cards.len(), 1,
        "No recovery when no live card in discard");
}
