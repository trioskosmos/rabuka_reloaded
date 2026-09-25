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
fn pl_s_bp3_021_l_live_start_member_to_deck_top_grants_blade() {
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
fn pl_s_bp3_021_l_live_start_declined_recovery_preserves_waitroom_without_blade() {
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
    game.select_indices(&[]);

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
fn pl_s_bp3_021_l_live_start_empty_waitroom_has_no_prompt_or_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let live = game.id("PL!S-bp3-021-L");
    game.state.player1.live_card_zone.cards.push(live);
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[1] = mate;

    fire_live_start(&mut game, live);

    assert!(
        !game.has_pending_choice(),
        "no member in waitroom -> no prompt"
    );
    assert_eq!(game.state.mods.get_blade_modifier(mate), 0);
}
