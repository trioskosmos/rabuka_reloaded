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

/// Butterfly Wing suppresses live_start abilities of stage members.
#[test]
fn butterfly_wing_suppresses_live_start() {
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

    let energy_before = game.state.player1.energy_zone.active_count();

    setup_p1_deck(&mut game, &[]);
    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(butterfly);
    game.set_live_card(butterfly);

    // Advance to live_start phase
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Both Meis' live_start should be suppressed — energy should NOT have increased
    let energy_after = game.state.player1.energy_zone.active_count();
    assert_eq!(
        energy_after, energy_before,
        "Butterfly Wing should suppress live_start: energy should not increase (was {}, got {})",
        energy_before, energy_after
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

    // BW's live_success should fire because members WITH live_start ability are on stage
    let score_mod = game.state.mods.get_score_modifier(butterfly);
    assert_eq!(
        score_mod, 1,
        "Butterfly Wing should get +1 score (live_start members on stage): got {}",
        score_mod
    );
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
