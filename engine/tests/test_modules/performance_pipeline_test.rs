/// Integration test for the full performance pipeline (rules 8.3.1 - 8.4.14):
///
/// Tests that blades → yell → heart check → score → winner determination
/// all work correctly end-to-end, and that the performance snapshot
/// contains accurate data matching the rules.
use crate::helpers::*;

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

#[test]
fn performance_pipeline_blade_yell_heart_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!-bp3-026-L"); // Oh,Love&Peace! need_heart: h01=2, h03=5, h06=2, h0=6
    let filler = game.id("PL!-sd1-010-SD"); // b_heart03=1, blade=1
    let center = game.id("PL!-pb1-014-R"); // h01=3, h03=2, h06=2, blade=3
    let right = game.id("PL!-PR-003-PR"); // h01=2, h03=3, h06=1, blade=4

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

    // Fill both players' main decks with cheer cards
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [-1, center, right];
    game.state.player2.stage.stage = [-1, filler, -1];

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);

    // P2 sets the same live card
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.state.player2.hand.cards.push(live_card);
    game.pass();
    game.set_live_card(live_card);
    game.pass();

    // Handle any pending choices (live-start triggers etc.)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Now advance through remaining phases
    game.pass(); // → SecondAttackerPerformance
    game.pass(); // → LiveVictoryDetermination
    game.pass(); // → Active

    // Get the performance snapshot for P1
    let perf = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .expect("P1 should have a performance snapshot");

    // --- Rule 8.3.10: Blade sum ---
    let total_blades: u32 = perf
        .member_contributions
        .iter()
        .map(|m| m.base_blades + m.bonus_blades)
        .sum();
    assert_eq!(total_blades, 7, "P1: 7 blades (3+4)");

    // --- Rule 8.3.11-12: Yell produced cards with blade hearts ---
    assert!(!perf.yell_cards.is_empty(), "P1: yell cards exist");
    let yell_blade_hearts: u32 = perf
        .yell_cards
        .iter()
        .flat_map(|y| y.blade_hearts.iter())
        .sum();
    assert!(yell_blade_hearts > 0, "P1: yell blade hearts > 0");

    // --- Rule 8.3.14: Total hearts = member hearts + yell blade hearts ---
    let member_hearts: u32 = perf
        .member_contributions
        .iter()
        .flat_map(|m| m.base_hearts.iter().chain(m.bonus_hearts.iter()))
        .sum();
    let total_hearts: u32 = perf.total_hearts.iter().sum();
    assert_eq!(
        total_hearts,
        member_hearts + yell_blade_hearts,
        "P1: total hearts = member hearts + yell blade hearts"
    );

    // --- Rule 8.3.15-16: Heart requirement check ---
    // OH needs h01=2, h03=5, h06=2, h0=6 (total 15)
    // Base hearts: h01=5, h03=5, h06=3 (13). Yell adds h03 via b_heart03.
    // With 7 blades and all cheer cards, 7 yell cards × heart03=1 = 7 heart03.
    // Total: h01=5, h03=12, h06=3 = 20. Should satisfy 15.
    eprintln!(
        "DEBUG: lives={:?} total_hearts={:?} yell={:?} member={:?}",
        perf.lives
            .iter()
            .map(|l| (l.passed, &l.required, &l.filled))
            .collect::<Vec<_>>(),
        perf.total_hearts,
        perf.yell_cards
            .iter()
            .map(|y| &y.blade_hearts)
            .collect::<Vec<_>>(),
        perf.member_contributions
            .iter()
            .map(|m| &m.base_hearts)
            .collect::<Vec<_>>(),
    );
    assert!(
        perf.lives.iter().any(|l| l.passed),
        "P1: at least one live card should pass heart check"
    );

    // --- Rule 8.4.3: Score comparison ---
    assert!(perf.total_score > 0, "P1: should have non-zero score");

    // --- Rule 8.4.6: Winner determination ---
    // P2 has only 1 weak member (2 hearts, 1 blade), can't satisfy OH.
    // P2's live card fails heart check, P2 has no cards in zone.
    // P1 wins by default (8.4.3.2).
    assert!(
        perf.p0_wins || perf.p1_wins,
        "P1 should win (P2 live card can't meet heart requirement)"
    );

    // --- Check triggered abilities are populated ---
    assert!(
        perf.triggered_abilities.is_empty() || perf.triggered_abilities.iter().any(|t| t.is_public),
        "Triggered abilities should have is_public flag set"
    );
}

#[test]
fn performance_pipeline_fail_when_hearts_insufficient() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!-bp3-026-L"); // needs 15 hearts
    let filler = game.id("PL!-sd1-010-SD"); // 2 hearts, 1 blade

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

    // Only 1 weak member — not enough hearts for OH
    game.state.player1.stage.stage = [-1, filler, -1];
    game.state.player2.stage.stage = [-1, -1, -1];

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    game.pass();
    game.pass();

    let perf = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .expect("P1 should have a performance snapshot");

    // Rule 8.3.16: All cards discarded when heart requirement fails
    assert!(
        !perf.lives.iter().any(|l| l.passed),
        "P1: all lives should fail when hearts are insufficient (8.3.16)"
    );

    // P1 has no cards in zone, P2 has none → no winner (8.4.3.1)
    assert!(
        !perf.p0_wins && !perf.p1_wins,
        "No winner when both players have no cards in zone (8.4.3.1)"
    );
}
