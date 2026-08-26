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

// PL!HS-bp1-019-L is is_null (parenthetical) - should count as no ability
#[test]
fn rin_is_null_live_counts_as_no_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rin = game.id("PL!-bp4-014-N");
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage = [mate, rin, -1];
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let null_live = game.id("PL!HS-bp1-019-L"); // is_null
    game.state.player1.live_card_zone.cards.push(null_live);
    fire_live_start(&mut game, rin);
    let blade = game.state.mods.get_blade_modifier(mate);
    assert_eq!(blade, 2, "is_null live should count as no LS/LSS -> should grant blade, got {}", blade);
}

// Live with both LS and LSS should NOT count
#[test]
fn rin_both_triggers_live_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rin = game.id("PL!-bp4-014-N");
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage = [mate, rin, -1];
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let both = game.id("PL!N-sd2-007-P"); // has both
    game.state.player1.live_card_zone.cards.push(both);
    fire_live_start(&mut game, rin);
    let blade = game.state.mods.get_blade_modifier(mate);
    assert_eq!(blade, 0, "live with both LS/LSS should not count, got {}", blade);
}

// Live with only LS should NOT count (has LS)
#[test]
fn rin_only_ls_live_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rin = game.id("PL!-bp4-014-N");
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage = [mate, rin, -1];
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    // Find a live with only LS (e.g., PL!N-bp1-027-L has LS)
    let only_ls = game.id("PL!N-bp1-027-L");
    game.state.player1.live_card_zone.cards.push(only_ls);
    fire_live_start(&mut game, rin);
    let blade = game.state.mods.get_blade_modifier(mate);
    assert_eq!(blade, 0, "live with only LS should not count");
}
