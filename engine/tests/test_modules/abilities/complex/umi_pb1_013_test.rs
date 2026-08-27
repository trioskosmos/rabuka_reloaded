/// Tests for PL!-pb1-013-R / PL!-pb1-013-P＋ (園田海未 / Umi Sonoda)
/// Q176: The opponent picks from YOUR hand blind (相手は見ないで)
use crate::helpers::*;

#[test]
fn q176_activate_creates_blind_reveal_choice_opponent_controls_pick() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-pb1-013-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = umi;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(3);

    game.activate_ability(umi);
    assert!(
        game.has_pending_choice(),
        "3 cards in hand, count=1 → choice needed"
    );

    let choice = game.state.get_pending_choice().unwrap();
    match choice {
        rabuka_engine::ability::types::Choice::SelectCard {
            count,
            blind,
            is_reveal,
            zone,
            target_player_id,
            ..
        } => {
            assert_eq!(*count, 1, "Should pick 1 card");
            assert!(
                *blind,
                "Should be blind — opponent cannot see card identities"
            );
            assert!(*is_reveal, "Should be a reveal action");
            assert_eq!(zone, "hand", "Zone should be hand");
            assert_eq!(
                target_player_id.as_deref(),
                Some("self"),
                "target_player_id='self' — YOUR hand is revealed by opponent's blind pick"
            );
        }
        _ => panic!("Expected SelectCard, got {:?}", choice),
    }
}

#[test]
fn q176_reveal_live_card_gains_plus1_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-pb1-013-R");
    let live_card = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage[1] = umi;
    game.state.player1.hand.cards.push(live_card);
    game.give_energy(3);

    game.activate_ability(umi);
    // 1 card in hand, count=1 → engine auto-selects, no pending choice
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "Gained 常時 live-total bonus should be +1 when a live card is revealed"
    );
}

#[test]
fn q176_reveal_non_live_card_no_score_modifier() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-pb1-013-R");
    let non_live = game.id("LL-E-001-SD");

    game.state.player1.stage.stage[1] = umi;
    game.state.player1.hand.cards.push(non_live);
    game.give_energy(3);

    game.activate_ability(umi);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        0,
        "No bonus when non-live card is revealed"
    );
}

#[test]
fn q176_use_limit_blocks_second_activation_same_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-pb1-013-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = umi;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(6);

    game.activate_ability(umi);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let result = game.try_activate_ability(umi);
    assert!(
        result.is_err(),
        "Second activation should fail due to use_limit=1"
    );
}

#[test]
fn q176_revealed_card_stays_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-pb1-013-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = umi;
    game.state.player1.hand.cards.push(filler);
    game.give_energy(3);

    game.activate_ability(umi);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.hand.len(),
        1,
        "Revealed card should remain in hand"
    );
    assert!(
        game.state.revealed_cards.contains(&filler),
        "Card should be tracked in revealed_cards"
    );
}

#[test]
fn q176_score_modifier_persists_through_phase_changes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-pb1-013-R");
    let live_card = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage[1] = umi;
    game.state.player1.hand.cards.push(live_card);
    game.give_energy(3);

    game.activate_ability(umi);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    game.pass();
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "Persists through phase changes"
    );
}

#[test]
fn q176_score_modifier_cleared_when_member_leaves_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-pb1-013-R");
    let live_card = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage[1] = umi;
    game.state.player1.hand.cards.push(live_card);
    game.give_energy(3);

    game.activate_ability(umi);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1, "Should have +1");

    game.state.player1.stage.stage[1] = -1;
    game.state.mods.clear_all_for_card(umi);
    game.state.clear_gained_abilities_for_card(umi);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        0,
        "Should be 0 after leaving stage"
    );
}

#[test]
fn q176_opponent_picks_exactly_one_from_many_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-pb1-013-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = umi;
    for _ in 0..5 {
        game.state.player1.hand.cards.push(filler);
    }
    game.give_energy(3);

    game.activate_ability(umi);
    let choice = game.state.get_pending_choice().unwrap();
    match choice {
        rabuka_engine::ability::types::Choice::SelectCard { count, .. } => {
            assert_eq!(*count, 1, "Should pick exactly 1 regardless of hand size");
        }
        _ => panic!("Expected SelectCard"),
    }

    game.select_indices(&[2]);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.player1.hand.len(),
        5,
        "All 5 cards should remain in hand"
    );
}

#[test]
fn q176_choice_path_pick_live_from_mixed_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-pb1-013-R");
    let live = game.id("PL!-sd1-020-SD");
    let non_live = game.id("LL-E-001-SD");

    game.state.player1.stage.stage[1] = umi;
    game.state.player1.hand.cards.push(non_live);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(non_live);
    game.give_energy(3);

    game.activate_ability(umi);
    assert!(game.has_pending_choice(), "3 cards → choice needed");
    game.select_indices(&[1]);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "Pick live card → live-total +1"
    );
    assert!(
        game.state.revealed_cards.contains(&live),
        "Live card should be revealed"
    );
    assert_eq!(
        game.state.player1.hand.len(),
        3,
        "All 3 cards remain in hand"
    );
}

#[test]
fn q176_choice_path_pick_non_live_from_mixed_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-pb1-013-R");
    let live = game.id("PL!-sd1-020-SD");
    let non_live = game.id("LL-E-001-SD");

    game.state.player1.stage.stage[1] = umi;
    game.state.player1.hand.cards.push(non_live);
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(non_live);
    game.give_energy(3);

    game.activate_ability(umi);
    game.select_indices(&[0]);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        0,
        "Pick non-live card → no bonus"
    );
    assert!(
        game.state.revealed_cards.contains(&non_live),
        "Non-live card should be revealed"
    );
}

#[test]
fn q176_picked_card_tracked_in_revealed_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-pb1-013-R");
    let card_a = game.id("PL!-sd1-020-SD");
    let card_b = game.id("LL-E-001-SD");

    game.state.player1.stage.stage[1] = umi;
    game.state.player1.hand.cards.push(card_a);
    game.state.player1.hand.cards.push(card_b);
    game.give_energy(3);

    game.activate_ability(umi);
    game.select_indices(&[1]);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.revealed_cards.contains(&card_b),
        "Picked card_b should be in revealed_cards"
    );
    assert!(
        !game.state.revealed_cards.contains(&card_a),
        "Unpicked card_a should NOT be in revealed_cards"
    );
}
