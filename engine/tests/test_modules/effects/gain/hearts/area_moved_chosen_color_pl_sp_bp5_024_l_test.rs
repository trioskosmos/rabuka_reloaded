use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

fn answer_all(game: &mut TestGame, idx: usize) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        if game.pending_choice_type().as_deref() == Some("SelectHeartColor") {
            game.select_choice_option(idx);
        } else {
            game.select_indices(&[idx]);
        }
    }
}

#[test]
fn pl_sp_bp5_024_l_live_start_chosen_heart_granted_only_to_area_moved_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!SP-bp5-024-L");
    game.add_to_hand(live);
    game.set_live_card(live);

    let mover = game.new_id("PL!S-bp5-001-R＋");
    let stationary = game.new_id("PL!HS-bp5-001-R＋");
    game.state.player1.stage.stage[0] = mover;
    game.state.player1.stage.stage[2] = stationary;
    game.state
        .push_movement_event(mover, "stage", "stage", None, "p1", true);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    answer_all(&mut game, 0);
    game.state.recalculate_constants();

    let hearts = |id: i16| {
        game.state.mods.get_heart_modifier(id, HeartColor::Heart01)
            + game.state.mods.get_heart_modifier(id, HeartColor::Heart02)
            + game.state.mods.get_heart_modifier(id, HeartColor::Heart06)
    };
    assert_eq!(
        hearts(mover),
        1,
        "area-moved member gains the chosen heart until live end"
    );
    assert_eq!(
        hearts(stationary),
        0,
        "member that did not move areas gains nothing"
    );
}
