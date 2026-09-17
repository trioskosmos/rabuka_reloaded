use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    crate::helpers::fire_trigger(game, cid, trigger, trig);
    game.drain_auto_ability_choices();
}

fn chisato_pl_sp_sd2_003_sd2_area_move_this_turn(game: &mut TestGame, do_swap: bool) -> i16 {
    let chisato = game.id("PL!SP-sd2-003-SD2");
    let kinako = game.id("PL!SP-bp5-006-R");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.add_to_hand(chisato);
    game.add_to_hand(kinako);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(30);

    game.try_play_to_stage(chisato, MemberArea::LeftSide)
        .expect("play chisato");
    game.try_play_to_stage(kinako, MemberArea::RightSide)
        .expect("play kinako");

    if do_swap {
        game.activate_ability(kinako);
        assert!(
            game.has_pending_choice(),
            "expected position|destination choice"
        );
        let actions = game.generated_actions();
        let left_action = actions
            .iter()
            .find(|a| {
                a.action_type == rabuka_engine::game_setup::ActionType::ChoicePosition
                    && a.description.contains("左")
            })
            .or_else(|| {
                actions.iter().find(|a| {
                    a.action_type == rabuka_engine::game_setup::ActionType::ChoicePosition
                })
            })
            .expect("a position option must exist");
        let p = left_action.parameters.as_ref().expect("params");
        rabuka_engine::turn::TurnEngine::resume_with_choice(
            &mut game.state,
            p.card_id,
            p.card_indices.clone(),
        )
        .expect("position change failed");
        assert_eq!(
            game.state.player1.stage.stage[2], chisato,
            "swap moved 千砂都 to the right side"
        );
    }
    chisato
}

#[test]
fn chisato_pl_sp_sd2_003_sd2_draws_extra_after_area_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = chisato_pl_sp_sd2_003_sd2_area_move_this_turn(&mut game, true);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(
        &mut game,
        chisato,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        2,
        "area-moved this turn -> draw 1 + 1 more"
    );
}

#[test]
fn chisato_pl_sp_sd2_003_sd2_single_draw_without_area_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = chisato_pl_sp_sd2_003_sd2_area_move_this_turn(&mut game, false);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(
        &mut game,
        chisato,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "no area move this turn -> only the base draw"
    );
}
