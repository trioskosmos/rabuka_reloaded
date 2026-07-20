use crate::helpers::*;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn trigger_ability(game: &mut TestGame, card_id: i16, trigger_str: &str) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .abilities
        .iter()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .cloned()
        .unwrap();
    let pid = game.state.player1.id.clone();
    let trigger = match trigger_str {
        "登場" => rabuka_engine::core::types::AbilityTrigger::Debut,
        "ライブ開始時" => rabuka_engine::core::types::AbilityTrigger::LiveStart,
        "起動" => rabuka_engine::core::types::AbilityTrigger::Activation,
        _ => rabuka_engine::core::types::AbilityTrigger::Auto,
    };
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
}

// =========================================================================
// PL!S-bp3-024-L — Deep Resonance (ab#0)
// ライブ開始時: 自分のステージのセンターエリアにコスト9以上の『Aqours』の
//   メンバーがいる場合、以下から1つを選ぶ。
//   - 自分のステージにいるメンバー1人は、ブレード+2を得る。
//   - 相手のステージにいるコスト4以下のメンバー1人をウェイトにする。
//
// PARSER FIX: cost_total was removed (it's a per-card cost check, not sum).
// ENGINE FIX: evaluate_comparison_condition now handles position:"center"
//   by checking the individual card's cost at that position.
// =========================================================================

fn make_test_game() -> TestGame {
    let db = load_real_database();
    TestGame::new(db)
}

// ── Positive condition tests ──

/// Condition passes: Aqours cost=9 at center → choice appears.
#[test]
fn deep_resonance_aqours_center_cost9_fires() {
    let mut game = make_test_game();
    let dr = game.id("PL!S-bp3-024-L");
    let aq_center = game.id("PL!S-bp2-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.live_card_zone.cards.push(dr);
    game.state.player1.stage.stage = [filler, aq_center, filler];
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, dr, "ライブ開始時");

    assert!(
        game.has_pending_choice(),
        "Aqours cost=9 at center → condition passes → choice"
    );
}

/// Aqours cost=9 at center with another Aqours at left (cost=13).
/// Only center matters for the condition.
#[test]
fn deep_resonance_aqours_center_cost9_ignores_left() {
    let mut game = make_test_game();
    let dr = game.id("PL!S-bp3-024-L");
    let aq_center = game.id("PL!S-bp2-001-R");
    let aq_left = game.id("PL!S-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.live_card_zone.cards.push(dr);
    game.state.player1.stage.stage = [aq_left, aq_center, filler];
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, dr, "ライブ開始時");

    assert!(
        game.has_pending_choice(),
        "Aqours cost=9 at center, Aqours cost=13 at left → condition passes (only center checked)"
    );
}

// ── Negative condition tests ──

/// Center empty → condition fails.
#[test]
fn deep_resonance_empty_center_no_fire() {
    let mut game = make_test_game();
    let dr = game.id("PL!S-bp3-024-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.live_card_zone.cards.push(dr);
    game.state.player1.stage.stage = [filler, -1, filler];
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, dr, "ライブ開始時");
    assert!(!game.has_pending_choice(), "empty center → condition fails");
}

/// Aqours cost=4 (< 9) at center → condition fails (below threshold).
#[test]
fn deep_resonance_aqours_cost4_below_threshold_no_fire() {
    let mut game = make_test_game();
    let dr = game.id("PL!S-bp3-024-L");
    let aq_low = game.id("PL!S-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.live_card_zone.cards.push(dr);
    game.state.player1.stage.stage = [filler, aq_low, filler];
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, dr, "ライブ開始時");

    assert!(
        !game.has_pending_choice(),
        "Aqours cost=4 < 9 at center → condition fails"
    );
}

/// Non-Aqours cost=11 at center (μ's) → condition fails (wrong group).
#[test]
fn deep_resonance_wrong_group_at_center_no_fire() {
    let mut game = make_test_game();
    let dr = game.id("PL!S-bp3-024-L");
    let non_aq = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.live_card_zone.cards.push(dr);
    game.state.player1.stage.stage = [filler, non_aq, filler];
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, dr, "ライブ開始時");

    assert!(
        !game.has_pending_choice(),
        "μ's cost=11 at center → group filter excludes → condition fails"
    );
}

/// Aqours cost=13 at Left (not center) → condition fails (wrong position).
#[test]
fn deep_resonance_aqours_cost13_at_left_no_fire() {
    let mut game = make_test_game();
    let dr = game.id("PL!S-bp3-024-L");
    let aq_left = game.id("PL!S-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.live_card_zone.cards.push(dr);
    game.state.player1.stage.stage = [aq_left, filler, filler];
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, dr, "ライブ開始時");

    assert!(
        !game.has_pending_choice(),
        "Aqours cost=13 at left, not center → condition fails"
    );
}

// ── Effect execution tests ──

/// Option 0: blade +2 to self member. Picks center card, verifies exact +2.
#[test]
fn deep_resonance_option_0_blade_gain_exact_modifier() {
    let mut game = make_test_game();
    let dr = game.id("PL!S-bp3-024-L");
    let aq_center = game.id("PL!S-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.live_card_zone.cards.push(dr);
    game.state.player1.stage.stage = [filler, aq_center, filler];
    fill_decks(&mut game, filler);

    let blade_before = game.state.mods.get_blade_modifier(aq_center);

    trigger_ability(&mut game, dr, "ライブ開始時");
    assert!(game.has_pending_choice(), "choice must appear");

    // Step 1: SelectTarget → pick option 0 (blade gain)
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "first choice: option selection"
    );
    game.select_option(0);

    // Step 2: SelectCard → pick which member gets the blade.
    // Indices map to stage: 0=Left, 1=Center, 2=Right.
    // Pick Center (aq_center).
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "second choice: member selection for blade gain"
    );
    game.select_indices(&[1]);

    let blade_after = game.state.mods.get_blade_modifier(aq_center);
    assert_eq!(
        blade_after,
        blade_before + 2,
        "center card gets exactly +2 blade (was {}, now {})",
        blade_before,
        blade_after
    );
    // The Left filler was not selected — must be unchanged
    assert_eq!(
        game.state.mods.get_blade_modifier(filler),
        0,
        "unselected filler at Left has blade modifier 0"
    );
}

/// Option 1: opponent member with cost <= 4 gets wait state.
/// (The sub-effect auto-selects the only valid opponent target when target_count=1.)
#[test]
fn deep_resonance_option_1_opponent_wait_exact_state() {
    let mut game = make_test_game();
    let dr = game.id("PL!S-bp3-024-L");
    let aq_center = game.id("PL!S-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.live_card_zone.cards.push(dr);
    game.state.player1.stage.stage = [filler, aq_center, filler];
    game.state.player2.stage.stage = [filler, -1, -1];
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, dr, "ライブ開始時");
    assert!(game.has_pending_choice(), "choice must appear");

    // SelectTarget → pick option 1 (opponent wait).
    // The sub-effect auto-selects the opponent filler (only valid target).
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "first choice: option selection"
    );
    game.select_option(1);

    // No further choice — effect auto-applied to opponent filler
    assert!(
        !game.has_pending_choice(),
        "no further choice: sub-effect auto-selected opponent filler"
    );

    let orientation = game.state.mods.get_orientation_modifier(filler);
    assert_eq!(
        orientation,
        Some("wait"),
        "opponent filler card should have 'wait' orientation modifier"
    );
}
