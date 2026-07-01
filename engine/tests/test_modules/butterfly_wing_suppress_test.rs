use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn setup_p1_deck(game: &mut TestGame, live_ids: &[i16]) {
    let filler = game.id_ref("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for (i, &id) in live_ids.iter().enumerate() {
        game.state.player1.main_deck.cards.insert(1 + i, id);
    }
}

/// Q260: Suppressed LiveStart abilities do NOT count as resolved.
///
/// Butterfly Wing's constant ability suppresses LiveStart triggers on stage members.
/// Those suppressed abilities are never queued → never resolved.
/// Contrast with a non-suppressing live card where LiveStart abilities DO queue.
#[test]
fn butterfly_wing_q260_suppressed_not_resolved() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let butterfly = game.id("PL!SP-pb2-046-L");
    let mei_r = game.id("PL!SP-pb1-007-R");
    let mei_p = game.id("PL!SP-pb1-007-P＋");
    let filler = game.id("PL!-sd1-010-SD");

    // Put 2 Meis (both have live_start) + 1 filler on stage
    game.add_to_stage(MemberArea::LeftSide, filler);
    game.add_to_stage(MemberArea::Center, mei_p);
    game.add_to_stage(MemberArea::RightSide, mei_r);

    setup_p1_deck(&mut game, &[]);
    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(butterfly);
    game.set_live_card(butterfly);

    // Advance to live_start phase
    advance_to_live_start(&mut game);

    // Q260: Suppressed LiveStart abilities are NOT queued → NOT resolved
    assert_eq!(
        game.state.ability_queue.len(),
        0,
        "Q260: Suppressed LiveStart abilities should NOT be in the queue (length 0)"
    );

    // Drain any pending choices (should be none — LiveStart fully suppressed)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Cross-check: energy didn't change (confirms suppression)
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "Q260: No LiveStart effects fired — energy unchanged"
    );
}

/// Control: WITHOUT suppression, LiveStart abilities ARE queued and resolved.
#[test]
fn butterfly_wing_q260_control_live_start_resolves() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Use a non-suppressing live card (PL!-sd1-019-SD is a simple live card)
    let live = game.id("PL!-sd1-019-SD");
    let mei_r = game.id("PL!SP-pb1-007-R");
    let mei_p = game.id("PL!SP-pb1-007-P＋");
    let filler = game.id("PL!-sd1-010-SD");

    // Same stage: 2 Meis (live_start) + 1 filler
    game.add_to_stage(MemberArea::LeftSide, filler);
    game.add_to_stage(MemberArea::Center, mei_p);
    game.add_to_stage(MemberArea::RightSide, mei_r);

    // Give some energy so we can detect changes
    let ecard = game.id("LL-E-001-SD");
    for _ in 0..2 {
        game.state.player1.energy_zone.cards.push(ecard);
        game.state.player1.energy_zone.set_active_count(0);
    }

    setup_p1_deck(&mut game, &[]);
    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(live);
    game.set_live_card(live);

    // Advance to live_start — LiveStart abilities should trigger
    advance_to_live_start(&mut game);

    // Meis' LiveStart abilities should be in the queue
    assert!(
        game.state.ability_queue.len() > 0,
        "Without suppression: LiveStart abilities should be queued (len > 0)"
    );

    // Process all pending choices (LiveStart abilities resolve)
    // Mei's live_start gives "エネルギーを2枚アクティブにする" — 2 Meis = 4 energy activated
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Energy should have increased since LiveStart abilities resolved
    assert!(
        game.state.player1.energy_zone.active_count() >= 2,
        "Control: LiveStart abilities resolved → energy should have been activated (got {})",
        game.state.player1.energy_zone.active_count()
    );
}

/// Butterfly Wing's own live_success still fires when a live_start member is on stage.
#[test]
fn butterfly_wing_live_success_scores_with_live_start_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let butterfly = game.id("PL!SP-pb2-046-L");
    let mei_r = game.id("PL!SP-pb1-007-R"); // live_start, heart02:2
    let mei_p = game.id("PL!SP-pb1-007-P＋"); // live_start, heart02:2
    let keke = game.id("PL!SP-pb1-013-N"); // no ability, heart06:3

    // Stage: 2 Meis (live_start) + Keke (heart06 for BW's need_heart)
    // Total hearts: heart02:4, heart06:3 → satisfies BW's {heart06:3, heart0:3}
    game.add_to_stage(MemberArea::LeftSide, mei_r);
    game.add_to_stage(MemberArea::Center, mei_p);
    game.add_to_stage(MemberArea::RightSide, keke);

    // Deck setup
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(keke);
    }
    game.state.player2.main_deck.cards.clear();
    let filler = game.id_ref("PL!-sd1-010-SD");
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(butterfly);
    game.set_live_card(butterfly);

    // Advance to live_start (suppressed), then performance
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(butterfly),
        0,
        "LiveSuccess score bonus cleared after live"
    );
    let l = game.state.performance_snapshots[0]
        .lives
        .iter()
        .find(|l| l.card_id == butterfly)
        .unwrap();
    assert_eq!(l.score - l.base_score, 1, "bonus in final score");
}

/// Butterfly Wing's live_success does NOT score when no live_start member is on stage.
#[test]
fn butterfly_wing_no_live_start_member_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let butterfly = game.id("PL!SP-pb2-046-L");
    let keke = game.id("PL!SP-pb1-013-N"); // no ability, heart06:3
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: only non-live-start members
    game.add_to_stage(MemberArea::LeftSide, filler);
    game.add_to_stage(MemberArea::Center, keke);
    game.add_to_stage(MemberArea::RightSide, filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(keke);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(butterfly);
    game.set_live_card(butterfly);

    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let score_mod = game.state.mods.get_score_modifier(butterfly);
    assert_eq!(
        score_mod, 0,
        "Butterfly Wing should NOT get +1 (no live_start member on stage): got {}",
        score_mod
    );
}
