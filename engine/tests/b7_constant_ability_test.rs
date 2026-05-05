/// Tests for 常時 (constant) abilities — continuous effects that don't trigger.

mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!-bp3-002-R (絢瀬絵里) ab#1 Q144: 「常時」自分のステージにいる
/// ウェイト状態のメンバー1人につき、ブレードを得る。
///
/// For each wait member on your stage, gain 1 blade.
/// Test: recalculate constant modifiers → blade is added.
#[test]
fn eri_constant_blade_per_wait_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let eri = game.id("PL!-bp3-002-R");
    let friend = game.id("PL!-sd1-010-SD");

    // Stage: eri center, friend left
    game.state.player1.stage.stage = [friend, eri, -1];
    game.state.add_orientation_modifier(friend, "wait");

    // Recalculate constant blade modifiers
    game.state.recalculate_constant_blade_modifiers();

    // The constant ability should have added a blade modifier
    let blade_mod = game.state.get_blade_modifier(eri);
    assert!(blade_mod >= 1,
        "Constant ability: 1 wait member → at least 1 blade, got {}", blade_mod);
}

/// PL!S-bp3-016-N (国木田花丸) Q155: 常時: 自分の成功ライブカード置き場にある
/// カード1枚につき、ステージにいるこのメンバーのコストが+1される。
///
/// Test: 1 success card → play cost = base_cost + 1.
#[test]
fn hanamaru_constant_cost_per_success_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hanamaru = game.id("PL!S-bp3-016-N");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Get base cost
    let card = db.get_card(hanamaru).unwrap();
    let base_cost = card.cost.unwrap_or(0);

    // Place a card in success live zone → cost +1
    game.state.player1.success_live_card_zone.cards.push(live);

    // Hand: hanamaru + filler
    game.add_to_hand(hanamaru);
    game.add_to_hand(filler);

    // Energy: base_cost + 1 + buffer
    game.give_energy(base_cost as usize + 5);

    // Play hanamaru to stage — cost should be base + 1
    game.play_to_stage(hanamaru, MemberArea::Center);

    // Verify energy was consumed: active_energy_count decreased
    // Base + 1 should be consumed
    let expected_cost = base_cost + 1;
    assert!(game.state.player1.energy_zone.active_energy_count <= (base_cost as usize + 5) - expected_cost as usize,
        "Should consume base + 1 energy (success_live_zone card increases cost)");
}
