use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn s_pr_039_pr_constant_four_combined_success_cards_gains_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-PR-039-PR");
    game.state.player1.stage.stage[0] = me;

    for _ in 0..2 {
        let a = game.new_id(FILLER);
        game.state.player1.success_live_card_zone.cards.push(a);
        let b = game.new_id(FILLER);
        game.state.player2.success_live_card_zone.cards.push(b);
    }
    // Combined = 4.

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        2,
        "4 combined success-zone cards -> +2 blades"
    );
}

#[test]
fn s_pr_039_pr_constant_two_combined_success_cards_gains_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-PR-039-PR");
    game.state.player1.stage.stage[0] = me;

    let a = game.new_id(FILLER);
    game.state.player1.success_live_card_zone.cards.push(a);
    let b = game.new_id(FILLER);
    game.state.player2.success_live_card_zone.cards.push(b);
    // Combined = 2 < 4.

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "only 2 combined success-zone cards -> no blades"
    );
}
