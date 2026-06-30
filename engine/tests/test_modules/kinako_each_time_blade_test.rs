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
