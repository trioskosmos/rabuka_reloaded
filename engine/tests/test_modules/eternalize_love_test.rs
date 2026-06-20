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
        "ライブ開始時" => rabuka_engine::core::types::AbilityTrigger::LiveStart,
        _ => rabuka_engine::core::types::AbilityTrigger::Auto,
    };
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid,
        Some(card.card_no.clone()),
        Some(card_id),
        None,
    );
    game.state
        .process_pending_auto_abilities(&game.state.player1.id.clone());
}

/// 2 same-named 虹ヶ咲 members on stage → condition passes.
/// Eternalize Love!! should get heart00 -3, NOT -6.
#[test]
fn eternalize_love_two_same_name_reduces_by_3_not_6() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id("PL!N-pb1-015-R");
    let live = game.id("PL!N-pb1-042-L");
    let filler = game.id("PL!-sd1-010-SD");

    // Two cards with the same name on stage
    game.state.player1.stage.stage = [member, member, -1];
    game.state.player1.live_card_zone.cards.push(live);
    fill_decks(&mut game, filler);

    // Trigger LiveStart ability
    trigger_ability(&mut game, live, "ライブ開始時");

    // Check the heart00 modifier on the live card
    let modifier = game
        .state
        .mods
        .get_need_heart_modifier(live, rabuka_engine::card::HeartColor::Heart00);
    assert_eq!(
        modifier, -3,
        "Expected -3 heart00 reduction (got {})",
        modifier
    );

    // Also verify no other colors were modified
    for hc in &[
        rabuka_engine::card::HeartColor::Heart01,
        rabuka_engine::card::HeartColor::Heart02,
        rabuka_engine::card::HeartColor::Heart03,
        rabuka_engine::card::HeartColor::Heart04,
        rabuka_engine::card::HeartColor::Heart05,
        rabuka_engine::card::HeartColor::Heart06,
    ] {
        let m = game.state.mods.get_need_heart_modifier(live, *hc);
        assert_eq!(m, 0, "Expected 0 modifier for {:?}", hc);
    }

    // Verify the original need_heart was 12 heart00, reduced to 9
    let card = game.db.get_card(live).unwrap();
    let need_heart = card.need_heart.as_ref().unwrap();
    let original_heart00 = need_heart
        .hearts
        .get(&rabuka_engine::card::HeartColor::Heart00)
        .copied()
        .unwrap_or(0);
    assert_eq!(original_heart00, 12, "Eternalize Love!! needs 12 heart00");
}

/// Full live flow: 2 same-named 虹ヶ咲 on stage → Eternalize Love!! set as live card
/// → advance to LiveStart → check modifier during and after performance.
#[test]
fn eternalize_love_full_live_flow_heart00_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id("PL!N-pb1-015-R");
    let live = game.id("PL!N-pb1-042-L");
    let filler = game.id("PL!-sd1-010-SD");

    // Two same-named 虹ヶ咲 on stage
    game.state.player1.stage.stage = [member, member, -1];
    // Put live card in hand for set_live_card
    game.state.player1.hand.cards.push(live);

    fill_decks(&mut game, filler);

    // Advance to LiveCardSet phase
    fn advance_to_live_card_set_p1(game: &mut TestGame) {
        for _ in 0..5 {
            game.pass();
        }
    }
    fn advance_to_live_start(game: &mut TestGame) {
        game.pass();
        game.pass();
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // Handle any pending choices from LiveStart
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Check modifier during performance
    let modifier = game
        .state
        .mods
        .get_need_heart_modifier(live, rabuka_engine::card::HeartColor::Heart00);
    println!("LIVE PHASE: need_heart modifier for heart00 = {}", modifier);

    // Advance through performance phases
    // FirstAttackerPerformance → SecondAttackerPerformance
    game.pass();
    game.pass();
    // LiveVictoryDetermination
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // LVD → Active (check_expired_effects runs)
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Check modifier after live
    let modifier_after = game
        .state
        .mods
        .get_need_heart_modifier(live, rabuka_engine::card::HeartColor::Heart00);
    println!(
        "AFTER LIVE: need_heart modifier for heart00 = {}",
        modifier_after
    );

    // After live end: modifier is -3 (correct single application).
    // Before the dedup fix this was -6 due to double-counting in snapshot restore.
    assert_eq!(
        modifier_after, -3,
        "After live end modifier is -3 (was -6 before fix)"
    );
}
