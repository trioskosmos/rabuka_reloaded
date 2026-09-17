use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn pl_n_sd2_005_sd2_paid_discard_grants_exactly_two_hearts_of_one_color() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id("PL!N-sd2-005-SD2");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id("PL!-sd1-010-SD"));
    game.add_to_hand(game.new_id("PL!S-sd1-001-SD"));
    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice(), "discard-2 cost prompted");
    game.select_indices(&[0, 1]);
    assert!(game.has_pending_choice(), "color specification offered");
    game.select_option(3);
    let all = [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ];
    let boosted: Vec<HeartColor> = all
        .iter()
        .filter(|c| game.state.mods.get_heart_modifier(me, **c) > 0)
        .copied()
        .collect();
    assert_eq!(boosted.len(), 1, "exactly one color boosted");
    assert_eq!(
        game.state.mods.get_heart_modifier(me, boosted[0]),
        2,
        "the chosen color gained exactly +2"
    );
}
