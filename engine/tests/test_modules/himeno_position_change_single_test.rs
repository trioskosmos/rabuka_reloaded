/// Tests for PL!HS-pb1-006-R (安養寺姫芽) ab#0 — LiveStart position change.
///
/// Card text:
///   ライブ開始時: 自分のステージにいるほかの『みらくらぱーく！』のメンバーが
///   いるエリアにポジションチェンジしてもよい。そうした場合、ライブ終了時まで、
///   heart01+bladeを得る。
///
/// Bug: after resolving one position change, a second position change is offered.
use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
}

/// Drain any SelectAutoAbility prompts, selecting the first option each time.
fn drain_auto_abilities(game: &mut TestGame) {
    while game.has_pending_choice() {
        match game.get_pending_choice().clone() {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0);
            }
            _ => break,
        }
    }
}

/// Count how many position change choices appear after the initial one.
/// Returns (first_choice_found, total_position_change_count).
/// Resolves each position change by selecting the first available destination.
#[allow(dead_code)]
fn resolve_and_count_position_changes(game: &mut TestGame) -> (bool, usize) {
    let mut first_found = false;
    let mut count = 0;

    while game.has_pending_choice() {
        let choice = game.get_pending_choice().clone();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "position|destination" =>
            {
                count += 1;
                if count == 1 {
                    first_found = true;
                } else {
                    eprintln!("[TEST] Position change choice #{} appeared", count);
                }
                // Resolve: select first available destination, or skip if possible
                let actions = game.generated_actions();
                if actions.is_empty() {
                    // No valid destinations - should not happen here
                    break;
                }
                // Select first destination
                game.select_generated(0);
            }
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0);
            }
            _ => {
                eprintln!("[TEST] Other choice type: {:?}", choice);
                break;
            }
        }
    }

    (first_found, count)
}

// ---------------------------------------------------------------------------
// Test: Two mirakura destinations → should get exactly ONE position change
// ---------------------------------------------------------------------------
#[test]
fn himeno_pb1_two_mirakura_one_position_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let himeno = game.id("PL!HS-pb1-006-R");
    let mk_a = game.id("PL!HS-sd1-014-SD");
    let mk_b = game.id("PL!HS-sd1-006-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: [himeno, mk_a, mk_b] — all mirakura
    game.state.player1.stage.stage = [himeno, mk_a, mk_b];
    fill_decks(&mut game, filler);
    game.give_energy(11);

    advance_to_live_set(&mut game);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    // Select the position change ability from the auto-ability prompt
    drain_auto_abilities(&mut game);

    // Should now have a position change choice
    assert!(
        game.has_pending_choice(),
        "Should have position change choice after LiveStart"
    );

    let choice = game.get_pending_choice().clone();
    match &choice {
        rabuka_engine::ability::types::Choice::SelectTarget { target, .. } => {
            assert_eq!(target, "position|destination");
        }
        _ => panic!("Expected position choice, got {:?}", choice),
    }

    // Should have 2 valid destinations (Center and Right)
    let actions = game.generated_actions();
    assert_eq!(actions.len(), 2, "Should have 2 valid destinations");

    // Select Center
    game.select_generated(0);

    // Verify swap
    assert_eq!(
        game.state.player1.stage.stage[1], himeno,
        "Himeno at Center"
    );
    assert_eq!(game.state.player1.stage.stage[0], mk_a, "mk_a at Left");

    // Check for any more position changes (BUG: a second one appears)
    let mut extra_position_changes = 0;
    while game.has_pending_choice() {
        match game.get_pending_choice().clone() {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "position|destination" =>
            {
                extra_position_changes += 1;
                eprintln!(
                    "[TEST] BUG: Extra position change #{}",
                    extra_position_changes
                );
                // Skip or resolve
                let actions = game.generated_actions();
                if actions.is_empty() {
                    break;
                }
                game.select_generated(0);
            }
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0);
            }
            _ => break,
        }
    }

    // THE BUG: extra_position_changes should be 0 but is currently 1+
    assert_eq!(
        extra_position_changes, 0,
        "Should NOT have extra position changes after resolving the first one, but got {}",
        extra_position_changes
    );

    // Verify resources gained
    let blade = game.state.mods.get_blade_modifier(himeno);
    assert!(blade > 0, "Himeno should have gained blade");
}

// ---------------------------------------------------------------------------
// Test: One mirakura destination → should get exactly ONE position change
// ---------------------------------------------------------------------------
#[test]
fn himeno_pb1_one_mirakura_one_position_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let himeno = game.id("PL!HS-pb1-006-R");
    let mk = game.id("PL!HS-sd1-014-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: [himeno, mk, filler] — only Center is mirakura
    game.state.player1.stage.stage = [himeno, mk, filler];
    fill_decks(&mut game, filler);
    game.give_energy(11);

    advance_to_live_set(&mut game);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    drain_auto_abilities(&mut game);

    assert!(
        game.has_pending_choice(),
        "Should have position change choice"
    );

    let actions = game.generated_actions();
    assert_eq!(actions.len(), 1, "Only Center should be valid");

    // Select Center
    game.select_generated(0);

    // Verify swap
    assert_eq!(game.state.player1.stage.stage[0], mk, "mk at Left");
    assert_eq!(
        game.state.player1.stage.stage[1], himeno,
        "Himeno at Center"
    );

    // Check for extra position changes
    let mut extra = 0;
    while game.has_pending_choice() {
        match game.get_pending_choice().clone() {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "position|destination" =>
            {
                extra += 1;
                let actions = game.generated_actions();
                if actions.is_empty() {
                    break;
                }
                game.select_generated(0);
            }
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_option(0);
            }
            _ => break,
        }
    }

    assert_eq!(
        extra, 0,
        "Should NOT have extra position changes, got {}",
        extra
    );

    let blade = game.state.mods.get_blade_modifier(himeno);
    assert!(blade > 0, "Himeno should have gained blade");
}

// ---------------------------------------------------------------------------
// Test: Skip position change → no resources gained
// ---------------------------------------------------------------------------
#[test]
fn himeno_pb1_skip_no_resources() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let himeno = game.id("PL!HS-pb1-006-R");
    let mk = game.id("PL!HS-sd1-014-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [himeno, mk, filler];
    fill_decks(&mut game, filler);
    game.give_energy(11);

    advance_to_live_set(&mut game);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    drain_auto_abilities(&mut game);

    assert!(game.has_pending_choice());

    // For position choices, skip is available as a generated action
    // with allow_skip=true. Use select_generated to find the skip action.
    let all_actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let skip_action = all_actions
        .iter()
        .find(|a| a.action_type == rabuka_engine::game_setup::ActionType::ChoiceSkip);

    if let Some(skip) = skip_action {
        let p = skip.parameters.as_ref().unwrap();
        TurnEngine::resume_with_choice(&mut game.state, p.card_id, p.card_indices.clone())
            .expect("skip failed");
    } else {
        // Fallback: try select_option with skip
        panic!("No skip action found in generated actions");
    }

    // No position change should have occurred
    assert_eq!(
        game.state.player1.stage.stage[0], himeno,
        "Himeno still at Left"
    );
    assert_eq!(game.state.player1.stage.stage[1], mk, "mk still at Center");

    let blade = game.state.mods.get_blade_modifier(himeno);
    assert_eq!(blade, 0, "No blade gained (position change was skipped)");
}

// ---------------------------------------------------------------------------
// Test: No valid destinations → auto-skip, no choice offered
// ---------------------------------------------------------------------------
#[test]
fn himeno_pb1_no_valid_destinations_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let himeno = game.id("PL!HS-pb1-006-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [himeno, filler, filler];
    fill_decks(&mut game, filler);
    game.give_energy(11);

    advance_to_live_set(&mut game);
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);
    game.state.player1.main_deck.cards.clear();
    finish_live_setup(&mut game);

    drain_auto_abilities(&mut game);

    // No valid destinations → position change auto-skips
    if game.has_pending_choice() {
        let choice = game.get_pending_choice().clone();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "position|destination" =>
            {
                panic!("Should NOT have a position choice with no valid destinations");
            }
            _ => {}
        }
    }
}
