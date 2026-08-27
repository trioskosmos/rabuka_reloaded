//! Diagnostic regression test for the 村野さやか (PL!HS-bp1-002-R) activation
//! loop observed in bot_arena game transcripts (turn 6, 30+ activations in
//! one main phase). The ability costs {{E}}{{E}} AND sends itself to the
//! waitroom, retrieving a ≤15-cost 『蓮ノ空』 member into its area.
//!
//! Questions this pins down:
//!   1. Is the 2-energy cost actually charged per activation?
//!   2. Is the offer count bounded by affordable energy (≤ floor(E/2) + 1)?
//!   3. What lands on stage after each activation?

use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};

fn count_sayaka_offers(game: &TestGame, sayaka: i16) -> usize {
    let acts: Vec<_> = game_setup::generate_possible_actions(&game.state)
        .into_iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(sayaka)
        })
        .collect();
    for a in &acts {
        println!("DBG cost_struct={:?}", a.parameters.as_ref().map(|p| p.base_cost));
    }
    acts.len()
}

#[test]
fn sayaka_activation_charges_energy_and_terminates() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp1-002-R");

    game.state.player1.stage.stage[1] = sayaka;
    game.give_energy(9);

    // Waitroom full of legal retrieval targets (蓮ノ空 members ≤15 cost).
    // Hasunosora series cards from the same set.
    for no in ["PL!HS-bp1-002-R", "PL!HS-bp1-005-R", "PL!HS-bp1-006-P"] {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| game.id(no))) {
            Ok(cid) => game.add_to_discard(cid),
            Err(_) => {}
        }
    }

    let mut log: Vec<String> = Vec::new();
    let mut activations = 0usize;
    // Dump umi's cost structure for comparison (Q228 group-reduction shape).
    if let Some(card) = game.state.card_database.get_card(game.id("PL!-bp5-004-R＋")) {
        for (idx, ar) in card.abilities.iter().enumerate() {
            let ab = ar.resolve();
            println!("DBG umi ability[{idx}] cost={:?}", ab.cost);
        }
    }
    for step in 0..20 {
        let en_before = game.state.player1.energy_zone.active_count();
        let offers = count_sayaka_offers(&game, sayaka);
        if offers == 0 {
            log.push(format!("step {step}: no longer offered (en={en_before})"));
            break;
        }
        activations += 1;
        let stage_before = game.state.player1.stage.stage;
        let res = turn_engine_use(&mut game, sayaka);
        let en_after = game.state.player1.energy_zone.active_count();
        log.push(format!(
            "step {step}: offers={offers} en {en_before}->{en_after} ok={} stage {:?}-> {:?}",
            res.is_ok(),
            stage_before,
            game.state.player1.stage.stage
        ));
        if res.is_err() {
            break;
        }
        // resolve any pending choices with first option
        for _ in 0..10 {
            if !game.has_pending_choice() {
                break;
            }
            game.select_generated(0);
        }
        game_setup::settle_single_player_state(&mut game.state);
    }

    for l in &log {
        println!("{}", l);
    }
    assert!(
        activations <= 5,
        "activation ran {} times — expected it bounded by energy (9/2 ≈ 4)",
        activations
    );
}

// Local wrapper so we don't depend on helper signature drift.
fn turn_engine_use(game: &mut TestGame, card_id: i16) -> Result<(), String> {
    rabuka_engine::turn::TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(card_id),
        None,
        None,
        None,
    )
}

/// Rules-correct semantics (9.6.2.3): an unpayable mandatory cost is not a
/// legal activation — generation withholds it entirely and a direct press is
/// rejected by the affordability pre-check. Nothing ever resolves, so no
/// bot-side fizzle guard is needed anymore.
#[test]
fn sayaka_unaffordable_not_offered_and_press_rejected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-bp1-002-R");

    game.state.player1.stage.stage[1] = sayaka;
    game.give_energy(1); // cost is {{E}}{{E}} = 2
    let cid = game.id("PL!HS-bp1-005-R");
    game.add_to_discard(cid);

    // Generation withholds the unpayable activation.
    let offers = count_sayaka_offers(&game, sayaka);
    assert_eq!(offers, 0, "cost 2 with 1 active energy must not be offered");

    // A direct press is rejected cleanly — no partial resolution, no queue.
    let err = game.try_activate_ability(sayaka).unwrap_err();
    assert!(
        err.contains("cost") || err.contains("energy"),
        "expected affordability rejection, got: {}",
        err
    );
    assert!(!game.has_pending_choice(), "nothing may be left pending");
}
