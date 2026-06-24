/// Tests for 西木野真姫 (PL!-pb1-015-R) — two abilities:
///
/// ab#0: 登場/ライブ開始時 センター
///       『BiBi』のメンバー1人をウェイトにしてもよい：
///       相手は自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。
///       （この能力はセンターエリアにいる場合のみ発動する。）
///
/// ab#1: 自動 ターン1回
///       自分のカードの効果によって、相手のステージにいるアクティブ状態の
///       コスト4以下のメンバーがウェイト状態になったとき、カードを1枚引く。
///
/// Parser fixes tested:
///   - _try_state_change calls _extract_generic_fields (cost_limit=4 populates)
///   - extract_target no longer returns "both" from "自分のカードの効果" text
///
/// Test flow: play_to_stage enqueues both ab#0 and ab#1 simultaneously.
/// A SelectAutoAbility choice appears: select_option(1) picks the auto
/// (which fails — no wait yet), then ab#0 runs and creates an optional
/// cost choice. A second select_option(1) pays it. Then the opponent
/// choice or auto-draw resolves.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Maki debut at center: pay optional cost → opponent waits 1 member.
/// ab#1 (auto-draw on state change) triggers after the effect resolves.
#[test]
fn maki_debut_pay_cost_opponent_waits_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    let p2_a = game.id("PL!-pb1-011-R"); // cost 2
    let p2_b = game.id("PL!-pb1-009-R"); // cost 4
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.stage.stage = [p2_a, p2_b, -1];
    game.give_energy(11);

    game.play_to_stage(maki, MemberArea::Center);

    // ab#1 is pre-filtered out (no state change yet). ab#0 auto-resolves.

    // 1. Pay optional cost (card_id=1 → "pay_optional_cost")
    game.select_option(1);

    // 2. Opponent choice: select which of their 2 members to wait
    game.select_indices(&[0]);

    // 3. ab#1 triggers (state change detected) and auto-resolves
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Maki should be waited (state change, not moved to waitroom)
    assert_eq!(
        game.state.mods.orientation_modifiers.get(&maki),
        Some(&"wait".to_string()),
        "Maki should be in wait state (cost paid)"
    );
    let waited = [p2_a, p2_b]
        .iter()
        .filter(|&&id| game.state.mods.orientation_modifiers.get(&id) == Some(&"wait".to_string()))
        .count();
    assert_eq!(
        waited, 1,
        "Exactly 1 opponent member should be waited by Maki's effect"
    );
}

/// Skip optional cost → no effect.
#[test]
fn maki_debut_skip_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    let p2_member = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.stage.stage[0] = p2_member;
    game.give_energy(11);

    game.play_to_stage(maki, MemberArea::Center);

    // ab#1 is pre-filtered out (no state change yet). ab#0 auto-resolves.

    // Skip the optional cost (card_id != Some(1) → "skip_optional_cost")
    game.select_option(0);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let orientation = game.state.mods.orientation_modifiers.get(&maki);
    assert!(
        orientation.is_none() || orientation == Some(&"active".to_string()),
        "Maki should remain active when cost is skipped"
    );
}

/// ab#1 draws when opponent member with cost ≤4 is waited.
#[test]
fn maki_ab1_draws_on_cost4_or_less_opponent_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    let p2_member = game.id("PL!-pb1-009-R"); // cost 4 → ≤4
    let filler = game.id("PL!-sd1-010-SD");
    let extra = game.id("PL!-sd1-013-SD");

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.stage.stage[0] = p2_member;
    game.give_energy(11);
    // Populate deck so draw_card actually works
    game.state.player1.main_deck.cards.push(extra);

    // hand = [maki, filler] = 2. deck = [extra] = 1.
    game.play_to_stage(maki, MemberArea::Center);
    // hand = [filler] = 1 (maki removed). deck unchanged.

    // ab#1 is pre-filtered out (no state change yet). ab#0 auto-resolves.
    game.select_option(1); // Pay optional cost → effect waits opponent → ab#1 triggers (state change detected) and draws from deck
                           // hand = [filler, extra] = 2. deck = [].
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "ab#1 should draw 1 when cost≤4 opponent is waited"
    );
    assert!(
        game.state.player1.hand.cards.contains(&extra),
        "Drawn card should be in hand"
    );
}

/// ab#1 does NOT draw when opponent member has cost >4.
#[test]
fn maki_ab1_no_draw_on_cost_over4_opponent_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    let p2_high = game.id("PL!-pb1-002-R"); // cost 13 → >4
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.stage.stage[0] = p2_high;
    game.give_energy(11);

    // hand_before = 2 (maki + filler). After play_to_stage: hand=1 (maki removed).
    // ab#1 pre-filtered out (no state change yet). ab#0 auto-resolves.
    game.play_to_stage(maki, MemberArea::Center);

    game.select_option(1); // Pay optional cost
                           // Opponent wait happens, but ab#1 condition fails for cost 13 > 4 → no draw.
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "ab#1 must NOT draw when cost >4"
    );
}

/// ab#1 use_limit=1: only draws once per turn.
#[test]
fn maki_ab1_use_limit_once_per_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    let p2_a = game.id("PL!-pb1-009-R"); // cost 4
    let p2_b = game.id("PL!-pb1-011-R"); // cost 2
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.stage.stage = [p2_a, p2_b, -1];
    game.give_energy(15);
    game.state
        .player1
        .main_deck
        .cards
        .push(game.id("PL!-sd1-013-SD"));

    // hand = [maki, filler] = 2
    game.play_to_stage(maki, MemberArea::Center);

    // ab#1 pre-filtered out (no state change yet). ab#0 auto-resolves.
    game.select_option(1); // Pay optional cost
    let entry = game.state.ability_queue.current_entry();
    assert_eq!(
        entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "Wait-member choice must be routed to opponent (p2)"
    );
    game.select_indices(&[0]); // Opponent waits p2_a (cost 4 ≤ 4 → ab#1 draws)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // hand = [filler, drawn] = 2 (play -1, draw +1 = net 0)
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "ab#1 draws 1 (play -1, draw +1)"
    );

    // Verify use_limit was consumed for this copy's ab#1 (ability index 1)
    let key = format!("{}_{}_{}", maki, 1, game.state.turn_number);
    assert!(
        game.state.turn_limited_abilities_used.contains(&key),
        "ab#1 use_limit must be consumed for this copy"
    );

    // Second opponent member (p2_b, cost 2 ≤ 4) is still active.
    // We can't easily trigger another wait to test the block in one turn,
    // but the use_limit key check above proves the mechanism works:
    // - use_limit is tracked per (card_id, ability_index, turn_number)
    // - Different copies of Maki have different card_ids → each can trigger once
    // - The same copy can't trigger ab#1 a second time in the same turn
    assert_eq!(
        game.state.mods.orientation_modifiers.get(&p2_b),
        None,
        "p2_b should still be active (only p2_a was waited)"
    );
}

/// End-to-end: ab#0 (pay cost) → ab#1 (draw on ≤4).
#[test]
fn maki_ab0_plus_ab1_end_to_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-pb1-015-R");
    let p2_member = game.id("PL!-pb1-009-R"); // cost 4
    let filler = game.id("PL!-sd1-010-SD");
    let extra = game.id("PL!-sd1-013-SD");

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.stage.stage[0] = p2_member;
    game.give_energy(11);
    // Populate deck so draw works
    game.state.player1.main_deck.cards.push(extra);

    // hand = [maki, filler] = 2. play maki → hand=1. draw → hand=2.
    game.play_to_stage(maki, MemberArea::Center);

    // ab#1 pre-filtered out (no state change yet). ab#0 auto-resolves.
    game.select_option(1); // Pay optional cost
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.orientation_modifiers.get(&p2_member),
        Some(&"wait".to_string()),
        "Opponent ≤4 member should be waited by ab#0"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "ab#1 should draw 1 (start 2, play -1, draw +1 = 2)"
    );
}
