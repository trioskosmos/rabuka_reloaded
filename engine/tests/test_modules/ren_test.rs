/// Tests for 葉月 恋 (PL!SP-bp5-005-R＋):
///
/// Ab#0 (起動, ターン1回):
///   デッキの上からカードを3枚控え室に置く：ライブ終了時まで、
///   これにより控え室に置いた『Liella!』のメンバーカード1枚につき、ブレードを得る。
///
/// Ab#1 (自動, ターン1回):
///   自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から
///   控え室に置かれるたび、Eを支払ってもよい。そうした場合、
///   それらのカードの中から1枚手札に加える。
///
/// Q221: 「それらのカードの中」refers to the cards placed by the trigger, not all discard.
/// Q233: Skipping the optional E cost allows re-triggering later in the same turn.
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

/// Ab#0: Activation sends deck top 3 to discard, grants 1 blade per Liella! member
/// among those 3. Per-unit formula: (matching / per_unit_count) * count.
#[test]
fn ren_ab0_2_liella_among_3_discarded_grants_2_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella_a = game.id("PL!SP-sd1-001-SD");
    let liella_b = game.id("PL!SP-sd1-004-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(3);

    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, liella_b);
    game.state.player1.main_deck.cards.insert(0, liella_a);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let discard_before = game.state.player1.waitroom.cards.len();

    game.activate_ability(ren);

    // Cost: deck top 3 → discard
    assert_eq!(game.state.player1.waitroom.cards.len(), discard_before + 3);

    // Per-unit: (2 Liella! matching / 1 per_unit_count) * 1 count = 2 blade
    assert_eq!(
        game.state.mods.get_blade_modifier(ren),
        2,
        "2 Liella! members among 3 discarded → 2 blade"
    );
}

/// Ab#0: 0 Liella! members among the 3 discarded → 0 blade.
#[test]
fn ren_ab0_no_liella_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let non_liella = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(3);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, non_liella);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(non_liella);
    }

    game.activate_ability(ren);

    assert_eq!(
        game.state.mods.get_blade_modifier(ren),
        0,
        "0 Liella! members discarded → 0 blade"
    );
}

/// Ab#0: All 3 discarded are Liella! members → 3 blade (per-unit: 3/1*1 = 3).
#[test]
fn ren_ab0_all_3_liella_grants_3_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(3);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, liella);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(liella);
    }

    game.activate_ability(ren);

    assert_eq!(
        game.state.mods.get_blade_modifier(ren),
        3,
        "3 Liella! members discarded → 3 blade"
    );
}

/// Ab#0: Blade has duration=live_end, persists after activation resolves.
#[test]
fn ren_ab0_blade_duration_live_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(3);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, liella);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(liella);
    }

    game.activate_ability(ren);

    assert_eq!(
        game.state.mods.get_blade_modifier(ren),
        3,
        "Blade modifier persists after ability resolves (duration=live_end)"
    );
}

/// Ab#0: Pre-existing Liella! members in discard do NOT count — only the 3
/// just placed by the cost are considered. (discard per_unit counts all matching
/// in discard, so pre-existing ones inflate the count — known limitation.)
#[test]
fn ren_ab0_preexisting_liella_in_discard_inflates_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Put 2 Liella! members in discard BEFORE activation
    game.state.player1.waitroom.cards.push(liella);
    game.state.player1.waitroom.cards.push(liella);

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(3);

    // Deck top 3: only 1 Liella!
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, liella);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.activate_ability(ren);

    // Without the "those_cards" tracking, engine counts ALL Liella! in discard
    // = 2 pre-existing + 1 just placed = 3
    // Expected: 1 (only the 1 placed by cost). Limitation is now resolved!
    assert_eq!(
        game.state.mods.get_blade_modifier(ren),
        1,
        "Expected 1 (only the 1 placed by cost). Limitation is now resolved!"
    );
}

// ========================================================================
// Ab#1 tests
// ========================================================================

/// Ab#1: Activate ab#0 (mill 3), then ab#1 auto-triggers. Pay 1 energy,
/// select 1 card from the 3 milled to recover to hand.
#[test]
fn ren_ab1_triggers_after_mill_pay_cost_recover_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    // Deck top: [liella, liella, filler, ...]
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, liella);
    game.state.player1.main_deck.cards.insert(0, liella);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren),
        None,
        None,
        None,
    )
    .expect("Activate ab#0");

    // Ab#1 should trigger → optional cost prompt → pay
    if game.has_pending_choice() {
        let ct = game.pending_choice_type().unwrap_or_default();
        if ct == "SelectTarget" {
            game.select_option(1);
        }
    }
    // Select 1 card from trigger cards to recover
    while game.has_pending_choice() {
        let ct = game.pending_choice_type().unwrap_or_default();
        match ct.as_str() {
            "SelectCard" => {
                game.select_indices(&[0]);
            }
            "SelectTarget" => {
                game.select_option(0);
            }
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "1 card should be recovered to hand"
    );
}

/// Ab#1: Decline the optional cost, no card recovered (per Q233).
#[test]
fn ren_ab1_decline_cost_no_recovery() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren),
        None,
        None,
        None,
    )
    .expect("Activate ab#0");

    // Ab#1 triggers → optional cost prompt → decline
    if game.has_pending_choice() {
        let ct = game.pending_choice_type().unwrap_or_default();
        if ct == "SelectTarget" {
            game.select_option(0);
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "No card recovered when cost declined"
    );
}

/// Ab#1: Pre-existing cards in discard should NOT be selectable (Q221).
#[test]
fn ren_ab1_only_trigger_cards_not_full_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Pre-fill discard
    game.state.player1.waitroom.cards.push(liella);

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let discard_before = game.state.player1.waitroom.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren),
        None,
        None,
        None,
    )
    .expect("Activate ab#0");

    // Ab#1 triggers → pay
    if game.has_pending_choice() {
        let ct = game.pending_choice_type().unwrap_or_default();
        if ct == "SelectTarget" {
            game.select_option(1);
        }
    }

    // Select 1 from the 3 trigger cards
    while game.has_pending_choice() {
        let ct = game.pending_choice_type().unwrap_or_default();
        match ct.as_str() {
            "SelectCard" => {
                game.select_indices(&[0]);
            }
            "SelectTarget" => {
                game.select_option(0);
            }
            _ => break,
        }
    }

    // 1 pre-existing + 3 milled - 1 recovered = 3 remaining
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before + 2, // 3 milled - 1 recovered = 2 net new
        "Pre-existing Liella! should not be recoverable (Q221)"
    );
}

/// card_type_filter "card" is a catch-all that matches every card type.
/// Paying the optional cost recovers 1 card regardless of the trigger card type.
#[test]
fn ren_ab1_card_type_catch_all_matches_any_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let energy = game.id("LL-E-001-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    game.state.player1.waitroom.cards.push(energy);
    game.state.player1.waitroom.cards.push(energy);
    game.state.player1.waitroom.cards.push(energy);
    game.state.recently_moved_cards = Some(vec![energy, energy, energy]);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "card_type='card' catch-all → 1 recovered"
    );
}

/// 2 Rens: first pays, second declines → only 1 card recovered.
#[test]
fn ren_ab1_two_copies_one_pays_one_declines() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren1 = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let ren2 = game.new_id("PL!SP-bp5-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [ren1, ren2, -1];
    game.give_energy(5);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren1),
        None,
        None,
        None,
    )
    .expect("Activate ab#0");

    // First prompt → ren1: pay.
    // Second prompt → ren2: decline.
    // Third prompt → no card selection (declined costs skip move_cards).
    // Only ren1 pays → 1 card recovered.
    let mut pay_count = 0;
    while game.has_pending_choice() && pay_count < 6 {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => {
                if pay_count == 0 {
                    game.select_option(1); // ren1 pays
                } else {
                    game.select_option(0); // ren2 declines
                }
                pay_count += 1;
            }
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "Only ren1 paid → 1 card recovered"
    );
}

/// look_and_select discard_remaining triggers auto abilities as a SINGLE
/// batched discard (not per-card). Uses PL!-sd1-011-SD which has a debut
/// look_and_select: look at top 3, pick 1 to hand, discard rest.
#[test]
fn ren_ab1_triggers_once_from_look_and_select_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let scry = game.id("PL!-sd1-011-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [ren, -1, -1];
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(scry);
    // Add a filler so the scry's optional discard cost can be paid
    game.state.player1.hand.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(5);

    // Play scry card to stage → debut triggers look_and_select
    game.play_to_stage(scry, rabuka_engine::zones::MemberArea::Center);

    // Handle the look_and_select: pick 1 card from top 3 to hand
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1),
            _ => break,
        }
    }

    // After look_and_select discards remaining 2 cards as a batch,
    // Ren's ab#1 should trigger exactly once. Handle the prompt.
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1),
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    // scry recovered 1 card to hand + Ren recovered 1 = 2 total.
    // The scry was played from hand (1 card) but 2 were recovered.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "scry picked 1 + Ren recovered 1 = 2 cards in hand"
    );
}

// ========================================================================
// Interaction tests
// ========================================================================

/// 2 copies of Ren on stage. Mill 3 → both ab#1 trigger independently.
#[test]
fn ren_ab1_two_copies_both_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren1 = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let ren2 = game.new_id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [ren1, ren2, -1];
    game.give_energy(5);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, liella);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();

    // Activate ab#0 on ren1 → mill 3 → both ab#1 trigger → 2 prompts
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren1),
        None,
        None,
        None,
    )
    .expect("Activate ab#0");

    // Handle all prompts: auto-ability selection, cost, card pick
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1),
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 2,
        "2 cards should be recovered (one per Ren copy)"
    );
}

/// Q233: Decline cost → ability can re-trigger when more cards are discarded.
#[test]
fn ren_ab1_decline_retrigger_same_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    // First discard event: 3 cards milled
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.state.recently_moved_cards = Some(vec![filler, filler, filler]);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    // Process the auto ability → decline cost
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(0), // decline
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "Q233: declined → no card recovered"
    );

    // Second discard event: 3 more cards milled
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.state.recently_moved_cards = Some(vec![filler, filler, filler]);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    // Process the auto ability → pay this time
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1), // pay
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Q233: second discard → card recovered after paying"
    );
}
