/// Tests for 村野さやか (PL!HS-bp1-002-R) — Activation ability:
///
/// {{kidou.png|起動}}このメンバーをステージから控え室に置く：
/// 自分の控え室からコスト15以下の「蓮ノ空」のメンバーカードを1枚、
/// このメンバーが置かれていたエリアに登場させる。
///
/// Q63: Ability debuts don't require paying the card's cost
/// Q80: Can debut to the area vacated by this same ability this turn
use crate::helpers::*;
/// Q63: Debuting via ability effect does not require paying the card's
/// cost. Activate 村野さやか with a matching 蓮ノ空 member in discard,
/// verify it appears on stage without requiring extra energy.
#[test]
fn sayaka_q63_ability_debut_no_cost_payment() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sayaka = game.id("PL!HS-bp1-002-R");
    let hasuno_member = game.id("PL!HS-bp2-004-R"); // 藤島慈, cost=2, group=蓮ノ空

    // Stage: 村野さやか in Center
    game.state.player1.stage.stage[1] = sayaka;

    // Hand: nothing needed
    // Discard: a 蓮ノ空 member
    game.state.player1.waitroom.cards.push(hasuno_member);

    // Energy: 2 for the activation cost
    game.give_energy(2);

    game.activate_ability(sayaka);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "Q63: 2E should be fully consumed by activation cost"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&sayaka),
        "sayaka should be in waitroom after activation cost"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], hasuno_member,
        "Q63: 蓮ノ空 member should appear in sayaka's former area without cost payment"
    );
}

/// Q80: Can debut to an area that was vacated by this same ability this
/// turn. The self_cost puts sayaka in waitroom, freeing the area, and
/// the effect debuts a new member to the same area — all within one
/// ability activation.
#[test]
fn sayaka_q80_debut_to_vacated_area_same_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sayaka = game.id("PL!HS-bp1-002-R");
    let hasuno_member = game.id("PL!HS-bp2-004-R");

    game.state.player1.stage.stage[1] = sayaka;
    game.state.player1.waitroom.cards.push(hasuno_member);
    game.give_energy(2);

    game.activate_ability(sayaka);

    if game.has_pending_choice() {
        // Select the only eligible card in discard
        game.select_indices(&[0]);
    }

    // The area vacated by sayaka should now be filled by the new member
    assert_eq!(
        game.state.player1.stage.stage[1], hasuno_member,
        "Q80: New member should debut to the vacated area (same_area)"
    );
    assert!(
        !game.state.player1.stage.stage.contains(&sayaka),
        "sayaka should no longer be on stage"
    );
}

#[test]
fn sayaka_bp5_002_distinct_costs_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sayaka_bp5 = game.id("PL!HS-bp5-002-R＋");

    // Place 3 members on stage: Sayaka (who owns the constant ability)
    // plus two cost-2 fillers. Costs [15, 2, 2] are NOT all distinct.
    // LeftSide: Sayaka (cost 15, ability owner)
    let member_cost2_a = game.id("PL!-sd1-002-SD"); // base cost 2
    let member_cost2_b = game.id("PL!HS-bp2-004-R"); // 藤島慈, base cost 2

    // Place them on stage
    game.state.player1.stage.stage[0] = sayaka_bp5;
    game.state.player1.stage.stage[1] = member_cost2_a;
    game.state.player1.stage.stage[2] = member_cost2_b;

    // Recalculate
    game.state.recalculate_constants();

    // Costs [15, 2, 2] are NOT all distinct (cost 2 appears twice) → no blade.
    assert_eq!(
        game.state.mods.get_blade_modifier(sayaka_bp5),
        0,
        "Should NOT gain blade modifier since costs 15, 2, 2 have a duplicate"
    );

    // Add +2 cost modifier to member_cost2_b: cost goes from 2 → 4.
    // Now costs are [15, 2, 4] → all distinct!
    game.state.mods.add_cost_modifier(member_cost2_b, 2);

    // Recalculate constant modifiers
    game.state.recalculate_constants();

    // Should now gain blade modifier since [15, 2, 4] are all distinct.
    assert_eq!(
        game.state.mods.get_blade_modifier(sayaka_bp5),
        1,
        "Should gain 1 blade modifier since costs 15, 2, 4 are distinct"
    );
}
