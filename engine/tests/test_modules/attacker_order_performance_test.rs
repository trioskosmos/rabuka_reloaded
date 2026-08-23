//! Regression test for the performer-selection bug in
//! `execute_performance_phase`: `check_live_success` used to select the player
//! by position (`is_first → player1`) while every other step in the function
//! (performer_id, yell, cheer assignment) is attacker-aware. Rule 8.4.13 swaps
//! the first attacker whenever exactly one player moves a card to the success
//! zone, so P2 leads lives routinely in real games — and when P2 led, P1's
//! zones were processed with P2's yell data.
//!
//! Fails on pre-fix builds: the P2 snapshot came back with no finalized lives.
use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD"; // blade=1, hearts {heart01:1,heart03:1}
const ERI: &str = "PL!-sd1-002-SD"; // blade=1, heart {heart06:1}
const DREAM_BELIEVERS: &str = "PL!HS-bp1-019-L"; // need {heart0:4}, score 1

/// P2 is the first attacker with a full board; P1 has nothing.
/// Deck layout for P2 applied after set_live_card: [sacrificial, F, F] so the
/// 3-blade yell reveals two fillers after the LiveCardSet refill eats index 0.
#[test]
fn p2_first_attacker_performance_finalizes_p2_snapshot() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let f1 = game.id(FILLER);
    let f2 = game.new_id(FILLER);
    let eri = game.id(ERI);
    let fill = game.id(FILLER);

    // P2 leads this turn (rule 8.4.13 alternation).
    game.state.player1.is_first_attacker = false;
    game.state.player2.is_first_attacker = true;

    // P2 stage: heart01+heart03+heart06 coverage, 3 blades total.
    game.state.player2.stage.stage = [f1, f2, eri];
    for _ in 0..25 {
        game.state.player2.main_deck.cards.push(fill);
        game.state.player1.main_deck.cards.push(fill);
    }

    let live = game.new_id(DREAM_BELIEVERS); // note-only card, no prompts
    game.state.player2.hand.cards.push(live);

    // Advance to P2's LiveCardSet window and set the live card.
    assert_eq!(game.state.current_phase.to_string(), "Main");
    for _ in 0..5 {
        game.pass();
    }
    assert!(
        game.state.current_phase.to_string().contains("LiveCardSet"),
        "phase={}",
        game.state.current_phase
    );
    game.set_live_card(live);

    // Layout AFTER set_live_card: index 0 sacrificed to the refill draw,
    // then the yell reveals two plain fillers.
    game.state.player2.main_deck.cards.insert(0, fill);
    let sac = game.new_id(FILLER);
    game.state.player2.main_deck.cards.insert(0, sac);

    // Drive performance + victory to the next Active, answering anything.
    let mut saw_result = false;
    for _ in 0..30 {
        if game.has_pending_choice() {
            game.select_indices(&[]);
            continue;
        }
        let phase = game.state.current_phase.to_string();
        if phase.contains("Live Result") {
            saw_result = true;
        }
        if saw_result && phase == "Active" {
            break;
        }
        game.pass();
    }
    assert!(saw_result, "never reached Live Result");

    let p2_snap = game
        .state
        .performance_snapshots
        .iter()
        .rev()
        .find(|s| s.player_id == "p2")
        .expect("P2 should have a performance snapshot");
    eprintln!(
        "[P2_FIRST] success={} total={} lives={:?} cheer={}",
        p2_snap.success,
        p2_snap.total_score,
        p2_snap.lives.iter().map(|l| l.passed).collect::<Vec<_>>(),
        game.state.player2_cheer_blade_heart_count
    );

    assert_eq!(p2_snap.lives.len(), 1, "P2's live must be judged (pre-fix this was 0)");
    assert!(
        p2_snap.lives[0].passed,
        "stage covers heart01+03+06 ≥ Dream Believers' heart0:4"
    );
    assert!(p2_snap.success, "P2's own performance must be finalized as a success");
    assert_eq!(
        p2_snap.total_score, 1,
        "Dream Believers score 1, no score icons revealed"
    );

    // P1 performed second with no live cards — no bogus judgment of P1 zones.
    match game
        .state
        .performance_snapshots
        .iter()
        .rev()
        .find(|s| s.player_id == "p1")
    {
        None => {}
        Some(p1_snap) => {
            assert!(
                p1_snap.lives.is_empty(),
                "P1 staged nothing; its snapshot must not contain judged lives"
            );
        }
    }

    // The live card stayed in P2's zone through resolution (success path).
    assert!(
        game.state.player2.live_card_zone.cards.contains(&live)
            || game.state.player2.success_live_card_zone.cards.contains(&live),
        "the successful live card belongs in P2's live/success zone"
    );
}
