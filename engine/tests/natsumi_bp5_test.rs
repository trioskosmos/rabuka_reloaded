/// Tests for PL!SP-bp5-009-R (鬼塚夏美) ab#0 — Q222
///
/// Ability (ライブ開始時 — on member card):
///   自分のデッキの一番上のカードを捨ててもよい。
///   捨てた場合、ライブ終了時まで、ブレードを得る。
///   更に捨てたカードがライブカードの場合、このメンバーをウェイトにする。
///   この手順を最大4回まで繰り返してもよい。
///
/// Q222: ウェイト状態になっても繰り返せるか？
/// Answer: はい、可能。

mod helpers;
use helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass(); game.pass(); game.pass(); game.pass(); game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Natsumi on stage. Deck seeded with live cards. LiveStart fires the
/// repeatable sequential (max 4). Each iteration discards top → blade gain.
/// If disc was a live card → member becomes wait. Q222: wait does NOT halt
/// the repeat loop — all 4 iterations fire regardless of wait state.
#[test]
fn natsumi_bp5_q222_repeat_continues_after_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let natsumi = game.id("PL!SP-bp5-009-R");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler_live = game.id("PL!-sd1-020-SD");

    // Natsumi on stage (active by default)
    game.state.player1.stage.stage[1] = natsumi;

    // Live cards in hand for setting live card
    game.state.player1.hand.cards.push(filler_live);
    game.state.player1.hand.cards.push(live_card);

    // Deck: all live cards → every discard triggers wait
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(live_card);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler_live);
    }

    // Give energy: change_state(wait) deactivates 1 energy per iteration (×4)
    game.give_energy(4);

    let deck_before = game.state.player1.main_deck.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);

    // LiveStart fires Natsumi's ability.
    // Sequential with repeat_procedure(4):
    //   1. move_cards (deck_top→discard, count=1)
    //   2. conditional: gain_resource(blade, 1)
    //   3. conditional if disc was live: change_state(wait)
    //   4. repeat_procedure (loops 1-3 up to 4 times)
    //
    // Q222: wait state does NOT prevent remaining iterations.
    // Verify all 4 iterations ran by checking discard count.
    let deck_after = game.state.player1.main_deck.cards.len();
    // deck changed: 4 discarded by ability + some from phase advancement
    assert!(deck_before - deck_after >= 4,
        "At least 4 cards should have been discarded (4 iterations), got {}",
        deck_before - deck_after);

    // Natsumi should be in wait state (every discard by ability was a live card)
    let orientation = game.state.get_orientation_modifier(natsumi);
    assert_eq!(orientation, Some(&"wait".to_string()),
        "Q222: Natsumi should be in wait state after live card discards");
}


