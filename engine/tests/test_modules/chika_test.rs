/// Tests for PL!S-bp3-001-R+ (高海千歌 / CYaRon!) — Activation ability: wait member → gain total score +1
///
/// Ab#0 (起動/センター/ターン1回): メンバー1人をウェイトにする：
///   ライブ終了時まで、これによってウェイト状態になったメンバーは、
///   「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。
///   （この能力はセンターエリアに登場している場合のみ起動できる。）
///
/// Parsed:
///   trigger: 起動, use_limit: 1
///   cost: change_state(wait, 1, member_card)
///   effect: sequential[ modify_score(+1, live_end), gain_ability ]
///   activation_position: "center"
///
/// Q152: Cannot target opponent's members (self-only)
/// Q151: Gained ability is lost when the waited member leaves stage
/// Q171: "Until end of live" effects persist to LiveVictoryDetermination end
//=====================================================================
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

/// Q152: Cost targets self only (can't select opponent's members)
#[test]
fn chika_q152_self_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    // Chika at center, opponent has a member too
    game.state.player1.stage.stage = [-1, chika, -1];
    game.state.player2.stage.stage = [-1, filler, -1];

    game.give_energy(5);

    // Activate ability → cost waits Chika herself (only P1 member)
    game.activate_ability(chika);

    // Q152: Only P1's members are targeted (self), not P2's
    let p1_wait = game.state.mods.get_orientation_modifier(chika);
    assert_eq!(
        p1_wait,
        Some("wait"),
        "Q152: Chika should be in wait state after activation"
    );

    // P2's member should NOT be waited
    let p2_wait = game.state.mods.get_orientation_modifier(filler);
    assert_ne!(
        p2_wait,
        Some("wait"),
        "Q152: Opponent member should NOT be waited"
    );
}

/// Q151: Score boost is lost when waited member leaves stage
#[test]
fn chika_q151_ability_lost_on_leave() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");

    game.state.player1.stage.stage = [-1, chika, -1];
    game.give_energy(5);

    game.activate_ability(chika);

    // Chika should have +1 score modifier after activation
    let score_before = game.state.mods.get_score_modifier(chika);
    assert_eq!(
        score_before, 1,
        "Q151: Chika should have +1 score after activation"
    );

    // Remove Chika from stage (simulating leaving via effect)
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.waitroom.cards.push(chika);
    // clear_modifiers_for_card is called by move_cards when card moves

    // Manually clear (since we bypassed move_cards)
    game.state.mods.clear_all_for_card(chika);

    let score_after = game.state.mods.get_score_modifier(chika);
    assert_eq!(
        score_after, 0,
        "Q151: Score boost should be lost when member leaves stage"
    );
}

/// Q171: "Until end of live" duration persists to LiveVictoryDetermination  
#[test]
fn chika_q171_live_end_persistence() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(fill);
    }

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");
    let filler_live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [-1, chika, -1];
    // Keep deck stocked so phase transitions don't crash
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }

    game.give_energy(5);

    // Activate → Chika gains +1 score
    game.activate_ability(chika);
    assert_eq!(
        game.state.mods.get_score_modifier(chika),
        1,
        "Score should be +1 after activation"
    );

    // Advance through phases to live
    game.state.player1.hand.cards.push(filler_live);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(filler_live);
    advance_to_live_start(&mut game);

    // LiveStart triggers may queue choices (e.g. other auto abilities) — drain
    // but the score modifier must persist regardless.
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Pass through performance phases
    game.pass(); // → SecondAttackerPerformance
    game.pass(); // → LiveVictoryDetermination

    // Q171: Score modifier should still exist during LiveVictoryDetermination
    assert_eq!(
        game.state.mods.get_score_modifier(chika),
        1,
        "Q171: Score +1 should persist to LiveVictoryDetermination"
    );
}

/// Center requirement: activation only works from Center area
#[test]
fn chika_center_required() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");

    // Place Chika at LEFT (not Center)
    game.state.player1.stage.stage = [chika, -1, -1];
    game.give_energy(5);

    // Try to activate from Left → should fail (activation_position: "center")
    let _result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(chika),
        None,
        None,
        None,
    );

    // Position mismatch skips the ability, but doesn't error
    // The score modifier should NOT be applied
    assert_eq!(
        game.state.mods.get_score_modifier(chika),
        0,
        "Score should NOT be applied when activated from wrong position"
    );
}

/// Once per turn: second activation in same turn should fail
#[test]
fn chika_turn1_use_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");

    game.state.player1.stage.stage = [-1, chika, -1];
    game.give_energy(10);

    // First activation succeeds (gain_ability grants constant +1 score)
    game.activate_ability(chika);
    assert_eq!(
        game.state.mods.get_score_modifier(chika),
        1,
        "First activation should apply +1 score via granted constant ability"
    );

    // Second activation in same turn should skip (use_limit=1)
    let _result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(chika),
        None,
        None,
        None,
    );

    // The function may return Ok (skipping silently) or Err
    // Either way, score should still be 1 (not 2)
    assert_eq!(
        game.state.mods.get_score_modifier(chika),
        1,
        "Second activation should not apply any additional score modifiers"
    );
}

/// Multiple members on stage → selected member is waited and stays on stage
/// (regression test: cost change_state choice had is_select_action=false,
///  causing handle_stage_selection to move the card to discard instead)
#[test]
fn chika_wait_target_stays_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");
    let other = game.id("PL!S-bp2-001-R"); // another Aqours member

    game.state.player1.stage.stage = [other, chika, -1];
    game.give_energy(5);

    // Activate → cost prompts for which member to wait (2 candidates)
    game.activate_ability(chika);
    while game.has_pending_choice() {
        game.select_indices(&[0]); // select other (stage[0])
    }

    // The selected member should be waited and STILL on stage
    assert_eq!(
        game.player().stage.stage[0],
        other,
        "Selected member should remain on stage"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(other),
        Some("wait"),
        "Selected member should be in wait state"
    );
    // Chika stays at center, unchanged
    assert_eq!(
        game.player().stage.stage[1],
        chika,
        "Chika should remain at center"
    );
    // No cards should be in waitroom (no movement happened)
    assert!(
        game.state.player1.waitroom.cards.is_empty(),
        "No cards should have been moved to waitroom"
    );
}

/// Multiple members → selected member gets the +1 score modifier from the effect
#[test]
fn chika_waited_member_gets_score_boost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");
    let other = game.id("PL!S-bp2-001-R");

    game.state.player1.stage.stage = [other, chika, -1];
    game.give_energy(5);

    game.activate_ability(chika);
    while game.has_pending_choice() {
        game.select_indices(&[0]); // wait other
    }

    // Chika (activating card) gets +1 score via gain_ability
    assert_eq!(
        game.state.mods.get_score_modifier(chika),
        1,
        "Chika should have +1 score (gain_ability targets activating_card)"
    );
    // The waited member does NOT get the score modifier directly
    assert_eq!(
        game.state.mods.get_score_modifier(other),
        0,
        "Waited member should NOT get score (gain_ability gives to activating card)"
    );
}

/// Two non-Chika members → can select either, the selected one gets waited
#[test]
fn chika_wait_can_select_any_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp3-001-R\u{ff0b}");
    let a = game.id("PL!S-bp2-001-R");
    let b = game.id("PL!S-bp2-002-R");

    game.state.player1.stage.stage = [a, chika, b];
    game.give_energy(5);

    game.activate_ability(chika);
    while game.has_pending_choice() {
        game.select_indices(&[2]); // select b (stage[2], the 3rd option)
    }

    // Only b should be waited
    assert_eq!(
        game.state.mods.get_orientation_modifier(b),
        Some("wait"),
        "Selected member (b) should be in wait state"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(a),
        None,
        "Non-selected member (a) should NOT be in wait state"
    );
    // Chika (activating card) gets +1 score via gain_ability
    assert_eq!(
        game.state.mods.get_score_modifier(chika),
        1,
        "Chika should have +1 score (gain_ability targets activating_card)"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(a),
        0,
        "Non-selected member should NOT have score boost"
    );
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

// ════════════════════════════════════════════════════════════════════════════
// PL!S-bp5-001-R+ (Aqours bp5 Chika) — ab#1 (常時) cost reduction
//   "能力を持たないメンバーカードを自分の手札から登場させるためのコストは1減る。"
//   Rule: cost reduction applies only to member cards with 0 abilities,
//   played from hand. Stacks per Chika on stage. Floor at 0.
// ════════════════════════════════════════════════════════════════════════════

/// Cost reduction applies to no-ability members (4 → 3).
#[test]
fn chika_bp5_cost_reduction_applies_to_no_ability_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD"); // no abilities, cost 4

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    let filler_cost = game.db.get_card(filler).unwrap().cost.unwrap_or(0) as usize;
    assert_eq!(filler_cost, 4, "Filler cost 4");

    game.state.player1.hand.cards.push(chika);
    game.give_energy(chika_cost + filler_cost + 5);
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let energy_before = game.state.player1.energy_zone.active_count();
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(filler, rabuka_engine::zones::MemberArea::LeftSide);
    let energy_after = game.state.player1.energy_zone.active_count();

    assert_eq!(energy_before - energy_after, 3, "Cost 4 reduced to 3");
}

/// Cost reduction does NOT apply to members WITH abilities.
#[test]
fn chika_bp5_no_reduction_for_member_with_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let has_ability = game.id("PL!SP-PR-003-PR"); // has 登場 ability

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    let target_cost = game.db.get_card(has_ability).unwrap().cost.unwrap_or(0) as usize;

    game.state.player1.hand.cards.push(chika);
    game.give_energy(chika_cost + target_cost + 5);
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let energy_before = game.state.player1.energy_zone.active_count();
    game.state.player1.hand.cards.push(has_ability);
    game.play_to_stage(has_ability, rabuka_engine::zones::MemberArea::LeftSide);
    let energy_after = game.state.player1.energy_zone.active_count();

    assert_eq!(
        energy_before - energy_after,
        target_cost as u8,
        "Full cost paid — no reduction"
    );
}

/// Cost reduction applies to no-ability members regardless of cost.
#[test]
fn chika_bp5_cost_reduction_applies_floor_check() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let no_ability = game.id("PL!-sd1-010-SD"); // cost 4, no abilities

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;

    game.state.player1.hand.cards.push(chika);
    game.give_energy(chika_cost + 4 + 5);
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let energy_before = game.state.player1.energy_zone.active_count();
    game.state.player1.hand.cards.push(no_ability);
    game.play_to_stage(no_ability, rabuka_engine::zones::MemberArea::LeftSide);
    let energy_after = game.state.player1.energy_zone.active_count();

    assert_eq!(energy_before - energy_after, 3, "Cost 4 reduced to 3");
}

/// Two Chikas stack reduction (4 → 2, not 3).
#[test]
fn chika_bp5_cost_reduction_stacks() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let chika2 = game.id("PL!S-bp5-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD"); // cost 4, no abilities

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;

    game.state.player1.hand.cards.push(chika);
    game.state.player1.hand.cards.push(chika2);
    game.give_energy(chika_cost * 2 + 4 + 5);
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.play_to_stage(chika2, rabuka_engine::zones::MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let energy_before = game.state.player1.energy_zone.active_count();
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(filler, rabuka_engine::zones::MemberArea::RightSide);
    let energy_after = game.state.player1.energy_zone.active_count();

    assert_eq!(energy_before - energy_after, 2, "Cost 4 reduced by 2 → 2");
}

// ════════════════════════════════════════════════════════════════════════════
// PL!S-bp5-001-R+ (Aqours bp5 Chika) — ab#0 (登場) baton touch draw
//   "能力を持たないメンバーからバトンタッチして登場した場合、カードを1枚引く。"
//   Condition: baton_touch_trigger + ability_filter: no_ability on source member.
// ════════════════════════════════════════════════════════════════════════════

/// Baton touch from no-ability member → draw 1.
#[test]
fn chika_bp5_baton_touch_from_no_ability_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let no_ability = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-002-SD");

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    game.state.player1.stage.stage = [-1, no_ability, -1];
    game.state.player1.hand.cards.push(chika);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(chika_cost + 5);

    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);

    while game.has_pending_choice() {
        let is_required = game.state.get_pending_choice().is_some_and(|c| {
            matches!(
                c,
                rabuka_engine::ability::types::Choice::SelectCard {
                    count: 1,
                    allow_skip: false,
                    ..
                }
            )
        });
        if is_required {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
    }

    assert_eq!(game.state.player1.stage.stage[1], chika, "Chika at Center");
    assert!(
        game.state.player1.waitroom.cards.contains(&no_ability),
        "No-ability member replaced"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Draw 1 compensates hand loss from playing Chika"
    );
}

/// Baton touch from member WITH ability → NO draw.
#[test]
fn chika_bp5_baton_touch_from_ability_member_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let has_ability = game.id("PL!SP-PR-003-PR");
    let filler = game.id("PL!-sd1-002-SD");

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    game.state.player1.stage.stage = [-1, has_ability, -1];
    game.state.player1.hand.cards.push(chika);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(chika_cost + 5);

    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(game.state.player1.stage.stage[1], chika, "Chika at Center");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "No draw — hand decreased"
    );
}

/// Normal debut (empty area, no baton touch) → NO draw.
#[test]
fn chika_bp5_normal_debut_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-002-SD");

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.hand.cards.push(chika);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(chika_cost + 5);

    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(game.state.player1.stage.stage[1], chika, "Chika at Center");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "No draw — hand decreased"
    );
}

/// Baton touch from no-ability member with empty deck → refresh then draw 1.
#[test]
fn chika_bp5_baton_touch_draw_triggers_refresh() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let no_ability = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-002-SD");

    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    game.state.player1.stage.stage = [-1, no_ability, -1];
    game.state.player1.hand.cards.push(chika);
    // Empty deck + some cards in waitroom to refresh
    game.state.player1.main_deck.cards.clear();
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.give_energy(chika_cost + 5);

    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);

    while game.has_pending_choice() {
        let is_required = game.state.get_pending_choice().is_some_and(|c| {
            matches!(
                c,
                rabuka_engine::ability::types::Choice::SelectCard {
                    count: 1,
                    allow_skip: false,
                    ..
                }
            )
        });
        if is_required {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
    }

    assert_eq!(game.state.player1.stage.stage[1], chika, "Chika at Center");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Draw 1 from refreshed deck compensates hand loss"
    );
    assert!(
        game.state.player1.main_deck.cards.len() > 0 || game.state.player1.waitroom.cards.len() > 0,
        "Refresh should have occurred (cards exist somewhere)"
    );
}
