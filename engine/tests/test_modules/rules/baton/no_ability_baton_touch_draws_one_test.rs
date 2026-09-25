use crate::helpers::*;

/// Baton touch from no-ability member → draw 1.
#[test]
fn chika_bp5_001_baton_touch_from_no_ability_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let no_ability = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-002-SD");

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    game.state.player1.stage.stage = [-1, no_ability, -1];
    game.state.player1.hand.cards.push(chika);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(chika_cost + 5);

    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);

    while game.has_pending_choice() {
        let is_required = game.state.get_pending_choice().is_some_and(|c| {
            matches!(
                c,
                rabuka_engine::ability::types::Choice::SelectCard {
                    count: 1,
                    allow_skip: false,
                    ..
                }
            )
        });
        if is_required {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
    }

    assert_eq!(game.state.player1.stage.stage[1], chika, "Chika at Center");
    assert!(
        game.state.player1.waitroom.cards.contains(&no_ability),
        "No-ability member replaced"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Draw 1 compensates hand loss from playing Chika"
    );
}

/// Baton touch from member WITH ability → NO draw.
#[test]
fn chika_bp5_001_baton_touch_from_ability_member_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let has_ability = game.id("PL!SP-PR-003-PR");
    let filler = game.id("PL!-sd1-002-SD");

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    game.state.player1.stage.stage = [-1, has_ability, -1];
    game.state.player1.hand.cards.push(chika);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(chika_cost + 5);

    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(game.state.player1.stage.stage[1], chika, "Chika at Center");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "No draw — hand decreased"
    );
}

/// Normal debut (empty area, no baton touch) → NO draw.
#[test]
fn chika_bp5_001_normal_debut_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-002-SD");

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(chika);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(chika_cost + 5);

    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(game.state.player1.stage.stage[1], chika, "Chika at Center");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "No draw — hand decreased"
    );
}

/// Baton touch from no-ability member with empty deck → refresh then draw 1.
#[test]
fn chika_bp5_001_baton_touch_draw_triggers_refresh() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let no_ability = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-002-SD");

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    game.state.player1.stage.stage = [-1, no_ability, -1];
    game.state.player1.hand.cards.push(chika);
    // Empty deck + some cards in waitroom to refresh
    game.state.player1.main_deck.cards.clear();
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.give_energy(chika_cost + 5);

    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);

    while game.has_pending_choice() {
        let is_required = game.state.get_pending_choice().is_some_and(|c| {
            matches!(
                c,
                rabuka_engine::ability::types::Choice::SelectCard {
                    count: 1,
                    allow_skip: false,
                    ..
                }
            )
        });
        if is_required {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
    }

    assert_eq!(game.state.player1.stage.stage[1], chika, "Chika at Center");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Draw 1 from refreshed deck compensates hand loss"
    );
    assert!(
        game.state.player1.main_deck.cards.len() > 0 || game.state.player1.waitroom.cards.len() > 0,
        "Refresh should have occurred (cards exist somewhere)"
    );
}
