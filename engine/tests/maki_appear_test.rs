/// Tests for PL!-PR-015-PR (西木野真姫 / Maki Nishikino) — Appear ability via baton touch
///
/// 登場:
///   このメンバーよりコストが低いメンバーからバトンタッチして登場した場合、
///   自分の手札からコスト4以下のメンバーカードを1枚ステージに登場させてもよい。

mod helpers;
use helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Baton touch from lower-cost (cost 4 < 17) → cost≤4 member from hand appears on stage.
#[test]
fn maki_baton_touch_places_cheap_member_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-PR-015-PR");
    let cheap = game.id("PL!SP-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, filler, -1];
    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(cheap);
    game.give_energy(17);

    TurnEngine::execute_main_phase_action(
        &mut game.state, &ActionType::PlayMemberToStage,
        Some(maki), None, Some(MemberArea::Center), Some(true),
    ).expect("baton touch");

    assert!(game.state.player1.stage.stage.contains(&maki),
        "Maki on stage after baton touch");
    assert!(game.state.player1.stage.stage.contains(&cheap),
        "cheap member appeared on stage");
    assert!(!game.state.player1.hand.cards.contains(&cheap),
        "cheap member removed from hand");
}

/// Equal-cost baton touch (17 == 17) → condition fails → no appear.
#[test]
fn maki_equal_cost_no_appear() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki_hand = game.new_id("PL!-PR-015-PR");
    let maki_stage = game.id("PL!-PR-015-PR");

    game.state.player1.stage.stage = [-1, maki_stage, -1];
    game.state.player1.hand.cards.push(maki_hand);
    game.give_energy(17);

    TurnEngine::execute_main_phase_action(
        &mut game.state, &ActionType::PlayMemberToStage,
        Some(maki_hand), None, Some(MemberArea::Center), Some(true),
    ).expect("baton touch");

    assert!(game.state.player1.stage.stage.contains(&maki_hand),
        "new Maki on stage after baton touch");
    assert_eq!(game.state.player1.stage.stage.iter().filter(|&&c| c != -1).count(), 1,
        "only 1 member on stage (old Maki replaced, no appear)");
}

/// Play to empty area (no baton touch) → ability not triggered.
#[test]
fn maki_no_baton_touch_no_appear() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-PR-015-PR");
    let cheap = game.id("PL!SP-sd1-019-SD");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(cheap);
    game.give_energy(17);

    game.play_to_stage(maki, MemberArea::Center);

    assert!(game.state.player1.hand.cards.contains(&cheap),
        "cheap member stays in hand (no baton touch)");
}

/// Baton touch lower cost, but no eligible card in hand → optional ability skips gracefully.
#[test]
fn maki_no_eligible_hand_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-PR-015-PR");
    let too_expensive = game.id("PL!-sd1-014-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, filler, -1];
    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(too_expensive);
    game.give_energy(17);

    TurnEngine::execute_main_phase_action(
        &mut game.state, &ActionType::PlayMemberToStage,
        Some(maki), None, Some(MemberArea::Center), Some(true),
    ).expect("baton touch");

    assert!(game.state.player1.hand.cards.contains(&too_expensive),
        "cost-9 card stays in hand");
}
