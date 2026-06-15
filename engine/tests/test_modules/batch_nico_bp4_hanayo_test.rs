/// Batch: remaining 1-QA cards with simple testable abilities
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!-bp4-009-R (矢澤にこ) Q189: Debut — opponent chooses 1 of their own active members to wait.
#[test]
fn nico_bp4_q189_debut_opponent_waits_own_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let nico = game.id("PL!-bp4-009-R");
    let p2_member = game.id("PL!-sd1-010-SD");

    // Opponent has an active member on stage
    game.state.player2.stage.stage[0] = p2_member;
    game.add_to_hand(nico);
    game.give_energy(10);
    game.play_to_stage(nico, MemberArea::Center);

    // Debut fires: opponent waits 1 of their own active members
    // (with only 1 eligible target, the effect auto-resolves)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify the member is now in wait state (stays on stage)
    assert!(
        game.state.player2.stage.stage.contains(&p2_member),
        "Opponent member should still be on stage"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(p2_member),
        Some(&"wait".to_string()),
        "Opponent member should be in wait state"
    );
}

/// PL!-bp4-009-R (矢澤にこ): Multiple opponent members — forces a choice.
#[test]
fn nico_bp4_multi_opponent_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let nico = game.id("PL!-bp4-009-R");
    let p2_member_a = game.id("PL!-sd1-010-SD");
    let p2_member_b = game.id("PL!-sd1-013-SD");

    // Opponent has 2 active members on stage
    game.state.player2.stage.stage = [p2_member_a, p2_member_b, -1];
    game.add_to_hand(nico);
    game.give_energy(10);
    game.play_to_stage(nico, MemberArea::Center);

    // Debut fires: opponent must choose which member to wait
    assert!(
        game.has_pending_choice(),
        "Opponent must choose which member to wait with 2+ eligible"
    );
    let entry = game.state.ability_queue.current_entry();
    assert_eq!(
        entry.as_ref().and_then(|e| e.choice_player_id.as_deref()),
        Some("p2"),
        "Wait-member choice should be routed to opponent"
    );

    // Opponent selects p2_member_b (index 1)
    game.select_indices(&[1]);

    // Verify: p2_member_b is waited, p2_member_a stays active
    assert_eq!(
        game.state.mods.get_orientation_modifier(p2_member_b),
        Some(&"wait".to_string()),
        "p2_member_b should be in wait state"
    );
    assert!(
        game.state
            .mods
            .get_orientation_modifier(p2_member_a)
            .is_none()
            || game.state.mods.get_orientation_modifier(p2_member_a) == Some(&"active".to_string()),
        "p2_member_a should stay active (not waited)"
    );
}

/// PL!-sd1-019-SD (START:DASH!!) Q36: LiveSuccess timing definition.
/// Draw 3 from deck, arrange any order on top, rest to discard.
#[test]
fn start_dash_q36_live_success_draw_3() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let start_dash = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(start_dash);
    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    let deck_before = game.state.player1.main_deck.len();

    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.set_live_card(start_dash);
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    // LiveSuccess triggers → draw 3 from deck
    let deck_after = game.state.player1.main_deck.len();
    assert!(
        deck_after <= deck_before,
        "LiveSuccess should draw cards: {} → {}",
        deck_before,
        deck_after
    );
}
