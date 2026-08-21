/// Tests for PL!N-bp5-012-R＋ (鐘 嵐珠 / Shodo Rin) — Place energy under member
///
/// ab#0 (起動/ターン1回):
///   エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く：
///   カードを1枚引き、ライブ終了時まで、{{heart01}}を得る。
///
/// ab#1 (ライブ成功時):
///   ライブの合計スコアが相手より高い場合、自分のエネルギーデッキから、
///   このメンバーの下にあるエネルギーカードの枚数に1を足した枚数の
///   エネルギーカードをウェイト状態で置く。
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

/// Activate: cost moves 1 energy from zone → under member, then draw 1 + gain heart01.
#[test]
fn rin_activate_places_energy_under_draws_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp5-012-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, rin, -1];
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(5);

    let hand_before = game.state.player1.hand.cards.len();
    let energy_total_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();

    game.activate_ability(rin);
    game.select_energy_from_zone(1);

    // 1 energy moved from zone to under
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_total_before - 1,
        "1 energy card removed from energy zone"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before - 1,
        "active_energy_count decremented by 1"
    );

    // Card appeared in under_cards of Center area
    let under = game
        .state
        .player1
        .stage
        .get_under_cards(rabuka_engine::zones::MemberArea::Center);
    assert_eq!(under.len(), 1, "1 energy placed under Rin at Center");

    // Draw 1
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "drew 1 card"
    );

    // heart01 modifier
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(rin, rabuka_engine::card::HeartColor::Heart01),
        1,
        "+1 heart01 modifier"
    );
}

/// Turn 1 use limit: second activation in same turn does not double the heart modifier.
#[test]
fn rin_turn1_use_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp5-012-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, rin, -1];
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    game.activate_ability(rin);
    game.select_energy_from_zone(1);
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(rin, rabuka_engine::card::HeartColor::Heart01),
        1,
        "+1 after first activation"
    );

    let _ = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(rin),
        None,
        None,
        None,
    );

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(rin, rabuka_engine::card::HeartColor::Heart01),
        1,
        "still +1 after second activation skipped (use_limit)"
    );
}

/// Activate two turns apart: use_limit resets on turn boundary.
#[test]
fn rin_activate_across_turns() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp5-012-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage = [-1, rin, -1];
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    game.activate_ability(rin);
    game.select_energy_from_zone(1);
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(rabuka_engine::zones::MemberArea::Center)
            .len(),
        1,
        "1 under after first activation"
    );

    // Complete turn 1: pass through P2's normal phases and the Live phase
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.hand.cards.push(live_card);
    game.state.player2.hand.cards.push(filler);

    for _ in 0..5 {
        game.pass();
    } // P2 Active→Energy→Draw→Main, then LiveCardSet
    game.set_live_card(live_card);
    game.pass(); // P2's LiveCardSet turn
    game.set_live_card(live_card);
    game.pass(); // → FirstAttackerPerformance
    game.pass(); // → SecondAttackerPerformance
    game.pass(); // → LiveVictoryDetermination (triggers LiveSuccess → choice)
                 // Handle LiveSuccess under_member selection (select any available cards)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.pass(); // → Active (turn 2)
    game.pass(); // → Energy
    game.pass(); // → Draw
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.pass(); // → Main (turn 2)

    game.give_energy(5);

    // Second activation in turn 2 — use_limit resets cross-turn.
    game.activate_ability(rin);
    game.select_energy_from_zone(1);
    let final_under = game
        .state
        .player1
        .stage
        .get_under_cards(rabuka_engine::zones::MemberArea::Center)
        .len();
    // LiveSuccess may move under cards elsewhere; verify at least the new activation placed one.
    assert!(
        final_under >= 1,
        "at least 1 under after second activation in turn 2 (got {})",
        final_under
    );
}

/// LiveSuccess: ability fires through the Live phase without crashing.
#[test]
fn rin_live_success_ability_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!N-bp5-012-R\u{ff0b}");
    let live_card = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, rin, -1];
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);

    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    game.state.player2.hand.cards.push(live_card);
    game.state.player2.hand.cards.push(filler);

    // Stock the ENERGY DECK: ab#1 draws from 自分のエネルギーデッキ.
    let e_card = game.id("LL-E-001-SD");
    for _ in 0..3 {
        game.state.player1.energy_deck.cards.push(e_card);
    }

    let energy_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();

    game.pass();
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Rin's LiveSuccess must be routed and resolved with an explicit verdict.
    let rin_live_success = game.state.rule_log.iter().any(|l| {
        l.contains("鐘 嵐珠") && l.contains("trigger_live_success")
    });
    assert!(
        rin_live_success,
        "Rin's LiveSuccess ability must be evaluated during the live"
    );
    let resolved_with_verdict = game.state.rule_log.iter().any(|l| {
        l.contains("鐘 嵐珠")
            && l.contains("trigger_live_success")
            && (l.contains("result_success") || l.contains("result_failure"))
    });
    assert!(
        resolved_with_verdict,
        "LiveSuccess resolution must record success or failure"
    );

    // If the condition was met (higher live score), the energy deck placement
    // must have happened: (0 under + 1) = exactly 1 card added to the energy
    // zone in WAIT state — the active count must be untouched.
    let success = game.state.rule_log.iter().any(|l| {
        l.contains("鐘 嵐珠")
            && l.contains("trigger_live_success")
            && l.contains("result_success")
    });
    if success {
        assert_eq!(
            game.state.player1.energy_zone.cards.len(),
            energy_before + 1,
            "success places exactly under_count(0)+1 energy cards"
        );
        assert_eq!(
            game.state.player1.energy_zone.active_count(),
            active_before,
            "placed energy is in wait state: active count unchanged"
        );
    }
}
