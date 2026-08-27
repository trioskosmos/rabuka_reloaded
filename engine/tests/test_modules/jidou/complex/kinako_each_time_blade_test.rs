use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_deck_and_energy(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(100);
}

fn blade_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state.mods.get_blade_modifier(card_id)
}

fn accept_position_swap(game: &mut TestGame, dest: &str) {
    assert!(
        game.has_pending_choice(),
        "expected position|destination choice for kinako each_time swap to {}, got none",
        dest
    );
    match game.get_pending_choice().clone() {
        rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
            if target == "position|destination" =>
        {
            let acts = game.generated_actions();
            let idx = acts
                .iter()
                .position(|a| {
                    a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some(dest)
                })
                .unwrap_or_else(|| panic!("destination {} not offered, got {:?}", dest, acts.iter().filter_map(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref())).collect::<Vec<_>>()));
            game.select_generated(idx);
            game.drain_auto_ability_choices();
        }
        other => panic!(
            "expected SelectTarget position|destination for kinako swap to {}, got {:?}",
            dest, other
        ),
    }
}

#[test]
fn kinako_each_time_blade_stack_draft() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako_watcher = game.id("PL!SP-pb1-006-R");
    let kinako_mover = game.id("PL!SP-bp5-006-R");
    fill_deck_and_energy(&mut game);

    // Place both directly, but trigger appearance for watcher
    game.state.player1.stage.stage = [kinako_watcher, kinako_mover, -1];
    game.state.record_card_appearance(kinako_watcher, "hand");
    game.state.record_card_movement(kinako_watcher);

    // === Move 1: 起動 きな子 swaps with watcher ===
    game.activate_ability(kinako_mover);
    game.drain_auto_ability_choices();
    let actions = game.generated_actions();
    let left_idx = actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("left"))
        .unwrap();
    game.select_generated(left_idx);
    game.drain_auto_ability_choices();

    // === Move 2: Keke debut → swap with watcher ===
    let keke1 = game.id("PL!SP-bp4-013-N");
    game.state.player1.hand.cards.push(keke1);
    game.play_to_stage(keke1, MemberArea::RightSide);
    game.drain_auto_ability_choices();
    accept_position_swap(&mut game, "center");

    // === Move 3: Mei debut → swap with watcher ===
    let mei = game.id("PL!SP-sd2-007-SD2");
    game.state.player1.hand.cards.push(mei);
    game.play_to_stage(mei, MemberArea::LeftSide);
    game.drain_auto_ability_choices();
    accept_position_swap(&mut game, "right");

    assert_eq!(
        blade_mod(&game, kinako_watcher),
        8,
        "1 appearance + 3 swaps = 4 triggers * 2 = 8"
    );
}
