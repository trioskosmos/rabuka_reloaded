use crate::helpers::*;
use rabuka_engine::core::types::Phase;
use rabuka_engine::turn::TurnEngine;

fn drain_auto(v: &mut TestGame) {
    while v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => v.select_indices(&[0]),
            _ => v.select_indices(&[]),
        }
    }
}

fn score_bonus(v: &TestGame, cid: i16) -> i32 {
    v.state.mods.get_score_modifier(cid)
}

fn setup_live_start(v: &mut TestGame, emotion: i16, success_zone_emotions: &[i16]) {
    v.state.player1.live_card_zone.cards.push(emotion);
    for &eid in success_zone_emotions {
        v.state.player1.success_live_card_zone.cards.push(eid);
    }
    v.state.current_phase = Phase::FirstAttackerPerformance;
    TurnEngine::trigger_live_start_abilities(&mut v.state, "p1");
    v.state.process_pending_auto_abilities("p1");
    drain_auto(v);
}

/// 0 EMOTION in success zone → +0 score, +0 heart00
#[test]
fn emotion_zero_in_success_zone() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let emo = v.id("PL!N-bp4-027-L");
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    setup_live_start(&mut v, emo, &[]);
    assert_eq!(
        score_bonus(&v, emo),
        0,
        "No EMOTION in success zone → +0 score"
    );
}

/// 1 EMOTION in success zone → +2 score, +3 heart00
#[test]
fn emotion_one_in_success_zone() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let emo = v.id("PL!N-bp4-027-L");
    let in_success = v.new_id("PL!N-bp4-027-L");
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    setup_live_start(&mut v, emo, &[in_success]);
    assert_eq!(
        score_bonus(&v, emo),
        2,
        "1 EMOTION in success zone → +2 score"
    );
    let heart_mods = v.state.mods.need_heart_modifiers.get(&emo);
    assert!(
        heart_mods.is_some(),
        "need_heart_modifiers should exist for the card"
    );
    if let Some(mods) = heart_mods {
        let h00 = mods
            .get(&rabuka_engine::card::HeartColor::Heart00)
            .map(|e| e.total())
            .unwrap_or(0);
        assert_eq!(h00, 3, "1 EMOTION in success zone → +3 heart00");
    }
}

/// 2 EMOTION cards in success zone → +4 score, +6 heart00
#[test]
fn emotion_two_in_success_zone() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let emo = v.id("PL!N-bp4-027-L");
    let in_success1 = v.new_id("PL!N-bp4-027-L");
    let in_success2 = v.new_id("PL!N-bp4-027-L");
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    setup_live_start(&mut v, emo, &[in_success1, in_success2]);
    assert_eq!(
        score_bonus(&v, emo),
        4,
        "2 EMOTION in success zone → +4 score"
    );
    let heart_mods = v.state.mods.need_heart_modifiers.get(&emo);
    assert!(
        heart_mods.is_some(),
        "need_heart_modifiers should exist for the card"
    );
    if let Some(mods) = heart_mods {
        let h00 = mods
            .get(&rabuka_engine::card::HeartColor::Heart00)
            .map(|e| e.total())
            .unwrap_or(0);
        assert_eq!(h00, 6, "2 EMOTION in success zone → +6 heart00");
    }
}

/// Each EMOTION card only modifies itself via self_target inheritance.
#[test]
fn emotion_modifier_only_on_self() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let emo1 = v.id("PL!N-bp4-027-L");
    let emo2 = v.id("PL!N-bp4-027-L");
    let in_success = v.id("PL!N-bp4-027-L");
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    // emo1 → live_card_zone triggers; in_success → success_zone triggers
    v.state.player1.live_card_zone.cards.push(emo1);
    v.state
        .player1
        .success_live_card_zone
        .cards
        .push(in_success);
    v.state.current_phase = Phase::FirstAttackerPerformance;
    TurnEngine::trigger_live_start_abilities(&mut v.state, "p1");
    v.state.process_pending_auto_abilities("p1");
    drain_auto(&mut v);
    // Both live_zone and success_zone cards trigger and modify themselves
    assert_eq!(score_bonus(&v, emo1), 2, "live_zone EMOTION gets +2");
    assert_eq!(
        score_bonus(&v, in_success),
        2,
        "success_zone EMOTION gets +2"
    );
    // emo2 was never placed in a trigger zone → no modifier
    assert_eq!(score_bonus(&v, emo2), 0, "Unplaced EMOTION not modified");
}
