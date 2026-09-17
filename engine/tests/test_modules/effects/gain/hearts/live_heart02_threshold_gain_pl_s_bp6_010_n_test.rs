use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const LIVE_START: &str = "ライブ開始時";
const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn pl_s_bp6_010_n_live_requires_four_heart02_grants_one_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!S-bp6-010-N");
    game.state.player1.stage.stage[1] = me;
    let live = game.id("PL!SP-pb1-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    game.state.current_phase = rabuka_engine::game_state::Phase::FirstAttackerPerformance;

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart02),
        1,
        "required-heart02 total 4 >= 4 -> gain one heart02 until live end"
    );
}

#[test]
fn pl_s_bp6_010_n_live_requires_no_heart02_grants_none() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!S-bp6-010-N");
    game.state.player1.stage.stage[1] = me;
    let live = game.id("PL!HS-bp2-020-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    game.state.current_phase = rabuka_engine::game_state::Phase::FirstAttackerPerformance;

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, LIVE_START);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart02),
        0,
        "aggregate heart02 total 0 < 4 -> no gain"
    );
}
