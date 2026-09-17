use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const LIVE_START: &str = "ライブ開始時";
const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn pl_sp_bp4_017_n_left_area_moved_this_turn_grants_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-017-N");
    game.state.player1.stage.stage[0] = me;
    game.state.cards_moved_this_turn.push(me);
    game.state.position_change_occurred_this_turn = true;

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        2,
        "moved-this-turn on the left side -> +2 blades until live end"
    );
}

#[test]
fn pl_sp_bp4_017_n_left_area_unmoved_grants_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-017-N");
    game.state.player1.stage.stage[0] = me;

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "did not move this turn -> no blades"
    );
}

#[test]
fn pl_sp_bp4_017_n_center_area_moved_grants_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-017-N");
    game.state.player1.stage.stage[1] = me;
    game.state.cards_moved_this_turn.push(me);
    game.state.position_change_occurred_this_turn = true;

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "ability only activates in the left-side area"
    );
}
