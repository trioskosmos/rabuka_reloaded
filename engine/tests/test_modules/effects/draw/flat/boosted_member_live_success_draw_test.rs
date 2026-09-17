use crate::helpers::*;
use rabuka_engine::card::HeartColor;

const BOOST: i16 = 2;

fn echoes_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let live = game.id("PL!HS-PR-028-PR");
    game.state.player1.live_card_zone.cards.push(live);
    let member = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = member;
    member
}

fn fire_live_success(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ成功時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn hs_pr_028_pr_live_success_boosted_staged_member_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = echoes_setup(&mut game);

    // Ability-granted hearts count as current (Q172).
    game.state.mods.add_heart_modifier(member, HeartColor::Heart01, BOOST);

    let deck_before = game.state.player1.main_deck.cards.len();
    let live_id = game.id_ref("PL!HS-PR-028-PR");
    fire_live_success(&mut game, live_id);

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "current > original -> draw 1"
    );
}

#[test]
fn hs_pr_028_pr_live_success_unboosted_staged_member_does_not_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let _member = echoes_setup(&mut game); // current == original

    let deck_before = game.state.player1.main_deck.cards.len();
    let live_id = game.id_ref("PL!HS-PR-028-PR");
    fire_live_success(&mut game, live_id);

    assert_eq!(
        deck_before, game.state.player1.main_deck.cards.len(),
        "current == original is not 'more than' -> no draw"
    );
}

#[test]
fn hs_pr_028_pr_live_success_empty_stage_does_not_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let live = game.id("PL!HS-PR-028-PR");
    game.state.player1.live_card_zone.cards.push(live);
    // No members on stage.

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_success(&mut game, live);

    assert_eq!(deck_before, game.state.player1.main_deck.cards.len());
}

#[test]
fn hs_pr_028_pr_live_success_boosted_waitroom_non_member_does_not_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let live = game.id("PL!HS-PR-028-PR");
    game.state.player1.live_card_zone.cards.push(live);
    // Boost applies to a NON-member card sitting in the waitroom — the stage
    // holds an unboosted member.
    let wait_card = game.id("PL!-sd1-019-SD");
    game.state.player1.waitroom.cards.push(wait_card);
    game.state.mods.add_heart_modifier(wait_card, HeartColor::Heart01, BOOST);
    game.state.player1.stage.stage[0] = game.id("PL!S-sd1-001-SD");

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_success(&mut game, live);

    assert_eq!(
        deck_before, game.state.player1.main_deck.cards.len(),
        "boosted non-member must not satisfy the gate"
    );
}
