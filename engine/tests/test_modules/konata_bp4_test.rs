/// ab#0: 近江彼方 (PL!N-bp4-006-R) — Debut: pay 2E → deploy a 虹ヶ咲 member (cost≤4) from hand.
///   If the deployed member has blade heart, wait this member (Kanata).
///
/// Tests:
///   - Deploy BH member → Kanata becomes wait
///   - Deploy non-BH member → Kanata stays active
///   - Skip cost → no effect
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Deploy a blade-heart 虹ヶ咲 member → Kanata becomes wait
#[test]
fn konata_bp4_deploy_blade_heart_member_waits() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp4-006-R");
    // A 虹ヶ咲 member with cost ≤4 and blade heart
    let bh_member = game.id("PL!N-sd1-006-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Kanata in hand to play
    game.state.player1.hand.cards.push(konata);
    game.state.player1.hand.cards.push(bh_member);
    fill_decks(&mut game, filler);
    game.give_energy(13); // 11 for deploy + 2 for optional cost

    // Play Kanata to stage — triggers debut ability
    game.play_to_stage(konata, MemberArea::Center);

    // Pay the optional cost (2E) - must be offered.
    assert!(
        game.has_pending_choice(),
        "optional 2E cost must be offered"
    );
    game.select_option(1);

    // Choose a position for the deployed member.
    assert!(
        game.has_pending_choice(),
        "position choice for the deployed member must appear"
    );
    game.select_generated(0);
    let konata_waited = game
        .state
        .mods
        .get_orientation_modifier(konata)
        .map_or(false, |o| o == "wait");
    assert!(
        konata_waited,
        "Kanata should become wait when deploying BH member"
    );
    // The deployed member must actually be ON STAGE — otherwise the wait
    // could come from a failed deploy path.
    assert!(
        game.state.player1.stage.stage.contains(&bh_member),
        "BH member was deployed to the stage"
    );
}

/// Deploy a non-blade-heart 虹ヶ咲 member → Kanata stays active
#[test]
fn konata_bp4_deploy_non_blade_heart_member_stays_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp4-006-R");
    // A 虹ヶ咲 member with cost ≤4 and NO blade heart
    let non_bh_member = game.id("PL!N-bp1-004-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(konata);
    game.state.player1.hand.cards.push(non_bh_member);
    fill_decks(&mut game, filler);
    game.give_energy(13);

    game.play_to_stage(konata, MemberArea::Center);

    if game.has_pending_choice() {
        game.select_option(1);
    }

    if game.has_pending_choice() {
        game.select_generated(0);
    }
    if game.has_pending_choice() {
        game.select_generated(0); // choose position
    }

    // The deployed member must actually be on stage for this negative to
    // mean anything.
    assert!(
        game.state.player1.stage.stage.contains(&non_bh_member),
        "non-BH member was deployed (precondition)"
    );

    // Assert Kanata is still active (deployed member has no blade heart)
    let konata_waited = game
        .state
        .mods
        .get_orientation_modifier(konata)
        .map_or(false, |o| o == "wait");
    assert!(
        !konata_waited,
        "Kanata should stay active when deploying non-BH member"
    );
}

/// Skip the optional cost → no effect, Kanata stays active
#[test]
fn konata_bp4_skip_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp4-006-R");
    let bh_member = game.id("PL!N-sd1-006-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(konata);
    game.state.player1.hand.cards.push(bh_member);
    fill_decks(&mut game, filler);
    game.give_energy(15); // ample: the 2E cost must be payable so skipping is a real choice

    game.play_to_stage(konata, MemberArea::Center);

    // The optional cost must be offered even when we intend to skip.
    assert!(
        game.has_pending_choice(),
        "optional cost prompt must appear before it can be skipped"
    );
    game.select_option(0); // skip optional cost

    // No more pending choices (cost was skipped, no effect)
    assert!(
        !game.has_pending_choice(),
        "No more choices after skipping cost: {}",
        game.pending_choice_summary()
    );

    // Kanata should still be active
    let konata_waited = game
        .state
        .mods
        .get_orientation_modifier(konata)
        .map_or(false, |o| o == "wait");
    assert!(
        !konata_waited,
        "Kanata should stay active when cost skipped"
    );
}

/// Q188: 近江彼方 (PL!N-bp4-018-N) — Auto: when active->wait during main, draw 1 drop 1.
#[test]
fn konata_bp4_q188_placed_in_wait_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp4-018-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [konata, filler, filler];
    game.state.mods.add_orientation_modifier(konata, "wait");

    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, "p1");
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "No cards drawn — condition not met"
    );
}
