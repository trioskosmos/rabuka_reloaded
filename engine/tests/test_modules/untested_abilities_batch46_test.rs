/// Untested-abilities batch 46 — simple gates & all-cards mills.
///
/// - PL!HS-PR-029-PR (ライブ開始時, opt. {E}): gain heart01 until live end.
/// - PL!-bp4-024-L (ライブ開始時): a staged 『μ's』 member gains 1 blade.
/// - PL!HS-bp5-013-N (ライブ開始時): mill 3 from deck top; if ALL are member
///   cards -> +2 blades.
/// - PL!HS-bp6-009-R (ライブ開始時): mill 4; if all are 『蓮ノ空』 cards ->
///   +1 blade.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
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
// PL!HS-PR-029-PR — optional energy -> heart01
// ====================================================================

fn pr0029_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id("PL!HS-PR-029-PR");
    game.state.player1.stage.stage[1] = me;
    me
}

#[test]
fn pr0029_pay_energy_grants_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = pr0029_setup(&mut game);
    let active_before = {
        game.give_energy(3);
        game.state.player1.energy_zone.active_count()
    };

    fire_live_start(&mut game, me);
    assert!(game.has_pending_choice(), "optional energy cost prompted");
    game.select_option(1); // pay

    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart01),
        1,
        "paid -> heart01 until live end"
    );
    assert!(
        game.state.player1.energy_zone.active_count() < active_before,
        "energy was consumed"
    );
}

#[test]
fn pr0029_decline_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = pr0029_setup(&mut game);
    game.give_energy(3);

    fire_live_start(&mut game, me);
    game.select_indices(&[]); // decline

    assert_eq!(
        game.state.mods.get_heart_modifier(me, HeartColor::Heart01),
        0
    );
}

// ====================================================================
// PL!-bp4-024-L — unconditional μ's-member blade
// ====================================================================

#[test]
fn bp4024_mus_member_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let live = game.id("PL!-bp4-024-L");
    game.state.player1.live_card_zone.cards.push(live);
    let mus_mate = game.id("PL!S-sd1-001-SD"); // Aqours — NOT μ's
    game.state.player1.stage.stage[1] = mus_mate;
    let mus_member = game.id("PL!-sd1-010-SD"); // μ's 高坂穂乃果
    game.state.player1.stage.stage[0] = mus_member;

    fire_live_start(&mut game, live);

    assert_eq!(game.state.mods.get_blade_modifier(mus_member), 1);
}

#[test]
fn bp4024_no_mus_member_on_stage_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let live = game.id("PL!-bp4-024-L");
    game.state.player1.live_card_zone.cards.push(live);
    // Only an Aqours member staged.
    let aq = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = aq;

    fire_live_start(&mut game, live);

    assert_eq!(game.state.mods.get_blade_modifier(aq), 0);
}

// ====================================================================
// PL!HS-bp5-013-N / PL!HS-bp6-009-R — all-members / all-Hasunosora mills
// ====================================================================

#[test]
fn bp5013_mill_three_all_members_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!HS-bp5-013-N");
    game.state.player1.stage.stage[1] = me;
    // Deck top: three MEMBER cards.
    let m1 = game.id("PL!N-bp3-006-R");
    let m2 = game.id("PL!SP-bp4-022-N");
    let m3 = game.id("PL!S-sd1-001-SD");
    for m in [m3, m2, m1] {
        game.state.player1.main_deck.cards.insert(0, m);
    }
    let deck_before = game.state.player1.main_deck.cards.len();

    fire_live_start(&mut game, me);

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        2,
        "all three milled cards are members -> +2 blades"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "sanity: deck shrank only by the mill"
    );
}

#[test]
fn bp5013_live_card_among_milled_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!HS-bp5-013-N");
    game.state.player1.stage.stage[1] = me;
    // One of the three is a LIVE card -> not all members.
    let m1 = game.id("PL!-sd1-019-SD"); // live
    let m2 = game.id("PL!N-bp3-006-R");
    let m3 = game.id("PL!S-sd1-001-SD");
    for m in [m3, m2, m1] {
        game.state.player1.main_deck.cards.insert(0, m);
    }

    fire_live_start(&mut game, me);

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "live card among the milled -> condition fails"
    );
}
