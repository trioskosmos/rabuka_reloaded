/// Comprehensive edge cases for PL!SP-bp2-004 idx289
/// 常時: 自分のステージにいるメンバーのうち、センターエリアにいるメンバーが最も大きいコストを持つ場合、heart03を得る。
/// Previously 35 tests covered base permutations but missed waited, opponent isolation,
/// cost-modifier respect, dynamic re-evaluation and position-change.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn heart03(game: &TestGame, id: i16) -> i32 {
    game.state.mods.get_heart_modifier(id, HeartColor::Heart03)
}

// Waited side member still counts for highest-cost (engine loops over stage ids without state filter).
#[test]
fn sp_bp2_004_waited_side_still_counts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R"); // cost 9 at left
    let center_high = game.id("PL!SP-pb1-001-R"); // cost 11 center
    let right_high_waited = game.id("PL!SP-pb1-001-R"); // cost 11 right, but waited
    game.state.player1.stage.stage = [sumire, center_high, right_high_waited];
    // Mark right as waited — should still be considered, so center 11 ties with right 11 -> not strictly highest -> no heart
    game.state.mods.add_orientation_modifier(right_high_waited, "wait");
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 0, "center 11 ties with waited right 11 -> no heart (waited still counts)");

    // Now make right waited but lower cost: right low waited should not block
    let right_low_waited = game.id("PL!-sd1-010-SD"); // cost 4
    game.state.player1.stage.stage = [sumire, center_high, right_low_waited];
    game.state.mods.add_orientation_modifier(right_low_waited, "wait");
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 1, "center 11 > left 9 and waited right 4 -> heart");
}

// Opponent stage does NOT affect condition (only own stage).
#[test]
fn sp_bp2_004_opponent_center_high_does_not_block() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let center_high = game.id("PL!SP-pb1-001-R"); // 11
    let right_low = game.id("PL!-sd1-010-SD"); // 4
    game.state.player1.stage.stage = [sumire, center_high, right_low];
    // Opponent center has even higher cost 15
    let opp_center = game.id("PL!-sd1-009-SD"); // cost 15
    game.state.player2.stage.stage = [-1, opp_center, -1];
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 1, "opponent high cost must not affect own center-high check");

    // Even if opponent center is low, no effect
    let opp_low = game.id("PL!-sd1-010-SD");
    game.state.player2.stage.stage = [-1, opp_low, -1];
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 1, "still gains regardless of opponent");
}

// Empty center -> always fails, regardless of sides.
#[test]
fn sp_bp2_004_center_empty_always_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let left_high = game.id("PL!SP-pb1-001-R");
    let right_high = game.id("PL!SP-pb1-001-R");
    game.state.player1.stage.stage = [left_high, -1, right_high];
    // Put sumire at left? Actually sumire must be somewhere to measure, but center empty should give 0 for sumire even if sumire is left
    game.state.player1.stage.stage[0] = sumire;
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 0, "center empty -> no heart even with high sides");

    // Even if only sumire at left, center empty -> no heart
    game.state.player1.stage.stage = [sumire, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 0, "only left present, center empty -> no heart");
}

// Single center member (sumire at center alone) -> vacuously highest -> gains.
#[test]
fn sp_bp2_004_single_center_alone_gains() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    game.state.player1.stage.stage = [-1, sumire, -1];
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 1, "sumire alone at center -> highest (no competitors) -> gains");
}

// Dynamic re-evaluation: after removing the high side, heart toggles.
#[test]
fn sp_bp2_004_dynamic_toggle_on_removal() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R");
    let center = game.id("PL!HS-PR-001-PR"); // cost 10
    let right_high = game.id("PL!SP-pb1-001-R"); // 11, initially higher than center
    game.state.player1.stage.stage = [sumire, center, right_high];
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 0, "center 10 < right 11 -> no heart");

    // Remove right high
    game.state.player1.stage.stage[2] = -1;
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 1, "after removing right 11, center 10 > left 9 -> gains");

    // Add back a higher right again
    let right_higher = game.id("PL!-sd1-009-SD"); // 15
    game.state.player1.stage.stage[2] = right_higher;
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 0, "new right 15 > center 10 -> loses again");
}

// Cost modifier respect: effective cost (base + mods) is used, not just base.
// We inject a live cost modifier via direct mods add (persists across constant recalc).
#[test]
fn sp_bp2_004_cost_modifier_flips_result() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R"); // 9 at left
    let center = game.id("PL!HS-PR-001-PR"); // 10 at center
    let right = game.id("PL!-sd1-010-SD"); // 4 at right
    // Base: center 10 > left 9 and right 4 -> gains
    game.state.player1.stage.stage = [sumire, center, right];
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 1, "base center 10 highest -> gains");

    // Make right effective cost 12 by adding +8 modifier (4+8=12 > center 10) -> center no longer highest
    game.state.mods.add_cost_modifier(right, 8);
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 0, "right effective 12 > center 10 -> no heart (modifier respected)");

    // Remove modifier by adding -8 (net 0) -> should regain
    game.state.mods.add_cost_modifier(right, -8);
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 1, "removed modifier -> center regains");

    // Make center itself cheaper via modifier: center 10 -5 =5 < left 9 -> no heart
    game.state.mods.add_cost_modifier(center, -5);
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 0, "center effective 5 < left 9 -> no heart");

    // Restore center
    game.state.mods.add_cost_modifier(center, 5);
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 1, "restored center 10 -> gains again");

    // Verify that modifier on sumire itself affects comparison: sumire left 9 +5 =14 > center 10 -> center loses
    game.state.mods.add_cost_modifier(sumire, 5);
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 0, "left sumire effective 14 > center 10 -> center not highest");
    game.state.mods.add_cost_modifier(sumire, -5);
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 1, "cleanup restores");
}

// Position change re-evaluates: moving a high-cost member away should flip.
#[test]
fn sp_bp2_004_position_change_re_evaluates() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-004-R"); // 9
    let center = game.id("PL!HS-PR-001-PR"); // 10
    let right_high = game.id("PL!SP-pb1-001-R"); // 11
    game.state.player1.stage.stage = [sumire, center, right_high];
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 0, "initially center 10 < right 11 -> no heart");
    // Simulate position change: swap right high with empty? Just move right high out
    game.state.player1.stage.stage[2] = -1;
    // Place that high elsewhere? Actually removal already tested; here test move left<->center
    game.state.player1.stage.stage = [center, sumire, -1]; // sumire now at center 9, center card now at left 10
    game.state.recalculate_constants();
    // Now center is sumire 9, left is 10 -> center 9 < left 10 -> no heart (sumire at center but left higher)
    assert_eq!(heart03(&game, sumire), 0, "sumire at center 9 < left 10 -> no heart after swap");
    // Move high left away, leaving only sumire at center
    game.state.player1.stage.stage = [-1, sumire, -1];
    game.state.recalculate_constants();
    assert_eq!(heart03(&game, sumire), 1, "only sumire at center -> gains");
}
