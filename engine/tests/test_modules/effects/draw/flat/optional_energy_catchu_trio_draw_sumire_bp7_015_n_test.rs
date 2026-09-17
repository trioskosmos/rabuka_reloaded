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
fn sumire_bp7_015_n_three_catchu_members_and_paid_energy_draw_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let sumire = game.id("PL!SP-bp7-015-N");
    game.state.player1.stage.stage[0] = sumire;
    let c1 = game.id("PL!SP-PR-003-PR");
    let c2 = game.id("PL!SP-PR-006-PR");
    game.state.player1.stage.stage[1] = c1;
    game.state.player1.stage.stage[2] = c2;

    let deck_before = game.state.player1.main_deck.cards.len();
    game.give_energy(5);
    fire_live_start(&mut game, sumire);
    assert!(game.has_pending_choice(), "optional energy cost prompted");
    assert!(
        game.pending_choice_type().is_some(),
        "energy cost prompt must carry a choice identity"
    );
    game.select_option(1);

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "3 CatChu! members staged + paid -> draw 1"
    );
}

#[test]
fn sumire_bp7_015_n_only_two_catchu_members_do_not_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let sumire = game.id("PL!SP-bp7-015-N");
    game.state.player1.stage.stage[0] = sumire;
    let c1 = game.id("PL!SP-PR-003-PR");
    game.state.player1.stage.stage[1] = c1;
    let mu_member = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[2] = mu_member;

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_live_start(&mut game, sumire);

    assert_eq!(
        deck_before,
        game.state.player1.main_deck.cards.len(),
        "only 2 CatChu! members -> no draw"
    );
}
