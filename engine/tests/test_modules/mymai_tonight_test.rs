/// Tests for MY舞☆TONIGHT (PL!S-bp2-023-L) ab#0 — LiveStart: give blade to ALL stage members.
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn setup_mymai_only(game: &mut TestGame) -> (i16, i16) {
    let filler = game.id("PL!-sd1-010-SD");
    let mymai = game.id("PL!S-bp2-023-L");
    let member_a = game.id("PL!S-sd1-001-SD");
    let member_b = game.id("PL!N-sd1-001-SD");
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [member_a, member_b, -1];
    game.state.player1.hand.cards.push(mymai);
    advance_to_live_set(game);
    game.set_live_card(mymai);
    (mymai, member_a)
}

fn advance_to_performance(game: &mut TestGame) {
    game.pass(); // LiveCardSetFirstAttacker → LiveCardSetSecondAttacker
    game.pass(); // LiveCardSetSecondAttacker → FirstAttackerPerformance → LiveStart
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

/// Condition: Aqours live card other than MY舞☆TONIGHT → blade gained.
#[test]
fn mymai_tonight_with_aqours_live_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aqours_live = game.id("PL!S-bp5-023-L");
    let (_, member_a) = setup_mymai_only(&mut game);
    game.state.player1.hand.cards.push(aqours_live);
    game.set_live_card(aqours_live);
    advance_to_performance(&mut game);
    assert!(
        game.state
            .mods
            .blade_modifiers
            .get(&member_a)
            .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total)
            > 0
    );
}

/// Only MY舞☆TONIGHT → condition fails (excluded by name).
#[test]
fn mymai_tonight_alone_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (_, member_a) = setup_mymai_only(&mut game);
    advance_to_performance(&mut game);
    assert_eq!(
        game.state
            .mods
            .blade_modifiers
            .get(&member_a)
            .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total),
        0
    );
}

/// Aqours MEMBER card (not live) → condition fails (card_type filter).
#[test]
fn mymai_tonight_with_aqours_member_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aqours_member = game.id("PL!S-sd1-001-SD");
    let (_, member_a) = setup_mymai_only(&mut game);
    game.state.player1.hand.cards.push(aqours_member);
    game.set_live_card(aqours_member);
    advance_to_performance(&mut game);
    assert_eq!(
        game.state
            .mods
            .blade_modifiers
            .get(&member_a)
            .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total),
        0
    );
}

/// Non-Aqours live card → condition fails (group filter).
#[test]
fn mymai_tonight_with_non_aqours_live_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let non_aqours = game.id("PL!-sd1-019-SD");
    let (_, member_a) = setup_mymai_only(&mut game);
    game.state.player1.hand.cards.push(non_aqours);
    game.set_live_card(non_aqours);
    advance_to_performance(&mut game);
    assert_eq!(
        game.state
            .mods
            .blade_modifiers
            .get(&member_a)
            .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total),
        0
    );
}

/// Blade disappears after LiveVictoryDetermination (duration=live_end).
#[test]
fn mymai_tonight_blade_disappears_after_live_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aqours_live = game.id("PL!S-bp5-023-L");
    let (_, member_a) = setup_mymai_only(&mut game);
    game.state.player1.hand.cards.push(aqours_live);
    game.set_live_card(aqours_live);
    advance_to_performance(&mut game);
    assert!(
        game.state
            .mods
            .blade_modifiers
            .get(&member_a)
            .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total)
            > 0
    );
    game.pass(); // FirstAttackerPerformance → SecondAttackerPerformance
    game.pass(); // SecondAttackerPerformance → LiveVictoryDetermination
    game.pass(); // LiveVictoryDetermination → Active (Turn 2, LiveEnd effects cleared)
    assert_eq!(
        game.state
            .mods
            .blade_modifiers
            .get(&member_a)
            .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total),
        0
    );
}
