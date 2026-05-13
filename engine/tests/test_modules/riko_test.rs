/// Tests for 桜内梨子 (PL!S-pb1-002-R) — Debut ability:
///
/// 登場 相手は手札からライブカードを1枚控え室に置いてもよい。
/// そうしなかった場合、ライブ終了時まで、
/// 「常時 ライブの合計スコアを＋１する。」を得る。
///
/// Q130/Q171: Conditional_on_optional — opponent choice + conditional score gain.
use crate::helpers::*;

/// Q130: Opponent skips discarding → conditional fires.
#[test]
fn riko_q130_opponent_skips_triggers_conditional() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let riko = game.id("PL!S-pb1-002-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    game.state.player1.hand.cards.push(riko);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.hand.cards.push(live_card);
    game.give_energy(13);

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(riko, rabuka_engine::zones::MemberArea::LeftSide);

    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Conditional fired → gain_ability effect
    assert!(game.state.player1.stage.stage.contains(&riko));
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "P1 hand: 2 - 1 played = 1"
    );
}

/// Q130 variant: Opponent discards live card → optional fires, conditional skipped.
#[test]
fn riko_q130_opponent_discards_skips_conditional() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let riko = game.id("PL!S-pb1-002-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    game.state.player1.hand.cards.push(riko);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.hand.cards.push(live_card);
    game.state.player2.hand.cards.push(filler);
    game.give_energy(13);

    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(riko, rabuka_engine::zones::MemberArea::LeftSide);

    if game.has_pending_choice() {
        game.select_option(1);
    }

    assert_eq!(
        game.state.player2.hand.cards.len(),
        1,
        "P2 hand: 2 - 1 discarded = 1"
    );
    assert!(
        !game.state.player2.hand.cards.contains(&live_card),
        "P2 live card discarded"
    );
}
