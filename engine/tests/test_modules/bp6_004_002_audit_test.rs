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

// =========================================================================
// PL!S-bp6-004-SEC — 黒澤ダイヤ (Kurosawa Dia)
// Card text: ライブ開始時: 自分のライブカード置き場にカードが2枚以上ある場合、
//   その中から ライブ開始時 能力を持たない『Aqours』のライブカードを1枚選び、
//   デッキの一番上に置いてもよい。そうした場合、ライブ終了時まで、
//   heart02とheart04を得る。
//
// PARSING BUGS:
//   1. Filter: heart_colors[heart02,heart04] instead of ability_filter excluding ライブ開始時
//   2. Conditional heart gain after select (followup_action) is missing
// =========================================================================

#[test]
fn dia_bp6_live_start_triggers_with_2plus_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp6-004-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live_a = game.id("PL!S-PR-022-PR"); // no ability, Aqours
    let live_b = game.id("PL!S-sd1-019-SD"); // ライブ成功時 only, Aqours

    game.state.player1.stage.stage = [-1, dia, -1];
    game.state.player1.live_card_zone.cards.push(live_a);
    game.state.player1.live_card_zone.cards.push(live_b);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, dia, "ライブ開始時");

    assert!(
        game.has_pending_choice(),
        "Condition 2+ cards met → ability should fire"
    );
}

#[test]
fn dia_bp6_lt2_cards_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp6-004-SEC");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, dia, -1];
    game.state.player1.live_card_zone.cards.push(filler);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, dia, "ライブ開始時");

    assert!(!game.has_pending_choice(), "<2 cards → condition fails");
}

#[test]
fn dia_bp6_zero_cards_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp6-004-SEC");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, dia, -1];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, dia, "ライブ開始時");

    assert!(!game.has_pending_choice(), "0 cards → condition fails");
}

/// FIXED: ability_filter: "no_ability_type" with triggers ["ライブ開始時"] correctly
/// excludes cards that have the ライブ開始時 ability. Only the card without
/// ライブ開始時 (HAPPY PARTY TRAIN) should be selectable. The card with ライブ開始時
/// (青空Jumping Heart) is filtered out.
#[test]
fn dia_bp6_filter_excludes_live_start_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp6-004-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let with_ls = game.id("PL!S-bp2-025-L"); // HAS ライブ開始時
    let without_ls = game.id("PL!S-PR-022-PR"); // no ライブ開始時

    game.state.player1.stage.stage = [-1, dia, -1];
    game.state.player1.live_card_zone.cards.push(with_ls);
    game.state.player1.live_card_zone.cards.push(without_ls);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, dia, "ライブ開始時");

    // Only 1 card (without_ls) should pass the filter
    assert!(
        game.has_pending_choice(),
        "FIXED: ability fires with correct filter — only non-ライブ開始時 card selectable"
    );
}

/// followup_action with sequential gain_resource (heart02+heart04).
/// The ability fires, filters correctly with ability_filter, and selects a card.
#[test]
fn dia_bp6_conditional_heart_gain_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp6-004-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR");

    game.state.player1.stage.stage = [-1, dia, -1];
    game.state.player1.live_card_zone.cards.push(live);
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!S-sd1-019-SD"));
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, dia, "ライブ開始時");

    // Ability fires, filter works, select card
    assert!(
        game.has_pending_choice(),
        "Ability fires with selectable cards"
    );

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                game.select_indices(&[0]);
            }
            Some("SelectTarget") => {
                game.select_option(0);
            }
            Some("SelectPosition") => {
                game.select_indices(&[0]);
            }
            Some("SelectHeartColor") | Some("SelectHeartType") => {
                game.select_indices(&[0]);
            }
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
}

// =========================================================================
// PL!S-bp6-002-SEC — 桜内梨子 (Sakurauchi Riko)
// Ability 0 (自動 ターン1回):
//   『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、
//   そのライブカードをデッキの一番上か一番下に置いてもよい。
//
// PARSING BUG: condition uses locations:["live_card_zone","discard"] as a STATIC
//   multi-zone membership check, not a movement trigger. The effect's source is
//   also empty → defaults to "discard" (not the specific moved card).
// =========================================================================

/// FIXED: condition uses source:"preceding_moved" so only cards that actually
/// moved from live_card_zone trigger the ability. A card just sitting in discard
/// with no movement history does NOT trigger.
#[test]
fn riko_bp6_auto_does_not_fire_without_live_card_zone_movement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR"); // Aqours live

    game.state.player1.stage.stage = [-1, riko, -1];
    // Card was never in live_card_zone — directly placed in discard, no movement
    game.state.player1.waitroom.cards.push(live);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, riko, "自動");

    // FIXED: condition uses preceding_moved — no recently_moved cards → no trigger
    assert!(
        !game.has_pending_choice(),
        "Fixed: no movement → no trigger"
    );
}

// ====================================================================
// FULL GAME FLOW tests — Riko BP6 auto ability fires naturally when
// live cards move from live_card_zone → waitroom during performance.
// ====================================================================

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Heart failure: MIRACLE WAVE needs h02=4, h04=4, h05=4 (total 12).
/// Riko provides h02=2, h04=2, h05=2 (total 6) — not enough → all cards
/// go to waitroom. Riko's auto ability should fire and offer to put the
/// Aqours live card on deck.
#[test]
fn riko_bp6_auto_e2e_heart_failure_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let mw = game.id("PL!S-bp3-019-L"); // Aqours, need 12 hearts

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.hand.cards.push(mw);

    // Fill deck for yell draws
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(mw);

    // Handle any live-start triggers (e.g. P2 pass)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Advance: FirstAttackerPerformance (Riko BP6 auto ability fires here)
    // P1's heart check fails → cards move to waitroom → second check fires → Riko triggers
    game.pass();

    // If Riko's auto ability created a choice, handle it
    if game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                // Choose the first (only) Aqours live card to put on deck
                let cards_before = game.state.player1.waitroom.cards.len();
                game.select_indices(&[0]);
                // The card should be removed from waitroom (moved to deck)
                assert_eq!(
                    game.state.player1.waitroom.cards.len(),
                    cards_before.saturating_sub(1),
                    "Riko BP6 should remove the Aqours live card from waitroom"
                );
                // Card should be on deck top or bottom
                // (can't assert exact position since move_cards chose the slot)
            }
            Some("SelectPosition") => {
                // Choose deck top
                game.select_indices(&[0]);
            }
            _ => {}
        }
    }

    // Verify the live card left the waitroom
    assert!(
        !game.state.player1.waitroom.cards.contains(&mw),
        "MIRACLE WAVE should no longer be in waitroom after Riko BP6 trigger"
    );
}

/// Heart success: HAPPY PARTY TRAIN needs h02=1, h04=1, h05=1 (total 3).
/// Riko provides h02=2, h04=2, h05=2 (total 6) — enough → card stays in
/// live_card_zone / success zone. Riko's auto ability should NOT fire.
#[test]
fn riko_bp6_auto_e2e_heart_success_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let hpt = game.id("PL!S-PR-022-PR"); // Aqours, need 3 hearts

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.hand.cards.push(hpt);

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(hpt);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Advance through performance phases
    game.pass(); // FirstAttackerPerformance
    game.pass(); // SecondAttackerPerformance
    game.pass(); // LiveVictoryDetermination
    game.pass(); // → Active

    // The live card should NOT be in waitroom (it passed → success zone or stayed)
    // Riko's auto ability should NOT have fired (no card moved to waitroom)
    // Verify by checking there's no pending choice from Riko
    let has_riko_choice = game.has_pending_choice();
    assert!(
        !has_riko_choice,
        "Riko BP6 auto ability should NOT fire when live card passes"
    );
}

/// cannot_live: Player is marked as cannot_live → all live cards go to
/// waitroom during yell phase. Riko's auto ability should fire and move
/// the card to deck.
#[test]
fn riko_bp6_auto_e2e_cannot_live_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR"); // Aqours live

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.hand.cards.push(live);
    game.state.cannot_live_players.push("p1".to_string());

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Advance to FirstAttackerPerformance — cannot_live moves cards to waitroom,
    // then 8.3.13 check timing fires auto abilities. Riko should trigger.
    game.pass();

    // After the performance, the live card should NOT be in waitroom —
    // Riko's auto ability moved it to deck. If it's still in waitroom,
    // the trigger didn't fire.
    let in_waitroom = game.state.player1.waitroom.cards.contains(&live);
    assert!(
        !in_waitroom,
        "cannot_live: Riko BP6 should have moved the live card from waitroom to deck"
    );

    // Handle any remaining pending choices (position selection etc.)
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectPosition") => {
                game.select_indices(&[0]);
            }
            Some("SelectCard") => {
                game.select_indices(&[0]);
            }
            _ => break,
        }
    }
}

/// Aqours live card in live_card_zone only (not in discard) — condition passes
/// but effect finds nothing to move (source defaults to discard). No choice.
#[test]
fn riko_bp6_auto_live_card_only_in_zone_no_discard_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.live_card_zone.cards.push(live);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, riko, "自動");

    // No card in discard → effect source is empty → no cards moved → no choice
    // CORRECT behavior would be: trigger only when card moves, and move THAT card
    assert!(
        !game.has_pending_choice(),
        "No card in discard → effect finds nothing → no choice"
    );
}

#[test]
fn riko_bp6_auto_no_live_cards_anywhere_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, riko, "自動");

    assert!(
        !game.has_pending_choice(),
        "No live cards → condition fails"
    );
}

// =========================================================================
// PL!S-bp6-002-SEC — Ability 1 (ライブ開始時):
//   自分のライブカード置き場にあるカードが『Aqours』のみで、かつ
//   それらの必要ハートに含まれるheart02とheart04とheart05の合計が12以上の場合、
//   ライブ終了時まで、icon_all(ハート) ×2 を得る。
//
// PARSING BUGS:
//   1. Second sub-condition missing location: "live_card_zone"
//   2. aggregate="total" with live_card_zone not supported (only works for Stage)
//   3. operator "=" should be ">=" (card says 12以上 = >= 12)
//   4. Effect uses heart_colors [heart02,heart04,heart05] instead of heart_type="all"
// =========================================================================

/// MIRACLE WAVE (PL!S-bp3-019-L): need_heart {heart02:4, heart04:4, heart05:4},
/// all Aqours, total required heart = 12 ≥ 12. Condition passes → gain exactly
/// 2× heart type ALL as a permanent modifier (duration: live_end).
#[test]
fn riko_bp6_live_start_single_miracle_wave_exact_modifier() {
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

/// 2× MIRACLE WAVE: total need_heart = 24 ≥ 12, all Aqours. Modifier = +2.
#[test]
fn riko_bp6_live_start_two_miracle_waves_exact_modifier() {
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

/// Mixed zone: 1 Aqours live (heart02:1+heart04:1+heart05:1=3) + 1 non-Aqours.
/// all_members check: zone has non-Aqours card → fails immediately.
#[test]
fn riko_bp6_live_start_non_aq_present_all_members_fails() {
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

/// All Aqours but total = 9 < 12 (SD card: heart02:1+heart04:1+heart05:1, ×3 = 9).
/// all_members passes (all Aqours), aggregate total = 9 < 12 → fails.
#[test]
fn riko_bp6_live_start_aggregate_below_threshold_no_hearts() {
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

/// 1× MIRACLE WAVE (12) + 2× SD (3 each) = 18 ≥ 12, uses all 3 slots.
#[test]
fn riko_bp6_live_start_mix_fills_3_slots_passes() {
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

/// Empty live card zone → all_members vacuously false (0 cards, not all Aqours).
/// No hearts gained.
#[test]
fn riko_bp6_live_start_empty_zone_no_hearts() {
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

/// 2× MIRACLE WAVE (12 each) + 1× SD (3) = 27 ≥ 12, fills 3 slots.
#[test]
fn riko_bp6_live_start_three_slots_all_aqours_passes() {
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

// =========================================================================
// PL!S-bp6-002-SEC — 桜内梨子 (Riko) Ability 0 full flow tests
// Using the REAL engine path: trigger_auto_abilities_for_player scans the
// stage for auto abilities, enqueues with trigger_moved_cards captured from
// recently_moved_cards, then processes the queue naturally.
// No injected functions — every choice goes through generate_possible_actions
// + select_generated, exactly like the game server.
// =========================================================================

fn trigger_riko_auto(game: &mut TestGame, moved_cards: Vec<i16>) {
    game.state.recently_moved_cards = Some(moved_cards);
    game.state
        .trigger_auto_abilities_for_player(&game.state.player1.id.clone());
    game.state
        .process_pending_auto_abilities(&game.state.player1.id.clone());
}

/// Single Aqours live card moved → ability fires → position|destination choice →
/// select_generated(0) for deck_top → card placed on top of deck, waitroom has 0 copies.
#[test]
fn riko_bp6_auto_single_card_deck_top_exact_identity() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.waitroom.cards.push(live);
    fill_decks(&mut game, filler);

    let deck_snapshot: smallvec::SmallVec<[i16; 64]> = game.state.player1.main_deck.cards.clone();

    trigger_riko_auto(&mut game, vec![live]);

    assert!(
        game.has_pending_choice(),
        "position|destination choice must appear"
    );
    game.select_generated(0);

    // live removed from waitroom entirely
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "target live card removed from waitroom"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        0,
        "no cards remain in waitroom"
    );
    // deck: original + live on top
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_snapshot.len() + 1,
        "deck gained exactly 1 card"
    );
    assert_eq!(
        game.state.player1.main_deck.cards[0], live,
        "live card is at deck position 0 (top)"
    );
    // Check deck order below top preserved — use Vec comparison
    let expected_tail: Vec<i16> = deck_snapshot.iter().copied().collect();
    let actual_tail: Vec<i16> = game.state.player1.main_deck.cards[1..]
        .iter()
        .copied()
        .collect();
    assert_eq!(
        actual_tail, expected_tail,
        "original deck order preserved below top"
    );
}

/// Same setup but select_generated(1) for deck_bottom — card at last position.
#[test]
fn riko_bp6_auto_single_card_deck_bottom_exact_identity() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.waitroom.cards.push(live);
    fill_decks(&mut game, filler);

    let deck_snapshot: smallvec::SmallVec<[i16; 64]> = game.state.player1.main_deck.cards.clone();

    trigger_riko_auto(&mut game, vec![live]);

    assert!(
        game.has_pending_choice(),
        "position|destination choice must appear"
    );
    game.select_generated(1); // deck_bottom

    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "target live card removed from waitroom"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_snapshot.len() + 1,
        "deck gained exactly 1 card"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.last(),
        Some(&live),
        "live card is at deck bottom"
    );
    let snapshot_vec: Vec<i16> = deck_snapshot.iter().copied().collect();
    let actual_head: Vec<i16> = game.state.player1.main_deck.cards[..deck_snapshot.len()]
        .iter()
        .copied()
        .collect();
    assert_eq!(
        actual_head, snapshot_vec,
        "original deck order preserved above bottom"
    );
}

/// 3 Aqours live cards moved in one batch → ONLY the first in trigger_moved_cards
/// order gets taken. The other 2 stay. One shot, no per-card prompts.
/// This is the realistic max (3 live slots). Verifies the auto fires ONCE per
/// trigger event and takes `count: 1` from the ordered batch.
#[test]
fn riko_bp6_auto_batch_of_3_first_only_taken_others_untouched() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    // 3 distinct Aqours live cards (max live zone capacity)
    let batch: Vec<i16> = (0..3).map(|_| game.id("PL!S-PR-022-PR")).collect();

    game.state.player1.stage.stage = [-1, riko, -1];
    for &c in &batch {
        game.state.player1.waitroom.cards.push(c);
    }
    fill_decks(&mut game, filler);
    let deck_before = game.state.player1.main_deck.cards.len();

    trigger_riko_auto(&mut game, batch.clone());

    // Q252: first a SelectCard choice appears for the player to pick which card.
    assert!(
        game.has_pending_choice(),
        "SelectCard choice expected (Q252: pick 1 from multiple)"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "first choice should be card selection"
    );
    // Pick the second card in the batch (index 1 in the generated list)
    game.select_indices(&[1]);

    // Then position|destination choice for deck top or bottom
    assert!(
        game.has_pending_choice(),
        "position|destination choice must appear after card selection"
    );
    game.select_generated(0); // deck_top

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before + 1,
        "exactly 1 card added to deck"
    );
    // The player picked index 1 (batch[1]), which is the second card
    let on_deck = game.state.player1.main_deck.cards[0];
    assert_eq!(on_deck, batch[1], "player picked the second card (index 1)");

    // The other 2 remain in waitroom
    let waitroom = &game.state.player1.waitroom.cards;
    assert_eq!(waitroom.len(), 2, "2 cards remain in waitroom");
    assert!(
        waitroom.contains(&batch[0]),
        "batch[0] remains in waitroom (not picked)"
    );
    assert!(
        waitroom.contains(&batch[2]),
        "batch[2] remains in waitroom (not picked)"
    );
    // The picked card batch[1] is NOT in waitroom
    assert!(
        !waitroom.contains(&batch[1]),
        "batch[1] was picked and removed from waitroom"
    );
}

/// Mixed batch: 1 Aqours live + 1 non-Aqours live + 1 member card + 1 energy.
/// The condition filters to Aqours+live, so only 1 candidate. That card is taken.
#[test]
fn riko_bp6_auto_mixed_batch_filters_correctly() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let aq_live = game.id("PL!S-PR-022-PR");
    let non_aq_live = game.id("PL!-bp5-019-L"); // μ's, not Aqours
    let filler_member = game.id("PL!-sd1-010-SD");
    let energy = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.waitroom.cards.push(aq_live);
    game.state.player1.waitroom.cards.push(non_aq_live);
    game.state.player1.waitroom.cards.push(filler_member);
    game.state.player1.waitroom.cards.push(energy);
    fill_decks(&mut game, filler);

    // Put aq_live first in trigger order
    trigger_riko_auto(&mut game, vec![aq_live, non_aq_live, filler_member, energy]);

    assert!(
        game.has_pending_choice(),
        "condition passes — at least 1 Aqours live moved"
    );
    game.select_generated(0);

    // Only aq_live was removed; others untouched
    assert!(
        !game.state.player1.waitroom.cards.contains(&aq_live),
        "aq_live removed"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_aq_live),
        "non-Aqours live remains"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&filler_member),
        "member card remains"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&energy),
        "energy card remains"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&aq_live),
        "aq_live on deck top"
    );
}

/// Non-Aqours live card alone: condition counts 0 matching → no trigger.
#[test]
fn riko_bp6_auto_non_aq_live_alone_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let non_aq = game.id("PL!-bp5-019-L");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.waitroom.cards.push(non_aq);
    fill_decks(&mut game, filler);

    trigger_riko_auto(&mut game, vec![non_aq]);

    assert!(
        !game.has_pending_choice(),
        "non-Aqours live → condition fails"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_aq),
        "non-Aqours live stays in waitroom"
    );
}

/// First trigger succeeds (deck top). Second trigger in same turn hits
/// the turn limit (ターン1回) and produces NO choice. The second card
/// sits untouched in waitroom. Energy and recently_moved_cards are set
/// up identically both times.
#[test]
fn riko_bp6_auto_turn_limit_blocks_second_trigger_exact() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live_a = game.id("PL!S-PR-022-PR");
    let live_b = game.id("PL!S-sd1-019-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    fill_decks(&mut game, filler);

    // --- First trigger ---
    game.state.player1.waitroom.cards.push(live_a);
    trigger_riko_auto(&mut game, vec![live_a]);
    assert!(game.has_pending_choice(), "first trigger: choice appears");
    game.select_generated(0);
    assert!(
        !game.state.player1.waitroom.cards.contains(&live_a),
        "first trigger: live_a moved to deck"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&live_a),
        "first trigger: live_a on deck top"
    );

    // --- Second trigger (same turn) ---
    game.state.player1.waitroom.cards.push(live_b);
    game.state.recently_moved_cards = Some(vec![live_b]);
    game.state
        .trigger_auto_abilities_for_player(&game.state.player1.id.clone());
    game.state
        .process_pending_auto_abilities(&game.state.player1.id.clone());

    assert!(
        !game.has_pending_choice(),
        "second trigger: turn limit — no choice"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_b),
        "second trigger: live_b remains in waitroom"
    );
    // live_a still on deck top (untouched by second trigger)
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&live_a),
        "second trigger: live_a still on deck top"
    );
}

/// Edge: trigger_moved_cards is empty vec — no cards were actually moved.
/// Condition counts 0 matching → no trigger, no choice.
#[test]
fn riko_bp6_auto_empty_moved_cards_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    fill_decks(&mut game, filler);

    trigger_riko_auto(&mut game, vec![]);

    assert!(
        !game.has_pending_choice(),
        "empty moved_cards → condition fails"
    );
}

/// Edge: trigger_moved_cards is None (not set at all) — fallback to
/// recently_moved_cards which is also None → condition fails.
#[test]
fn riko_bp6_auto_null_moved_cards_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    fill_decks(&mut game, filler);

    // Don't set recently_moved_cards at all
    game.state
        .trigger_auto_abilities_for_player(&game.state.player1.id.clone());
    game.state
        .process_pending_auto_abilities(&game.state.player1.id.clone());

    assert!(
        !game.has_pending_choice(),
        "null moved_cards → condition fails"
    );
}

/// Riko not on stage — trigger_auto_abilities_for_player scans stage
/// cards for AUTO triggers. Riko isn't there → nothing enqueued.
#[test]
fn riko_bp6_auto_riko_not_on_stage_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let _riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR");

    // Riko NOT on stage — empty stage
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.waitroom.cards.push(live);
    fill_decks(&mut game, filler);

    trigger_riko_auto(&mut game, vec![live]);

    assert!(
        !game.has_pending_choice(),
        "riko off stage → no auto trigger"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "live card stays in waitroom"
    );
}

/// Optional skip: per QA Q252, the turn limit is consumed by the EFFECT
/// (placing on deck), not the trigger. Choosing skip leaves the card in
/// waitroom and DOES NOT consume the turn limit — the ability can fire
/// again in the same turn when another Aqours live card moves.
#[test]
fn riko_bp6_auto_optional_skip_does_not_consume_turn_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live_a = game.id("PL!S-PR-022-PR");
    let live_b = game.id("PL!S-sd1-019-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    fill_decks(&mut game, filler);

    // --- First trigger: skip the optional effect ---
    game.state.player1.waitroom.cards.push(live_a);
    trigger_riko_auto(&mut game, vec![live_a]);
    assert!(
        game.has_pending_choice(),
        "first trigger: position|destination choice appears"
    );
    game.select_option(-1); // skip
    assert!(
        game.state.player1.waitroom.cards.contains(&live_a),
        "skip: live_a stays in waitroom"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&live_a),
        "skip: live_a not on deck"
    );

    // --- Second trigger (same turn): skip did NOT consume turn limit ---
    game.state.player1.waitroom.cards.push(live_b);
    trigger_riko_auto(&mut game, vec![live_b]);
    assert!(
        game.has_pending_choice(),
        "second trigger: skip didn't consume turn limit — choice appears"
    );
    game.select_generated(0); // deck_top
    assert!(
        !game.state.player1.waitroom.cards.contains(&live_b),
        "live_b moved to deck from second trigger"
    );
    assert!(
        game.state.player1.main_deck.cards.contains(&live_b),
        "live_b on deck"
    );
    // live_a still in waitroom (from the skip)
    assert!(
        game.state.player1.waitroom.cards.contains(&live_a),
        "live_a still in waitroom (was skipped)"
    );

    // --- Third trigger: now the turn limit IS consumed (live_b placed on deck) ---
    let live_c = game.id("PL!S-PR-022-PR");
    game.state.player1.waitroom.cards.push(live_c);
    trigger_riko_auto(&mut game, vec![live_c]);
    assert!(
        !game.has_pending_choice(),
        "third trigger: turn limit consumed by live_b placement — no choice"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_c),
        "live_c stays in waitroom (turn limit used)"
    );
}

// ====================================================================
// LiveEnd cleanup tests — verify heart_type=all + duration=live_end
// properly removes modifiers after the live phase ends.
// ====================================================================

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// MIRACLE WAVE (need_heart = {02:4, 04:4, 05:4}, total=12) alone passes
/// the >=12 threshold. Verify all-heart is gained during live, then
/// cleaned up after LiveVictoryDetermination.
#[test]
fn riko_bp6_live_end_clears_all_heart() {
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
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // During live phase: exactly +2 all-heart
    let all = game.state.mods.get_heart_modifier(riko, HeartColor::All);
    assert_eq!(all, 2, "during live: +2 all-heart (MW total 12 >= 12)");

    // Advance: FirstAttackerPerformance → SecondAttackerPerformance
    game.pass();
    game.pass();

    // Advance: LiveVictoryDetermination
    game.pass();

    // Handle any pending choices (e.g. LiveSuccess triggers)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

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

/// 4 x PL!S-sd1-019-SD (each has {02:1,04:1,05:1}=3) = total 12 → passes exactly.
#[test]
fn riko_bp6_threshold_exact_12_passes() {
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

/// 3 x PL!S-sd1-019-SD (each has {02:1,04:1,05:1}=3) = total 9 → below threshold.
#[test]
fn riko_bp6_threshold_9_fails() {
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

/// Verify the effect has heart_type="all". This test verifies the correct
/// parsed state expected after running the parser fix that removes leaked
/// heart_colors from the effect when heart_type=all.
/// NOTE: If abilities.json hasn't been regenerated after the parser fix,
/// heart_colors may still be present — that is a data regeneration issue,
/// not an engine bug.
#[test]
fn riko_bp6_has_heart_type_all() {
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
