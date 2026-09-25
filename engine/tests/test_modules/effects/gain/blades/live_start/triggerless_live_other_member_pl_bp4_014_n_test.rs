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

fn rin_pl_bp4_014_n_setup(game: &mut TestGame) -> (i16, i16) {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let rin = game.id("PL!-bp4-014-N");
    game.state.player1.stage.stage[1] = rin;
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = mate;
    (rin, mate)
}

#[test]
fn pl_bp4_014_n_live_start_triggerless_live_grants_other_member_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (rin, mate) = rin_pl_bp4_014_n_setup(&mut game);
    let plain_live = game.id("PL!-sd1-020-SD");
    game.state.player1.live_card_zone.cards.push(plain_live);

    fire_live_start(&mut game, rin);

    assert_eq!(
        game.state.mods.get_blade_modifier(mate),
        2,
        "live card without LS/LSS abilities present -> other member +2 blades"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(rin),
        0,
        "exclude_self: Rin never boosts herself"
    );
}

#[test]
fn pl_bp4_014_n_live_start_all_lives_have_triggers_grants_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (rin, mate) = rin_pl_bp4_014_n_setup(&mut game);
    let triggered = game.id("PL!N-sd2-007-P");
    game.state.player1.live_card_zone.cards.push(triggered);

    fire_live_start(&mut game, rin);

    assert_eq!(
        game.state.mods.get_blade_modifier(mate),
        0,
        "every live card has LS/LSS -> gate fails"
    );
}

#[test]
fn pl_bp4_014_n_live_start_empty_live_zone_grants_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (rin, mate) = rin_pl_bp4_014_n_setup(&mut game);

    fire_live_start(&mut game, rin);

    assert_eq!(game.state.mods.get_blade_modifier(mate), 0);
}
