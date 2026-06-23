use crate::helpers::*;
use rabuka_engine::card::HeartColor;

// ═══════════════════════════════════════════════════════════════
// #16 — Hazuki (PL!SP-bp4-016-N): on_energy_placed_each_time
//
// カードの効果によって自分のエネルギー置き場にエネルギーカードが
// 置かれるたび、ライブ終了時まで、heart06を得る。
// (相手のカードの効果でも発動する。)
//
// Stage card (member). each_time auto ability.
// trigger_condition: comparison_condition
//   location: "energy_zone", resource_type: "energy",
//   card_type: "energy_card"
// Condition: compares count of energy cards ≥ 0.
// Effect: gain_resource(heart06, duration=live_end)
//
// The scan gate at abilities.rs checks for push_movement_event energy events,
// but trigger_auto_abilities_for_player called directly in tests
// bypasses that gate.  In real gameplay the gate prevents
// re-enqueue between effects.  Tests must place an energy card
// in the zone so the default comparison (≥ 1, or ≥ 0 with an
// explicit operator) passes.
// ═══════════════════════════════════════════════════════════════

fn setup_hazuki(game: &mut TestGame) -> i16 {
    let hazuki = game.id("PL!SP-bp4-016-N");
    game.state.player1.stage.stage = [-1, hazuki, -1];
    let energy_card = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(energy_card);
    hazuki
}

fn heart06_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state
        .mods
        .get_heart_modifier(card_id, HeartColor::Heart06)
}

fn trigger_auto(v: &mut TestGame) {
    let pid = v.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut v.state, &pid);
    v.state.process_pending_auto_abilities(&pid);
}

/// Energy placed by own effect → triggers heart06 gain.
#[test]
fn hazuki_energy_by_own_effect_triggers() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hazuki = setup_hazuki(&mut v);
    v.state
        .push_movement_event(-1, "energy_deck", "energy", None, "player1", true);

    trigger_auto(&mut v);

    assert_eq!(
        heart06_mod(&v, hazuki),
        1,
        "Hazuki gains heart06 when energy placed by own effect"
    );
}

/// Energy placed by opponent's effect → STILL triggers (card text says
/// "相手のカードの効果でも発動する" — also activates with opponent's card effects).
#[test]
fn hazuki_energy_by_opponent_effect_triggers() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hazuki = setup_hazuki(&mut v);
    v.state
        .push_movement_event(-1, "energy_deck", "energy", None, "player2", true);

    trigger_auto(&mut v);

    assert_eq!(
        heart06_mod(&v, hazuki),
        1,
        "Hazuki gains heart06 even when energy placed by opponent per card text"
    );
}

/// Energy phase draw — no push_movement_event for energy.
/// The each_time guard at abilities.rs blocks enqueue when
/// batch_movements is clear, even though the
/// comparison_condition would pass (energy card in zone).
#[test]
fn hazuki_energy_phase_no_effect_flag() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hazuki = setup_hazuki(&mut v);
    v.state.batch_movements.clear();

    trigger_auto(&mut v);

    // Energy placed by phase action (not a card effect) — Hazuki should NOT trigger
    // because her text says "カードの効果によって" (by a card effect).
    // The each_time guard checks for a push_movement_event energy for comparison conditions
    // on energy_zone.
    assert_eq!(
        heart06_mod(&v, hazuki),
        0,
        "Hazuki should NOT trigger when energy placed by phase action, not a card effect"
    );
}
