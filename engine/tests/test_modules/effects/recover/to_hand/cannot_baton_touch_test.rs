/// Test for cannot_baton_touch restriction filtering
/// This test verifies that the fix in game_setup.rs correctly filters out baton touch actions
/// when a member has the cannot_baton_touch restriction.
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_cannot_baton_touch_restriction_blocks_action() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // LL-bp2-001-R+ (唐可可＆平安名すみれ＆米女メイ) has cannot_baton_touch restriction:
    // "このメンバーはバトンタッチで控え室に置かれない。"
    let you = game.id("LL-bp2-001-R\u{ff0b}");
    let filler_member = game.id("PL!-sd1-001-SD");

    // Place protected card on stage
    game.state.player1.stage.stage = [you, -1, -1];

    // Add member to hand for baton touch attempt
    game.state.player1.hand.cards.push(filler_member);
    game.give_energy(5);

    // Try baton touch: play filler to same area (LeftSide = index 0)
    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(filler_member),
        None,
        Some(MemberArea::LeftSide),
        Some(true),
    );
    assert!(
        result.is_err(),
        "Baton touch should be rejected for member with cannot_baton_touch restriction"
    );
}

/// Positive control: the SAME baton-touch play against a member WITHOUT the
/// restriction succeeds. Without this, the blocking test above cannot tell
/// the restriction apart from generic slot/cost rejection.
#[test]
fn test_baton_touch_succeeds_without_restriction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let unprotected = game.id("PL!-sd1-001-SD"); // vanilla member, no restrictions
    let filler_member = game.id("PL!-sd1-002-SD");

    game.state.player1.stage.stage = [unprotected, -1, -1];
    game.state.player1.hand.cards.push(filler_member);
    // Enough to pay the arriving member's cost minus the replaced cost.
    game.give_energy(30);

    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(filler_member),
        None,
        Some(MemberArea::LeftSide),
        Some(true),
    );
    assert!(
        result.is_ok(),
        "identical baton touch must succeed without the restriction: {:?}",
        result.err()
    );
    assert_eq!(
        game.state.player1.stage.stage[0], filler_member,
        "arriving member replaces the unprotected one"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&unprotected),
        "replaced member went to the waitroom (baton touch happened)"
    );
}
