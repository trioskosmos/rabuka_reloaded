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

fn blade_mod(game: &TestGame, card_id: i16) -> i32 {
    game.state.mods.get_blade_modifier(card_id)
}

/// Accept an optional position change from a debut-triggered card on the field.
fn accept_position_swap(game: &mut TestGame, dest: &str) {
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

/// きな子 each_time blade stacking — no turn limit, every move = +2 blade.
#[test]
fn kinako_each_time_blade_stack_draft() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako_watcher = game.id("PL!SP-pb1-006-R");
    let kinako_mover = game.id("PL!SP-bp5-006-R");
    fill_deck_and_energy(&mut game);
    game.state.player1.stage.stage = [kinako_watcher, kinako_mover, -1];

    // === Move 1: 起動 きな子 swaps with watcher ===
    game.activate_ability(kinako_mover);
    println!(
        "  after activate: blade={}",
        blade_mod(&game, kinako_watcher)
    );
    game.drain_auto_ability_choices();
    println!("  after drain: blade={}", blade_mod(&game, kinako_watcher));
    let actions = game.generated_actions();
    let left_idx = actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("left"))
        .unwrap();
    println!(
        "  before select: blade={}",
        blade_mod(&game, kinako_watcher)
    );
    game.select_generated(left_idx);
    game.drain_auto_ability_choices();
    println!("Move 1: blade={}", blade_mod(&game, kinako_watcher));

    // === Move 2: Keke debut → swap with watcher ===
    let keke1 = game.id("PL!SP-bp4-013-N");
    game.state.player1.hand.cards.push(keke1);
    println!(
        "  before play Keke: blade={}",
        blade_mod(&game, kinako_watcher)
    );
    game.play_to_stage(keke1, MemberArea::RightSide);
    println!(
        "  after play Keke: blade={}",
        blade_mod(&game, kinako_watcher)
    );
    game.drain_auto_ability_choices();
    println!(
        "  after drain Keke: blade={}",
        blade_mod(&game, kinako_watcher)
    );
    accept_position_swap(&mut game, "center");
    println!("Move 2 (Keke1): blade={}", blade_mod(&game, kinako_watcher));

    // === Move 3: Mei debut → swap with watcher ===
    let mei = game.id("PL!SP-sd2-007-SD2");
    game.state.player1.hand.cards.push(mei);
    println!(
        "  before play Mei: blade={} turn_area_moves={}",
        blade_mod(&game, kinako_watcher),
        game.state.turn_area_movements.len()
    );
    game.play_to_stage(mei, MemberArea::LeftSide);
    println!(
        "  after play Mei: blade={} turn_area_moves={} pos_changes={}",
        blade_mod(&game, kinako_watcher),
        game.state.turn_area_movements.len(),
        game.state.position_change_events.len()
    );
    game.drain_auto_ability_choices();
    println!(
        "  after drain Mei: blade={}",
        blade_mod(&game, kinako_watcher)
    );
    accept_position_swap(&mut game, "right");
    println!(
        "Move 3 (Mei): blade={} turn_area_moves={}",
        blade_mod(&game, kinako_watcher),
        game.state.turn_area_movements.len()
    );

    let final_blade = blade_mod(&game, kinako_watcher);
    println!("\n=== FINAL BLADE: {} ===", final_blade);
    assert_eq!(
        final_blade, 8,
        "Each_time blade should stack: 4 triggers * 2 = 8"
    );
}

/// Both players play きな子 to stage — EACH should get +2 blade on 登場.
#[test]
fn both_players_kinako_appear_gets_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Each player gets their own instance of きな子 watcher
    let p1_kinako = game.id("PL!SP-pb1-006-R");
    let p2_kinako = game.id("PL!SP-pb1-006-R");
    // Each player gets their own mover
    let p1_mover = game.id("PL!SP-bp5-006-R");
    let p2_mover = game.id("PL!SP-bp5-006-R");
    fill_deck_and_energy(&mut game);
    // Fill deck for player2 too
    for _ in 0..40 {
        game.state
            .player2
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    for _ in 0..15 {
        game.state
            .player2
            .energy_zone
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    game.state.player2.energy_zone.set_active_count(15);

    // Stage: both players have きな子 watcher + mover
    game.state.player1.stage.stage = [p1_kinako, p1_mover, -1];
    game.state.player2.stage.stage = [p2_kinako, p2_mover, -1];

    // Player1: activate mover to swap with watcher → watcher moves → +2 blade
    game.activate_ability(p1_mover);
    game.drain_auto_ability_choices();
    let actions = game.generated_actions();
    let left_idx = actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("left"))
        .unwrap();
    game.select_generated(left_idx);
    game.drain_auto_ability_choices();
    println!("P1 blade after swap: {}", blade_mod(&game, p1_kinako));

    // Player1's きな子 should have +2 blade from the swap
    assert_eq!(
        blade_mod(&game, p1_kinako),
        2,
        "P1's きな子 should get +2 from position change"
    );

    // Player2's きな子 should have 0 (didn't move)
    assert_eq!(
        blade_mod(&game, p2_kinako),
        0,
        "P2's きな子 should NOT get blade (didn't move)"
    );

    // Now switch to player2's turn
    game.state.turn_number += 1;
    game.state.clear_card_movement_tracking();

    // Activate player2's mover
    // (activate_ability works on P1 by default — need P2 to activate their own)
    // For simplicity: simulate a movement via push_movement_event for P2's watcher
    game.state
        .push_movement_event(p2_kinako, "stage", "stage", Some(p2_mover), "p2", true);
    let pid = game.state.player2.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    println!(
        "P2 blade after move on turn 2: {}",
        blade_mod(&game, p2_kinako)
    );

    // Player2's きな子 should ALSO get +2 blade when it moves on its own turn
    assert_eq!(
        blade_mod(&game, p2_kinako),
        2,
        "P2's きな子 should get +2 from its own movement on turn 2"
    );
    // Player1's きな子 should still have 2 (not affected by P2's events)
    assert_eq!(
        blade_mod(&game, p1_kinako),
        2,
        "P1's きな子 blade unchanged by P2's events"
    );
}
