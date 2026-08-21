//! Regression test: ターン1回 activation abilities must be offered/executable
//! exactly once per turn through the PUBLIC action pipeline.
//!
//! Background (bot arena, game "turn-1 draw" trace): v2 activated
//! 中須かすみ (PL!N-bp5-014-N)'s retrieval 起動 200+ times inside a single
//! main phase — the arena's stuck-guard killed those games as draws. The
//! generation side filters on `turn_limited_abilities_used` (game_setup.rs),
//! so if the action keeps being offered after a successful use, either the
//! use was never recorded or the filter is bypassed. This test pins the
//! end-to-end contract: generate → execute → re-generate must stop offering
//! the ability after its limit is consumed.
//!
//! Card under test: PL!N-bp5-014-N 中須かすみ
//!   {{起動}}{{ターン1回}} cost 2 energy + discard 1 from hand:
//!   add 1 『虹ヶ咲』 live card from waitroom to hand.

use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};
use rabuka_engine::turn::TurnEngine;

/// Drive one full UseAbility of `card_id` through the public pipeline,
/// resolving any pending choices by always taking the first generated
/// option. Returns the number of pipeline steps taken.
fn use_ability_resolving_choices(
    game: &mut TestGame,
    card_id: i16,
) -> Result<usize, String> {
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(card_id),
        None,
        None,
        None,
    )?;
    let mut steps = 1;
    for _ in 0..15 {
        // Drive automatic phases first; a queued ability resolves into a
        // choice here.
        game_setup::settle_single_player_state(&mut game.state);
        if !game.has_pending_choice() {
            break;
        }
        game.select_generated(0);
        steps += 1;
    }
    game_setup::settle_single_player_state(&mut game.state);
    Ok(steps)
}

fn count_kasumi_use_offers(game: &TestGame, kasumi: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(kasumi)
        })
        .count()
}

#[test]
fn kasumi_turn1_ability_offered_only_once_per_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp5-014-N");

    // Stage kasumi, fund the 2-energy cost, give her a hand card to discard.
    game.state.player1.stage.stage[1] = kasumi;
    game.give_energy(4);
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(filler);

    // A 虹ヶ咲 live card into the waitroom for the retrieval to find.
    let niji_live = game.id("PL!SP-SD2-025-SD2");
    game.add_to_discard(niji_live);

    // First activation: must be offered and must resolve.
    assert_eq!(
        count_kasumi_use_offers(&game, kasumi),
        1,
        "fresh turn should offer the ターン1回 ability exactly once"
    );
    use_ability_resolving_choices(&mut game, kasumi).expect("first activation failed");

    // The use must be recorded.
    let key = (kasumi, 0, game.state.turn_number);
    assert!(
        game.state.turn_limited_abilities_used.contains_key(&key),
        "use_limit must be recorded after a successful activation"
    );

    // THE REGRESSION: re-generation must NOT offer it again this turn.
    for attempt in 0..10 {
        let offers = count_kasumi_use_offers(&game, kasumi);
        assert_eq!(
            offers, 0,
            "attempt {attempt}: ターン1回 ability offered again after being consumed"
        );
        // Advance the main phase with a pass to see if any state change
        // resurrects the offer.
        game.pass();
        if game.state.current_phase != rabuka_engine::game_state::Phase::Main {
            break;
        }
    }
}

#[test]
fn kasumi_turn1_ability_available_again_next_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp5-014-N");

    game.state.player1.stage.stage[1] = kasumi;
    game.give_energy(4);
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(filler);
    let niji_live = game.id("PL!SP-SD2-025-SD2");
    game.add_to_discard(niji_live);

    use_ability_resolving_choices(&mut game, kasumi).expect("first activation failed");
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(kasumi, 0, game.state.turn_number)),
        "use recorded"
    );

    // Next turn: the limit resets and the ability is offered again.
    game.state.turn_number += 1;
    assert_eq!(
        count_kasumi_use_offers(&game, kasumi),
        1,
        "ターン1回 limit must reset on the next turn"
    );
}

/// Edge: with NO 虹ヶ咲 live in the waitroom the retrieval has no legal
/// target. Whatever the engine decides (don't offer / offer and fizzle),
/// it must TERMINATE: the bot-flow loop below must exit, never spin.
#[test]
fn kasumi_no_valid_target_terminates() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp5-014-N");

    game.state.player1.stage.stage[1] = kasumi;
    game.give_energy(4);
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(filler);
    // Waitroom intentionally left WITHOUT any 虹ヶ咲 live.
    let other_live = game.id("PL!SP-BP4-025-L");
    game.add_to_discard(other_live);

    // Bot-flow reproduction of the arena pathology: keep taking the
    // UseAbility action whenever it is offered, resolving choices with the
    // first option, all within ONE turn. Must terminate well under 20.
    let mut activations = 0usize;
    for _ in 0..20 {
        let offers = count_kasumi_use_offers(&game, kasumi);
        if offers == 0 {
            break;
        }
        activations += 1;
        let res = use_ability_resolving_choices(&mut game, kasumi);
        if res.is_err() {
            break; // engine refused — also fine, as long as we stop
        }
    }
    assert!(
        activations <= 1,
        "activation looped {activations} times in a single turn — \
         ターン1回 enforcement leaked"
    );
}

/// Edge: EXACT energy (2) must be enough; the activation consumes it and
/// the limit is recorded.
#[test]
fn kasumi_exact_energy_is_sufficient() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = game.id("PL!N-bp5-014-N");

    game.state.player1.stage.stage[1] = kasumi;
    game.give_energy(2); // exactly the cost
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(filler);
    let niji_live = game.id("PL!SP-SD2-025-SD2");
    game.add_to_discard(niji_live);

    assert_eq!(
        count_kasumi_use_offers(&game, kasumi),
        1,
        "affordable ability must be offered"
    );
    use_ability_resolving_choices(&mut game, kasumi).expect("activation with exact cost failed");
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(kasumi, 0, game.state.turn_number)),
        "use recorded after exact-cost activation"
    );
}

/// Edge: two DISTINCT かすみ instances each get their own ターン1回 budget.
#[test]
fn kasumi_limit_is_per_instance() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let k1 = game.new_id("PL!N-bp5-014-N");
    let k2 = game.new_id("PL!N-bp5-014-N");

    game.state.player1.stage.stage[0] = k1;
    game.state.player1.stage.stage[1] = k2;
    game.give_energy(8);
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..4 {
        game.add_to_hand(filler);
    }
    let niji_live = game.id("PL!SP-SD2-025-SD2");
    game.add_to_discard(niji_live);
    let niji_live2 = game.id("PL!SP-SD2-023-SD2");
    game.add_to_discard(niji_live2);

    // Both instances are offered independently.
    let offers = game_setup::generate_possible_actions(&game.state)
        .iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a
                    .parameters
                    .as_ref()
                    .and_then(|p| p.card_id)
                    .map_or(false, |cid| cid == k1 || cid == k2)
        })
        .count();
    assert_eq!(offers, 2, "each instance gets its own ターン1回 budget");

    // Using k1 must not consume k2's budget.
    use_ability_resolving_choices(&mut game, k1).expect("k1 activation failed");
    let k2_offers = count_kasumi_use_offers(&game, k2);
    assert_eq!(
        k2_offers, 1,
        "k2 must still be offered after k1 used its own activation"
    );
}
