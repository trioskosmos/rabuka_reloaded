use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn heart_mod(game: &TestGame, card_id: i16, heart: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(card_id, heart)
}

fn fill_deck_and_energy(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..15 {
        game.state.player1.energy_zone.cards.push(filler);
    }
    game.state.player1.energy_zone.set_active_count(15);
}

fn drain_auto_choices(game: &mut TestGame) {
    while game.has_pending_choice() {
        match game.get_pending_choice().clone() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            other => panic!(
                "expected only auto-ability ordering choices, got {:?}",
                other
            ),
        }
    }
}

fn trigger_p1_auto_abilities(game: &mut TestGame) {
    let player_id = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    drain_auto_choices(game);
}

fn accept_optional_position_change(game: &mut TestGame, destination: &str) {
    if matches!(
        game.get_pending_choice().clone(),
        Choice::SelectAutoAbility { .. }
    ) {
        game.select_option(0);
    }

    // Keke's optional position change creates a single position|destination
    // choice with allow_skip: true — no separate conditional_optional step.
    match game.get_pending_choice().clone() {
        Choice::SelectTarget { target, .. } => {
            assert_eq!(
                target, "position|destination",
                "Keke debut should ask for a position-change destination"
            );
            let action_index = game
                .generated_actions()
                .iter()
                .position(|action| {
                    action
                        .parameters
                        .as_ref()
                        .and_then(|params| params.stage_area.as_deref())
                        == Some(destination)
                })
                .unwrap_or_else(|| {
                    panic!(
                        "position-change destination '{}' should be a legal generated action",
                        destination
                    )
                });
            assert!(
                !game.generated_actions().is_empty(),
                "position-change destination choice should expose legal generated actions"
            );
            game.select_generated(action_index);
        }
        other => panic!(
            "Keke debut should ask for a position-change destination, got {:?}",
            other
        ),
    }
}

#[test]
fn hazuki_debut_energy_placement_triggers_energy_watchers() {
    let mut game = TestGame::new(load_real_database());

    let sumire = game.id("PL!SP-bp5-004-R+");
    let hazuki_watcher = game.id("PL!SP-bp4-016-N");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [sumire, hazuki_watcher, -1];

    let energy_from_deck = game.id("PL!-sd1-010-SD");
    game.state.player1.energy_deck.cards.push(energy_from_deck);
    let trigger = game.id("PL!SP-pb1-005-R");
    game.state.player1.hand.cards.push(trigger);

    let hand_before = game.state.player1.hand.cards.len();
    let main_deck_before = game.state.player1.main_deck.cards.len();
    let energy_deck_before = game.state.player1.energy_deck.cards.len();
    let energy_zone_before = game.state.player1.energy_zone.cards.len();

    game.play_to_stage(trigger, MemberArea::RightSide);
    drain_auto_choices(&mut game);

    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::RightSide),
        Some(trigger),
        "Hazuki Kano should remain in the played right-side slot"
    );
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        energy_deck_before - 1,
        "Hazuki Kano debut should move exactly one card out of the energy deck"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_zone_before + 1,
        "Hazuki Kano debut should place exactly one card into the energy zone"
    );
    assert!(
        game.state
            .player1
            .energy_zone
            .cards
            .contains(&energy_from_deck),
        "the card moved from energy deck should be present in the energy zone"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        main_deck_before - 1,
        "Sumire should draw exactly one card from the main deck"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "playing Hazuki spends one hand card and Sumire draws one replacement"
    );
    assert_eq!(
        heart_mod(&game, sumire, HeartColor::Heart02),
        1,
        "Sumire should gain heart02 when own effect puts energy into the energy zone"
    );
    assert_eq!(
        heart_mod(&game, hazuki_watcher, HeartColor::Heart06),
        1,
        "Hazuki watcher should gain heart06 when an effect puts energy into the energy zone"
    );
}

#[test]
fn opponent_energy_effect_triggers_hazuki_but_not_sumire() {
    let mut game = TestGame::new(load_real_database());

    let sumire = game.id("PL!SP-bp5-004-R+");
    let hazuki_watcher = game.id("PL!SP-bp4-016-N");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [sumire, hazuki_watcher, -1];

    let hand_before = game.state.player1.hand.cards.len();
    game.state
        .push_movement_event(-1, "energy_deck", "energy", None, "p2", true);

    trigger_p1_auto_abilities(&mut game);

    assert_eq!(
        heart_mod(&game, sumire, HeartColor::Heart02),
        0,
        "Sumire says 'by your own card effect', so opponent energy placement should not trigger her"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Sumire should not draw from opponent energy placement"
    );
    assert_eq!(
        heart_mod(&game, hazuki_watcher, HeartColor::Heart06),
        1,
        "Hazuki watcher explicitly says opponent card effects also trigger it"
    );
}

#[test]
fn keke_position_change_swapping_with_sumire_triggers_sumire() {
    let mut game = TestGame::new(load_real_database());

    let sumire = game.id("PL!SP-bp5-004-R+");
    let natsumi = game.id("PL!SP-pb1-020-N");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [sumire, natsumi, -1];

    let trigger = game.id("PL!SP-bp4-013-N");
    game.state.player1.hand.cards.push(trigger);

    let main_deck_before = game.state.player1.main_deck.cards.len();

    game.play_to_stage(trigger, MemberArea::RightSide);
    accept_optional_position_change(&mut game, "left");
    drain_auto_choices(&mut game);

    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::LeftSide),
        Some(trigger),
        "Keke should move from right to left"
    );
    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::RightSide),
        Some(sumire),
        "Sumire should be swapped from left to right by Keke's position change"
    );
    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::Center),
        Some(natsumi),
        "Natsumi should stay center and should not satisfy 'this member moved'"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        main_deck_before - 1,
        "only Sumire should draw because only Sumire moved"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "playing Keke spends one hand card, then Sumire draws one"
    );
    assert_eq!(
        heart_mod(&game, sumire, HeartColor::Heart02),
        1,
        "Sumire should gain heart02 when Keke swaps into her area; Q220 says this movement triggers"
    );
}

#[test]
fn keke_position_change_swapping_with_natsumi_triggers_natsumi() {
    let mut game = TestGame::new(load_real_database());

    let sumire = game.id("PL!SP-bp5-004-R+");
    let natsumi = game.id("PL!SP-pb1-020-N");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [sumire, natsumi, -1];

    let trigger = game.id("PL!SP-bp4-013-N");
    game.state.player1.hand.cards.push(trigger);

    let main_deck_before = game.state.player1.main_deck.cards.len();

    game.play_to_stage(trigger, MemberArea::RightSide);
    accept_optional_position_change(&mut game, "center");
    drain_auto_choices(&mut game);

    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::Center),
        Some(trigger),
        "Keke should move from right to center"
    );
    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::RightSide),
        Some(natsumi),
        "Natsumi should be swapped from center to right by Keke's position change"
    );
    assert_eq!(
        game.state.player1.stage.get_area(MemberArea::LeftSide),
        Some(sumire),
        "Sumire should stay left and should not satisfy 'this member moved'"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        main_deck_before - 1,
        "only Natsumi should draw because only Natsumi moved"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "playing Keke spends one hand card, then Natsumi draws one"
    );
    assert_eq!(
        heart_mod(&game, sumire, HeartColor::Heart02),
        0,
        "Sumire should not gain heart02 when she did not move and no energy was placed"
    );
}
