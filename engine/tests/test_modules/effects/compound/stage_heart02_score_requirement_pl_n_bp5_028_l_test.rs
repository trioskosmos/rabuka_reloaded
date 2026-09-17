use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_n_bp5_028_l_live_start_heart02_member_grants_score_and_heart02_requirement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!N-bp5-028-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    game.state.current_phase = rabuka_engine::game_state::Phase::FirstAttackerPerformance;
    let chika = game.new_id("PL!S-bp5-001-R＋");
    game.state.player1.stage.stage[0] = chika;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        2,
        "heart02 x4 member on stage -> live +2"
    );
    let nh02 = game
        .state
        .mods
        .need_heart_modifiers
        .get(&live)
        .and_then(|m| m.get(&HeartColor::Heart02))
        .map(|e| e.total())
        .unwrap_or(0);
    assert!(
        nh02 >= 5,
        "required hearts must include heart02 totalling >=5 (got {nh02})"
    );
}
