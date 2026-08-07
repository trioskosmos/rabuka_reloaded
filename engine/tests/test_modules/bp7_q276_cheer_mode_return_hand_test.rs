/// Q276 — 虹ヶ咲 統括 / Cheer Mode PL!N-bp7-030-L.
///
/// ab#0 (ライブ成功時): 自分のデッキの上からカードを3枚見る。その中から好きな枚数
/// を好きな順番でデッキの上に置き、残りを控え室に置く。
/// ab#1 (ライブ成功時): このカードをライブカード置き場から手札に戻す。その後、
/// 手札を1枚控え室に置く。
///
/// Official QA Q276:
/// 質問: 「このカードのみでライブに勝利しました。そのとき、成功ライブカード置き場に
/// このカードを置くことはできますか？」
/// 回答: いいえ。下のライブ成功時能力によって必ず手札に戻ります。そのため、ライブ
/// カード置き場にこのカードがなくなるため、成功ライブカード置き場には置けません。
///
/// rule: the card's own ライブ成功時 return-to-hand is mandatory and beats the success-zone
/// placement — after the winning live the card is in the hand, not the success zone.
use crate::helpers::*;

const CHEER: &str = "PL!N-bp7-030-L"; // need {heart04:1, heart0:1}, score 0
// PL!S-bp2-015-PR: heart04=1, heart05=1 → covers heart04 note; surplus fills heart0.
const MEMBER: &str = "PL!S-bp2-015-PR";

fn advance_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
fn advance_to_live_victory(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Q276 exact case: win a live with only Cheer Mode; its ライブ成功時 ab#1 returns it to
/// hand, so it is NOT placed in the success live-card zone.
#[test]
fn q276_cheer_mode_returns_to_hand_not_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let cheer = game.id(CHEER);
    let member = game.id(MEMBER);
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(cheer);
    // Extra hand cards so ライブ成功時 ab#1's discard step can discard a non-Cheer card.
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set(&mut game);
    game.set_live_card(cheer);
    assert!(
        game.state.player1.live_card_zone.cards.contains(&cheer),
        "Cheer Mode set as the live card"
    );
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    advance_to_live_victory(&mut game);

    // Drain all LiveSuccess choices: look-at-3 (ab#0, skip), return-to-hand + discard ab#1.
    // The discard step is a SelectCard over the hand; pick the FIRST hand card (a filler),
    // NOT Cheer, so Cheer stays in hand.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice().clone() {
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. } if zone == "hand" => {
                game.select_indices(&[0]);
            }
            rabuka_engine::ability::types::Choice::SelectTarget { .. } if game
                .pending_choice_type()
                .as_deref()
                == Some("SelectAutoAbility")
            => {
                game.select_choice_option(0);
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }

    assert!(
        !game.state.player1.success_live_card_zone.cards.contains(&cheer),
        "Q276: Cheer Mode must NOT be in the success zone (returned to hand); success zone={:?}",
        game.state.player1.success_live_card_zone.cards
    );
    assert!(
        game.state.player1.hand.cards.contains(&cheer),
        "Q276: Cheer Mode is returned to the hand by its ライブ成功時 ability"
    );
}