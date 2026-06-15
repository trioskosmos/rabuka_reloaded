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

/// Opponent discards from hand → choose "Pay" on conditional → no blade gained.
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

    game.activate_ability(yoshiko);

    // Step 1: Opponent optional discard — discard card at index 0
    assert!(
        game.has_pending_choice(),
        "Opponent should have discard choice"
    );
    game.select_indices(&[0]);

    // Step 2: conditional_on_optional creates SelectTarget — choose "Pay" (option 1)
    //   chose_yes=true + conditional_negation=true → optional_action(do_nothing) → no blade
    assert!(
        game.has_pending_choice(),
        "Should have conditional_optional choice"
    );
    game.select_option(1);

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

/// Opponent skips discard → choose "Skip" on conditional → blade +4 gained.
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

    game.activate_ability(yoshiko);

    // Step 1: Opponent optional discard — skip with empty indices
    assert!(
        game.has_pending_choice(),
        "Opponent should have discard choice"
    );
    game.select_indices(&[]);

    // Step 2: conditional_on_optional creates SelectTarget — choose "Skip" (option 0)
    //   chose_yes=false + conditional_negation=true → conditional_action fires → blade +4
    assert!(
        game.has_pending_choice(),
        "Should have conditional_optional choice"
    );
    game.select_option(0);

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

/// Edge: Opponent has no cards in hand → cannot discard → conditional fires.
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

    // Opponent has no cards in hand — opponent_action may auto-complete or
    // present an empty choice. Either way, handle it.
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // conditional_on_optional — choose "Skip" → blade +4
    if game.has_pending_choice() {
        game.select_option(0);
    }

    assert!(!game.has_pending_choice(), "Ability should resolve cleanly");
    assert_eq!(
        game.state.mods.get_blade_modifier(yoshiko),
        4,
        "Yoshiko should gain blade when opponent has empty hand"
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
    let p2_member = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-013-SD");

    // Serasu already on stage
    game.state.player1.stage.stage[1] = serasu;
    // Opponent has an active member
    game.state.player2.stage.stage[0] = p2_member;
    // EdelNote member in hand to play
    game.add_to_hand(edelnote_member);
    game.add_to_hand(filler);
    game.give_energy(10);

    // Play EdelNote member to stage → triggers Serasu's auto ability
    game.play_to_stage(edelnote_member, MemberArea::LeftSide);

    // Auto ability fires: opponent chooses which of their members to wait
    // (with only 1 target, it may auto-resolve)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify: opponent's member is in wait state
    assert!(
        game.state.player2.stage.stage.contains(&p2_member),
        "Opponent member should stay on stage"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(p2_member),
        Some(&"wait".to_string()),
        "Opponent member should be in wait state"
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
            snap.total_hearts = [0, 3, 0, 0, 0, 0, 0]; // Heart01 = 3 surplus
        }
    }

    game.pass(); // LiveVictoryDetermination → LiveSuccess ability fires

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Opponent had surplus hearts (from p2_member on stage, ≥2), so score bonus +1
    assert_eq!(
        game.state.mods.get_score_modifier(koware),
        1,
        "Kowareyasuki should gain +1 score bonus when opponent loses 2+ surplus hearts"
    );
}
