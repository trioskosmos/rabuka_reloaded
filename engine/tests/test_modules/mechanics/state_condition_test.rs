/// Tests for wait/active state conditions on members.
///
/// Cards tested:
/// - PL!-bp5-333-R | 統堂英玲奈: as long as this member is wait → heart05
/// - PL!-bp3-002-R | 絢瀬絵里: per wait opponent member → blade
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// 統堂英玲奈: "このメンバーがウェイト状態であるかぎり、heart05を得る。"
/// In wait state → gain heart05.
#[test]
fn erena_in_wait_gains_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let erena = game.id("PL!-bp5-333-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [erena, -1, -1];
    fill_decks(&mut game, filler);

    // Set member to wait state
    game.state.mods.add_orientation_modifier(erena, "wait");
    game.state.recalculate_constants();

    let hc = game
        .state
        .mods
        .get_heart_modifier(erena, HeartColor::Heart05);
    assert!(
        hc >= 1,
        "Erena should have heart05 when in wait state, got {}",
        hc
    );
}

/// 統堂英玲奈: in active state → NO heart05.
#[test]
fn erena_active_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let erena = game.id("PL!-bp5-333-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [erena, -1, -1];
    fill_decks(&mut game, filler);

    // Set member to active state
    game.state.mods.add_orientation_modifier(erena, "active");
    game.state.recalculate_constants();

    let hc = game
        .state
        .mods
        .get_heart_modifier(erena, HeartColor::Heart05);
    assert_eq!(
        hc, 0,
        "Erena should NOT have heart05 when active, got {}",
        hc
    );
}

/// 統堂英玲奈: no orientation set (defaults to active) → NO heart05.
#[test]
fn erena_default_active_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let erena = game.id("PL!-bp5-333-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [erena, -1, -1];
    fill_decks(&mut game, filler);

    // No orientation set — default is active
    game.state.recalculate_constants();

    let hc = game
        .state
        .mods
        .get_heart_modifier(erena, HeartColor::Heart05);
    assert_eq!(
        hc, 0,
        "Erena should NOT have heart05 by default (active), got {}",
        hc
    );
}

/// 統堂英玲奈: set wait → gain heart, then set active → lose heart.
#[test]
fn erena_wait_to_active_switches_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let erena = game.id("PL!-bp5-333-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [erena, -1, -1];
    fill_decks(&mut game, filler);

    game.state.mods.add_orientation_modifier(erena, "wait");
    game.state.recalculate_constants();

    let hc_wait = game
        .state
        .mods
        .get_heart_modifier(erena, HeartColor::Heart05);
    assert!(
        hc_wait >= 1,
        "Erena should have heart05 when wait, got {}",
        hc_wait
    );

    // Now set to active and verify it's gone
    game.state.mods.add_orientation_modifier(erena, "active");
    game.state.recalculate_constants();

    let hc_active = game
        .state
        .mods
        .get_heart_modifier(erena, HeartColor::Heart05);
    assert_eq!(
        hc_active, 0,
        "Erena should lose heart05 when active, got {}",
        hc_active
    );
}

/// 統堂英玲奈: wait but not on stage → no heart.
#[test]
fn erena_wait_not_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let erena = game.id("PL!-bp5-333-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, -1, -1];
    fill_decks(&mut game, filler);

    // Set orientation even though not on stage
    game.state.mods.add_orientation_modifier(erena, "wait");
    game.state.recalculate_constants();

    let hc = game
        .state
        .mods
        .get_heart_modifier(erena, HeartColor::Heart05);
    assert_eq!(
        hc, 0,
        "Erena should NOT have heart05 when not on stage, got {}",
        hc
    );
}
