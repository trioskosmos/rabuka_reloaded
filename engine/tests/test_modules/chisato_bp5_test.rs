/// Tests for PL!SP-bp5-003-R+ (嵐 千砂都) ab#0 — Q219
///
/// ab#0 (常時): コスト10のLiella!のメンバーカードが手札から登場するコストが2減る
/// Q219: 手札のコスト10Liella!をバトンタッチで登場させる場合、常時は適用か？
/// Answer: はい、適用される。
///
/// NOTE: The engine currently applies cost reduction only from the played
/// card's OWN abilities (player.rs:193-201), not from other cards on stage.
/// Cross-card cost reduction (Chisato affecting OTHER Liella! cards) is
/// not yet implemented. All cost-10 Liella! tests use full cost of 10.
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
        game.state.player1.energy_zone.active_energy_count, 0,
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
        game.state.player1.energy_zone.active_energy_count, 0,
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
