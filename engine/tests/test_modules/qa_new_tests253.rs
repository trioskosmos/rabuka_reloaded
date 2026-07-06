use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn setup_p1_deck(game: &mut TestGame, live_ids: &[i16]) {
    let filler = game.id_ref("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for (i, &id) in live_ids.iter().enumerate() {
        game.state.player1.main_deck.cards.insert(1 + i, id);
    }
}

/// GALAXY with no Kanan — live card in yell → +1.
#[test]
fn q253_galaxy_gets_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let galaxy = game.id("PL!S-bp6-023-L");
    let yell_live = game.id("PL!-sd1-020-SD");

    game.add_to_stage(MemberArea::LeftSide, game.id("PL!S-bp2-017-N"));
    game.add_to_stage(MemberArea::Center, game.id("PL!S-bp3-015-N"));
    game.add_to_stage(MemberArea::RightSide, game.id("PL!S-PR-014-PR"));
    setup_p1_deck(&mut game, &[yell_live]);

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(galaxy);
    game.set_live_card(galaxy);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(galaxy),
        0,
        "LiveSuccess score bonus cleared after live"
    );
    let l = &game.state.performance_snapshots[0].lives[0];
    assert_eq!(l.score - l.base_score, 1, "bonus in final score");
}

/// Kanan takes the only live card from yell → GALAXY gets nothing (Q253 ruling).
#[test]
fn q253_kanan_first_galaxy_gets_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let galaxy = game.id("PL!S-bp6-023-L");
    let yell_live = game.id("PL!-sd1-020-SD");

    game.add_to_stage(MemberArea::LeftSide, game.id("PL!S-pb1-003-R"));
    game.add_to_stage(MemberArea::Center, game.id("PL!S-bp2-017-N"));
    game.add_to_stage(MemberArea::RightSide, game.id("PL!S-bp3-015-N"));
    setup_p1_deck(&mut game, &[yell_live]);

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(galaxy);
    game.set_live_card(galaxy);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // First performance phase — yell, then Kanan's LiveSuccess
    game.pass();
    // Process all pending choices: auto-skip yell-triggered abilities,
    // but explicitly select a card for Kanan's non-optional move.
    while game.has_pending_choice() {
        let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
        let has_skip = actions
            .iter()
            .any(|a| a.action_type == ActionType::ChoiceSkip);
        if !has_skip
            && actions
                .iter()
                .any(|a| a.action_type == ActionType::ChoiceSelect)
        {
            game.select_indices(&[0]);
            break;
        }
        game.select_indices(&[]);
    }
    // Consume any remaining choices from performance phase
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // Second performance phase
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // Victory determination
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(galaxy),
        0,
        "Q253: Kanan took the live card from yell, GALAXY should get +0"
    );
}

/// Two live cards in yell — Kanan takes one, GALAXY still has one → +1.
#[test]
fn q253_both_succeed_two_live_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let galaxy = game.id("PL!S-bp6-023-L");
    let yell_live_1 = game.id("PL!-sd1-020-SD");
    let yell_live_2 = game.id("PL!-sd1-021-SD");

    game.add_to_stage(MemberArea::LeftSide, game.id("PL!S-pb1-003-R"));
    game.add_to_stage(MemberArea::Center, game.id("PL!S-bp2-017-N"));
    game.add_to_stage(MemberArea::RightSide, game.id("PL!S-bp3-015-N"));
    setup_p1_deck(&mut game, &[yell_live_1, yell_live_2]);

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(galaxy);
    game.set_live_card(galaxy);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // First performance phase — yell, then Kanan's LiveSuccess
    game.pass();
    // Process all pending choices: auto-skip yell-triggered abilities,
    // but explicitly select a card for Kanan's non-optional move.
    while game.has_pending_choice() {
        let actions = rabuka_engine::game_setup::generate_possible_actions(&game.state);
        let has_skip = actions
            .iter()
            .any(|a| a.action_type == ActionType::ChoiceSkip);
        if !has_skip
            && actions
                .iter()
                .any(|a| a.action_type == ActionType::ChoiceSelect)
        {
            game.select_indices(&[0]);
            break;
        }
        game.select_indices(&[]);
    }
    // Consume any remaining choices from performance phase
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // Second performance phase
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    // Victory determination
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(game.state.mods.get_score_modifier(galaxy), 1);
}
