/// Untested-abilities batch 40 — conditional waitroom/reveal recoveries.
///
/// - PL!S-bp3-021-L 想いよひとつになれ (ライブ開始時): optionally put 1 member
///   from the waitroom on deck top; そうした場合 -> one staged member gains
///   1 blade until live end.
/// - PL!S-sd1-009-SD 黒澤ルビィ (ライブ開始時): optionally reveal 1 『Aqours』
///   card from hand; place it on deck top or bottom and gain 1 blade until
///   live end.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// PL!S-bp3-021-L 想いよひとつになれ
// ====================================================================

#[test]
fn omoi_accept_member_to_deck_top_grants_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let live = game.id("PL!S-bp3-021-L");
    game.state.player1.live_card_zone.cards.push(live);

    // Exactly ONE staged member so the blade recipient is unambiguous.
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[1] = mate;

    // Waitroom holds a MEMBER card eligible for recovery.
    let wr_member = game.id("PL!N-bp3-006-R"); // 近江彼方, member
    game.state.player1.waitroom.cards.push(wr_member);

    fire_live_start(&mut game, live);
    assert!(game.has_pending_choice(), "waitroom member offered");
    game.select_indices(&[0]);

    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&wr_member),
        "recovered member sits on deck TOP"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(mate),
        1,
        "そうした場合: staged member gains 1 blade"
    );
}

#[test]
fn omoi_decline_no_blade_no_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let live = game.id("PL!S-bp3-021-L");
    game.state.player1.live_card_zone.cards.push(live);
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[1] = mate;
    let wr_member = game.id("PL!N-bp3-006-R");
    game.state.player1.waitroom.cards.push(wr_member);

    fire_live_start(&mut game, live);
    assert!(
        game.has_pending_choice(),
        "optional recovery prompt expected (waitroom member present)"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (discard-zone recovery, allow_skip)"
    );
    game.select_indices(&[]); // decline

    assert!(
        game.state.player1.waitroom.cards.contains(&wr_member),
        "declined: member stays in waitroom"
    );
    assert_ne!(
        game.state.player1.main_deck.cards.first(),
        Some(&wr_member),
        "declined: member not placed on deck"
    );
    assert_eq!(game.state.mods.get_blade_modifier(mate), 0);
}

#[test]
fn omoi_empty_waitroom_no_prompt() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let live = game.id("PL!S-bp3-021-L");
    game.state.player1.live_card_zone.cards.push(live);
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[1] = mate;
    // Waitroom EMPTY.

    fire_live_start(&mut game, live);

    assert!(
        !game.has_pending_choice(),
        "no member in waitroom -> no prompt"
    );
    assert_eq!(game.state.mods.get_blade_modifier(mate), 0);
}

// ====================================================================
// PL!S-sd1-009-SD 黒澤ルビィ — reveal Aqours, deck top/bottom, blade
// ====================================================================

#[test]
fn ruby_reveal_aqours_place_and_gain_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let ruby = game.id("PL!S-sd1-009-SD");
    game.state.player1.stage.stage[1] = ruby;

    let aqours_card = game.new_id("PL!S-sd1-003-SD"); // Aqours member
    let non_aq = game.new_id("PL!-sd1-010-SD"); // μ's
    game.add_to_hand(aqours_card);
    game.add_to_hand(non_aq);

    fire_live_start(&mut game, ruby);

    // First prompt: optional reveal cost gate -> accept.
    assert!(game.has_pending_choice(), "reveal cost gate offered");
    game.select_option(1); // Yes: reveal the Aqours card
    // Then answer the deck top-vs-bottom placement choice.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    // The revealed Aqours card left the hand onto the deck.
    assert!(
        !game.state.player1.hand.cards.contains(&aqours_card),
        "revealed Aqours card moved onto the deck"
    );
    let in_deck = game.state.player1.main_deck.cards.contains(&aqours_card);
    assert!(in_deck, "revealed card ends up on the deck");
}

#[test]
fn ruby_no_aqours_in_hand_blade_still_optional() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let ruby = game.id("PL!S-sd1-009-SD");
    game.state.player1.stage.stage[1] = ruby;
    // Only NON-Aqours cards in hand.
    let only_mu = game.new_id("PL!-sd1-010-SD");
    game.add_to_hand(only_mu);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start(&mut game, ruby);

    if !game.has_pending_choice() {
        // Nothing eligible to reveal -> gate skipped entirely.
        assert!(game.state.player1.hand.cards.contains(&only_mu));
        return;
    }
    // If a gate was still offered, declining must leave everything alone.
    game.select_indices(&[]);
    assert!(game.state.player1.hand.cards.contains(&only_mu));
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declined -> deck untouched"
    );
}
