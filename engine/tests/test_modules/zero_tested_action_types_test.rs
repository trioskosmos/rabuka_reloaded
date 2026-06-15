/// Tests for action types that had zero test coverage:
///
/// 1. invalidate_ability — Kanon (PL!SP-bp2-001) can nullify another member's live_start
/// 2. set_blade_type — VIVID WORLD (PL!N-bp4-025-L), Dazzling Game (PL!SP-bp4-023-L)
///
/// (play_baton_touch is tested via Sumire double baton in sumire_bp4_test.rs)
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Kanon's debut: may nullify a Liella! member's live_start abilities.
/// If nullified, followup: add a Liella! card from waitroom to hand.
#[test]
fn kanon_invalidate_liella_live_start() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp2-001-R＋");
    let target = game.id("PL!SP-pb1-001-R"); // Kanon duplicate with live_start ability
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player2.hand.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Stage: the target member that has a live_start ability
    game.state.player1.stage.stage = [-1, target, -1];
    game.state.player1.hand.cards.push(kanon);
    game.give_energy(20);

    // Play Kanon to Center with baton touch (replaces target)
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(kanon),
        None,
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("Play Kanon with baton touch");

    // Process pending auto abilities (debut trigger)
    while game.has_pending_choice() {
        game.select_option(0); // Yes, invalidate
    }
    game.state.process_pending_auto_abilities("p1");

    // Kanon should now be on stage
    assert!(
        game.state.player1.stage.stage.contains(&kanon),
        "Kanon on stage"
    );

    // The followup should have added a Liella! card from waitroom to hand
    assert!(
        game.state.player1.hand.cards.len() >= 1,
        "Hand has at least 1 card"
    );
}

/// VIVID WORLD: both abilities through a real live phase.
/// ab#0 (ライブ開始時): set_blade_type — yell blades become 青ブレード (Blue).
/// ab#1 (ライブ成功時): conditional modify_score — checks yelled 虹ヶ咲 cards for all 6 hearts.
#[test]
fn vivid_world_live_phase_blade_and_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!N-bp4-025-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!N-sd1-006-SD");

    // Multiple yell cards that collectively provide all 6 heart colors
    // (each card matches 虹ヶ咲 group via series check: "ラブライブ！虹ヶ咲学園")
    let h01 = game.id("PL!N-bp1-002-R"); // 上原歩夢 — heart01 in base_heart
    let h02 = game.id("PL!N-bp1-005-R"); // 中須かすみ — heart02
    let h03 = game.id("PL!N-bp1-007-R"); // 桜坂しずく — heart03
    let h04 = game.id("PL!N-bp1-010-R"); // 近江彼方 — heart04
    let h05 = game.id("PL!N-bp1-004-R"); // 朝香果林 — heart05
    let h06 = game.id("PL!N-bp1-003-R"); // エマ・ヴェルデ — heart06

    game.state.player1.main_deck.cards.clear();
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player2.hand.cards.clear();

    // Deck: 6 rainbow members at top for yell, then fillers
    for c in [h01, h02, h03, h04, h05, h06] {
        game.state.player1.main_deck.cards.push(c);
    }
    for _ in 0..34 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [-1, member, -1];
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.player1.energy_zone.cards.clear();
    for _ in 0..30 { game.state.player1.energy_zone.cards.push(filler); }

    // Advance to LiveCardSet phase
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    match game.state.current_phase {
        rabuka_engine::game_state::Phase::Main => {
            game.pass(); game.pass(); game.pass(); game.pass(); game.pass();
        }
        _ => {}
    }
    assert!(
        game.state.current_phase.to_string().contains("LiveCardSet"),
        "Reached LiveCardSet phase"
    );

    game.set_live_card(live_card);
    game.pass();
    game.pass();
    while game.has_pending_choice() { game.select_indices(&[]); }
    game.pass(); game.pass(); game.pass();

    // ab#0: set_blade_type applies 青ブレード to stage members
    assert!(
        !game.state.mods.blade_type_modifiers.is_empty(),
        "VIVID WORLD ab#0: blade_type_modifiers set on stage"
    );

    // ab#1: live_success checks yelled cards. All 6 cards are 虹ヶ咲 members
    // with blade_heart across colors → condition met → score +1
    let score_mod = game.state.mods.get_score_modifier(live_card);
    assert_eq!(score_mod, 1,
        "VIVID WORLD ab#1: 6 虹ヶ咲 yell cards cover all hearts → score +1"
    );
}

    game.state.player1.stage.stage = [-1, member, -1];
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.player1.energy_zone.cards.clear();
    for _ in 0..30 {
        game.state.player1.energy_zone.cards.push(filler);
    }

    // Set up the live card and advance through live phase
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    match game.state.current_phase {
        rabuka_engine::game_state::Phase::Main => {
            game.pass();
            game.pass();
            game.pass();
            game.pass();
            game.pass();
        }
        _ => {}
    }
    assert!(
        game.state.current_phase.to_string().contains("LiveCardSet"),
        "Reached LiveCardSet phase, got {:?}",
        game.state.current_phase
    );

    game.set_live_card(live_card);
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    game.pass();
    game.pass();

    // Check the performance snapshot
    let perf = game
        .state
        .performance_snapshots
        .iter()
        .find(|s| s.player_id == "p1")
        .expect("P1 performance snapshot");

    // ab#0: set_blade_type — should apply to stage members
    // blade_type_modifiers stores per-card BladeColor
    assert!(
        !game.state.mods.blade_type_modifiers.is_empty(),
        "VIVID WORLD ab#0: blade_type_modifiers should be set on stage members"
    );
    // Verify the member has a blade type modifier
    let member_mod = game.state.mods.blade_type_modifiers.get(&member);
    assert!(
        member_mod.is_some(),
        "VIVID WORLD ab#0: stage member should have blade_type modifier"
    );

    // ab#1: live_success condition check — yell cards have all 6 heart colors
    // The filler (PL!-sd1-010-SD) doesn't have all colors, so score should NOT be modified
    let score_mod = game.state.mods.get_score_modifier(live_card);
    assert_eq!(
        score_mod, 0,
        "VIVID WORLD ab#1: condition not met, no score mod"
    );
}
