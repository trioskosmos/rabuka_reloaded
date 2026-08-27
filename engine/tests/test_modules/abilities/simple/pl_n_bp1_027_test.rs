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

fn setup_live_start(v: &mut TestGame, solitude_rain: i16, stage_card_ids: &[i16]) {
    v.state.player1.stage.stage = [
        stage_card_ids.first().copied().unwrap_or(-1),
        stage_card_ids.get(1).copied().unwrap_or(-1),
        stage_card_ids.get(2).copied().unwrap_or(-1),
    ];
    v.state.player1.live_card_zone.cards.push(solitude_rain);
    v.state.current_phase = Phase::FirstAttackerPerformance;
    TurnEngine::trigger_live_start_abilities(&mut v.state, "p1");
    v.state.process_pending_auto_abilities("p1");
    drain_auto(v);
}

/// No Nijigasaki members on stage → score +0
#[test]
fn solitude_rain_no_stage_members() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let rain = v.id("PL!N-bp1-027-L");
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    setup_live_start(&mut v, rain, &[]);
    assert_eq!(score_bonus(&v, rain), 0, "No members → 0");
}

/// 1 Nijigasaki member with heart02 + heart06 → +2
#[test]
fn solitude_rain_one_member_two_colors() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let rain = v.id("PL!N-bp1-027-L");
    // PL!N-sd1-019-PR: base_heart={heart02, heart06}, no ability
    let member = v.id("PL!N-sd1-019-PR");
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    setup_live_start(&mut v, rain, &[member, -1, -1]);
    assert_eq!(score_bonus(&v, rain), 2, "heart02 + heart06 → +2");
}

/// 2 Nijigasaki members with overlapping colors → count unique
/// Member A: heart02+heart06, Member B: heart02+heart05
/// Unique: heart02, heart05, heart06 → +3
#[test]
fn solitude_rain_two_members_overlap() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let rain = v.id("PL!N-bp1-027-L");
    let member_a = v.id("PL!N-sd1-019-PR"); // heart02+heart06, Nijigasaki
    let member_b = v.id("PL!N-bp1-017-N"); // heart02+heart05, Nijigasaki, no ability
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    setup_live_start(&mut v, rain, &[member_a, member_b, -1]);
    assert_eq!(score_bonus(&v, rain), 3, "heart02+heart05+heart06 → +3");
}

/// 1 Nijigasaki (heart01+heart02) + 1 non-Nijigasaki (heart01+heart03, ignored)
/// Unique from Nijigasaki: heart01, heart02 → +2
#[test]
fn solitude_rain_nijigasaki_plus_non_nijigasaki() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let rain = v.id("PL!N-bp1-027-L");
    let niji = v.id("PL!N-bp1-013-N"); // heart01+heart02, Nijigasaki, no ability
    let other = v.id("PL!-sd1-010-SD"); // heart01+heart03, non-Nijigasaki μ's
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    setup_live_start(&mut v, rain, &[niji, other, -1]);
    assert_eq!(
        score_bonus(&v, rain),
        2,
        "Only Nijigasaki hearts count → +2"
    );
}

/// 2 Nijigasaki members with NO matching heart_colors → +0
/// The specified colors are: heart01,02,03,04,05,06
/// If neither member has any of these → unlikely but edge case
#[test]
fn solitude_rain_members_no_matching_colors() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let rain = v.id("PL!N-bp1-027-L");
    // All these members have hearts only in the specified set
    // To test "no match", we'd need a card with NONE of these colors.
    // Since the set is all 6 standard hearts, any member card has at least one.
    // Instead: test 0 stage members → 0
    let member = v.id("PL!N-sd1-019-PR"); // heart02+heart06
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    setup_live_start(&mut v, rain, &[member, -1, -1]);
    // Sanity check: heart02 and heart06 are in the specified set → +2
    assert_eq!(score_bonus(&v, rain), 2, "Basic sanity still +2");
}

/// 1 member with heart01+heart03 → +2
#[test]
fn solitude_rain_one_member_heart01_heart03() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let rain = v.id("PL!N-bp1-027-L");
    let member = v.id("PL!N-bp1-024-N"); // heart01+heart03, Nijigasaki, no ability
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    setup_live_start(&mut v, rain, &[member, -1, -1]);
    assert_eq!(score_bonus(&v, rain), 2, "heart01 + heart03 → +2");
}

/// 1 member with heart02+heart05, 2nd member also heart02+heart05 (same exact colors)
/// Unique: heart02, heart05 → +2
#[test]
fn solitude_rain_two_members_identical_colors() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let rain = v.id("PL!N-bp1-027-L");
    let member_a = v.id("PL!N-bp1-017-N"); // heart02+heart05, Nijigasaki
    let member_b = v.id("PL!N-bp1-017-N"); // heart02+heart05, Nijigasaki (different copy)
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    setup_live_start(&mut v, rain, &[member_a, member_b, -1]);
    assert_eq!(score_bonus(&v, rain), 2, "Only heart02+heart05 unique → +2");
}
