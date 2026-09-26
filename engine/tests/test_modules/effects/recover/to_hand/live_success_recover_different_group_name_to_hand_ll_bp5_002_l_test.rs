use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::core::types::Phase;
use rabuka_engine::turn::TurnEngine;
/// Tests for LL-bp5-002-L (Bring the LOVE!) distinct group name condition.
///
/// ab#0 (LiveStart): if 3+ members with different group names on stage,
///   center member gains all hearts until live end.
/// ab#1 (LiveSuccess): from discard, move 1 card whose group differs from
///   ALL stage members to hand (group_reference: "different_group_names").
///
/// Q225: Multi-name card counts as 1 member on stage.
/// Q89:  Multi-name cards have the group printed on them (via series).
/// Q105: Multi-name contributes ONE constituent group.
/// Q208: When a multi-name card shares a name with another card on stage,
///   it uses one of its OTHER names.

fn fill_deck(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn has_all_heart(game: &TestGame, card_id: i16) -> bool {
    game.state
        .mods
        .heart_modifiers
        .get(&card_id)
        .and_then(|mods| mods.get(&HeartColor::All))
        .map_or(false, |entry| entry.total() > 0)
}

/// Bypass the actual live resolution and directly trigger LiveSuccess
/// abilities.  Injects enough hearts into stage_hearts so the engine's
/// should_trigger_live_success check passes, then fires the abilities.
/// The live card must be in live_card_zone for the trigger scan to find it.
fn force_live_success(game: &mut TestGame, live_card_id: i16) {
    // Technique: inject stage_hearts directly (heart00=20 wildcard
    // is enough to satisfy ANY live card's need_heart).
    let mut heart_map = HeartMap::new();
    heart_map.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts: heart_map });

    // Place the live card in the zone so trigger_live_success_abilities scans it
    game.state.player1.live_card_zone.cards.push(live_card_id);

    // Set phase to LiveVictoryDetermination (required by should_trigger_live_success)
    game.state.current_phase = Phase::LiveVictoryDetermination;

    // Fire LiveSuccess and process
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

// ═══ ab#0: LiveStart — distinct group condition ═══════════════════════

/// 3 members from 3 different groups → center gets all hearts.
#[test]
fn three_distinct_groups_center_gets_all_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let aqours = game.id("PL!S-pb1-003-R");
    let nijigasaki = game.id("PL!N-pb1-001-R");
    let muse = game.id("PL!-bp3-003-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [muse, aqours, nijigasaki];
    game.give_energy(15);
    game.state.player1.hand.cards.push(live);
    fill_deck(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let hm = game.state.mods.heart_modifiers.get(&aqours);
    assert!(hm.is_some(), "Center (Aqours) should have heart modifier");
    if let Some(mods) = hm {
        let all_entry = mods.get(&HeartColor::All);
        assert!(all_entry.is_some(), "Should have All heart modifier");
        assert_eq!(all_entry.unwrap().total(), 1, "Should have +1 All heart");
    }
}

/// 2 members from 2 different groups → not enough for condition.
///
/// The fixture used to be `[aqours, aqours, muse]` — the SAME card id in two
/// areas, which no legal state can hold — so this was never a "two groups on
/// stage" case at all. Now two DISTINCT 『Aqours』 instances plus a μ member, and
/// the claim is checked as a granted AMOUNT rather than the absence of a map
/// entry (a zero-valued entry would fail `.is_none()` while nothing was granted).
#[test]
fn two_distinct_groups_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let aqours = game.id("PL!S-pb1-003-R");
    let aqours2 = game.new_id("PL!S-pb1-003-R");
    let muse = game.id("PL!-bp3-003-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.assert_card_identity(aqours, "PL!S-pb1-003-R");
    game.assert_same_card_name(aqours, aqours2, "two 『Aqours』 instances");
    assert_ne!(aqours, aqours2, "two separate card instances");
    game.assert_distinct_card_names(aqours, muse, "『Aqours』 vs μ");

    game.state.player1.stage.stage = [aqours, aqours2, muse];
    game.give_energy(15);
    game.state.player1.hand.cards.push(live);
    fill_deck(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_heart_modifier(aqours, HeartColor::All),
        0,
        "2 distinct groups on stage must not grant the All heart"
    );
}

/// All 3 members from the same group → no effect.
///
/// The name said "same group" but the fixture was `[muse1, muse1, muse2]` with
/// `muse1` a 虹ヶ咲 member and `muse2` a μ one — two groups, not one, and the
/// same card id in two areas besides. Now three separate μ instances.
#[test]
fn all_same_group_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let muse1 = game.id("PL!-bp3-003-R");
    let muse2 = game.id("PL!-sd1-001-SD");
    let muse3 = game.id("PL!-sd1-003-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.assert_same_card_name(muse1, muse2, "three μ members");
    game.assert_same_card_name(muse1, muse3, "three μ members");
    assert_ne!(
        [muse1, muse2, muse3],
        [muse1, muse1, muse1],
        "three SEPARATE instances, not one card in three areas"
    );

    game.state.player1.stage.stage = [muse1, muse2, muse3];
    game.give_energy(15);
    game.state.player1.hand.cards.push(live);
    fill_deck(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_heart_modifier(muse1, HeartColor::All),
        0,
        "all three members from ONE group must not grant the All heart"
    );
}

/// Center empty → no effect even with 3 groups.
#[test]
fn center_empty_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let aqours = game.id("PL!S-pb1-003-R");
    let nijigasaki = game.id("PL!N-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [aqours, -1, nijigasaki];
    game.give_energy(15);
    game.state.player1.hand.cards.push(live);
    fill_deck(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // "Empty center → no effect" means NOBODY gained a heart, not that the
    // sentinel id -1 has no modifier. The old assertion keyed on -1, which is
    // never a modifier key and so passed whatever the engine did — it would
    // have passed with the effect firing.
    for &cid in game.state.player1.stage.stage.iter() {
        if cid == -1 {
            continue;
        }
        let hearts: Vec<HeartColor> = vec![
            HeartColor::Heart00,
            HeartColor::Heart01,
            HeartColor::Heart02,
            HeartColor::Heart03,
            HeartColor::Heart04,
            HeartColor::Heart05,
            HeartColor::Heart06,
        ];
        let gained: Vec<HeartColor> = hearts
            .into_iter()
            .filter(|h| game.state.mods.get_heart_modifier(cid, *h) != 0)
            .collect();
        assert!(
            gained.is_empty(),
            "Empty center should not get hearts, but card {cid} gained {gained:?}"
        );
    }
    assert_eq!(
        game.state.player1.stage.stage[1],
        -1,
        "setup guard: the center area really is empty"
    );
}

/// Single multi-name card counts as ONE group, not three.
#[test]
fn multi_name_card_single_slot_one_group_not_three() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let multi = game.id("LL-bp2-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [multi, -1, -1];
    game.give_energy(15);
    game.state.player1.hand.cards.push(live);
    fill_deck(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Q105: one multi-name card = one group → 3-group condition fails.
    // `is_none()` on a HashMap entry only proves no ENTRY exists; a zero-valued
    // entry for every heart would pass it while the effect had fired. Check the
    // granted amount instead.
    let gained: Vec<HeartColor> = [
        HeartColor::Heart00,
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ]
    .into_iter()
    .filter(|h| game.state.mods.get_heart_modifier(multi, *h) != 0)
    .collect();
    assert!(
        gained.is_empty(),
        "Single multi-name card (1 group) should NOT satisfy 3-group condition, \
         but the center member gained {gained:?}"
    );
}

#[test]
fn q225_joint_card_counts_as_one_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let joint = game.id("LL-bp1-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [joint, -1, -1];
    game.give_energy(15);
    game.state.player1.hand.cards.push(live);
    fill_deck(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        !has_all_heart(&game, joint),
        "Q225: one joint card is one member, not three"
    );
}

#[test]
fn q225_joint_cards_can_supply_distinct_groups() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let joint1 = game.id("LL-bp1-001-R\u{ff0b}");
    let joint2 = game.id("LL-bp2-001-R\u{ff0b}");
    let muse = game.id("PL!-bp3-003-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [muse, joint2, joint1];
    game.give_energy(30);
    game.state.player1.hand.cards.push(live);
    fill_deck(&mut game, filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        has_all_heart(&game, joint2),
        "Q225: two joint cards can each represent one distinct group"
    );
}

// ═══ ab#1: LiveSuccess — different_group_names discard filter ════════
//
// All ab#1 tests use force_live_success() which bypasses the performance
// phase and directly injects stage_hearts + fires LiveSuccess triggers.
// This lets us focus on testing the discard group filter without needing
// specific base hearts on stage members.
// phase and directly injects stage_hearts + fires LiveSuccess triggers.
// This lets us focus on testing the discard group filter without needing
// specific base hearts on stage members.

// ── Simple positive/negative ─────────────────────────────────────────

/// Discard has a card from a group NOT on stage → auto-moved to hand.
#[test]
fn ab1_different_group_moved_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let muse = game.id("PL!-bp3-003-R"); // μ's (stage, left)
    let filler = game.id("PL!-sd1-010-SD"); // μ's (stage, center)
    let aqours_discard = game.id("PL!S-pb1-003-R"); // Aqours (discard)

    game.state.player1.stage.stage = [muse, filler, -1];
    game.state.player1.waitroom.cards.push(aqours_discard);
    force_live_success(&mut game, live);

    assert!(
        game.state.player1.hand.cards.contains(&aqours_discard),
        "Aqours card should move to hand (differs from stage μ's)"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&aqours_discard),
        "Aqours card should no longer be in discard"
    );
}

#[test]
fn ab1_group_bearing_live_card_can_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let muse = game.id("PL!-bp3-003-R");
    let aqours_live = game.id("PL!S-sd1-019-SD");

    game.state.player1.stage.stage = [muse, -1, -1];
    game.state.player1.waitroom.cards.push(aqours_live);
    force_live_success(&mut game, live);

    assert!(
        !game.has_pending_choice(),
        "A single eligible card should auto-resolve"
    );
    assert!(
        game.state.player1.hand.cards.contains(&aqours_live),
        "Aqours live card should move to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&aqours_live),
        "Aqours live card should leave the waitroom"
    );
}

/// Discard has a card from a group ON stage → NOT moved, stays in discard.
#[test]
fn ab1_same_group_not_moved() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let muse_stage = game.id("PL!-bp3-003-R"); // μ's (stage)
    let muse_discard = game.id("PL!-sd1-010-SD"); // μ's (discard, same group)

    game.state.player1.stage.stage = [muse_stage, -1, -1];
    game.state.player1.waitroom.cards.push(muse_discard);
    force_live_success(&mut game, live);

    assert!(
        !game.state.player1.hand.cards.contains(&muse_discard),
        "μ's card should NOT move to hand (same group as stage)"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&muse_discard),
        "μ's card should remain in discard"
    );
}

/// Empty discard → no cards moved.
#[test]
fn ab1_empty_discard_nothing_happens() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let muse = game.id("PL!-bp3-003-R");

    game.state.player1.stage.stage = [muse, -1, -1];
    let hand_before = game.state.player1.hand.cards.len();
    force_live_success(&mut game, live);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "No cards should move (empty discard)"
    );
}

/// Multiple discard cards: only the non-matching-group one is moved.
#[test]
fn ab1_mixed_discard_only_different_group_moved() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let muse_stage = game.id("PL!-bp3-003-R"); // μ's (stage)
    let filler = game.id("PL!-sd1-010-SD"); // μ's (stage)
    let aqours_discard = game.id("PL!S-pb1-003-R"); // Aqours (should move)
    let muse_discard = game.id("PL!-sd1-010-SD"); // μ's (should stay)

    game.state.player1.stage.stage = [muse_stage, filler, -1];
    game.state.player1.waitroom.cards.push(muse_discard);
    game.state.player1.waitroom.cards.push(aqours_discard);
    force_live_success(&mut game, live);

    assert!(
        game.state.player1.hand.cards.contains(&aqours_discard),
        "Aqours card should move (differs from stage)"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&muse_discard),
        "μ's card should NOT move (same group as stage)"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&muse_discard),
        "μ's card should remain in discard"
    );
}

#[test]
fn ab1_empty_stage_moves_one_eligible_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let aqours_card = game.id("PL!S-pb1-003-R");
    let muse_card = game.id("PL!-bp3-003-R");

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.waitroom.cards.push(aqours_card);
    game.state.player1.waitroom.cards.push(muse_card);
    force_live_success(&mut game, live);

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "Bring the LOVE must select exactly one eligible card"
    );
    game.select_indices(&[0]);

    let moved = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&id| id == aqours_card || id == muse_card)
        .count();
    assert_eq!(moved, 1, "Only one eligible card moves");
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "The other eligible card remains in the waitroom"
    );
}

// ── Multi-name card on stage ─────────────────────────────────────────

/// Multi-name card on stage blocks ALL constituent groups from discard.
/// Q89: multi-name has the group via series.  For exclusion we block ALL
/// groups the multi-name can match via its series.
#[test]
fn ab1_multiname_stage_blocks_all_its_groups() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let multi_stage = game.id("LL-bp2-001-R\u{ff0b}"); // Aqours, Liella!, 蓮ノ空
    let aqours_discard = game.id("PL!S-pb1-003-R"); // Aqours — blocked by multi
    let muse_discard = game.id("PL!-bp3-003-R"); // μ's — NOT blocked

    game.state.player1.stage.stage = [multi_stage, -1, -1];
    game.state.player1.waitroom.cards.push(aqours_discard);
    game.state.player1.waitroom.cards.push(muse_discard);
    force_live_success(&mut game, live);

    // Aqours card blocked (multi matches Aqours)
    assert!(
        game.state.player1.waitroom.cards.contains(&aqours_discard),
        "Aqours should stay in discard (blocked by multi-name's Aqours group)"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&aqours_discard),
        "Aqours should NOT be in hand"
    );

    // μ's card should move (μ's is not in multi's groups)
    assert!(
        game.state.player1.hand.cards.contains(&muse_discard),
        "μ's should move to hand (not blocked by multi)"
    );
}

/// Two multi-name cards on stage: combined groups block even more.
#[test]
fn ab1_two_multiname_stage_blocks_more_groups() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let multi1 = game.id("LL-bp2-001-R\u{ff0b}"); // Aqours, Liella!, 蓮ノ空
    let multi2 = game.id("LL-bp1-001-R\u{ff0b}"); // 虹ヶ咲, Liella!, 蓮ノ空
    let muse = game.id("PL!-bp3-003-R"); // μ's — NOT blocked
    let aqours = game.id("PL!S-pb1-003-R"); // Aqours — blocked by multi1
    let nijigasaki = game.id("PL!N-pb1-001-R"); // 虹ヶ咲 — blocked by multi2

    game.state.player1.stage.stage = [multi1, multi2, -1];
    game.state.player1.waitroom.cards.push(muse);
    game.state.player1.waitroom.cards.push(aqours);
    game.state.player1.waitroom.cards.push(nijigasaki);
    force_live_success(&mut game, live);

    assert!(
        game.state.player1.hand.cards.contains(&muse),
        "μ's should move (not blocked)"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&aqours),
        "Aqours should NOT move (blocked by multi1)"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&nijigasaki),
        "虹ヶ咲 should NOT move (blocked by multi2)"
    );
}

// ── Multi-name card in discard ──────────────────────────────────────

/// Multi-name card in discard is selectable when ALL its groups differ
/// from every stage member's group.
#[test]
fn ab1_multiname_discard_selectable_when_groups_differ() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let muse = game.id("PL!-bp3-003-R");
    let filler = game.id("PL!-sd1-010-SD");
    let multi_discard = game.id("LL-bp2-001-R\u{ff0b}"); // Aqours, Liella!, 蓮ノ空

    game.state.player1.stage.stage = [muse, filler, -1];
    game.state.player1.waitroom.cards.push(multi_discard);
    force_live_success(&mut game, live);

    assert!(
        game.state.player1.hand.cards.contains(&multi_discard),
        "Multi-name card should move (all its groups differ from μ's)"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&multi_discard),
        "Multi-name card should no longer be in discard"
    );
}

/// Multi-name card in discard is BLOCKED when any of its constituent
/// groups matches a stage member's group.
#[test]
fn ab1_multiname_discard_blocked_when_group_matches_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let aqours_stage = game.id("PL!S-pb1-003-R");
    let filler = game.id("PL!-bp3-003-R");
    let multi_discard = game.id("LL-bp2-001-R\u{ff0b}"); // has Aqours → blocked

    game.state.player1.stage.stage = [aqours_stage, filler, -1];
    game.state.player1.waitroom.cards.push(multi_discard);
    force_live_success(&mut game, live);

    assert!(
        !game.state.player1.hand.cards.contains(&multi_discard),
        "Multi-name card should NOT move (its Aqours group matches stage)"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&multi_discard),
        "Multi-name card should remain in discard"
    );
}

/// Multi-name card in discard blocked when ANY of its groups matches.
#[test]
fn ab1_multiname_discard_blocked_by_one_matching_group() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let nijigasaki = game.id("PL!N-pb1-001-R"); // 虹ヶ咲 (stage)
    let filler = game.id("PL!-bp3-003-R");
    let multi_discard = game.id("LL-bp1-001-R\u{ff0b}"); // has 虹ヶ咲 → blocked

    game.state.player1.stage.stage = [nijigasaki, filler, -1];
    game.state.player1.waitroom.cards.push(multi_discard);
    force_live_success(&mut game, live);

    assert!(
        !game.state.player1.hand.cards.contains(&multi_discard),
        "Multi-name card should NOT move (its 虹ヶ咲 group matches stage)"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&multi_discard),
        "Multi-name card should remain in discard"
    );
}

#[test]
fn ab1_two_multiname_discards_moves_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("LL-bp5-002-L");
    let muse = game.id("PL!-bp3-003-R");
    let filler = game.id("PL!-sd1-010-SD");
    let multi = game.id("LL-bp2-001-R\u{ff0b}");

    game.state.player1.stage.stage = [muse, filler, -1];
    game.state.player1.waitroom.cards.push(multi);
    game.state.player1.waitroom.cards.push(multi);
    force_live_success(&mut game, live);

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "Bring the LOVE must select one of two eligible cards"
    );
    game.select_indices(&[0]);

    let count_in_hand = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&c| c == multi)
        .count();
    assert_eq!(count_in_hand, 1, "Exactly one multi-name card moves");
    assert_eq!(
        game.state
            .player1
            .waitroom
            .cards
            .iter()
            .filter(|&&c| c == multi)
            .count(),
        1,
        "The second multi-name card remains in the waitroom"
    );
}
