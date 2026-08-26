use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card.resolved_abilities().find(|a| a.triggers.as_deref() == Some("ライブ開始時")).unwrap();
    let ability_id = format!("{}_{}", card.card_no, ab.full_text);
    let card_no = card.card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(ability_id, AbilityTrigger::LiveStart, pid.clone(), Some(card_no), Some(cid), None, None);
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn rin_multiple_plain_lives_still_grants() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rin = game.id("PL!-bp4-014-N");
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage = [mate, rin, -1];
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let plain1 = game.id("PL!-sd1-020-SD");
    let plain2 = game.id("PL!-sd1-020-SD");
    game.state.player1.live_card_zone.cards.push(plain1);
    game.state.player1.live_card_zone.cards.push(plain2);
    fire_live_start(&mut game, rin);
    assert_eq!(game.state.mods.get_blade_modifier(mate), 2);
}

#[test]
fn rin_mixed_lives_plain_present_grants() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rin = game.id("PL!-bp4-014-N");
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage = [mate, rin, -1];
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let plain = game.id("PL!-sd1-020-SD");
    let with_ability = game.id("PL!N-sd2-007-P");
    game.state.player1.live_card_zone.cards.push(plain);
    game.state.player1.live_card_zone.cards.push(with_ability);
    fire_live_start(&mut game, rin);
    assert_eq!(game.state.mods.get_blade_modifier(mate), 2, "one plain live is enough");
}

#[test]
fn rin_exclude_self_with_only_rin_and_mate() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rin = game.id("PL!-bp4-014-N");
    // Only rin and one mate, plain live present -> mate gets blade, rin does not
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage = [mate, rin, -1];
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let plain = game.id("PL!-sd1-020-SD");
    game.state.player1.live_card_zone.cards.push(plain);
    fire_live_start(&mut game, rin);
    assert_eq!(game.state.mods.get_blade_modifier(mate), 2);
    assert_eq!(game.state.mods.get_blade_modifier(rin), 0, "rin excluded");
}
