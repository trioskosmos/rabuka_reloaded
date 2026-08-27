/// Tests for PL!SP-pb1-010-R (ウィーン・マルガレーテ / Wien Margarete) — Cost increase
///
/// 常時:
///   自分のエネルギーが10枚以上ある場合、ステージにいるこのメンバーのコストを＋４する。
use crate::helpers::*;

/// Energy < 10 → play for base cost 4. On stage, no cost modifier.
#[test]
fn wien_low_energy_no_cost_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-pb1-010-R");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(wien);
    game.give_energy(9);

    game.play_to_stage(wien, rabuka_engine::zones::MemberArea::Center);

    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(remaining, 5, "spent 4 energy (base cost)");

    // Recalc should NOT add cost mod (9 < 10)
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(wien),
        0,
        "no cost modifier when energy < 10"
    );
}

/// Energy >= 10 → play for base cost 4. On stage, cost modifier +4 appears.
#[test]
fn wien_high_energy_cost_modifier_applied() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-pb1-010-R");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(wien);
    game.give_energy(10);

    game.play_to_stage(wien, rabuka_engine::zones::MemberArea::Center);

    // Play cost is base 4 (modifier is on-stage only)
    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(
        remaining, 6,
        "spent 4 energy (base cost, modifier on-stage only)"
    );

    // Recalc SHOULD add +4 cost mod (10 >= 10)
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(wien),
        4,
        "+4 cost modifier when energy >= 10"
    );
}

/// Modifier appears at 10 energy, disappears when dropping to 9.
#[test]
fn wien_cost_modifier_dynamic() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-pb1-010-R");
    let energy_id = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [-1, wien, -1];
    game.give_energy(9);

    // At 9 energy: no modifier
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(wien),
        0,
        "no modifier at 9 energy"
    );

    // Add 1 more → 10 energy
    game.state.player1.energy_zone.cards.push(energy_id);
    game.state.player1.energy_zone.add_active(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(wien),
        4,
        "+4 modifier at 10 energy"
    );

    // Remove 1 → back to 9
    game.state.player1.energy_zone.cards.pop();
    game.state.player1.energy_zone.sub_active(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(wien),
        0,
        "modifier removed when energy drops back to 9"
    );
}

/// Modifier cleared when card leaves stage.
#[test]
fn wien_cost_modifier_cleared_on_leave() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wien = game.id("PL!SP-pb1-010-R");

    game.state.player1.stage.stage = [-1, wien, -1];
    game.give_energy(10);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(wien),
        4,
        "+4 modifier while on stage with 10 energy"
    );

    // Remove from stage
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.waitroom.cards.push(wien);
    game.state.mods.clear_all_for_card(wien);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(wien),
        0,
        "modifier cleared after card leaves stage"
    );
}
