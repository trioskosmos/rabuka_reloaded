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

// 笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊・// 001 窶・"Other 繧ｹ繝ｪ繝ｼ繧ｺ繝悶・繧ｱ member appears" 竊・optional pay E 竊・active 2 energy
// 笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊・
/// Play a Cerise Bouquet ally while 001 is on stage 竊・choice appears to pay E.
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

    // Play ally to an adjacent slot 窶・this naturally triggers 001's each_time
    let energy_before = v.state.player1.energy_zone.active_count();
    v.play_to_stage(ally, MemberArea::LeftSide);
    drain_auto(&mut v);

    // The each_time fired: an optional payment choice was offered (we skip).
    assert!(
        v.state.rule_log.iter().any(|l| l.contains("offered")),
        "Cerise Bouquet ally appearance must offer the optional payment choice"
    );
    // Skipped payment: only the ally's play cost (4) was deducted.
    assert_eq!(
        v.state.player1.energy_zone.active_count(),
        energy_before - 4,
        "Declining the payment must leave energy at play-cost only"
    );
}

/// Play a non-Cerise Bouquet ally 窶・no trigger.
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

    let energy_before = v.state.player1.energy_zone.active_count();
    v.play_to_stage(non_ally, MemberArea::LeftSide);
    drain_auto(&mut v);

    assert!(
        !v.state.rule_log.iter().any(|l| l.contains("offered")),
        "Non-Cerise-Bouquet ally must not offer 001's payment choice"
    );
    // Only the play cost (4) deducted — no activation bonus.
    assert_eq!(
        v.state.player1.energy_zone.active_count(),
        energy_before - 4,
        "No trigger means no energy activation"
    );
}

/// Play 001 herself to center 窶・her ability checks "縺ｻ縺九・" (other), so no self-trigger.
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

    let energy_before = v.state.player1.energy_zone.active_count();
    v.play_to_stage(hana, MemberArea::Center);
    drain_auto(&mut v);

    // "ほかの" excludes self-appearance: no payment choice offered, and only
    // Hana's own play cost (11) was deducted.
    assert!(
        !v.state.rule_log.iter().any(|l| l.contains("offered")),
        "001's own appearance must not trigger her each_time (ほかの)"
    );
    assert_eq!(
        v.state.player1.energy_zone.active_count(),
        energy_before - 11,
        "Self-appearance must not activate energy"
    );
}

// 笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊・// 009 窶・"闢ｮ繝守ｩｺ member appears" 竊・gain blade+2 (center only)
// 笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊・
/// Play a 闢ｮ繝守ｩｺ ally while 009 is at center 竊・gains blade+2.
#[test]
fn hana_009_center_ally_appears_gains_blade() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hana_center = v.id("PL!HS-pb1-009-R");
    let ally = v.id("PL!HS-sd1-012-SD"); // 闢ｮ繝守ｩｺ / Cerise Bouquet member
    let filler = v.id("PL!-sd1-010-SD");

    v.state.player1.stage.stage = [-1, hana_center, -1];
    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(ally);
    v.give_energy(15);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(filler);
    }

    let blade_before = blade_count(&v, hana_center);
    // Play ally to an adjacent slot 竊・triggers 009's each_time
    v.play_to_stage(ally, MemberArea::LeftSide);
    drain_auto(&mut v);
    let blade_after = blade_count(&v, hana_center);

    assert!(
        blade_after > blade_before,
        "Hana 009 (center) should gain blade when 闢ｮ繝守ｩｺ ally appears ({} 竊・{})",
        blade_before,
        blade_after
    );
}

/// Play a 闢ｮ繝守ｩｺ ally while 009 is at NON-center 竊・no trigger.
#[test]
fn hana_009_not_center_no_trigger() {
    let db = load_real_database();
    let mut v = TestGame::new(db);
    let hana_center = v.id("PL!HS-pb1-009-R");
    let ally = v.id("PL!HS-sd1-012-SD"); // 闢ｮ繝守ｩｺ member
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
        "Hana 009 (non-center) must NOT gain blade ({} 竊・{})",
        blade_before, blade_after
    );
}

/// Real cross-card flow: 009 on stage, play a 闢ｮ繝守ｩｺ ally via play_to_stage 竊・blade+2.
#[test]
fn hana_009_play_ally_triggers_blade() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let hanaho = v.id("PL!HS-pb1-009-R");
    let hasu_ally = v.id("PL!HS-sd1-001-SD"); // 闢ｮ繝守ｩｺ
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
        "Hanaho 009 at Center should gain blade when 闢ｮ繝守ｩｺ ally is played (0 竊・{})",
        blade_after
    );
}

/// Real cross-card: 001 on stage, play a 繧ｹ繝ｪ繝ｼ繧ｺ繝悶・繧ｱ ally 竊・optional E prompt handled.
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

    let energy_before = v.state.player1.energy_zone.active_count();
    v.play_to_stage(ally, MemberArea::Center);
    drain_auto(&mut v);

    assert!(
        v.state.rule_log.iter().any(|l| l.contains("offered")),
        "Ally appearing while 001 on stage must offer the payment choice"
    );
    // Skipped payment: only the ally's play cost (9) deducted.
    assert_eq!(
        v.state.player1.energy_zone.active_count(),
        energy_before - 9,
        "Declining must leave energy at play-cost only"
    );
}

// 笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊・// Q245: 009 at center, play HER from hand to center 竊・own each_time triggers
// 笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊絶武笊・
/// Q245: Kaho (009) deployed to center 窶・her own each_time fires since
/// she IS a 闢ｮ繝守ｩｺ member appearing on stage and she's at center.
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

    // Play Kaho to center 窶・play_to_stage records appearance + triggers auto-abilities
    v.play_to_stage(hanaho, MemberArea::Center);
    drain_auto(&mut v);

    let blade_after = blade_count(&v, hanaho);
    assert!(
        blade_after >= 2,
        "Q245: Kaho deployed to center should trigger own each_time (blade 0 竊・{})",
        blade_after
    );
}

/// Q245 edge: 009 played to NON-center 竊・her each_time requires center 竊・no trigger.
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
