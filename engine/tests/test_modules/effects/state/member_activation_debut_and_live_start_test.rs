use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
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

fn advance_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

#[test]
fn bp3_005_debut_activates_all_waited_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp3-005-R");
    let mate_a = game.id("PL!-sd1-010-SD");
    let mate_b = game.new_id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(20);
    fill_decks(&mut game, filler);

    // Two teammates on stage, both waited.
    // Direct placement: there is no natural action that waits an own member
    // here, and the ability under test needs waited targets.
    game.add_to_stage(MemberArea::LeftSide, mate_a);
    game.add_to_stage(MemberArea::Center, mate_b);
    game.state.mods.add_orientation_modifier(mate_a, "wait");
    game.state.mods.add_orientation_modifier(mate_b, "wait");

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::RightSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_orientation_modifier(mate_a),
        Some("active"),
        "waited teammate A should be activated by the debut"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(mate_b),
        Some("active"),
        "waited teammate B should be activated by the debut"
    );
}

#[test]
fn combo_bp3_005_mass_activate_readies_self_waited_ability_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mass = game.id("PL!-bp3-005-R"); // cost 4
    let kanata = game.id("PL!N-pb1-006-R"); // cost 9, 起動: wait self → +1 energy
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(25);
    fill_decks(&mut game, filler);

    game.state.player1.hand.cards.push(kanata);
    game.play_to_stage(kanata, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Kanata's 起動: waits herself, activates 1 energy.
    let energy_before = game.state.player1.energy_zone.active_count();
    game.activate_ability(kanata);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(kanata),
        Some("wait"),
        "cost: kanata should be waited"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before + 1,
        "effect: one energy activated"
    );

    // Play bp3-005 — its debut activates ALL members incl. kanata.
    game.state.player1.hand.cards.push(mass);
    game.play_to_stage(mass, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_orientation_modifier(kanata),
        Some("active"),
        "bp3-005 debut should re-activate the self-waited kanata"
    );
    // Kanata can immediately be used again (no turn limit on her ability).
    let e2 = game.state.player1.energy_zone.active_count();
    game.activate_ability(kanata);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        e2 + 1,
        "reactivated kanata should be able to activate again"
    );
}

#[test]
fn bp3_001_live_start_activates_one_chosen_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp3-001-R"); // cost 13, carries the live-start too
    let sleeper = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.give_energy(15);
    fill_decks(&mut game, filler);

    // A waited teammate for the ability to wake up.
    game.add_to_stage(MemberArea::LeftSide, sleeper);
    game.state.mods.add_orientation_modifier(sleeper, "wait");

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);

    // Two passes out of LiveCardSet: P1 refills +1 (the set live), then
    // ライブ開始時 fires. Activate-one draws nothing.
    let hand_before = game.state.player1.hand.cards.len();
    let p1_zone = game.state.player1.live_card_zone.cards.len();
    game.pass();
    game.pass();

    // The live-start fires via the auto queue: accept the ability, then
    // choose WHICH member to activate ("1人まで" → SelectCard, skippable).
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectCard") => {
                assert!(
                    matches!(
                        game.get_pending_choice(),
                        Choice::SelectCard { ref zone, allow_skip, .. }
                            if zone == "stage" && *allow_skip,
                    ),
                    "expected skippable stage SelectCard, got {}",
                    game.pending_choice_summary()
                );
                game.select_indices(&[0]);
            }
            _ => game.select_indices(&[]),
        }
    }

    assert_eq!(
        game.state.mods.get_orientation_modifier(sleeper),
        Some("active"),
        "live-start should activate the chosen waited member"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + p1_zone,
        "activate-one draws nothing beyond the live-zone refill"
    );
}

#[test]
fn bp3_001_live_start_can_skip_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp3-001-R");
    let sleeper = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.give_energy(15);
    fill_decks(&mut game, filler);

    game.add_to_stage(MemberArea::LeftSide, sleeper);
    game.state.mods.add_orientation_modifier(sleeper, "wait");

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    // Decline every optional prompt ("1人まで" = may choose zero).
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
    }

    assert_eq!(
        game.state.mods.get_orientation_modifier(sleeper),
        Some("wait"),
        "declined activation: member must stay waited"
    );
}
