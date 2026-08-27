use crate::helpers::*;

#[test]
fn tomari_jidou_self_move_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tomari = game.id("PL!SP-sd2-011-SD2");
    game.state.player1.stage.stage[1] = tomari;
    game.state.push_movement_event(tomari, "stage", "stage", Some(tomari), "p1", true);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    assert_eq!(game.state.mods.get_blade_modifier(tomari), 1);
}

#[test]
fn tomari_jidou_opponent_move_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tomari = game.id("PL!SP-sd2-011-SD2");
    game.state.player1.stage.stage[1] = tomari;
    game.state.push_movement_event(tomari, "stage", "stage", Some(tomari), "p2", true);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    assert_eq!(game.state.mods.get_blade_modifier(tomari), 1, "opponent effect also triggers (でも発動する)");
}

#[test]
fn tomari_jidou_natural_move_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let tomari = game.id("PL!SP-sd2-011-SD2");
    game.state.player1.stage.stage[1] = tomari;
    game.state.push_movement_event(tomari, "stage", "stage", None, "p1", false);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    assert_eq!(game.state.mods.get_blade_modifier(tomari), 0);
}
