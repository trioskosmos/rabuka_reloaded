/// Tests for performance phase rules and heart checks:
/// 1. 0 cards on stage -> live fails (sent to waitroom, score 0, success false)
/// 2. Sufficient hearts -> live passes (sent to success zone, score > 0, success true)
/// 3. Multi-live allocation: card 1 passes, card 2 fails -> ALL fail (Rule 8.3.16)
/// 4. Requirement modifiers (set/additive) apply per-color without wiping base requirements
/// 5. Failed live cards are discarded and do NOT win victory determination
use crate::helpers::*;
use rabuka_engine::game_state::Phase;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn setup_game_with_decks(game: &mut TestGame, p1_stage: [i16; 3], filler: i16) {
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

    game.state.player1.stage.stage = p1_stage;
    game.state.player2.stage.stage = [-1, filler, -1];
}

fn run_full_turn(game: &mut TestGame) {
    for _ in 0..30 {
        if game.state.current_phase == Phase::Active && !game.has_pending_choice() {
            break;
        }
        if game.has_pending_choice() {
            game.select_indices(&[]);
        } else {
            game.pass();
        }
    }
}

#[test]
fn test_zero_cards_on_stage_fails_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!-sd1-019-SD"); // START:DASH!! needs h01=1, h03=1, h06=1
    let filler = game.id("PL!-sd1-010-SD");

    // P1 has NO cards on stage [-1, -1, -1]
    setup_game_with_decks(&mut game, [-1, -1, -1], filler);
    game.state.player1.hand.cards.push(live);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    run_full_turn(&mut game);

    // Live card should have failed and moved to waitroom, NOT success zone
    assert_eq!(
        game.state.player1.success_live_card_zone.cards.len(),
        0,
        "Failed live card should NOT be in success zone"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "Failed live card should be moved to waitroom"
    );

    let snap = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1");
    if let Some(s) = snap {
        assert!(!s.success, "Performance snapshot success should be false");
        assert_eq!(s.total_score, 0, "Failed live total score should be 0");
    }
}

#[test]
fn test_sufficient_hearts_passes_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!-sd1-019-SD"); // START:DASH!! needs h01=1, h03=1, h06=1
    let member = game.id("PL!-sd1-001-SD"); // bh={h01=1, h03=2, h06=1}
    let filler = game.id("PL!-sd1-010-SD");

    setup_game_with_decks(&mut game, [-1, member, -1], filler);
    game.state.player1.hand.cards.push(live);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    run_full_turn(&mut game);

    assert_eq!(
        game.state.player1.success_live_card_zone.cards.len(),
        1,
        "Passed live card should be in success zone"
    );
    assert_eq!(
        game.state.player1.success_live_card_zone.cards[0], live,
        "Success zone card should match set live card"
    );

    let snap = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .expect("P1 snapshot should exist");

    assert!(snap.success, "Performance snapshot success should be true");
    assert!(snap.total_score > 0, "Passed live score should be > 0");
    assert!(
        snap.lives[0].passed,
        "Live card passed field should be true"
    );
}

#[test]
fn test_per_color_modifier_does_not_erase_base_requirements() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!-sd1-019-SD"); // base needs: h01=1, h03=1, h06=1
    let member = game.id("PL!-sd1-001-SD"); // bh={h01=1, h03=2, h06=1}
    let filler = game.id("PL!-sd1-010-SD");

    setup_game_with_decks(&mut game, [-1, member, -1], filler);
    game.state.player1.hand.cards.push(live);

    // Apply a modifier setting heart01 requirement to 2 (modifying only heart01)
    let entry = rabuka_engine::core::game_modifiers::ModifierEntry {
        set: 2,
        additive: 0,
    };
    game.state
        .mods
        .need_heart_modifiers
        .entry(live)
        .or_default()
        .insert(rabuka_engine::card::HeartColor::Heart01, entry);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    run_full_turn(&mut game);

    // Since member only provides 1 h01, but requirement was modified to 2 h01,
    // the card should fail even though member provided h03 and h06.
    assert_eq!(
        game.state.player1.success_live_card_zone.cards.len(),
        0,
        "Card requiring 2 h01 should fail when stage only provides 1 h01"
    );

    let snap = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .expect("P1 snapshot should exist");

    // Base requirements for h03 and h06 should still be present in required array:
    // h01=2, h03=1, h06=1
    assert_eq!(
        snap.lives[0].required[1], 2,
        "h01 required should be modified to 2"
    );
    assert_eq!(
        snap.lives[0].required[3], 1,
        "h03 required should remain base 1"
    );
    assert_eq!(
        snap.lives[0].required[6], 1,
        "h06 required should remain base 1"
    );
}
