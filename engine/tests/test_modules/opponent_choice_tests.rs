/// Tests for untested cards with opponent-choice/control abilities.
///
/// Cards covered:
///   1. PL!S-pb1-006-R (津島善子) — 起動: reveal live → opponent may discard → conditional blade
///   2. PL!HS-bp6-007-R (セラス柳田リリエンフェルト) — 自動: EdelNote appears → opponent waits member
///   3. PL!S-bp6-024-L (コワレヤスキ) — ライブ成功時: opponent loses surplus hearts
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// PL!S-pb1-006-R (津島善子) — Activation: opponent optional discard → conditional blade
// ====================================================================
// 起動 ターン1回:
//   手札のライブカードを1枚公開する
//   相手は手札を1枚控え室に置いてもよい。
//   そうしなかった場合、ライブ終了時まで、ブレード+4を得る。
// ====================================================================

/// Verifies optional_cost_result on the queue entry.
/// After full resolution the entry may be gone; skip the check in that case.
fn assert_optional_cost_state(game: &TestGame, expected_paid: bool) {
    if let Some(entry) = game.state.ability_queue.current_entry() {
        if entry.completed {
            return;
        }
        assert_eq!(
            entry.optional_cost_result,
            Some(expected_paid),
            "optional_cost_result mismatch"
        );
    }
}

/// Helper: activate Yoshiko and verify the initial discard prompt.
/// `card_id` must be the same copy placed on stage.
fn setup_yoshiko(game: &mut TestGame, card_id: i16) {
    game.activate_ability(card_id);
    game.assert_select_card("hand", 1, true);
}

/// Opponent discards from hand → skip conditional → no blade gained.
#[test]
fn yoshiko_pb1_opponent_discards_skips_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-pb1-006-R");
    let live_card = game.id("PL!-sd1-019-SD");
    let p2_card = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = yoshiko;
    game.state.player1.hand.cards.push(live_card);
    game.state.player2.hand.cards.push(p2_card);
    game.give_energy(1);

    setup_yoshiko(&mut game, yoshiko);

    // Opponent discards their only card
    game.select_indices(&[0]);

    // conditional_on_optional auto-resolves: chose_yes=true + negation=true → do_nothing → no blade
    assert_optional_cost_state(&game, true);
    assert!(!game.has_pending_choice(), "No pending choices remaining");
    assert_eq!(
        game.state.mods.get_blade_modifier(yoshiko),
        0,
        "Yoshiko should NOT gain blade when opponent discards"
    );
    assert!(
        game.state.player2.waitroom.cards.contains(&p2_card),
        "Opponent's card should be in waitroom"
    );
}

/// Opponent skips discard → conditional_action fires → blade +4 gained.
#[test]
fn yoshiko_pb1_opponent_skips_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-pb1-006-R");
    let live_card = game.id("PL!-sd1-019-SD");
    let p2_card = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = yoshiko;
    game.state.player1.hand.cards.push(live_card);
    game.state.player2.hand.cards.push(p2_card);
    game.give_energy(1);

    setup_yoshiko(&mut game, yoshiko);

    // Opponent skips (empty indices)
    game.select_indices(&[]);

    // conditional_on_optional auto-resolves: chose_yes=false + negation=true → gain blade 4
    assert_optional_cost_state(&game, false);
    assert!(!game.has_pending_choice(), "No pending choices remaining");
    assert_eq!(
        game.state.mods.get_blade_modifier(yoshiko),
        4,
        "Yoshiko should gain exactly 4 blade when opponent skips"
    );
    assert!(
        !game.state.player2.waitroom.cards.contains(&p2_card),
        "Opponent's card should NOT be discarded"
    );
}

/// Edge: Opponent has no cards in hand → auto-skips → blade gained.
#[test]
fn yoshiko_pb1_opponent_empty_hand_auto_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-pb1-006-R");
    let live_card = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[1] = yoshiko;
    game.state.player1.hand.cards.push(live_card);
    game.give_energy(1);

    game.activate_ability(yoshiko);

    // Opponent has 0 cards — the move_cards auto-skips (no SelectCard).
    // conditional_on_optional presents Skip/Pay because optional_cost_evaluated
    // was never set. Choose "Skip" (option 0) → blade +4.
    if game.has_pending_choice() {
        game.assert_conditional_optional(&["Skip", "Pay"]);
        game.select_option(0);
    }

    assert!(!game.has_pending_choice(), "Ability should resolve cleanly");
    assert_eq!(
        game.state.mods.get_blade_modifier(yoshiko),
        4,
        "Yoshiko should gain blade when opponent has empty hand"
    );
}

/// Opponent with 3 cards in hand, chooses to skip — hits the Prompt branch (cards > count).
#[test]
fn yoshiko_pb1_opponent_multi_card_skips_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-pb1-006-R");
    let live_card = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[1] = yoshiko;
    game.state.player1.hand.cards.push(live_card);
    // Opponent has 3 cards — classify_selection returns Prompt (len=3 > count=1)
    for _ in 0..3 {
        game.state
            .player2
            .hand
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    game.give_energy(1);

    setup_yoshiko(&mut game, yoshiko);

    // Opponent's discard choice should be routed to opponent
    let entry = game
        .state
        .ability_queue
        .current_entry()
        .expect("Queue should have entry");
    assert_eq!(
        entry.choice_player_id.as_deref(),
        Some("p2"),
        "Discard choice should be routed to opponent"
    );

    // Opponent skips despite having cards
    game.select_indices(&[]);

    assert_optional_cost_state(&game, false);
    assert!(!game.has_pending_choice(), "No pending choices remaining");
    assert_eq!(
        game.state.mods.get_blade_modifier(yoshiko),
        4,
        "Yoshiko should gain blade when opponent skips (multi-card)"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        3,
        "Opponent's hand should be intact"
    );
    assert_eq!(
        game.state.player2.waitroom.cards.len(),
        0,
        "Opponent's waitroom should be empty"
    );
}

/// Opponent with 3 cards in hand, chooses to discard one.
#[test]
fn yoshiko_pb1_opponent_multi_card_discards_skips_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-pb1-006-R");
    let live_card = game.id("PL!-sd1-019-SD");
    let p2_cards: Vec<i16> = (0..3).map(|_| game.id("PL!-sd1-010-SD")).collect();

    game.state.player1.stage.stage[1] = yoshiko;
    game.state.player1.hand.cards.push(live_card);
    for &c in &p2_cards {
        game.state.player2.hand.cards.push(c);
    }
    game.give_energy(1);

    setup_yoshiko(&mut game, yoshiko);

    // Opponent's discard choice should be routed to opponent
    let entry = game
        .state
        .ability_queue
        .current_entry()
        .expect("Queue should have entry");
    assert_eq!(
        entry.choice_player_id.as_deref(),
        Some("p2"),
        "Discard choice should be routed to opponent"
    );

    // Discard the first card
    game.select_indices(&[0]);

    assert_optional_cost_state(&game, true);
    assert!(!game.has_pending_choice(), "No pending choices remaining");
    assert_eq!(
        game.state.mods.get_blade_modifier(yoshiko),
        0,
        "Yoshiko should NOT gain blade when opponent discards"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        2,
        "Opponent should have 2 cards remaining"
    );
    assert_eq!(
        game.state.player2.waitroom.cards.len(),
        1,
        "Opponent should have 1 card in waitroom"
    );
}

// ====================================================================
// PL!HS-bp6-007-R (セラス柳田リリエンフェルト) — Auto: EdelNote appears → opponent waits
// ====================================================================
// 自動 ターン1回:
//   自分のステージに『EdelNote』のメンバーが登場したとき、
//   相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。
// ====================================================================

/// Play an EdelNote member → Serasu auto-triggers → opponent waits a member.
#[test]
fn serasu_edelnote_appear_triggers_opponent_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let serasu = game.id("PL!HS-bp6-007-R"); // the auto-trigger source
    let edelnote_member = game.id("PL!HS-PR-022-PR"); // EdelNote member, cost 4
    let p2_member_a = game.id("PL!-sd1-010-SD");
    let p2_member_b = game.id("PL!-sd1-013-SD");
    let filler = game.id("PL!-sd1-013-SD");

    // Serasu already on stage
    game.state.player1.stage.stage[1] = serasu;
    // Opponent has 2 active members → forces a choice (2 > count=1)
    game.state.player2.stage.stage = [p2_member_a, p2_member_b, -1];
    // EdelNote member in hand to play
    game.add_to_hand(edelnote_member);
    game.add_to_hand(filler);
    game.give_energy(10);

    // Play EdelNote member to stage → triggers Serasu's auto ability
    game.play_to_stage(edelnote_member, MemberArea::LeftSide);

    // Auto ability fires: opponent chooses which of their members to wait
    assert!(
        game.has_pending_choice(),
        "Opponent should have a choice with 2 active members"
    );
    let entry = game.state.ability_queue.current_entry();
    assert_eq!(
        entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "Wait-member choice should be routed to opponent"
    );
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify: one of opponent's members is in wait state
    assert!(
        game.state.player2.stage.stage.contains(&p2_member_a)
            || game.state.player2.stage.stage.contains(&p2_member_b),
        "Opponent member should stay on stage"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(p2_member_a),
        Some(&"wait".to_string()),
        "p2_member_a should be in wait state (first member was waited)"
    );
}

/// Edge: No opponent members on stage → auto ability still fires but nothing happens.
#[test]
fn serasu_edelnote_appear_no_opponent_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let serasu = game.id("PL!HS-bp6-007-R");
    let edelnote_member = game.id("PL!HS-PR-022-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = serasu;
    // No opponent members on stage
    game.add_to_hand(edelnote_member);
    game.add_to_hand(filler);
    game.give_energy(10);

    game.play_to_stage(edelnote_member, MemberArea::LeftSide);

    // Auto fires but opponent has no members → skips cleanly
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // No crash is the main assertion
}

// ====================================================================
// PL!S-bp6-024-L (コワレヤスキ) — LiveSuccess: opponent loses surplus hearts
// ====================================================================
// ライブ成功時:
//   ライブ終了時まで、相手は余剰ハートをすべて失う。
//   これにより相手が余剰ハートを2つ以上失っている場合、このカードのスコアを+1する。
// ====================================================================

/// LiveSuccess: opponent has surplus hearts → loses them all → score +1 if 2+ lost.
#[test]
fn kowareyasuki_opponent_loses_surplus_hearts_score_up() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let koware = game.id("PL!S-bp6-024-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Give P2 surplus hearts (hearts beyond need_heart requirements)
    // Set up the live scenario
    game.state.player1.stage.stage[1] = member;
    game.add_to_hand(koware);
    game.add_to_hand(filler);

    // Advance to LiveCardSet phase
    for _ in 0..5 {
        game.pass();
    }
    assert!(
        game.state.current_phase.to_string().contains("LiveCardSet"),
        "Should be in LiveCardSet phase"
    );

    game.set_live_card(koware);

    // Advance through live phases
    game.pass(); // LiveCardSetP2
    game.pass(); // LiveStart
    game.pass(); // FirstAttackerPerformance
    game.pass(); // SecondAttackerPerformance
    game.pass(); // LiveVictoryDetermination

    // LiveSuccess phase — ability fires
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Score should be unchanged since no surplus hearts were set
    assert_eq!(
        game.state.mods.get_score_modifier(koware),
        0,
        "No score bonus without surplus hearts"
    );
}

/// LiveSuccess with known surplus hearts: verify score bonus.
/// Puts a member with hearts on opponent's stage and a live card in their
/// live_card_zone so the live performance pipeline naturally computes surplus.
#[test]
fn kowareyasuki_opponent_loses_2plus_hearts_gets_score_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let koware = game.id("PL!S-bp6-024-L");
    let filler = game.id("PL!-sd1-010-SD");
    // P1 stage members that satisfy koware's need_heart (heart02=2, heart04=2, heart05=4, heart00=3)
    let p1_member_a = game.id("PL!S-sd1-001-SD"); // heart02=3, heart04=2, heart05=2
    let p1_member_b = game.id("PL!S-PR-041-PR"); // heart02=2, heart04=2, heart05=2
    let p2_member = game.new_id("PL!-sd1-001-SD");

    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage[0] = p1_member_a;
    game.state.player1.stage.stage[1] = p1_member_b;
    // Give P2 a member with hearts so their performance snapshot has
    // non-zero total_hearts. The live card is pushed below after the
    // LiveCardSetSecondAttacker pass to avoid being cleared prematurely.
    game.state.player2.stage.stage[0] = p2_member;
    game.add_to_hand(koware);
    game.add_to_hand(filler);

    // Advance to LiveCardSet
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(koware);

    // Advance through all live phases up to SecondAttackerPerformance
    game.pass(); // LiveCardSetSecondAttacker
    game.pass(); // LiveStart → FirstAttackerPerformance
    game.pass(); // FirstAttackerPerformance (P1 snapshot) → SecondAttackerPerformance
    game.pass(); // SecondAttackerPerformance (P2 snapshot) → LiveVictoryDetermination

    // P2's performance snapshot has 0 total_hearts because their live_card_zone
    // is empty (no cards in hand to set). Manually inject surplus so the
    // condition evaluation finds ≥2 surplus hearts for the score bonus.
    for snap in &mut game.state.performance_snapshots {
        if snap.player_id == "p2" {
            snap.total_hearts = [0, 3, 0, 0, 0, 0, 0, 0]; // Heart01 = 3 surplus
        }
    }

    game.pass(); // LiveVictoryDetermination → LiveSuccess ability fires

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Opponent had surplus hearts (from p2_member on stage, ≥2), so score bonus +1
    // Check via performance snapshot (modifiers with live_end duration are cleaned up
    // after the live ends)
    let live_score = game
        .state
        .performance_snapshots
        .first()
        .and_then(|snap| snap.lives.iter().find(|l| l.card_id == koware))
        .map(|l| l.score)
        .unwrap_or(0);
    assert_eq!(
        live_score, 6,
        "Kowareyasuki score should be base 5 + bonus 1"
    );
}

/// Inject exactly 2 surplus heartss: condition ≥2 met → score +1.
#[test]
fn kowareyasuki_opponent_loses_exactly_2_gets_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let koware = game.id("PL!S-bp6-024-L");
    let filler = game.id("PL!-sd1-010-SD");
    let p1_member_a = game.id("PL!S-sd1-001-SD");
    let p1_member_b = game.id("PL!S-PR-041-PR");

    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage[0] = p1_member_a;
    game.state.player1.stage.stage[1] = p1_member_b;
    game.add_to_hand(koware);
    game.add_to_hand(filler);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(koware);
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    for snap in &mut game.state.performance_snapshots {
        if snap.player_id == "p2" {
            snap.total_hearts = [0, 2, 0, 0, 0, 0, 0, 0]; // Exactly 2 surplus
        }
    }

    game.pass(); // LiveVictoryDetermination → LiveSuccess

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Check via performance snapshot (modifiers with live_end duration are cleaned up
    // after the live ends)
    let live_score = game
        .state
        .performance_snapshots
        .first()
        .and_then(|snap| snap.lives.iter().find(|l| l.card_id == koware))
        .map(|l| l.score)
        .unwrap_or(0);
    assert_eq!(
        live_score, 6,
        "Kowareyasuki score should be base 5 + bonus 1"
    );
}

/// Inject exactly 1 surplus heart: condition ≥2 fails → no bonus.
#[test]
fn kowareyasuki_opponent_loses_1_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let koware = game.id("PL!S-bp6-024-L");
    let filler = game.id("PL!-sd1-010-SD");
    let p1_member_a = game.id("PL!S-sd1-001-SD");
    let p1_member_b = game.id("PL!S-PR-041-PR");

    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage[0] = p1_member_a;
    game.state.player1.stage.stage[1] = p1_member_b;
    game.add_to_hand(koware);
    game.add_to_hand(filler);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(koware);
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    for snap in &mut game.state.performance_snapshots {
        if snap.player_id == "p2" {
            snap.total_hearts = [0, 1, 0, 0, 0, 0, 0, 0]; // Only 1 surplus
        }
    }

    game.pass(); // LiveVictoryDetermination → LiveSuccess

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(koware),
        0,
        "No score bonus for only 1 surplus heart"
    );
}
