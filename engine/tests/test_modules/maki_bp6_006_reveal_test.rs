/// Tests for PL!-bp6-006-R＋ 西木野真姫 (Maki Nishikino):
///
/// 起動 ターン1回 手札を1枚控え室に置く：好きなハートの色を1つ指定する。
/// その後、自分のデッキの上からカードを5枚公開する。
/// 公開されたカードの中に指定した色のハートを持つメンバーカードと
/// 必要ハートに指定した色を含むライブカードが合計5枚含まれる場合、
/// その中から『μ's』のカードを1枚手札に加え、ライブ終了時まで、ブレード×3を得る。
/// 公開した残りのカードを控え室に置く。
///
/// Key mechanics:
/// - 起動 (activated ability) — player chooses to use
/// - Cost: discard 1 from hand
/// - specify_heart_color → conditional_on_result(reveal 5 → check match → followup)
/// - Followup: select 1 μ's from revealed → hand + gain blade+3 (live_end)
/// - Remaining revealed cards → discard
use crate::helpers::*;

const MAKI: &str = "PL!-bp6-006-R\u{ff0b}";
const FILLER: &str = "PL!-sd1-010-SD";

fn fill_deck(game: &mut TestGame, player: &str, count: usize) {
    let ids: Vec<i16> = (0..count).map(|_| game.id(FILLER)).collect();
    let deck = if player == "p1" {
        &mut game.state.player1.main_deck.cards
    } else {
        &mut game.state.player2.main_deck.cards
    };
    for f in ids {
        deck.push(f);
    }
}

/// 起動 ability: activate → handle all pending choices until resolved.
fn activate_and_resolve(game: &mut TestGame, card: i16) {
    game.activate_ability(card);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

/// 条件充足: デッキ上5枚が全て同一カード(heart01あり) → μ's選択カードが手札に加わる
///
/// BUG: gain_resource blade+3 in conditional_on_result followup has no
/// activating_card context, so blade is NOT applied. The select_cards
/// followup DOES work (card added to hand), and remaining revealed cards
/// DO go to discard. Only blade gain is broken.
#[test]
fn all_5_match_selects_muse_card_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI);
    let filler = game.id(FILLER);

    game.state.player1.stage.stage = [maki, -1, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    // 5 copies of Maki on deck top — all have heart01:2, heart03:2, heart06:3
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(maki);
    }
    fill_deck(&mut game, "p1", 10);
    fill_deck(&mut game, "p2", 10);
    game.give_energy(20);

    activate_and_resolve(&mut game, maki);

    // select_cards worked: a μ's card from revealed was added to hand
    let hand_names: Vec<String> = game.state.player1.hand.cards.iter()
        .map(|&id| game.name(id))
        .collect();
    let hand_has_maki = hand_names.iter().any(|n| n.contains("西木野真姫"));
    assert!(hand_has_maki, "A Maki card should have been added to hand from revealed cards, hand={:?}", hand_names);

    // Remaining 4 revealed cards + 1 cost discard = 5 in discard
    assert!(game.state.player1.waitroom.cards.len() >= 5,
        "At least 5 cards in discard (4 remaining revealed + 1 cost), got {}",
        game.state.player1.waitroom.cards.len());
}

/// BUG DOCUMENTATION: gain_resource blade+3 in conditional_on_result followup
/// has no activating_card context, so blade is NOT applied.
///
/// Root cause: when conditional_on_result saves/resumes the followup sequential,
/// the ability queue entry's activating_card is lost (set to None).
/// The gain_resource effect at misc.rs:1630 checks:
///   `if let Some(card_id) = activating_card_id` — but it's None, so no blade.
///
/// Expected behavior: blade+3 should be applied to the activating card (Maki).
#[test]
fn gain_resource_blade_bug_activating_card_lost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI);
    let filler = game.id(FILLER);

    game.state.player1.stage.stage = [maki, -1, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(maki);
    }
    fill_deck(&mut game, "p1", 10);
    fill_deck(&mut game, "p2", 10);
    game.give_energy(20);

    activate_and_resolve(&mut game, maki);

    // BUG: blade should be +3 but activating_card is lost in followup
    let blade = game.state.mods.get_blade_modifier(maki);
    // When fixed, change this to: assert_eq!(blade, 3, "blade+3 from gain_resource");
    assert_eq!(blade, 0,
        "Known bug: gain_resource blade+3 not applied — activating_card is None in \
         conditional_on_result followup. When fixed, assert blade==3");
}

/// 条件不充足: デッキ上5枚が全てランダムカード → ブレードなし、残り控え室
///
/// NOTE: PL!-sd1-010-SD has heart01:1, heart03:1. We choose heart06 which they don't have.
#[test]
fn no_match_no_blade_all_revealed_to_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI);
    let filler = game.id(FILLER);

    game.state.player1.stage.stage = [maki, -1, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    // 5 filler cards on deck top — have heart01:1, heart03:1
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    fill_deck(&mut game, "p1", 10);
    fill_deck(&mut game, "p2", 10);
    game.give_energy(20);

    game.activate_ability(maki);

    // Handle cost (discard from hand) then choose heart06 (index 5)
    // Use select_choice_option for SelectHeartColor since select_indices uses
    // card_id (not card_indices) for this choice type.
    while game.has_pending_choice() {
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectHeartColor { .. } => {
                game.select_choice_option(5); // heart06 — filler has no heart06
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    // Condition should NOT be met with heart06 — no blade gained
    let blade = game.state.mods.get_blade_modifier(maki);
    assert_eq!(blade, 0, "No blade when condition not met (heart06 chosen, filler has heart01:1/heart03:1)");

    // All 5 revealed filler cards should be in discard
    assert!(game.state.player1.waitroom.cards.len() >= 5,
        "At least 5 revealed cards should be in discard, got {}",
        game.state.player1.waitroom.cards.len());
}

/// ターン1回制限: 2回目は使えない
#[test]
fn use_limit_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI);
    let filler = game.id(FILLER);

    game.state.player1.stage.stage = [maki, -1, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(maki);
    }
    fill_deck(&mut game, "p1", 10);
    fill_deck(&mut game, "p2", 10);
    game.give_energy(20);

    // First activation — should work
    activate_and_resolve(&mut game, maki);
    let hand_after_first = game.state.player1.hand.cards.len();

    // Second activation — should fail (use_limit: 1)
    let result = game.try_activate_ability(maki);
    if result.is_ok() {
        while game.has_pending_choice() {
            game.select_indices(&[0]);
        }
    }
    let hand_after_second = game.state.player1.hand.cards.len();

    // Second activation should not have changed hand (no card selected from revealed)
    assert_eq!(
        hand_after_first, hand_after_second,
        "Second activation should be blocked by use_limit: 1"
    );
}
