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

#[test]
fn pl_hs_bp5_013_n_mill_three_all_members_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id("PL!HS-bp5-013-N");
    game.state.player1.stage.stage[1] = me;
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
fn pl_hs_bp5_013_n_milled_live_card_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id("PL!HS-bp5-013-N");
    game.state.player1.stage.stage[1] = me;
    let m1 = game.id("PL!-sd1-019-SD");
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
