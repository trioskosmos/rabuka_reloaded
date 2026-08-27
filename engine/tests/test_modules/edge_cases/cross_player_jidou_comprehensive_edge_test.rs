/// Comprehensive cross-player jidou for 233,600,847 (opponent effect also triggers)
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::turn::TurnEngine;

fn trigger_auto(game: &mut TestGame, target_id: i16) {
    assert!(game.state.player1.stage.stage.contains(&target_id), "target should be on stage before trigger");
    let pid = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
}

#[test]
fn koko_self_effect_triggers() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let koko = g.id("PL!SP-sd2-002-SD2");
    g.state.player1.stage.stage[1] = koko;
    g.state.push_movement_event(koko, "stage", "stage", Some(koko), "p1", true);
    trigger_auto(&mut g, koko);
    assert_eq!(g.state.mods.get_heart_modifier(koko, HeartColor::Heart06), 1, "self effect should trigger");
}

#[test]
fn koko_opponent_effect_triggers() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let koko = g.id("PL!SP-sd2-002-SD2");
    g.state.player1.stage.stage[1] = koko;
    g.state.push_movement_event(koko, "stage", "stage", Some(koko), "p2", true);
    trigger_auto(&mut g, koko);
    assert_eq!(g.state.mods.get_heart_modifier(koko, HeartColor::Heart06), 1, "opponent effect should also trigger (でも発動する)");
}

#[test]
fn koko_turn1_blocks_second() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let koko = g.id("PL!SP-sd2-002-SD2");
    g.state.player1.stage.stage[1] = koko;
    g.state.push_movement_event(koko, "stage", "stage", Some(koko), "p1", true);
    trigger_auto(&mut g, koko);
    assert_eq!(g.state.mods.get_heart_modifier(koko, HeartColor::Heart06), 1);
    // Second move same turn should be blocked by turn1
    g.state.push_movement_event(koko, "stage", "stage", Some(koko), "p1", true);
    trigger_auto(&mut g, koko);
    assert_eq!(g.state.mods.get_heart_modifier(koko, HeartColor::Heart06), 1, "turn1 should block second (still 1)");
}

#[test]
fn natsume_self_and_opponent_draw() {
    // PL!SP-pb1-020-N 鬼塚夏美: area move each time -> draw 1 (opponent also)
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let natsume = g.id("PL!SP-pb1-020-N");
    g.state.player1.stage.stage[0] = natsume;
    for _ in 0..10 { g.state.player1.main_deck.cards.push(g.id("PL!-sd1-010-SD")); }
    let before = g.state.player1.hand.cards.len();
    g.state.push_movement_event(natsume, "stage", "stage", Some(natsume), "p1", true);
    trigger_auto(&mut g, natsume);
    assert_eq!(g.state.player1.hand.cards.len(), before + 1, "self move should draw");

    let mut g2 = TestGame::new(load_real_database());
    let n2 = g2.id("PL!SP-pb1-020-N");
    g2.state.player1.stage.stage[0] = n2;
    for _ in 0..10 { g2.state.player1.main_deck.cards.push(g2.id("PL!-sd1-010-SD")); }
    let before2 = g2.state.player1.hand.cards.len();
    g2.state.push_movement_event(n2, "stage", "stage", Some(n2), "p2", true);
    trigger_auto(&mut g2, n2);
    assert_eq!(g2.state.player1.hand.cards.len(), before2 + 1, "opponent move should also draw");
}

#[test]
fn tomari_self_and_opponent_blade() {
    // PL!SP-sd2-011-SD2 鬼塚冬毬: area move -> blade (opponent also) turn1
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let tomari = g.id("PL!SP-sd2-011-SD2");
    g.state.player1.stage.stage[0] = tomari;
    g.state.push_movement_event(tomari, "stage", "stage", Some(tomari), "p1", true);
    trigger_auto(&mut g, tomari);
    let b1 = g.state.mods.blade_modifiers.get(&tomari).map(|e| e.total()).unwrap_or(0);
    assert_eq!(b1, 1, "self move blade");

    let mut g2 = TestGame::new(load_real_database());
    let t2 = g2.id("PL!SP-sd2-011-SD2");
    g2.state.player1.stage.stage[0] = t2;
    g2.state.push_movement_event(t2, "stage", "stage", Some(t2), "p2", true);
    trigger_auto(&mut g2, t2);
    let b2 = g2.state.mods.blade_modifiers.get(&t2).map(|e| e.total()).unwrap_or(0);
    assert_eq!(b2, 1, "opponent move blade");
}
