/// Tests for 安養寺 姫芽 (PL!HS-sd1-006-SD) debut ability.
///
/// Ability text:
///   登場: 自分のステージに「大沢瑠璃乃」か「百生吟子」か「徒町小鈴」が
///   いる場合、エネルギーを1枚アクティブにし、自分の控え室から
///   『蓮ノ空』のライブカードを1枚手札に加える。
///
/// Parser fix: "アクティブにし" (し-form) was missing from STATE_CHANGE_PATTERNS.
/// Now emits sequential [change_state(active, energy_card), move_cards].
use crate::helpers::*;
use rabuka_engine::core::types::LogMetadata;
use rabuka_engine::zones::MemberArea;

fn play_himeno(game: &mut TestGame, left: i16, center: i16, right: i16) {
    let himeno = game.id("PL!HS-sd1-006-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.state.player1.stage.stage = [left, center, right];
    game.add_to_hand(himeno);
    game.add_to_hand(filler);
    game.give_energy(15);

    game.play_to_stage(himeno, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

fn debut_result_and_actions(game: &TestGame) -> (String, Vec<String>) {
    let mut result = "not_found".to_string();
    let mut actions = Vec::new();
    for entry in &game.state.structured_log {
        if let Some(LogMetadata::AbilityResolution {
            result: r, items, ..
        }) = &entry.metadata
        {
            if entry.text.contains("trigger_debut") {
                result = r.clone();
                for item in items {
                    if let Some(a) = item.get("action").and_then(|v| v.as_str()) {
                        if !actions.contains(&a.to_string()) {
                            actions.push(a.to_string());
                        }
                    }
                }
            }
        }
    }
    (result, actions)
}

// ====================================================================
// Condition: matching character → ability fires
// ====================================================================

#[test]
fn condition_passes_with_sd_osawa() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let f = g.id("PL!-sd1-013-SD");
    let o = g.id("PL!HS-sd1-003-SD");
    play_himeno(&mut g, f, f, o);
    let (r, _) = debut_result_and_actions(&g);
    assert_eq!(r, "success");
}

#[test]
fn condition_passes_with_bp1_osawa() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let f = g.id("PL!-sd1-013-SD");
    let o = g.id("PL!HS-bp1-005-PR");
    play_himeno(&mut g, f, f, o);
    let (r, _) = debut_result_and_actions(&g);
    assert_eq!(r, "success");
}

#[test]
fn condition_passes_with_bp5_osawa() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let f = g.id("PL!-sd1-013-SD");
    let o = g.id("PL!HS-bp5-003-P");
    play_himeno(&mut g, f, f, o);
    let (r, _) = debut_result_and_actions(&g);
    assert_eq!(r, "success");
}

#[test]
fn condition_passes_with_momoo() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let f = g.id("PL!-sd1-013-SD");
    let o = g.id("PL!HS-sd1-004-SD");
    play_himeno(&mut g, f, f, o);
    let (r, _) = debut_result_and_actions(&g);
    assert_eq!(r, "success");
}

#[test]
fn condition_passes_with_kodomo() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let f = g.id("PL!-sd1-013-SD");
    let o = g.id("PL!HS-sd1-005-SD");
    play_himeno(&mut g, f, f, o);
    let (r, _) = debut_result_and_actions(&g);
    assert_eq!(r, "success");
}

#[test]
fn condition_passes_with_two_bp_osawa() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let o1 = g.id("PL!HS-bp1-005-PR");
    let o2 = g.id("PL!HS-bp5-003-P");
    play_himeno(&mut g, -1, o1, o2);
    let (r, _) = debut_result_and_actions(&g);
    assert_eq!(r, "success");
}

// ====================================================================
// Condition: no matching character → ability must NOT fire
// ====================================================================

#[test]
fn condition_fails_with_no_match() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let f = g.id("PL!-sd1-013-SD");
    let n = g.id("PL!-sd1-010-SD");
    play_himeno(&mut g, n, f, f);
    let (r, _) = debut_result_and_actions(&g);
    assert_eq!(r, "failure");
}

// ====================================================================
// Effects: both change_state and move_cards execute when condition passes
// ====================================================================

#[test]
fn both_effects_execute() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let f = g.id("PL!-sd1-013-SD");
    let o = g.id("PL!HS-sd1-003-SD");
    play_himeno(&mut g, f, f, o);
    let (r, actions) = debut_result_and_actions(&g);
    assert_eq!(r, "success");
    assert!(
        actions.contains(&"change_state".to_string()),
        "Missing energy activation, got: {:?}",
        actions
    );
    assert!(
        actions.contains(&"move_cards".to_string()),
        "Missing live card retrieval, got: {:?}",
        actions
    );
}

#[test]
fn no_effects_without_match() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let f = g.id("PL!-sd1-013-SD");
    let n = g.id("PL!-sd1-010-SD");
    play_himeno(&mut g, n, f, f);
    let (r, actions) = debut_result_and_actions(&g);
    assert_eq!(r, "failure");
    assert!(
        actions.is_empty(),
        "No effects when condition fails, got: {:?}",
        actions
    );
}
