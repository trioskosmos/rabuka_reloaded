use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn sd2_002_natural_move_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = game.id("PL!SP-sd2-002-SD2");
    game.state.player1.stage.stage[1] = keke;
    // Natural move (effect_only false) should NOT trigger the jidou
    game.state.push_movement_event(keke, "stage", "stage", None, "p1", false);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    let heart_mod = game.state.mods.get_heart_modifier(keke, HeartColor::Heart06);
    assert_eq!(heart_mod, 0, "natural move (effect_only false) should not trigger jidou");
}

#[test]
fn sd2_002_turn_limit_blocks_second_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = game.id("PL!SP-sd2-002-SD2");
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, keke);
    game.add_to_stage(rabuka_engine::zones::MemberArea::LeftSide, filler);
    game.give_energy(4);
    // First activation
    game.activate_ability(keke);
    let actions = game.generated_actions();
    let left_idx = actions.iter().position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("left")).unwrap();
    game.select_generated(left_idx);
    game.drain_auto_ability_choices();
    let first = game.state.mods.get_heart_modifier(keke, HeartColor::Heart06);
    assert_eq!(first, 1);
    // Second activation same turn should be blocked by ターン1回
    game.give_energy(2);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        game.activate_ability(keke);
    }));
    let second = game.state.mods.get_heart_modifier(keke, HeartColor::Heart06);
    assert_eq!(second, 1, "ターン1回 should block second heart06, got {}", second);
}
