/// Tests for 鬼塚夏美 (PL!SP-bp2-009-R+) — LiveStart ability:
///
/// {{live_start.png|ライブ開始時}}ライブ終了時まで、自分の手札2枚につき、
/// {{icon_blade.png|ブレード}}を得る。
///
/// For each 2 cards in hand, gain 1 blade until live end.
use crate::helpers::*;

/// Ab#0 (LiveStart): For each 2 cards in hand, gain 1 blade.
/// 4 cards in hand → 2 blades (4 ÷ 2 = 2)
#[test]
fn natsumi_live_start_blade_per_2_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let natsumi = game.id("PL!SP-bp2-009-P");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    // Stage: 鬼塚夏美
    game.state.player1.stage.stage[1] = natsumi;

    // Hand: 5 fillers + live_card. After set_live_card removes live_card, 5 remain.
    // After LiveCardSetP1 pass draws 1 replacement, 6 remain.
    // At LiveStart: 6 hand cards → 6 ÷ 2 = 3 blade
    for _ in 0..5 {
        game.state.player1.hand.cards.push(filler);
    }
    game.state.player1.hand.cards.push(live_card);

    // Seed decks
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Advance to LiveStart
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));

    game.set_live_card(live_card);
    game.pass(); // LiveCardSetP2
    game.pass(); // FirstAttackerPerformance (LiveStart fires here)

    // LiveStart fired: per_unit(hand, 2) → 6 hand cards ÷ 2 = 3 blade
    let blade_mod = game.state.mods.get_blade_modifier(natsumi);
    assert_eq!(
        blade_mod, 3,
        "6 hand cards should give 3 blade (per 2 cards): 6/2*1=3"
    );
}

/// Q109: Blade count is a snapshot at resolution. Draw 2 cards after LiveStart
/// → hand goes from 6 to 8, but blade stays at 3 (not 4).
#[test]
fn q109_natsumi_blade_snapshot_hand_change_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let natsumi = game.id("PL!SP-bp2-009-P");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[1] = natsumi;

    // 5 fillers + live_card = 6 in hand at LiveStart → 6/2 = 3 blade
    for _ in 0..5 {
        game.state.player1.hand.cards.push(filler);
    }
    game.state.player1.hand.cards.push(live_card);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);
    game.pass(); // LiveCardSetP2
    game.pass(); // FirstAttackerPerformance (LiveStart fires)

    let blade_before = game.state.mods.get_blade_modifier(natsumi);
    assert_eq!(blade_before, 3, "6 hand → 3 blade at resolution");

    // Draw 2 cards (simulating an ability that adds cards to hand)
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    // Hand is now 8, but blade should STILL be 3 (snapshot)
    let blade_after = game.state.mods.get_blade_modifier(natsumi);
    assert_eq!(
        blade_after, 3,
        "Q109: Blade stays 3 even after hand increases to 8 (snapshot at resolution)"
    );
}

/// Q109 edge: Discard cards after resolution → blade unchanged.
#[test]
fn q109_natsumi_blade_unchanged_after_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let natsumi = game.id("PL!SP-bp2-009-P");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[1] = natsumi;

    for _ in 0..5 {
        game.state.player1.hand.cards.push(filler);
    }
    game.state.player1.hand.cards.push(live_card);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    let blade_before = game.state.mods.get_blade_modifier(natsumi);
    assert_eq!(blade_before, 3, "6 hand → 3 blade");

    // Discard 3 cards from hand
    let new_len = game.state.player1.hand.cards.len().saturating_sub(3);
    game.state.player1.hand.cards.truncate(new_len);

    let blade_after = game.state.mods.get_blade_modifier(natsumi);
    assert_eq!(
        blade_after, 3,
        "Q109: Blade stays 3 after discarding (hand=3, snapshot frozen)"
    );
}

/// Q109 edge: 0 hand at resolution → 0 blade.
#[test]
fn q109_natsumi_zero_hand_zero_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let natsumi = game.id("PL!SP-bp2-009-P");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[1] = natsumi;

    // Only live_card → after set_live_card, hand empty → 0 blade
    game.state.player1.hand.cards.push(live_card);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    let blade = game.state.mods.get_blade_modifier(natsumi);
    assert_eq!(blade, 0, "Q109: 0 hand at resolution → 0 blade");
}

/// Q109 edge: Odd hand count → floor division. 5 hand → 5/2 = 2 blade.
#[test]
fn q109_natsumi_odd_hand_floor_division() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let natsumi = game.id("PL!SP-bp2-009-P");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[1] = natsumi;

    // 4 fillers + live_card = 5 in hand → after set: 4 → after pass draw: 5
    // 5 ÷ 2 = 2 blade (floor division)
    for _ in 0..4 {
        game.state.player1.hand.cards.push(filler);
    }
    game.state.player1.hand.cards.push(live_card);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    let blade = game.state.mods.get_blade_modifier(natsumi);
    assert_eq!(blade, 2, "Q109: 5 hand → 5/2 = 2 blade (floor division)");
}
