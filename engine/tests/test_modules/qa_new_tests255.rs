use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;

/// Q255: When Live Start or Live Success resolves, if the member with that ability
/// has moved from the Center area, does Dancing stars on me!'s auto ability still trigger?
/// Answer: Yes.
///
/// Dancing stars on me! (PL!-bp6-020-L) has two auto abilities:
///   ab#0: When a μ's member in center resolves Live Start → position change that member
///   ab#1: When a μ's member in center resolves Live Success → if moved this turn → +1 score
///
/// The test ab#1 verifies that still fires even after ab#0 position-changed the
/// member out of center: ab#1's has_moved condition should check the member card,
/// not the live card's own movement.
#[test]
fn q255_dancing_stars_live_success_after_position_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dancing_stars = game.id("PL!-bp6-020-L");
    let honoka = game.id("PL!-bp6-001-R\u{ff0b}"); // μ's, has Live Start & Live Success
    let _filler = game.id("PL!-sd1-010-SD");

    // Stage: Honoka in center
    game.state.player1.stage.stage = [-1, honoka, -1];

    // Put Dancing stars on me! in live card zone (so it counts as "live card")
    game.state.player1.live_card_zone.cards.push(dancing_stars);

    // Give enough energy
    game.give_energy(10);

    let player_id = game.state.player1.id.clone();

    // --- Step 1: Trigger Live Start, then trigger each_time(LIVE_START) ---
    // This simulates the Live Start phase: Honoka's Live Start resolves,
    // then Dancing stars' ab#0 fires → position change Honoka out of center.

    // First, trigger all AUTO abilities (Live Start etc.)
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Now trigger each_time for LIVE_START (simulating post-LiveStart phase)
    TurnEngine::trigger_each_time_abilities(
        &mut game.state,
        &player_id,
        rabuka_engine::triggers::LIVE_START,
        None,
    );
    game.state.process_pending_auto_abilities(&player_id);
    // Drain the position change choice if there is one
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // After ab#0 fires, Honoka should no longer be in center
    let center_card = game.state.player1.stage.stage[1];
    assert_ne!(
        center_card, honoka,
        "Honoka should no longer be in center after ab#0 position change"
    );
    let on_stage = game.state.player1.stage.stage.iter().any(|&c| c == honoka);
    assert!(
        on_stage,
        "Honoka should still be on stage after position change"
    );

    // Verify position change was tracked
    assert!(
        game.state.position_change_occurred_this_turn,
        "position_change_occurred_this_turn should be true"
    );
    assert!(
        game.state.has_card_moved_this_turn(honoka),
        "Honoka should be tracked as moved this turn"
    );

    // --- Step 2: Trigger Live Success, then trigger each_time(LIVE_SUCCESS) ---
    // Set up stage hearts so Live Success conditions pass.
    // Dancing stars on me! needs: heart01=2, heart03=2, heart06=2, heart0=6 (total 12)
    // Honoka provides heart03=1 at center, but was moved. Give enough hearts.
    use rabuka_engine::card::HeartColor;
    use std::collections::HashMap;
    let mut heart_map = HashMap::new();
    heart_map.insert(HeartColor::Heart01, 2);
    heart_map.insert(HeartColor::Heart03, 2);
    heart_map.insert(HeartColor::Heart06, 2);
    heart_map.insert(HeartColor::Heart00, 6);
    let hearts = rabuka_engine::card::BaseHeart { hearts: heart_map };
    game.state.player1.stage_hearts = Some(hearts);

    // Trigger Live Success abilities (Honoka's Live Success)
    TurnEngine::trigger_live_success_abilities(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Now trigger each_time for LIVE_SUCCESS (Dancing stars' ab#1)
    TurnEngine::trigger_each_time_abilities(
        &mut game.state,
        &player_id,
        rabuka_engine::triggers::LIVE_SUCCESS,
        None,
    );
    game.state.process_pending_auto_abilities(&player_id);
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // --- Verification ---
    // ab#1 should have applied score +1 to Dancing stars on me!
    let score_mod = game.state.mods.score_modifiers.get(&dancing_stars);
    assert!(
        score_mod.is_some(),
        "Score modifier should exist on Dancing stars on me!"
    );
    let total = score_mod.unwrap().total();
    assert_eq!(
        total, 1,
        "Score modifier should be exactly +1, got {}",
        total
    );
}
