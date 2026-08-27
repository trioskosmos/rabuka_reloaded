/// Untested-abilities batch 35 — current-vs-original hearts comparison
/// (「元々持つハートの数より多い数のハートを持つメンバー」).
///
/// - PL!HS-PR-028-PR Echoes Beyond (ライブ成功時): any staged member whose
///   current hearts exceed its printed hearts -> draw 1.
/// - PL!HS-pb1-029-L 全方位キュン♡ (ライブ開始時): 1+ boosted 『みらくらぱーく！』
///   member -> draw 1; 2+ -> also reduce this live card's required hearts by
///   two heart00.
///
/// Heart grants use the manual-additive modifier idiom (see
/// modifier_layer_characterization_test); per Q172 ability-granted hearts
/// count toward "current", yell blade-hearts do not.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

const BOOST: i16 = 2;

// ====================================================================
// PL!HS-PR-028-PR Echoes Beyond
// ====================================================================

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
fn pr028_boosted_member_draws() {
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
fn pr028_unboosted_member_no_draw() {
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
fn pr028_empty_stage_no_draw() {
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
fn pr028_boost_on_non_member_does_not_count() {
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

// ====================================================================
// PL!HS-pb1-029-L 全方位キュン♡ — tiered draw + need-heart reduction
// ====================================================================

const MIRAKU_A: &str = "PL!HS-bp1-005-PR"; // みらくらぱーく！ member
const MIRAKU_B: &str = "PL!HS-PR-005-PR"; // みらくらぱーく！ member

fn kyun_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    // Use the TEMPLATE id: trigger processing re-resolves activating_card
    // from the card_no, so the zone copy must be the same instance.
    let live = crate::helpers::card_id(&game.db, "PL!HS-pb1-029-L");
    game.state.player1.live_card_zone.cards.push(live);
    live
}

fn fire_live_start_kyun(game: &mut TestGame, live: i16) {
    let ability_id = {
        let card = game.db.get_card(live).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(live).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        rabuka_engine::core::types::AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(live),
        None,
        None,
    );
    game.state.activating_card = Some(live);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn pb1029_one_boosted_miraku_draws_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = kyun_setup(&mut game);

    let a = game.id(MIRAKU_A);
    game.state.player1.stage.stage[0] = a;
    game.state.mods.add_heart_modifier(a, HeartColor::Heart01, BOOST);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start_kyun(&mut game, live);

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "one boosted みらくらぱーく！ member -> draw 1"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart00),
        0,
        "one member is not '2 or more' -> no need-heart reduction"
    );
}

#[test]
fn pb1029_two_boosted_miraku_draw_and_reduce_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = kyun_setup(&mut game);

    let a = game.id(MIRAKU_A);
    let b = game.id(MIRAKU_B);
    game.state.player1.stage.stage[0] = a;
    game.state.player1.stage.stage[1] = b;
    game.state.mods.add_heart_modifier(a, HeartColor::Heart01, BOOST);
    game.state.mods.add_heart_modifier(b, HeartColor::Heart02, BOOST);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start_kyun(&mut game, live);

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "two boosted members -> draw 1 (not 2)"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart00),
        -2,
        "2+ members -> this live card's heart00 requirement -2"
    );
}

#[test]
fn pb1029_unboosted_miraku_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = kyun_setup(&mut game);

    let a = game.id(MIRAKU_A);
    game.state.player1.stage.stage[0] = a;

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start_kyun(&mut game, live);

    assert_eq!(
        deck_before, game.state.player1.main_deck.cards.len(),
        "みらくらぱーく！ without extra hearts doesn't qualify"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(live, HeartColor::Heart00),
        0
    );
}

#[test]
fn pb1029_boosted_non_miraku_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = kyun_setup(&mut game);

    // μ's member with extra hearts — wrong group.
    let other = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = other;
    game.state.mods.add_heart_modifier(other, HeartColor::Heart01, BOOST);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start_kyun(&mut game, live);

    assert_eq!(deck_before, game.state.player1.main_deck.cards.len());
}
