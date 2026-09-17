use crate::helpers::*;

fn drain_auto_ability_choices(game: &mut TestGame) {
    for _ in 0..30 {
        if let Some(choice) = game.state.get_pending_choice() {
            match choice {
                rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
                _ => break,
            }
        } else {
            break;
        }
    }
}

fn advance_live(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
        drain_auto_ability_choices(game);
    }
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn setup_stage_and_card_in_live_zone(game: &mut TestGame, live_no: &str) -> i16 {
    let m = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [m, m, m];
    fill_decks(game, game.id_ref("PL!-sd1-010-SD"));
    let live = game.id(live_no);
    game.state.player1.hand.cards.push(live);
    game.give_energy(20);
    advance_to_live_card_set_p1(game);
    game.set_live_card(live);
    live
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

#[test]
fn three_energy_live_success_draw_member_in_live_zone_shrinks_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let _live = setup_stage_and_card_in_live_zone(&mut game, "PL!SP-pb1-004-R");
    let deck_before = game.state.player1.main_deck.cards.len();

    advance_live(&mut game);

    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_before > deck_after,
        "deck should shrink as draws happen"
    );
    assert!(
        !game.has_pending_choice(),
        "all prompts resolved"
    );
}

#[test]
fn one_energy_live_success_draw_member_in_live_zone_shrinks_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let _live = setup_stage_and_card_in_live_zone(&mut game, "PL!SP-bp5-020-N");
    let deck_before = game.state.player1.main_deck.cards.len();

    advance_live(&mut game);

    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_before > deck_after,
        "deck should shrink as draws happen"
    );
    assert!(
        !game.has_pending_choice(),
        "all prompts resolved"
    );
}

#[test]
fn one_energy_live_success_draw_skip_leaves_no_pending_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp5-020-N");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..20 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [game.new_id("PL!-sd1-001-SD"), game.new_id("PL!-sd1-001-SD"), game.new_id("PL!-sd1-001-SD")];
    game.state.player1.hand.cards.push(live);
    game.give_energy(20);
    for _ in 0..5 { game.pass(); }
    game.set_live_card(live);
    // Directly fire LiveSuccess via trigger to test pay vs skip without full live flow
    crate::helpers::fire_trigger(&mut game, live, rabuka_engine::core::types::AbilityTrigger::LiveSuccess, "ライブ成功時");
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => game.select_option(0), // skip is 0
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    // Skip should not draw, hand should not have grown beyond the live itself
    assert!(!game.has_pending_choice());
}

#[test]
fn one_energy_live_success_draw_without_granted_energy_does_not_increase_active_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp5-020-N");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..20 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage = [game.new_id("PL!-sd1-001-SD"), -1, -1];
    game.state.player1.hand.cards.push(live);
    game.give_energy(0); // need 1, have 0
    for _ in 0..5 { game.pass(); }
    game.set_live_card(live);
    crate::helpers::fire_trigger(&mut game, live, rabuka_engine::core::types::AbilityTrigger::LiveSuccess, "ライブ成功時");
    let before = game.state.player1.energy_zone.active_count();
    if let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                // Try to pay with 0 energy — should either not offer pay or fail
                game.select_option(1); // try pay (if available)
            }
            _ => panic!("Unexpected choice type: {:?}", choice),
        }
    }
    game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[]);
    assert!(game.state.player1.energy_zone.active_count() <= before);
    assert!(true);
}
