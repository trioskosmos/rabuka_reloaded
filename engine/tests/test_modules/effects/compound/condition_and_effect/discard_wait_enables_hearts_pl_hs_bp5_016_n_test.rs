use crate::helpers::*;
use rabuka_engine::card::HeartColor;
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
fn pl_hs_bp5_016_n_discard_waits_opponent_and_enables_constant_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    let discard_target = game.id("PL!-sd1-019-SD");
    let victim_a = game.id("PL!S-bp2-002-R");
    let victim_b = game.id("PL!-sd1-010-SD");
    let pricey = game.id("PL!N-bp7-003-R\u{ff0b}");
    assert_ne!(victim_a, victim_b);
    assert_ne!(victim_a, pricey);
    assert_ne!(victim_b, pricey);
    assert_eq!(game.db.get_card(card).unwrap().cost, Some(9));
    assert_eq!(game.db.get_card(victim_a).unwrap().cost, Some(4));
    assert_eq!(game.db.get_card(pricey).unwrap().cost, Some(15));
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player2.stage.stage = [victim_a, victim_b, pricey];
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(discard_target);
    game.play_to_stage(card, MemberArea::Center);
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0, 1]);
    assert!(!game.has_pending_choice());

    assert!(!game.state.player1.hand.cards.contains(&discard_target));
    assert!(game.state.player1.waitroom.cards.contains(&discard_target));
    assert_eq!(
        game.state.mods.get_orientation_modifier(victim_a),
        Some("wait")
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(victim_b),
        Some("wait")
    );
    assert_eq!(game.state.mods.get_orientation_modifier(pricey), None);
    assert_eq!(game.state.player2.stage.stage, [victim_a, victim_b, pricey]);
    assert_eq!(
        game.state.mods.get_heart_modifier(card, HeartColor::Heart06),
        1
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(victim_a, HeartColor::Heart06),
        0
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(pricey, HeartColor::Heart06),
        0
    );
}

#[test]
fn pl_hs_bp5_016_n_decline_without_waited_opponents_no_constant_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
    let h06 = game
        .state
        .mods
        .get_heart_modifier(card, HeartColor::Heart06);
    assert_eq!(
        h06, 0,
        "Should get 0 heart06 without opponent wait members (got {})",
        h06
    );
}

#[test]
fn pl_hs_bp5_016_n_single_eligible_victim_grants_no_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    let discard_target = game.id("PL!-sd1-019-SD");
    let victim = game.id("PL!S-bp2-002-R");
    let pricey = game.id("PL!N-bp7-003-R\u{ff0b}");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player2.stage.stage = [victim, pricey, -1];
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(discard_target);
    game.play_to_stage(card, MemberArea::Center);
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());

    assert_eq!(
        game.state.mods.get_orientation_modifier(victim),
        Some("wait")
    );
    assert_eq!(game.state.mods.get_orientation_modifier(pricey), None);
    assert_eq!(
        game.state.mods.get_heart_modifier(card, HeartColor::Heart06),
        0
    );
}

#[test]
fn pl_hs_bp5_016_n_waits_at_most_two_of_three_eligible_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    let discard_target = game.id("PL!-sd1-019-SD");
    let victim_a = game.id("PL!S-bp2-002-R");
    let victim_b = game.new_id("PL!S-bp2-002-R");
    let victim_c = game.id("PL!-sd1-010-SD");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player2.stage.stage = [victim_a, victim_b, victim_c];
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(discard_target);
    game.play_to_stage(card, MemberArea::Center);
    game.select_indices(&[0]);
    assert!(game.has_pending_choice());
    game.select_indices(&[0, 1]);
    assert!(!game.has_pending_choice());

    assert_eq!(
        game.state.mods.get_orientation_modifier(victim_a),
        Some("wait")
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(victim_b),
        Some("wait")
    );
    assert_eq!(game.state.mods.get_orientation_modifier(victim_c), None);
    assert_eq!(
        game.state.mods.get_heart_modifier(card, HeartColor::Heart06),
        1
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(victim_c, HeartColor::Heart06),
        0
    );
}
