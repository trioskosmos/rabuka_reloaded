use crate::helpers::*;
use rabuka_engine::ability::condition::ConditionContext;
use rabuka_engine::card::{ComparisonTarget, ComparisonType, Condition, ConditionCardType};
use rabuka_engine::zones::MemberArea;

/// Helper to calculate total score in P1's success_live_card_zone
fn total_success_score(game: &TestGame) -> u32 {
    let db = game.state.card_database.clone();
    game.state
        .player1
        .success_live_card_zone
        .cards
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter_map(|c| c.score)
        .sum()
}

/// Build a live card ID by name (for readability)
fn live_named(game: &TestGame, name: &str) -> i16 {
    game.id(name)
}

fn score_comparison_condition(operator: Option<&str>) -> Condition {
    Condition::Comparison {
        text: None,
        negation: None,
        phase: None,
        phase_target: None,
        cache: None,
        trigger_event: None,
        comparison_type: Some(ComparisonType::Score),
        comparison_target: Some(ComparisonTarget::Opponent),
        target: None,
        location: None,
        operator: operator.map(Box::from),
        count: None,
        values: None,
        card_type: None,
        group_names: None,
        position: None,
        position_compare: None,
        aggregate: None,
        heart_colors: None,
        scope: None,
        cost_total: None,
        cost_total_operator: None,
        resource_type: None,
        delta: None,
        cost_limit: None,
        source: None,
        comparison_source: None,
        locations: None,
        exclude_group_names: None,
        characters: None,
        exclude_characters: None,
        same_name: None,
        distinct: None,
        all: None,
        all_areas: None,
        exclude_self: None,
        self_target: None,
        destination: None,
        state: None,
        require_position_cards: None,
        temporal: None,
        yell_trigger: None,
        no_excess_heart: None,
        card_property: None,
        ability_filter: None,
        ability_filter_triggers: None,
        baton_touch_trigger: None,
        min_baton_touch_count: None,
        from_state: None,
        to_state: None,
    }
}

fn group_condition() -> Condition {
    Condition::Group {
        text: None,
        negation: None,
        phase: None,
        phase_target: None,
        cache: None,
        trigger_event: None,
        group_names: Some(Box::new(vec!["蓮ノ空".to_string()])),
        all_members: None,
        location: Some(Box::from("stage")),
        target: Some(Box::from("self")),
        heart_colors: None,
        card_type: Some(ConditionCardType::MemberCard),
        operator: None,
        count: None,
        aggregate: None,
        exclude_characters: None,
        temporal: None,
        self_target: None,
        exclude_self: None,
        heart_source: None,
        source: None,
        locations: None,
        position: None,
    }
}

/// Helper: create a score comparison condition
fn score_gt_opponent_condition() -> Condition {
    score_comparison_condition(Some(">"))
}

/// Helper: create compound condition (score > opponent AND 蓮ノ空 on stage)
fn compound_dododo_condition() -> Condition {
    Condition::Compound {
        text: None,
        negation: None,
        phase: None,
        phase_target: None,
        cache: None,
        trigger_event: None,
        operator: Some(Box::from("and")),
        target: None,
        conditions: Some(vec![
            Box::new(score_comparison_condition(Some(">"))),
            Box::new(group_condition()),
        ]),
    }
}

/// RIN (PL!-bp5-005-R): 登場, if success_zone total score >= 6 → gain 1 energy from energy_deck.
///
/// Test: score starts below threshold (2), then rises above (8), and condition responds correctly.
#[test]
fn rin_score_condition_below_then_above() {
    let mut game = TestGame::new(load_real_database());

    let filler = game.id("PL!-sd1-010-SD");
    let rin_a = game.id("PL!-bp5-005-R");
    let rin_b = game.id("PL!-bp5-005-R");
    let low_live = live_named(&game, "PL!-bp3-024-L"); // score 2
    let high_live = live_named(&game, "PL!-bp3-022-L"); // score 5

    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Populate energy deck (RIN moves from energy_deck to energy_zone)
    let energy_card = game.id("LL-E-001-SD");
    for _ in 0..5 {
        game.state.player1.energy_deck.cards.push(energy_card);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(20);

    // Start with score = 2 (below threshold)
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(low_live);
    assert_eq!(total_success_score(&game), 2, "Initial score should be 2");

    // Play first RIN → condition should FAIL (2 < 6)
    let deck_before = game.state.player1.energy_deck.cards.len();
    game.state.player1.hand.cards.push(rin_a);
    game.play_to_stage(rin_a, MemberArea::Center);
    let deck_after = game.state.player1.energy_deck.cards.len();
    assert_eq!(
        deck_after, deck_before,
        "RIN should NOT consume energy deck when score=2 (< 6)"
    );

    // Add score-5 card → total = 7 (now >= 6)
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(high_live);
    assert_eq!(total_success_score(&game), 7);

    // Play another RIN → condition should PASS
    let deck_before2 = game.state.player1.energy_deck.cards.len();
    game.state.player1.hand.cards.push(rin_b);
    game.play_to_stage(rin_b, MemberArea::LeftSide);
    let deck_after2 = game.state.player1.energy_deck.cards.len();
    assert!(
        deck_after2 < deck_before2,
        "RIN should consume 1 energy deck card when score=7 (>= 6), deck {} -> {}",
        deck_before2,
        deck_after2
    );
    assert_eq!(
        deck_before2 - deck_after2,
        1,
        "Exactly 1 energy should be moved"
    );
}

/// Boundary: exactly score=6
#[test]
fn rin_score_condition_at_exactly_6() {
    let mut game = TestGame::new(load_real_database());

    let filler = game.id("PL!-sd1-010-SD");
    let rin = game.id("PL!-bp5-005-R");
    let live_a = live_named(&game, "PL!-bp3-024-L"); // score 2
    let live_b = live_named(&game, "PL!-bp4-019-L"); // score 4

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let energy_card = game.id("LL-E-001-SD");
    for _ in 0..3 {
        game.state.player1.energy_deck.cards.push(energy_card);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(10);

    game.state.player1.success_live_card_zone.cards.push(live_a);
    game.state.player1.success_live_card_zone.cards.push(live_b);
    assert_eq!(total_success_score(&game), 6);

    let deck_before = game.state.player1.energy_deck.cards.len();
    game.state.player1.hand.cards.push(rin);
    game.play_to_stage(rin, MemberArea::Center);
    let deck_after = game.state.player1.energy_deck.cards.len();
    assert!(
        deck_after < deck_before,
        "RIN should activate at exactly score=6 (>= 6)"
    );
}

/// No success zone cards at all
#[test]
fn rin_score_condition_no_cards() {
    let mut game = TestGame::new(load_real_database());

    let filler = game.id("PL!-sd1-010-SD");
    let rin = game.id("PL!-bp5-005-R");

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let energy_card = game.id("LL-E-001-SD");
    for _ in 0..3 {
        game.state.player1.energy_deck.cards.push(energy_card);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(10);

    assert_eq!(total_success_score(&game), 0);

    let deck_before = game.state.player1.energy_deck.cards.len();
    game.state.player1.hand.cards.push(rin);
    game.play_to_stage(rin, MemberArea::Center);
    let deck_after = game.state.player1.energy_deck.cards.len();
    assert_eq!(
        deck_after, deck_before,
        "RIN should NOT activate when score=0"
    );
}

// ======================================================================
// Direct condition evaluation: score > opponent with operator=">"
// Uses real game state + real cards, evaluates through the engine's
// ConditionContext::evaluate_condition() — the same code path used during
// actual gameplay ability resolution.
//
// This tests the fix for continuative adjective forms (高く → ">")
// where the operator was missing when compound conditions used 高く / 低く
// instead of 高い / 低い before かつ/、.
// ======================================================================

/// P1 score=2, P2 score=0, operator=">" → PASS
#[test]
fn score_gt_opponent_p1_higher_passes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dododo = game.id("PL!HS-bp1-023-L"); // score 2
    game.state.player1.success_live_card_zone.cards.push(dododo);
    // P2 success zone is empty → P2 total = 0

    let ctx = ConditionContext::new(&game.state);
    assert!(
        ctx.evaluate_condition(&score_gt_opponent_condition()),
        "P1 score 2 > P2 score 0 should PASS with operator='>'"
    );
}

/// P1 score=2, P2 score=2, operator=">" → FAIL (strictly greater)
#[test]
fn score_gt_opponent_equal_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dododo = game.id("PL!HS-bp1-023-L"); // score 2
    let p2_live = game.new_id("PL!HS-bp1-020-L"); // 365 Days, score 2
    game.state.player1.success_live_card_zone.cards.push(dododo);
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(p2_live);

    let ctx = ConditionContext::new(&game.state);
    assert!(
        !ctx.evaluate_condition(&score_gt_opponent_condition()),
        "P1 score 2 == P2 score 2 should FAIL with operator='>' (strictly greater)"
    );
}

/// P1 score=2, P2 score=4, operator=">" → FAIL
#[test]
fn score_gt_opponent_p1_lower_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dododo = game.id("PL!HS-bp1-023-L"); // score 2
    let p2_a = game.new_id("PL!HS-bp1-020-L"); // 365 Days, score 2
    let p2_b = game.new_id("PL!HS-bp1-020-L"); // 365 Days, score 2
    game.state.player1.success_live_card_zone.cards.push(dododo);
    game.state.player2.success_live_card_zone.cards.push(p2_a);
    game.state.player2.success_live_card_zone.cards.push(p2_b);

    let ctx = ConditionContext::new(&game.state);
    assert!(
        !ctx.evaluate_condition(&score_gt_opponent_condition()),
        "P1 score 2 < P2 score 4 should FAIL with operator='>'"
    );
}

/// Compound condition: score > opponent AND 蓮ノ空 on stage → both true → PASS
#[test]
fn compound_dododo_hasunosora_on_stage_passes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // P1 score setup: dododo alone → score 2
    let dododo = game.id("PL!HS-bp1-023-L");
    game.state.player1.success_live_card_zone.cards.push(dododo);

    // P1 stage: 蓮ノ空 no-ability member (PL!HS-pb1-023-N)
    let hasunosora_member = game.id("PL!HS-pb1-023-N");
    game.state.player1.stage.stage = [-1, hasunosora_member, -1];

    let ctx = ConditionContext::new(&game.state);
    assert!(
        ctx.evaluate_condition(&compound_dododo_condition()),
        "P1 score 2 > P2 score 0 AND 蓮ノ空 on stage → should PASS"
    );
}

/// Compound condition: score > opponent true, but NO 蓮ノ空 member → FAIL
#[test]
fn compound_dododo_no_hasunosora_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // P1 score setup: dododo alone → score 2
    let dododo = game.id("PL!HS-bp1-023-L");
    game.state.player1.success_live_card_zone.cards.push(dododo);

    // P1 stage: non-蓮ノ空 member (filler)
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, filler, -1];

    let ctx = ConditionContext::new(&game.state);
    assert!(
        !ctx.evaluate_condition(&compound_dododo_condition()),
        "P1 score 2 > P2 score 0 but NO 蓮ノ空 on stage → should FAIL"
    );
}

/// Compound condition: score > opponent FAILS (P2 has higher score)
/// even though 蓮ノ空 member is present → FAIL
#[test]
fn compound_dododo_tied_score_with_hasunosora_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // P1 score: dododo (2)
    let dododo = game.id("PL!HS-bp1-023-L");
    game.state.player1.success_live_card_zone.cards.push(dododo);

    // P2 score: 365 Days (2) — equal
    let p2_live = game.new_id("PL!HS-bp1-020-L");
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(p2_live);

    // P1 stage: 蓮ノ空 member (group condition would pass)
    let hasunosora_member = game.id("PL!HS-pb1-023-N");
    game.state.player1.stage.stage = [-1, hasunosora_member, -1];

    let ctx = ConditionContext::new(&game.state);
    assert!(
        !ctx.evaluate_condition(&compound_dododo_condition()),
        "P1 score 2 == P2 score 2 AND 蓮ノ空 on stage → should FAIL (score not >)"
    );
}

// ======================================================================
// Group condition verification: 蓮ノ空 group condition
// ======================================================================

/// 蓮ノ空 member on stage → PASS
#[test]
fn hasunosora_group_member_present_passes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hasunosora = game.id("PL!HS-pb1-023-N");
    game.state.player1.stage.stage = [-1, hasunosora, -1];

    let ctx = ConditionContext::new(&game.state);
    assert!(
        ctx.evaluate_condition(&group_condition()),
        "蓮ノ空 member on stage → group condition should PASS"
    );
}

/// No 蓮ノ空 member on stage → FAIL
#[test]
fn hasunosora_group_no_member_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, filler, -1];

    let ctx = ConditionContext::new(&game.state);
    assert!(
        !ctx.evaluate_condition(&group_condition()),
        "Non-蓮ノ空 member on stage → group condition should FAIL"
    );
}

/// Mixed: 蓮ノ空 member + non-蓮ノ空 members → still PASS (at least one)
#[test]
fn hasunosora_group_mixed_stage_passes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hasunosora = game.id("PL!HS-pb1-023-N");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [filler, hasunosora, filler];

    let ctx = ConditionContext::new(&game.state);
    assert!(
        ctx.evaluate_condition(&group_condition()),
        "蓮ノ空 member + other members on stage → group condition should PASS (at least one)"
    );
}

// ======================================================================
// Missing-operator regression test: without operator=">", the engine
// defaults to pass-through (always true).  Test that operator=">"
// IS properly required for correct evaluation.
// ======================================================================

/// Without operator, compare_counts returns true regardless — this is the
/// default fallback. Test that having NO operator gives wrong answer,
/// confirming that the parser fix (adding operator=">") is essential.
#[test]
fn score_comparison_without_operator_always_passes() {
    let db = load_real_database();
    let game = TestGame::new(db);

    // P1 score = 0 (no cards), P2 score = 0 (no cards) — equal
    let condition_no_op = score_comparison_condition(None);

    let ctx = ConditionContext::new(&game.state);
    assert!(
        ctx.evaluate_condition(&condition_no_op),
        "Without operator, compare_counts returns true even when scores are equal (0 == 0)"
    );

    // Now verify that WITH operator=">" it correctly fails for equal scores
    let condition_with_op = score_comparison_condition(Some(">"));
    assert!(
        !ctx.evaluate_condition(&condition_with_op),
        "With operator='>', 0 == 0 should FAIL"
    );
}
