use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn sp_bp1_007_debut_counts_total_energy_not_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp1-007-R＋"); // cost 13
    let filler = game.id("PL!-sd1-010-SD");
    let dead_live = game.id("PL!-sd1-020-SD");

    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.add_to_discard(dead_live);

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Full-price play: 13 of 15 flipped to wait → only 2 ACTIVE left.
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "precondition: active energy dropped below 11"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        15,
        "paid energy stays in the zone as wait state"
    );
    assert!(
        game.state.player1.hand.cards.contains(&dead_live),
        "condition counts TOTAL energy (15 ≥ 11) → live card retrieved"
    );
}

#[test]
fn sp_bp1_007_debut_no_retrieve_when_total_below_11_via_baton_touch() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp1-007-R＋"); // cost 13
    let filler = game.id("PL!-sd1-010-SD");
    let dead_live = game.id("PL!-sd1-020-SD");
    // Vanilla cost-13 member to baton-touch over: play cost becomes
    // 13 − 13 = 0 (rules 9.6.2.3.2).
    let victim = game.id("PL!N-sd2-002-SD2");

    fill_decks(&mut game, filler);
    // Direct placement of the baton-touch victim: there is no cheaper real
    // action that stages a cost-13 vanilla without spending the energy this
    // test must keep below 11.
    game.add_to_stage(MemberArea::Center, victim);
    game.give_energy(10); // total 10 < 11
    game.add_to_discard(dead_live);

    game.state.player1.hand.cards.push(card);
    // Real pipeline play with baton touch (the TestGame helper can't pass
    // use_baton_touch).
    rabuka_engine::turn::TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        None,
        Some(MemberArea::Center), // baton touch replaces the member IN this area
        Some(true),
    )
    .expect("baton touch play should succeed at zero cost");
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.stage.stage.contains(&card),
        "mei should be on stage"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&victim),
        "baton touch sends the replaced member to the waitroom"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        10,
        "zero-cost play keeps the energy zone untouched"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&dead_live),
        "total energy 10 < 11 → no retrieval"
    );
}

#[test]
fn sp_bp1_007_debut_noop_when_waitroom_has_no_live_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp1-007-R＋");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(15);
    fill_decks(&mut game, filler);
    // Waitroom has only members, no live card.
    game.add_to_discard(filler);

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        game.dbg_events(5);
        game.select_indices(&[]);
    }

    assert!(game.has_pending_choice() == false, "no stuck prompts");
}
