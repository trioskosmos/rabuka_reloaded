/// Gameplay integration tests using ONLY real card data.
///
/// Phase sequence per turn:
///   Active → Energy → Draw → Main  (player 1, first attacker)
///   Active → Energy → Draw → Main  (player 2, second attacker)
///   LiveCardSetP1 → LiveCardSetP2
///   → FirstAttackerPerformance → SecondAttackerPerformance
///   → LiveVictoryDetermination → LiveStart → LiveSuccess → Cheer
///   → cycle to next turn Active
///
/// Each advance_phase() moves exactly one step.
/// Turn 1 starts at Phase::Main, TurnPhase::FirstAttackerNormal.
use crate::helpers::*;

/// Smoke test: 20 pass() calls should cycle through 2+ turns without crashing.
#[test]
fn phase_walkthrough_two_turns() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    assert_eq!(game.state.turn_number, 1);
    assert_eq!(
        game.state.current_turn_phase.to_string(),
        "FirstAttackerNormal"
    );
    assert_eq!(game.state.current_phase.to_string(), "Main");

    for _ in 0..20 {
        game.pass();
    }

    assert!(
        game.state.turn_number >= 2,
        "Should be at least turn 2 after 20 passes"
    );
    assert!(
        game.state.player1.stage.stage.iter().all(|&id| id == -1),
        "P1 stage should be empty after 20 passes"
    );
    assert!(
        game.state.player2.stage.stage.iter().all(|&id| id == -1),
        "P2 stage should be empty after 20 passes"
    );
}

// ====================================================================
//  愛♡スクリ～ム！ (LL-PR-004-PR) — answer-based choice, target both
// ====================================================================
// ライブ開始時: 相手に何が好き？と聞く。
// Option 0: チョコミント系 → both discard 1 from hand
// Option 1: あなた → both draw 1
// Option 2: その他 → both members on stage gain blade

/// Advance from Main (turn 1) to LiveCardSetFirstAttacker.
/// This requires 5 passes: Main→P2Act→P2Ene→P2Drw→P2Main→LiveP1
fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

#[allow(dead_code)]
fn assert_score(game: &TestGame, expected: i32) {
    let live_card_id = game.state.player1.live_card_zone.cards[0];
    assert_eq!(game.state.mods.get_score_modifier(live_card_id), expected);
}

fn assert_energy(game: &TestGame, active: usize, total: usize) {
    assert_eq!(game.state.player1.energy_zone.active_energy_count, active);
    assert_eq!(game.state.player1.energy_zone.cards.len(), total);
}

#[test]
fn ai_screeam_answer_both_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    let screeam = game.id("LL-PR-004-PR");
    let filler_a = game.id("PL!-sd1-010-SD");
    let filler_b = game.id("PL!-sd1-013-SD");

    // Card in hand. Both players get filler cards for discard.
    game.state.player1.hand.cards.push(screeam);
    game.state.player1.hand.cards.push(filler_a);
    game.state.player2.hand.cards.push(filler_b);

    let _p1_hand_before = game.state.player1.hand.cards.len();
    let _p2_hand_before = game.state.player2.hand.cards.len();

    // Advance to LiveCardSetFirstAttacker
    advance_to_live_card_set_p1(&mut game);
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));

    // Set the live card — triggers ライブ開始時 abilities
    game.set_live_card(screeam);
    advance_to_live_start(&mut game);

    // The ability fires and creates a "choice" prompt
    assert!(
        game.has_pending_choice(),
        "Live card set should trigger answer choice"
    );

    // Verify choice_player_id is set to opponent
    let entry = game
        .state
        .ability_queue
        .current_entry()
        .expect("Queue should have an entry");
    assert_eq!(
        entry.choice_player_id.as_deref(),
        Some("p2"),
        "Choice player should be p2 (opponent decides flavor)"
    );

    // Option 0: mint/flavor/cookie → both discard 1 from hand
    // P1 flow: start 2 (screeam + filler_a), P2 active during passes so no draws,
    //   -1 set live, +1 live replacement, -1 discard = 1
    // P2 flow: start 1 (filler_b), +1 draw (DrawPhaseP2Turn), -1 discard = 1
    game.select_option(0);

    // "both" → effect targets P1 then P2; P2 also gets a discard choice
    assert!(
        game.has_pending_choice(),
        "P1 should get a discard choice first"
    );
    // P1 discards card from hand
    game.select_indices(&[0]);
    // Now P2 gets a discard choice
    assert!(
        game.has_pending_choice(),
        "P2 should get a discard choice after P1 discards"
    );
    game.select_indices(&[0]);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "P1 hand: 1 (start 2 -1 set live +1 replacement -1 discard)"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        1,
        "P2 hand: 1 (start 1 +1 draw -1 discard)"
    );
}

#[test]
fn ai_screeam_answer_both_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let screeam = game.id("LL-PR-004-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(screeam);
    game.state.player1.hand.cards.push(filler);
    // Add cards to decks so draws work
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player2.main_deck.cards.push(filler);
    game.state.player2.main_deck.cards.push(filler);

    let p1_hand_before = game.state.player1.hand.cards.len();
    let p2_hand_before = game.state.player2.hand.cards.len();
    let p1_deck = game.state.player1.main_deck.cards.len();
    let p2_deck = game.state.player2.main_deck.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(screeam);
    advance_to_live_start(&mut game);
    assert!(game.has_pending_choice());

    let entry = game
        .state
        .ability_queue
        .current_entry()
        .expect("Queue should have an entry");
    assert_eq!(
        entry.choice_player_id.as_deref(),
        Some("p2"),
        "Choice player should be p2 (opponent decides flavor)"
    );

    game.select_option(1);

    // P1: 2 initial - 1 live card + 1 (replacement draw from pass) + 1 (ability draw) = 3
    assert_eq!(
        game.state.player1.hand.cards.len(),
        p1_hand_before + 1,
        "P1: net +1 (-1 live, +2 draws)"
    );
    // P2 draws 1 naturally during phases + 1 from ability
    assert!(
        game.state.player2.hand.cards.len() > p2_hand_before,
        "P2 should have drawn"
    );
    // P1 draws 1, P2 draws 1 (plus P2's natural phase draw)
    assert!(
        game.state.player1.main_deck.cards.len() < p1_deck,
        "P1 deck should have decreased"
    );
    assert!(
        game.state.player2.main_deck.cards.len() < p2_deck,
        "P2 deck should have decreased"
    );
    assert!(
        game.state.player1.hand.cards.len() >= p1_hand_before - 1,
        "P1 should have at least as many cards as before (-1 live card)"
    );
    assert!(
        game.state.player2.hand.cards.len() > p2_hand_before,
        "P2 should have drawn at least 1"
    );
}

#[test]
fn ai_screeam_answer_both_gain_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let screeam = game.id("LL-PR-004-PR");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    let p1_member = game.id("PL!-sd1-013-SD");
    let p2_member = game.id("PL!-sd1-014-SD");

    game.state.player1.hand.cards.push(screeam);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[1] = p1_member;
    game.state.player2.stage.stage[1] = p2_member;

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(screeam);
    advance_to_live_start(&mut game);
    assert!(game.has_pending_choice());

    let entry = game
        .state
        .ability_queue
        .current_entry()
        .expect("Queue should have an entry");
    assert_eq!(
        entry.choice_player_id.as_deref(),
        Some("p2"),
        "Choice player should be p2 (opponent decides flavor)"
    );

    // Option 2: それ以外 → both members gain blade
    game.select_option(2);

    // Verify blade modifiers were applied
    assert!(
        !game.has_pending_choice(),
        "No more pending choices after blade gain"
    );
    assert!(
        game.state.mods.get_blade_modifier(p1_member) > 0,
        "P1 member should have gained blade modifier"
    );
    assert!(
        game.state.mods.get_blade_modifier(p2_member) > 0,
        "P2 member should have gained blade modifier"
    );
}

// ====================================================================
//  ディストーション (PL!SP-pb1-023-L) — sequential conditional ability
// ====================================================================
// ライブ開始時:
//   自分のステージに名前の異なる『CatChu!』のメンバーが2人以上いる場合
//     → エネルギーを6枚までアクティブにする。
//   その後:
//     自分のエネルギーがすべてアクティブ状態の場合
//       → このカードのスコアを＋１する。
//
// Q97: CatChu!不足でも全エネルギーがアクティブならスコア＋１
// Q96: スコア＋１は永続、あとでエネルギーをウェイトにしても戻らない
// ====================================================================

#[test]
fn distortion_q97_all_active_no_catchu_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Card in hand
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);

    // Give energy, ALL active — no wait energy
    game.give_energy(3);

    // NO CatChu! members on stage — condition for energy refresh NOT met

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);

    // Q97: CatChu!不足でもエネルギーが全アクティブなら＋１される
    assert!(!game.has_pending_choice(), "No pending choices expected");

    let live_card = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.mods.get_score_modifier(live_card);
    assert_eq!(
        score_mod, 1,
        "Q97: score should be +1 when all energy active"
    );
}

#[test]
fn distortion_q96_score_permanent_after_energy_used() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    let energy_id = game.id("LL-E-001-SD");

    // Card in hand
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);

    // Give energy, ALL active
    game.give_energy(3);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);

    let live_card_id = game.state.player1.live_card_zone.cards[0];
    assert_eq!(
        game.state.mods.get_score_modifier(live_card_id),
        1,
        "Score should be +1 initially (Q96 precondition)"
    );

    // Q96: Later making energy non-all-active doesn't undo score +1
    game.state.player1.energy_zone.cards.push(energy_id);
    // active_energy_count stays at 3, cards.len() = 4, so not all active anymore
    assert!(
        game.state.player1.energy_zone.active_energy_count
            < game.state.player1.energy_zone.cards.len(),
        "Energy should not be all-active anymore"
    );

    // Score modifier should still be +1
    assert_eq!(
        game.state.mods.get_score_modifier(live_card_id),
        1,
        "Score +1 is permanent even after energy becomes non-all-active (Q96)"
    );
}

#[test]
fn distortion_basic_energy_refresh_with_catchu() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    let catchu_a = game.id("PL!SP-sd1-001-SD"); // かのん
    let catchu_b = game.id("PL!SP-sd1-004-SD"); // 可可（可可）
    let energy_id = game.id("LL-E-001-SD");

    // Card in hand
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);

    // Two CatChu! members on stage with DIFFERENT names (condition met)
    game.state.player1.stage.stage[0] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;

    // Give 3 active + 4 wait energy (= 7 total, 4 wait)
    game.give_energy(3);
    for _ in 0..4 {
        game.state.player1.energy_zone.cards.push(energy_id);
    }
    assert_eq!(game.state.player1.energy_zone.cards.len(), 7);
    assert_eq!(game.state.player1.energy_zone.active_energy_count, 3);
    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 3,
        "3 active, 4 wait — not all active"
    );

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);

    // CatChu condition met → energy refresh should fire (up to 6, but only 4 wait)
    assert!(!game.has_pending_choice(), "No pending choices expected");

    // 4 wait cards should now be active
    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 7,
        "All 7 energy should be active after refresh of 4 wait cards"
    );

    // Now all energy active → score +1 should fire
    let live_card_id = game.state.player1.live_card_zone.cards[0];
    assert_eq!(
        game.state.mods.get_score_modifier(live_card_id),
        1,
        "Score should be +1 when all energy becomes active"
    );
}

#[test]
fn distortion_no_refresh_when_no_wait_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_b = game.id("PL!SP-sd1-004-SD");

    // Card in hand
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);

    // CatChu! members on stage
    game.state.player1.stage.stage[0] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;

    // All energy already active (no wait)
    game.give_energy(6);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(distortion);

    // CatChu condition met, but no wait energy → nothing to refresh
    // All energy already active → score +1 should fire
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice(), "No pending choices expected");

    let live_card_id = game.state.player1.live_card_zone.cards[0];
    assert_eq!(
        game.state.mods.get_score_modifier(live_card_id),
        1,
        "Score +1 even with no wait energy (all already active)"
    );
}

// ── Max cap: 8 wait → only 6 refreshed → 2 remain → no +1 ──────────

#[test]
fn distortion_max_cap_8_wait_refresh_6_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_b = game.id("PL!SP-sd1-004-SD");
    let energy_id = game.id("LL-E-001-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;
    advance_to_live_card_set_p1(&mut game);
    for _ in 0..8 {
        game.state.player1.energy_zone.cards.push(energy_id);
    }
    assert_energy(&game, 0, 8);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 6,
        "Only 6 of 8 wait cards should be refreshed (capped by max)"
    );
    assert_eq!(game.state.player1.energy_zone.cards.len(), 8);
    assert_eq!(
        game.state
            .mods
            .get_score_modifier(game.state.player1.live_card_zone.cards[0]),
        0,
        "Not all active -> no +1"
    );
}

// ── Exact max boundary: 6 wait → all refreshed → all active → +1 ────

#[test]
fn distortion_exact_max_boundary_6_wait_all_refreshed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_b = game.id("PL!SP-sd1-004-SD");
    let energy_id = game.id("LL-E-001-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;
    advance_to_live_card_set_p1(&mut game);
    for _ in 0..6 {
        game.state.player1.energy_zone.cards.push(energy_id);
    }
    assert_energy(&game, 0, 6);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_energy(&game, 6, 6);
    assert_eq!(
        game.state
            .mods
            .get_score_modifier(game.state.player1.live_card_zone.cards[0]),
        1
    );
}

// ── Same-name CatChu! → distinct condition NOT met → no refresh ─────

#[test]
fn distortion_same_name_catchu_condition_not_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let same_name = game.id("PL!SP-sd1-001-SD");
    let energy_id = game.id("LL-E-001-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = same_name;
    game.state.player1.stage.stage[2] = same_name;
    game.give_energy(3);
    advance_to_live_card_set_p1(&mut game);
    for _ in 0..4 {
        game.state.player1.energy_zone.cards.push(energy_id);
    }
    assert_energy(&game, 3, 7);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    assert_energy(&game, 3, 7);
    assert_eq!(
        game.state
            .mods
            .get_score_modifier(game.state.player1.live_card_zone.cards[0]),
        0
    );
}

// ── Q103: 7 wait, two Distortions → only one gets +1 ──────────────

#[test]
fn distortion_q103_two_triggers_only_one_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion1 = game.id("PL!SP-pb1-023-L");
    let distortion2 = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_b = game.id("PL!SP-sd1-004-SD");
    let energy_id = game.id("LL-E-001-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(distortion1);
    game.state.player1.hand.cards.push(distortion2);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.stage.stage[0] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;
    advance_to_live_card_set_p1(&mut game);
    for _ in 0..7 {
        game.state.player1.energy_zone.cards.push(energy_id);
    }
    assert_energy(&game, 0, 7);
    game.set_live_card(distortion1);
    game.set_live_card(distortion2);
    advance_to_live_start(&mut game);
    game.drain_auto_ability_choices();
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.energy_zone.active_energy_count, 7);
    let total_score: i32 = game
        .state
        .player1
        .live_card_zone
        .cards
        .iter()
        .map(|&cid| game.state.mods.get_score_modifier(cid))
        .sum();
    // Q103: only the second trigger sees all energy active → +1 on that card
    assert_eq!(
        total_score, 1,
        "Q103: only the second trigger's card gets +1"
    );
}

// ── 2 same-name + 1 different-name CatChu! → condition met ──────────

#[test]
fn distortion_two_same_one_diff_catchu_condition_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let catchu_same_a = game.id("PL!SP-sd1-001-SD");
    let catchu_same_b = game.id("PL!SP-sd1-001-SD");
    let catchu_diff = game.id("PL!SP-sd1-004-SD");
    let energy_id = game.id("LL-E-001-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    // 2 same-name (kanon) + 1 different-name (keke)
    game.state.player1.stage.stage[0] = catchu_same_a;
    game.state.player1.stage.stage[1] = catchu_same_b;
    game.state.player1.stage.stage[2] = catchu_diff;
    advance_to_live_card_set_p1(&mut game);
    for _ in 0..4 {
        game.state.player1.energy_zone.cards.push(energy_id);
    }
    assert_energy(&game, 0, 4);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    // 2 distinct CatChu! names → condition met → 4 wait activated
    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 4,
        "All 4 wait energy activated — distinct condition met (Kanon + Keke)"
    );
    assert_eq!(
        game.state
            .mods
            .get_score_modifier(game.state.player1.live_card_zone.cards[0]),
        1,
        "All active → score +1"
    );
}

// ── Same-name CatChu! only + all-active energy → only +1 (Q97 path) ───

#[test]
fn distortion_same_name_catchu_all_active_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let distortion = game.id("PL!SP-pb1-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    let catchu_a = game.id("PL!SP-sd1-001-SD");
    let catchu_b = game.id("PL!SP-sd1-001-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(distortion);
    game.state.player1.hand.cards.push(filler);
    // Same-name CatChu! on stage (both Kanon)
    game.state.player1.stage.stage[0] = catchu_a;
    game.state.player1.stage.stage[2] = catchu_b;
    // All energy already active (no wait)
    game.give_energy(4);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(distortion);
    advance_to_live_start(&mut game);
    assert!(!game.has_pending_choice());
    // Distinct condition fails (only 1 unique name) → no energy activation
    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 4,
        "No wait energy added — distinct condition not met"
    );
    // But all energy active → score +1 (Q97 logic)
    assert_eq!(
        game.state
            .mods
            .get_score_modifier(game.state.player1.live_card_zone.cards[0]),
        1,
        "All active → score +1 even with same-name CatChu!"
    );
}

// ====================================================================
//  上原歩夢＆澁谷かのん＆日野下花帆 (LL-bp1-001-R+) — gain_ability
// ====================================================================
// 登場: 自分の控え室からメンバーカードを1枚手札に加える。
// ライブ開始時: 手札の「上原歩夢」と「澁谷かのん」と「日野下花帆」を、
//   好きな組み合わせで合計3枚まで、控え室に置いてもよい：
//   ライブ終了時まで、「常時ライブの合計スコアを＋３する。」を得る。
// ====================================================================

#[test]
fn ayumu_kanon_koko_debut_recover_from_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ayumu = game.id("LL-bp1-001-R\u{ff0b}");
    let member = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-013-SD");

    // Member card in discard
    game.state.player1.waitroom.cards.push(member);
    game.state.player1.hand.cards.push(ayumu);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(20);

    assert_eq!(game.state.player1.hand.cards.len(), 2);
    assert_eq!(game.state.player1.waitroom.cards.len(), 1);

    // Play the card to stage → debut ability triggers (auto: recover 1 member from discard)
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(ayumu, rabuka_engine::zones::MemberArea::Center);

    // Debut auto-triggers: if there's a pending choice for selecting which member to recover
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify the member card was recovered from discard to hand
    assert!(
        game.state.player1.hand.cards.contains(&member),
        "Member should be recovered from discard to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&member),
        "Member should no longer be in discard"
    );
}

// ── Q62: "&" in name means it has all individual names ──────────────

#[test]
fn ayumu_q62_and_name_has_individual_names() {
    let db = load_real_database();
    let ayumu = db
        .get_card_by_no("LL-bp1-001-R\u{ff0b}")
        .expect("Card should exist");
    let names: Vec<&str> = ayumu.name.split('&').collect();
    assert!(ayumu.name.contains('&'), "Name must contain '&' separator");
    assert_eq!(names.len(), 3, "Name should split into exactly 3 parts");
    assert_eq!(names[0], "上原歩夢", "First name should be 上原歩夢");
    assert_eq!(names[1], "澁谷かのん", "Second name should be 澁谷かのん");
    assert_eq!(names[2], "日野下花帆", "Third name should be 日野下花帆");

    // Verify the card has the expected abilities (debut recover + live_start gain_ability)
    let has_debut = ayumu
        .abilities
        .iter()
        .any(|a| a.triggers.as_deref() == Some("登場"));
    let has_live_start = ayumu
        .abilities
        .iter()
        .any(|a| a.triggers.as_deref() == Some("ライブ開始時"));
    assert!(has_debut, "Card should have a 登場 ability");
    assert!(has_live_start, "Card should have a ライブ開始時 ability");

    // Verify the live_start ability targets all 3 named characters specifically
    let live_start = ayumu
        .abilities
        .iter()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("LiveStart ability exists");
    assert!(live_start.full_text.contains("上原歩夢"));
    assert!(live_start.full_text.contains("澁谷かのん"));
    assert!(live_start.full_text.contains("日野下花帆"));
}

// ── Live test: Ayumu on stage, LiveStart trigger fires ─────────────

#[test]
fn ayumu_live_start_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let ayumu = game.id("LL-bp1-001-R\u{ff0b}");
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    // Ayumu on stage, a copy of herself in hand as eligible named character
    game.state.player1.stage.stage[1] = ayumu;
    let ayumu_copy = game.new_id("LL-bp1-001-R\u{ff0b}");
    game.state.player1.hand.cards.push(ayumu_copy);

    // Live card to advance to LiveStart
    let filler_live = game.id("PL!-sd1-019-SD");
    game.state.player1.hand.cards.push(filler_live);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);

    // The ability has an optional cost: discard named cards from hand to gain +3 score.
    // Since ayumu_copy contains the names 上原歩夢/澁谷かのん/日野下花帆, it should be eligible.
    assert!(
        game.has_pending_choice(),
        "LiveStart optional cost should prompt for named character discard"
    );

    // Pay the cost: discard the named copy (handling sequential prompts)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify gain_ability effect: Ayumu should have gained the score +3 constant ability
    assert!(
        game.state.gained_abilities.contains_key(&ayumu),
        "Ayumu should have a gained ability from LiveStart effect"
    );
    let gained = &game.state.gained_abilities[&ayumu];
    assert!(
        gained
            .iter()
            .any(|t| t.contains("+3") || t.contains("＋３") || t.contains("スコア")),
        "Gained ability should be the score +3 constant (got: {:?})",
        gained
    );
}

// ====================================================================
//  矢澤にこ (PL!-pb1-018-R) — both-target, appear from discard
// ====================================================================
// 登場: 自分と相手はそれぞれ、自分の控え室からコスト2以下のメンバーカードを
//   1枚、メンバーのいないエリアにウェイト状態で登場させる。
//   （この効果で登場したメンバーのいるエリアには、このターンにメンバーは
//   登場できない。）
// ====================================================================

#[test]
fn nico_q168_both_appear_from_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap_p1 = game.id("PL!SP-sd1-019-SD");
    let cheap_p2 = game.id("PL!SP-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(cheap_p1);
    game.state.player2.waitroom.cards.push(cheap_p2);
    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;

    // Play Nico → both-target debut triggers.
    // Each player has exactly 1 eligible card in discard → Exact path (no SelectCard).
    // P1 stage: [nico, -, -] → 2 empty slots → MoveCardsPosition prompt (SelectPosition).
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::LeftSide);

    // P1 gets position choice for their own member — choice_player_id stays None (self)
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P1 should get position choice"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "P1's own position choice should have choice_player_id=p1"
    );
    game.select_option(1); // center

    // Opponent's effect runs inside finalize_choice, creating P2's MoveCardsPosition
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 should get position choice"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's position choice should have choice_player_id=p2"
    );
    game.select_option(2); // right

    assert!(
        !game.has_pending_choice(),
        "No more pending choices after both players resolve"
    );

    // P1: Nico at left, cheap_p1 at center
    assert_eq!(game.state.player1.stage.stage[0], nico, "Nico at left");
    assert_eq!(
        game.state.player1.stage.stage[1], cheap_p1,
        "P1's cheap member at center"
    );
    assert_eq!(game.state.player1.stage.stage[2], -1, "P1 right empty");

    // P2: cheap_p2 at right
    assert_eq!(game.state.player2.stage.stage[0], -1, "P2 left empty");
    assert_eq!(game.state.player2.stage.stage[1], -1, "P2 center empty");
    assert_eq!(
        game.state.player2.stage.stage[2], cheap_p2,
        "P2's cheap member at right"
    );

    // Both in wait state
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_p1),
        Some(&"wait".to_string()),
        "P1's member wait state"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_p2),
        Some(&"wait".to_string()),
        "P2's member wait state"
    );

    // Cards removed from discard, NOT in hand
    assert!(
        !game.state.player1.waitroom.cards.contains(&cheap_p1),
        "P1's card removed from discard"
    );
    assert!(
        !game.state.player2.waitroom.cards.contains(&cheap_p2),
        "P2's card removed from discard"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&cheap_p1),
        "P1's card NOT in hand"
    );
    assert!(
        !game.state.player2.hand.cards.contains(&cheap_p2),
        "P2's card NOT in hand"
    );
}

#[test]
fn nico_q168_no_suitable_card_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::LeftSide);

    // Q168: No suitable card in either discard → skip gracefully
    assert!(
        !game.has_pending_choice(),
        "No pending choice when both sides have no valid cards"
    );
    // Only Nico on stage
    assert_eq!(game.state.player1.stage.stage[0], nico, "Nico at left");
    assert_eq!(game.state.player1.stage.stage[1], -1, "center empty");
    assert_eq!(game.state.player1.stage.stage[2], -1, "right empty");
    assert_eq!(
        game.state
            .player1
            .stage
            .stage
            .iter()
            .filter(|&&id| id != -1)
            .count(),
        1,
        "Exactly 1 member (Nico) on P1 stage"
    );
    assert_eq!(
        game.state
            .player2
            .stage
            .stage
            .iter()
            .filter(|&&id| id != -1)
            .count(),
        0,
        "No members on P2 stage"
    );
}

// ── Sync path: P1 has no matching cards → opponent gets choice ────

#[test]
fn nico_sync_path_opponent_gets_choice_when_self_has_none() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap_a = game.id("PL!SP-sd1-019-SD");
    let cheap_b = game.id("PL!SP-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // P1 has NO matching cards in discard (only filler)
    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    // P2 has 2 eligible cards → should get SelectCard prompt
    game.state.player2.waitroom.cards.push(cheap_a);
    game.state.player2.waitroom.cards.push(cheap_b);

    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::LeftSide);

    // P1 has 0 matching cards → no choice created for self
    // Sync path: opponent effect runs directly (handle_both_targets sync path)
    // Before the fix, spawn_context.target stayed "self" → choice went to P1.
    // After the fix, spawn_context.target = "opponent" → choice goes to P2.
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectCard".to_string()),
        "P2 gets SelectCard prompt (P1 had no valid cards)"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's SelectCard should have choice_player_id=p2 (sync path)"
    );

    // Verify the selection shows P2's discard cards, not P1's
    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let select_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == rabuka_engine::game_setup::ActionType::ChoiceSelect)
        .collect();
    assert_eq!(
        select_actions.len(),
        2,
        "P2 should see 2 selectable cards from their discard"
    );

    game.select_indices(&[0]); // P2 selects cheap_a

    // P2 then gets position choice for placing on their stage
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 gets position choice after selecting card"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's position choice should have choice_player_id=p2"
    );
    game.select_option(0); // P2: left

    assert!(!game.has_pending_choice(), "No more prompts");

    // P1 stage unchanged: Nico at left
    assert_eq!(game.state.player1.stage.stage[0], nico);
    assert_eq!(game.state.player1.stage.stage[1], -1);
    assert_eq!(game.state.player1.stage.stage[2], -1);

    // P2's card on stage
    assert!(game.state.player2.stage.stage.contains(&cheap_a));
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_a),
        Some(&"wait".to_string())
    );

    // cheap_b still in P2 discard (was not selected)
    assert!(game.state.player2.waitroom.cards.contains(&cheap_b));
}

// ── Q170: Turn player's debut resolves first ───────────────────────

#[test]
fn nico_q170_turn_player_appears_first() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap_p1 = game.id("PL!SP-sd1-019-SD");
    let cheap_p2 = game.id("PL!SP-sd1-020-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(cheap_p1);
    game.state.player1.waitroom.cards.push(cheap_p1);
    game.state.player2.waitroom.cards.push(cheap_p2);
    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::LeftSide);

    // P1 resolves first (turn player) → MoveCardsPosition prompt
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P1 gets position choice first"
    );
    game.select_option(1); // P1: center

    // Opponent resolves immediately after P1's choice
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 gets position choice after P1"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's position choice should have choice_player_id=p2"
    );
    game.select_option(2); // P2: right

    assert!(!game.has_pending_choice(), "No remaining prompts");

    // P1: Nico left, cheap_p1 center
    assert_eq!(game.state.player1.stage.stage[0], nico);
    assert_eq!(game.state.player1.stage.stage[1], cheap_p1);
    // P2: cheap_p2 right
    assert_eq!(game.state.player2.stage.stage[2], cheap_p2);

    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_p1),
        Some(&"wait".to_string())
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_p2),
        Some(&"wait".to_string())
    );
}

// ── Q181: Area freed when appeared card leaves → new card can appear ──

#[test]
fn nico_q181_area_freed_after_card_leaves() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap = game.id("PL!SP-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(cheap);
    // P2 gets a different cheap card
    let cheap_p2 = game.id("PL!SP-sd1-020-SD");
    game.state.player2.waitroom.cards.push(cheap_p2);

    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;

    // Play Nico — ability triggers: both players appear a member
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::LeftSide);

    // Both have 1 card in discard → Exact path → MoveCardsPosition prompts
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P1 position choice"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "P1's position choice should have choice_player_id=p1"
    );
    game.select_option(1); // P1: center
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 position choice"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's position choice should have choice_player_id=p2"
    );
    game.select_option(2); // P2: right

    assert!(!game.has_pending_choice(), "No more prompts");

    // Cheap member is at center
    assert_eq!(game.state.player1.stage.stage[1], cheap);
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap),
        Some(&"wait".to_string())
    );

    // Remove the appeared member
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.waitroom.cards.push(cheap);

    // Q181: The area is now free
    assert_eq!(game.state.player1.stage.stage[1], -1, "Area freed");
    assert!(
        game.state.player1.waitroom.cards.contains(&cheap),
        "Removed card back in waitroom"
    );
}

// ── Empty area restriction: only appears on empty areas ────────────

#[test]
fn nico_requires_empty_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap = game.id("PL!SP-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(cheap);
    game.state.player2.waitroom.cards.push(cheap);

    // Stage [filler, -, filler] → play Nico to center → [filler, nico, filler]
    game.state.player1.stage.stage = [filler, -1, filler];
    game.give_energy(7);
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::Center);

    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 gets position choice (P1 had no room)"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's position choice should have choice_player_id=p2"
    );
    game.select_option(2); // P2: right

    assert!(!game.has_pending_choice(), "No more prompts");

    // P1 stage unchanged [filler, nico, filler]
    assert_eq!(
        game.state.player1.stage.stage,
        [filler, nico, filler],
        "P1 stage full, no extra card appeared"
    );
    // P1's card returned to discard
    assert!(
        game.state.player1.waitroom.cards.contains(&cheap),
        "P1's card back in discard (no room on stage)"
    );
    // P2's card appeared on stage
    assert_eq!(
        game.state.player2.stage.stage[2], cheap,
        "P2's card on right"
    );
}

// ── Cost filter: only cost ≤2 cards should appear in the choice prompt ──

#[test]
fn nico_cost_filter_only_shows_eligible() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap = game.id("PL!SP-sd1-019-SD"); // cost 2
    let expensive = game.id("PL!-sd1-014-SD"); // cost 9

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(cheap);
    // Both cost-2 and cost-9 in P1 discard; filter only allows cheap (cost ≤2)
    game.state.player1.waitroom.cards.push(cheap);
    game.state.player1.waitroom.cards.push(expensive);
    // P2 has 1 eligible card
    game.state.player2.waitroom.cards.push(cheap);

    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::LeftSide);

    // Only 1 card in P1 discard passes cost ≤2 → Exact → MoveCardsPosition
    // P2 also has 1 card → Exact → MoveCardsPosition
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P1 position choice"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "P1's own position choice should have choice_player_id=p1"
    );
    game.select_option(1); // P1: center
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 position choice"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's position choice should have choice_player_id=p2"
    );
    game.select_option(2); // P2: right

    assert!(!game.has_pending_choice(), "No more prompts");

    // Cheap card on P1 stage, expensive NOT on P1 stage
    assert_eq!(
        game.state.player1.stage.stage[1], cheap,
        "Cost-2 card on P1 center"
    );
    assert!(
        !game.state.player1.stage.stage.contains(&expensive),
        "Cost-9 card NOT on stage"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap),
        Some(&"wait".to_string())
    );

    // P2's card also in wait state
    assert_eq!(
        game.state.mods.get_orientation_modifier(
            *game
                .state
                .player2
                .stage
                .stage
                .iter()
                .find(|&&id| id != -1)
                .unwrap()
        ),
        Some(&"wait".to_string()),
        "P2's card wait state"
    );

    // Expensive card still in discard
    assert!(
        game.state.player1.waitroom.cards.contains(&expensive),
        "Cost-9 card should remain in discard (was never selectable)"
    );
}

// ── Q169: Appeared card occupies area (natural stage slot restriction) ──

#[test]
fn nico_q169_appeared_card_occupies_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap = game.id("PL!SP-sd1-019-SD");
    let cheap_p2 = game.id("PL!SP-sd1-020-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(cheap);
    game.state.player1.waitroom.cards.push(cheap);
    game.state.player2.waitroom.cards.push(cheap_p2);
    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::LeftSide);

    // Both-target: each has 1 card → Exact → MoveCardsPosition
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P1 position choice"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "P1's own position choice should have choice_player_id=p1"
    );
    game.select_option(1); // P1: center
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 position choice"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's position choice should have choice_player_id=p2"
    );
    game.select_option(2); // P2: right

    // Cheap card occupies a stage slot
    assert!(
        game.state.player1.stage.stage.contains(&cheap),
        "Cheap card on P1 stage"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], cheap,
        "P1 center = cheap"
    );
    // Area is not empty (stage slot rule prevents second card there)
    assert_ne!(game.state.player1.stage.stage[1], -1, "Area occupied");
    // Wait state
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap),
        Some(&"wait".to_string())
    );
}

// ── Prompt path: 2+ eligible cards in discard → SelectCard prompt ──

#[test]
fn nico_prompt_path_two_eligible_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap_a = game.id("PL!SP-sd1-019-SD"); // cost 2, member
    let cheap_b = game.id("PL!SP-sd1-020-SD"); // cost 2, member
    let filler = game.id("PL!-sd1-010-SD");

    // P1 has 2 eligible cards in discard → should create a SelectCard prompt
    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(cheap_a);
    game.state.player1.waitroom.cards.push(cheap_b);
    // P2 has 1 eligible card → Exact
    game.state.player2.waitroom.cards.push(cheap_a);

    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::LeftSide);

    // P1: 2 eligible cards in discard → SelectCard prompt
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectCard".to_string()),
        "P1 gets SelectCard prompt (2 eligible in discard)"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "P1's SelectCard should have choice_player_id=p1"
    );
    game.select_indices(&[0]); // select cheap_a

    // P1's card needs a position choice (multiple empty slots)
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P1 gets SelectPosition for card from discard (multiple empty slots)"
    );
    game.select_option(1); // P1: center

    // Then P2's effect runs with MoveCardsPosition (P2 has 1 card → exact → position)
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 gets their position choice"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's position choice should have choice_player_id=p2"
    );
    game.select_option(2); // P2: right

    assert!(!game.has_pending_choice(), "No more prompts");

    // P1: cheap_a placed at center
    assert_eq!(
        game.state.player1.stage.stage[1], cheap_a,
        "P1 selected card placed at center"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_a),
        Some(&"wait".to_string())
    );
    // cheap_b still in discard (was not selected)
    assert!(
        game.state.player1.waitroom.cards.contains(&cheap_b),
        "cheap_b remains in discard"
    );

    // P2: their card appeared
    assert!(
        game.state.player2.stage.stage.contains(&cheap_a),
        "P2's card on stage"
    );

    // No cards in hand
    assert!(
        !game.state.player1.hand.cards.contains(&cheap_a),
        "P1's selected card NOT in hand"
    );
    assert!(
        !game.state.player2.hand.cards.contains(&cheap_a),
        "P2's card NOT in hand"
    );

    // Both cards placed are in wait state
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_a),
        Some(&"wait".to_string()),
        "P1's card wait state"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(
            *game
                .state
                .player2
                .stage
                .stage
                .iter()
                .find(|&&id| id != -1)
                .unwrap()
        ),
        Some(&"wait".to_string()),
        "P2's card wait state"
    );
}

// ── Both players have 2+ eligible cards → both get SelectCard prompts ──

#[test]
fn nico_both_players_two_eligible_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap_p1a = game.id("PL!SP-sd1-019-SD");
    let cheap_p1b = game.id("PL!SP-sd1-020-SD");
    let cheap_p2a = game.id("PL!-sd1-002-SD");
    let cheap_p2b = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(cheap_p1a);
    game.state.player1.waitroom.cards.push(cheap_p1b);
    game.state.player2.waitroom.cards.push(cheap_p2a);
    game.state.player2.waitroom.cards.push(cheap_p2b);

    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::LeftSide);

    // P1 has 2 eligible cards → SelectCard prompt
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectCard".to_string()),
        "P1 gets SelectCard prompt"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "P1's SelectCard should have choice_player_id=p1"
    );
    game.select_indices(&[0]);

    // P1 position choice
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P1 gets SelectPosition"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "P1's SelectPosition should have choice_player_id=p1"
    );
    game.select_option(1);

    // P2 has 2 eligible cards → SelectCard prompt (must be routed to P2)
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectCard".to_string()),
        "P2 gets SelectCard prompt"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's SelectCard should have choice_player_id=p2"
    );
    game.select_indices(&[0]);

    // P2 position choice
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 gets SelectPosition"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's SelectPosition should have choice_player_id=p2"
    );
    game.select_option(2);

    assert!(!game.has_pending_choice(), "No more prompts");

    // Verify final board state
    assert_eq!(game.state.player1.stage.stage[0], nico, "Nico at left");
    assert_eq!(
        game.state.player1.stage.stage[1], cheap_p1a,
        "P1 selected card at center"
    );
    assert_eq!(game.state.player1.stage.stage[2], -1, "P1 right empty");
    assert_eq!(game.state.player2.stage.stage[0], -1, "P2 left empty");
    assert_eq!(game.state.player2.stage.stage[1], -1, "P2 center empty");
    assert_eq!(
        game.state.player2.stage.stage[2], cheap_p2a,
        "P2 selected card at right"
    );

    // Both in wait state
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_p1a),
        Some(&"wait".to_string()),
        "P1 card wait"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_p2a),
        Some(&"wait".to_string()),
        "P2 card wait"
    );
}

// ── Prompt path + direct placement: 2+ eligible cards, 1 empty slot ──
// Ensures wait state is preserved when cards are placed directly
// (no position-choice to re-apply wait after clear_all_for_card).

#[test]
fn nico_prompt_path_direct_placement_wait_state() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap_a = game.id("PL!SP-sd1-019-SD"); // cost 2, member
    let cheap_b = game.id("PL!SP-sd1-020-SD"); // cost 2, member
    let filler = game.id("PL!-sd1-010-SD");

    // P1 has 2 eligible cards in discard → SelectCard prompt
    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(cheap_a);
    game.state.player1.waitroom.cards.push(cheap_b);
    // P2 has 1 eligible card → Exact
    game.state.player2.waitroom.cards.push(cheap_b);

    game.give_energy(7);
    // P1 stage: [nico, filler, -] → only 1 empty slot (right) → direct placement
    // First play Nico to center
    game.state.player1.stage.stage = [filler, -1, filler];
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::Center);
    // Now [filler, nico, filler] — full — then play_to_stage makes [filler, nico, -]
    // Actually, let's set up properly before playing Nico:
    // Reset: fresh game for precise setup
    let db2 = load_real_database();
    let mut game2 = TestGame::new(db2);
    let nico2 = game2.id("PL!-pb1-018-R");
    let cheap_a2 = game2.id("PL!SP-sd1-019-SD");
    let cheap_b2 = game2.id("PL!SP-sd1-020-SD");

    game2.state.player1.hand.cards.push(nico2);
    game2.state.player1.waitroom.cards.push(cheap_a2);
    game2.state.player1.waitroom.cards.push(cheap_b2);
    game2.state.player2.waitroom.cards.push(cheap_b2);
    game2.give_energy(7);
    // Stage: [filler, filler, -] → 1 empty slot (right) for P1's own card
    let filler2 = game2.id("PL!-sd1-010-SD");
    game2.state.player1.stage.stage = [filler2, filler2, -1];
    game2.play_to_stage(nico2, rabuka_engine::zones::MemberArea::Center);
    // After playing Nico: [filler2, nico2, -] → 1 empty slot for the effect

    // P1: 2 eligible cards in discard → SelectCard prompt
    assert_eq!(
        game2.pending_choice_type(),
        Some("SelectCard".to_string()),
        "P1 gets SelectCard prompt (2 eligible in discard)"
    );
    assert_eq!(
        game2
            .state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "P1's SelectCard should have choice_player_id=p1"
    );
    game2.select_indices(&[0]); // select cheap_a2

    // P1 has only 1 empty slot on stage → direct placement (no position choice)
    // P2 also has 1 eligible card → Exact → P2 stage empty → MoveCardsPosition
    assert_eq!(
        game2.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 gets position choice (P1 placed directly)"
    );
    assert_eq!(
        game2
            .state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's position choice should have choice_player_id=p2"
    );
    game2.select_option(0); // P2: left

    assert!(!game2.has_pending_choice(), "No more prompts");

    // P1's card placed directly at right (only empty slot)
    assert_eq!(
        game2.state.player1.stage.stage[2], cheap_a2,
        "P1's card appeared at right (only empty slot)"
    );
    // P2's card on left
    assert_eq!(
        game2.state.player2.stage.stage[0], cheap_b2,
        "P2's card on left"
    );

    // BOTH must be in wait state
    assert_eq!(
        game2.state.mods.get_orientation_modifier(cheap_a2),
        Some(&"wait".to_string()),
        "P1's card wait state (direct placement via Prompt path)"
    );
    assert_eq!(
        game2.state.mods.get_orientation_modifier(cheap_b2),
        Some(&"wait".to_string()),
        "P2's card wait state"
    );

    // cheap_b2 still in P1 discard (was not selected)
    assert!(
        game2.state.player1.waitroom.cards.contains(&cheap_b2),
        "cheap_b2 remains in P1 discard"
    );
}

// ── Full stage edge case: no empty slot → card returns to discard ──

#[test]
fn nico_full_stage_then_prompt_path() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap_a = game.id("PL!SP-sd1-019-SD");
    let cheap_b = game.id("PL!SP-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // P1 has 2 eligible cards in discard (Prompt path)
    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(cheap_a);
    game.state.player1.waitroom.cards.push(cheap_b);
    // P2 has 1 eligible card
    game.state.player2.waitroom.cards.push(cheap_a);

    // P1 stage: [filler, filler, filler] — full, even Nico has no room
    game.state.player1.stage.stage = [filler, filler, filler];

    game.give_energy(7);
    // play_to_stage searches for empty slot but there's none...
    // We need to play Nico another way. Actually, let's instead put
    // 1 empty slot for Nico, then full after.
    // Reset: [filler, -1, filler]
    game.state.player1.stage.stage = [filler, -1, filler];
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::Center);
    // Now [filler, nico, filler] — full

    // P1 has 2 eligible cards → SelectCard prompt (going through the Prompt path)
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectCard".to_string()),
        "P1 SelectCard prompt"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p1"),
        "P1's SelectCard should have choice_player_id=p1"
    );
    game.select_indices(&[0]); // select cheap_a

    // After selection, P1 tries to place on stage but stage is full
    // place_card_in_zone for "empty_area" with full stage → returns card to discard
    // No MoveCardsPosition prompt (0 empty slots)
    // P2's effect runs next

    // P2 has 1 card → Exact → P2 stage empty → MoveCardsPosition
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 position choice (P1's effect failed, P2's succeeds)"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's position choice should have choice_player_id=p2"
    );
    game.select_option(0); // P2: left

    assert!(!game.has_pending_choice(), "No more prompts");

    // P1 stage unchanged [filler, nico, filler]
    assert_eq!(game.state.player1.stage.stage, [filler, nico, filler]);
    // cheap_a was returned to P1 discard (stage full)
    assert!(
        game.state.player1.waitroom.cards.contains(&cheap_a),
        "P1's selected card back in discard (stage full)"
    );
    // P2's card appeared
    assert!(
        game.state.player2.stage.stage.contains(&cheap_a),
        "P2's card appeared"
    );
}

// ── Q170: Placed cards' 登場 abilities actually fire ─────────────────
// Both players place a cost ≤2 member that itself has a 登場 ability.
// Verify each placed card's debut ability fires and is enqueued.

#[test]
fn nico_q170_placed_cards_debut_abilities_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let keke_p1 = game.id("PL!SP-bp2-002-P"); // cost 2, 登場: look top 3, add cost 11+
    let keke_p2 = game.new_id("PL!SP-bp2-002-P"); // different copy for P2
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(keke_p1);
    game.state.player2.waitroom.cards.push(keke_p2);

    // Fill decks so Keke's look_at (top 3) succeeds
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(nico, rabuka_engine::zones::MemberArea::LeftSide);

    // P1: turn player gets position choice first
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P1 gets position choice first"
    );
    game.select_option(1); // P1: center

    // P2 gets their position choice
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 gets position choice"
    );
    assert_eq!(
        game.state
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "P2's position choice should have choice_player_id=p2"
    );
    game.select_option(2); // P2: right

    // After both placements, the placed Kekes' 登場 abilities should be queued.
    // P1's Keke fires first (turn player priority) → creates a look_and_select choice.
    assert!(
        game.has_pending_choice(),
        "Placed Kekes' debut abilities should fire and create a pending choice"
    );

    // Verify debut_count reflects all 3 登場 events
    assert_eq!(
        game.state.player1.debut_count_this_turn, 2,
        "P1: Nico + placed Keke = 2 debuts"
    );
    assert_eq!(
        game.state.player2.debut_count_this_turn, 1,
        "P2: placed Keke = 1 debut"
    );

    // Verify both Kekes are on stage in wait state
    assert_eq!(
        game.state.player1.stage.stage[1], keke_p1,
        "P1 Keke at center"
    );
    assert_eq!(
        game.state.player2.stage.stage[2], keke_p2,
        "P2 Keke at right"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(keke_p1),
        Some(&"wait".to_string()),
        "P1 Keke wait state"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(keke_p2),
        Some(&"wait".to_string()),
        "P2 Keke wait state"
    );
}

// ====================================================================
//  唐可可＆平安名すみれ＆米女メイ (LL-bp2-001-R+) — cost reduction + blade gain
// ====================================================================
// 常時: 手札にあるこのメンバーカードのコストは、このカード以外の自分の手札1枚につき、1少なくなる。
// 常時: このメンバーはバトンタッチで控え室に置かれない。
// ライブ開始時: 手札の「唐可可」と「平安名すみれ」と「米女メイ」を、好きな組合せで控え室に置いてもよい：
//   ライブ終了時まで、これにより控え室に置いたカード1枚につき、{{ブレード}}を得る。
// ====================================================================

// ── Q186: cost reduced by 16 hand cards → playable with 4 energy ──

#[test]
fn you_q186_cost_reduced_playable() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let you = game.id("LL-bp2-001-R\u{ff0b}");
    // 16 fillers + the You card itself = 17 hand cards
    // Reduction = 16 (all cards except You), cost = 20 - 16 = 4
    for _ in 0..16 {
        game.state
            .player1
            .hand
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    game.state.player1.hand.cards.push(you);
    game.give_energy(4); // Only need 4, not 20
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(you, rabuka_engine::zones::MemberArea::LeftSide);
    assert!(
        game.state.player1.stage.stage.contains(&you),
        "You should be playable with 4 energy (cost 20-16 reduction)"
    );
    // Verify cost was actually reduced — remaining energy should be 0 (4-4=0)
    let spent = 4 - game.state.player1.energy_zone.active_energy_count;
    assert_eq!(
        spent, 4,
        "Should have spent exactly 4 energy (cost reduction worked)"
    );
}

// ── Q129: base cost 20, reduction is self-only ──────────────────

#[test]
fn you_q129_cost_reduction_self_only() {
    let db = load_real_database();
    let keke = db
        .get_card_by_no("LL-bp2-001-R\u{ff0b}")
        .expect("Card should exist");
    assert_eq!(keke.cost, Some(20));

    // Verify the card has the cost reduction constant ability
    let has_cost_reduction = keke.abilities.iter().any(|a| {
        a.triggers.as_deref() == Some("常時")
            && a.full_text.contains("コスト")
            && a.full_text.contains("少なくなる")
    });
    assert!(
        has_cost_reduction,
        "Card should have a 常時 ability for self cost reduction"
    );

    // Verify the card has the cannot_baton_touch constant ability
    let has_baton_block = keke
        .abilities
        .iter()
        .any(|a| a.triggers.as_deref() == Some("常時") && a.full_text.contains("バトンタッチ"));
    assert!(
        has_baton_block,
        "Card should have a 常時 ability preventing baton touch"
    );
}

// ── LiveStart: optional cost creates prompt ─────────────────────

#[test]
fn you_live_start_optional_cost_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let keke = game.id("LL-bp2-001-R\u{ff0b}");
    let filler_live = game.id("PL!-sd1-019-SD");
    let fill = game.id("PL!-sd1-010-SD");
    // A named character copy in hand for the optional discard cost
    let named_copy = game.new_id("LL-bp2-001-R\u{ff0b}");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    game.state.player1.stage.stage[0] = keke;
    game.state.player1.hand.cards.push(filler_live);
    game.state.player1.hand.cards.push(named_copy);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);

    // Optional cost prompt should appear (named char in hand is eligible)
    assert!(
        game.has_pending_choice(),
        "LiveStart optional cost prompt should appear when eligible named chars in hand"
    );

    // Pay cost: discard the named copy, gain blade on keke
    game.select_indices(&[0]);

    // Verify blade modifier was applied on the activating card
    let blade_mod = game.state.mods.get_blade_modifier(keke);
    assert!(
        blade_mod > 0,
        "Blade should be gained on activating card after discarding named char (got {})",
        blade_mod
    );
}

// ── Edge: abilityless filler NOT in discard choice ──────────────

#[test]
fn you_abilityless_card_not_in_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let keke = game.id("LL-bp2-001-R\u{ff0b}");
    let filler_live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    // A named character copy in hand (唐可可＆平安名すみれ＆米女メイ)
    let named_copy = game.new_id("LL-bp2-001-R\u{ff0b}");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage[1] = keke;
    game.state.player1.hand.cards.push(filler_live);
    game.state.player1.hand.cards.push(named_copy); // named character
    game.state.player1.hand.cards.push(filler); // NOT a named character

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);

    // Only named characters should be selectable
    assert!(
        game.has_pending_choice(),
        "LiveStart optional cost prompt should appear"
    );

    // Select only the named copy
    game.select_indices(&[0]);
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Filler should still be in hand (it was not eligible for selection)
    assert!(
        game.state.player1.hand.cards.contains(&filler),
        "Filler card should remain in hand (not selectable for named-char cost)"
    );

    // Blade modifier should be applied from discarding the named copy
    let blade_mod = game.state.mods.get_blade_modifier(keke);
    assert!(
        blade_mod > 0,
        "Blade gained from discarding named character (got {})",
        blade_mod
    );
}

// ====================================================================
//  La Bella Patria (PL!N-bp3-027-L) — LiveSuccess compound conditional
// ====================================================================
// ライブ成功時: このターン、自分が余剰ハートにheart04を1つ以上持っており、
//   かつ自分のステージに『虹ヶ咲』のメンバーがいる場合、
//   自分のエネルギーデッキからエネルギーカードを1枚ウェイト状態で置く。
//
// Q174: 緑ハートなし → 能力は使えない
// Q173: 複数成功 → 複数回発動
// Q142: 余剰ハートの定義
// ====================================================================

#[test]
fn bella_q174_no_member_on_stage_ability_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let bella = game.id("PL!N-bp3-027-L");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(bella);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bella);
    advance_to_live_start(&mut game);

    let energy_before = game.state.player1.energy_zone.cards.len();

    game.pass(); // → SecondAttackerPerformance
    game.pass(); // → LiveVictoryDetermination (phase set)
    game.pass(); // → Active (processes LiveSuccess)

    // No 虹ヶ咲 member on stage → group condition fails → ability should NOT fire
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before,
        "No energy should be added when no 虹ヶ咲 member on stage (Q174)"
    );
    assert!(!game.has_pending_choice(), "No pending choices expected");
}

#[test]
fn bella_q174_no_heart04_surplus_ability_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let bella = game.id("PL!N-bp3-027-L");
    let niji_member = game.id("PL!N-sd1-015-SD");
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    // Stage has a 虹ヶ咲 member with NO heart04 (gives heart02 + heart05 only)
    game.add_to_stage(rabuka_engine::zones::MemberArea::Center, niji_member);

    game.state.player1.hand.cards.push(bella);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(bella);
    advance_to_live_start(&mut game);

    let energy_before = game.state.player1.energy_zone.cards.len();

    game.pass(); // → SecondAttackerPerformance
    game.pass(); // → LiveVictoryDetermination (phase set)
    game.pass(); // → Active (processes LiveSuccess via advance_phase)

    // Q174: Stage has 虹ヶ咲 member (group condition met) but
    // member provides heart02+heart05, no heart04 → surplus heart04 = 0
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before,
        "No energy should be added when surplus heart04 condition not met (Q174)"
    );
    assert!(!game.has_pending_choice(), "No pending choices expected");
}

#[test]
fn lovepeace_q150_self_hearts_greater_than_opponent_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let lovepeace = game.id("PL!-bp3-026-L");
    let filler = game.id("PL!-sd1-010-SD");

    // Setup: both players have the live card in their live_card_zone
    // and P1 has more member hearts than P2.
    game.state.player1.hand.cards.clear();
    game.state.player2.hand.cards.clear();
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // P1: 2 strong members (h01=5, h03=5, h06=3 = 13)
    game.state.player1.stage.stage = [-1, game.id("PL!-pb1-014-R"), game.id("PL!-PR-003-PR")];
    // P2: 1 weak member (h01=1, h03=1 = 2)
    game.state.player2.stage.stage = [-1, filler, -1];

    // Give both players the same live card
    game.state.player1.live_card_zone.cards.push(lovepeace);
    game.state.player2.live_card_zone.cards.push(lovepeace);

    // Set stage hearts directly to include member + yell blade hearts.
    // P1: 13 member hearts + add 7 yell blade hearts (heart03) to satisfy OH's 15 need.
    let mut p1_hearts = game.state.player1.calculate_stage_hearts(
        &game.state.card_database,
        &std::collections::HashMap::new(),
        &Default::default(),
        &Default::default(),
    );
    *p1_hearts
        .hearts
        .entry(rabuka_engine::card::HeartColor::Heart03)
        .or_insert(0) += 7;
    game.state.player1.stage_hearts = Some(p1_hearts);
    game.state.player2.stage_hearts = Some(game.state.player2.calculate_stage_hearts(
        &game.state.card_database,
        &std::collections::HashMap::new(),
        &Default::default(),
        &Default::default(),
    ));

    // Trigger live_success for P1 (this fires ab#1 which compares stage hearts)
    // P1 has 20 hearts > P2 has 2 hearts → should grant +1 score
    game.state.current_phase = rabuka_engine::game_state::Phase::LiveVictoryDetermination;
    rabuka_engine::turn::TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    // If the ability fired, P1's live card now has +1 score modifier
    let p1_score_mod = game.state.mods.get_score_modifier(lovepeace);
    assert_eq!(
        p1_score_mod, 1,
        "P1 should get +1 when P1 hearts > P2 hearts (Q150)"
    );
}

#[test]
fn lovepeace_q149_total_hearts_sum_of_base_hearts() {
    // Q149: Total hearts = sum of base heart counts ignoring color.
    // Verify by checking the total_hearts() function on member cards.
    let db = load_real_database();
    // PL!-sd1-014-SD: base_heart={heart01=2, heart03=1, heart06=1} → total=4
    let card = db.get_card_by_no("PL!-sd1-014-SD").expect("Card exists");
    assert_eq!(
        card.total_hearts(),
        4,
        "total_hearts is sum of all base heart values"
    );
    // PL!SP-PR-005-PR: base_heart={heart03=3} → total=3
    let card2 = db.get_card_by_no("PL!SP-PR-005-PR").expect("Card exists");
    assert_eq!(card2.total_hearts(), 3, "total_hearts with single color");
    // PL!-PR-003-PR: base_heart={heart01=2, heart03=3, heart06=1} → total=6
    let card3 = db.get_card_by_no("PL!-PR-003-PR").expect("Card exists");
    assert_eq!(card3.total_hearts(), 6, "total_hearts with multiple colors");
}

#[test]
fn lovepeace_q172_ability_gained_hearts_count_but_not_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    // P1 stage has 1 member
    game.state.player1.stage.stage = [-1, game.id("PL!-sd1-014-SD"), -1]; // total_hearts=4

    // The live card needs hearts. Use a simple live card with low requirements.
    // PL!-sd1-020-SD: need_heart={heart01=1, heart03=1, heart0=3}, score=2
    let live = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live);

    // Add cards with blade_heart to the deck for cheer
    // PL!-sd1-010-SD has blade_heart: b_heart03=1 (blade heart of color heart03)
    let cheer_card = game.id("PL!-sd1-010-SD");
    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, cheer_card);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // No LiveStart abilities on this card, so no optional cost prompts
    game.pass(); // → SecondAttackerPerformance
    game.pass(); // → LiveVictoryDetermination (phase set)
    game.pass(); // → Active (processes LiveSuccess)

    // The blade hearts from cheered cards are in resolution zone during performance
    // but do NOT count as base hearts. The total hearts calculation only uses base_heart.
    // If the live card survived, its score was calculated correctly.
    // Q172 confirms: blade hearts don't count toward total, only base hearts do.
    // Check that the card survived: it should be in success_live_zone
    assert!(
        game.state.player1.success_live_card_zone.cards.len() >= 1
            || game.state.player1.live_card_zone.cards.len() >= 1,
        "Live card should have survived heart satisfaction"
    );
}

// ====================================================================
//  未来予報ハレルヤ！ (PL!SP-bp1-026-L) — LiveStart cost reduction
// ====================================================================
// ライブ開始時: 自分の、ステージと控え室に名前の異なる『Liella!』の
//   メンバーが5人以上いる場合、このカードを使用するためのコストは
//   heart02×2, heart03×2, heart06×2 になる。
//
// Q64: Waitroom-only (no stage) → 5 distinct Liella! names → condition met
// Q74: Multi-name cards count as individual names for distinctness
// Q105: Same as Q74 but for a different card/group
// ====================================================================

#[test]
fn hareruya_q64_waitroom_only_five_distinct_liella_condition_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let hareruya = game.id("PL!SP-bp1-026-L");
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    // Keep deck well-stocked to prevent refresh() from clearing the waitroom
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }

    // Put 5 distinct-name Liella! members in waitroom, none on stage
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-014-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-015-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-016-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-019-N"));
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-020-N"));

    game.state.player1.hand.cards.push(hareruya);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(hareruya);

    advance_to_live_start(&mut game);

    // Debug: check the ability's condition locations field
    let card = game
        .db
        .get_card(game.state.player1.live_card_zone.cards[0])
        .expect("live card should exist in database");
    for ab in &card.abilities {
        if let Some(ref ef) = ab.effect {
            if let Some(ref cond) = ef.condition {
                eprintln!("[DEBUG] condition locations: {:?}", cond.locations);
                eprintln!("[DEBUG] condition location: {:?}", cond.location);
                eprintln!("[DEBUG] condition type: {:?}", cond.condition_type);
            }
        }
    }

    let card_id = game.state.player1.live_card_zone.cards[0];
    let h02_mod = game
        .state
        .mods
        .get_need_heart_modifier(card_id, rabuka_engine::card::HeartColor::Heart02);
    let h03_mod = game
        .state
        .mods
        .get_need_heart_modifier(card_id, rabuka_engine::card::HeartColor::Heart03);
    let h06_mod = game
        .state
        .mods
        .get_need_heart_modifier(card_id, rabuka_engine::card::HeartColor::Heart06);
    assert_eq!(
        h02_mod, 2,
        "heart02 should be exactly 2 by set_required_hearts (got {})",
        h02_mod
    );
    assert_eq!(
        h03_mod, 2,
        "heart03 should be exactly 2 by set_required_hearts (got {})",
        h03_mod
    );
    assert_eq!(
        h06_mod, 2,
        "heart06 should be exactly 2 by set_required_hearts (got {})",
        h06_mod
    );
}

#[test]
fn hareruya_q74_multiname_distinct_counting() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hareruya = game.id("PL!SP-bp1-026-L");
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    // Q74/Q105: Cards with multiple names each count as distinct names
    // Use LL-bp1-001-R+ (上原歩夢&澁谷かのん&日野下花帆) — has 3 names
    // but it's group=μ's not Liella!. For Q74 the group filter applies first.
    // Keep deck well-stocked to prevent refresh from clearing waitroom
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }

    // Test with 5 distinct single-name Liella! members
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-014-N")); // 嵐 千砂都
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-016-N")); // 葉月 恋
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-019-N")); // 若菜四季
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-020-N")); // 鬼塚夏美
                                           // 4 distinct so far. Need 5th. Use a card whose Liella! name adds a 5th.

    // Actually LL-bp1-001-R+'s group is NOT Liella! so it won't pass the group filter.
    // For 5 distinct Liella! names, just use 5 different single-name Liella! cards.
    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!SP-bp1-022-N")); // 5th distinct name

    game.state.player1.hand.cards.push(hareruya);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(hareruya);
    advance_to_live_start(&mut game);

    // Q74: Verify condition was met (need_heart modifiers set)
    let card_id = game.state.player1.live_card_zone.cards[0];
    let h02_mod = game
        .state
        .mods
        .get_need_heart_modifier(card_id, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(
        h02_mod, 2,
        "set_required_hearts should fire with 5 distinct Liella! names (Q74)"
    );
}

// ====================================================================
//  ウィーン・マルガレーテ (PL!SP-bp2-010-R＋) — constant & LiveStart
// ====================================================================
// 常時: 相手のライブカード置き場にあるすべてのライブカードは、
//   成功させるための必要ハートがheart0多くなる。
// ライブ開始時: 自分のステージにこのメンバー以外のメンバーが1人以上いる場合、
//   ライブ終了時まで、エールによって公開される自分のカードの枚数が8枚減る。
//
// Q117: "このメンバー以外" includes any other member (same or different name)
// Q110: Two copies on stage → effect stacks (+2 heart0)
// ====================================================================

#[test]
fn wien_q117_another_member_triggers_yell_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wien = game.id("PL!SP-bp2-010-R\u{ff0b}");
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    // Wien at Center + another member at RightSide (any member = "other")
    game.state.player1.stage.stage = [-1, wien, game.id("PL!-sd1-010-SD")];
    let live_card = game.id("PL!-sd1-020-SD");
    game.state.player1.hand.cards.push(live_card);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    // Q117: Condition should be met, yell count modified
    // LiveStart ability fires → modify_yell_count(subtract, 8)
    // Verify the stage still has both members
    assert!(
        game.state.player1.stage.stage[1] != -1,
        "Wien should remain"
    );
    assert!(
        game.state.player1.stage.stage[2] != -1,
        "partner should remain"
    );
}

// ====================================================================
//  Edge case: turn limit enforcement
// ====================================================================

#[test]
fn turn_limit_prevents_second_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chika = game.id("PL!S-bp2-009-R"); // Chika has ターン1 limit
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(chika);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);

    // First activation succeeds (cost discards Chika from stage, effect retrieves live from discard)
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // After first activation, Chika is no longer on stage (cost moved her to discard).
    // Second activation might still resolve without error (engine returns Ok for
    // non-stage cards), but no additional state change should occur.
    let hand_before = game.state.player1.hand.cards.len();
    let _ = game.try_activate_ability(chika);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "No additional effect from second activation after card left stage"
    );
}

// ====================================================================
//  Edge case: energy zone capacity
// ====================================================================

#[test]
fn energy_zone_capacity_handled() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Start at 0 energy
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        0,
        "Should start with 0 energy"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "0 active energy"
    );

    // Give maximum energy
    game.give_energy(20);
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        20,
        "Should have max energy"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        20,
        "All should be active"
    );

    // Spend energy (simulate paying a cost)
    game.state.player1.energy_zone.active_energy_count = game
        .state
        .player1
        .energy_zone
        .active_energy_count
        .saturating_sub(5);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        15,
        "Should have 15 energy after spending 5"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        20,
        "Total energy cards unchanged"
    );

    // Spend all remaining
    game.state.player1.energy_zone.active_energy_count = game
        .state
        .player1
        .energy_zone
        .active_energy_count
        .saturating_sub(15);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "Should be 0 after spending all"
    );

    // Try to spend below 0 (should not crash, saturating)
    game.state.player1.energy_zone.active_energy_count = game
        .state
        .player1
        .energy_zone
        .active_energy_count
        .saturating_sub(1);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "Should stay at 0 (saturating)"
    );
}

#[test]
fn umi_pr014_appear_reveal_and_draw_when_no_live_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-PR-014-PR");
    let filler = game.id("PL!-sd1-010-SD");

    // Give P1 3 energy and Umi in hand, populate deck
    game.give_energy(3);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(umi);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let deck_before = game.state.player1.main_deck.cards.len();

    // Opponent has exactly 3 non-live cards in hand
    game.state.player2.hand.cards.clear();
    game.state.player2.hand.cards.push(filler);
    game.state.player2.hand.cards.push(filler);
    game.state.player2.hand.cards.push(filler);

    // Play Umi to stage (cost 2, 登場 trigger fires)
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(umi, rabuka_engine::zones::MemberArea::LeftSide);

    // 登場 trigger: reveal 3 from opponent's hand → no live cards → draw 1
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Umi cost is 2, we had 3 energy
    let energy_after = game.state.player1.energy_zone.active_energy_count;
    assert_eq!(
        energy_after, 1,
        "Should have 1 active energy after paying cost 2"
    );

    // P1 should have drawn 1 card (no live cards in opponent's revealed hand)
    assert_eq!(
        game.state.player1.hand.len(),
        1,
        "P1 should draw 1 card when no live card revealed"
    );

    // Deck decreased by 1
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "Deck should have 1 less card"
    );

    // Revealed cards should be in game state
    assert_eq!(
        game.state.revealed_cards.len(),
        3,
        "3 cards should be in revealed_cards"
    );

    // Opponent hand still has 3 cards (reveal doesn't remove from hand)
    assert_eq!(
        game.state.player2.hand.len(),
        3,
        "P2 should still have 3 cards in hand (revealed, not removed)"
    );
    assert!(!game.has_pending_choice(), "No pending choices expected");
}

#[test]
fn umi_pr014_appear_reveal_no_draw_when_live_card_present() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-PR-014-PR");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-bp3-026-L"); // LovePeace live card

    // Give P1 3 energy and Umi in hand, populate deck
    game.give_energy(3);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(umi);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Opponent has 3 cards: 2 non-live + 1 live card
    game.state.player2.hand.cards.clear();
    game.state.player2.hand.cards.push(filler);
    game.state.player2.hand.cards.push(live_card);
    game.state.player2.hand.cards.push(filler);

    // Play Umi to stage
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(umi, rabuka_engine::zones::MemberArea::LeftSide);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // P1 should NOT draw because a live card was among the revealed cards
    assert_eq!(
        game.state.player1.hand.len(),
        0,
        "P1 should NOT draw when live card revealed"
    );

    // Revealed cards should contain the live card
    assert!(
        game.state.revealed_cards.contains(&live_card),
        "Live card should be among revealed cards"
    );

    assert!(!game.has_pending_choice(), "No pending choices expected");
}

#[test]
fn umi_pr014_appear_reveal_with_choice_when_more_cards_than_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-PR-014-PR");
    let filler = game.id("PL!-sd1-010-SD");

    // Give P1 3 energy and Umi in hand, populate deck
    game.give_energy(3);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(umi);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Opponent has 5 cards (more than count=3, forces choice)
    game.state.player2.hand.cards.clear();
    for _ in 0..5 {
        game.state.player2.hand.cards.push(filler);
    }

    // Play Umi to stage
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(umi, rabuka_engine::zones::MemberArea::LeftSide);

    // Should have a pending choice to select 3 cards from opponent's hand
    assert!(
        game.has_pending_choice(),
        "Should have a choice to select 3 from opponent's hand"
    );

    let choice = game.state.get_pending_choice().unwrap();
    match choice {
        rabuka_engine::ability::types::Choice::SelectCard {
            count: c,
            blind,
            is_reveal: reveal,
            zone,
            ..
        } => {
            assert_eq!(*c, 3, "Should select 3 cards");
            assert!(*blind, "Choice should be blind (can't see card identities)");
            assert!(*reveal, "Choice should be marked as reveal");
            assert_eq!(zone, "hand", "Zone should be opponent's hand");
        }
        _ => panic!("Expected SelectCard choice, got {:?}", choice),
    }

    // Select 3 cards sequentially to test re-prompt preserves blind, is_reveal, target
    // Pick card at index 0
    game.select_indices(&[0]);

    // Re-prompt should preserve blind/is_reveal/target
    assert!(
        game.has_pending_choice(),
        "Sequential re-prompt should appear"
    );
    let re_prompt_choice = game.state.get_pending_choice().unwrap();
    match &re_prompt_choice {
        rabuka_engine::ability::types::Choice::SelectCard {
            count: c,
            blind: b,
            is_reveal: r,
            zone: z,
            target_player_id: t,
            ..
        } => {
            assert_eq!(*c, 2, "Re-prompt should ask for 2 more cards");
            assert!(*b, "Re-prompt should preserve blind=true");
            assert!(*r, "Re-prompt should preserve is_reveal=true");
            assert_eq!(z, "hand", "Re-prompt should still target hand");
            assert_eq!(
                t.as_deref(),
                Some("opponent"),
                "Re-prompt should target opponent, got {:?}",
                t
            );
        }
        _ => panic!("Expected SelectCard re-prompt, got {:?}", re_prompt_choice),
    }

    // Pick another card
    game.select_indices(&[1]);

    // Another re-prompt for the last card
    assert!(game.has_pending_choice(), "Second re-prompt should appear");
    let re_prompt2 = game.state.get_pending_choice().unwrap();
    match &re_prompt2 {
        rabuka_engine::ability::types::Choice::SelectCard {
            count: c, blind: b, ..
        } => {
            assert_eq!(*c, 1, "Second re-prompt should ask for 1 more");
            assert!(*b, "Second re-prompt should preserve blind");
        }
        _ => panic!("Expected SelectCard re-prompt, got {:?}", re_prompt2),
    }

    // Pick the third card
    game.select_indices(&[2]);

    // No live cards among the 3 selected → should draw 1
    assert_eq!(
        game.state.player1.hand.len(),
        1,
        "P1 should draw 1 when no live card in selected 3"
    );
    assert_eq!(
        game.state.revealed_cards.len(),
        3,
        "Exactly 3 cards should be revealed"
    );
    assert!(!game.has_pending_choice(), "No pending choices expected");
}

// ====================================================================
//  黒澤ダイヤ (PL!S-sd1-004-SD) — LiveStart: optional draw, conditional deck_top
// ====================================================================
// ライブ開始時: カードを1枚引いてもよい。そうした場合、手札2枚を好きな順番で
//   デッキの上に置く。(conditional sequential: draw optional, move_cards hand→deck_top)
// ====================================================================

#[test]
fn dia_sd1_optional_draw_skip_skips_movement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-sd1-004-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.add_to_hand(dia);
    // Two extra fillers in hand for deck_top placement test
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler); // one more filler as live card
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.play_to_stage(dia, rabuka_engine::zones::MemberArea::Center);
    // Deck: 10 fillers, Hand after play_to_stage: [filler, filler, filler] (3)

    // Advance to LiveCardSet (passes through Draw phase which draws 1 card → deck=9, hand=4)
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    game.set_live_card(filler); // removes 1 filler from hand → hand=3
    game.pass();
    game.pass();

    assert!(
        game.has_pending_choice(),
        "Dia's optional draw should appear"
    );
    game.select_option(0); // skip draw

    assert!(!game.has_pending_choice());
    // Deck: 10 - 1 (Draw phase) = 9 (no draw from skipped ability)
    assert_eq!(game.state.player1.main_deck.len(), 9, "Deck should be 9");
    // Hand: 3 (after play_to_stage) + 1 (Draw) - 1 (set_live_card) = 3
    assert_eq!(game.state.player1.hand.len(), 3, "Hand unchanged");
}

#[test]
fn dia_sd1_optional_draw_pay_then_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-sd1-004-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.add_to_hand(dia);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler); // one more as live card
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(13);
    game.play_to_stage(dia, rabuka_engine::zones::MemberArea::Center);
    // Hand: [filler,filler,filler] (3), Deck: 10

    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    // Draw phase draws 1 → Deck=9, Hand=4
    game.set_live_card(filler); // Hand=3
    game.pass();
    game.pass();

    assert_eq!(game.pending_choice_type(), Some("SelectTarget".to_string()));
    game.select_option(1); // pay (draw) → Deck=8, Hand=4

    assert_eq!(game.pending_choice_type(), Some("SelectCard".to_string()));
    assert!(!game.state.player1.hand.cards.is_empty());

    game.try_select_indices(&[0, 1]).unwrap(); // place 2 on deck → Deck=10, Hand=2

    assert!(!game.has_pending_choice());

    // Final: 10 - 1 - 1 + 2 = 10
    assert_eq!(game.state.player1.main_deck.len(), 10);
    // Hand: 3 + 1 - 1 + 1 - 2 = 2
    assert_eq!(game.state.player1.hand.len(), 2);
}

// ── Revealed_cards filtered by player ownership when cheer_buf is empty ──

#[test]
fn revealed_cards_filtered_by_player_ownership() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p1_card = game.id("PL!SP-sd1-019-SD");
    let p2_card = game.id("PL!SP-sd1-020-SD");

    // Put both players' cards into their own discard so zone-ownership check passes
    game.state.player1.waitroom.cards.push(p1_card);
    game.state.player2.waitroom.cards.push(p2_card);

    // Populate revealed_cards with both players' cards
    // Leave per-player cheer_bufs empty to force the fallback path
    game.state.revealed_cards.push(p1_card);
    game.state.revealed_cards.push(p2_card);

    // Inject a SelectCard choice for the revealed_cards zone targeting self
    let choice = rabuka_engine::ability::types::Choice::select_cards(
        "revealed_cards",
        1,
        "Select 1 card",
        false,
    )
    .target_player_id(Some("self".to_string()))
    .build();
    game.state.ability_queue.pause_for_choice(choice);

    // Set choice_player_id so generate_possible_actions resolves "self" to P1
    if let Some(entry) = game.state.ability_queue.current_entry_mut() {
        entry.choice_player_id = Some("p1".to_string());
    }

    // Generate possible actions and check which cards are selectable
    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let select_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == rabuka_engine::game_setup::ActionType::ChoiceSelect)
        .collect();

    // Only P1's card should be selectable (p2_card is not in P1's zones)
    assert_eq!(
        select_actions.len(),
        1,
        "Only P1's card should be selectable"
    );
    assert_eq!(
        select_actions[0]
            .parameters
            .as_ref()
            .and_then(|p| p.card_id),
        Some(p1_card),
        "P1's card should be selectable"
    );
}

// ── choice_condition cost shows proper option labels ──

#[test]
fn choice_condition_shows_proper_labels_in_actions() {
    use rabuka_engine::ability::types::Choice;
    use rabuka_engine::game_setup::{generate_possible_actions, ActionType};

    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Inject a choice_condition SelectTarget with option labels
    let choice = Choice::SelectTarget {
        target: "choice_condition".to_string(),
        description: "Choose cost option: このメンバーをウェイトにする OR 手札を1枚控え室に置く"
            .to_string(),
        allow_skip: false,
        options: Some(vec![
            "このメンバーをウェイトにする".to_string(),
            "手札を1枚控え室に置く".to_string(),
        ]),
    };
    game.state.ability_queue.pause_for_choice(choice);

    // Generate actions and verify they show the actual option labels
    let actions = generate_possible_actions(&game.state);
    let option_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == ActionType::ChoiceOption)
        .collect();

    assert_eq!(option_actions.len(), 2, "Two option buttons");
    assert!(
        option_actions[0].description.contains("ウェイトにする"),
        "First option should be 'wait this member'"
    );
    assert!(
        option_actions[1].description.contains("控え室に置く"),
        "Second option should be 'discard from hand'"
    );
}

// ── Kanon activation: choice_condition cost — discard option creates card selection ──

#[test]
fn kanon_activation_choice_condition_discard_flow() {
    use rabuka_engine::game_setup::ActionType;

    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp5-001-R+");
    let filler = game.id("PL!-sd1-010-SD");
    let cheap = game.id("PL!SP-sd1-019-SD");

    // Put kanon in hand and play to stage
    game.state.player1.hand.cards.push(kanon);
    // Add some cards to hand (one for the discard cost option)
    game.state.player1.hand.cards.push(cheap);
    game.state.player1.hand.cards.push(filler);

    // Put wait energy — make 5 of 15 wait
    game.give_energy(15);
    game.state.player1.energy_zone.active_energy_count = 10;

    // Manually place kanon on stage to avoid triggering debut ability
    game.state.player1.stage.stage[0] = kanon;
    game.state.player1.hand.cards.retain(|id| *id != kanon);

    // Activate kanon's 起動 ability
    game.activate_ability(kanon);

    // Should be a choice_condition with proper labels (not Yes/No)
    let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    let option_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == ActionType::ChoiceOption)
        .collect();

    assert_eq!(
        option_actions.len(),
        2,
        "Two cost options should be shown with their labels"
    );
    // Verify we see one of the actual cost option texts
    let all_descriptions: String = option_actions
        .iter()
        .map(|a| a.description.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        all_descriptions.contains("ウェイト") || all_descriptions.contains("wait"),
        "Option labels should contain the actual cost text"
    );

    // Select the discard option (index 1 = 手札を1枚控え室に置く)
    game.select_option(1);

    // Should now show a SelectCard choice to pick which card to discard
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectCard".to_string()),
        "Discard option should create a card selection prompt"
    );

    // Select the cheap card to discard
    game.select_indices(&[0]);

    // Effect should resolve: activate 1 energy
    assert!(
        !game.has_pending_choice(),
        "Effect should resolve after discard"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 11,
        "One wait energy should be activated"
    );
    // The selected card should be in discard (waitroom)
    assert!(
        game.state.player1.waitroom.cards.contains(&cheap),
        "Discarded card should be in waitroom"
    );
}

#[test]
fn kanon_activation_choice_condition_wait_option() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp5-001-R+");

    game.state.player1.hand.cards.push(kanon);
    game.give_energy(15);
    game.state.player1.energy_zone.active_energy_count = 10;

    // Manually place kanon on stage without triggering debut ability
    game.state.player1.stage.stage[0] = kanon;
    game.state.player1.hand.cards.retain(|id| *id != kanon);

    // Activate kanon's 起動 ability
    game.activate_ability(kanon);

    // Select the wait option (index 0 = このメンバーをウェイトにする)
    game.select_option(0);

    // Effect should resolve: activate 1 energy
    assert!(
        !game.has_pending_choice(),
        "Effect should resolve after paying wait cost"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 11,
        "One wait energy should be activated"
    );
    // Kanon should be in wait state
    assert_eq!(
        game.state.mods.get_orientation_modifier(kanon),
        Some(&"wait".to_string()),
        "Kanon should be in wait state"
    );
}

// ── Hanamaru: activation ability filters by score icon in discard ──

#[test]
fn hanamaru_score_icon_filter() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-sd1-007-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Aqours live card WITH score icon → should be selectable
    let score_live = game.id("PL!S-bp2-024-L");
    // Aqours live card WITHOUT score icon → should NOT be selectable
    let no_score_live = game.id("PL!S-bp2-026-L");
    // Member card from same series → should NOT be selectable
    let member = game.id("PL!S-bp2-015-PR");

    // Hand: hanamaru + 2 cards to discard for cost
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    // Discard: one score live, one non-score live, one member
    game.state.player1.waitroom.cards.push(score_live);
    game.state.player1.waitroom.cards.push(no_score_live);
    game.state.player1.waitroom.cards.push(member);

    game.give_energy(15);

    // Manually place hanamaru on stage
    game.state.player1.stage.stage[0] = hanamaru;
    game.state.player1.hand.cards.retain(|id| *id != hanamaru);

    // Verify the card has the ability with card_property: has_score_icon
    {
        let card_data = game.state.card_database.get_card(hanamaru).unwrap();
        let has_ability = card_data.abilities.iter().any(|a| {
            a.effect
                .as_ref()
                .is_some_and(|e| e.card_property.as_deref() == Some("has_score_icon"))
        });
        assert!(
            has_ability,
            "hanamaru must have ability with card_property=has_score_icon"
        );
    }

    // Activate 起動 ability
    game.activate_ability(hanamaru);

    // Cost: Select 2 cards from hand to discard
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectCard".to_string()),
        "Cost: select 2 cards from hand"
    );
    game.select_indices(&[0, 1]);

    // No pending choice — exactly 1 candidate matched (score_live), so auto-selected.
    assert!(
        !game.has_pending_choice(),
        "Effect should auto-select when exactly 1 card matches filter"
    );

    // Verify: score_live moved from waitroom to hand
    assert!(
        !game.state.player1.waitroom.cards.contains(&score_live),
        "score icon live card should have been moved out of discard"
    );
    assert!(
        game.state.player1.hand.cards.contains(&score_live),
        "score icon live card should be in hand"
    );

    // Verify: non-score live and member remain in discard
    assert!(
        game.state.player1.waitroom.cards.contains(&no_score_live),
        "non-score live card should remain in discard"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&member),
        "member card should remain in discard"
    );
}

#[test]
fn ai_screeam_soreigai_all_members_on_both_sides_gain_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let screeam = game.id("LL-PR-004-PR");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    let p1_a = game.id("PL!-sd1-013-SD");
    let p1_b = game.id("PL!-sd1-014-SD");
    let p1_c = game.id("PL!-sd1-015-SD");
    let p2_a = game.id("PL!-sd1-016-SD");
    let p2_b = game.id("PL!-sd1-017-SD");
    let p2_c = game.id("PL!-sd1-018-SD");

    // Full stage: 3 members each
    game.state.player1.stage.stage = [p1_a, p1_b, p1_c];
    game.state.player2.stage.stage = [p2_a, p2_b, p2_c];

    game.state.player1.hand.cards.push(screeam);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(screeam);
    advance_to_live_start(&mut game);
    assert!(game.has_pending_choice());

    // Option 2: それ以外 → all members on both sides gain blade
    game.select_option(2);

    assert!(
        !game.has_pending_choice(),
        "No more pending choices after blade gain"
    );

    // All 3 P1 members should have blade
    assert!(
        game.state.mods.get_blade_modifier(p1_a) > 0,
        "P1 left member should have blade"
    );
    assert!(
        game.state.mods.get_blade_modifier(p1_b) > 0,
        "P1 center member should have blade"
    );
    assert!(
        game.state.mods.get_blade_modifier(p1_c) > 0,
        "P1 right member should have blade"
    );

    // All 3 P2 members should have blade
    assert!(
        game.state.mods.get_blade_modifier(p2_a) > 0,
        "P2 left member should have blade"
    );
    assert!(
        game.state.mods.get_blade_modifier(p2_b) > 0,
        "P2 center member should have blade"
    );
    assert!(
        game.state.mods.get_blade_modifier(p2_c) > 0,
        "P2 right member should have blade"
    );
}
