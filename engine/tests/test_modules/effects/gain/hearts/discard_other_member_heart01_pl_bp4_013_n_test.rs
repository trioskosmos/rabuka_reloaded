use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const LIVE_START: &str = "ライブ開始時";
const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn pl_bp4_013_n_paid_discard_grants_other_member_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp4-013-N");
    let mate = game.new_id(FILLER);
    game.state.player1.stage.stage[1] = me;
    game.state.player1.stage.stage[0] = mate;
    game.add_to_hand(game.new_id(FILLER));

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    assert!(
        game.has_pending_choice(),
        "optional discard gate must be offered"
    );
    game.select_option(0);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    game.state.recalculate_constants();

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(mate, HeartColor::Heart01),
        1,
        "the OTHER stage member gains heart01 until live end"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart01),
        0,
        "exclude_self: the ability holder gains nothing"
    );
}

#[test]
fn pl_bp4_013_n_declined_discard_grants_no_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp4-013-N");
    let mate = game.new_id(FILLER);
    game.state.player1.stage.stage[1] = me;
    game.state.player1.stage.stage[0] = mate;
    game.add_to_hand(game.new_id(FILLER));

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    game.select_option(1);
    game.state.recalculate_constants();

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(mate, HeartColor::Heart01),
        0,
        "declined -> no heart gain"
    );
}
