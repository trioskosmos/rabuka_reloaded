use crate::helpers::*;

/// Helper to set up baton touch state and trigger the Niji card's auto ability.
fn setup_baton_touch_scenario(
    game: &mut TestGame,
    niji_card: i16,
    arriving_card: i16,
) -> (usize, usize) {
    // Add energy cards (inactive, available for activation effect)
    for _ in 0..5 {
        let e = game.new_id("LL-E-001-SD");
        game.state.player1.energy_zone.cards.push(e);
    }
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..10 {
        game.state
            .player2
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }

    let initial_hand = game.state.player1.hand.cards.len();
    let initial_energy_active = game.state.player1.energy_zone.active_count();

    game.state.player1.waitroom.cards.push(niji_card);
    game.state.recently_moved_cards = Some(vec![niji_card].into());
    game.state.recently_moved_from_zone = Some("stage".to_string());
    game.state.baton_touch_count_p1 = 1;
    game.state.baton_touch_replaced_member_id = Some(niji_card);
    game.state.baton_touch_arriving_card_id = Some(arriving_card);
    // Place arriving card on stage (as it would be after baton touch)
    game.state.player1.stage.stage[0] = arriving_card;

    let _ = game.state.trigger_auto_abilities_for_player("p1");
    let _ = game.state.process_pending_auto_abilities("p1");

    (initial_hand, initial_energy_active)
}

/// Nijigasaki partner with cost >= 10 AND no blade heart → energize 2 fires.
#[test]
fn niji_bp5_partner_cost10_noblade_energizes_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji = game.id("PL!N-bp5-005-R+");
    let partner = game.id("PL!N-bp3-009-P");

    let (_, initial_energy_active) = setup_baton_touch_scenario(&mut game, niji, partner);
    let final_energy_active = game.state.player1.energy_zone.active_count();

    assert!(
        final_energy_active >= initial_energy_active + 2,
        "Partner cost=10 no blade → should activate 2 energy (was {}, now {})",
        initial_energy_active,
        final_energy_active
    );
}

/// Nijigasaki partner with cost >= 15 AND no blade heart → both actions fire.
#[test]
fn niji_bp5_partner_cost15_noblade_energizes_2_and_draws_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji = game.id("PL!N-bp5-005-R+");
    // Partner: 三船栞子 (PL!N-pb1-009-P+, cost=15, QU4RTZ, no blade heart, no auto ability)
    let partner = game.id("PL!N-pb1-009-P+");

    let (initial_hand, initial_energy_active) =
        setup_baton_touch_scenario(&mut game, niji, partner);
    let final_energy_active = game.state.player1.energy_zone.active_count();
    let final_hand = game.state.player1.hand.cards.len();

    assert!(
        final_energy_active >= initial_energy_active + 2,
        "Partner cost=15 no blade → should activate 2 energy (was {}, now {})",
        initial_energy_active,
        final_energy_active
    );
    assert!(
        final_hand >= initial_hand + 1,
        "Partner cost=15 no blade → should draw 1 card (was {}, now {})",
        initial_hand,
        final_hand
    );
}

/// Partner has blade heart → nothing fires.
#[test]
fn niji_bp5_partner_with_blade_heart_nothing_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji = game.id("PL!N-bp5-005-R+");
    let partner = game.id("PL!N-bp1-003-P");

    let (initial_hand, initial_energy_active) =
        setup_baton_touch_scenario(&mut game, niji, partner);
    let final_energy_active = game.state.player1.energy_zone.active_count();
    let final_hand = game.state.player1.hand.cards.len();

    assert_eq!(
        final_energy_active, initial_energy_active,
        "Partner with blade heart → should NOT activate energy"
    );
    assert_eq!(
        final_hand, initial_hand,
        "Partner with blade heart → should NOT draw"
    );
}

/// Partner cost < 10 → nothing fires.
#[test]
fn niji_bp5_partner_low_cost_nothing_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji = game.id("PL!N-bp5-005-R+");
    let partner = game.id("PL!N-bp4-001-P");

    let (initial_hand, initial_energy_active) =
        setup_baton_touch_scenario(&mut game, niji, partner);
    let final_energy_active = game.state.player1.energy_zone.active_count();
    let final_hand = game.state.player1.hand.cards.len();

    assert_eq!(
        final_energy_active, initial_energy_active,
        "Partner cost=2 → should NOT activate energy"
    );
    assert_eq!(final_hand, initial_hand, "Partner cost=2 → should NOT draw");
}

/// Non-Nijigasaki partner → nothing fires.
#[test]
fn niji_bp5_partner_not_nijigasaki_nothing_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let niji = game.id("PL!N-bp5-005-R+");
    let partner = game.id("PL!-sd1-010-SD");

    let (initial_hand, initial_energy_active) =
        setup_baton_touch_scenario(&mut game, niji, partner);
    let final_energy_active = game.state.player1.energy_zone.active_count();
    let final_hand = game.state.player1.hand.cards.len();

    assert_eq!(
        final_energy_active, initial_energy_active,
        "Non-Nijigasaki partner → should NOT activate energy"
    );
    assert_eq!(
        final_hand, initial_hand,
        "Non-Nijigasaki partner → should NOT draw"
    );
}
