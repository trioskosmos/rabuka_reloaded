//! Regression pin: 「…（ライブ開始時|ライブ成功時）能力が解決したとき/たび」
//! watchers must arm ONLY via the post-resolution hook
//! (trigger_each_time_for_member) after an actual member LS/LSS ability
//! completes — never from a bare board-state TAS scan.
//!
//! Before the fix, these watchers fired on every auto-scan whenever their
//! group/location condition read true (e.g. any member staged), silently
//! drawing cards / prompting repositions out of nowhere.
use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;

#[test]
fn resolution_watcher_dormant_without_actual_ls_lss_resolution() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // Victory Road (PL!N-bp5-030-L): 「メンバーのライブ成功時能力が解決するたび、
    // カードを1枚引く。」 — the canonical each_time LSS watcher.
    let victory = game.id("PL!N-bp5-030-L");
    let member = game.id("PL!SP-bp2-009-R\u{ff0b}"); // has an LSS ability (never resolved here)
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(victory);
    game.state.player1.stage.stage[1] = member;
    game.state.player1.hand.cards.push(filler);

    let deck_before = game.state.player1.main_deck.cards.len();
    let pid = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();

    assert!(
        !game.has_pending_choice(),
        "watcher must not prompt without a resolution event"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "each_time LSS watcher must NOT draw from a bare TAS scan"
    );
}
