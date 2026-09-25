use crate::helpers::*;

#[test]
fn distinct_stage_costs_constant_blade_uses_modified_cost() {
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
