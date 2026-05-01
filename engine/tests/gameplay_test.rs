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
// Pattern #10 — Debut/LiveStart: may put self to wait,
//               put 1 opponent member (cost ≤ 4) to wait
// ====================================================================
//
// JP ability text:
//   登場/ライブ開始時：このメンバーをウェイトにしてもよい：
//       相手のステージにいるコスト4以下のメンバー1人をウェイトにする。
//
// Real card:  PL!-PR-007-PR (星空凛), cost 4
//
// Flow:
//   1. Put opponent filler member (cost ≤ 4) on opponent's stage
//   2. Give player1 5 energy
//   3. Play card to stage → Debut triggers
//   4. Optional cost: put self to wait? → YES
//   5. Effect: choose opponent member to wait → select it
//   6. Verify: self in wait state, opponent member in wait state
#[test]
fn debut_change_opponent_to_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rin = game.id("PL!-PR-007-PR");
    // Filler member with cost ≤ 4 for opponent's stage
    let opp_member = game.id("PL!-sd1-010-SD");  // cost 4

    // Set up opponent's stage
    game.state.player2.stage.set_area(MemberArea::Center, opp_member);

    game.add_to_hand(rin);
    game.give_energy(5);

    // Play Rin to stage — Debut triggers
    game.play_to_stage(rin, MemberArea::LeftSide);
    assert!(game.has_pending_choice(),
        "Optional cost choice (put self to wait?) should appear");

    // Cost is SelectTarget (YES/NO) — choose YES (card_id=1)
    // This needs resume_with_choice with card_id=Some(1)
    TurnEngine::resume_with_choice(
        &mut game.state,
        Some(1),  // 1 = yes, pay optional cost
        None,
    ).expect("pay optional cost (put self to wait)");

    // Self should now be in wait state
    let orientation = game.state.orientation_modifiers.get(rin);
    assert_eq!(orientation, Some(&"wait".to_string()),
        "Rin should be in wait state");

    // Effect should prompt: select opponent member to put to wait
    assert!(game.has_pending_choice(),
        "Effect should prompt: select opponent member to wait");

    let choice = game.state.ability_queue.is_waiting_for_choice()
        .cloned().expect("Should be waiting for choice");
    match &choice {
        Choice::SelectCard { zone, count, .. } => {
            // Zone should be "stage" (opponent's stage)
            assert_eq!(zone, "stage", "Should select from stage");
            assert_eq!(*count, 1, "Should select 1 member");
        }
        other => panic!("Expected SelectCard stage, got {other:?}"),
    }

    // Opponent member is on stage[1] (Center) — index 1 in the stage array
    game.select_indices(&[1]);

    // Opponent member should now be in wait state
    let opp_orientation = game.state.orientation_modifiers.get(opp_member);
    assert_eq!(opp_orientation, Some(&"wait".to_string()),
        "Opponent member should be in wait state");
    assert!(!game.has_pending_choice(), "No more pending choices");
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
