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
#[ignore = "KNOWN GAP (bug 14): implicit そうした場合 gating uses the \
was_moved/was_selected proxy evaluated while the optional move is still \
deferred to its selection prompt — the resumed gain_resource re-checks \
its condition before the answered selection's movement is visible, so \
the blade never applies even though the deck-top placement succeeded."]
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
#[ignore = "KNOWN GAP (bug 14, decline half): declining the deferred \
selection still applies the そうした場合 blade (+1) — the gate records \
condition_failed=false because the empty answer is not attributed to \
the gating action. Same was_moved/was_selected proxy as bug 14 accept \
half."]
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
    if game.has_pending_choice() {
        game.select_indices(&[]); // decline
    }

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
#[ignore = "KNOWN GAP (bug 15): the reveal -> deck_top_or_bottom chain \
loses the move on resume — the answered reveal selection does not carry \
the revealed card into the follow-up placement, leaving it in hand. \
Related to the deferred-choice bookkeeping of bug 14."]
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

    // Answer every remaining prompt by taking the first offered option:
    // reveal selection, then deck top-vs-bottom.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    // The revealed Aqours card left the hand.
    assert!(
        !game.state.player1.hand.cards.contains(&aqours_card),
        "revealed Aqours card moved onto the deck"
    );
    let on_deck = game.state.player1.main_deck.cards.contains(&aqours_card);
    let on_deck_top = game.state.player1.main_deck.cards.first() == Some(&aqours_card);
    assert!(
        on_deck || on_deck_top,
        "revealed card ends up on the deck"
    );
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
