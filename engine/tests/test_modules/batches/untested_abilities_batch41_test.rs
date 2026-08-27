/// Untested-abilities batch 41 — live-zone aggregate twin + ability-filter gate.
///
/// - PL!S-bp5-017-N 小原鞠莉 (ライブ開始時): own live-card zone's need_heart
///   heart05 total ≥4 -> gain heart05 until live end (twin of batch-38 dia).
/// - PL!-bp4-014-N 星空凛 (ライブ開始時): if any card in own live zone has
///   NEITHER a ライブ開始時 nor ライブ成功時 ability -> every OTHER staged
///   member gains +2 blades until live end (ability_filter no_ability_type).
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
// PL!S-bp5-017-N 小原鞠莉 — live-zone heart05 aggregate
// ====================================================================

fn mari_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let mari = game.id("PL!S-bp5-017-N");
    game.state.player1.stage.stage[1] = mari;
    mari
}

#[test]
fn mari_heart05_total_exactly_four_grants() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mari = mari_setup(&mut game);
    // PL!HS-PR-011-PR need_heart contains heart05 x4 -> exactly 4.
    let l = game.id("PL!HS-PR-011-PR");
    game.state.player1.live_card_zone.cards.push(l);

    fire_live_start(&mut game, mari);

    assert_eq!(
        game.state.mods.get_heart_modifier(mari, HeartColor::Heart05),
        1,
        "aggregate == 4 satisfies >=4"
    );
}

#[test]
fn mari_heart05_total_two_no_grant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mari = mari_setup(&mut game);
    let l = game.id("PL!S-PR-023-PR"); // heart05 x2 only
    game.state.player1.live_card_zone.cards.push(l);

    fire_live_start(&mut game, mari);

    assert_eq!(
        game.state.mods.get_heart_modifier(mari, HeartColor::Heart05),
        0,
        "aggregate 2 < 4 -> no grant"
    );
}

#[test]
fn mari_empty_live_zone_no_grant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mari = mari_setup(&mut game);

    fire_live_start(&mut game, mari);

    assert_eq!(
        game.state.mods.get_heart_modifier(mari, HeartColor::Heart05),
        0
    );
}

// ====================================================================
// PL!-bp4-014-N 星空凛 — no_ability_type filter over the live zone
// ====================================================================

fn rin_setup(game: &mut TestGame) -> (i16, i16) {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let rin = game.id("PL!-bp4-014-N");
    game.state.player1.stage.stage[1] = rin;
    let mate = game.id("PL!S-sd1-001-SD");
    game.state.player1.stage.stage[0] = mate;
    (rin, mate)
}

#[test]
fn rin_live_without_triggers_grants_other_member_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (rin, mate) = rin_setup(&mut game);
    // PL!-sd1-020-SD is a live card with NO ライブ開始時/ライブ成功時 ability.
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
fn rin_all_lives_have_triggers_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (rin, mate) = rin_setup(&mut game);
    // PL!N-sd2-007-P carries both ライブ開始時 and ライブ成功時 abilities.
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
fn rin_empty_live_zone_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (rin, mate) = rin_setup(&mut game);

    fire_live_start(&mut game, rin);

    assert_eq!(game.state.mods.get_blade_modifier(mate), 0);
}
