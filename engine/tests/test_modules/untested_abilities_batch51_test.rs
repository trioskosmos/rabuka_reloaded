/// Untested-abilities batch 51 — named-member look fetches & score-gated look.
///
/// - PL!N-pb1-016-R 朝香果林 (登場, opt. discard 1): look at top 2; optionally
///   reveal a 「朝香果林」 member to hand, rest to waitroom.
/// - PL!-bp4-006-R (登場): own success-zone score total >= 3 -> look at top
///   5; optionally reveal a 『μ's』 member to hand, rest to waitroom.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

fn fire_debut(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("登場"))
            .unwrap_or_else(|| panic!("card {} lacks a 登場 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Debut,
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
// PL!N-pb1-016-R 朝香果林 — named-member look2 fetch
// ====================================================================

#[test]
fn pb1016_look_two_reveals_karin_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    // Deck top: [karin (member), other].
    let karin = game.id("PL!N-bp1-004-R"); // a DIFFERENT 朝香果林 member
    let other = game.new_id("PL!S-sd1-001-SD");
    game.state.player1.main_deck.cards.insert(0, other);
    game.state.player1.main_deck.cards.insert(0, karin);
    // Waitroom holds a non-Karin card so the optional cost has a target.
    let wr_card = game.new_id("PL!-sd1-010-SD");
    game.state.player1.waitroom.cards.push(wr_card);

    // The ability holder is pb1-016-R herself; stage her directly.
    let me = game.id("PL!N-pb1-016-R");
    game.state.player1.stage.stage[1] = me;
    fire_debut(&mut game, me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.main_deck.cards.contains(&karin),
        "Karin left the deck"
    );
}

// ====================================================================
// PL!-bp4-006-R — success-zone score >=3 gates look5 μ's fetch
// ====================================================================

fn bp4006_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id("PL!-bp4-006-R");
    game.add_to_hand(me);
    game.give_energy(30);
    me
}

#[test]
fn bp4006_score_three_or_more_looks_five_mus_fetch() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = bp4006_setup(&mut game);

    // Success zone: two lives totalling >= 3 score.
    for _ in 0..2 {
        let s = game.id("PL!S-pb1-023-L"); // score 9
        game.state.player1.success_live_card_zone.cards.push(s);
    }
    // Deck top: μ's member among the looked five.
    let mus_member = game.new_id("PL!-sd1-007-SD");
    game.state.player1.main_deck.cards.insert(0, mus_member);

    game.play_to_stage(me, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&mus_member),
        "score gate met -> μ's member fetched to hand"
    );
}

#[test]
fn bp4006_low_score_no_fetch() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = bp4006_setup(&mut game);

    // Score-0 lives -> total 0 < 3.
    let s1 = game.id("PL!HS-bp2-020-L");
    game.state.player1.success_live_card_zone.cards.push(s1);
    let mus_member = game.new_id("PL!-sd1-007-SD");
    game.state.player1.main_deck.cards.insert(0, mus_member);

    game.play_to_stage(me, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&mus_member),
        "score total < 3 -> no look, no fetch"
    );
}
