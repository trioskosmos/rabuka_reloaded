use crate::helpers::*;
use crate::test_modules::support::ability_trigger_and_deck_setup::*;

#[test]
fn other_subunit_member_grants_exactly_one_heart05_and_excludes_self() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let c = g.id("PL!-bp5-111-R");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, c, -1];
    fill_both_main_decks(&mut g, f);
    g.give_energy(5);
    g.state.recalculate_constants();
    use rabuka_engine::card::HeartColor;
    // No other A-RISE member → no heart05 bonus.
    assert_eq!(
        g.state.mods.get_heart_modifier(c, HeartColor::Heart05),
        0,
        "no other A-RISE means no heart05"
    );
    // Another A-RISE member on stage → +1 heart05 per such member.
    let anju = g.id("PL!-bp5-222-R"); // 優木あんじゅ, A-RISE
    g.state.player1.stage.stage = [anju, c, -1];
    g.state.recalculate_constants();
    assert_eq!(
        g.state.mods.get_heart_modifier(c, HeartColor::Heart05),
        1,
        "1 other A-RISE member must grant exactly 1 heart05"
    );
}
