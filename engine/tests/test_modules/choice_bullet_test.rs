/// Choice/Bullet-point cards — engine fix validation.
use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor};
use rabuka_engine::core::types::Phase;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use std::collections::HashMap;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for p in [&mut game.state.player1, &mut game.state.player2] {
        p.main_deck.cards.clear();
        for _ in 0..50 {
            p.main_deck.cards.push(filler);
        }
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}
fn debut_play(game: &mut TestGame, card_id: i16, energy: usize, area: MemberArea) {
    fill_decks(game);
    game.state.player1.hand.cards.push(card_id);
    game.give_energy(energy);
    game.state.turn_number = 1;
    game.play_to_stage(card_id, area);
}

/// Dia (PL!S-bp5-004-R): Debut → choose 1 from {blade, position change}
/// Tests: execute_choice reads conditional_choice, executes selected option.
#[test]
fn dia_choose_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp5-004-R");
    let aqours = game.id("PL!S-bp2-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [aqours, -1, filler];
    debut_play(&mut game, dia, 15, MemberArea::Center);
    while game.state.has_pending_choice() {
        game.select_option(0);
    }
    while game.state.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.mods.get_blade_modifier(aqours) > 0,
        "Chika should have blade"
    );
}

/// Bouken (PL!S-bp6-020-L): LiveStart → choose 1 from 3
/// Tests: 3-option choice creation, option count verification.
#[test]
fn bouken_three_options() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let bouken = game.id("PL!S-bp6-020-L");
    let aqours = game.id("PL!S-bp2-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    game.state.player1.live_card_zone.cards.push(bouken);
    game.state.player1.stage.stage = [aqours, filler, -1];
    advance_to_live_card_set_p1(&mut game);

    while game.state.has_pending_choice() {
        if let Some(ref pc) = game.state.get_pending_choice_json() {
            if let Some(opts) = pc
                .as_object()
                .and_then(|j| j.get("options"))
                .and_then(|o| o.as_array())
            {
                assert_eq!(opts.len(), 3, "Should have exactly 3 options");
            }
        }
        game.select_option(0);
    }
}

/// Kotori (PL!-bp5-003-R+): 起動 — conditional_alternative
/// Discard μ's → look at top 4, add 2 to hand, discard rest.
#[test]
fn kotori_discard_muse_look_and_select() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    let kotori = game.id("PL!-bp5-003-R\u{ff0b}");
    let muse_card = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");

    for p in [&mut game.state.player1, &mut game.state.player2] {
        p.main_deck.cards.clear();
        for _ in 0..40 {
            p.main_deck.cards.push(filler);
        }
    }
    game.state.player1.stage.stage = [kotori, filler, -1];
    game.state.player1.hand.cards.push(muse_card);
    game.state.player1.waitroom.cards.push(live);
    game.give_energy(10);
    game.state.turn_number = 1;

    game.activate_ability(kotori);

    // Cost: discard 1 from hand
    assert!(game.has_pending_choice(), "Discard prompt expected");
    game.select_indices(&[0]);

    // Primary effect: look at top 4, select 2 to hand
    assert!(
        game.has_pending_choice(),
        "Look-and-select prompt expected for primary effect"
    );

    let choice = game.get_pending_choice();
    match choice {
        rabuka_engine::ability::types::Choice::SelectCard { zone, count, .. } => {
            assert_eq!(zone, "looked_at", "Should select from looked_at cards");
            assert_eq!(*count, 2, "Should select 2 cards");
        }
        _ => panic!("Expected SelectCard for look-and-select, got {:?}", choice),
    }

    game.select_indices(&[0, 1]);
    assert!(!game.has_pending_choice(), "No remaining prompts");

    // 2 looked-at cards moved to hand, 2 discarded
    assert_eq!(game.state.player1.hand.cards.len(), 2, "2 selected to hand");
    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "Live card stays in discard (primary effect ran, not alternative)"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "Live card stays in discard (primary effect ran, not alternative)"
    );
}

/// Kotori (PL!-bp5-003-R+): 起動 — conditional_alternative
/// Discard non-μ's → retrieve 1 live card from discard.
#[test]
fn kotori_discard_non_muse_retrieve_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    let kotori = game.id("PL!-bp5-003-R\u{ff0b}");
    let non_muse = game.id("PL!S-bp2-009-R");
    let live = game.id("PL!-sd1-019-SD");

    for p in [&mut game.state.player1, &mut game.state.player2] {
        p.main_deck.cards.clear();
        for _ in 0..40 {
            p.main_deck.cards.push(filler);
        }
    }
    game.state.player1.stage.stage = [kotori, filler, -1];
    game.state.player1.hand.cards.push(non_muse);
    game.state.player1.waitroom.cards.push(live);
    game.give_energy(10);
    game.state.turn_number = 1;

    game.activate_ability(kotori);

    // Cost: discard 1 from hand
    assert!(game.has_pending_choice(), "Discard prompt expected");
    game.select_indices(&[0]);

    // Alternative effect: retrieve live from discard (no further prompts)
    assert!(
        !game.has_pending_choice(),
        "No prompts after discard — alternative effect auto-retrieves"
    );

    assert!(
        game.state.player1.hand.cards.contains(&live),
        "Live card should be retrieved from discard"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "Retrieved card no longer in waitroom"
    );
}

/// Dia (PL!S-bp5-004-R): Choose position_change → source must be SaintSnow,
/// destination can be any position (not restricted to SaintSnow).
#[test]
fn dia_position_change_saintsnow_source_any_destination() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp5-004-R");
    let seira = game.id("PL!S-bp5-222-R"); // 理亞 (SaintSnow, auto: activate 2 energy on move)
    let filler = game.id("PL!-sd1-010-SD"); // Honoka (Printemps, non-SaintSnow)

    // Stage: left=理亞(SaintSnow), center=filler(non-SS), right=empty
    game.state.player1.stage.stage = [seira, filler, -1];
    debut_play(&mut game, dia, 10, MemberArea::RightSide);

    // Dia's 登場 → choice between blade (0) and position_change (1)
    assert!(game.has_pending_choice(), "Dia's 登場 choice expected");
    game.select_option(1);

    // Source selection: only left (理亞/SaintSnow) must be valid
    assert!(game.has_pending_choice(), "Source selection expected");
    let src_actions = game.generated_actions();
    assert_eq!(
        src_actions.len(),
        1,
        "Only SaintSnow member should be valid source, got: {:?}",
        src_actions
            .iter()
            .map(|a| a
                .parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .unwrap_or("?"))
            .collect::<Vec<_>>()
    );
    let src_area = src_actions[0]
        .parameters
        .as_ref()
        .and_then(|p| p.stage_area.as_deref());
    assert_eq!(
        src_area,
        Some("left"),
        "Only left (理亞/SaintSnow) should be selectable"
    );

    // Select left (理亞) as source
    game.select_generated(0);

    // Destination selection: must include center (non-SaintSnow filler)
    assert!(game.has_pending_choice(), "Destination selection expected");
    let dst_actions = game.generated_actions();
    let dst_areas: Vec<&str> = dst_actions
        .iter()
        .filter_map(|a| a.parameters.as_ref()?.stage_area.as_deref())
        .collect();
    assert!(
        dst_areas.contains(&"center"),
        "Center (non-SaintSnow) must be a valid destination, got: {:?}",
        dst_areas
    );
    assert!(
        dst_areas.contains(&"right"),
        "Right (Dia) must be a valid destination (any position), got: {:?}",
        dst_areas
    );

    // Select center as destination
    let center_idx = dst_actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("center"))
        .unwrap();
    game.select_generated(center_idx);

    // Drain any auto-ability choices triggered by the position change
    game.drain_auto_ability_choices();

    // Verify swap: 理亞 at center, filler at left
    assert_eq!(
        game.state.player1.stage.stage[0], filler,
        "Left should have filler (was at center)"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], seira,
        "Center should have 理亞 (moved from left)"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], dia,
        "Right should still have Dia"
    );
}

/// Dia (PL!S-bp5-004-R): Choose position_change — no SaintSnow on stage,
/// source options should be empty and the effect should skip gracefully.
#[test]
fn dia_position_change_no_saintsnow_skips_gracefully() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp5-004-R");
    let filler = game.id("PL!-sd1-010-SD");
    let filler2 = game.id("PL!-sd1-013-SD");

    // Stage: [filler, filler2, -] — no SaintSnow at all
    game.state.player1.stage.stage = [filler, filler2, -1];
    debut_play(&mut game, dia, 10, MemberArea::RightSide);

    // Dia's 登場 → choose position_change (option 1)
    assert!(game.has_pending_choice(), "Dia's 登場 choice expected");
    game.select_option(1);

    // No SaintSnow on stage → source selection should not appear
    // (valid_sources is empty → execute_position_change returns Ok(()) immediately)
    if game.has_pending_choice() {
        let actions = game.generated_actions();
        assert!(
            actions.is_empty(),
            "No source options expected with zero SaintSnow, got: {:?}",
            actions
                .iter()
                .map(|a| a
                    .parameters
                    .as_ref()
                    .and_then(|p| p.stage_area.as_deref())
                    .unwrap_or("?"))
                .collect::<Vec<_>>()
        );
    }

    // Verify stage unchanged
    assert_eq!(game.state.player1.stage.stage[0], filler);
    assert_eq!(game.state.player1.stage.stage[1], filler2);
    assert_eq!(game.state.player1.stage.stage[2], dia);
}

/// Dia (PL!S-bp5-004-R): Choose position_change — two SaintSnow members,
/// both should be selectable as source.
#[test]
fn dia_position_change_two_saintsnow_both_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp5-004-R");
    let seira1 = game.id("PL!S-bp5-222-R"); // 理亞 (SaintSnow)
    let seira2 = game.id("PL!S-bp5-222-R"); // second copy
    let _filler = game.id("PL!-sd1-010-SD");

    // Stage: [理亞1(SaintSnow), 理亞2(SaintSnow), -]
    game.state.player1.stage.stage = [seira1, seira2, -1];
    debut_play(&mut game, dia, 10, MemberArea::RightSide);

    // Choose position_change (option 1)
    assert!(game.has_pending_choice(), "Dia's 登場 choice expected");
    game.select_option(1);

    // Source selection: both left and center should be valid
    assert!(game.has_pending_choice(), "Source selection expected");
    let src_actions = game.generated_actions();
    assert_eq!(
        src_actions.len(),
        2,
        "Both SaintSnow members should be valid sources, got: {:?}",
        src_actions
            .iter()
            .map(|a| a
                .parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .unwrap_or("?"))
            .collect::<Vec<_>>()
    );
    let src_areas: Vec<&str> = src_actions
        .iter()
        .filter_map(|a| a.parameters.as_ref()?.stage_area.as_deref())
        .collect();
    assert!(src_areas.contains(&"left"), "Left should be selectable");
    assert!(src_areas.contains(&"center"), "Center should be selectable");

    // Select left (理亞1) as source
    game.select_generated(0);

    // Destination selection: must include center (理亞2), right (Dia)
    assert!(game.has_pending_choice(), "Destination selection expected");
    let dst_actions = game.generated_actions();
    let dst_areas: Vec<&str> = dst_actions
        .iter()
        .filter_map(|a| a.parameters.as_ref()?.stage_area.as_deref())
        .collect();
    assert!(
        dst_areas.contains(&"center"),
        "Center (理亞2) must be valid destination, got: {:?}",
        dst_areas
    );
    assert!(
        dst_areas.contains(&"right"),
        "Right (Dia) must be valid destination, got: {:?}",
        dst_areas
    );

    // Select center as destination
    let center_idx = dst_actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("center"))
        .unwrap();
    game.select_generated(center_idx);

    game.drain_auto_ability_choices();

    // Verify swap: 理亞1 at center, 理亞2 at left
    assert_eq!(
        game.state.player1.stage.stage[0], seira2,
        "Left should have 理亞2 (was at center)"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], seira1,
        "Center should have \u{7406}\u{4e9c}1 (moved from left)"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], dia,
        "Right should still have Dia"
    );
}

/// Trigger a card's ability directly by trigger type.
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
        "ライブ開始時" => rabuka_engine::core::types::AbilityTrigger::LiveStart,
        _ => rabuka_engine::core::types::AbilityTrigger::Auto,
    };
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.clone()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
}

/// Bouken (PL!S-bp6-020-L): LiveStart → choose gain_ability → LiveSuccess draws.
#[test]
fn bouken_gain_ability_draws_on_live_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let bouken = game.id("PL!S-bp6-020-L");
    let aqours = game.id("PL!S-bp2-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    let initial_hand = game.state.player1.hand.cards.len();
    game.state.player1.live_card_zone.cards.push(bouken);
    game.state.player1.stage.stage = [aqours, filler, -1];

    trigger_ability(&mut game, bouken, "ライブ開始時");

    // The LiveStart choice should appear
    while game.state.has_pending_choice() {
        game.select_option(0);
    }

    assert!(
        game.state.gained_card_abilities.contains_key(&bouken),
        "Gained ability should be stored"
    );
    let gained = &game.state.gained_card_abilities[&bouken];
    assert_eq!(gained.len(), 1);
    assert_eq!(
        gained[0].triggers.as_deref(),
        Some(rabuka_engine::triggers::LIVE_SUCCESS),
        "Trigger should be LIVE_SUCCESS"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        initial_hand,
        "No draw at LiveStart"
    );

    // Trigger LiveSuccess with hearts
    game.state.current_phase = Phase::LiveVictoryDetermination;
    let mut h = BaseHeart {
        hearts: HashMap::new(),
    };
    h.hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(h);

    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        initial_hand + 1,
        "Draw on LiveSuccess"
    );
    assert!(
        game.state.gained_card_abilities.contains_key(&bouken),
        "Gained ability persists after LiveSuccess"
    );
}

/// Bouken: LiveStart → gain_ability → no hearts → no draw.
#[test]
fn bouken_gain_ability_no_draw_on_failed_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let bouken = game.id("PL!S-bp6-020-L");
    let aqours = game.id("PL!S-bp2-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game);
    let initial_hand = game.state.player1.hand.cards.len();
    game.state.player1.live_card_zone.cards.push(bouken);
    game.state.player1.stage.stage = [aqours, filler, -1];

    trigger_ability(&mut game, bouken, "ライブ開始時");

    while game.state.has_pending_choice() {
        game.select_option(0);
    }

    assert!(game.state.gained_card_abilities.contains_key(&bouken));

    // No hearts → LiveSuccess should not trigger
    game.state.current_phase = Phase::LiveVictoryDetermination;
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        initial_hand,
        "No draw on failed live"
    );
}
