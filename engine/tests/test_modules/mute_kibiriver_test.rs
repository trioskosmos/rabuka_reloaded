use crate::helpers::*;

/// 無敵級*ビリーバー (PL!N-bp5-029-L):
/// Live Start: If "中須かすみ" on stage → reveal 4 from deck top,
/// select 1 "中須かすみ" from them, gain that card's heart colors,
/// discard all revealed cards.
#[test]
fn mute_kibiriver_normal_flow() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kibiriver = game.id("PL!N-bp5-029-L");
    let kasumi = game.id("PL!N-bp5-002-R");

    game.state.player1.stage.stage = [kasumi, -1, -1];
    game.state.player1.live_card_zone.cards.push(kibiriver);
    game.give_energy(10);

    // Deck top: include a Kasumi card for selection (different copy from stage)
    for _ in 0..5 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!N-bp5-002-R"));
    }
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);

    // Handle pending choice — select a card
    while game.has_pending_choice() {
        let acts = game.generated_actions();
        if !acts.is_empty() {
            game.select_generated(0);
            game.drain_auto_ability_choices();
        } else {
            game.select_indices(&[]);
            game.drain_auto_ability_choices();
        }
    }

    // Debug: check all modifiers
    for color in &[
        rabuka_engine::card::HeartColor::Heart01,
        rabuka_engine::card::HeartColor::Heart03,
        rabuka_engine::card::HeartColor::Heart04,
        rabuka_engine::card::HeartColor::Heart05,
        rabuka_engine::card::HeartColor::Heart06,
    ] {
        let val = game.state.mods.get_heart_modifier(kasumi, *color);
        eprintln!(
            "[TEST_CHECK] card={} color={:?} modifier={}",
            kasumi, color, val
        );
    }

    // PL!N-bp5-002-R has heart03=3, heart04=1, heart05=1, heart06=1 in base_heart
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart01),
        0,
        "heart01 not in selected card → 0"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart03),
        1,
        "heart03 in selected card → +1"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart04),
        1,
        "heart04 in selected card → +1"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart05),
        1,
        "heart05 in selected card → +1"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart06),
        1,
        "heart06 in selected card → +1"
    );

    // All revealed cards discarded
    assert!(
        game.state.player1.waitroom.cards.len() >= 4,
        "Revealed cards should be discarded (got {})",
        game.state.player1.waitroom.cards.len()
    );
}

/// No Kasumi on stage → condition fails → no effect
#[test]
fn mute_kibiriver_no_kasumi_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kibiriver = game.id("PL!N-bp5-029-L");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.live_card_zone.cards.push(kibiriver);
    game.give_energy(10);

    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);

    assert!(!game.has_pending_choice(), "No Kasumi → no effect");
}

#[test]
fn mute_kibiriver_no_kasumi_in_revealed_no_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kibiriver = game.id("PL!N-bp5-029-L");
    let kasumi = game.id("PL!N-bp5-002-R");
    game.state.player1.stage.stage = [kasumi, -1, -1];
    game.state.player1.live_card_zone.cards.push(kibiriver);
    game.give_energy(10);
    let filler = game.id("PL!-sd1-010-SD");
    // Deck top 4: all filler, no Kasumi
    for _ in 0..4 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    // Reveal 4 filler, no Kasumi to select → should have no SelectCard or auto-skip
    // The ability should still discard the 4 revealed
    while game.has_pending_choice() {
        let acts = game.generated_actions();
        if !acts.is_empty() { game.select_generated(0); } else { game.select_indices(&[]); }
        game.drain_auto_ability_choices();
    }
    assert!(!game.has_pending_choice());
    assert!(game.state.player1.waitroom.cards.len() >= 4, "revealed filler should be discarded");
    // No heart should be gained because no Kasumi selected
    assert_eq!(game.state.mods.get_heart_modifier(kasumi, rabuka_engine::card::HeartColor::Heart03), 0);
}

#[test]
fn mute_kibiriver_multiple_kasumi_in_revealed_select_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kibiriver = game.id("PL!N-bp5-029-L");
    let kasumi = game.id("PL!N-bp5-002-R");
    game.state.player1.stage.stage = [kasumi, -1, -1];
    game.state.player1.live_card_zone.cards.push(kibiriver);
    game.give_energy(10);
    // Deck top 4: 2 Kasumi + 2 filler
    for _ in 0..2 { game.state.player1.main_deck.cards.push(game.new_id("PL!N-bp5-002-R")); }
    for _ in 0..2 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); }
    for _ in 0..10 { let f=game.id("PL!-sd1-010-SD"); game.state.player1.main_deck.cards.push(f); }
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    assert!(game.has_pending_choice(), "should have selection with multiple Kasumi");
    game.select_indices(&[0]);
    while game.has_pending_choice() { game.select_indices(&[]); game.drain_auto_ability_choices(); }
    assert!(!game.has_pending_choice());
    assert!(game.state.player1.waitroom.cards.len() >= 4);
}
