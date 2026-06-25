/// Tests for 日野下花帆 (PL!HS-PR-016-PR) same_unit_name filter:
///
/// ライブ開始時 手札の同じユニット名を持つカード2枚を控え室に置いてもよい：
/// ライブ終了時まで、heart04×2 + blade×2 を得る。
///
/// The cost is optional ("置いてもよい"). When the player cannot pay
/// (no unit has ≥2 cards), the cost is silently skipped.
/// When a qualifying unit exists, ALL cards from qualifying units (≥2)
/// are shown — the player picks 2 from the same unit.
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass(); // LiveCardSetP1→P2, draws 1 for P1
    game.pass(); // LiveCardSetP2→FirstAttackerPerf, draws for P2, triggers LiveStart
}

/// 3 Printemps in hand → eligible unit (≥2). First prompt shows all,
/// then re-prompt filters to chosen unit. Player picks 2.
#[test]
fn hanano_same_unit_choice_discards_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanano = game.id("PL!HS-PR-016-PR");
    let live = game.id("PL!-sd1-019-SD");
    let p_a = game.id("PL!-sd1-010-SD"); // Printemps
    let p_b = game.id("PL!-sd1-008-SD"); // Printemps
    let p_c = game.id("PL!-sd1-003-SD"); // Printemps
    let lily = game.id("PL!-sd1-013-SD"); // lilywhite

    game.state.player1.stage.stage[0] = hanano;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(p_a);
    game.state.player1.hand.cards.push(p_b);
    game.state.player1.hand.cards.push(p_c);
    game.state.player1.hand.cards.push(lily);

    for _ in 0..20 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-002-SD"));
    }
    for _ in 0..20 {
        game.state
            .player2
            .main_deck
            .cards
            .push(game.id("PL!-sd1-002-SD"));
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // Printemps(3) ≥ 2 → choice must exist
    assert!(
        game.has_pending_choice(),
        "3 same-unit cards should create a choice"
    );

    let before = game.state.player1.hand.cards.len();
    // First prompt: pick 1 card from any qualifying unit
    game.try_select_indices(&[0]).unwrap();
    // Re-prompt: pick 1 more card, filtered to same unit
    game.try_select_indices(&[0]).unwrap();
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before - 2,
        "2 cards were discarded"
    );
    assert!(
        game.state.player1.hand.cards.contains(&lily),
        "lilywhite (different unit) should remain"
    );
}

/// No unit has ≥2 cards → optional cost silently skipped.
#[test]
fn hanano_no_qualifying_unit_skips_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanano = game.id("PL!HS-PR-016-PR");
    let live = game.id("PL!-sd1-019-SD"); // unit=None
    let print = game.id("PL!-sd1-010-SD"); // Printemps (1 copy)
    let lily = game.id("PL!-sd1-013-SD"); // lilywhite (1 copy)
    let bibi = game.id("PL!-sd1-002-SD"); // BiBi

    game.state.player1.stage.stage[0] = hanano;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(print);
    game.state.player1.hand.cards.push(lily);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(bibi);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(bibi);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // Hand: live + print + lily + 1 drawn bibi = 4 cards, all from different units
    // No unit has ≥2 → optional cost skipped
    let hand_count = game.state.player1.hand.cards.len();
    assert!(
        !game.has_pending_choice(),
        "No unit has ≥2 → cost skipped, no choice"
    );
    assert!(
        game.state.player1.hand.cards.contains(&print),
        "Printemps still in hand"
    );
    assert!(
        game.state.player1.hand.cards.contains(&lily),
        "lilywhite still in hand"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_count,
        "No cards discarded when cost is skipped"
    );
}
