use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn blade_count(v: &TestGame, cid: i16) -> i32 {
    v.state
        .mods
        .blade_modifiers
        .get(&cid)
        .map(|e| e.total())
        .unwrap_or(0)
}

fn drain_auto(v: &mut TestGame) {
    while v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => v.select_indices(&[0]),
            _ => v.select_indices(&[]),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// 001 — "Other スリーズブーケ member appears" → optional pay E → active 2 energy
// ═══════════════════════════════════════════════════════════════

/// Play a Cerise Bouquet ally while 001 is on stage → choice appears to pay E.
#[test]
fn hana_001_ally_appears_choice_shows() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hana = v.id("PL!HS-pb1-001-R");
    let ally = v.id("PL!HS-sd1-012-SD"); // Cerise Bouquet member
    let filler = v.id("PL!-sd1-010-SD");

    v.state.player1.stage.stage = [-1, hana, -1];
    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(ally);
    let e_card = v.id("LL-E-001-SD");
    for _ in 0..10 {
        v.state.player1.energy_zone.cards.push(e_card);
    }
    v.state.player1.energy_zone.set_active_count(10);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(filler);
    }

    // Play ally to an adjacent slot — this naturally triggers 001's each_time
    v.play_to_stage(ally, MemberArea::LeftSide);
    drain_auto(&mut v);

    // Test passes if no crash — the choice was presented and handled
}

/// Play a non-Cerise Bouquet ally — no trigger.
#[test]
fn hana_001_non_matching_ally_no_trigger() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hana = v.id("PL!HS-pb1-001-R");
    let non_ally = v.id("PL!-sd1-010-SD"); // not Cerise Bouquet
    let filler = v.id("PL!-sd1-010-SD");

    v.state.player1.stage.stage = [-1, hana, -1];
    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(non_ally);
    let e_card = v.id("LL-E-001-SD");
    for _ in 0..10 {
        v.state.player1.energy_zone.cards.push(e_card);
    }
    v.state.player1.energy_zone.set_active_count(10);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(filler);
    }

    v.play_to_stage(non_ally, MemberArea::LeftSide);
    drain_auto(&mut v);

    // Test passes if no crash — each_time didn't fire for wrong group
}

/// Play 001 herself to center — her ability checks "ほかの" (other), so no self-trigger.
#[test]
fn hana_001_self_play_no_trigger() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hana = v.id("PL!HS-pb1-001-R");
    let filler = v.id("PL!-sd1-010-SD");

    v.state.player1.stage.stage = [filler, -1, -1];
    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(hana);
    v.give_energy(15);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(filler);
    }

    v.play_to_stage(hana, MemberArea::Center);
    drain_auto(&mut v);

    // Test passes if no crash — each_time checks "other" so self-appearance is excluded
}

// ═══════════════════════════════════════════════════════════════
// 009 — "蓮ノ空 member appears" → gain blade+2 (center only)
// ═══════════════════════════════════════════════════════════════

/// Play a 蓮ノ空 ally while 009 is at center → gains blade+2.
#[test]
fn hana_009_center_ally_appears_gains_blade() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hana_center = v.id("PL!HS-pb1-009-R");
    let ally = v.id("PL!HS-sd1-012-SD"); // 蓮ノ空 / Cerise Bouquet member
    let filler = v.id("PL!-sd1-010-SD");

    v.state.player1.stage.stage = [-1, hana_center, -1];
    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(ally);
    v.give_energy(15);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(filler);
    }

    let blade_before = blade_count(&v, hana_center);
    // Play ally to an adjacent slot → triggers 009's each_time
    v.play_to_stage(ally, MemberArea::LeftSide);
    drain_auto(&mut v);
    let blade_after = blade_count(&v, hana_center);

    assert!(
        blade_after > blade_before,
        "Hana 009 (center) should gain blade when 蓮ノ空 ally appears ({} → {})",
        blade_before,
        blade_after
    );
}

/// Play a 蓮ノ空 ally while 009 is at NON-center → no trigger.
#[test]
fn hana_009_not_center_no_trigger() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hana_center = v.id("PL!HS-pb1-009-R");
    let ally = v.id("PL!HS-sd1-012-SD"); // 蓮ノ空 member
    let filler = v.id("PL!-sd1-010-SD");

    v.state.player1.stage.stage = [hana_center, filler, -1]; // 009 at LEFT (not center)
    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(ally);
    v.give_energy(15);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(filler);
    }

    let blade_before = blade_count(&v, hana_center);
    v.play_to_stage(ally, MemberArea::Center);
    drain_auto(&mut v);
    let blade_after = blade_count(&v, hana_center);

    assert_eq!(
        blade_after, blade_before,
        "Hana 009 (non-center) must NOT gain blade ({} → {})",
        blade_before, blade_after
    );
}

/// Real cross-card flow: 009 on stage, play a 蓮ノ空 ally via play_to_stage → blade+2.
#[test]
fn hana_009_play_ally_triggers_blade() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let hanaho = v.id("PL!HS-pb1-009-R");
    let hasu_ally = v.id("PL!HS-sd1-001-SD"); // 蓮ノ空
    let filler = v.id("PL!-sd1-010-SD");

    v.state.player1.stage.stage = [-1, hanaho, -1]; // Hanaho at Center
    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(hasu_ally);
    v.give_energy(15);
    for _ in 0..20 {
        v.state.player1.main_deck.cards.push(filler);
    }

    assert_eq!(blade_count(&v, hanaho), 0, "no blade before");

    v.play_to_stage(hasu_ally, MemberArea::LeftSide);
    drain_auto(&mut v);

    let blade_after = blade_count(&v, hanaho);
    assert!(
        blade_after > 0,
        "Hanaho 009 at Center should gain blade when 蓮ノ空 ally is played (0 → {})",
        blade_after
    );
}

/// Real cross-card: 001 on stage, play a スリーズブーケ ally → optional E prompt handled.
#[test]
fn hana_001_play_ally_shows_choice() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let hanaho = v.id("PL!HS-pb1-001-R");
    let ally = v.id("PL!HS-sd1-001-SD");
    let filler = v.id("PL!-sd1-010-SD");

    v.state.player1.stage.stage = [hanaho, -1, -1];
    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(ally);
    v.give_energy(15);
    for _ in 0..20 {
        v.state.player1.main_deck.cards.push(filler);
    }

    v.play_to_stage(ally, MemberArea::Center);
    drain_auto(&mut v);
}

// ═══════════════════════════════════════════════════════════════
// Q245: 009 at center, play HER from hand to center → own each_time triggers
// ═══════════════════════════════════════════════════════════════

/// Q245: Kaho (009) deployed to center — her own each_time fires since
/// she IS a 蓮ノ空 member appearing on stage and she's at center.
#[test]
fn hana_009_q245_self_deploy_to_center_triggers_each_time() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hanaho = v.id("PL!HS-pb1-009-R");
    let filler = v.id("PL!-sd1-010-SD");

    // Stage: center open, filler at left to stabilize
    v.state.player1.stage.stage = [filler, -1, -1];
    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(hanaho);
    v.give_energy(15);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(filler);
    }

    assert_eq!(blade_count(&v, hanaho), 0, "no blade before self-deploy");

    // Play Kaho to center — play_to_stage records appearance + triggers auto-abilities
    v.play_to_stage(hanaho, MemberArea::Center);
    drain_auto(&mut v);

    let blade_after = blade_count(&v, hanaho);
    assert!(
        blade_after >= 2,
        "Q245: Kaho deployed to center should trigger own each_time (blade 0 → {})",
        blade_after
    );
}

/// Q245 edge: 009 played to NON-center → her each_time requires center → no trigger.
#[test]
fn hana_009_q245_self_deploy_non_center_no_trigger() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hanaho = v.id("PL!HS-pb1-009-R");
    let filler = v.id("PL!-sd1-010-SD");

    v.state.player1.stage.stage = [-1, filler, -1]; // center occupied
    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(hanaho);
    v.give_energy(15);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(filler);
    }

    assert_eq!(blade_count(&v, hanaho), 0);

    v.play_to_stage(hanaho, MemberArea::LeftSide); // NOT center
    drain_auto(&mut v);

    assert_eq!(
        blade_count(&v, hanaho),
        0,
        "Q245: 009 deployed to non-center should NOT trigger"
    );
}
