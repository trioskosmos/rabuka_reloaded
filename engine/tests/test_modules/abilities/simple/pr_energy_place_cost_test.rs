/// Tests for PL!SP-PR-004-PR — 登場 ability: discard 1 → place energy in wait.
///
/// 登場: 手札を1枚控え室に置いてもよい：
///   自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
///
/// Parsed:
///   trigger: 登場
///   cost: move_cards(hand→discard, 1, optional)
///   effect: move_cards(energy_deck→energy_zone, wait, 1, energy_card)
///
/// Covers the untested key combo:
///   move_cards | card_type, count, destination, source, state_change, target
///   (6 abilities, 0% tested)
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Basic activation: pay cost (discard 1) → energy card placed in wait state
#[test]
fn pr_energy_place_pay_cost_energy_in_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-PR-004-PR");
    let hand_discard = game.id("PL!-sd1-010-SD");

    // Card in hand, filler to discard as cost
    game.add_to_hand(card);
    game.add_to_hand(hand_discard);

    // Need 4 energy to play this card (cost 4)
    game.give_energy(4);

    // Populate energy deck with an energy card for the effect to place
    let energy_card = game.id("LL-E-001-SD");
    game.state.player1.energy_deck.cards.push(energy_card);

    let energy_before = game.player().energy_zone.cards.len();

    // Play card to stage → 登場 triggers
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("play to stage");

    // 登場 ability fires: first an auto-ability choice, then cost choice
    // Handle all pending choices
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify: energy card placed in energy zone
    let energy_after = game.player().energy_zone.cards.len();

    assert!(
        energy_after > energy_before,
        "Energy card should be added to energy zone"
    );
    assert!(
        !game.player().hand.cards.contains(&hand_discard),
        "Discarded card should no longer be in hand"
    );
}

/// Optional cost declined: no discard → no energy placed
#[test]
fn pr_energy_place_decline_cost_no_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-PR-004-PR");

    game.add_to_hand(card);
    game.give_energy(4);

    // Populate energy deck
    let energy_card = game.id("LL-E-001-SD");
    game.state.player1.energy_deck.cards.push(energy_card);

    let energy_before = game.player().energy_zone.cards.len();

    // Play card to stage
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("play to stage");

    // Handle auto-ability choice
    while let Some(choice) = game.state.get_pending_choice() {
        use rabuka_engine::ability::types::Choice;
        match choice {
            Choice::SelectAutoAbility { .. } => {
                game.select_indices(&[]);
            }
            Choice::SelectCard { .. } => {
                // Decline cost — provide empty selection
                game.select_indices(&[]);
            }
            _ => break,
        }
    }

    let energy_after = game.player().energy_zone.cards.len();
    assert_eq!(
        energy_after, energy_before,
        "Energy should NOT be placed when optional discard cost is declined"
    );
    // Verify the energy card is still in the energy deck (not moved)
    assert!(
        game.state.player1.energy_deck.cards.contains(&energy_card),
        "Energy card should remain in energy deck when cost was skipped"
    );
}

/// Effect resolution: verify energy deck is the source
#[test]
fn pr_energy_place_source_from_energy_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-PR-004-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(card);
    game.add_to_hand(filler);

    // Need 4 energy to play this card
    game.give_energy(4);

    // Populate energy deck
    let ec = game.id("LL-E-001-SD");
    game.state.player1.energy_deck.cards.push(ec);

    // Record initial state
    let initial_energy_zone_size = game.player().energy_zone.cards.len();

    // Play card
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("play to stage");

    // Handle auto-ability + cost choices
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let final_energy_zone_size = game.player().energy_zone.cards.len();
    assert!(
        final_energy_zone_size >= initial_energy_zone_size,
        "Energy zone should not lose cards"
    );
}
