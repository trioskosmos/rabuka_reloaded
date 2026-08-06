/// BP07 C1: PL!S-bp7-009-R 黒澤ルビィ ab#0.
///
/// 常時：このメンバーの正面のエリアにいるコスト4以下のメンバーは、ブレードを1つ失う。
///
/// Members with cost ≤ 4 in the area directly in front of this member lose 1 blade (constant).
///
/// Card facts:
///   RUBY            cost 2  (the ability owner — cost 2 ≤ 4)
///   COST4_MEMBER    PL!-sd1-010-SD (高坂穂乃果) cost 4  → should be affected
///   COST5_MEMBER    PL!-pb1-021-PR (南ことり)   cost 5  → cost > 4, NOT affected
use crate::helpers::*;

const RUBY: &str = "PL!S-bp7-009-R";
const COST4_MEMBER: &str = "PL!-sd1-010-SD"; // cost 4
const COST5_MEMBER: &str = "PL!-pb1-021-PR"; // cost 5

// ── helpers ──────────────────────────────────────────────────────────────────

fn setup(ruby_slot: usize, opp_slot: usize, opp_card: &str) -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id(RUBY);
    let opp = game.id(opp_card);
    let mut p1 = [-1i16; 3];
    let mut p2 = [-1i16; 3];
    p1[ruby_slot] = ruby;
    p2[opp_slot] = opp;
    game.state.player1.stage.stage = p1;
    game.state.player2.stage.stage = p2;
    game.state.recalculate_constant_blade_modifiers();
    (game, ruby, opp)
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Ruby P1 Center (slot 1) → opponent P2 Center (slot 1) is the front slot.
/// Cost 4 member in P2 Center: loses 1 blade.
#[test]
fn ruby_front_member_cost_4_loses_one_blade() {
    let (game, _ruby, opp) = setup(1, 1, COST4_MEMBER);
    let mod_val = game.state.mods.get_blade_modifier(opp);
    assert_eq!(
        mod_val, -1,
        "Cost 4 member directly in front of Ruby should lose 1 blade (got {mod_val})"
    );
}

/// Cost 5 member in P2 Center (front of P1 Center): NOT affected (cost > 4).
#[test]
fn ruby_front_member_cost_5_not_affected() {
    let (game, _ruby, opp) = setup(1, 1, COST5_MEMBER);
    let mod_val = game.state.mods.get_blade_modifier(opp);
    assert_eq!(mod_val, 0, "Cost 5 member should NOT be affected (got {mod_val})");
}

/// Cost 4 member in P2 Left (slot 0) — front of P1 Right (slot 2), NOT P1 Center.
/// Ruby is at P1 Center, so P2 Left is not the front slot.
#[test]
fn ruby_center_does_not_affect_side_slots() {
    let (game, _ruby, opp) = setup(1, 0, COST4_MEMBER); // opp at P2 Left
    let mod_val = game.state.mods.get_blade_modifier(opp);
    assert_eq!(mod_val, 0, "Cost 4 member at P2 Left is not in front of P1 Center Ruby (got {mod_val})");
}

/// Ruby at P1 Left (slot 0) → mirrors to P2 Right (slot 2).
/// Cost 4 at P2 Right: loses 1 blade. Cost 4 at P2 Center: unaffected.
#[test]
fn ruby_left_affects_p2_right_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id(RUBY);
    let opp_right = game.id(COST4_MEMBER);
    let opp_center = game.id(COST4_MEMBER);

    // Ruby at P1 Left; cost 4 members at P2 Center and P2 Right
    game.state.player1.stage.stage = [ruby, -1, -1];
    game.state.player2.stage.stage = [-1, opp_center, opp_right];
    game.state.recalculate_constant_blade_modifiers();

    assert_eq!(
        game.state.mods.get_blade_modifier(opp_right), -1,
        "P2 Right is directly in front of P1 Left Ruby — should lose 1 blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(opp_center), 0,
        "P2 Center is NOT in front of P1 Left Ruby — should be unaffected"
    );
}

/// Ruby at P1 Right (slot 2) → mirrors to P2 Left (slot 0).
/// Cost 4 at P2 Left: loses 1 blade.
#[test]
fn ruby_right_affects_p2_left_only() {
    let (game, _ruby, opp) = setup(2, 0, COST4_MEMBER);
    let mod_val = game.state.mods.get_blade_modifier(opp);
    assert_eq!(
        mod_val, -1,
        "P2 Left is directly in front of P1 Right Ruby — should lose 1 blade"
    );
}

/// Two Rubys facing each other on Center slots (P1 Center & P2 Center).
/// Both Rubys have cost 2 (≤ 4), so both Rubys lose 1 blade (-1 each).
#[test]
fn ruby_facing_ruby_both_debuffed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby_p1 = game.id(RUBY);
    let ruby_p2 = game.id(RUBY);

    game.state.player1.stage.stage = [-1, ruby_p1, -1];
    game.state.player2.stage.stage = [-1, ruby_p2, -1];
    game.state.recalculate_constant_blade_modifiers();

    assert_eq!(
        game.state.mods.get_blade_modifier(ruby_p1), -1,
        "P1 Ruby should lose 1 blade from P2 Ruby"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby_p2), -1,
        "P2 Ruby should lose 1 blade from P1 Ruby"
    );
}

/// Empty front slot (no opponent member on stage): recalculate doesn't panic.
#[test]
fn empty_front_slot_no_crash() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id(RUBY);

    game.state.player1.stage.stage = [-1, ruby, -1];
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.recalculate_constant_blade_modifiers();

    assert_eq!(game.state.mods.constant_blade_bonuses.len(), 0);
}

/// When opponent member moves out of the front slot to a side slot, blade is restored.
#[test]
fn opponent_member_moves_out_of_front_recovers_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id(RUBY);
    let opp = game.id(COST4_MEMBER);

    // Opponent member at P2 Center (in front of Ruby)
    game.state.player1.stage.stage = [-1, ruby, -1];
    game.state.player2.stage.stage = [-1, opp, -1];
    game.state.recalculate_constant_blade_modifiers();
    assert_eq!(game.state.mods.get_blade_modifier(opp), -1);

    // Opponent moves to P2 Left (slot 0)
    game.state.player2.stage.stage = [opp, -1, -1];
    game.state.recalculate_constant_blade_modifiers();
    assert_eq!(
        game.state.mods.get_blade_modifier(opp), 0,
        "After moving to side slot out of front area, blade should recover to 0"
    );
}

/// When Ruby leaves the stage, the negative blade modifier is removed.
#[test]
fn ruby_leaves_stage_modifier_removed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id(RUBY);
    let opp = game.id(COST4_MEMBER);

    game.state.player1.stage.stage = [-1, ruby, -1];
    game.state.player2.stage.stage = [-1, opp, -1];
    game.state.recalculate_constant_blade_modifiers();
    assert_eq!(game.state.mods.get_blade_modifier(opp), -1, "Should have -1 with Ruby on stage");

    game.state.player1.stage.stage[1] = -1; // Ruby leaves
    game.state.recalculate_constant_blade_modifiers();
    assert_eq!(game.state.mods.get_blade_modifier(opp), 0, "Modifier should be 0 after Ruby leaves");
}
