use crate::helpers::*;

// PL!HS-bp2-006-R 藤島 慈 ab#1:
// 常時: 自分のステージにいるほかの『みらくらぱーく！』のメンバー1人につき、ブレードを得る。
//
// Uses recalculate_constants() (the real game path) to verify:
// - group_names filter works
// - exclude_self correctly excludes the card itself
// - each card gets its own independent bonus
//
// Test 1: self + 1 other matching + 1 unrelated → 1 blade (other only, self excluded)
#[test]
fn per_other_unit_member_constant_blades_excludes_self_and_wrong_unit() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let megumi = game.id("PL!HS-bp2-006-R");
    let hime = game.id("PL!HS-bp1-005-R");
    let unrelated = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hime, megumi, unrelated];

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(megumi);
    assert_eq!(
        blade_mod, 1,
        "1 other みらくらぱーく！ member → exactly 1 blade (self excluded), got {}",
        blade_mod
    );
}

// Test 2: 2 copies of the card on stage → each sees 1 other → 1 blade each
#[test]
fn per_other_unit_member_constant_blades_two_copies_each_gain_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let megumi_a = game.id("PL!HS-bp2-006-R");
    let megumi_b = game.id("PL!HS-bp2-006-R");
    let unrelated = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [megumi_b, megumi_a, unrelated];

    game.state.recalculate_constants();

    let mod_a = game.state.mods.get_blade_modifier(megumi_a);
    let mod_b = game.state.mods.get_blade_modifier(megumi_b);
    let mod_u = game.state.mods.get_blade_modifier(unrelated);
    assert_eq!(mod_a, 1, "Copy A: 1 other → 1 blade, got {}", mod_a);
    assert_eq!(mod_b, 1, "Copy B: 1 other → 1 blade, got {}", mod_b);
    assert_eq!(mod_u, 0, "Unrelated: 0 blade, got {}", mod_u);
}

// Test 3: 3 copies of the card on stage → each sees 2 others → 2 blade each
#[test]
fn per_other_unit_member_constant_blades_three_copies_each_gain_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let megumi_a = game.id("PL!HS-bp2-006-R");
    let megumi_b = game.id("PL!HS-bp2-006-R");
    let megumi_c = game.id("PL!HS-bp2-006-R");

    game.state.player1.stage.stage = [megumi_a, megumi_b, megumi_c];

    game.state.recalculate_constants();

    let mod_a = game.state.mods.get_blade_modifier(megumi_a);
    let mod_b = game.state.mods.get_blade_modifier(megumi_b);
    let mod_c = game.state.mods.get_blade_modifier(megumi_c);
    assert_eq!(mod_a, 2, "Copy A: 2 others → 2 blade, got {}", mod_a);
    assert_eq!(mod_b, 2, "Copy B: 2 others → 2 blade, got {}", mod_b);
    assert_eq!(mod_c, 2, "Copy C: 2 others → 2 blade, got {}", mod_c);
}
