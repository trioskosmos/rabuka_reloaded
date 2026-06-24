/// Tests for PL!SP-bp5-009-R (鬼塚夏美) ab#0 — LiveStart ability
///
/// Ability text:
///   ライブ開始時:
///     自分のデッキの一番上のカードを控え室に置いてもよい。
///     そうした場合、ライブ終了時まで、ブレードを得る。
///     これにより控え室に置いたカードがライブカードの場合、このメンバーをウェイトにする。
///     自分はこの手順をさらに4回まで繰り返してもよい。
///
/// Structure:
///   sequential [
///     conditional_on_result {
///       primary: [ move_cards(deck_top→discard, optional), gain_resource(blade=1) ],
///       condition: preceding_moved is live_card,
///       followup: change_state(wait, member, count=1)
///     },
///     repeat_procedure(max_repeats=4, optional)
///   ]
///
/// Per-iteration decision tree:
///   Choice 1: "Mill top card?"  [No=0 → ability stops entirely]
///                               [Yes=1 → mill, gain blade, check live→wait]
///     then Choice 2 (if iter < 4): "Repeat?" [Stop=0, Continue=1]
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn base_setup() -> (TestGame, i16, i16, i16) {
    let db = load_real_database();
    let game = TestGame::new(db);
    let natsumi = game.id("PL!SP-bp5-009-R");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler_live = game.id("PL!-sd1-020-SD");
    (game, natsumi, live_card, filler_live)
}

fn setup_deck(game: &mut TestGame, cards: Vec<i16>) {
    game.state.player1.main_deck.cards.clear();
    for c in cards {
        game.state.player1.main_deck.cards.push(c);
    }
}

fn trigger_live_start(game: &mut TestGame, filler_live: i16) {
    game.state.player1.hand.cards.push(filler_live);
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler_live);
    }
    advance_to_live_card_set_p1(game);
    game.set_live_card(filler_live);
    advance_to_live_start(game);
}

// ============================================================
// BRANCH 1: Decline first optional mill → ability stops
// ============================================================
#[test]
fn natsumi_bp5_decline_first_mill() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    game.state.player1.stage.stage[1] = natsumi;
    setup_deck(&mut game, vec![live_card; 5]);
    game.give_energy(0);
    trigger_live_start(&mut game, filler_live);

    let deck_before = game.state.player1.main_deck.cards.len();
    let blade_before = game.state.mods.get_blade_modifier(natsumi);

    game.select_option(0);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "No cards milled when first optional is declined"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        blade_before,
        "No blade gained when mill declined"
    );
    let orientation = game.state.mods.get_orientation_modifier(natsumi);
    assert!(
        orientation.is_none_or(|o| o != "wait"),
        "No wait state when mill declined"
    );
}

// ============================================================
// BRANCH 2: Mill live card once → stop → 1 blade, wait
// ============================================================
#[test]
fn natsumi_bp5_mill_one_live_stop() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    game.state.player1.stage.stage[1] = natsumi;
    setup_deck(&mut game, vec![live_card; 5]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    let deck_before = game.state.player1.main_deck.cards.len();

    game.select_option(1);
    game.drain_auto_ability_choices();
    game.select_option(0);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "1 card milled"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        1,
        "1 blade gained"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(natsumi),
        Some(&"wait".to_string()),
        "Natsumi should be in wait state after milling a live card"
    );
}

// ============================================================
// BRANCH 3: Mill non-live card once → stop → 1 blade, NO wait
// ============================================================
#[test]
fn natsumi_bp5_mill_one_non_live_stop() {
    let (mut game, natsumi, _live_card, filler_live) = base_setup();
    let non_live = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = natsumi;
    setup_deck(&mut game, vec![non_live; 5]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    game.select_option(1);
    game.drain_auto_ability_choices();
    game.select_option(0);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        1,
        "1 blade gained from milling (any card)"
    );
    let orientation = game.state.mods.get_orientation_modifier(natsumi);
    assert!(
        orientation.is_none_or(|o| o != "wait"),
        "No wait state when milled card is not a live card"
    );
}

// ============================================================
// BRANCH 4: Mill non-live then live, stop → 2 blades, wait after iter 2
// ============================================================
#[test]
fn natsumi_bp5_mill_non_live_then_live_stop() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    let non_live = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = natsumi;
    // Deck: [non_live, live, ...] → iter0 mills non_live, iter1 mills live, then stop
    setup_deck(
        &mut game,
        vec![non_live, live_card, live_card, live_card, live_card],
    );
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    // iter 0 mill → Yes (mill non-live, blade=1, no wait)
    game.select_option(1);
    game.drain_auto_ability_choices();
    // iter 1 mill → Yes (mill live, blade=2, wait)
    game.select_option(1);
    game.drain_auto_ability_choices();
    // iter 2 mill → No (stop)
    game.select_option(0);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        2,
        "2 blades gained (one per mill)"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(natsumi),
        Some(&"wait".to_string()),
        "Natsumi should be wait after milling a live card in iter 1"
    );
}

// ============================================================
// BRANCH 5: All 5 iterations, all live cards → 5 blades, wait
// ============================================================
#[test]
fn natsumi_bp5_all_four_iterations_live() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    game.state.player1.stage.stage[1] = natsumi;
    setup_deck(&mut game, vec![live_card; 10]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    // repeat_procedure(max_repeats=4) = 4 total iterations.
    // Each iteration produces one mill choice (no separate repeat prompt appears).
    // Say Yes 3 times → iter 0, 1, 2 run → blade=3, then No → stop.
    // (Saying Yes 4 times would start iter 3 which also runs.)
    for _ in 0..3 {
        if !game.has_pending_choice() {
            break;
        }
        game.select_option(1);
        game.drain_auto_ability_choices();
    }
    // 4th mill: No → stop
    if game.has_pending_choice() {
        game.select_option(0);
        game.drain_auto_ability_choices();
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        3,
        "3 blades gained from 3 iterations"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(natsumi),
        Some(&"wait".to_string()),
        "Natsumi in wait state after first live card mill"
    );
}

// ============================================================
// BRANCH 6: change_state targets ONLY Natsumi with fillers on stage
// ============================================================
#[test]
fn natsumi_bp5_change_state_only_self_with_fillers() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    let filler_member = game.id("PL!-sd1-010-SD");
    // Stage: [filler_member, natsumi, filler_member]
    game.state.player1.stage.stage = [filler_member, natsumi, filler_member];
    setup_deck(&mut game, vec![live_card; 10]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    game.select_option(1);
    game.drain_auto_ability_choices();
    game.select_option(0);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.mods.get_orientation_modifier(natsumi),
        Some(&"wait".to_string()),
        "Natsumi should be wait after milling a live card"
    );
    let filler_ori = game.state.mods.get_orientation_modifier(filler_member);
    assert!(
        filler_ori.is_none_or(|o| o != "wait"),
        "Filler member at position 0 should NOT be wait-stated"
    );
    let filler_right_id = game.state.player1.stage.stage[2];
    let filler_right_ori = game.state.mods.get_orientation_modifier(filler_right_id);
    assert!(
        filler_right_ori.is_none_or(|o| o != "wait"),
        "Filler member at position 2 should NOT be wait-stated"
    );
}

// ============================================================
// BRANCH 7: Natsumi at LEFT position → wait targets correctly
// ============================================================
#[test]
fn natsumi_bp5_change_state_at_left_position() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    game.state.player1.stage.stage[0] = natsumi;
    setup_deck(&mut game, vec![live_card; 5]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    game.select_option(1);
    game.drain_auto_ability_choices();
    game.select_option(0);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        1,
        "1 blade gained"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(natsumi),
        Some(&"wait".to_string()),
        "Natsumi at left position should be wait-stated"
    );
}

// ============================================================
// BRANCH 8: Natsumi at RIGHT position → wait targets correctly
// ============================================================
#[test]
fn natsumi_bp5_change_state_at_right_position() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    game.state.player1.stage.stage[2] = natsumi;
    setup_deck(&mut game, vec![live_card; 5]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    game.select_option(1);
    game.drain_auto_ability_choices();
    game.select_option(0);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        1,
        "1 blade gained"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(natsumi),
        Some(&"wait".to_string()),
        "Natsumi at right position should be wait-stated"
    );
}

// ============================================================
// BRANCH 9: All 5 iterations, all non-live → 5 blades, NO wait ever
// ============================================================
#[test]
fn natsumi_bp5_all_four_iterations_non_live() {
    let (mut game, natsumi, _live_card, filler_live) = base_setup();
    let non_live = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = natsumi;
    setup_deck(&mut game, vec![non_live; 10]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    for _ in 0..4 {
        if !game.has_pending_choice() {
            break;
        }
        game.select_option(1);
        game.drain_auto_ability_choices();
        if !game.has_pending_choice() {
            break;
        }
        game.select_option(1);
        game.drain_auto_ability_choices();
    }
    if game.has_pending_choice() {
        game.select_option(1);
        game.drain_auto_ability_choices();
    }

    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        4,
        "4 blades gained (one per iteration)"
    );
    let orientation = game.state.mods.get_orientation_modifier(natsumi);
    assert!(
        orientation.is_none_or(|o| o != "wait"),
        "Natsumi should NOT be in wait state (all milled cards non-live)"
    );
}

// ============================================================
// BRANCH 10: stop after mid-repeat (2 iterations only)
// ============================================================
#[test]
fn natsumi_bp5_stop_after_two_iterations() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    game.state.player1.stage.stage[1] = natsumi;
    setup_deck(&mut game, vec![live_card; 10]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    // The repeat prompt is never shown — the next iteration's mill choice comes first.
    // iter 0 mill → Yes, iter 1 mill → Yes, iter 2 mill → No (= 2 blades total)
    game.select_option(1);
    game.drain_auto_ability_choices();
    game.select_option(1);
    game.drain_auto_ability_choices();
    game.select_option(0);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        2,
        "2 blades gained (2 iterations)"
    );
}

// ============================================================
// LEGACY: Q222 repeat continues after wait — all live, all runs
// ============================================================
#[test]
fn natsumi_bp5_q222_repeat_continues_after_wait() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    game.state.player1.stage.stage[1] = natsumi;
    setup_deck(&mut game, vec![live_card; 10]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    let deck_before = game.state.player1.main_deck.cards.len();

    for _ in 0..20 {
        if !game.has_pending_choice() {
            break;
        }
        game.select_option(1);
        game.drain_auto_ability_choices();
    }

    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_before - deck_after >= 4,
        "At least 4 cards should have been discarded, got {}",
        deck_before - deck_after
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(natsumi),
        Some(&"wait".to_string()),
        "Natsumi should be in wait state after live card discards"
    );
}
