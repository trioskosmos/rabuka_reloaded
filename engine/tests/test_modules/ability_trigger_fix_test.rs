/// Regression tests for ability trigger fixes.
///
/// Fix 1 (phases.rs): clear_baton_touch_tracking at start of play action
/// Fix 2 (resolver.rs): merge effect.group_names into appearance conditions
/// Fix 3 (condition/card.rs): self-trigger exclusion + ANY not ALL group check
/// Fix 4 (abilities.rs): discard guard also checks condition.target == "self"
/// Fix 5 (abilities.rs): re-scan sets activating_card to skip just-completed card
/// Fix 6 (condition/state.rs): baton touch uses arriving card for group/cost checks
///
/// Cards:
///   PL!HS-bp6-007-R (セラス柳田リリエンフェルト) — auto: EdelNote appears → opponent waits
///   PL!HS-sd1-001-SD (日野下花帆) — auto: baton-touched by cost10+ 蓮ノ空 → activate 2 energy
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// SERASU: self-trigger prevention + group filtering
// ====================================================================
// 自動 ターン1回:
//   自分のステージに『EdelNote』のメンバーが登場したとき、
//   相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。
// ====================================================================

/// Q245: Serasu's own appearance DOES trigger the ability (no "ほかの" in text).
/// The opponent chooses which of their active members to wait.
#[test]
fn serasu_played_to_stage_triggers_self() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let serasu = game.id("PL!HS-bp6-007-R");
    let p2_member = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.state.player2.stage.stage[1] = p2_member;
    game.add_to_hand(serasu);
    game.add_to_hand(filler);
    game.give_energy(15);

    game.play_to_stage(serasu, MemberArea::Center);

    // Serasu's own appearance IS an EdelNote member appearing → ability fires.
    // Opponent has exactly 1 active member → auto-selects, no prompt.

    // Opponent member must now be waited
    let orientation = game.state.mods.orientation_modifiers.get(&p2_member);
    assert_eq!(
        orientation,
        Some(&"wait".to_string()),
        "Opponent member must be waited when Serasu self-plays"
    );

    // Queue should be idle (no pending choices)
    assert!(
        game.state.ability_queue.is_idle(),
        "Ability queue should be idle after auto-resolution"
    );

    // use_limit IS consumed (the ability actually fired)
    let turn = game.state.turn_number;
    let key = (serasu, 0, turn);
    assert!(
        game.state.turn_limited_abilities_used.contains_key(&key),
        "use_limit must be consumed after successful trigger"
    );

    // Opponent member must now be waited
    let orientation = game.state.mods.orientation_modifiers.get(&p2_member);
    assert_eq!(
        orientation,
        Some(&"wait".to_string()),
        "Opponent member must be waited when Serasu self-plays"
    );

    // use_limit IS consumed (the ability actually fired)
    let turn = game.state.turn_number;
    let key = (serasu, 0, turn);
    assert!(
        game.state.turn_limited_abilities_used.contains_key(&key),
        "use_limit must be consumed after successful trigger"
    );
}

/// Non-EdelNote member appears while Serasu is on stage → ability must NOT fire
/// (the appearance condition now filters by group_names from the effect).
#[test]
fn serasu_on_stage_non_edelnote_appears_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let serasu = game.id("PL!HS-bp6-007-R");
    let non_edelnote = game.id("PL!HS-sd1-002-SD"); // DOLLCHESTRA, NOT EdelNote
    let p2_member = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.state.player1.stage.stage[1] = serasu;
    game.state.player2.stage.stage[0] = p2_member;
    game.add_to_hand(non_edelnote);
    game.add_to_hand(filler);
    game.give_energy(12);

    game.play_to_stage(non_edelnote, MemberArea::LeftSide);

    assert!(
        !game.has_pending_choice(),
        "Non-EdelNote appearance must NOT trigger Serasu's ability"
    );
    let orientation = game.state.mods.orientation_modifiers.get(&p2_member);
    assert!(
        orientation.is_none() || orientation == Some(&"active".to_string()),
        "Opponent member must not be waited by wrong-group trigger"
    );
}

/// Serasu on stage + another EdelNote member appears → ability fires exactly once,
/// and the opponent must be prompted to choose which member to wait.
#[test]
fn serasu_edelnote_appears_fires_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let serasu = game.id("PL!HS-bp6-007-R");
    let edelnote_member = game.id("PL!HS-PR-022-PR");
    let p2_member_a = game.id("PL!-sd1-010-SD");
    let p2_member_b = game.id("PL!-sd1-013-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.state.player1.stage.stage[1] = serasu;
    game.state.player2.stage.stage = [p2_member_a, p2_member_b, -1];
    game.add_to_hand(edelnote_member);
    game.add_to_hand(filler);
    game.give_energy(10);

    game.play_to_stage(edelnote_member, MemberArea::LeftSide);

    // Opponent should get exactly one choice prompt
    assert!(
        game.has_pending_choice(),
        "Opponent must have a choice with 2 active members"
    );
    let entry = game
        .state
        .ability_queue
        .current_entry()
        .expect("Queue must have an entry");
    assert_eq!(
        entry.choice_player_id.as_deref(),
        Some("p2"),
        "Choice must be routed to opponent (p2)"
    );

    // Drain the choice
    game.select_indices(&[0]);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.ability_queue.is_idle(),
        "Queue must be idle after single ability resolves"
    );

    // Exactly one member waited, the other stays active
    let a_waited =
        game.state.mods.orientation_modifiers.get(&p2_member_a) == Some(&"wait".to_string());
    let b_waited =
        game.state.mods.orientation_modifiers.get(&p2_member_b) == Some(&"wait".to_string());
    assert_eq!(
        (a_waited as i32) + (b_waited as i32),
        1,
        "Exactly 1 of 2 opponent members must be waited"
    );
}

/// Serasu + non-EdelNote on stage, then EdelNote appears → fires.
/// Group check is ANY (the appearing card is EdelNote), not ALL.
#[test]
fn serasu_edelnote_appears_with_non_edelnote_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let serasu = game.id("PL!HS-bp6-007-R");
    let non_edelnote = game.id("PL!HS-sd1-002-SD");
    let edelnote_member = game.id("PL!HS-PR-022-PR");
    let p2_member_a = game.id("PL!-sd1-010-SD");
    let p2_member_b = game.id("PL!-sd1-013-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.state.player1.stage.stage[0] = non_edelnote;
    game.state.player1.stage.stage[1] = serasu;
    game.state.player2.stage.stage = [p2_member_a, p2_member_b, -1];
    game.add_to_hand(edelnote_member);
    game.add_to_hand(filler);
    game.give_energy(10);

    game.play_to_stage(edelnote_member, MemberArea::RightSide);

    assert!(
        game.has_pending_choice(),
        "Ability must fire despite non-EdelNote on stage"
    );
    game.select_indices(&[0]);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let a_waited =
        game.state.mods.orientation_modifiers.get(&p2_member_a) == Some(&"wait".to_string());
    let b_waited =
        game.state.mods.orientation_modifiers.get(&p2_member_b) == Some(&"wait".to_string());
    assert_eq!(
        (a_waited as i32) + (b_waited as i32),
        1,
        "Exactly 1 of 2 members waited (non-EdelNote present)"
    );
}

// ====================================================================
// HANAHO: baton-touch condition + no-false-trigger
// ====================================================================
// 自動:
//   このメンバーがコスト10以上の『蓮ノ空』のメンバーとバトンタッチして
//   控え室に置かれたとき、エネルギーを2枚アクティブにする。
// ====================================================================

/// Playing Hanaho normally (no baton touch) must NOT trigger her ability.
#[test]
fn hanaho_played_to_stage_no_baton_touch_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanaho = game.id("PL!HS-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(hanaho);
    game.add_to_hand(filler);
    game.give_energy(9);

    let energy_before = game.state.player1.energy_zone.active_count();

    game.play_to_stage(hanaho, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let energy_after = game.state.player1.energy_zone.active_count();
    assert_eq!(
        energy_after,
        energy_before - 9,
        "Energy must decrease by exactly 9 (only play cost, no activation)"
    );

    let turn = game.state.turn_number;
    let key = (hanaho, 0, turn);
    assert!(
        !game.state.turn_limited_abilities_used.contains_key(&key),
        "use_limit must not be recorded for a condition-failed trigger"
    );
}

/// Hanaho played normally must not appear in the ability queue at all
/// (discard-location guard must prevent premature enqueueing).
#[test]
fn hanaho_play_no_baton_touch_queue_stays_empty() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanaho = game.id("PL!HS-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(hanaho);
    game.add_to_hand(filler);
    game.give_energy(9);

    game.play_to_stage(hanaho, MemberArea::Center);

    // After play_to_stage processes everything, the queue must be idle
    // (Hanaho's discard-location ability must not be enqueued from stage)
    let waiting = game.state.ability_queue.pending_entries();
    assert!(
        waiting.is_empty() || waiting.iter().all(|e| e.completed),
        "No pending ability entries for Hanaho when played without baton touch"
    );
}

/// Hanaho baton-touched by a cost 10+ 蓮ノ空 member → ability fires, activates 2 energy.
#[test]
fn hanaho_baton_touch_triggers_activates_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanaho = game.id("PL!HS-sd1-001-SD"); // cost 9
    let arriver = game.id("PL!HS-sd1-006-SD"); // cost 15, みらくらぱーく!, Hasunosora series
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = hanaho;
    game.give_energy(25);

    let energy_before = game.state.player1.energy_zone.active_count();
    assert!(energy_before >= 20, "Must have enough energy for the test");

    game.state.player1.hand.cards.push(arriver);
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(arriver, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&hanaho),
        "Hanaho must be in waitroom after baton touch"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], arriver,
        "Arriver must occupy center after baton touch"
    );

    let energy_after = game.state.player1.energy_zone.active_count();
    assert_eq!(
        energy_after,
        energy_before - (15 - 9) + 2,
        "Must activate exactly 2 energy (cost 6, net -4): got {} expected {}",
        energy_after,
        energy_before - 6 + 2
    );
}

/// Hanaho baton-touched by cost < 10 → must NOT fire.
#[test]
fn hanaho_baton_touch_low_cost_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanaho = game.id("PL!HS-sd1-001-SD"); // cost 9
    let low_cost = game.id("PL!HS-sd1-007-SD"); // cost 4, EdelNote, Hasunosora series (cost < 10)
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = hanaho;
    game.give_energy(20);

    let energy_before = game.state.player1.energy_zone.active_count();

    game.state.player1.hand.cards.push(low_cost);
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(low_cost, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let energy_after = game.state.player1.energy_zone.active_count();
    // low_cost = 4, hanaho = 9. Since cost < 4, the engine pays 0 (cost wraps).
    // The actual cost is card_cost - replaced_cost = 4 - 9 = 0 (saturating).
    // Ability should NOT fire (cost 4 < 10).
    assert!(
        energy_after <= energy_before,
        "Energy must not increase: low-cost baton touch must not activate energy"
    );
}

/// Hanaho baton-touched by a non-蓮ノ空 arriver → must NOT fire.
/// Uses PL!S-sd1-001-SD (Schoo Idol Festival series, not Hasunosora).
#[test]
fn hanaho_baton_touch_wrong_group_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanaho = game.id("PL!HS-sd1-001-SD"); // cost 9
    let wrong_group = game.id("PL!-sd1-006-SD"); // cost 9, NOT Hasunosora series
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = hanaho;
    game.give_energy(20);

    let energy_before = game.state.player1.energy_zone.active_count();

    game.state.player1.hand.cards.push(wrong_group);
    game.state.player1.hand.cards.push(filler);
    // Play to an occupied area triggers auto baton touch
    game.play_to_stage(wrong_group, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let energy_after = game.state.player1.energy_zone.active_count();
    // wrong_group cost = 9, hanaho cost = 9, baton cost = 0
    // Group check: wrong_group is NOT 蓮ノ空 → ability must not activate
    assert!(
        energy_after <= energy_before,
        "Energy must not increase: wrong-group baton touch must not activate"
    );
}

/// Hanaho baton-touched: verify exactly one trigger (no double-trigger).
#[test]
fn hanaho_baton_touch_triggers_exactly_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanaho = game.id("PL!HS-sd1-001-SD");
    let arriver = game.id("PL!HS-sd1-006-SD"); // cost 15
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = hanaho;
    game.give_energy(25);

    let energy_before = game.state.player1.energy_zone.active_count();

    game.state.player1.hand.cards.push(arriver);
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(arriver, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let energy_after = game.state.player1.energy_zone.active_count();
    // - If fires once:  -6 + 2 = -4
    // - If fires twice: -6 + 4 = -2
    // - If zero:        -6
    assert_eq!(
        energy_after,
        energy_before - 6 + 2,
        "Must activate exactly 2 energy (once): got {}, expected {}",
        energy_after,
        energy_before - 6 + 2
    );
    assert_ne!(
        energy_after,
        energy_before - 6 + 4,
        "Must NOT fire twice (double-trigger): got {}, would be {}",
        energy_after,
        energy_before - 6 + 4
    );
    assert_ne!(
        energy_after,
        energy_before - 6,
        "Must fire at least once (zero-trigger): got {}, would be {}",
        energy_after,
        energy_before - 6
    );
}

// ====================================================================
// BATON TOUCH STATE CLEARANCE
// ====================================================================

/// Baton touch tracking fields are cleared between separate play actions.
#[test]
fn baton_touch_cleared_between_actions() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanaho = game.id("PL!HS-sd1-001-SD");
    let arriver = game.id("PL!HS-sd1-006-SD");
    let fresh_card = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-013-SD");

    // Action 1: baton touch Hanaho
    game.state.player1.stage.stage[1] = hanaho;
    game.give_energy(30);
    game.state.player1.hand.cards.push(arriver);
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(arriver, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Action 2: play a plain card to an empty area (no baton touch)
    game.add_to_hand(fresh_card);
    let energy_before = game.state.player1.energy_zone.active_count();
    game.play_to_stage(fresh_card, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // No stale baton touch state should leak into action 2
    let cost = game
        .db
        .get_card(fresh_card)
        .and_then(|c| c.cost)
        .unwrap_or(0) as usize;
    let energy_after = game.state.player1.energy_zone.active_count();
    assert_eq!(
        energy_after,
        energy_before - cost,
        "Second action must pay exactly {} energy (no baton touch activation): got {}, expected {}",
        cost,
        energy_after,
        energy_before - cost
    );

    // Verify tracking fields are clean after action 2
    assert_eq!(
        game.state.baton_touch_count.get("p1").copied().unwrap_or(0),
        0,
        "baton_touch_count must be 0 after cleared second action"
    );
    assert_eq!(
        game.state.baton_touch_replaced_member_id, None,
        "baton_touch_replaced_member_id must be None after cleared second action"
    );
    assert_eq!(
        game.state.baton_touch_arriving_card_id, None,
        "baton_touch_arriving_card_id must be None after cleared second action"
    );
    assert_eq!(
        game.state.baton_touch_zero_cost, false,
        "baton_touch_zero_cost must be false after cleared second action"
    );
}

// ====================================================================
// DOUBLE-TRIGGER REGRESSION
// ====================================================================

/// Single opponent member scenario: Serasu ability must fire exactly once.
#[test]
fn serasu_double_trigger_regression() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let serasu = game.id("PL!HS-bp6-007-R");
    let edelnote_member = game.id("PL!HS-PR-022-PR");
    let p2_member = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-013-SD");

    game.state.player1.stage.stage[1] = serasu;
    game.state.player2.stage.stage[0] = p2_member;
    game.add_to_hand(edelnote_member);
    game.add_to_hand(filler);
    game.give_energy(10);

    game.play_to_stage(edelnote_member, MemberArea::LeftSide);

    // Drain any pending choices (single opponent = auto-resolve)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.ability_queue.is_idle(),
        "Queue must be idle after single ability resolution"
    );

    // Exactly one member must be waited (the single opponent member)
    let waited_count = game
        .state
        .mods
        .orientation_modifiers
        .iter()
        .filter(|(_, v)| *v == "wait")
        .count();
    assert_eq!(
        waited_count, 1,
        "Double-trigger regression: exactly one opponent member must be waited, got {}",
        waited_count
    );

    let p2_mod = game
        .state
        .mods
        .orientation_modifiers
        .get(&p2_member)
        .cloned()
        .unwrap_or_else(|| "active".to_string());
    assert_eq!(
        p2_mod, "wait",
        "The single opponent member must be in wait state, got {:?}",
        p2_mod
    );
}
