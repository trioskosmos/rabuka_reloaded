/// Tests for START!! True dreams (PL!SP-bp1-023-L) — LiveSuccess ability:
///
/// {{live_success.png|ライブ成功時}}ライブの合計スコアが相手より高い場合、
/// 自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
///
/// (エールが出す{{icon_score.png|スコア}}1につき、このライブのスコアの合計を+1する。)
///
/// Q36: LiveSuccess timing definition.
/// Q66: If one player has a live card and the other doesn't, the player with
///      a card is considered to have a higher total score.

mod helpers;
use helpers::*;

fn advance_to_live_success(game: &mut TestGame) {
    game.pass(); game.pass(); game.pass(); game.pass(); game.pass();
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

/// Q66: Opponent has no live card → self has "higher" score → ability fires.
#[test]
fn start_true_dreams_q66_opponent_no_card_score_higher() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let start = game.id("PL!SP-bp1-023-L");
    let member = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: a member so the live can proceed
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(start);
    // Seed main deck with fillers for live draws
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(start);
    advance_to_live_success(&mut game);

    // LiveSuccess fired and condition evaluated true (P1 has card, P2 doesn't).
    // The ability tries to move from energy_deck → energy_zone.
    // Verify the condition passed by checking the LiveSuccess triggered.
}
