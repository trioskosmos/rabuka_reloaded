/// Untested-abilities batch 49 — cost-threshold blades, binary choice, draw+mill.
///
/// - PL!HS-cl1-010-CL (ライブ開始時): staged 『蓮ノ空』 member with cost >= 10
///   gains +2 blades until live end.
/// - PL!HS-cl1-004-CL (登場): binary choice — mill 3 from deck top OR wait
///   an enemy member with cost <= 2.
/// - PL!HS-bp6-030-L (ライブ開始時): draw 1, then mill 1 from deck top.
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
// PL!HS-cl1-010-CL — cost>=10 Hasunosora member +2 blades
// ====================================================================

#[test]
fn cl1010_expensive_member_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let live = game.id("PL!HS-cl1-010-CL");
    game.state.player1.live_card_zone.cards.push(live);
    // 百生吟子 cost 15 >= 10, staged.
    let big = game.id("PL!HS-bp5-004-R");
    game.state.player1.stage.stage[1] = big;

    fire_live_start(&mut game, live);

    assert_eq!(
        game.state.mods.get_blade_modifier(big),
        2,
        "cost-15 Hasunosora member gains +2 blades"
    );
}

#[test]
fn cl1010_cheap_member_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let live = game.id("PL!HS-cl1-010-CL");
    game.state.player1.live_card_zone.cards.push(live);
    // 村野さやか DOLLCHESTRA cost 5 < 10 — below the threshold.
    let low = game.id("PL!HS-cl1-002-CL");
    game.state.player1.stage.stage[1] = low;

    fire_live_start(&mut game, live);

    assert_eq!(
        game.state.mods.get_blade_modifier(low),
        0,
        "cost-5 member does not meet the >=10 threshold"
    );
}

// ====================================================================
// PL!HS-cl1-004-CL — binary choice debut
// ====================================================================

fn cl1004_debut(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.new_id("PL!HS-cl1-004-CL");
    game.add_to_hand(me);
    game.give_energy(20);
    game.play_to_stage(me, MemberArea::LeftSide);
    me
}

#[test]
fn cl1004_choose_mill_three() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let _me = cl1004_debut(&mut game);

    let deck_before = game.state.player1.main_deck.cards.len();
    game.select_option(0); // first option: mill 3

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "chose mill -> deck top 3 moved to waitroom"
    );
}

#[test]
fn cl1004_choose_wait_enemy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = cl1004_debut(&mut game);
    let _ = me;

    // A cheap enemy member on the opponent's stage.
    let enemy = game.id("PL!SP-PR-010-PR"); // cost 2
    game.state.player2.stage.stage[1] = enemy;

    game.select_option(1); // second option: wait an enemy cost<=2 member

    assert_eq!(
        game.state.mods.get_orientation_modifier(enemy),
        Some("wait"),
        "chose wait -> enemy cost<=2 member waited"
    );
}

// ====================================================================
// PL!HS-bp6-030-L — draw 1 then mill 1
// ====================================================================

#[test]
fn bp6030_draw_one_discard_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let live = game.id("PL!HS-bp6-030-L");
    game.state.player1.live_card_zone.cards.push(live);

    let drawn = game.new_id("PL!S-sd1-001-SD");
    game.state.player1.main_deck.cards.insert(0, drawn);
    // Hand starts EMPTY: the drawn card is then the only discard candidate,
    // making the end state fully deterministic.

    fire_live_start(&mut game, live);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&drawn),
        "the drawn card was immediately paid back to the waitroom"
    );
    assert_eq!(game.state.player1.hand.cards.len(), 0);
    assert!(
        game.state.player1.waitroom.cards.contains(&drawn),
        "discarded card lands in the waitroom"
    );
}
