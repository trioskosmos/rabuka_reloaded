/// Tests for 優木せつ菜 (PL!N-pb1-007-R) — Q205
///
/// 常時: During live, if live card's need_heart contains heart01-06 each >= 1,
/// gain ALL heart (heart00).
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass(); // LiveCardSetP1 → LiveCardSetP2
    game.pass(); // LiveCardSetP2 → FirstAttackerPerformance
}

/// Setsuna on stage, TOKIMEKI Runners (needs all 6 hearts) as live card.
/// During live, constant ability should grant +1 ALL heart.
#[test]
fn setsuna_q205_all_heart_granted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let setsuna = game.id("PL!N-pb1-007-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!N-bp3-030-L"); // Love U my friends — needs heart01-06 each 1

    game.state.player1.stage.stage = [setsuna, filler, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    game.pass(); // FirstAttackerPerformance → SecondAttackerPerformance (snapshot created)

    let snapshot = game
        .state
        .performance_snapshots
        .first()
        .expect("Performance snapshot should exist after live start");
    let setsuna_contrib = snapshot
        .member_contributions
        .iter()
        .find(|mc| mc.source_id == setsuna)
        .expect("Setsuna should have a performance contribution");

    let total_hearts: u8 = setsuna_contrib.base_hearts.iter().sum::<u8>()
        + setsuna_contrib.bonus_hearts.iter().sum::<u8>();

    assert_eq!(
        total_hearts, 6,
        "Setsuna should have 6 hearts (5 base + 1 ALL), got {}",
        total_hearts
    );

    let all_bonus = setsuna_contrib
        .bonus_hearts
        .get(rabuka_engine::card::HeartColor::Heart00.index())
        .copied()
        .unwrap_or(0);
    assert!(
        all_bonus > 0,
        "Setsuna should have ALL heart bonus, got {}",
        all_bonus
    );
}

/// Setsuna on stage with a live card that only has heart0 (wildcard), not the
/// specific 6 heart types. Condition should NOT be met — no ALL heart bonus.
#[test]
fn setsuna_q205_condition_not_met_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let setsuna = game.id("PL!N-pb1-007-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!N-bp5-026-L"); // TOKIMEKI Runners — needs heart03 + heart0 (wildcard) only

    game.state.player1.stage.stage = [setsuna, filler, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    game.pass(); // FirstAttackerPerformance → SecondAttackerPerformance (snapshot created)

    let snapshot = game
        .state
        .performance_snapshots
        .first()
        .expect("Performance snapshot should exist after live start");
    let setsuna_contrib = snapshot
        .member_contributions
        .iter()
        .find(|mc| mc.source_id == setsuna)
        .expect("Setsuna should have a performance contribution");

    let total_hearts: u8 = setsuna_contrib.base_hearts.iter().sum::<u8>()
        + setsuna_contrib.bonus_hearts.iter().sum::<u8>();

    assert_eq!(
        total_hearts, 5,
        "Without all 6 specific heart types, Setsuna should have only base 5 hearts, got {}",
        total_hearts
    );
}

/// Setsuna on stage during non-live phase (Main). The temporal_condition
/// "during_live" should prevent the ALL heart bonus outside of live phases.
#[test]
fn setsuna_q205_no_bonus_outside_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let setsuna = game.id("PL!N-pb1-007-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [setsuna, filler, -1];
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    // Still in Main phase — no live ongoing
    let modifier = game
        .state
        .mods
        .get_heart_modifier(setsuna, rabuka_engine::card::HeartColor::All);
    assert_eq!(modifier, 0, "No ALL heart modifier outside live phase");
}
