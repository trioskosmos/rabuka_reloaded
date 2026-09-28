use crate::helpers::*;

#[test]
fn own_effect_energy_placement_blades_remain_at_most_two_after_three_events() {
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
    // Second same turn — ターン2回 allows it, and the re-scan guard must not
    // veto it. Assert the real number, not "1 or 2": that leniency is exactly
    // where the re-scan-guard leak lived (docs/JIDOU_COMBINATION_WORK.md §6,
    // Bug B), and 恋 could never use her second allowance with no test noticing.
    game.state.push_movement_event(-1, "energy_deck", "energy_zone", Some(ren), "p1", true);
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    let second = game.state.mods.get_blade_modifier(ren);
    assert_eq!(
        second, 2,
        "the SECOND own-effect energy placement in the same turn must grant again \
         (ターン2回). This previously asserted '1 or 2', which is precisely the \
         window the re-scan-guard leak hid in."
    );
    // Third same turn should be blocked (ターン2回)
    game.state.push_movement_event(-1, "energy_deck", "energy_zone", Some(ren), "p1", true);
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    let third = game.state.mods.get_blade_modifier(ren);
    assert_eq!(
        third, 2,
        "ターン2回 is a CEILING: a third own-effect placement in the same turn must \
         grant nothing. `<= 2` would also pass if the second firing had silently \
         failed and the total never rose; the exact value pins both halves."
    );
}
