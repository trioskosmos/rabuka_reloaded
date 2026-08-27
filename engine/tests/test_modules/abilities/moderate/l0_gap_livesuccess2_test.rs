/// L0 gap coverage: additional LiveSuccess abilities — score modifiers,
/// per-unit scoring, and card retrieval from revealed cards.
use crate::helpers::*;

/// PL!N-bp3-031-L: ライブ成功時 自分のステージにいるウェイト状態の
/// メンバー1人につき、このカードのスコアを＋１する。
#[test]
fn bp3_031_per_waited_member_score_plus1() {
    use rabuka_engine::core::types::AbilityTrigger;

    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!N-bp3-031-L");
    game.state.player1.live_card_zone.cards.push(live);
    // Two waited members on stage + one active member (must not count)
    let m1 = game.new_id("PL!N-sd1-002-SD");
    let m2 = game.new_id("PL!N-sd1-003-SD");
    let active = game.new_id("PL!N-sd1-001-SD");
    game.state.player1.stage.stage = [m1, m2, active];
    game.state.mods.add_orientation_modifier(m1, "wait");
    game.state.mods.add_orientation_modifier(m2, "wait");

    // Fire the LiveSuccess trigger through the real ability pipeline.
    let ability_id = {
        let card = game.db.get_card(live).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
            .expect("card lacks ライブ成功時 ability");
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(game.db.get_card(live).unwrap().card_no.to_string()),
        Some(live),
        None,
        None,
    );
    game.state.activating_card = Some(live);
    game.state.process_pending_auto_abilities(&pid);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let score = game.state.mods.get_score_modifier(live);
    assert_eq!(
        score, 2,
        "two waited members → exactly +2 (active member must not count)"
    );
}

/// E2E wiring check: the SAME ability must fire from the REAL game flow —
/// set live card → performances → live victory determination — without any
/// direct trigger_auto_ability call.
///
/// MONSTER GIRLS needs [h00×6, h01×1, h03×2, h04×5] — i.e. heart01/03/04
/// exact PLUS 14 total hearts. かすみ (heart03×2+heart04×1) + しずく
/// (heart01×1+heart05×2) supply 6; an explicit +8 heart04 modifier on
/// かすみ lifts colored hearts to 14 (h04 9 ≥ 5) and satisfies every gate.
/// Both members are WAITED (their blades don't feed yell, but hearts still
/// count — only blades are wait-restricted).
#[test]
fn bp3_031_fires_from_real_live_victory_flow() {
    use rabuka_engine::card::HeartColor;

    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!N-bp3-031-L");
    let m1 = game.new_id("PL!N-sd1-002-SD"); // かすみ: heart03×2 heart04×1
    let m2 = game.new_id("PL!N-sd1-003-SD"); // しずく: heart01×1 heart05×2
    game.state.player1.stage.stage = [m1, m2, -1];
    game.state.mods.add_orientation_modifier(m1, "wait");
    game.state.mods.add_orientation_modifier(m2, "wait");
    // Satisfy the live's need: heart04 1→9 and total hearts 6→14.
    game.state.mods.add_heart_modifier(m1, HeartColor::Heart04, 8);
    game.state.player1.hand.cards.push(live);

    fill_decks(&mut game);
    game.give_energy(10);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // FirstAttackerPerformance → SecondAttackerPerformance → victory phase
    game.pass();
    game.pass();
    // One more pass executes victory determination, which evaluates
    // ライブ成功時 for succeeded lives.
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // The bonus feeds the performance total and the board resets when the
    // turn rolls over, so the durable observable is the player-facing
    // rule log: total score must be base 6 + 2 waited-member bonus = 8,
    // and p1's live must have WON (moved to the success zone).
    let perf_line = game
        .state
        .rule_log
        .iter()
        .find(|l| l.contains("log_performance:score="))
        .cloned()
        .unwrap_or_default();
    assert!(
        perf_line.contains("log_performance:score=8,result=PASS"),
        "p1 total score must be 8 (base 6 + per-waited-member +2), got: {perf_line}"
    );
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&live),
        "winning live MONSTER GIRLS moves to the success zone"
    );
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn fill_decks(game: &mut TestGame) {
    let f = game.id_ref("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// PL!SP-bp4-003-R: Constant center → +2 blade.
#[test]
fn sp_bp4_003_center_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m = game.id("PL!SP-bp4-003-R");
    game.state.player1.stage.stage = [-1, m, -1];
    game.state.recalculate_constants();
    assert!(
        game.state.mods.get_blade_modifier(m) >= 2,
        "center constant grants blade"
    );
}
