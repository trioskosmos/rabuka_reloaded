/// Audit tests for performance snapshot correctness.
/// Uses only abilityless cards to minimize variables.
/// DUMPS all snapshot values so we can verify them against rules.txt.
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn setup_game(game: &mut TestGame, p1_stage: [i16; 3], filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.success_live_card_zone.cards.clear();
    game.state.player1.energy_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player2.hand.cards.clear();
    game.state.player2.waitroom.cards.clear();
    game.state.player2.success_live_card_zone.cards.clear();
    game.state.player2.energy_deck.cards.clear();

    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = p1_stage;
    game.state.player2.stage.stage = [-1, filler, -1];
}

fn run_to_end(game: &mut TestGame, live_cards: &[i16]) {
    advance_to_live_card_set_p1(game);
    for &lc in live_cards {
        game.set_live_card(lc);
    }
    game.pass(); // LiveCardSetFirstAttacker -> LiveCardSetSecondAttacker
    game.pass(); // LiveCardSetSecondAttacker -> FirstAttackerPerformance

    // Loop until we return to Active phase, resolving any choices created by LiveStart/yell/LiveSuccess.
    // The ONLY legal prompt in these abilityless-card flows is START:DASH!!'s
    // optional looked_at arrange (SelectCard, empty = legal skip); anything
    // else must fail loudly instead of being blindly skipped.
    for _ in 0..20 {
        if game.state.current_phase == rabuka_engine::game_state::Phase::Active
            && !game.has_pending_choice()
        {
            break;
        }
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[]),
            Some(other) => panic!(
                "unexpected prompt during run_to_end (expected SelectCard arrange or none), got {:?}",
                other
            ),
            None => {}
        }
        if !game.has_pending_choice() {
            game.pass();
        }
    }
}

fn dump_perf(game: &mut TestGame, label: &str) {
    eprintln!("\n===== {} =====", label);
    for snap in &game.state.performance_snapshots {
        if snap.player_id != "p1" {
            continue;
        }
        eprintln!("  total_hearts={:?}", snap.total_hearts);
        eprintln!(
            "  total_score={} success={}",
            snap.total_score, snap.success
        );
        eprintln!(
            "  note_icons={} yell_count={}",
            snap.note_icons, snap.yell_count
        );
        eprintln!("  surplus_hearts={:?}", snap.surplus_hearts);
        eprintln!(
            "  base_score_total={} card_bonus_total={}",
            snap.base_score_total, snap.card_bonus_total
        );
        for (i, l) in snap.lives.iter().enumerate() {
            let req_total: u8 = l.required.iter().sum();
            let filled_total: u8 = l.filled.iter().sum();
            eprintln!(
                "  live[{}]: passed={} score={} base_score={} req_total={} filled_total={}",
                i, l.passed, l.score, l.base_score, req_total, filled_total
            );
            eprintln!("    required={:?}", l.required);
            eprintln!("    filled={:?}", l.filled);
            eprintln!("    spare={:?}", l.spare);
        }
        for (i, a) in snap.breakdown.allocations.iter().enumerate() {
            eprintln!(
                "  alloc[{}]: target={} color={} amt={} phase={:?}",
                i, a.target_idx, a.color, a.amount, a.phase
            );
        }
    }
    eprintln!("===== END {} =====\n", label);
}

// ──────────────────────────────────────────────────────────────────
// TEST 1: Sufficient hearts → card PASS
//
// Live: PL!-sd1-019-SD (START:DASH!!) — score=1, needs h01=1 h03=1 h06=1
// Members: PL!-sd1-001-SD (穂乃果) — bh={h01=1,h03=2,h06=1} blade=3
// Filler: PL!-sd1-010-SD — bh={h01=1,h03=1} bs={b_h03=1} blade=1
//
// Stage base: h01=1, h03=2, h06=1
// Blades: 3 → 3 yell cards × b_h03=1 → h03+=3
// Total: h01=1, h03=5, h06=1 = 7
// Need: h01=1, h03=1, h06=1 = 3 → PASS
// ──────────────────────────────────────────────────────────────────
#[test]
fn audit_sufficient_hearts_passes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD");
    let member = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    setup_game(&mut game, [-1, member, -1], filler);
    game.state.player1.hand.cards.push(live);

    run_to_end(&mut game, &[live]);
    dump_perf(&mut game, "sufficient_hearts");

    let perf = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .unwrap();
    let lc = &perf.lives[0];

    assert!(lc.passed, "Card should PASS: 7 hearts >= 3 needed");
    assert!(perf.total_score > 0, "score > 0 on pass");
    assert!(perf.success, "success on pass");
    eprintln!(
        "VERIFY: base_score_total={} card_bonus_total={}",
        perf.base_score_total, perf.card_bonus_total
    );
}

// ──────────────────────────────────────────────────────────────────
// TEST 2: Two live cards, second fails → Rule 8.3.16
//
// Live A: PL!N-sd1-025-SD — needs h0=4 (any 4 hearts)
// Live B: same
// Member: PL!-sd1-001-SD — bh={h01=1,h03=2,h06=1} blade=3
// Filler: PL!-sd1-010-SD — bs={b_h03=1}
//
// Total: h01=1, h03=5, h06=1 = 7
// Live A gets h03=4 → PASS, leaves h01=1,h03=1,h06=1=3
// Live B needs h0=4, only 3 left → FAIL
// Rule 8.3.16: when ANY fails, success=false
// ──────────────────────────────────────────────────────────────────
#[test]
fn audit_two_cards_second_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_a = game.id("PL!N-sd1-025-SD");
    let live_b = game.new_id("PL!N-sd1-025-SD");
    let member = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    setup_game(&mut game, [-1, member, -1], filler);
    game.state.player1.hand.cards.push(live_a);
    game.state.player1.hand.cards.push(live_b);

    run_to_end(&mut game, &[live_a, live_b]);
    dump_perf(&mut game, "two_cards");

    let perf = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .unwrap();

    eprintln!(
        "VERIFY: live[0] passed={} live[1] passed={}",
        perf.lives[0].passed, perf.lives[1].passed
    );
    assert!(
        !perf.lives[1].passed,
        "Second card should FAIL: only 3 hearts left, needs 4"
    );
    assert!(!perf.success, "success=false when any card fails");
}

// ──────────────────────────────────────────────────────────────────
// TEST 3: Allocations match filled array
// ──────────────────────────────────────────────────────────────────
#[test]
fn audit_allocations_match_filled() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!N-sd1-025-SD");
    let member = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    setup_game(&mut game, [-1, member, -1], filler);
    game.state.player1.hand.cards.push(live);

    run_to_end(&mut game, &[live]);
    dump_perf(&mut game, "alloc_match");

    let perf = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .unwrap();

    let mut alloc_filled = [0u8; 8];
    for a in &perf.breakdown.allocations {
        if a.target_idx == 0 {
            alloc_filled[a.color as usize] += a.amount;
        }
    }
    let lc = &perf.lives[0];
    eprintln!(
        "VERIFY: alloc_filled={:?} snapshot_filled={:?}",
        alloc_filled, lc.filled
    );
    assert_eq!(
        alloc_filled, lc.filled,
        "filled from allocations must match snapshot filled"
    );
}

// ──────────────────────────────────────────────────────────────────
// TEST 4: Additive requirement modifier stacks on top of base
//
// Live: PL!-sd1-019-SD — base: h01=1, h03=1, h06=1
// Modifier: heart03 += 2 (additive, no set)
// Effective: h01=1, h03=3, h06=1 = 5 needed
//
// Member: PL!-sd1-001-SD — bh={h01=1, h03=2, h06=1} blade=3
// Filler: PL!-sd1-010-SD — bs={b_h03=1}
// Total: h01=1, h03=5, h06=1
//
// With modifier: h03 requirement = 3, only 5 total but h01=1, h03=5, h06=1.
// h03 covers 3 of h03_req → PASS.
// ──────────────────────────────────────────────────────────────────
#[test]
fn audit_additive_modifier_stacks_on_base() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD");
    let member = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    setup_game(&mut game, [-1, member, -1], filler);
    game.state.player1.hand.cards.push(live);

    // Additive +2 to h03 requirement
    {
        use rabuka_engine::card::HeartColor;
        use rabuka_engine::core::game_modifiers::ModifierEntry;
        game.state
            .mods
            .need_heart_modifiers
            .entry(live)
            .or_default()
            .insert(
                HeartColor::Heart03,
                ModifierEntry {
                    set: 0,
                    additive: 2,
                },
            );
    }

    run_to_end(&mut game, &[live]);
    dump_perf(&mut game, "additive_modifier");

    let perf = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .unwrap();
    let lc = &perf.lives[0];

    // Base h03=1 + additive 2 = 3 required for h03
    // Other colors unchanged from base: h01=1, h06=1
    eprintln!(
        "VERIFY: required={:?} filled={:?} passed={}",
        lc.required, lc.filled, lc.passed
    );
    assert_eq!(
        lc.required[3], 3,
        "h03 required should be base(1) + additive(2) = 3"
    );
    assert_eq!(lc.required[1], 1, "h01 required unchanged from base");
    assert_eq!(lc.required[6], 1, "h06 required unchanged from base");
    // h03 provided = 5, so h03_req(3) is satisfied → PASS
    assert!(
        lc.passed,
        "Card should PASS: h03 filled(5) >= h03_req(3) and others met"
    );
}

// ──────────────────────────────────────────────────────────────────
// TEST 5: Additive modifier making total requirement too high → FAIL
//
// Live: PL!-sd1-019-SD — base: h01=1, h03=1, h06=1
// Modifier: heart01 += 5 (additive)
// Effective: h01=6 — impossible with stage providing only 1 h01
// ──────────────────────────────────────────────────────────────────
#[test]
fn audit_additive_modifier_can_cause_failure() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD");
    let member = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    setup_game(&mut game, [-1, member, -1], filler);
    game.state.player1.hand.cards.push(live);

    // Additive +5 to h01 requirement — makes it impossible to satisfy
    {
        use rabuka_engine::card::HeartColor;
        use rabuka_engine::core::game_modifiers::ModifierEntry;
        game.state
            .mods
            .need_heart_modifiers
            .entry(live)
            .or_default()
            .insert(
                HeartColor::Heart01,
                ModifierEntry {
                    set: 0,
                    additive: 5,
                },
            );
    }

    run_to_end(&mut game, &[live]);
    dump_perf(&mut game, "additive_causes_fail");

    let perf = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .unwrap();
    let lc = &perf.lives[0];

    eprintln!(
        "VERIFY: required={:?} filled={:?} passed={}",
        lc.required, lc.filled, lc.passed
    );
    assert_eq!(
        lc.required[1], 6,
        "h01 required should be base(1) + additive(5) = 6"
    );
    assert_eq!(lc.required[3], 1, "h03 required unchanged");
    assert_eq!(lc.required[6], 1, "h06 required unchanged");
    assert!(
        !lc.passed,
        "Card should FAIL: h01 required=6, only 1 available"
    );
    assert!(!perf.success, "success=false when card fails");
}

// ──────────────────────────────────────────────────────────────────
// TEST 6: set=0 modifier zeroes a color requirement → card easier to pass
//
// Live: PL!-sd1-019-SD — base: h01=1, h03=1, h06=1
// Modifier: heart06 set=0 → requirement for h06 becomes 0 (no longer needed)
// Stage: only h01=1, h03=2 (no h06)
//
// Without modifier: would fail (missing h06).
// With set=0: h06 required = 0, so h01+h03 is enough → PASS.
// ──────────────────────────────────────────────────────────────────
#[test]
fn audit_set_zero_removes_requirement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD"); // needs h01=1, h03=1, h06=1
    let filler = game.id("PL!-sd1-010-SD"); // bh={h01=1, h03=1}

    // Stage has filler providing h01=1, h03=1 but NO h06
    setup_game(&mut game, [-1, filler, -1], filler);
    game.state.player1.hand.cards.push(live);

    // set h06 requirement to 0 — h06 no longer needed
    {
        use rabuka_engine::card::HeartColor;
        use rabuka_engine::core::game_modifiers::ModifierEntry;
        game.state
            .mods
            .need_heart_modifiers
            .entry(live)
            .or_default()
            .insert(
                HeartColor::Heart06,
                ModifierEntry {
                    set: 0,
                    additive: -1,
                },
            );
    }

    run_to_end(&mut game, &[live]);
    dump_perf(&mut game, "set_zero_removes_req");

    let perf = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .unwrap();
    let lc = &perf.lives[0];

    eprintln!(
        "VERIFY: required={:?} filled={:?} passed={}",
        lc.required, lc.filled, lc.passed
    );
    // h01 and h03 still required, h06 reduced to 0
    assert_eq!(lc.required[1], 1, "h01 still required");
    assert_eq!(lc.required[3], 1, "h03 still required");
    assert_eq!(
        lc.required[6], 0,
        "h06 requirement should be 0 after additive -1"
    );
    assert!(lc.passed, "Card should PASS with h06 requirement removed");
    assert!(perf.success, "success=true when all requirements met");
}

// ──────────────────────────────────────────────────────────────────
// TEST 7: Card with no need_heart field always passes
//
// Use a member/energy card (no need_heart) placed in live zone.
// Even with 0 stage hearts, it should always report passed=true.
// ──────────────────────────────────────────────────────────────────
#[test]
fn audit_no_need_heart_always_passes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // PL!N-sd1-025-SD has need_heart h0=4 — use a card without any need_heart.
    // PL!-sd1-001-SD (穂乃果 member) has no need_heart
    let live_no_req = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Zero stage — no hearts at all
    setup_game(&mut game, [-1, -1, -1], filler);
    game.state.player1.hand.cards.push(live_no_req);

    run_to_end(&mut game, &[live_no_req]);
    dump_perf(&mut game, "no_need_heart");

    let perf = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .unwrap();

    eprintln!(
        "VERIFY: lives={:?}",
        perf.lives.iter().map(|l| l.passed).collect::<Vec<_>>()
    );
    if let Some(lc) = perf.lives.first() {
        assert!(
            lc.passed,
            "Card without need_heart must always pass regardless of stage"
        );
    }
    // Score may be >0 (card score) even without requirements
    eprintln!(
        "VERIFY: total_score={} success={}",
        perf.total_score, perf.success
    );
}

// ──────────────────────────────────────────────────────────────────
// TEST 8: Both players fail live → neither wins, neither gets success zone card
// ──────────────────────────────────────────────────────────────────
#[test]
fn audit_both_players_fail_no_winner() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD"); // needs h01=1, h03=1, h06=1
    let live2 = game.new_id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Both players have no stage cards → both fail
    game.state.player1.main_deck.cards.clear();
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.success_live_card_zone.cards.clear();
    game.state.player1.energy_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player2.hand.cards.clear();
    game.state.player2.waitroom.cards.clear();
    game.state.player2.success_live_card_zone.cards.clear();
    game.state.player2.energy_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player2.stage.stage = [-1, -1, -1];

    game.state.player1.hand.cards.push(live);
    game.state.player2.hand.cards.push(live2);

    // Advance to live card set
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live);
    game.pass(); // → p2 set
    game.state.player2.hand.cards.push(live2);
    game.set_live_card(live2);

    // Run to Active — same strict prompt contract as run_to_end: only the
    // START:DASH!! looked_at arrange may appear.
    for _ in 0..20 {
        if game.state.current_phase == rabuka_engine::game_state::Phase::Active
            && !game.has_pending_choice()
        {
            break;
        }
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[]),
            Some(other) => panic!(
                "unexpected prompt running to Active (expected SelectCard arrange or none), got {:?}",
                other
            ),
            None => {}
        }
        if !game.has_pending_choice() {
            game.pass();
        }
    }

    eprintln!(
        "VERIFY: p1_success_zone={} p2_success_zone={}",
        game.state.player1.success_live_card_zone.cards.len(),
        game.state.player2.success_live_card_zone.cards.len()
    );

    assert_eq!(
        game.state.player1.success_live_card_zone.cards.len(),
        0,
        "P1 failed live should NOT get a success zone card"
    );
    assert_eq!(
        game.state.player2.success_live_card_zone.cards.len(),
        0,
        "P2 failed live should NOT get a success zone card"
    );

    let p1_snap = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1");
    let p2_snap = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p2");
    if let Some(s) = p1_snap {
        assert!(!s.success, "P1 success should be false");
    }
    if let Some(s) = p2_snap {
        assert!(!s.success, "P2 success should be false");
    }
}

// ──────────────────────────────────────────────────────────────────
// TEST 9: THE CORE REGRESSION — zero stage, live card with requirements → FAIL
//
// This is the exact bug: "even with no cards on stage every live card passes."
// Live: PL!-sd1-019-SD — needs h01=1, h03=1, h06=1
// Stage: [-1, -1, -1] — zero hearts
//
// Expected: passed=false, success=false, card in waitroom, NOT success zone.
//
// Dumps every relevant field so any future regression is immediately visible.
// ──────────────────────────────────────────────────────────────────
#[test]
fn audit_zero_stage_live_card_must_fail() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD"); // needs h01=1, h03=1, h06=1
    let filler = game.id("PL!-sd1-010-SD");

    // P1 has zero stage cards — NO hearts whatsoever
    setup_game(&mut game, [-1, -1, -1], filler);
    game.state.player1.hand.cards.push(live);

    run_to_end(&mut game, &[live]);
    dump_perf(&mut game, "zero_stage_must_fail");

    // Dump card locations
    eprintln!(
        "VERIFY card locations: waitroom={} success_zone={}",
        game.state.player1.waitroom.cards.contains(&live),
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&live),
    );
    eprintln!(
        "VERIFY live_card_zone after: {:?}",
        game.state.player1.live_card_zone.cards
    );

    // Card must NOT be in success zone
    assert!(
        !game
            .state
            .player1
            .success_live_card_zone
            .cards
            .contains(&live),
        "FAIL: card landed in success zone despite zero stage hearts"
    );

    // Card must be in waitroom
    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "FAIL: card not found in waitroom after failing requirements"
    );

    let perf = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .expect("P1 must have a snapshot");

    eprintln!(
        "VERIFY snapshot: success={} total_score={} lives_len={}",
        perf.success,
        perf.total_score,
        perf.lives.len()
    );

    // Snapshot must reflect failure
    assert!(
        !perf.success,
        "FAIL: snapshot.success=true with zero stage hearts"
    );
    assert_eq!(
        perf.total_score, 0,
        "FAIL: total_score > 0 on a failed live"
    );

    // If lives is populated, every entry must show passed=false and required > 0
    for (i, lc) in perf.lives.iter().enumerate() {
        let req_total: u8 = lc.required.iter().sum();
        let filled_total: u8 = lc.filled.iter().sum();
        eprintln!(
            "VERIFY live[{}]: passed={} required={:?} (total={}) filled={:?} (total={})",
            i, lc.passed, lc.required, req_total, lc.filled, filled_total
        );
        assert!(!lc.passed, "FAIL: live[{}] passed=true with zero hearts", i);
        assert!(
            req_total > 0,
            "FAIL: live[{}] required_total=0 — requirements not loaded from card",
            i
        );
        assert_eq!(
            filled_total, 0,
            "FAIL: live[{}] filled_total={} with zero stage hearts",
            i, filled_total
        );
    }
}

// ──────────────────────────────────────────────────────────────────
// TEST 10: P1 passes, P2 fails → P1 wins success zone card, P2 waitroom
//
// P1: member on stage providing all needed hearts → PASS
// P2: empty stage → FAIL
// Expected: P1 gets success zone card, P2 gets waitroom
// ──────────────────────────────────────────────────────────────────
#[test]
fn audit_p1_passes_p2_fails_p1_wins() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD");
    let live2 = game.new_id("PL!-sd1-019-SD");
    let member = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // P1 has member → provides h01, h03, h06 → PASS
    // P2 has empty stage → FAIL
    game.state.player1.main_deck.cards.clear();
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.success_live_card_zone.cards.clear();
    game.state.player1.energy_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player2.hand.cards.clear();
    game.state.player2.waitroom.cards.clear();
    game.state.player2.success_live_card_zone.cards.clear();
    game.state.player2.energy_deck.cards.clear();

    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [-1, member, -1];
    game.state.player2.stage.stage = [-1, -1, -1]; // zero stage for P2

    game.state.player1.hand.cards.push(live);
    game.state.player2.hand.cards.push(live2);

    // Advance to live card set
    for _ in 0..5 {
        game.pass();
    }

    game.set_live_card(live);
    game.pass(); // → p2 set
    game.state.player2.hand.cards.push(live2);
    game.set_live_card(live2);

    // Run to Active — same strict prompt contract as run_to_end: only the
    // START:DASH!! looked_at arrange may appear.
    for _ in 0..20 {
        if game.state.current_phase == rabuka_engine::game_state::Phase::Active
            && !game.has_pending_choice()
        {
            break;
        }
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[]),
            Some(other) => panic!(
                "unexpected prompt running to Active (expected SelectCard arrange or none), got {:?}",
                other
            ),
            None => {}
        }
        if !game.has_pending_choice() {
            game.pass();
        }
    }

    eprintln!(
        "VERIFY: p1_success={} p2_success={}",
        game.state.player1.success_live_card_zone.cards.len(),
        game.state.player2.success_live_card_zone.cards.len()
    );
    eprintln!(
        "VERIFY: p1_waitroom_has_live={} p2_waitroom_has_live2={}",
        game.state.player1.waitroom.cards.contains(&live),
        game.state.player2.waitroom.cards.contains(&live2)
    );

    let p1 = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1");
    let p2 = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p2");
    if let Some(s) = p1 {
        eprintln!(
            "VERIFY P1 snap: success={} total_score={}",
            s.success, s.total_score
        );
        for (i, l) in s.lives.iter().enumerate() {
            eprintln!(
                "  live[{}]: passed={} required={:?} filled={:?}",
                i, l.passed, l.required, l.filled
            );
        }
    }
    if let Some(s) = p2 {
        eprintln!(
            "VERIFY P2 snap: success={} total_score={}",
            s.success, s.total_score
        );
        for (i, l) in s.lives.iter().enumerate() {
            eprintln!(
                "  live[{}]: passed={} required={:?} filled={:?}",
                i, l.passed, l.required, l.filled
            );
        }
    }

    assert_eq!(
        game.state.player1.success_live_card_zone.cards.len(),
        1,
        "FAIL: P1 (passed requirements) should have card in success zone"
    );
    assert_eq!(
        game.state.player2.success_live_card_zone.cards.len(),
        0,
        "FAIL: P2 (failed requirements) should NOT have card in success zone"
    );
    assert!(
        game.state.player2.waitroom.cards.contains(&live2),
        "FAIL: P2 failed live card should be in waitroom"
    );
}
