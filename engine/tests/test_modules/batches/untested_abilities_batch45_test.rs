/// Untested-abilities batch 45 — named grants, unit-count retrieval, choice
/// heart grants, look-and-fetch.
///
/// - PL!SP-bp7-025-L (ライブ開始時): staged 「嵐千砂都」 gains 1 blade until
///   live end.
/// - PL!SP-bp7-019-N (登場): with ≥3 staged 『5yncri5e!』 members, retrieve a
///   live card from the waitroom.
/// - PL!N-sd2-005-SD2 (ライブ開始時, opt. discard 2 hand): choose a heart
///   color -> gain 2 of it until live end.
/// - PL!N-sd2-009-SD2 (登場): look at top 3; optionally reveal a 『虹ヶ咲』
///   card to hand, rest to waitroom.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

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
// PL!SP-bp7-025-L — named-member blade grant
// ====================================================================

#[test]
fn bp7025_staged_chisato_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let holder = game.id("PL!SP-bp7-025-L"); // the live card itself
    game.state.player1.live_card_zone.cards.push(holder);
    // Chisato is on stage and receives the blade. Her stored name is
    // "嵐 千砂都" (with a space), so pin the card number directly.
    let chisato_card = game.id("PL!SP-pb1-014-PR");
    game.state.player1.stage.stage[1] = chisato_card;
    let other = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = other;

    fire_live_start(&mut game, holder);

    assert_eq!(
        game.state.mods.get_blade_modifier(chisato_card),
        1,
        "staged Chisato gains 1 blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(other),
        0,
        "other members gain nothing"
    );
}

// ====================================================================
// PL!SP-bp7-019-N — 5yncri5e! x3 gates live-card retrieval
// ====================================================================

#[test]
fn bp7019_three_5ync_retrieves_live_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp7-019-N");
    game.add_to_hand(me);
    game.give_energy(30);

    // Two other 5yncri5e! members pre-staged; me debuting makes three.
    let s1 = game.id("PL!SP-PR-005-PR");
    let s2 = game.id("PL!SP-PR-008-PR");
    game.state.player1.stage.stage[0] = s1;
    game.state.player1.stage.stage[1] = s2;
    // A live card waits for retrieval.
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);

    game.play_to_stage(me, MemberArea::RightSide);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "3x 5yncri5e! staged -> live card retrieved to hand"
    );
}

#[test]
fn bp7019_only_two_5ync_no_retrieval() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp7-019-N");
    game.add_to_hand(me);
    game.give_energy(30);

    let s1 = game.id("PL!SP-PR-005-PR");
    game.state.player1.stage.stage[0] = s1;
    let outsider = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = outsider;
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);

    game.play_to_stage(me, MemberArea::RightSide);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&mus_live),
        "only 2x 5yncri5e! -> no retrieval"
    );
}
