use crate::helpers::*;
use crate::test_modules::support::baton_swap_auto_helpers::*;
use rabuka_engine::core::game_modifiers::CardOrientation;
use rabuka_engine::core::types::Phase;
use rabuka_engine::zones::MemberArea;

// PL!N-bp7-022-N 三船栞子 (was accept-path only)
// {{jidou.png|自動}}{{turn1.png|ターン1回}}ライブフェイズの間、自分のステージに
// いる『虹ヶ咲』のメンバー1人がウェイト状態になったとき、手札を1枚控え室に
// 置いてもよい。そうしたとき、そのメンバーをアクティブにする。

/// Stage Shioriko + a Niji member; record the member's active→wait change
/// during the live phase; run the watcher scan.
fn shioriko_wait_setup(game: &mut TestGame, waited: i16) {
    let shioriko = game.id("PL!N-bp7-022-N");
    game.state.player1.stage.set_area(MemberArea::Center, waited);
    game.state.player1.stage.set_area(MemberArea::LeftSide, shioriko);
    game.state.current_phase = Phase::FirstAttackerPerformance;
    game.state
        .recently_state_changed
        .push((waited, "active".to_string(), "wait".to_string(), "".to_string()));
    game.state.mods.orientation_modifiers.insert(waited, CardOrientation::Wait);
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn live_phase_group_member_wait_optional_discard_removes_wait_state() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shioriko = game.id("PL!N-bp7-022-N"); // 虹ヶ咲 member
    let waited = game.id(NIJI_MEMBER); // 上原歩夢, 虹ヶ咲 member
    game.state.player1.hand.cards.push(game.id(FILLER)); // optional discard
    game.state.player1.stage.set_area(MemberArea::Center, waited);
    game.state.player1.stage.set_area(MemberArea::LeftSide, shioriko);

    // Enter the live performance phase and record the wait state change.
    game.state.current_phase = Phase::FirstAttackerPerformance;
    game.state
        .recently_state_changed
        .push((waited, "active".to_string(), "wait".to_string(), "".to_string()));
    game.state.mods.orientation_modifiers.insert(waited, CardOrientation::Wait);

    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);

    // The auto offers the optional discard → accept → the waited member activates.
    resolve_auto_choices_accepting_optionals(&mut game);

    assert_ne!(
        game.state.mods.orientation_modifiers.get(&waited),
        Some(&CardOrientation::Wait),
        "三船栞子 ab#0 should activate a waited 虹ヶ咲 member during the live phase"
    );
}

/// Edge: decline the optional discard → member stays wait.
#[test]
fn live_phase_group_wait_decline_discard_stays_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let waited = game.id(NIJI_MEMBER);
    game.state.player1.hand.cards.push(game.id(FILLER));
    shioriko_wait_setup(&mut game, waited);

    assert!(game.has_pending_choice(), "optional discard prompted");
    game.select_indices(&[]);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.mods.orientation_modifiers.get(&waited),
        Some(&CardOrientation::Wait),
        "declined: member stays wait"
    );
}

/// Edge: wait happens OUTSIDE the live phase → must not fire.
#[test]
fn non_live_phase_wait_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shioriko = game.id("PL!N-bp7-022-N");
    let waited = game.id(NIJI_MEMBER);
    game.state.player1.hand.cards.push(game.id(FILLER));
    game.state.player1.stage.set_area(MemberArea::Center, waited);
    game.state.player1.stage.set_area(MemberArea::LeftSide, shioriko);
    game.state.current_phase = Phase::Main;
    game.state
        .recently_state_changed
        .push((waited, "active".to_string(), "wait".to_string(), "".to_string()));
    game.state.mods.orientation_modifiers.insert(waited, CardOrientation::Wait);
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);

    assert!(
        !game.has_pending_choice(),
        "outside live phase: no prompt"
    );
    assert_eq!(
        game.state.mods.orientation_modifiers.get(&waited),
        Some(&CardOrientation::Wait),
        "outside live phase: stays wait"
    );
}

/// Edge: waited member is NOT Niji → must not fire.
#[test]
fn non_nijigasaki_wait_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // FILLER (Honoka, μ's) waits during the live phase.
    let waited = game.id(FILLER);
    game.state.player1.hand.cards.push(game.id(FILLER));
    shioriko_wait_setup(&mut game, waited);

    assert!(
        !game.has_pending_choice(),
        "non-Niji wait: no prompt"
    );
}

/// Edge: empty hand → optional cost auto-skips, no prompt, stays wait.
#[test]
fn empty_hand_auto_skips_no_reactivate() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let waited = game.id(NIJI_MEMBER);
    assert!(
        game.state.player1.hand.cards.is_empty(),
        "test setup: hand starts empty"
    );
    shioriko_wait_setup(&mut game, waited);

    assert!(
        !game.has_pending_choice(),
        "empty hand: auto-skip, no prompt"
    );
    assert_eq!(
        game.state.mods.orientation_modifiers.get(&waited),
        Some(&CardOrientation::Wait),
        "empty hand: stays wait"
    );
}

/// Edge: turn1 budget — second Niji wait in the same live phase does not
/// re-fire.
#[test]
fn second_wait_same_live_phase_does_not_refire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let waited_a = game.id(NIJI_MEMBER);
    let waited_b = game.new_id(NIJI_MEMBER);
    game.state.player1.hand.cards.push(game.id(FILLER));
    game.state.player1.hand.cards.push(game.id(FILLER));
    shioriko_wait_setup(&mut game, waited_a);
    resolve_auto_choices_accepting_optionals(&mut game);
    assert_ne!(
        game.state.mods.orientation_modifiers.get(&waited_a),
        Some(&CardOrientation::Wait),
        "first wait reactivates"
    );

    // Second Niji member waits in the same live phase.
    game.state.player1.stage.set_area(MemberArea::RightSide, waited_b);
    game.state
        .recently_state_changed
        .push((waited_b, "active".to_string(), "wait".to_string(), "".to_string()));
    game.state.mods.orientation_modifiers.insert(waited_b, CardOrientation::Wait);
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);

    assert!(
        !game.has_pending_choice(),
        "budget consumed: no second prompt"
    );
    assert_eq!(
        game.state.mods.orientation_modifiers.get(&waited_b),
        Some(&CardOrientation::Wait),
        "budget consumed: second member stays wait"
    );
}

/// Edge: Shioriko HERSELF waits (she is Niji) → fires for herself.
#[test]
fn self_wait_fires_for_shioriko() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let shioriko = game.id("PL!N-bp7-022-N");
    game.state.player1.hand.cards.push(game.id(FILLER));
    // Only Shioriko on stage; she waits.
    game.state.player1.stage.set_area(MemberArea::Center, shioriko);
    game.state.current_phase = Phase::FirstAttackerPerformance;
    game.state
        .recently_state_changed
        .push((shioriko, "active".to_string(), "wait".to_string(), "".to_string()));
    game.state.mods.orientation_modifiers.insert(shioriko, CardOrientation::Wait);
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_abilities_for_player(&pid);
    game.state.process_pending_auto_abilities(&pid);

    assert!(game.has_pending_choice(), "self wait prompts");
    resolve_auto_choices_accepting_optionals(&mut game);
    assert_ne!(
        game.state.mods.orientation_modifiers.get(&shioriko),
        Some(&CardOrientation::Wait),
        "self wait reactivates Shioriko herself"
    );
}
