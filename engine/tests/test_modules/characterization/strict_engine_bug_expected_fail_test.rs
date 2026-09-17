/// STRICT expected-fail tests — no slop. These assert CORRECT behavior per card text.
/// They WILL FAIL on current engine/parser, documenting the bugs found during comprehensive edge hardening.
/// Do not "fix" them by loosening the asserts; fix engine/parser instead.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

// ---------------------------------------------------------------------------
// PB1-007 idx344: cost should be 3 - success_count (clamped 0). Engine currently stays 3.
// ---------------------------------------------------------------------------
fn setup_pb1007_with_success(n: usize) -> (TestGame, i16) {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let me = g.id("PL!-pb1-007-R");
    let lily = g.id("PL!-bp3-014-N");
    g.state.player1.stage.stage[0] = me;
    g.state.player1.stage.stage[1] = lily;
    g.give_energy(10);
    for _ in 0..n {
        let s = g.new_id("PL!N-bp1-025-L");
        g.state.player1.success_live_card_zone.cards.push(s);
    }
    for _ in 0..5 {
        let f = g.new_id("PL!-sd1-010-SD");
        g.state.player1.hand.cards.push(f);
    }
    let mus = g.id("PL!-sd1-020-SD");
    g.state.player1.waitroom.cards.push(mus);
    (g, me)
}

#[test]
fn strict_pb1007_cost_2_with_1_success() {
    let (mut g, me) = setup_pb1007_with_success(1);
    g.activate_ability(me);
    assert!(g.has_pending_choice(), "should prompt for cost");
    let cnt = g.pending_choice_count();
    assert_eq!(cnt, 2, "STRICT: 1 success -> cost 2, engine gives {}", cnt);
}

#[test]
fn strict_pb1007_cost_1_with_2_success() {
    let (mut g, me) = setup_pb1007_with_success(2);
    g.activate_ability(me);
    let cnt = g.pending_choice_count();
    assert_eq!(cnt, 1, "STRICT: 2 success -> cost 1, engine gives {}", cnt);
}

#[test]
fn strict_pb1007_cost_0_with_3_success() {
    // 3 success is win (not reachable), so test 0 success -> cost 3 as baseline
    let (mut g, me) = setup_pb1007_with_success(0);
    g.activate_ability(me);
    let cnt = g.pending_choice_count();
    assert_eq!(cnt, 3, "STRICT: 0 success -> cost 3, engine gives {}", cnt);
}

// ---------------------------------------------------------------------------
// PR045 idx560: only cost7 baton should draw. Engine currently draws for any cost.
// ---------------------------------------------------------------------------
fn try_baton_strict(replaced_no: &str) -> bool {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let replaced = g.id(replaced_no);
    g.state.player1.stage.stage[0] = replaced;
    let me = g.new_id("PL!S-PR-045-PR");
    g.state.player1.hand.cards.push(me);
    g.give_energy(25);
    let d1 = g.new_id("PL!-sd1-010-SD");
    let d2 = g.new_id("PL!-sd1-010-SD");
    g.state.player1.main_deck.cards.push(d1);
    g.state.player1.main_deck.cards.push(d2);
    let before = g.state.player1.main_deck.cards.len();
    g.play_to_stage(me, MemberArea::LeftSide);
    let had = g.has_pending_choice();
    if had { g.select_indices(&[0]); }
    let after = g.state.player1.main_deck.cards.len();
    had && (before - after == 2)
}

#[test]
fn strict_pr045_cost6_should_not_draw() {
    let drew = try_baton_strict("PL!-sd1-003-SD"); // cost6
    assert!(!drew, "STRICT: cost6 baton should NOT draw, engine draws");
}

#[test]
fn strict_pr045_cost8_should_not_draw() {
    let drew = try_baton_strict("PL!SP-bp5-111-R"); // cost8
    assert!(!drew, "STRICT: cost8 baton should NOT draw");
}

#[test]
fn strict_pr045_cost4_should_not_draw() {
    let drew = try_baton_strict("PL!-sd1-001-SD"); // cost4
    assert!(!drew, "STRICT: cost4 baton should NOT draw");
}

// ---------------------------------------------------------------------------
// Keke idx485: 1 Liella under -> cost +1. Engine gives 0.
// ---------------------------------------------------------------------------
#[test]
fn strict_keke_1_liella_cost_plus1() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let keke = g.id("PL!SP-pb2-006-R");
    let liella = g.id("PL!SP-pb2-012-R"); // Liella Kanon
    g.state.player1.stage.stage[0] = keke;
    g.state.player1.stage.under_cards[0].push(liella);
    g.state.recalculate_constants();
    let m = g.state.mods.get_cost_modifier(keke);
    assert_eq!(m, 1, "STRICT: 1 Liella under -> cost +1, engine gives {}", m);
}

#[test]
fn strict_keke_2_liella_cost_plus2() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let keke = g.id("PL!SP-pb2-006-R");
    let l1 = g.id("PL!SP-pb2-012-R");
    let l2 = g.new_id("PL!SP-pb2-012-R");
    g.state.player1.stage.stage[0] = keke;
    g.state.player1.stage.under_cards[0].push(l1);
    g.state.player1.stage.under_cards[0].push(l2);
    g.state.recalculate_constants();
    let m = g.state.mods.get_cost_modifier(keke);
    assert_eq!(m, 2, "STRICT: 2 Liella under -> cost +2, engine gives {}", m);
}

// ---------------------------------------------------------------------------
// Joint Sumire+Wien no-blade yell: both should gain. Engine currently gives 0 for Sumire when Wien present (or vice versa).
// ---------------------------------------------------------------------------
#[test]
fn strict_joint_sumire_wien_both_gain_no_blade() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let sumire = g.id("PL!SP-bp2-015-N");
    let wien = g.id("PL!SP-bp2-021-N");
    let filler = g.id("PL!-sd1-010-SD");
    let m_no_blade = g.id("PL!S-bp2-002-R");
    g.state.player1.stage.stage = [filler, sumire, wien];
    g.state.revealed_cards.clear();
    g.state.revealed_cards.push(m_no_blade);
    g.state.yell_occurred = true;
    g.state.player1.waitroom.cards.push(m_no_blade);
    g.state.trigger_auto_abilities_for_player("p1");
    g.state.process_pending_auto_abilities("p1");
    g.drain_auto_ability_choices();
    let s = g.state.mods.get_heart_modifier(sumire, HeartColor::Heart06);
    let w = g.state.mods.get_heart_modifier(wien, HeartColor::Heart03);
    assert_eq!(s, 1, "STRICT: Sumire should gain heart06 on no-blade joint yell, got {}", s);
    assert_eq!(w, 1, "STRICT: Wien should gain heart03 on no-blade joint yell, got {}", w);
}

// ---------------------------------------------------------------------------
// SP-bp2-004 highest_cost with cost modifier respected (already fixed for state, but cost modifier via mods should flip)
// ---------------------------------------------------------------------------
#[test]
fn strict_highest_cost_respects_modifier() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let sumire = g.id("PL!SP-bp2-004-R"); // 9 at left
    let center = g.id("PL!HS-PR-001-PR"); // 10
    let right = g.id("PL!-sd1-010-SD"); // 4
    g.state.player1.stage.stage = [sumire, center, right];
    g.state.recalculate_constants();
    let h_before = g.state.mods.get_heart_modifier(sumire, HeartColor::Heart03);
    assert_eq!(h_before, 1, "center 10 > left 9 -> gains");
    // Make right effective 12 via +8
    g.state.mods.add_cost_modifier(right, 8);
    g.state.recalculate_constants();
    let h_after = g.state.mods.get_heart_modifier(sumire, HeartColor::Heart03);
    assert_eq!(h_after, 0, "STRICT: right effective 12 > center 10 -> center no longer highest -> no heart, got {}", h_after);
}
