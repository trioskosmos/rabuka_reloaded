/// Tests for PL!SP-bp5-003-R+ (嵐 千砂都) ab#0 — Q219
///
/// ab#0 (常時): コスト10のLiella!のメンバーカードが手札から登場するコストが2減る
/// Q219: 手札のコスト10Liella!をバトンタッチで登場させる場合、常時は適用か？
/// Answer: はい、適用される。
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

/// Without cost reduction, a cost-10 Liella! member needs exactly 10 energy.
/// 10 works, 9 fails.
#[test]
fn chisato_bp5_q219_cost_10_needs_10_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let liella = game.id("PL!SP-bp2-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(liella);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = -1;
    game.give_energy(10);

    let r = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(liella),
        None,
        Some(rabuka_engine::zones::MemberArea::LeftSide),
        Some(false),
    );
    assert!(r.is_ok(), "Cost-10 needs 10 energy: {:?}", r);
    assert!(
        game.state.player1.stage.stage[0] == liella,
        "Member on stage"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(), 0,
        "Spent 10"
    );
}

/// 9 energy is NOT enough for a cost-10 member.
#[test]
fn chisato_bp5_q219_cost_10_fails_with_9() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let liella = game.id("PL!SP-bp2-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(liella);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = -1;
    game.give_energy(9);

    let r = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(liella),
        None,
        Some(rabuka_engine::zones::MemberArea::LeftSide),
        Some(false),
    );
    assert!(r.is_err(), "9 energy should NOT be enough for cost-10");
}

/// Chisato on stage → cost-10 Liella! needs only 8 energy (10 - 2).
#[test]
fn chisato_bp5_q219_cross_card_reduction_8_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let chisato = game.id("PL!SP-bp5-003-R\u{ff0b}");
    let liella = game.id("PL!SP-bp2-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    // Chisato on stage (cost-10 Liella! reducer)
    game.state.player1.stage.stage = [chisato, -1, -1];
    game.state.player1.hand.cards.push(liella);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(8); // 10 - 2 = 8

    let r = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(liella),
        None,
        Some(rabuka_engine::zones::MemberArea::Center),
        Some(false),
    );
    assert!(
        r.is_ok(),
        "Chisato on stage: cost-10 Liella! needs 8 energy: {:?}",
        r
    );
    assert!(
        game.state.player1.stage.stage[1] == liella,
        "Liella! on stage after cost reduction"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(), 0,
        "Spent 8 (reduced from 10)"
    );
}

/// Chisato on stage, but only 7 energy → NOT enough (needs 8).
#[test]
fn chisato_bp5_q219_cross_card_reduction_7_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let chisato = game.id("PL!SP-bp5-003-R\u{ff0b}");
    let liella = game.id("PL!SP-bp2-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [chisato, -1, -1];
    game.state.player1.hand.cards.push(liella);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(7);

    let r = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(liella),
        None,
        Some(rabuka_engine::zones::MemberArea::Center),
        Some(false),
    );
    assert!(
        r.is_err(),
        "Chisato on stage: 7 energy should NOT be enough for cost-10 (needs 8)"
    );
}

/// Verify that PL!SP-bp5-003-P (cost 17) costs exactly 17 to play from hand,
/// and does not get its own -2 cost reduction (which is only for cost-10 members).
#[test]
fn chisato_bp5_self_cost_not_reduced() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let chisato_promo = game.id("PL!SP-bp5-003-P");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(chisato_promo);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = -1;

    // 16 energy should NOT be enough
    game.give_energy(16);
    let r = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(chisato_promo),
        None,
        Some(rabuka_engine::zones::MemberArea::LeftSide),
        Some(false),
    );
    assert!(
        r.is_err(),
        "16 energy should fail to play 17-cost card: {:?}",
        r
    );

    // 17 energy should work
    game.give_energy(1); // 16 + 1 = 17
    let r = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(chisato_promo),
        None,
        Some(rabuka_engine::zones::MemberArea::LeftSide),
        Some(false),
    );
    assert!(r.is_ok(), "17 energy is enough to play Chisato: {:?}", r);
}

/// Chisato promo on stage → cost-10 Liella! needs only 8 energy (10 - 2).
#[test]
fn chisato_promo_ab0_cross_card_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let chisato = game.id("PL!SP-bp5-003-P");
    let liella = game.id("PL!SP-bp2-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    // Chisato on stage (cost-10 Liella! reducer)
    game.state.player1.stage.stage = [chisato, -1, -1];
    game.state.player1.hand.cards.push(liella);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(8); // 10 - 2 = 8

    let r = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(liella),
        None,
        Some(rabuka_engine::zones::MemberArea::Center),
        Some(false),
    );
    assert!(
        r.is_ok(),
        "Chisato promo on stage: cost-10 Liella! needs 8 energy: {:?}",
        r
    );
    assert!(
        game.state.player1.stage.stage[1] == liella,
        "Liella! on stage after cost reduction"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(), 0,
        "Spent 8 (reduced from 10)"
    );
}

/// LiveStart Center ability: activates all Liella! members and all energy.
#[test]
fn chisato_promo_ab1_live_start() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chisato = game.id("PL!SP-bp5-003-P");
    let liella = game.id("PL!SP-bp2-006-R\u{ff0b}"); // Kanon (Liella!)
    let filler = game.id("PL!-sd1-010-SD");

    // Chisato must be in the CENTER area for her Center ability to activate
    game.state.player1.stage.stage = [liella, chisato, filler];

    // Set Liella member and Chisato to wait (rested) state
    game.state
        .mods
        .orientation_modifiers
        .insert(liella, "wait".to_string());
    game.state
        .mods
        .orientation_modifiers
        .insert(chisato, "wait".to_string());

    // Give 3 energy, but make them all rested
    game.give_energy(3);
    game.state.player1.energy_zone.set_active_count(0);

    // Set up deck and live card to trigger LiveStart phase
    let live_card = game.id("PL!-sd1-019-SD"); // Valid live card
    game.state.player1.hand.cards.push(live_card);
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Pass turns to enter live card set phase
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);

    // Pass to run LiveStart phase and trigger abilities
    game.pass();
    game.pass();

    // Since Chisato's ab#1 triggers automatically and is not optional,
    // the members and energy should be activated.
    let liella_ori = game.state.mods.orientation_modifiers.get(&liella).cloned();
    let chisato_ori = game.state.mods.orientation_modifiers.get(&chisato).cloned();
    assert!(
        liella_ori != Some("wait".to_string()),
        "Liella! member should be active"
    );
    assert!(
        chisato_ori != Some("wait".to_string()),
        "Chisato should be active"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(), 3,
        "All 3 energy should be active"
    );
}
