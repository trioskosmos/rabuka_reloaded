/// Q261: ニュートラル (PL!SP-pb2-049-L) — two ライブ成功時 abilities on the same card.
///
/// ab#0: If 5+ KALEIDOSCORE cards revealed during yell →
///       put 1 energy from energy deck to energy zone (wait).
/// ab#1: If 11+ energy → this card's score +1.
///
/// Both trigger on ライブ成功時. Q261 confirms the player can choose the order.
use crate::helpers::*;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..60 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn setup(game: &mut TestGame) -> i16 {
    let neutral = game.id("PL!SP-pb2-049-L");
    let st1 = game.id("PL!SP-sd1-010-SD"); // heart02=3, heart06=3, blade=4
    let st2 = game.id("PL!SP-bp1-019-N"); // heart02=2, heart03=1, heart06=2, blade=3
    let st3 = game.id("PL!SP-sd1-013-SD"); // heart06=2, blade=1, KALEIDOSCORE
    game.state.player1.stage.stage = [st1, st2, st3];
    game.state.player1.hand.cards.push(neutral);

    let kale = game.id("PL!SP-pb1-013-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(kale);
    }
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..50 {
        game.state.player2.main_deck.cards.push(filler);
    }

    let energy_card = game.id("LL-E-001-SD");
    for _ in 0..5 {
        game.state.player1.energy_deck.cards.push(energy_card);
    }
    game.give_energy(12);
    neutral
}

// 5 passes from Main → LiveCardSetFirstAttacker
fn advance_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

// 5 passes: LiveCardSetP2 → PerformanceP1 → PerformanceP2 → LiveVictoryDetermination
// (execute_live_victory_determination fires on the 5th pass)
fn advance_to_live_victory(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn drain_until_auto_ability(game: &mut TestGame) {
    for _ in 0..30 {
        if !game.has_pending_choice() {
            break;
        }
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => break,
            _ => {
                game.select_indices(&[]);
            }
        }
    }
}

// ====================================================================
// Test 1: Both abilities enqueue → SelectAutoAbility with 2 options
// ====================================================================
#[test]
fn q261_both_abilities_enqueue() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let neutral = setup(&mut game);

    advance_to_live_card_set(&mut game);
    game.set_live_card(neutral);
    advance_to_live_victory(&mut game);
    drain_until_auto_ability(&mut game);

    assert!(
        game.has_pending_choice(),
        "Q261: SelectAutoAbility should appear after LiveVictoryDetermination"
    );
    game.assert_pending_choice_type(
        "SelectAutoAbility",
        "Q261: Both LiveSuccess abilities should produce SelectAutoAbility",
    );

    if let rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } =
        game.get_pending_choice()
    {
        assert_eq!(
            options.len(),
            2,
            "Q261: Should have 2 options (ab#0 energy, ab#1 score+1), got {}",
            options.len()
        );
    }
}

// ====================================================================
// Test 2: Ability #1 first (score boost), then ability #0 (energy gain)
// ====================================================================
#[test]
fn q261_score_first_then_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let neutral = setup(&mut game);

    advance_to_live_card_set(&mut game);
    game.set_live_card(neutral);
    advance_to_live_victory(&mut game);
    drain_until_auto_ability(&mut game);

    game.assert_pending_choice_type("SelectAutoAbility", "SelectAutoAbility should appear");

    let score_idx = {
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } => {
                assert_eq!(options.len(), 2);
                options
                    .iter()
                    .position(|o| o.ability_text.contains("スコア"))
                    .unwrap() as i16
            }
            _ => unreachable!(),
        }
    };
    game.select_option(score_idx);

    let score_mod = game
        .state
        .mods
        .score_modifiers
        .get(&neutral)
        .map(|e| e.total())
        .unwrap_or(0);
    assert_eq!(score_mod, 1, "Score +1 from ab#1");

    // Observed: choosing an option from SelectAutoAbility starts queue processing
    // which auto-resolves ALL remaining queued abilities in order — no further prompts.
    assert!(
        !game.has_pending_choice(),
        "no prompt expected: remaining queued abilities auto-resolve after the order choice"
    );

    assert!(
        game.state.player1.energy_zone.cards.len() >= 13,
        "Energy should be >= 13 (12 base + 1 from ab#0)"
    );
}

// ====================================================================
// Test 3: Ability #0 first (energy gain), then ability #1 (score boost)
// ====================================================================
#[test]
fn q261_energy_first_then_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let neutral = setup(&mut game);

    advance_to_live_card_set(&mut game);
    game.set_live_card(neutral);
    advance_to_live_victory(&mut game);
    drain_until_auto_ability(&mut game);

    game.assert_pending_choice_type("SelectAutoAbility", "SelectAutoAbility should appear");

    let energy_idx = {
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } => {
                assert_eq!(options.len(), 2);
                options
                    .iter()
                    .position(|o| o.ability_text.contains("エール"))
                    .unwrap() as i16
            }
            _ => unreachable!(),
        }
    };
    game.select_option(energy_idx);

    // Observed: choosing an option from SelectAutoAbility starts queue processing
    // which auto-resolves ALL remaining queued abilities in order — no further prompts.
    assert!(
        !game.has_pending_choice(),
        "no prompt expected: remaining queued abilities auto-resolve after the order choice"
    );

    assert!(
        game.state.player1.energy_zone.cards.len() >= 13,
        "Energy should be >= 13 after ab#0 (12 base + 1)"
    );

    let score_mod = game
        .state
        .mods
        .score_modifiers
        .get(&neutral)
        .map(|e| e.total())
        .unwrap_or(0);
    assert_eq!(score_mod, 1, "Score +1 from ab#1 even when resolved second");
}

// ====================================================================
// Test 4: Edge case — only 10 energy (ab#1 condition NOT met)
// ====================================================================
#[test]
fn q261_energy_below_threshold() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let neutral = setup(&mut game);

    // Override to 10 energy (below 11+ threshold for ab#1)
    game.state.player1.energy_zone.cards.clear();
    game.state.player1.energy_zone.set_active_count(0);
    let ecard = game.id("LL-E-001-SD");
    for _ in 0..10 {
        game.state.player1.energy_zone.cards.push(ecard);
    }
    game.state.player1.energy_zone.add_active(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(neutral);
    advance_to_live_victory(&mut game);
    drain_until_auto_ability(&mut game);

    game.assert_pending_choice_type(
        "SelectAutoAbility",
        "Both abilities queue even with 10 energy (condition checked at resolution)",
    );
    if let rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } =
        game.get_pending_choice()
    {
        assert_eq!(options.len(), 2);
    }

    let score_idx = {
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } => options
                .iter()
                .position(|o| o.ability_text.contains("スコア"))
                .unwrap()
                as i16,
            _ => unreachable!(),
        }
    };
    game.select_option(score_idx);

    let score_mod = game
        .state
        .mods
        .score_modifiers
        .get(&neutral)
        .map(|e| e.total())
        .unwrap_or(0);
    assert_eq!(
        score_mod, 0,
        "ab#1 should NOT apply score +1 when only 10 energy"
    );

    // Observed: even though ab#1's condition failed at resolution time, choosing an
    // option from SelectAutoAbility starts queue processing which auto-resolves the
    // remaining queued abilities (ab#0 still moves 1 energy) — no further prompts.
    assert!(
        !game.has_pending_choice(),
        "no prompt expected: remaining queued abilities auto-resolve after the order choice"
    );

    assert!(
        game.state.player1.energy_zone.cards.len() > 10,
        "Energy should increase from ab#0"
    );
}

// ====================================================================
// Test 5: Live fails → no LiveSuccess triggers
// ====================================================================
#[test]
fn q261_live_fails_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let neutral = game.id("PL!SP-pb2-049-L");

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [filler, filler, filler];
    game.state.player1.hand.cards.push(neutral);
    fill_decks(&mut game, filler);
    game.give_energy(1);

    advance_to_live_card_set(&mut game);
    game.set_live_card(neutral);
    advance_to_live_victory(&mut game);

    assert!(
        !game.has_pending_choice(),
        "When live fails (need_heart unmet), no LiveSuccess abilities trigger"
    );
}

// ====================================================================
// Test 6: Both conditions satisfied, resolve both
// ====================================================================
#[test]
fn q261_both_conditions_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let neutral = setup(&mut game);

    advance_to_live_card_set(&mut game);
    game.set_live_card(neutral);
    advance_to_live_victory(&mut game);
    drain_until_auto_ability(&mut game);

    game.assert_pending_choice_type("SelectAutoAbility", "Both abilities should queue");
    if let rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } =
        game.get_pending_choice()
    {
        assert_eq!(options.len(), 2);
    }

    let energy_idx = {
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } => options
                .iter()
                .position(|o| o.ability_text.contains("エール"))
                .unwrap()
                as i16,
            _ => unreachable!(),
        }
    };
    game.select_option(energy_idx);

    // Observed: choosing an option from SelectAutoAbility starts queue processing
    // which auto-resolves ALL remaining queued abilities in order — no further prompts.
    assert!(
        !game.has_pending_choice(),
        "no prompt expected: remaining queued abilities auto-resolve after the order choice"
    );

    let score_mod = game
        .state
        .mods
        .score_modifiers
        .get(&neutral)
        .map(|e| e.total())
        .unwrap_or(0);
    assert_eq!(
        score_mod, 1,
        "Score should be +1 after both abilities resolve"
    );
}
