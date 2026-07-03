/// Tests for Q209 — discard as cost then retrieve from waitroom.
///
/// Q209: When using an ability, can you recover the live card you just
/// discarded as cost? Answer: Yes — cost is paid first (cards go to
/// waitroom), then the effect searches the waitroom.
///
/// Cards tested:
///   PL!HS-bp5-007-R/P/AR (セラス柳田リリエンフェルト) — debut(登場):
///     Discard 2 from hand → retrieve 1 EdelNote live from waitroom
///   PL!N-bp5-014-N (中須かすみ) — activation(起動):
///     Pay 2E + discard 1 → retrieve 1 虹ヶ咲 live from waitroom
use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::zones::MemberArea;

// ============================================================
// PL!HS-bp5-007-R (セラス柳田リリエンフェルト) — 登場 ability
// ============================================================
const CERAS: &str = "PL!HS-bp5-007-R";
const EDELIED_LIVE: &str = "PL!HS-pb1-030-L";
const FILLER_MEMBER: &str = "PL!-sd1-010-SD";

/// Q209: Discard an EdelNote live card as cost, then retrieve it back.
#[test]
fn q209_ceras_discard_edelnote_live_recover_same() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ceras = game.id(CERAS);
    let edel_live = game.id(EDELIED_LIVE);
    let filler = game.id(FILLER_MEMBER);

    game.state.player1.hand.cards.push(ceras);
    game.state.player1.hand.cards.push(edel_live);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    // Play Ceras to stage → debut triggers
    game.play_to_stage(ceras, MemberArea::Center);

    // Debut fires: choose to pay optional cost (discard 2 from hand)?
    // First choice: SelectAutoAbility for the debut trigger
    if game.has_pending_choice() {
        game.select_option(0); // trigger the debut ability
    }

    // Cost choice: select 2 cards from hand to discard
    // Hand has [edel_live, filler]. Select both (indices 0, 1)
    if game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }

    // Effect: select 1 EdelNote live from waitroom to retrieve
    if game.has_pending_choice() {
        // The retrieved card is the edel_live we just discarded
        game.select_indices(&[0]);
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&edel_live),
        "Q209: EdelNote live card discarded as cost should be retrievable"
    );
}

/// Discard non-EdelNote cards as cost, retrieve different EdelNote from waitroom.
#[test]
fn q209_ceras_discard_filler_retrieve_preexisting_edelnote() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ceras = game.id(CERAS);
    let edel_live = game.id(EDELIED_LIVE);
    let filler = game.id(FILLER_MEMBER);

    // Pre-place an EdelNote live card in waitroom
    game.state.player1.waitroom.cards.push(edel_live);

    game.state.player1.hand.cards.push(ceras);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.play_to_stage(ceras, MemberArea::Center);

    if game.has_pending_choice() {
        game.select_option(0); // trigger debut
    }

    // Discard 2 fillers from hand
    if game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }

    // Retrieve the pre-existing EdelNote live from waitroom
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&edel_live),
        "Pre-existing EdelNote live card should be retrievable"
    );
}

/// No EdelNote live in waitroom → effect skips gracefully.
#[test]
fn q209_ceras_no_edelnote_in_discard_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ceras = game.id(CERAS);
    let filler = game.id(FILLER_MEMBER);

    game.state.player1.hand.cards.push(ceras);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.play_to_stage(ceras, MemberArea::Center);

    if game.has_pending_choice() {
        game.select_option(0);
    }

    // Pay cost: discard 2 fillers
    if game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }

    // No EdelNote live in waitroom → no choice for retrieval
    // Hand: original hand was [ceras, filler, filler] = 3
    // After play_to_stage: ceras removed from hand, [filler, filler]
    // After cost discard 2: hand empty
    // After effect (no match): hand still empty
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "No cards recovered when no EdelNote live in waitroom"
    );
}

/// Decline the optional cost → no discard, no retrieval.
#[test]
fn q209_ceras_decline_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ceras = game.id(CERAS);
    let edel_live = game.id(EDELIED_LIVE);
    let filler = game.id(FILLER_MEMBER);

    game.state.player1.hand.cards.push(ceras);
    game.state.player1.hand.cards.push(edel_live);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.play_to_stage(ceras, MemberArea::Center);

    if game.has_pending_choice() {
        game.select_option(0); // trigger debut
    }

    // Cost is optional — select Skip to decline
    if game.has_pending_choice() {
        // The skip action has card_id=-1
        TurnEngine::resume_with_choice(&mut game.state, Some(-1), None).expect("skip");
    }

    // No cost paid → no effect → hand unchanged (minus ceras which is on stage)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "Hand should still have 2 cards when cost declined"
    );
}

/// Multiple EdelNote live cards in waitroom → choose 1.
#[test]
fn q209_ceras_multiple_edelnote_choose_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ceras = game.id(CERAS);
    let edel_a = game.id(EDELIED_LIVE);
    let edel_b = game.new_id(EDELIED_LIVE);
    let filler = game.id(FILLER_MEMBER);

    // Pre-place 2 EdelNote live cards in waitroom
    game.state.player1.waitroom.cards.push(edel_a);
    game.state.player1.waitroom.cards.push(edel_b);

    game.state.player1.hand.cards.push(ceras);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.play_to_stage(ceras, MemberArea::Center);

    if game.has_pending_choice() {
        game.select_option(0);
    }

    if game.has_pending_choice() {
        game.select_indices(&[0, 1]); // discard 2 fillers
    }

    // Effect: choose which EdelNote live to retrieve
    if game.has_pending_choice() {
        game.select_indices(&[1]); // pick the second one
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&edel_b),
        "Should have retrieved the selected EdelNote live card"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&edel_a),
        "Unselected EdelNote live should remain in waitroom"
    );
}

// ============================================================
// PL!N-bp5-014-N (中須かすみ) — 起動 ability
// ============================================================
const KASUMI: &str = "PL!N-bp5-014-N";
const NIJI_LIVE: &str = "PL!N-sd1-025-SD";

/// Q209: Discard a 虹ヶ咲 live card as activation cost, retrieve it back.
#[test]
fn q209_kasumi_discard_niji_live_recover_same() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kasumi = game.id(KASUMI);
    let niji_live = game.id(NIJI_LIVE);
    let filler = game.id(FILLER_MEMBER);

    game.state.player1.hand.cards.push(kasumi);
    game.state.player1.hand.cards.push(niji_live);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    // Play Kasumi to stage
    game.play_to_stage(kasumi, MemberArea::Center);

    // Activate ability
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kasumi),
        None,
        None,
        None,
    )
    .expect("activate");

    // Cost 1: pay 2 energy. The choice is SelectEnergy or similar.
    // Use generated actions to pay the energy cost.
    if game.has_pending_choice() {
        game.select_generated(0);
    }

    // Cost 2: discard 1 card from hand (the niji_live or filler)
    if game.has_pending_choice() {
        game.select_indices(&[0]); // discard first card (niji_live)
    }

    // Effect: retrieve 1 虹ヶ咲 live from waitroom
    if game.has_pending_choice() {
        game.select_indices(&[0]); // retrieve niji_live back
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // After activation: hand had [niji_live, filler] → discard niji_live → [filler] → retrieve niji_live → [filler, niji_live]
    assert!(
        game.state.player1.hand.cards.contains(&niji_live),
        "Q209: 虹ヶ咲 live card discarded as cost should be retrievable"
    );
}

/// No 虹ヶ咲 live in waitroom → effect skips.
#[test]
fn q209_kasumi_no_niji_in_discard_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kasumi = game.id(KASUMI);
    let filler = game.id(FILLER_MEMBER);

    game.state.player1.hand.cards.push(kasumi);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    game.play_to_stage(kasumi, MemberArea::Center);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kasumi),
        None,
        None,
        None,
    )
    .expect("activate");

    if game.has_pending_choice() {
        game.select_generated(0); // pay 2 energy
    }

    if game.has_pending_choice() {
        game.select_indices(&[0]); // discard filler
    }

    // No 虹ヶ咲 live in waitroom → no retrieval choice
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Hand: initially [kasumi, filler, filler] = 3
    // After play_to_stage: [filler, filler] = 2
    // After discard: [filler] = 1
    // After effect (no match): still [filler] = 1
    assert_eq!(game.state.player1.hand.cards.len(), 1);
}

/// No 虹ヶ咲 live in waitroom and energy is available → cost can be paid
/// but effect finds nothing → hand decreases by 1 (the discarded card).
#[test]
fn q209_kasumi_energy_available_no_target_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kasumi = game.id(KASUMI);
    let filler = game.id(FILLER_MEMBER);

    game.state.player1.hand.cards.push(kasumi);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    game.play_to_stage(kasumi, MemberArea::Center);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kasumi),
        None,
        None,
        None,
    )
    .expect("activate");

    // Pay 2 energy
    if game.has_pending_choice() {
        game.select_generated(0);
    }

    // Discard 1 card
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // No 虹ヶ咲 live in waitroom → no retrieval
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Hand: initially 3, after play: 2, after discard: 1, no retrieval: 1
    assert_eq!(game.state.player1.hand.cards.len(), 1);
}

/// Use limit: 1/turn — second activation does nothing.
#[test]
fn q209_kasumi_use_limit_blocks_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kasumi = game.id(KASUMI);
    let niji_live = game.id(NIJI_LIVE);
    let filler = game.id(FILLER_MEMBER);

    game.state.player1.hand.cards.push(kasumi);
    game.state.player1.hand.cards.push(niji_live);
    for _ in 0..5 {
        game.state.player1.hand.cards.push(filler);
    }
    game.give_energy(10);

    game.play_to_stage(kasumi, MemberArea::Center);

    // First activation — should succeed
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kasumi),
        None,
        None,
        None,
    )
    .expect("first activate");

    if game.has_pending_choice() {
        game.select_generated(0); // pay energy
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]); // discard
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]); // retrieve
    }
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Second activation — blocked by use_limit
    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kasumi),
        None,
        None,
        None,
    );

    assert!(result.is_err(), "Second activation should be blocked by use_limit");
}

/// Retrieve a 虹ヶ咲 live that was pre-existing in waitroom (not the discarded one).
#[test]
fn q209_kasumi_retrieve_different_niji_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kasumi = game.id(KASUMI);
    let niji_a = game.id(NIJI_LIVE);
    let niji_b = game.new_id(NIJI_LIVE);
    let filler = game.id(FILLER_MEMBER);

    // Pre-place one 虹ヶ咲 live in waitroom
    game.state.player1.waitroom.cards.push(niji_a);

    game.state.player1.hand.cards.push(kasumi);
    game.state.player1.hand.cards.push(niji_b);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);

    game.play_to_stage(kasumi, MemberArea::Center);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kasumi),
        None,
        None,
        None,
    )
    .expect("activate");

    if game.has_pending_choice() {
        game.select_generated(0); // pay energy
    }

    if game.has_pending_choice() {
        game.select_indices(&[0]); // discard niji_b
    }

    // Now waitroom has [niji_a, niji_b]. Choose niji_a (index 0).
    if game.has_pending_choice() {
        game.select_indices(&[0]); // retrieve niji_a
    }

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&niji_a),
        "Pre-existing niji_live (niji_a) should be retrievable"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&niji_b),
        "Discarded niji_b should stay in waitroom"
    );
}
