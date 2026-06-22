/// Tests for PL!HS-bp6-005-R＋ 徒町 小鈴 (ab#0):
///
/// ライブ開始時: discard 1 from hand (optional cost)
///   → this member's cost +6 until live end
///   → if 蓮ノ空 members' total cost on your stage > opponent's stage total cost
///     → gain heart05 + blade until live end
///
/// Both cases tested on a clean board with no-ability fillers only.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_card_set(game: &mut TestGame) {
    game.pass(); // → Active
    game.pass(); // → Energy
    game.pass(); // → Draw
    game.pass(); // → Main
    game.pass(); // → LiveCardSetP1
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass(); // LiveCardSetP1 → LiveCardSetP2
    game.pass(); // LiveCardSetP2 → LiveStart
}

/// Condition MET: 蓮ノ空 total cost (10+11+15=36) > opponent total (4).
/// Pay discard cost → cost+6 → gain heart05 + blade.
#[test]
fn kosuzu_bp6_condition_met_gains_heart05_and_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kosuzu = game.id("PL!HS-bp6-005-R＋"); // 蓮ノ空 cost 10
    let kanoha = game.id("PL!HS-bp1-001-R"); // 蓮ノ空 cost 11
    let lilienthal = game.id("PL!HS-bp6-007-R"); // 蓮ノ空 cost 15
    let p2_filler = game.id("PL!-sd1-010-SD"); // cost 4, no abilities
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.new_id("PL!-sd1-010-SD");

    // P1 stage: 3 蓮ノ空 members (total cost = 10+11+15 = 36)
    game.state.player1.stage.stage = [kanoha, kosuzu, lilienthal];
    // P2 stage: 1 filler (total cost = 4)
    game.state.player2.stage.stage = [p2_filler, -1, -1];

    // Hand: 1 discardable card + 1 live card
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(live);

    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
        game.state
            .player2
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }
    game.state
        .player2
        .hand
        .cards
        .push(game.new_id("PL!-sd1-010-SD"));

    game.give_energy(20);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    // Optional discard cost prompt
    assert!(
        game.has_pending_choice(),
        "Should prompt for optional discard cost"
    );
    // Pay: discard 1 card from hand
    game.select_indices(&[0]);

    // Resolve any remaining choices
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.print_trace();

    eprintln!(
        "[DEBUG] blade_modifiers: {:?}",
        game.state.mods.blade_modifiers
    );
    eprintln!(
        "[DEBUG] heart_modifiers: {:?}",
        game.state.mods.heart_modifiers
    );
    eprintln!("[DEBUG] kosuzu={}", kosuzu);
    for (k, v) in &game.state.mods.blade_modifiers {
        eprintln!("  blade mod: card={} val={}", k, v);
    }
    for (k, v) in &game.state.mods.heart_modifiers {
        eprintln!("  heart mod: card={} val={:?}", k, v);
    }

    // 蓮ノ空 total (36) > opponent (4) → should gain blade + heart05
    let blade = game.state.mods.get_blade_modifier(kosuzu);
    assert!(blade > 0, "Condition met: should gain blade, got {}", blade);

    let heart05 = game
        .state
        .mods
        .get_heart_modifier(kosuzu, HeartColor::Heart05);
    assert!(
        heart05 > 0,
        "Condition met: should gain heart05, got {}",
        heart05
    );
}

/// Condition NOT MET: 蓮ノ空 total cost (10) <= opponent total cost (4+4+4=12).
/// Pay discard cost → cost+6 → NO heart05, NO blade.
#[test]
fn kosuzu_bp6_condition_not_met_no_heart05_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kosuzu = game.id("PL!HS-bp6-005-R＋"); // 蓮ノ空 cost 10
                                               // Opponent needs total > 16 (kosuzu 10 + 6 modifier) so condition fails
    let p2_high = game.id("PL!-sd1-009-SD"); // cost 15
    let p2_filler = game.new_id("PL!-sd1-010-SD"); // cost 4
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.new_id("PL!-sd1-010-SD");

    // P1 stage: only kosuzu (蓮ノ空, base cost 10, +6 = 16 after ability)
    game.state.player1.stage.stage = [-1, kosuzu, -1];
    // P2 stage: total cost = 15 + 4 = 19 > 16 → condition NOT met
    game.state.player2.stage.stage = [p2_high, p2_filler, -1];

    // Hand: 1 discardable + 1 live
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(live);

    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
        game.state
            .player2
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }
    game.state
        .player2
        .hand
        .cards
        .push(game.new_id("PL!-sd1-010-SD"));

    game.give_energy(20);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert!(
        game.has_pending_choice(),
        "Should prompt for optional discard cost"
    );
    // Pay the cost
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // 蓮ノ空 total with +6 = 16 < opponent 19 → should NOT gain blade or heart05
    let blade = game.state.mods.get_blade_modifier(kosuzu);
    assert_eq!(
        blade, 0,
        "Condition not met: 16 < 19, should not gain blade, got {}",
        blade
    );

    let heart05 = game
        .state
        .mods
        .get_heart_modifier(kosuzu, HeartColor::Heart05);
    assert_eq!(
        heart05, 0,
        "Condition not met: 16 < 19, should not gain heart05, got {}",
        heart05
    );
}
