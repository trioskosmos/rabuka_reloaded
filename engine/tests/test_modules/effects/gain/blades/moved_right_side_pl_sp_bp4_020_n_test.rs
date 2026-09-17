use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_sp_bp4_020_n_moved_right_side_member_gains_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let me = game.id("PL!SP-bp4-020-N");
    game.state.player1.stage.stage[2] = me;
    game.state.cards_moved_this_turn.push(me);
    game.state.position_change_occurred_this_turn = true;
    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(me), 2);
}

#[test]
fn pl_sp_bp4_020_n_moved_center_member_gains_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let me = game.id("PL!SP-bp4-020-N");
    game.state.player1.stage.stage[1] = me;
    game.state.cards_moved_this_turn.push(me);
    game.state.position_change_occurred_this_turn = true;
    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "right-side-only ability must not fire from center"
    );
}
