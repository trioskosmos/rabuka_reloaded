use crate::helpers::*;

/// Into the live card set, then on to the ライブ開始時 window — both named.
/// The old shape was two 5-pass walks with a `contains("LiveCardSet")` assert
/// between them, so a phase that gained a step would have moved the test
/// without failing the assert that was supposed to catch it.
fn advance_to_live(game: &mut TestGame) {
    game.advance_to_phase(rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker);
    assert_eq!(
        game.state.current_phase,
        rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker,
        "the helper promises the live card set, by name"
    );
    game.advance_to_phase(rabuka_engine::game_state::Phase::FirstAttackerPerformance);
}

/// Two yell-triggered watchers share one yell: Sumire (heart06) and Wien
/// (heart03) each grant their own colour, because the condition is
/// "the yelLED set contains no blade heart" — true for both.
///
/// This test previously asserted `h == 0 || h == 1` for each watcher, which is
/// true for every value, and drove the live with a 0-SCORE live card, so **no
/// yell occurred at all** and both were trivially 0. It proved nothing.
///
/// The corrected real-pipeline test — a live card with a real score requirement,
/// reading each watcher during the live — is
/// `jidou/combination/yell_real_yell_no_blade_heart_grants_heart_test.rs`
/// (`wien_real_yell_...` and `sumire_real_yell_...`, plus the no-yell negative).
/// This file keeps only the phase-advance helper it shares.
#[test]
fn yell_two_no_blade_heart_watchers_are_covered_by_the_real_yell_test() {
    // Intentionally does not assert a grant: a 0-score live cannot yell, so the
    // only honest statement here is that this fixture cannot exercise the
    // abilities. See the module note above for where the real coverage lives.
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let wien = game.id("PL!SP-bp2-021-N");
    let bladed = game.id("PL!S-sd1-003-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let energy = game.id("LL-E-001-SD");
    game.state.player1.stage.stage = [bladed, sumire, wien];
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(energy);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(filler);
    advance_to_live(&mut game);
    game.set_live_card(filler); // 0-SCORE live => no yell
    game.advance_to_phase(rabuka_engine::game_state::Phase::LiveVictoryDetermination);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
    assert!(
        !game.state.yell_occurred,
        "a 0-score live card produces no yell, so this fixture cannot exercise \
         these watchers — the real coverage is in \
         jidou/combination/yell_real_yell_no_blade_heart_grants_heart_test.rs"
    );
}
