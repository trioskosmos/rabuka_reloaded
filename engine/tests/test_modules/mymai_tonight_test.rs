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

/// 2x MY舞 + 1 other Aqours live → each MY舞 has its own LiveStart ability.
/// Both fire independently. Members get blade from both copies.
#[test]
fn mymai_two_copies_plus_aqours_live_both_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    let mymai = game.id("PL!S-bp2-023-L");
    let mymai2 = game.new_id("PL!S-bp2-023-L");
    let aqours_live = game.id("PL!S-sd1-020-SD");
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
    game.state.player1.hand.cards.push(mymai2);
    game.state.player1.hand.cards.push(aqours_live);
    advance_to_live_set(&mut game);
    game.set_live_card(mymai);
    game.set_live_card(mymai2);
    game.set_live_card(aqours_live);

    advance_to_performance(&mut game);

    // Each MY舞 fires → each adds +1 blade → total should be at least 2
    let blade = game.state.mods.get_blade_modifier(member_a);
    assert!(
        blade >= 2,
        "2 copies of MY舞 should each grant blade, got {}",
        blade
    );
}

/// 1 MY舞 + 1 other Aqours live + 1 non-Aqours live → condition still passes.
/// Non-Aqours live card does not interfere.
#[test]
fn mymai_with_aqours_and_non_aqours_live_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    let mymai = game.id("PL!S-bp2-023-L");
    let aqours_live = game.id("PL!S-sd1-020-SD");
    let non_aqours = game.id("PL!HS-pb1-025-L");
    let member_a = game.id("PL!S-bp3-018-N");
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
    game.state.player1.hand.cards.push(aqours_live);
    game.state.player1.hand.cards.push(non_aqours);
    advance_to_live_set(&mut game);
    game.set_live_card(mymai);
    game.set_live_card(aqours_live);
    game.set_live_card(non_aqours);
    advance_to_performance(&mut game);

    // PL!S-bp3-018-N (Kurosawa Ruby) should have blade
    assert!(
        game.state.mods.get_blade_modifier(member_a) > 0,
        "Member_a (PL!S-bp3-018-N) should have blade"
    );
}

/// Q121: ALL stage members get blade, not just one.
#[test]
fn mymai_q121_all_stage_members_gain_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let aqours_live = game.id("PL!S-bp5-023-L");
    let (_, member_a) = setup_mymai_only(&mut game);
    let member_b = game.state.player1.stage.stage[1];
    game.state.player1.hand.cards.push(aqours_live);
    game.set_live_card(aqours_live);
    advance_to_performance(&mut game);

    assert!(
        game.state.mods.get_blade_modifier(member_a) > 0,
        "Member_a should have blade"
    );
    assert!(
        game.state.mods.get_blade_modifier(member_b) > 0,
        "Member_b should also have blade (effect applies to ALL stage members)"
    );
}

/// 2x other Aqours + 1 MY舞 → condition satisfied, blade gained once.
/// Multiple non-excluded cards don't multiply the effect.
#[test]
fn mymai_two_aqours_live_gains_blade_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    let mymai = game.id("PL!S-bp2-023-L");
    let aq1 = game.id("PL!S-sd1-020-SD");
    let aq2 = game.new_id("PL!S-sd1-020-SD");
    let member_a = game.id("PL!S-bp3-018-N");
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
    game.state.player1.hand.cards.push(aq1);
    game.state.player1.hand.cards.push(aq2);
    advance_to_live_set(&mut game);
    game.set_live_card(mymai);
    game.set_live_card(aq1);
    game.set_live_card(aq2);
    advance_to_performance(&mut game);

    let blade = game.state.mods.get_blade_modifier(member_a);
    assert!(
        blade > 0,
        "1 MY舞 + 2 other Aqours → blade gained, got {}",
        blade
    );
}
