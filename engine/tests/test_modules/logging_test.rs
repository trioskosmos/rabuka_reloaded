// Logging tests: the structured + rule log behavior behind the ability/rule
// logging overhaul.
//
// Covered:
//   1. A triggered ability, once resolved, does NOT leave a "pending" entry —
//      the trigger_evaluation entry is committed in place and the resolution
//      can be found.
//   2. Player choices are recorded with both the offered options and the
//      chosen outcome (ChoiceOffered / ChoiceResolved metadata).
//   3. Both log buffers are bounded (oldest dropped) so a long match cannot
//      grow them without limit.
use crate::helpers::*;
use rabuka_engine::core::game_state::{LOG_BOUND_RULE, LOG_BOUND_STRUCTURED};
use rabuka_engine::core::types::{LogEntry, LogMetadata};
use rabuka_engine::zones::MemberArea;

// 安養寺 姫芽 (PL!HS-sd1-006-SD) debut ability: with 大沢瑠璃乃 on stage, energy
// becomes active + a live card is fetched. Mirrors the himeno BP tests so we get
// a deterministic trigger → resolution (and no negate/skip path).
fn play_himeno(game: &mut TestGame) {
    let himeno = game.id("PL!HS-sd1-006-SD");
    let filler = game.id("PL!-sd1-013-SD");
    let osawa = game.id("PL!HS-sd1-003-SD");
    game.state.player1.stage.stage = [filler, filler, osawa];
    game.add_to_hand(himeno);
    game.add_to_hand(filler);
    game.give_energy(15);
    game.play_to_stage(himeno, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

#[test]
fn resolved_ability_leaves_no_pending() {
    let mut g = TestGame::new(load_real_database());
    play_himeno(&mut g);

    let himeno_id = g.id("PL!HS-sd1-006-SD");
    let himeno_name = g
        .state
        .card_database
        .get_card(himeno_id)
        .map(|c| c.name.to_string())
        .unwrap_or_default();

    let mut resolutions = 0;
    let mut pending_himeno = 0;
    for entry in &g.state.structured_log {
        match &entry.metadata {
            Some(LogMetadata::AbilityResolution { resolved: Some(true), .. }) => {
                resolutions += 1;
            }
            Some(LogMetadata::TriggerEvaluation { result, .. }) if result == "pending" => {
                if entry.source_card_name.as_deref() == Some(&himeno_name) {
                    pending_himeno += 1;
                }
            }
            _ => {}
        }
    }

    assert!(
        resolutions > 0,
        "expected at least one resolved ability; structured log:\n{}",
        format_structured(&g)
    );
    assert_eq!(
        pending_himeno, 0,
        "left an unresolved 'pending' trigger_evaluation for the played card:\n{}",
        format_structured(&g)
    );
}

#[test]
fn choice_metadata_preserves_offered_and_chosen() {
    use rabuka_engine::ability::types::Choice;
    let mut g = TestGame::new(load_real_database());

    // Drive the recording helper directly with a SelectCard choice so we verify
    // the offered vs chosen metadata regardless of a specific card line-up.
    let choice = Choice::SelectCard {
        zone: "hand".to_string(),
        card_type: None,
        count: 1,
        description: "choose a live card".to_string(),
        description_en: None,
        description_ja: None,
        allow_skip: true,
        cost_limit: None,
        cost_limit_operator: None,
        cost_total: None,
        cost_total_operator: None,
        cost_values: None,
        group: None,
        characters: None,
        filtered_indices: Some(vec![0]),
        is_select_action: false,
        heart_colors: Vec::new(),
        require_all_heart_colors: None,
        name_fragments: None,
        target_player_id: None,
        blind: false,
        is_reveal: false,
        picker: None,
        destination: None,
        discard_remaining: None,
    };
    g.state
        .push_choice_offered(&choice);
    g.state
        .push_choice_resolved(&choice, vec!["card".to_string()], false);

    let offered_entries: Vec<_> = g
        .state
        .structured_log
        .iter()
        .filter(|e| e.category == "choice_offered")
        .collect();
    let resolved_entries: Vec<_> = g
        .state
        .structured_log
        .iter()
        .filter(|e| e.category == "choice_resolved")
        .collect();
    assert_eq!(offered_entries.len(), 1, "expected one choice_offered entry");
    assert_eq!(resolved_entries.len(), 1, "expected one choice_resolved entry");
    assert!(
        !resolved_entries.is_empty(),
        "expected a choice_resolved entry; log:\n{}",
        format_structured(&g)
    );

    match &offered_entries[0].metadata {
        Some(LogMetadata::ChoiceOffered { offered, skip_allowed }) => {
            assert!(!offered.is_empty(), "offered options must be non-empty");
            assert!(*skip_allowed, "SelectCard with allow_skip=true reports skip");
        }
        other => panic!("expected ChoiceOffered metadata, got {:?}", other),
    }

    match &resolved_entries[0].metadata {
        Some(LogMetadata::ChoiceResolved {
            offered_count,
            chosen,
            skipped,
        }) => {
            assert!(*offered_count > 0, "offered_count must be non-zero");
            assert_eq!(chosen, &["card".to_string()], "chosen must be preserved");
            assert!(!*skipped, "not a skip for a card selection");
        }
        other => panic!("expected ChoiceResolved metadata, got {:?}", other),
    }
}

#[test]
fn shared_log_never_leaks_private_zone_card_names() {
    // Per rules 4.1.2.2/4.1.2.3 & 4.8/4.9/4.11, hand, main-deck and
    // energy-deck are 非公開領域 (private) — their card *identities* must never
    // appear in the shared rule_log / structured_log. Only the activating
    // (stage) card may be named. Verify that a SelectCard choice over "hand"
    // and "deck" records neutral placeholder labels, not real card names.
    use rabuka_engine::ability::types::Choice;
    let mut g = TestGame::new(load_real_database());
    let secret = g.id("PL!HS-sd1-006-SD");
    let secret_name = g
        .state
        .card_database
        .get_card(secret)
        .map(|c| c.name.to_string())
        .unwrap_or_default();

    // Give P1 a hand & deck containing the secret card so resolution can find it.
    g.state.player1.hand.cards.push(secret);
    g.state.player1.main_deck.cards.push(secret);

    // A private-zone SelectCard: offered labels must NOT contain secret_name.
    let hand_choice = Choice::SelectCard {
        zone: "hand".to_string(),
        card_type: None,
        count: 1,
        description: "choose from hand".to_string(),
        description_en: None,
        description_ja: None,
        allow_skip: true,
        cost_limit: None,
        cost_limit_operator: None,
        cost_total: None,
        cost_total_operator: None,
        cost_values: None,
        group: None,
        characters: None,
        filtered_indices: Some(vec![0]),
        is_select_action: false,
        heart_colors: Vec::new(),
        require_all_heart_colors: None,
        name_fragments: None,
        target_player_id: None,
        blind: false,
        is_reveal: false,
        picker: None,
        destination: None,
        discard_remaining: None,
    };
    let deck_choice = Choice::SelectCard {
        zone: "deck".to_string(),
        card_type: None,
        count: 1,
        description: "choose from deck".to_string(),
        description_en: None,
        description_ja: None,
        allow_skip: true,
        cost_limit: None,
        cost_limit_operator: None,
        cost_total: None,
        cost_total_operator: None,
        cost_values: None,
        group: None,
        characters: None,
        filtered_indices: Some(vec![0]),
        is_select_action: false,
        heart_colors: Vec::new(),
        require_all_heart_colors: None,
        name_fragments: None,
        target_player_id: None,
        blind: false,
        is_reveal: false,
        picker: None,
        destination: None,
        discard_remaining: None,
    };

    for choice in [&hand_choice, &deck_choice] {
        g.state.push_choice_offered(choice);
        g.state.push_choice_resolved(choice, vec!["#0".to_string()], false);
    }

    let all_lines: Vec<String> = g
        .state
        .structured_log
        .iter()
        .flat_map(|e| {
            let mut v = vec![e.text.clone()];
            if let Some(name) = &e.source_card_name {
                v.push(name.clone());
            }
            if let Some(LogMetadata::ChoiceOffered { offered, .. }) = &e.metadata {
                v.extend(offered.iter().cloned());
            }
            if let Some(LogMetadata::ChoiceResolved { chosen, .. }) = &e.metadata {
                v.extend(chosen.iter().cloned());
            }
            v
        })
        .collect();

    for line in &all_lines {
        assert!(
            !line.contains(&secret_name),
            "private-zone card name leaked into shared log: '{line}' (secret '{secret_name}')"
        );
    }
}

#[test]
fn real_choice_flow_emits_offered_and_resolved() {
    // "Awaken the power" (PL!S-bp5-023-L) live-start: Aqours + SaintSnow on
    // stage with combined cost >= 20 surfaces a real SelectCard choice. Resolve
    // it through the actual select_indices -> resume_with_choice path. This
    // proves the wiring (not just the helper methods) records both
    // choice_offered and choice_resolved entries in the structured log.
    use rabuka_engine::zones::MemberArea;

    let db = load_real_database();
    let mut g = TestGame::new(db);

    let awaken = g.id("PL!S-bp5-023-L");
    let aq = g.id("PL!S-sd1-001-SD"); // Aqours (cost 17)
    let ss = g.id("PL!S-bp5-222-R"); // SaintSnow (cost 11)
    let aq_live = g.id("PL!S-bp2-019-L"); // WATER BLUE NEW WORLD (Aqours live)
    for _ in 0..30 {
        g.state.player1.main_deck.cards.push(aq);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(aq);
    }
    g.add_to_hand(aq);
    g.add_to_hand(ss);
    g.give_energy(30);
    g.play_to_stage(aq, MemberArea::LeftSide);
    g.play_to_stage(ss, MemberArea::Center);
    g.add_to_discard(aq_live);
    g.add_to_hand(awaken);
    // Live phase setup, mirroring awaken_the_power_test's known-good sequence:
    for _ in 0..5 {
        g.pass();
    }
    g.set_live_card(awaken);
    g.pass();
    g.pass();
    g.drain_auto_ability_choices();

    assert!(
        g.has_pending_choice(),
        "expected a SelectCard pending choice from Awaken the power"
    );

    // The pending choice itself is the "offered" moment.
    assert!(
        g.state
            .structured_log
            .iter()
            .any(|e| e.category == "choice_offered"),
        "real play should have offered one choice; log:\n{}",
        format_structured(&g)
    );

    // Resolve via the real path: pick the first offered card.
    g.select_indices(&[0]);
    assert!(
        g.state
            .structured_log
            .iter()
            .any(|e| e.category == "choice_resolved"),
        "resolving the choice should produce a choice_resolved entry; log:\n{}",
        format_structured(&g)
    );
}

#[test]
fn buffers_are_bounded() {
    let mut g = TestGame::new(load_real_database());
    let total = LOG_BOUND_RULE + 300;
    for i in 0..total {
        g.state.push_rule_log(format!("line_{}", i));
        g.state.push_structured_log(LogEntry {
            text: format!("line_{}", i),
            turn: g.state.turn_number,
            player_label: "p1".to_string(),
            source_card_id: None,
            source_card_name: None,
            category: "test".to_string(),
            metadata: None,
        });
    }

    assert!(
        g.state.rule_log.len() <= LOG_BOUND_RULE,
        "rule_log grew unbounded: {} (cap {})",
        g.state.rule_log.len(),
        LOG_BOUND_RULE
    );
    assert!(
        g.state.structured_log.len() <= LOG_BOUND_STRUCTURED,
        "structured_log grew unbounded: {} (cap {})",
        g.state.structured_log.len(),
        LOG_BOUND_STRUCTURED
    );
    // Oldest dropped: the newest pushed line is retained.
    assert_eq!(
        g.state.rule_log.last().map(|s| s.as_str()),
        Some(format!("line_{}", total - 1).as_str()),
        "newest rule log line should be retained after truncation"
    );
}

#[test]
fn choice_resolved_metadata_is_trimmed_and_captures_chosen() {
    // After the trimming refactor, a `choice_resolved` entry stores
    // `offered_count` (not the full offered array) plus the non-empty `chosen`,
    // so resolved entries are compact and preserve what the player picked.
    use rabuka_engine::ability::types::Choice;
    let mut g = TestGame::new(load_real_database());
    g.state.player1.hand.cards.push(g.id("PL!HS-sd1-006-SD"));

    let choice = Choice::SelectCard {
        zone: "hand".to_string(),
        card_type: None,
        count: 1,
        description: "choose from hand".to_string(),
        description_en: None,
        description_ja: None,
        allow_skip: false,
        cost_limit: None,
        cost_limit_operator: None,
        cost_total: None,
        cost_total_operator: None,
        cost_values: None,
        group: None,
        characters: None,
        filtered_indices: Some(vec![0]),
        is_select_action: false,
        heart_colors: Vec::new(),
        require_all_heart_colors: None,
        name_fragments: None,
        target_player_id: None,
        blind: false,
        is_reveal: false,
        picker: None,
        destination: None,
        discard_remaining: None,
    };

    g.state.push_choice_offered(&choice);
    g.state.push_choice_resolved(&choice, vec!["#0".to_string()], false);

    let resolved = g
        .state
        .structured_log
        .iter()
        .find(|e| e.category == "choice_resolved")
        .expect("expected a choice_resolved entry");

    match &resolved.metadata {
        Some(LogMetadata::ChoiceResolved {
            offered_count,
            chosen,
            skipped,
        }) => {
            assert!(*offered_count > 0, "offered_count must be non-zero");
            assert_eq!(chosen, &["#0".to_string()], "chosen must be preserved");
            assert!(!*skipped, "not a skip");
        }
        other => panic!("expected ChoiceResolved metadata, got {:?}", other),
    }
}

fn format_structured(g: &TestGame) -> String {
    g.state
        .structured_log
        .iter()
        .map(|e| format!("[{}] {}", e.category, e.text))
        .collect::<Vec<_>>()
        .join("\n")
}