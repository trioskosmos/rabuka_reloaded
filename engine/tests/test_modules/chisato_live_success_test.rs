use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

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

fn accept_swap_to(game: &mut TestGame, dest: &str) {
    if !game.has_pending_choice() {
        return;
    }
    match game.get_pending_choice().clone() {
        Choice::SelectTarget { target, .. } if target == "position|destination" => {
            let acts = game.generated_actions();
            let idx = acts
                .iter()
                .position(|a| {
                    a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some(dest)
                })
                .unwrap_or(0);
            game.select_generated(idx);
            game.drain_auto_ability_choices();
        }
        _ => {
            game.select_indices(&[]);
            game.drain_auto_ability_choices();
        }
    }
}

/// Fire a LiveSuccess trigger for a card manually (same pattern as jimo_ai_dash_test).
fn trigger_live_success(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
}

fn score_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state.mods.get_score_modifier(card_id)
}

/// Liella! card swaps Chisato → she moved by Liella! effect → +1 on live success.
#[test]
fn chisato_liella_move_grants_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-pb2-003-R");
    let keke = game.id("PL!SP-bp4-013-N");

    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [chisato, -1, -1];
    game.state.player1.hand.cards.push(keke);

    game.play_to_stage(keke, MemberArea::RightSide);
    game.drain_auto_ability_choices();
    accept_swap_to(&mut game, "left");

    trigger_live_success(&mut game, chisato);

    assert_eq!(
        score_mod(&game, chisato),
        1,
        "Liella! effect moved Chisato → +1 score"
    );
}

/// Non-Liella! (藤島慈 HS-bp2-006-R, 蓮ノ空) formation change moves Chisato → NO bonus.
#[test]
fn chisato_non_liella_move_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-pb2-003-R");
    let chii = game.id("PL!HS-bp2-006-R"); // 蓮ノ空, not Liella!
    let filler = game.id("PL!-sd1-010-SD");

    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [chisato, filler, -1];
    game.state.player1.hand.cards.push(chii);

    // Play 慈 → debut triggers formation change (moves all members)
    game.play_to_stage(chii, MemberArea::RightSide);
    game.drain_auto_ability_choices();

    // Navigate formation change choices: move Chisato to a different position
    for _choice_idx in 0..3 {
        if !game.has_pending_choice() {
            break;
        }
        match game.get_pending_choice().clone() {
            Choice::SelectTarget { target, .. } if target == "position|destination" => {
                let acts = game.generated_actions();
                // Pick a destination that actually moves the card (not its current pos)
                let pick = acts
                    .iter()
                    .position(|a| {
                        a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("right")
                            || a.parameters.as_ref().and_then(|p| p.stage_area.as_deref())
                                == Some("center")
                    })
                    .unwrap_or(0);
                game.select_generated(pick);
                game.drain_auto_ability_choices();
            }
            _ => {
                game.select_indices(&[]);
                game.drain_auto_ability_choices();
            }
        }
    }

    trigger_live_success(&mut game, chisato);

    assert_eq!(
        score_mod(&game, chisato),
        0,
        "Non-Liella! effect (蓮ノ空) → no bonus"
    );
}

/// Opponent's effect moves Chisato → NO bonus (自分の = own card).
#[test]
fn chisato_opponent_move_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-pb2-003-R");

    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [chisato, -1, -1];
    game.state
        .push_movement_event(chisato, "stage", "stage", Some(chisato), "p2", true);

    trigger_live_success(&mut game, chisato);

    assert_eq!(score_mod(&game, chisato), 0, "Opponent effect → no bonus");
}

/// Natural movement (effect_only=false) → NO bonus.
#[test]
fn chisato_natural_move_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-pb2-003-R");

    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [chisato, -1, -1];
    game.state
        .push_movement_event(chisato, "stage", "stage", Some(-1), "p1", false);

    trigger_live_success(&mut game, chisato);

    assert_eq!(score_mod(&game, chisato), 0, "Natural move → no bonus");
}

/// No movement at all → NO bonus.
#[test]
fn chisato_no_move_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-pb2-003-R");

    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [chisato, -1, -1];

    trigger_live_success(&mut game, chisato);

    assert_eq!(score_mod(&game, chisato), 0, "No movement → no bonus");
}

/// Movement in PREVIOUS turn → NO bonus (temporal: this_turn).
#[test]
fn chisato_previous_turn_move_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-pb2-003-R");
    let keke = game.id("PL!SP-bp4-013-N");

    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [chisato, -1, -1];
    game.state.player1.hand.cards.push(keke);

    game.play_to_stage(keke, MemberArea::RightSide);
    game.drain_auto_ability_choices();
    accept_swap_to(&mut game, "left");

    game.state.turn_area_movements.clear();
    game.state.turn_number += 1;

    trigger_live_success(&mut game, chisato);

    assert_eq!(score_mod(&game, chisato), 0, "Previous turn → no bonus");
}

/// Same turn: Liella! move + live success → bonus applied.
#[test]
fn chisato_move_and_live_success_gives_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-pb2-003-R");
    let keke = game.id("PL!SP-bp4-013-N");

    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [chisato, -1, -1];
    game.state.player1.hand.cards.push(keke);

    game.play_to_stage(keke, MemberArea::RightSide);
    game.drain_auto_ability_choices();
    accept_swap_to(&mut game, "left");

    let has_move = game
        .state
        .turn_area_movements
        .iter()
        .any(|m| m.moved_card_id == chisato && m.effect_only && m.cause_player_id == "p1");
    assert!(
        has_move,
        "Chisato's movement should be in turn_area_movements"
    );

    trigger_live_success(&mut game, chisato);

    assert_eq!(score_mod(&game, chisato), 1, "Liella! move → +1 score");
}
