use crate::helpers::*;

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

fn trigger_live_start_ability(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let live_start_ab = card
        .abilities
        .iter()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .cloned()
        .expect("Card must have LiveStart ability");

    let ability_id = format!("{}_{}", card.card_no, live_start_ab.full_text);
    game.state.trigger_auto_ability(
        ability_id,
        rabuka_engine::core::types::AbilityTrigger::LiveStart,
        game.state.player1.id.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    let pid = game.state.player1.id.clone();
    game.state.process_pending_auto_abilities(&pid);
}

/// Drain all remaining auto-ability prompts.
fn drain_auto_abilities(game: &mut TestGame) {
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            Some("SelectTarget") => {
                game.select_indices(&[]);
            }
            Some("SelectCard") => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
    let pid = game.state.player1.id.clone();
    game.state.process_pending_auto_abilities(&pid);
}

/// The cost is optional & any_number: directly prompts SelectCard.
/// Provide indices to pick cards; empty = skip the optional cost.
fn handle_optional_cost(game: &mut TestGame, indices: &[usize]) {
    if game.has_pending_choice() && game.pending_choice_type().as_deref() == Some("SelectCard") {
        game.select_indices(indices);
    }
    // After selection (if not enough), sequential picks re-prompt.
    // Handle subsequent prompts with remaining indices, then skip leftover.
    let mut remaining = indices.to_vec();
    while game.has_pending_choice() {
        let ct = game.pending_choice_type().unwrap_or_default();
        if ct != "SelectCard" {
            break;
        }
        if remaining.is_empty() {
            game.select_indices(&[]); // skip remaining
        } else {
            let idx = remaining.remove(0);
            game.select_indices(&[idx]);
        }
    }
    drain_auto_abilities(game);
}

// ================================================================
// LL-bp1-001-R+ — ab#1: ライブ開始時
// Cost: optional discard any combination of up to 3 named cards
//   (上原歩夢, 澁谷かのん, 日野下花帆) from hand
// Effect: gain +3 constant score until live end
// ================================================================

#[test]
fn joint_live_start_discard_three_gains_score3() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let joint = game.id("LL-bp1-001-R+");
    let ayumu = game.id("PL!N-bp1-001-R");
    let kanon = game.id("PL!SP-bp1-001-R");
    let kaho = game.id("PL!HS-bp1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, joint, -1];
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(ayumu);
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(kaho);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let hand_before = game.state.player1.hand.cards.len();
    trigger_live_start_ability(&mut game, joint);
    handle_optional_cost(&mut game, &[0, 1, 2]);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 3,
        "3 discarded"
    );
    assert_eq!(game.state.mods.get_score_modifier(joint), 3, "+3 score");
}

#[test]
fn joint_live_start_discard_two_gains_score3() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let joint = game.id("LL-bp1-001-R+");
    let ayumu = game.id("PL!N-bp1-001-R");
    let kanon = game.id("PL!SP-bp1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, joint, -1];
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(ayumu);
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let hand_before = game.state.player1.hand.cards.len();
    trigger_live_start_ability(&mut game, joint);
    handle_optional_cost(&mut game, &[0, 1]);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 2,
        "2 discarded"
    );
    assert_eq!(game.state.mods.get_score_modifier(joint), 3, "+3 score");
}

#[test]
fn joint_live_start_skip_optional_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let joint = game.id("LL-bp1-001-R+");
    let ayumu = game.id("PL!N-bp1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, joint, -1];
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(ayumu);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let hand_before = game.state.player1.hand.cards.len();
    trigger_live_start_ability(&mut game, joint);
    handle_optional_cost(&mut game, &[]); // skip with empty indices

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no discard"
    );
    assert_eq!(game.state.mods.get_score_modifier(joint), 0, "no score");
}

#[test]
fn joint_live_start_no_named_skips_gracefully() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let joint = game.id("LL-bp1-001-R+");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, joint, -1];
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_live_start_ability(&mut game, joint);

    // No matching cards in hand → cost auto-skips, effect should not fire
    let pid = game.state.player1.id.clone();
    game.state.process_pending_auto_abilities(&pid);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "no cards discarded when skipping"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(joint),
        0,
        "no score modifier when cost skipped"
    );
}

#[test]
fn joint_live_start_discard_mixed_named_gains_score3() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let joint = game.id("LL-bp1-001-R+");
    let kanon = game.id("PL!SP-bp1-001-R");
    let kaho = game.id("PL!HS-bp1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, joint, -1];
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(kaho);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let hand_before = game.state.player1.hand.cards.len();
    trigger_live_start_ability(&mut game, joint);
    handle_optional_cost(&mut game, &[0, 1]);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 2,
        "2 mixed discarded"
    );
    assert_eq!(game.state.mods.get_score_modifier(joint), 3, "+3 score");
}
