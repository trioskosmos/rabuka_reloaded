use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// Extra edge for Keke sd2-002 jidou: effect_only true vs false and cause player
#[test]
fn keke_jidou_effect_only_false_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = game.id("PL!SP-sd2-002-SD2");
    game.state.player1.stage.stage[1] = keke;
    // Natural move (effect_only false) should NOT trigger
    game.state.push_movement_event(keke, "stage", "stage", None, "p1", false);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    assert_eq!(game.state.mods.get_heart_modifier(keke, HeartColor::Heart06), 0);
}

#[test]
fn keke_jidou_self_effect_move_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = game.id("PL!SP-sd2-002-SD2");
    game.state.player1.stage.stage[1] = keke;
    game.state.push_movement_event(keke, "stage", "stage", Some(keke), "p1", true);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    assert_eq!(game.state.mods.get_heart_modifier(keke, HeartColor::Heart06), 1);
}
