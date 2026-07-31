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
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    game.pass();
    game.pass();
    game.pass();
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
            alloc_filled[a.color] += a.amount;
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
