use crate::helpers::*;
use rabuka_engine::card::HeartColor;

#[test]
fn ren_005_turn2_blocks_third_energy_placed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [ren, -1, -1];
    game.state.player1.energy_zone.cards.push(game.id("PL!-sd1-010-SD"));
    game.state.player1.energy_zone.set_active_count(1);
    // First energy placed - use energy_zone
    game.state.push_movement_event(-1, "energy_deck", "energy_zone", Some(ren), "p1", true);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    assert_eq!(game.state.mods.get_blade_modifier(ren), 1);
    // Second same turn (turn2 allows 2)
    game.state.push_movement_event(-1, "energy_deck", "energy_zone", Some(ren), "p1", true);
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    let second = game.state.mods.get_blade_modifier(ren);
    assert!(second == 1 || second == 2, "second placement should be 1 or 2, got {}", second);
    // Third same turn should be blocked (ターン2回)
    game.state.push_movement_event(-1, "energy_deck", "energy_zone", Some(ren), "p1", true);
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    let third = game.state.mods.get_blade_modifier(ren);
    assert!(third <= 2, "ターン2回 should block third, got {}", third);
}
