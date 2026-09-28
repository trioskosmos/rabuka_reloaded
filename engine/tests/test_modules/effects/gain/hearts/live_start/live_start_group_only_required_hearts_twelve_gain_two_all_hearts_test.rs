use crate::helpers::*;
use rabuka_engine::card::HeartColor;

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
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
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

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

#[test]
fn group_only_required_hearts_twelve_live_start_single_live_exact_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let mw = game.id("PL!S-bp3-019-L");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.live_card_zone.cards.push(mw);
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, riko, "ライブ開始時");

    // Exactly +2 all-heart modifier
    let all = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(all, 2, "exactly 2 all-heart gained (single MIRACLE WAVE)");

    // Other colors unchanged
    let h02 = game
        .state
        .mods
        .get_heart_modifier(riko, HeartColor::Heart02);
    let h04 = game
        .state
        .mods
        .get_heart_modifier(riko, HeartColor::Heart04);
    let h05 = game
        .state
        .mods
        .get_heart_modifier(riko, HeartColor::Heart05);
    assert_eq!(h02, 0, "heart02 unchanged");
    assert_eq!(h04, 0, "heart04 unchanged");
    assert_eq!(h05, 0, "heart05 unchanged");
}

#[test]
fn group_only_required_hearts_twelve_live_start_total_twenty_four_exact_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let mw_a = game.id("PL!S-bp3-019-L");
    let mw_b = game.id("PL!S-bp3-019-L");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.live_card_zone.cards.push(mw_a);
    game.state.player1.live_card_zone.cards.push(mw_b);
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, riko, "ライブ開始時");

    let all = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(all, 2, "2× all-heart gained (two MIRACLE WAVEs, total 24)");
}

#[test]
fn group_only_required_hearts_twelve_live_start_non_aq_present_all_members_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let non_aq = game.id("PL!-bp5-019-L"); // μ's live
    let aq = game.id("PL!S-sd1-019-SD"); // Aqours live, 3 total

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.live_card_zone.cards.push(non_aq);
    game.state.player1.live_card_zone.cards.push(aq);
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, riko, "ライブ開始時");

    let all = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(
        all, 0,
        "non-Aqours in zone → all_members check fails → no hearts"
    );
}

#[test]
fn group_only_required_hearts_twelve_live_start_aggregate_below_threshold_no_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    // PL!S-sd1-019-SD: heart02:1+heart04:1+heart05:1 = 3, Aqours
    let live_a = game.id("PL!S-sd1-019-SD");
    let live_b = game.id("PL!S-sd1-019-SD");
    let live_c = game.id("PL!S-sd1-019-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.live_card_zone.cards.push(live_a);
    game.state.player1.live_card_zone.cards.push(live_b);
    game.state.player1.live_card_zone.cards.push(live_c);
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, riko, "ライブ開始時");

    let all = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(all, 0, "total 9 < 12 → condition fails → no hearts");
}

#[test]
fn group_only_required_hearts_twelve_live_start_mix_fills_3_slots_passes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let mw = game.id("PL!S-bp3-019-L"); // need_heart {02:4,04:4,05:4} = 12
    let sd_a = game.id("PL!S-sd1-019-SD"); // {02:1,04:1,05:1} = 3
    let sd_b = game.id("PL!S-sd1-019-SD"); // {02:1,04:1,05:1} = 3

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.live_card_zone.cards.push(mw);
    game.state.player1.live_card_zone.cards.push(sd_a);
    game.state.player1.live_card_zone.cards.push(sd_b);
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, riko, "ライブ開始時");

    let all = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(all, 2, "18 >= 12 with all 3 slots filled → +2 all-heart");
}

#[test]
fn group_only_required_hearts_twelve_live_start_empty_zone_no_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, riko, "ライブ開始時");

    let all = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(all, 0, "empty zone → no hearts");
}

#[test]
fn group_only_required_hearts_twelve_live_start_three_slots_all_aqours_passes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let mw_a = game.id("PL!S-bp3-019-L"); // need_heart {02:4,04:4,05:4} = 12
    let mw_b = game.id("PL!S-bp3-019-L");
    let sd = game.id("PL!S-sd1-019-SD"); // {02:1,04:1,05:1} = 3

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.live_card_zone.cards.push(mw_a);
    game.state.player1.live_card_zone.cards.push(mw_b);
    game.state.player1.live_card_zone.cards.push(sd);
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, riko, "ライブ開始時");

    let all = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(all, 2, "27 >= 12 with 3 Aqours live cards → +2 all-heart");
}

#[test]
fn required_hearts_twelve_gain_two_all_hearts_expires_at_live_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let mw = game.id("PL!S-bp3-019-L");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.hand.cards.push(mw);
    game.state.player1.hand.cards.push(filler);

    // Fill deck
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(mw);
    advance_to_live_start(&mut game);

    // Handle any pending choices from LiveStart triggers
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    // During live phase: exactly +2 all-heart
    let all = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(all, 2, "during live: +2 all-heart (MW total 12 >= 12)");

    // Advance: FirstAttackerPerformance → SecondAttackerPerformance
    game.pass();
    game.pass();

    // Advance: LiveVictoryDetermination
    game.pass();

    // Handle any pending choices (e.g. LiveSuccess triggers)
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    // Advance: LiveVictoryDetermination → Active
    game.pass();
    // check_expired_effects runs here, cleaning up live_end-temporary effects

    // All-heart must be cleared after LiveVictoryDetermination
    let all_after = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(
        all_after, 0,
        "all-heart must be cleared after LiveVictoryDetermination (duration=live_end)"
    );
}

#[test]
fn required_hearts_exact_twelve_gain_two_all_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let sd = game.id("PL!S-sd1-019-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    for _ in 0..4 {
        game.state.player1.live_card_zone.cards.push(sd);
    }
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, riko, "ライブ開始時");

    let all = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(all, 2, "4× SD (total 12) → +2 all-heart");
}

#[test]
fn required_hearts_nine_grants_no_all_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let sd = game.id("PL!S-sd1-019-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    for _ in 0..3 {
        game.state.player1.live_card_zone.cards.push(sd);
    }
    fill_decks(&mut game, filler);

    trigger_ability(&mut game, riko, "ライブ開始時");

    let all = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(all, 0, "3× SD (total 9) → condition fails, no hearts");
}

#[test]
fn live_start_required_hearts_gain_parses_as_all_heart_resource() {
    let db = load_real_database();
    let riko_card = db.get_card_id("PL!S-bp6-002-SEC").unwrap();
    let card = db.get_card(riko_card).unwrap();

    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("Riko BP6 should have ライブ開始時 ability");

    let effect = ab.effect.as_ref().expect("Should have an effect");
    assert_eq!(
        effect.action,
        rabuka_engine::ability::enums::ActionType::GainResource,
        "Action should be gain_resource"
    );
    assert_eq!(
        effect.resource_any(),
        Some("heart"),
        "Resource should be heart"
    );
    assert_eq!(
        effect.heart_type_any(),
        Some("all"),
        "heart_type should be all"
    );
}
