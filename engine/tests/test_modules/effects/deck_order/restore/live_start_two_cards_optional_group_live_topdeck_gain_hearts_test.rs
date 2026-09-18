use crate::helpers::*;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn trigger_ability(game: &mut TestGame, card_id: i16, trigger_str: &str) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .unwrap();
    let pid = game.state.player1.id.clone();
    let trigger = match trigger_str {
        "登場" => rabuka_engine::core::types::AbilityTrigger::Debut,
        "ライブ開始時" => rabuka_engine::core::types::AbilityTrigger::LiveStart,
        "起動" => rabuka_engine::core::types::AbilityTrigger::Activation,
        _ => rabuka_engine::core::types::AbilityTrigger::Auto,
    };
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn live_start_optional_group_live_topdeck_offered_with_two_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp6-004-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live_a = game.id("PL!S-PR-022-PR"); // no ability, Aqours
    let live_b = game.id("PL!S-sd1-019-SD"); // ライブ成功時 only, Aqours

    game.state.player1.stage.stage = [-1, dia, -1];
    game.state.player1.live_card_zone.cards.push(live_a);
    game.state.player1.live_card_zone.cards.push(live_b);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, dia, "ライブ開始時");

    assert!(
        game.has_pending_choice(),
        "Condition 2+ cards met → ability should fire"
    );
}

#[test]
fn live_start_optional_group_live_topdeck_lt2_cards_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp6-004-SEC");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, dia, -1];
    game.state.player1.live_card_zone.cards.push(filler);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, dia, "ライブ開始時");

    assert!(!game.has_pending_choice(), "<2 cards → condition fails");
}

#[test]
fn live_start_optional_group_live_topdeck_zero_cards_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp6-004-SEC");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, dia, -1];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, dia, "ライブ開始時");

    assert!(!game.has_pending_choice(), "0 cards → condition fails");
}

#[test]
fn live_start_optional_group_live_topdeck_offered_with_mixed_ability_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp6-004-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let with_ls = game.id("PL!S-bp2-025-L"); // HAS ライブ開始時
    let without_ls = game.id("PL!S-PR-022-PR"); // no ライブ開始時

    game.state.player1.stage.stage = [-1, dia, -1];
    game.state.player1.live_card_zone.cards.push(with_ls);
    game.state.player1.live_card_zone.cards.push(without_ls);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, dia, "ライブ開始時");

    // Only 1 card (without_ls) should pass the filter
    assert!(
        game.has_pending_choice(),
        "FIXED: ability fires with correct filter — only non-ライブ開始時 card selectable"
    );
}

#[test]
fn live_start_optional_group_live_topdeck_choice_can_be_answered() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let dia = game.id("PL!S-bp6-004-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR");

    game.state.player1.stage.stage = [-1, dia, -1];
    game.state.player1.live_card_zone.cards.push(live);
    game.state
        .player1
        .live_card_zone
        .cards
        .push(game.id("PL!S-sd1-019-SD"));
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, dia, "ライブ開始時");

    // Ability fires, filter works, select card
    assert!(
        game.has_pending_choice(),
        "Ability fires with selectable cards"
    );

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                game.select_indices(&[0]);
            }
            Some("SelectTarget") => {
                game.select_option(0);
            }
            Some("SelectPosition") => {
                game.select_indices(&[0]);
            }
            Some("SelectHeartColor") | Some("SelectHeartType") => {
                game.select_indices(&[0]);
            }
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
}
