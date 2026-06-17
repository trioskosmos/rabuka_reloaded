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
    game.select_indices(&[0]);

    assert!(game.has_pending_choice(), "Should re-prompt for any_number");
    game.select_indices(&[0]);

    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&stage)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 2, "2 cards discarded → 2 blades, got {}", blade);
}

/// Verify generated actions at each re-prompt step for any_number cost with
/// characters filter. Before the fix, the re-prompt's filtered_indices
/// contained EXCLUDED (non-matching) indices, so the frontend saw zero
/// selectable cards and only "Skip" remained.
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

    // --- Step 1: Initial cost choice ---
    assert!(game.has_pending_choice(), "Optional cost should appear");
    let actions = generate_possible_actions(&game.state);
    let select_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == ActionType::ChoiceSelect)
        .collect();
    // 4 cards in hand: 2 matching copies + live + filler. The characters
    // filter (card_matches_characters) is already applied by the action
    // generator, so only the 2 joint card copies appear as selectable.
    assert_eq!(
        select_actions.len(),
        2,
        "Only 2 matching cards should be selectable initially"
    );

    // Select hand1 (index 0)
    game.select_indices(&[0]);

    // --- Step 2: Re-prompt after first discard ---
    // Hand now: [hand2, live, filler] at indices [0, 1, 2]
    // After fix: filtered_indices=[0] (hand2, the only matching card remaining)
    // Before fix: filtered_indices=[1,2] (live+filler, excluded → zero selectable)
    assert!(
        game.has_pending_choice(),
        "Re-prompt should appear after first discard"
    );
    let actions = generate_possible_actions(&game.state);
    let select_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == ActionType::ChoiceSelect)
        .collect();
    let skip_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == ActionType::ChoiceSkip)
        .collect();
    // Only hand2 matches characters; live and filler should be excluded
    assert_eq!(
        select_actions.len(),
        1,
        "Only 1 matching card should be selectable in re-prompt"
    );
    assert_eq!(skip_actions.len(), 1, "Skip should be available");
    // Verify the selectable card is hand2 (card_index should match its hand position)
    assert_eq!(
        select_actions[0]
            .parameters
            .as_ref()
            .and_then(|p| p.card_index),
        Some(0),
        "hand2 should be at index 0 in the re-prompt hand"
    );

    // Select hand2 (index 0 in current hand)
    game.select_indices(&[0]);

    // --- Step 3: All matching cards exhausted — code finalizes directly ---
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

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

    // First ability: discard 2 cards → 2 blades
    assert!(game.has_pending_choice(), "First cost should appear");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "Re-prompt after 1st discard");
    game.select_indices(&[]); // finalize with 1 card

    let p1_id = game.state.player1.id.clone();
    game.state.process_pending_auto_abilities(&p1_id);

    // Second ability: discard 1 card → 1 blade
    assert!(game.state.has_pending_choice(), "Second cost should appear");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "Re-prompt after 1st discard");
    game.select_indices(&[]);

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

/// Sequential any_number selection: pick one at a time, finalize on done.
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
    for _ in 0..3 {
        game.select_indices(&[0]);
        if game.has_pending_choice() {
            // re-prompt for next card — continue
        }
    }
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&stage)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 3, "3 cards discarded → 3 blades, got {}", blade);
}

/// Partial discard: discard 1 of 3 matching, then skip → 1 blade.
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
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "Re-prompt should appear");
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

    // Step 1: Initial cost prompt — 2 matching cards at indices 0 and 2
    assert!(game.has_pending_choice(), "Optional cost should appear");
    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let select_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == ActionType::ChoiceSelect)
        .collect();
    assert_eq!(
        select_actions.len(),
        2,
        "2 matching cards should be selectable"
    );

    // Select hand1 (index 0)
    game.select_indices(&[0]);

    // Step 2: Re-prompt — hand2 should be at filtered index 0 (zone position 1)
    assert!(game.has_pending_choice(), "Re-prompt should appear");
    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let select_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == ActionType::ChoiceSelect)
        .collect();
    assert_eq!(select_actions.len(), 1, "Only 1 matching card remains");
    // Verify the action's card_indices is the filtered index 0 (frontend standard)
    assert_eq!(
        select_actions[0]
            .parameters
            .as_ref()
            .and_then(|p| p.card_indices.as_deref()),
        Some(&[0usize][..]),
        "Action should use filtered index 0, not zone position 1"
    );

    // Select hand2 via filtered index 0
    game.select_indices(&[0]);

    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&stage)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 2, "2 cards discarded → 2 blades, got {}", blade);
}
