/// Tests for PL!N-bp5-006-R (近江彼方 / Kanata Konoe) — Restriction + LiveSuccess wait
///
/// ab#0 (常時):
///   このメンバーは自分のアクティブフェイズにアクティブにしない。
///
/// ab#1 (ライブ成功時):
///   自分のステージにこのメンバー以外のメンバーがいる場合、このメンバーをウェイトにする。
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
fn advance_to_live_victory(game: &mut TestGame) {
    for _ in 0..3 {
        game.pass();
    }
}

#[test]
fn kanata_not_auto_activated_in_active_phase_but_others_are() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanata = game.id("PL!N-bp5-006-R");
    let other_member = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-013-SD");
    game.state.player1.stage.stage = [kanata, other_member, -1];
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.mods.add_orientation_modifier(kanata, "wait");
    game.state
        .mods
        .add_orientation_modifier(other_member, "wait");
    game.state.recalculate_constants();
    assert!(game
        .state
        .constant_cannot_activate_members
        .contains(&kanata.to_string()));
    assert!(!game
        .state
        .constant_cannot_activate_members
        .contains(&other_member.to_string()));
    assert!(!game
        .state
        .cannot_activate_members
        .contains(&"p1".to_string()));
    game.state.current_phase = rabuka_engine::types::Phase::Active;
    rabuka_engine::turn::TurnEngine::advance_phase(&mut game.state);
    assert_eq!(
        game.state.mods.get_orientation_modifier(kanata).cloned(),
        Some("wait".to_string())
    );
    let other_ori = game
        .state
        .mods
        .get_orientation_modifier(other_member)
        .cloned();
    assert!(other_ori != Some("wait".to_string()));
}

#[test]
fn kanata_can_be_activated_by_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanata = game.id("PL!N-bp5-006-R");
    let filler = game.id("PL!-sd1-013-SD");
    game.state.player1.stage.stage = [kanata, -1, -1];
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.recalculate_constants();
    game.state.mods.add_orientation_modifier(kanata, "wait");
    game.state.mods.add_orientation_modifier(kanata, "active");
    assert!(game.state.mods.get_orientation_modifier(kanata).cloned() != Some("wait".to_string()));
}

#[test]
fn kanata_live_success_with_others_waits_self() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanata = game.id("PL!N-bp5-006-R");
    let other = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [kanata, other, -1];
    game.state.player1.hand.cards.push(live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    advance_to_live_victory(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    assert_eq!(
        game.state.mods.get_orientation_modifier(kanata).cloned(),
        Some("wait".to_string()),
        "Kanata should be waited by her own LiveSuccess ability"
    );
}

#[test]
fn kanata_live_success_no_others_does_not_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanata = game.id("PL!N-bp5-006-R");
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [kanata, -1, -1];
    game.state.player1.hand.cards.push(live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    advance_to_live_victory(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    assert!(
        game.state.mods.get_orientation_modifier(kanata).cloned() != Some("wait".to_string()),
        "Kanata should NOT be waited (no other members)"
    );
}
