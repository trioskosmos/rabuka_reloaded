/// Tests for PL!HS-bp6-008 桂城 泉 (Izumi Keijou) — Debut ability:
///
/// Q257: このメンバーが登場したとき、該当のライブカードが控え室にありませんでした。
///       このとき、このメンバーをウェイトにする必要はありますか？
///       → はい、必ずウェイトになります。
///
/// Ability text:
///   登場：このメンバーをウェイトにする。その後、自分の控え室からスコア4以下の
///   『蓮ノ空』のライブカードを1枚手札に加える。
///
/// Step 1 (mandatory): this member → wait (orientation modifier)
/// Step 2 (conditional): from waitroom, score ≤ 4 蓮ノ空 live_card → hand (1 card)
///   − Only runs if there is at least one matching card in waitroom.
///   − If multiple, player chooses.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::{AbilityTrigger, Phase};
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn izumi_energy_needed() -> usize {
    12
}

fn trigger_izumi_live_start(game: &mut TestGame, izumi: i16) {
    game.state.current_phase = Phase::LiveCardSetFirstAttacker;
    fire_trigger(game, izumi, AbilityTrigger::LiveStart, "ライブ開始時");
}

fn is_waited(game: &TestGame, card_id: i16) -> bool {
    game.state.mods.get_orientation_modifier(card_id) == Some("wait")
}

fn trigger_izumi_debut_for(game: &mut TestGame, izumi: i16, side: Side) {
    let card = game.db.get_card(izumi).unwrap();
    let ability = card
        .resolved_abilities()
        .find(|ability| ability.triggers.as_deref() == Some("登場"))
        .unwrap();
    let player_id = match side {
        Side::P1 => game.state.player1.id.clone(),
        Side::P2 => game.state.player2.id.clone(),
    };
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ability.full_text),
        AbilityTrigger::Debut,
        player_id.clone(),
        Some(card.card_no.to_string()),
        Some(izumi),
        None,
        None,
    );
    game.state.activating_card = Some(izumi);
    game.state.process_pending_auto_abilities(&player_id);
}

/// Case A: Waitroom empty → card becomes wait, no second effect, no crash.
#[test]
fn izumi_bp6_q257_waitroom_empty_becomes_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let izumi = game.id("PL!HS-bp6-008-R");

    game.add_to_hand(izumi);
    game.state.player1.waitroom.cards.clear();
    game.give_energy(izumi_energy_needed());

    game.play_to_stage(izumi, MemberArea::Center);

    assert_eq!(
        game.state.mods.get_orientation_modifier(izumi),
        Some("wait"),
        "Q257 Case A: Izumi must become wait even when waitroom is empty"
    );

    assert!(
        !game.has_pending_choice(),
        "Q257 Case A: no choice should appear when waitroom has no matching live card"
    );

    assert!(
        game.state.player1.stage.stage.contains(&izumi),
        "Q257 Case A: Izumi should remain on stage in wait state"
    );
}

/// Case B: Waitroom has exactly 1 matching card → Izumi becomes wait,
/// and that live card moves to hand automatically.
#[test]
fn izumi_bp6_q257_single_match_moves_to_hand_for_both_rarities() {
    for card_no in ["PL!HS-bp6-008-R", "PL!HS-bp6-008-P"] {
        let db = load_real_database();
        let mut game = TestGame::new(db);
        let izumi = game.id(card_no);
        let matching_live = game.id("PL!HS-bp6-026-L");
        let filler = game.id("PL!-sd1-010-SD");

        game.add_to_hand(izumi);
        game.add_to_discard(matching_live);
        game.add_to_discard(filler);
        game.give_energy(izumi_energy_needed());

        game.play_to_stage(izumi, MemberArea::Center);

        assert!(is_waited(&game, izumi), "{card_no} must wait");
        assert!(game.state.player1.hand.cards.contains(&matching_live));
        assert!(game.state.player1.waitroom.cards.contains(&filler));
        assert!(!game.state.player1.waitroom.cards.contains(&matching_live));
    }
}

/// Case C: Waitroom has only non-matching cards → Izumi becomes wait,
/// no second effect runs, no crash.
#[test]
fn izumi_bp6_q257_non_matching_only_no_second_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp6-008-R");
    let wrong_group_live = game.id("PL!-sd1-019-SD");
    let high_score_live = game.id("PL!HS-bp1-022-L");
    let member = game.id("PL!S-PR-022-PR");

    game.add_to_hand(izumi);
    game.add_to_discard(wrong_group_live);
    game.add_to_discard(high_score_live);
    game.add_to_discard(member);
    game.give_energy(izumi_energy_needed());

    game.play_to_stage(izumi, MemberArea::Center);

    assert!(is_waited(&game, izumi));
    assert!(!game.has_pending_choice());
    for card_id in [wrong_group_live, high_score_live, member] {
        assert!(game.state.player1.waitroom.cards.contains(&card_id));
    }
}

#[test]
fn izumi_bp6_q257_multiple_matches_use_filtered_waitroom_indices() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp6-008-R");
    let wrong_group = game.id("PL!S-PR-022-PR");
    let score_five = game.id("PL!HS-bp6-027-L");
    let score_four = game.id("PL!HS-bp6-025-L");
    let score_three = game.id("PL!HS-bp6-026-L");

    game.add_to_hand(izumi);
    for card_id in [wrong_group, score_five, score_four, score_three] {
        game.add_to_discard(card_id);
    }
    game.give_energy(izumi_energy_needed());
    game.play_to_stage(izumi, MemberArea::Center);

    assert!(is_waited(&game, izumi));
    match game.get_pending_choice() {
        Choice::SelectCard {
            zone,
            card_type,
            count,
            allow_skip,
            cost_limit,
            cost_limit_operator,
            filtered_indices,
            ..
        } => {
            assert_eq!(zone, "discard");
            assert_eq!(card_type.as_deref(), Some("live_card"));
            assert_eq!(*count, 1);
            assert!(!*allow_skip);
            assert_eq!(*cost_limit, Some(4));
            assert_eq!(cost_limit_operator.as_deref(), Some("<="));
            assert_eq!(filtered_indices.as_deref(), Some(&[2, 3][..]));
        }
        choice => panic!("expected mandatory SelectCard, got {choice:?}"),
    }

    game.select_waitroom_card_filtered(score_four);

    assert!(game.state.player1.hand.cards.contains(&score_four));
    for card_id in [wrong_group, score_five, score_three] {
        assert!(game.state.player1.waitroom.cards.contains(&card_id));
    }
}

#[test]
fn izumi_bp6_q257_waits_only_izumi_when_other_hasunosora_is_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp6-008-R");
    let ally = game.id("PL!HS-bp6-009-R");

    game.state.player1.stage.stage[0] = ally;
    game.add_to_hand(izumi);
    game.give_energy(izumi_energy_needed());
    game.play_to_stage(izumi, MemberArea::Center);

    assert!(is_waited(&game, izumi));
    assert!(!is_waited(&game, ally));
    assert!(!game.has_pending_choice());
}

#[test]
fn izumi_bp6_q257_p2_uses_only_p2_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp6-008-R");
    let ally = game.id("PL!HS-bp6-009-R");
    let p1_live = game.id("PL!HS-bp6-025-L");
    let p2_live = game.id("PL!HS-bp6-026-L");

    game.state.player2.stage.stage = [ally, izumi, -1];
    game.state.player1.waitroom.cards.push(p1_live);
    game.state.player2.waitroom.cards.push(p2_live);
    trigger_izumi_debut_for(&mut game, izumi, Side::P2);

    assert!(is_waited(&game, izumi));
    assert!(!is_waited(&game, ally));
    assert!(game.state.player1.waitroom.cards.contains(&p1_live));
    assert!(game.state.player2.hand.cards.contains(&p2_live));
    assert!(!game.state.player2.waitroom.cards.contains(&p2_live));
}

#[test]
fn izumi_bp6_q257_debut_fires_after_baton_touch() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp6-008-R");
    let replaced = game.id("PL!HS-bp6-009-R");
    let matching_live = game.id("PL!HS-bp6-025-L");

    game.add_to_stage(MemberArea::Center, replaced);
    game.add_to_hand(izumi);
    game.add_to_discard(matching_live);
    game.give_energy(izumi_energy_needed());
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(izumi),
        None,
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("Izumi baton touch should succeed");

    assert_eq!(game.state.player1.stage.stage[1], izumi);
    assert!(is_waited(&game, izumi));
    assert!(game.state.player1.waitroom.cards.contains(&replaced));
    assert!(game.state.player1.hand.cards.contains(&matching_live));
}

#[test]
fn izumi_bp6_live_start_activates_at_score_two_for_both_rarities() {
    for card_no in ["PL!HS-bp6-008-R", "PL!HS-bp6-008-P"] {
        let db = load_real_database();
        let mut game = TestGame::new(db);
        let izumi = game.id(card_no);
        let score_two = game.id("PL!HS-bp1-020-L");
        let filler = game.id("PL!-sd1-010-SD");

        game.state.player1.stage.stage[0] = izumi;
        game.state.player1.live_card_zone.cards.push(score_two);
        fill_decks(&mut game, filler);

        trigger_izumi_live_start(&mut game, izumi);

        assert!(!is_waited(&game, izumi), "{card_no} score 2");
    }
}

#[test]
fn izumi_bp6_live_start_stays_waited_at_score_three() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp6-008-R");
    let score_three = game.id("PL!HS-bp6-026-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = izumi;
    game.state.mods.add_orientation_modifier(izumi, "wait");
    game.state.player1.live_card_zone.cards.push(score_three);
    fill_decks(&mut game, filler);

    trigger_izumi_live_start(&mut game, izumi);

    assert!(is_waited(&game, izumi));
}

#[test]
fn izumi_bp6_live_start_ignores_opponents_score_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let izumi = game.id("PL!HS-bp6-008-R");
    let opponent_score_two = game.id("PL!HS-bp1-020-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = izumi;
    game.state.mods.add_orientation_modifier(izumi, "wait");
    game.state
        .player2
        .live_card_zone
        .cards
        .push(opponent_score_two);
    fill_decks(&mut game, filler);

    trigger_izumi_live_start(&mut game, izumi);

    assert!(is_waited(&game, izumi));
}
