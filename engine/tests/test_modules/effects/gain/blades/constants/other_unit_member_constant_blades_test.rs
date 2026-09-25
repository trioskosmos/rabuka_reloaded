use crate::helpers::*;

#[test]
fn constant_counts_other_unit_member_but_not_self_or_wrong_unit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!HS-bp2-006-R");
    // Another みらくらぱーく！ member + a non-Mirakuraku member
    let other_mk = game.id("PL!HS-bp1-005-R");
    let not_mk = game.id("PL!HS-sd1-005-SD"); // DOLLCHESTRA
    game.state.player1.stage.stage = [other_mk, member, not_mk];
    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(member);
    assert_eq!(
        blade, 1,
        "one other Mirakuraku member → exactly +1 blade (DOLLCHESTRA must not count)"
    );
}
