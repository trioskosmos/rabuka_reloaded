use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// PL!S-PR-029-PR 渡辺 曜:
/// 常時: 自分か相手のステージにコスト13以上のメンバーがいる場合、ブレード×2を得る。
///
/// Cost 13+ on either stage → gain 2 blade.
/// The card itself is cost 9. When it's alone on stage, condition should NOT be met.
#[test]
fn cost9_alone_does_not_meet_cost13_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let you = game.id("PL!S-PR-029-PR"); // cost 9
    game.state.player1.stage.stage = [you, -1, -1];

    // Process constant abilities
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::process_constant_abilities(&mut game.state, &pid);

    // Condition "self or opponent stage has cost >= 13 member" should FAIL
    // because the only card on stage is cost 9, which is less than 13.
    let blade = game.state.mods.get_blade_modifier(you);
    assert_eq!(
        blade, 0,
        "Cost-9 card alone should NOT trigger cost>=13 condition (got blade={})",
        blade
    );
}

/// When a cost-13+ card is on either player's stage, the condition passes.
#[test]
fn cost13_on_opponent_stage_meets_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let you = game.id("PL!S-PR-029-PR"); // cost 9
    let cost13_card = game.id("PL!S-sd1-001-SD"); // need to find a cost-13+ card

    game.state.player1.stage.stage = [you, -1, -1];
    game.state.player2.stage.stage = [cost13_card, -1, -1];

    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::process_constant_abilities(&mut game.state, &pid);

    let blade = game.state.mods.get_blade_modifier(you);
    assert_eq!(
        blade, 2,
        "Cost-13+ on opponent stage should grant exactly +2 (got {blade}); \
         a >= bound would mask over-application"
    );
}
