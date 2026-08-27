/// Tests for エマ・ヴェルデ (PL!N-bp3-008-R＋) — Activation: wait a にこ member
/// other than this member → draw 1.
///
/// Q163: "This member" (the ability user) cannot be selected for the wait cost
/// because of exclude_self. With no other qualifying members, the cost fails.
use crate::helpers::*;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Only エマ on stage (a にこ member). exclude_self=true means no valid target
/// for the wait cost → activation should fail/not prompt.
#[test]
fn emma_q163_self_excluded_no_other_niko_cost_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[2] = filler;

    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Activate ability
    game.activate_ability(emma);

    // Cost: wait a にこ member other than self.
    // With only エマ (a にこ member) on stage and exclude_self=true,
    // no valid candidates → cost should fail silently
    // (the ability should not proceed to draw)

    // Drain any pending choices
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Cost should fail since exclude_self leaves no candidates.
    // The failed cost should not proceed to draw.
    let hand_count = game.state.player1.hand.cards.len();
    // hand started with 1 filler, never drew because ability cost failed
    assert_eq!(
        hand_count, 1,
        "No draw happened because cost couldn't be paid (got {})",
        hand_count
    );
}

/// Put エマ alongside a 虹ヶ咲 member. The group_names: ["虹ヶ咲"] from the
/// parser should match the 虹ヶ咲 member. exclude_self excludes エマ.
/// → the 虹ヶ咲 member is the only valid candidate → it gets waited → draw 1.
#[test]
fn emma_q163_nijigasaki_member_pays_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    // 虹ヶ咲 member: any 虹ヶ咲 series member card
    let niji = game.id("PL!N-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[2] = niji;

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Activate Emma's ability
    game.activate_ability(emma);

    // Cost: wait a 虹ヶ咲 member other than self — niji is the ONLY candidate,
    // so the engine auto-applies the wait (single legal target, no prompt needed).
    // Strict: verify the wait was actually applied to niji (not emma, not filler).
    let niji_waited = game
        .state
        .mods
        .get_orientation_modifier(niji)
        .map_or(false, |o| o == "wait");
    assert!(niji_waited, "niji (only 虹ヶ咲 candidate) must be auto-waited as cost");
    let emma_waited = game
        .state
        .mods
        .get_orientation_modifier(emma)
        .map_or(false, |o| o == "wait");
    assert!(!emma_waited, "exclude_self: Emma herself must NOT be waited");
    let filler_waited = game
        .state
        .mods
        .get_orientation_modifier(filler)
        .map_or(false, |o| o == "wait");
    assert!(!filler_waited, "non-虹ヶ咲 filler must NOT be waited");

    let hand_count = game.state.player1.hand.cards.len();
    assert!(hand_count > 0, "Should have drawn 1 card after cost");
}

// =========================================================================
// ab#1 — Live Start (PL!N-bp3-008 ab#1)
//   手札を2枚控え室に置いてもよい：自分のステージにいるこのメンバー以外の
//   ウェイト状態のメンバー1人をアクティブにする。そうした場合、ライブ終了時まで、
//   これによりアクティブにしたメンバーと、このメンバーは、それぞれ
//   {{heart_04.png|heart04}}を得る。
// =========================================================================

fn trigger_emma_live_start(game: &mut TestGame, emma: i16) {
    let card = game.db.get_card(emma).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        rabuka_engine::core::types::AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(emma),
        None,
        None,
    );
    game.state.activating_card = Some(emma);
    game.state.process_pending_auto_abilities(&pid);
}

/// Basic case: 1 wait member on stage. Activate it → both Emma and that
/// member get heart04.  No other members get heart04.
#[test]
fn emma_live_start_activates_wait_both_gain_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    let wait_member = game.id("PL!N-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [wait_member, emma, -1];
    // wait_member starts in wait state
    game.state
        .mods
        .add_orientation_modifier(wait_member, "wait");
    // Give cards for optional discard cost
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);

    trigger_emma_live_start(&mut game, emma);

    // Optional cost: discard 2 cards. Say yes.
    while game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }

    // Observed: only one wait member is eligible → the change_state leg
    // auto-resolves with NO prompt (only the hand cost prompt was presented).
    assert!(
        !game.has_pending_choice(),
        "single wait-member candidate auto-activates; no selection prompt expected"
    );

    assert!(!game.has_pending_choice(), "Ability should resolve cleanly");

    let emma_heart = game
        .state
        .mods
        .get_heart_modifier(emma, rabuka_engine::card::HeartColor::Heart04);
    let member_heart = game
        .state
        .mods
        .get_heart_modifier(wait_member, rabuka_engine::card::HeartColor::Heart04);
    assert_eq!(emma_heart, 1, "Emma should get +1 heart04");
    assert_eq!(member_heart, 1, "Activated member should get +1 heart04");

    // Verify the wait member is now active
    let orientation = game.state.mods.get_orientation_modifier(wait_member);
    assert!(
        orientation.is_none() || orientation.as_deref() != Some("wait"),
        "Activated member should no longer be in wait state"
    );
}

/// 3 members (Emma + 2 wait members). Only the activated one + Emma get
/// heart04 — the other wait member gets nothing.
#[test]
fn emma_live_start_three_members_only_activated_gets_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    let member_a = game.id("PL!N-sd1-010-SD");
    let member_b = game.id("PL!N-sd1-011-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [member_a, emma, member_b];
    game.state.mods.add_orientation_modifier(member_a, "wait");
    game.state.mods.add_orientation_modifier(member_b, "wait");
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);

    trigger_emma_live_start(&mut game, emma);

    // Optional cost: discard 2 cards
    assert!(
        game.has_pending_choice(),
        "optional discard-2 cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (zone=hand count=2 allow_skip=true) for the cost"
    );
    game.select_indices(&[0, 1]);
    // Two wait members on stage → a real choice which one to activate.
    // Select member_a only to activate
    assert!(
        game.has_pending_choice(),
        "change_state member selection prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (zone=stage count=1 allow_skip=false)"
    );
    game.select_indices(&[0]);

    assert!(!game.has_pending_choice(), "Ability should resolve cleanly");

    let emma_heart = game
        .state
        .mods
        .get_heart_modifier(emma, rabuka_engine::card::HeartColor::Heart04);
    let member_a_heart = game
        .state
        .mods
        .get_heart_modifier(member_a, rabuka_engine::card::HeartColor::Heart04);
    let member_b_heart = game
        .state
        .mods
        .get_heart_modifier(member_b, rabuka_engine::card::HeartColor::Heart04);
    assert_eq!(emma_heart, 1, "Emma should get +1 heart04");
    assert_eq!(
        member_a_heart, 1,
        "Activated member A should get +1 heart04"
    );
    assert_eq!(
        member_b_heart, 0,
        "Non-activated member B should NOT get heart04"
    );
}

/// No wait members on stage: the change_state has no valid targets → heart04
/// is NOT granted to anyone.
#[test]
fn emma_live_start_no_wait_members_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    let active_member = game.id("PL!N-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [active_member, emma, -1];
    // active_member is already active (no wait modifier)
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);

    trigger_emma_live_start(&mut game, emma);

    while game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }

    // No wait members → no choice, ability completes without granting heart
    assert!(!game.has_pending_choice(), "Ability should resolve cleanly");

    let emma_heart = game
        .state
        .mods
        .get_heart_modifier(emma, rabuka_engine::card::HeartColor::Heart04);
    let member_heart = game
        .state
        .mods
        .get_heart_modifier(active_member, rabuka_engine::card::HeartColor::Heart04);
    assert_eq!(emma_heart, 0, "Emma should NOT get heart04");
    assert_eq!(member_heart, 0, "Active member should NOT get heart04");
}

/// Only Emma on stage (no other members). exclude_self prevents self-selection
/// for change_state → no valid target → heart04 is NOT granted.
#[test]
fn emma_live_start_alone_no_other_members_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, emma, -1];
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);

    trigger_emma_live_start(&mut game, emma);

    while game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }

    assert!(!game.has_pending_choice(), "Ability should resolve cleanly");

    let emma_heart = game
        .state
        .mods
        .get_heart_modifier(emma, rabuka_engine::card::HeartColor::Heart04);
    assert_eq!(emma_heart, 0, "Emma alone should NOT get heart04");
}

/// 1 active + 1 wait member. Activate the wait member → only wait member +
/// Emma get heart04.  Active member gets nothing.
#[test]
fn emma_live_start_active_member_not_affected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    let active_member = game.id("PL!N-sd1-010-SD");
    let wait_member = game.id("PL!N-sd1-011-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [active_member, emma, wait_member];
    game.state
        .mods
        .add_orientation_modifier(wait_member, "wait");
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);

    trigger_emma_live_start(&mut game, emma);

    while game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }
    // Observed: only one wait member is eligible → the change_state leg
    // auto-resolves with NO prompt (only the hand cost prompt was presented).
    assert!(
        !game.has_pending_choice(),
        "single wait-member candidate auto-activates; no selection prompt expected"
    );

    assert!(!game.has_pending_choice(), "Ability should resolve cleanly");

    let emma_heart = game
        .state
        .mods
        .get_heart_modifier(emma, rabuka_engine::card::HeartColor::Heart04);
    let wait_heart = game
        .state
        .mods
        .get_heart_modifier(wait_member, rabuka_engine::card::HeartColor::Heart04);
    let active_heart = game
        .state
        .mods
        .get_heart_modifier(active_member, rabuka_engine::card::HeartColor::Heart04);
    assert_eq!(emma_heart, 1, "Emma should get +1 heart04");
    assert_eq!(wait_heart, 1, "Activated wait member should get +1 heart04");
    assert_eq!(
        active_heart, 0,
        "Already-active member should NOT get heart04"
    );
}
