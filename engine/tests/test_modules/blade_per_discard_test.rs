use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// LL-bp2-001-R+ 渡辺曜&鬼塚夏美&大沢瑠璃乃 ab#2:
/// LiveStart: discard named characters from hand → gain 1 blade per discard.
/// new_id: distinct hand copy (name contains "渡辺曜") vs stage copy.
#[test]
fn triple_discard_gives_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let stage = game.id("LL-bp2-001-R+");
    let hand = game.new_id("LL-bp2-001-R+");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = stage;
    game.give_energy(15);
    game.state.player1.hand.cards.push(hand);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    assert!(game.has_pending_choice(), "Optional cost should appear");
    game.select_indices(&[0]); // hand copy at index 0 — matches characters
    if game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&stage)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 1, "1 card discarded → 1 blade, got {}", blade);
}

/// No matching cards → skip → 0 blades.
#[test]
fn triple_no_matching_zero_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id("LL-bp2-001-R+");
    let live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = triple;
    game.give_energy(15);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let blade = game
        .state
        .mods
        .blade_modifiers
        .get(&triple)
        .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
    assert_eq!(blade, 0, "No matching → 0 blades, got {}", blade);
}
