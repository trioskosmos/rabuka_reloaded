use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!S-bp3-016-N (国木田花丸) Q155: 常時: 自分の成功ライブカード置き場にある
/// カード1枚につき、ステージにいるこのメンバーのコストが+1される。
///
/// Test: 1 success card → play cost = base_cost + 1.
#[test]
fn success_zone_stage_cost_ability_deploy_spends_at_least_base_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hanamaru = game.id("PL!S-bp3-016-N");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Get base cost
    let card = game
        .db
        .get_card(hanamaru)
        .expect("Hanamaru card should exist");
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
    // Exact, not "<=": the old assertion was `active <= given - (base+1)`,
    // which a play that cost base+4, or one that cost nothing while some other
    // effect drained the zone, would both satisfy. The number spent is the
    // whole point of the card, so it is asserted as a number.
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        ((base_cost as u8) + 5) - expected_cost as u8,
        "exactly base + 1 energy consumed — one success_live_zone card means \
         cost +1 (base {})",
        base_cost
    );
}
