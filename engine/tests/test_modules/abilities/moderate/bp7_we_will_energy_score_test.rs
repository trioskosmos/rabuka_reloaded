/// WE WILL!! PL!SP-bp7-024-L ab#0 (ライブ成功時).
///
/// ライブ成功時 自分のエネルギーが相手より2枚以上多い場合、このカードのスコアを＋１する。
///
/// At LiveSuccess: if your energy is 2+ more than the opponent's, +1 score.
/// i.e.  self_energy - opp_energy >= 2  →  self_energy >= opp_energy + 2.
///
/// The condition must compare SELF vs OPPONENT energy, not just check self >= 2.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const WW: &str = "PL!SP-bp7-024-L";
const ENERGY: &str = "LL-E-001-SD";

fn score(g: &TestGame, id: i16) -> i32 {
    g.state.mods.get_score_modifier(id)
}

/// Put the WE WILL!! live card in P1's live zone and give each player a
/// specific energy count, then trigger the ライブ成功時 ability. Returns WW's id.
fn trigger_success(game: &mut TestGame, p1_energy: u32, p2_energy: u32) -> i16 {
    let ww = game.id(WW);
    game.state.player1.live_card_zone.cards.push(ww);
    // Set P1 energy
    for _ in 0..p1_energy {
        game.state.player1.energy_zone.cards.push(game.id(ENERGY));
    }
    game.state.player1.energy_zone.add_active(p1_energy as u8);
    // Set P2 energy
    for _ in 0..p2_energy {
        game.state.player2.energy_zone.cards.push(game.id(ENERGY));
    }
    game.state.player2.energy_zone.add_active(p2_energy as u8);

    let card = game.db.get_card(ww).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .expect("ab#0 should be ライブ成功時");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(ww),
        None,
        None,
    );
    game.state.activating_card = Some(ww);
    game.state.process_pending_auto_abilities(&pid);
    ww
}

// ====================================================================
// Condition: self_energy - opp_energy >= 2  (self >= opp + 2)
// ====================================================================

/// P1 energy 2, P2 energy 0 → diff 2 → +1 score (boundary).
#[test]
fn we_will_diff_exactly_2_gains_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ww = trigger_success(&mut game, 2, 0);
    assert_eq!(
        score(&game, ww),
        1,
        "P1 2 > P2 0 (diff 2) → +1 score"
    );
}

/// P1 energy 3, P2 energy 0 → diff 3 (>2) → +1 score.
#[test]
fn we_will_diff_3_gains_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ww = trigger_success(&mut game, 3, 0);
    assert_eq!(score(&game, ww), 1, "P1 3 > P2 0 (diff 3) → +1 score");
}

/// P1 energy 2, P2 energy 1 → diff 1 (<2) → NO score.
#[test]
fn we_will_diff_1_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ww = trigger_success(&mut game, 2, 1);
    assert_eq!(
        score(&game, ww),
        0,
        "P1 2 vs P2 1 (diff 1) → no score"
    );
}

/// P1 energy 2, P2 energy 2 → diff 0 → NO score.
#[test]
fn we_will_equal_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ww = trigger_success(&mut game, 2, 2);
    assert_eq!(score(&game, ww), 0, "P1 2 == P2 2 → no score");
}

/// P1 energy 2, P2 energy 4 → P1 has LESS → NO score.
#[test]
fn we_will_less_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ww = trigger_success(&mut game, 2, 4);
    assert_eq!(score(&game, ww), 0, "P1 2 < P2 4 → no score");
}

/// P1 energy 2, P2 energy 0, but P1 has MORE (5) → +1.
/// Confirms it's based on energy counts, not something else.
#[test]
fn we_will_high_margin_gains_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ww = trigger_success(&mut game, 5, 1);
    assert_eq!(score(&game, ww), 1, "P1 5 vs P2 1 (diff 4) → +1 score");
}

/// P1 energy 0, P2 energy 0 → diff 0 → NO score (also no crash).
#[test]
fn we_will_both_zero_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ww = trigger_success(&mut game, 0, 0);
    assert_eq!(score(&game, ww), 0, "P1 0 == P2 0 → no score");
}
