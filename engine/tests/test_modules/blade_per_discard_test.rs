use crate::helpers::*;
use rabuka_engine::game_setup::{generate_possible_actions, ActionType};

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// LL-bp2-001-R+ 渡辺曜&鬼塚夏美&大沢瑠璃乃 ab#2:
/// LiveStart: discard named characters from hand → gain 1 blade per discard.
/// new_id: distinct hand copy (name contains "渡辺曜") vs stage copy.
#[test]
fn triple_discard_gives_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let stage = game.id("LL-bp2-001-R+");
    let hand = game.new_id("LL-bp2-001-R+");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = stage;
    game.give_energy(15);
    game.state.player1.hand.cards.push(hand);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice(), "Optional cost should appear");
    game.select_indices(&[0]); // hand copy at index 0 — matches characters
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&stage)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 1, "1 card discarded → 1 blade, got {}", blade);
}

/// No matching cards → skip → 0 blades.
#[test]
fn triple_no_matching_zero_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id("LL-bp2-001-R+");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = triple;
    game.give_energy(15);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&triple)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 0, "No matching → 0 blades, got {}", blade);
}

#[test]
fn triple_discard_two_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let stage = game.id("LL-bp2-001-R+");
    let hand1 = game.new_id("LL-bp2-001-R+");
    let hand2 = game.new_id("LL-bp2-001-R+");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = stage;
    game.give_energy(15);
    game.state.player1.hand.cards.push(hand1);
    game.state.player1.hand.cards.push(hand2);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice(), "Optional cost should appear");
    // Select all matching cards in one batch, then skip to finalize
    game.select_indices(&[0, 1]);
    game.select_indices(&[]);

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&stage)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 2, "2 cards discarded → 2 blades, got {}", blade);
}

/// Verify generated actions for any_number cost with characters filter.
/// Only matching cards should appear as selectable in the initial prompt.
#[test]
fn triple_discard_re_prompt_shows_correct_actions() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let stage = game.id("LL-bp2-001-R+");
    let hand1 = game.new_id("LL-bp2-001-R+");
    let hand2 = game.new_id("LL-bp2-001-R+");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = stage;
    game.give_energy(15);
    // Hand: [hand1(match), hand2(match), live, filler]
    game.state.player1.hand.cards.push(hand1);
    game.state.player1.hand.cards.push(hand2);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // --- Initial cost choice ---
    assert!(game.has_pending_choice(), "Optional cost should appear");
    let actions = generate_possible_actions(&game.state);
    let select_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == ActionType::ChoiceSelect)
        .collect();
    // 4 cards in hand: 2 matching copies + live + filler. All 4 are shown,
    // but only the 2 matching copies are selectable (non-disabled).
    assert_eq!(select_actions.len(), 4, "All 4 cards should be visible");
    let selectable: Vec<_> = select_actions
        .iter()
        .filter(|a| a.parameters.as_ref().and_then(|p| p.disabled) != Some(true))
        .collect();
    assert_eq!(
        selectable.len(),
        2,
        "Only 2 matching cards should be selectable"
    );

    // Select both matching cards in one batch, then skip to finalize
    game.select_indices(&[0, 1]);
    game.select_indices(&[]);

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&stage)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 2, "2 cards discarded → 2 blades, got {}", blade);
}

/// Two copies of the joint card on stage: both should fire independently.
#[test]
fn two_copies_on_stage_both_gain_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let stage1 = game.id("LL-bp2-001-R+");
    let stage2 = game.new_id("LL-bp2-001-R+");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = stage1;
    game.state.player1.stage.stage[1] = stage2;
    game.give_energy(15);
    // Add 8 matching copies + live + filler = 10 hand cards
    for _ in 0..8 {
        game.state
            .player1
            .hand
            .cards
            .push(game.new_id("LL-bp2-001-R+"));
    }
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice(), "SelectAutoAbility should appear");
    game.select_option(0);

    // First ability: discard 1 card → 1 blade
    assert!(game.has_pending_choice(), "First cost should appear");
    game.select_indices(&[0]);
    game.select_indices(&[]); // skip re-prompt, finalize first ability

    let p1_id = game.state.player1.id.clone();
    game.state.process_pending_auto_abilities(&p1_id);

    // Second ability: discard 1 card → 1 blade
    assert!(game.state.has_pending_choice(), "Second cost should appear");
    game.select_indices(&[0]);
    game.select_indices(&[]); // skip re-prompt, finalize second ability

    let blade1 = game
        .state
        .mods
        .blade_modifiers
        .get(&stage1)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    let blade2 = game
        .state
        .mods
        .blade_modifiers
        .get(&stage2)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert!(
        blade1 > 0 && blade2 > 0,
        "Both copies should gain blades: copy1={} copy2={}",
        blade1,
        blade2
    );
    assert!(
        blade1 + blade2 >= 2,
        "Total blades should be at least 2: copy1={} copy2={}",
        blade1,
        blade2
    );
}

/// Sequential any_number selection: pick all cards in one batch.
#[test]
fn any_number_discard_sequential_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let stage = game.id("LL-bp2-001-R+");
    let hand1 = game.new_id("LL-bp2-001-R+");
    let hand2 = game.new_id("LL-bp2-001-R+");
    let hand3 = game.new_id("LL-bp2-001-R+");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = stage;
    game.give_energy(15);
    game.state.player1.hand.cards.push(hand1);
    game.state.player1.hand.cards.push(hand2);
    game.state.player1.hand.cards.push(hand3);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice(), "Optional cost should appear");
    // Select all 3 matching cards in one batch, then skip to finalize
    game.select_indices(&[0, 1, 2]);
    game.select_indices(&[]);

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&stage)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 3, "3 cards discarded → 3 blades, got {}", blade);
}

/// Partial discard: discard 1 of 3 matching → 1 blade (batch selection, no re-prompt).
#[test]
fn any_number_partial_discard_grants_partial_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let stage = game.id("LL-bp2-001-R+");
    let hand1 = game.new_id("LL-bp2-001-R+");
    let hand2 = game.new_id("LL-bp2-001-R+");
    let hand3 = game.new_id("LL-bp2-001-R+");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = stage;
    game.give_energy(15);
    game.state.player1.hand.cards.push(hand1);
    game.state.player1.hand.cards.push(hand2);
    game.state.player1.hand.cards.push(hand3);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice(), "Optional cost should appear");
    // Select 1 card in the batch, then skip to finalize
    game.select_indices(&[0]);
    game.select_indices(&[]);

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&stage)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 1, "1 card discarded → 1 blade, got {}", blade);
}

/// Edge case: non-contiguous matching cards in hand.
/// Hand: [hand1(match), live(no-match), hand2(match)]
/// After discarding hand1 (index 0): hand = [live(0), hand2(1)], fi = [1]
/// The re-prompt action should have card_indices=[0] (filtered index for hand2),
/// NOT card_indices=[1] (zone position), because the frontend sends filtered-relative indices.
/// Before the cost-phase Hand handler fix, this would silently skip hand2
/// (hand_cards[0] = live, validate_card fails → treated as skip).
#[test]
fn non_contiguous_matches_via_action_pipeline() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let stage = game.id("LL-bp2-001-R+");
    let hand1 = game.new_id("LL-bp2-001-R+");
    let hand2 = game.new_id("LL-bp2-001-R+");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = stage;
    game.give_energy(15);
    // Non-contiguous: matching cards at index 0 and 2, non-matching at 1
    game.state.player1.hand.cards.push(hand1);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(hand2);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // Initial cost prompt — 2 matching cards at indices 0 and 2
    assert!(game.has_pending_choice(), "Optional cost should appear");
    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let select_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == ActionType::ChoiceSelect)
        .collect();
    assert_eq!(select_actions.len(), 4, "4 total cards visible");
    let selectable: Vec<_> = select_actions
        .iter()
        .filter(|a| a.parameters.as_ref().and_then(|p| p.disabled) != Some(true))
        .collect();
    assert_eq!(selectable.len(), 2, "2 matching cards should be selectable");

    // Select matching cards one at a time (like the real game frontend),
    // then skip to finalize.
    game.select_indices(&[0]); // discard hand1 at hand index 0
    game.select_indices(&[0]); // re-prompt shows hand2 at filtered index 0, discard it
    game.select_indices(&[]); // skip to finalize

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&stage)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 2, "2 cards discarded → 2 blades, got {}", blade);
}
