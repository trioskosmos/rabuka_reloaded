//! Ability-chain combinations: multi-ability cards where one ability's
//! resolution FEEDS the other. Each test drives the full chain through real
//! triggers, not isolated effect calls.
//!
//! 1. PL!SP-pb2-006 桜小路きな子 — jidou places a 『Liella!』 member under
//!    herself → her 常時 (+1 cost per Liella! under) must reflect it.
//! 2. PL!SP-pb2-011 鬼塚冬毬 — her own ライブ開始時 repositions herself out
//!    of center → that area-move arms her own jidou (Q263) with its
//!    3-option choice; drawing is verified end-to-end.
//! 3. PL!SP-bp4-025-L Special Color — ab#0 (set center blades to 3) and
//!    ab#1 (score +1 if center Liella! moved) resolve in the SAME live.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::types::PositionChangeEvent;

/// Manually move a member between stage slots (toubatsu_q263 idiom) so
/// movement watchers arm exactly like a real reposition would.
fn manually_move(game: &mut TestGame, cid: i16, from: usize, to: usize) {
    game.state.player1.stage.stage[from] = -1;
    game.state.player1.stage.stage[to] = cid;
    game.state.position_change_events.push(PositionChangeEvent {
        moved_card_id: cid,
        old_position: from as u8,
        new_position: to as u8,
        cause_card_id: None,
        cause_player_id: "p1".to_string(),
        effect_only: false,
    });
    game.state.record_card_movement(cid);
    game.state
        .push_movement_event(cid, "stage", "stage", None, "p1", false);
    game.state.position_change_occurred_this_turn = true;
}

// ====================================================================
// 1. SP-pb2-006 — jidou placement feeds its own 常時 cost-up
// ====================================================================

/// Chain: kinako moves area → jidou places 1 『Liella!』 member from discard
/// under herself → 常時 (+1 cost per Liella! under) now shows cost +1.
#[test]
fn pb2006_jidou_placement_feeds_constant_cost_up() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako = game.id("PL!SP-pb2-006-R");
    let liella_member = game.id("PL!SP-pb1-006-R"); // 『Liella!』 member card
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    game.state.player1.stage.stage[1] = kinako;
    game.state.player1.waitroom.cards.push(liella_member);

    // Control: no Liella! under yet → no cost modifier.
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.constant_cost_bonuses.get(&kinako).copied().unwrap_or(0),
        0,
        "control: nothing under kinako → constant grants no cost"
    );

    // Area-move trigger for her jidou (ライブ成功 OR this member moves).
    manually_move(&mut game, kinako, 1, 0);
    let pid = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();

    // The jidou placed the Liella! member under kinako.
    assert_eq!(
        game.state.player1.stage.under_cards[0].len(),
        1,
        "jidou places 1 Liella! member from discard under kinako on area move"
    );
    assert!(
        game.state.player1.stage.under_cards[0].contains(&liella_member),
        "the Liella! member from discard is the card placed under"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&liella_member),
        "placed card left the discard"
    );

    // The chain payoff: her 常時 now counts 1 Liella! under → cost +1.
    // Constant cost bonuses land in constant_cost_bonuses (recalc output).
    game.state.recalculate_constants();
    assert_eq!(
        *game.state.mods.constant_cost_bonuses.get(&kinako).unwrap_or(&0),
        1,
        "常時: 1 『Liella!』 under → cost +1 (fed by the jidou placement)"
    );
}

/// ターン1回 on the jidou: a second move same turn places nothing more.
#[test]
fn pb2006_jidou_once_per_turn_no_second_placement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako = game.id("PL!SP-pb2-006-R");
    let liella_a = game.id("PL!SP-pb1-006-R");
    let liella_b = game.id("PL!SP-pb1-006-R");
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    game.state.player1.stage.stage[1] = kinako;
    game.state.player1.waitroom.cards.push(liella_a);
    game.state.player1.waitroom.cards.push(liella_b);

    manually_move(&mut game, kinako, 1, 0);
    let pid = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();

    // Second move same turn.
    manually_move(&mut game, kinako, 0, 2);
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.player1.stage.under_cards[2].len(),
        0,
        "ターン1回: second area move same turn must NOT place a second card"
    );
}

// ====================================================================
// 2. SP-pb2-011 — own LS reposition feeds own 3-choice jidou (Q263)
// ====================================================================

/// Natural chain: 冬毬 at center resolves her LS (optional self-reposition)
/// → accepting it IS a center-area move → her jidou offers the 3-option
/// choice → choosing the draw option draws exactly 1 card.
#[test]
fn pb2011_own_live_start_reposition_triggers_own_three_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let toubatsu = game.id("PL!SP-pb2-011-R");
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage[1] = toubatsu; // center

    // Her LS resolves (real queue entry → real movement effects).
    fire_trigger(
        &mut game,
        toubatsu,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );

    // Accept the optional reposition, then pick a destination.
    let mut guard = 0;
    let mut drew = false;
    let hand_before = game.state.player1.hand.cards.len();
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectTarget { target, .. }
                if target == "conditional_optional" =>
            {
                game.select_choice_option(1); // accept the reposition
            }
            Choice::SelectTarget { target, .. }
                if target == "position|destination" =>
            {
                game.select_generated(0); // first destination ≠ center
            }
            Choice::SelectTarget { .. } => {
                // The jidou's 3-option choice (blade+2 / enemy wait / draw 1);
                // options arrive index-based, no labels payload.
                game.select_choice_option(2); // draw 1
                drew = true;
            }
            _ => break,
        }
    }
    assert!(drew, "the jidou's 3-option choice must appear after her own reposition");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "choosing the draw option draws exactly 1 card"
    );
    assert_ne!(
        game.state.player1.stage.stage.iter().position(|&c| c == toubatsu),
        Some(1),
        "she left center via her own LS reposition"
    );
}

// ====================================================================
// 3. Special Color — both abilities resolve within the same live
// ====================================================================

/// ab#0 sets the center Liella!'s blades to 3 at Live Start; she then moves
/// (another effect), and ab#1 scores +1 because the center Liella! moved.
#[test]
fn special_color_set_blades_and_score_twin_in_one_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let special = game.id("PL!SP-bp4-025-L");
    let liella = game.id("PL!SP-bp1-001-R"); // Liella!, blade=3
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, liella, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Live Start: ab#0 sets center Liella!'s blades.
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(special);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[0]);
    }
    assert_eq!(
        game.state.mods.get_blade_modifier(liella),
        3,
        "ab#0: center Liella!'s blades set to 3"
    );

    // She then moves out of center (an unrelated effect's reposition).
    manually_move(&mut game, liella, 1, 0);

    // Her Live Success resolves → ab#1 sees the center Liella! moved → +1.
    fire_trigger(
        &mut game,
        special,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    while game.has_pending_choice() {
        game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[0]);
    }

    let total = game
        .state
        .mods
        .score_modifiers
        .get(&special)
        .map(|m| m.total())
        .unwrap_or(0);
    assert_eq!(total, 1, "ab#1: center Liella! moved this turn → +1 score");
    assert_eq!(
        game.state.mods.get_blade_modifier(liella),
        3,
        "ab#0's blade set persists alongside ab#1's score"
    );
}
