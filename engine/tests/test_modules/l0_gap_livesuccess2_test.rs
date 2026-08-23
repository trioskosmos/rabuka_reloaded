/// L0 gap coverage: additional LiveSuccess abilities — score modifiers,
/// per-unit scoring, and card retrieval from revealed cards.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn drain_skips(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { allow_skip: true, .. } => game.select_indices(&[]),
            _ => break,
        }
    }
}

fn advance_live(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
        drain_skips(game);
    }
}

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// PL!N-bp3-031-L: LiveSuccess → per WAITED member on own stage,
/// this card's score +1.
/// TODO: needs investigation — per-waited-member score trigger path.
#[test]
#[ignore = "per-waited-member score needs investigation"]
fn bp3_031_per_waited_member_score_plus1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!N-bp3-031-L");
    // Two waited members on stage
    let m1 = game.new_id("PL!N-sd1-002-SD");
    let m2 = game.new_id("PL!N-sd1-003-SD");
    game.state.player1.stage.stage = [m1, m2, -1];
    game.state.mods.add_orientation_modifier(m1, "wait");
    game.state.mods.add_orientation_modifier(m2, "wait");
    let fid = game.id_ref("PL!-sd1-010-SD");
    fill_decks(&mut game, fid);
    game.state.player1.hand.cards.push(live);

    advance_live(&mut game);

    let score = game
        .state
        .mods
        .get_score_modifier(live)
        .abs();
    assert!(
        score >= 2,
        "two waited members → >= +2 score modifier"
    );
}

/// PL!SP-bp4-003-R: Constant center → +2 blade.
#[test]
fn sp_bp4_003_center_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let m = game.id("PL!SP-bp4-003-R");
    game.state.player1.stage.stage = [-1, m, -1];
    game.state.recalculate_constants();
    assert!(
        game.state.mods.get_blade_modifier(m) >= 2,
        "center constant grants blade"
    );
}
