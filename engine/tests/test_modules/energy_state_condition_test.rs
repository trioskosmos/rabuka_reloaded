/// Coverage for energy_state_condition (0 tested before)
/// idx 855 PL!SP-pb2-026-N 平安名すみれ 常時 active energy -> heart02 x2 (as_long_as)
/// idx 389 PL!SP-bp4-028-L DAISUKI FULL POWER ライブ開始時 active energy -> score+1
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const SUMIRE: &str = "PL!SP-pb2-026-N";
const DAISUKI: &str = "PL!SP-bp4-028-L";

fn stage_hearts(game: &TestGame, cid: i16) -> u8 {
    game.state.mods.heart_modifiers.get(&cid)
        .and_then(|m| m.get(&rabuka_engine::card::HeartColor::Heart02))
        .map(|e| e.total() as u8)
        .unwrap_or(0)
}

// --- 855: 常時 heart gate ---

#[test]
fn sumire_no_active_energy_no_hearts() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let sumire = g.id(SUMIRE);
    g.add_to_stage(MemberArea::Center, sumire);
    // no energy at all -> active_count 0
    g.state.recalculate_constants();
    assert_eq!(stage_hearts(&g, sumire), 0, "no active energy => no hearts");
}

#[test]
fn sumire_with_one_active_energy_gains_two_hearts() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let sumire = g.id(SUMIRE);
    g.add_to_stage(MemberArea::Center, sumire);
    g.give_energy(1); // 1 active
    g.state.recalculate_constants();
    assert_eq!(stage_hearts(&g, sumire), 2, "1 active => +2 heart02, got {}", stage_hearts(&g, sumire));
}

#[test]
fn sumire_inactive_energy_does_not_count() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let sumire = g.id(SUMIRE);
    g.add_to_stage(MemberArea::Center, sumire);
    g.give_energy(1);
    // tap it
    g.state.player1.energy_zone.pay_energy(1).unwrap();
    g.state.recalculate_constants();
    assert_eq!(stage_hearts(&g, sumire), 0, "wait energy should not satisfy active check");
}

#[test]
fn sumire_toggling_active_recalculates() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let sumire = g.id(SUMIRE);
    g.add_to_stage(MemberArea::Center, sumire);
    g.give_energy(2);
    g.state.recalculate_constants();
    assert_eq!(stage_hearts(&g, sumire), 2);
    g.state.player1.energy_zone.pay_energy(2).unwrap();
    g.state.recalculate_constants();
    assert_eq!(stage_hearts(&g, sumire), 0);
    // re-activate via Active phase would re-add, but manual add_active
    g.state.player1.energy_zone.add_active(1);
    g.state.recalculate_constants();
    assert_eq!(stage_hearts(&g, sumire), 2);
}

// --- 389: ライブ開始時 score gate ---

fn trigger_daisuki_live_start(g: &mut TestGame, live: i16) {
    let card = g.db.get_card(live).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("DAISUKI should have live_start");
    let pid = g.state.player1.id.clone();
    g.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        rabuka_engine::core::types::AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(live),
        None,
        None,
    );
    g.state.activating_card = Some(live);
    g.state.process_pending_auto_abilities(&pid);
    g.drain_auto_ability_choices();
}

#[test]
fn daisuki_score_plus_one_when_active_energy_exists() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let live = g.id(DAISUKI);
    g.state.player1.live_card_zone.cards.push(live);
    g.give_energy(1);
    trigger_daisuki_live_start(&mut g, live);
    assert!(!g.state.has_pending_choice());
    let score = g.state.mods.get_score_modifier(live);
    assert_eq!(score, 1, "active energy should give score+1, got {}", score);
}

#[test]
fn daisuki_score_not_added_when_no_active_energy() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let live = g.id(DAISUKI);
    g.state.player1.live_card_zone.cards.push(live);
    g.give_energy(1);
    g.state.player1.energy_zone.pay_energy(1).unwrap(); // make it wait
    trigger_daisuki_live_start(&mut g, live);
    assert!(!g.state.has_pending_choice());
    let score = g.state.mods.get_score_modifier(live);
    assert_eq!(score, 0, "wait energy should not satisfy active check, got {}", score);
    g.state.recalculate_constants();
}

#[test]
fn daisuki_opponent_active_does_not_satisfy_self() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let live = g.id(DAISUKI);
    g.state.player1.live_card_zone.cards.push(live);
    let eng = g.id("LL-E-001-SD");
    g.state.player2.energy_zone.cards.push(eng);
    g.state.player2.energy_zone.add_active(1);
    trigger_daisuki_live_start(&mut g, live);
    assert!(!g.state.has_pending_choice());
    let score = g.state.mods.get_score_modifier(live);
    assert_eq!(score, 0, "opponent active should not satisfy self, got {}", score);
}
