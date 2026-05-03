/// Gameplay integration tests using ONLY real card data.
///
/// Each test loads the actual cards.json + abilities.json from the real card database,
/// picks a real card with a specific ability pattern, sets up a board state,
/// plays through the scenario, and asserts expected outcomes.
///
/// Filler cards (zero abilities) come from `tests/data/cards.json` — see
/// GAMEPLAY_TEST_GUIDE.md for the full reference.

use rabuka_engine::ability::types::Choice;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

mod helpers;
use helpers::*;

// ====================================================================
// Activation cost "move self from stage to discard"
// to search discard for a live card and add it to hand
// ====================================================================
//
// Real card:  黒澤ルビィ (PL!S-bp2-009-R)
// Ability text (JP):
//   起動：このメンバーをステージから控え室に置く：
//       自分の控え室からライブカードを1枚手札に加える。
//
// Filler cards used (all abilityless):
//   PL!-sd1-021-SD — live (searched card)
//   PL!-sd1-020-SD — live (unsearched, stays in discard)
//   PL!-sd1-010-SD — member (should never be selectable)
//
// Flow:
//   1. Give 3 energy → play Ruby (cost 2) to stage center
//   2. Activate Ruby's 起動 ability
//   3. Cost: Ruby moves from stage to discard
//   4. Effect: choose 1 live card from discard → goes to hand
//   5. Member card must remain in discard (correct card_type filter)
#[test]
fn ruby_activation_search_live_from_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = game.id("PL!S-bp2-009-R");
    let live_a = game.id("PL!-sd1-021-SD");
    let live_b = game.id("PL!-sd1-020-SD");
    let filler_member = game.id("PL!-sd1-010-SD");

    game.add_to_hand(ruby);
    game.add_to_discard(live_a);
    game.add_to_discard(filler_member);
    game.add_to_discard(live_b);
    game.give_energy(3);

    game.play_to_stage(ruby, MemberArea::Center);
    assert_eq!(game.state.player1.stage.get_area(MemberArea::Center), Some(ruby));
    assert!(!game.state.player1.hand.cards.contains(&ruby));

    game.activate_ability(ruby);

    assert!(game.has_pending_choice(),
        "Expected a choice prompt for selecting a live card from discard");
    assert!(game.state.player1.waitroom.cards.contains(&ruby),
        "Ruby should be in discard after paying the self-cost");

    // discard = [live_a(0), filler_member(1), live_b(2), ruby(3)] — pick first live
    game.select_indices(&[0]);

    assert!(game.state.player1.hand.cards.contains(&live_a),
        "Selected live card should be in hand");
    assert!(!game.state.player1.waitroom.cards.contains(&live_a),
        "Selected live card should be gone from discard");
    assert!(game.state.player1.waitroom.cards.contains(&live_b),
        "Unselected live card stays in discard");
    assert!(game.state.player1.waitroom.cards.contains(&filler_member),
        "Member card (wrong card_type) stays in discard");
    assert!(game.state.player1.waitroom.cards.contains(&ruby),
        "Ruby (cost) stays in discard");
    assert!(!game.has_pending_choice(), "No more pending choices");
}

// ====================================================================
// Pattern #11 — Debut: may discard 1 from hand, place 1 energy from
//               energy deck into energy zone in wait state
// ====================================================================
//
// JP: 登場：手札を1枚控え室に置いてもよい：自分のエネルギー置場から、
//     エネルギーカードを1枚ウェイト状態で置く。
// Card: PL!SP-PR-004-PR (唐可可), cost 4
#[test]
fn debut_energy_deck_to_zone_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let keke = game.id("PL!SP-PR-004-PR");
    let energy = game.id("LL-E-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.energy_deck.cards.push(energy);
    game.add_to_hand(keke);
    game.add_to_hand(filler);
    game.give_energy(5);

    let ed_before = game.state.player1.energy_deck.cards.len();
    let ez_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_energy_count;

    game.play_to_stage(keke, MemberArea::Center);

    // Optional cost: skip (empty indices)
    assert!(game.has_pending_choice(), "Optional discard choice");
    game.select_indices(&[]);

    assert_eq!(game.state.player1.energy_zone.cards.len(), ez_before + 1,
        "Energy zone +1");
    assert_eq!(game.state.player1.energy_deck.cards.len(), ed_before - 1,
        "Energy deck -1");
    // Cost 4 paid from 5 active = 1 remaining. Wait energy didn't add to active count.
    assert_eq!(game.state.player1.energy_zone.active_energy_count, active_before - 4,
        "Active = 1 (5 - 4 cost, wait energy not counted)");
    assert!(!game.has_pending_choice(), "Done");
}

// ====================================================================
// Pattern #12 — Debut: may discard 1 from hand, search discard for
//               live card of specified color → hand
// ====================================================================
//
// JP: 登場：手札を1枚控え室に置いてもよい：自分の控え室から
//     「指定された色」のライブカードを1枚手札に加える。
// Card: PL!N-bp1-003-R＋ (上原歩夢), cost 10
#[test]
fn debut_discard_search_specified_color_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ayumu = game.id("PL!N-bp1-003-R\u{FF0B}");
    // This ability has a group filter (虹ヶ咲). Use a 虹ヶ咲 live card.
    let live_target = game.id("PL!N-sd1-025-SD");
    let filler_member = game.id("PL!-sd1-010-SD");

    game.add_to_hand(ayumu);
    game.add_to_hand(filler_member);
    game.add_to_discard(live_target);
    game.add_to_discard(filler_member);
    game.give_energy(11);

    game.play_to_stage(ayumu, MemberArea::Center);

    // Optional cost: skip
    assert!(game.has_pending_choice(), "Optional discard choice");
    game.select_indices(&[]);

    // Effect: auto-selects the only matching live card (1 match, count=1 → no choice)
    assert!(!game.has_pending_choice(), "Auto-resolved (1 match)");

    assert!(game.state.player1.hand.cards.contains(&live_target),
        "虹ヶ咲 live card should be in hand from discard search");
    assert!(!game.has_pending_choice(), "Done");
}

// Pattern #12b — same ability with 2 matching live cards → choice prompt
#[test]
fn debut_discard_search_grouped_live_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ayumu = game.id("PL!N-bp1-003-R\u{FF0B}");
    let live_a = game.id("PL!N-sd1-025-SD");
    let live_b = game.id("PL!N-sd1-026-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(ayumu);
    game.add_to_hand(filler);
    game.add_to_discard(live_a);
    game.add_to_discard(live_b);
    game.give_energy(11);

    game.play_to_stage(ayumu, MemberArea::Center);
    assert!(game.has_pending_choice(), "Optional discard choice");
    game.select_indices(&[]);

    // 2 matching live cards, count=1 → choice prompt
    assert!(game.has_pending_choice(), "Should prompt to choose from 2 live cards");
    game.select_indices(&[0]);

    assert!(game.state.player1.hand.cards.contains(&live_a),
        "Selected live card in hand");
    assert!(!game.has_pending_choice(), "Done");
}

// ====================================================================
// Pattern #13 — Debut: draw 2, discard 1
// ====================================================================
//
// JP: 登場：カードを2枚引き、手札を1枚控え室に置く。
// Card: PL!HS-bp1-006-R＋ (夕霧綴理), cost 11
#[test]
fn debut_draw_two_discard_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let tsuzuri = game.id("PL!HS-bp1-006-R\u{FF0B}");
    let f1 = game.id("PL!-sd1-010-SD");
    let f2 = game.id("PL!-sd1-013-SD");

    game.state.player1.main_deck.cards.push(f1);
    game.state.player1.main_deck.cards.push(f2);
    game.add_to_hand(tsuzuri);
    game.add_to_hand(f1);
    game.add_to_hand(f2);
    game.give_energy(12);

    let db_before = game.state.player1.main_deck.cards.len();
    game.play_to_stage(tsuzuri, MemberArea::Center);

    assert!(game.has_pending_choice(), "Discard choice after draw 2");
    game.select_indices(&[0]);

    assert_eq!(game.state.player1.hand.cards.len(), 3,
        "Hand = 3 (started 3, played 1, drew 2, discarded 1)");
    assert_eq!(game.state.player1.main_deck.cards.len(), db_before - 2,
        "Deck -2");
    assert!(!game.has_pending_choice(), "Done");
}

// ====================================================================
// Pattern #14 — LiveStart: choose a heart color, gain that heart on
//               each live card until live end
// ====================================================================
//
// JP: ライブ開始時：heart01/heart03/heart06から1つ選ぶ。
//     ライブ終了まで、自分のライブカード置き場のカード1枚につき、
//     選んだハートを1つ得る。
// Card: PL!-bp3-012-PR (南ことり) — this is a live card, not a member!
//       This pattern tests live start trigger on a live card.
// Note: tested live card ability, not member — skipped for now (complexity)
#[test]
fn live_start_choose_heart_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kasumi = game.id("PL!N-bp3-014-N");

    game.add_to_stage(MemberArea::Center, kasumi);

    let card = game.db.get_card(kasumi).expect("Kasumi card");
    let ability = card.abilities.first().expect("Kasumi has at least 1 ability");
    let ability_id = format!("{}_{}", card.card_no, ability.full_text);

    game.state.trigger_auto_ability(
        ability_id,
        rabuka_engine::game_state::AbilityTrigger::LiveStart,
        "p1".to_string(),
        Some("PL!N-bp3-014-N".to_string()),
    );

    game.state.process_pending_auto_abilities("p1");

    assert!(game.has_pending_choice(), "Should have a pending heart color choice");

    let choice = game.state.ability_queue.is_waiting_for_choice()
        .cloned()
        .expect("Should have a queued choice");
    match &choice {
        rabuka_engine::ability_resolver::Choice::SelectHeartColor { count, options, description: _ } => {
            assert_eq!(*count, 3, "Should have count 3");
            assert!(options.contains(&"heart01".to_string()), "Should offer heart01");
            assert!(options.contains(&"heart03".to_string()), "Should offer heart03");
            assert!(options.contains(&"heart04".to_string()), "Should offer heart04");
            // Select heart03 (index 1 in unique sorted list)
            TurnEngine::resume_with_choice(&mut game.state, Some(1), None)
                .expect("resume_with_choice should succeed");
        }
        _ => panic!("Expected SelectHeartColor choice, got {:?}", choice),
    }

    // After choice, heart_override should be set for Kasumi
    assert!(game.state.heart_override.contains_key(&kasumi), "Heart override should be set for Kasumi");
    let (color, count) = game.state.heart_override.get(&kasumi).expect("Heart override for Kasumi");
    assert_eq!(*color, rabuka_engine::card::HeartColor::Heart03, "Should override to heart03");
    assert_eq!(*count, 3, "Override count should be 3");

    // Verify temporary effect is registered for cleanup
    let has_temp = game.state.temporary_effects.iter().any(|te| te.effect_type == "heart_override");
    assert!(has_temp, "Should have a heart_override temporary effect registered for cleanup");
}

// ====================================================================
// Pattern #15 — Debut: draw 2, discard 2
// ====================================================================
//
// JP: 登場：カードを2枚引き、手札を2枚控え室に置く。
// Card: PL!N-PR-005-PR (上原歩夢), cost 13
#[test]
fn debut_draw_two_discard_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ayumu = game.id("PL!N-PR-005-PR");
    let f1 = game.id("PL!-sd1-010-SD");
    let f2 = game.id("PL!-sd1-013-SD");
    let f3 = game.id("PL!-sd1-014-SD");

    game.state.player1.main_deck.cards.push(f1);
    game.state.player1.main_deck.cards.push(f2);
    game.add_to_hand(ayumu);
    game.add_to_hand(f1);
    game.add_to_hand(f2);
    game.add_to_hand(f3);
    game.give_energy(14);

    let db_before = game.state.player1.main_deck.cards.len();
    game.play_to_stage(ayumu, MemberArea::Center);

    assert!(game.has_pending_choice(), "Discard choice after draw 2");
    // Discard 2 from hand (indices 0 and 1)
    game.select_indices(&[0, 1]);

    assert_eq!(game.state.player1.hand.cards.len(), 3,
        "Hand = 3 (started 4, played 1, drew 2, discarded 2)");
    assert_eq!(game.state.player1.main_deck.cards.len(), db_before - 2,
        "Deck -2");
    assert!(!game.has_pending_choice(), "Done");
}

// ====================================================================
// Pattern #2 — Activation: self stage→discard, search discard→hand for MEMBER
// ====================================================================
//
// JP ability text:
//   起動：このメンバーをステージから控え室に置く：
//       自分の控え室からメンバーカードを1枚手札に加える。
//
// Real card:  園田海未 (PL!-sd1-002-SD)
// Same cost pattern as Ruby, but effect searches member not live.
//
// Filler: PL!-sd1-010-SD (member, searched), PL!-sd1-020-SD (live, wrong type)
#[test]
fn activation_search_member_from_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sonoda = game.id("PL!-sd1-002-SD");
    let target_member = game.id("PL!-sd1-010-SD");
    let filler_live = game.id("PL!-sd1-020-SD");

    game.add_to_hand(sonoda);
    game.add_to_discard(target_member);
    game.add_to_discard(filler_live);
    game.give_energy(3);

    game.play_to_stage(sonoda, MemberArea::Center);
    game.activate_ability(sonoda);

    // Cost paid: sonoda moved to discard
    assert!(game.state.player1.waitroom.cards.contains(&sonoda));

    // Choice prompts: must filter to member_card
    let choice = game.state.ability_queue.is_waiting_for_choice()
        .cloned()
        .expect("Should be waiting for choice");
    match &choice {
        Choice::SelectCard { zone, card_type, count, allow_skip, .. } => {
            assert_eq!(zone, "discard");
            assert_eq!(card_type.as_deref(), Some("member_card"));
            assert_eq!(*count, 1);
            assert!(!allow_skip);
        }
        other => panic!("Expected SelectCard, got {other:?}"),
    }

    // discard = [target_member(0), filler_live(1), sonoda(2)]
    game.select_indices(&[0]);

    assert!(game.state.player1.hand.cards.contains(&target_member),
        "Selected member should be in hand");
    assert!(game.state.player1.waitroom.cards.contains(&filler_live),
        "Live card (wrong type) stays in discard");
    assert!(!game.has_pending_choice(), "No more pending choices");
}

// ====================================================================
// Pattern #3 — Debut: may discard 1 from hand, look at top 3,
//              choose 1 to hand, rest to discard
// ====================================================================
//
// JP ability text:
//   登場：手札を1枚控え室に置いてもよい：
//       自分のデッキの上からカードを3枚見る。
//       その中から1枚を手札に加え、残りを控え室に置く。
//
// Real card:  園田海未 (PL!-sd1-011-SD), cost 4
//
// Flow:
//   1. Give 5 energy → play card to stage (cost 4)
//   2. Debut triggers:
//      a. Optional: may discard 1 from hand (skip for this test)
//      b. Look at top 3 of deck
//      c. Choose 1 to hand, rest to discard
//   3. Verify: 1 card in hand, 2 in discard, original hand unchanged
#[test]
fn debut_look_top3_choose1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sonoda = game.id("PL!-sd1-011-SD");
    let filler_member = game.id("PL!-sd1-010-SD");

    // Deck "top" = index 0 (peek_top/draw reads from front).
    // Push the 3 target cards first (they become top), then filler at bottom.
    let deck_top = [
        game.id("PL!-sd1-013-SD"),  // top → looked_at[0]
        game.id("PL!-sd1-014-SD"),  //        looked_at[1]
        game.id("PL!-sd1-017-SD"),  //        looked_at[2]
    ];
    for &c in &deck_top {
        game.state.player1.main_deck.cards.push(c);
    }
    game.state.player1.main_deck.cards.push(filler_member);
    // deck = [sd1-013, sd1-014, sd1-017, filler_member]
    // peek_top(3) = [sd1-013, sd1-014, sd1-017]

    game.add_to_hand(sonoda);
    // Extra card in hand for optional discard (we will skip it)
    game.add_to_hand(filler_member);
    game.give_energy(5);

    game.play_to_stage(sonoda, MemberArea::Center);

    // Debut triggered → optional cost choice (may discard 1 from hand)
    assert!(game.has_pending_choice(), "Optional cost choice expected");

    // Skip the optional cost → empty indices
    game.select_indices(&[]);

    // Now look_and_select choice: pick 1 of the 3 looked-at cards
    assert!(game.has_pending_choice(), "Look-and-select choice expected");

    let choice = game.state.ability_queue.is_waiting_for_choice()
        .cloned()
        .expect("Should be waiting for choice");
    match &choice {
        Choice::SelectCard { zone, .. } => {
            assert_eq!(zone, "looked_at", "Choice should be from looked_at zone");
        }
        other => panic!("Expected SelectCard with looked_at zone, got {other:?}"),
    }

    // The looked_at cards are: [sd1-013(0), sd1-014(1), sd1-017(2)]
    // looked_at = [sd1-013(0), sd1-014(1), sd1-017(2)]
    // Select index 2 = sd1-017 → goes to hand, [sd1-013, sd1-014] go to discard
    game.select_indices(&[2]);

    assert!(game.state.player1.hand.cards.contains(&deck_top[2]),
        "Selected card sd1-017 should be in hand, hand={:?}",
        game.state.player1.hand.cards);
    assert!(game.state.player1.waitroom.cards.contains(&deck_top[0]),
        "Unselected sd1-013 should be in discard");
    assert!(game.state.player1.waitroom.cards.contains(&deck_top[1]),
        "Unselected sd1-014 should be in discard");
    assert!(!game.state.player1.waitroom.cards.contains(&deck_top[2]),
        "Selected sd1-017 should not be in discard");
    assert!(!game.has_pending_choice(), "No more pending choices");
}

// ====================================================================
// Pattern #4 — Debut: draw 1, discard 1 (sequential)
// ====================================================================
//
// JP ability text:
//   登場：カードを1枚引き、手札を1枚控え室に置く。
//
// Real card:  PL!N-bp1-019-PR (中須かすみ), cost 4
//
// Flow:
//   1. Give 5 energy
//   2. Put card in hand + 2 extra fillers in hand
//   3. Put fillers in deck
//   4. Play to stage (cost 4)
//   5. Debut → draw 1 from deck (auto) → discard 1 from hand (choice)
//   6. Verify hand: -1 (played) +1 (drew) -1 (discarded) = -1 net
#[test]
fn debut_draw_one_discard_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kasumi = game.id("PL!N-bp1-019-PR");
    let filler_a = game.id("PL!-sd1-010-SD");
    let filler_b = game.id("PL!-sd1-013-SD");

    game.state.player1.main_deck.cards.push(filler_a);
    game.state.player1.main_deck.cards.push(filler_b);

    game.add_to_hand(kasumi);
    game.add_to_hand(filler_a);
    game.add_to_hand(filler_b);
    game.give_energy(5);

    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();

    game.play_to_stage(kasumi, MemberArea::Center);

    // After draw (auto), before discard choice: hand = 2 fillers + 1 drawn = 3
    assert!(game.has_pending_choice(),
        "Discard-from-hand choice should appear after draw");

    let choice = game.state.ability_queue.is_waiting_for_choice()
        .cloned().expect("Should be waiting for choice");
    match &choice {
        Choice::SelectCard { zone, count, allow_skip, .. } => {
            assert_eq!(zone, "hand");
            assert_eq!(*count, 1);
            assert!(!allow_skip);
        }
        other => panic!("Expected SelectCard hand, got {other:?}"),
    }

    game.select_indices(&[0]);

    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(hand_after, hand_before - 1,
        "Hand net -1 (played 1, drew 1, discarded 1), was {hand_before}, now {hand_after}");
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 1,
        "Deck -1 (drew 1)");
    assert!(!game.has_pending_choice(), "No more pending choices");
}
