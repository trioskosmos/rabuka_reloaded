use crate::helpers::*;
use crate::test_modules::support::ability_trigger_and_deck_setup::*;

#[test]
fn live_start_optional_two_member_bottomdeck_shrinks_waitroom() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let c = g.id("PL!N-bp3-009-R+");
    let m = g.id("PL!-sd1-001-SD");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, c, -1];
    g.state.player1.waitroom.cards.push(m);
    g.state.player1.waitroom.cards.push(m);
    fill_both_main_decks(&mut g, f);
    g.give_energy(5);
    let w = g.state.player1.waitroom.cards.len();
    trigger_printed_ability_and_resolve_choices(&mut g, c, "ライブ開始時");
    assert!(g.state.player1.waitroom.cards.len() < w, "waitroom shrank");
}
