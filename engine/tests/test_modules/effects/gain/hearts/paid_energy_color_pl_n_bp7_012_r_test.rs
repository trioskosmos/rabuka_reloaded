use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_n_bp7_012_r_paid_energy_and_first_color_choice_grant_one_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let me = game.id("PL!N-bp7-012-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(3);
    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice(), "optional {{E}} gate offered");
    game.select_option(1);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        if game.pending_choice_type().as_deref() == Some("SelectHeartColor") {
            game.select_choice_option(0);
        } else {
            game.select_indices(&[0]);
        }
    }
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart01),
        1,
        "first offered color gained until live end"
    );
}
