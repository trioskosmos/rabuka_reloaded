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
    // repeat → Continue
    game.select_option(1);
    game.drain_auto_ability_choices();
    // iter 1 mill → Yes (mill live, blade=2, wait)
    game.select_option(1);
    game.drain_auto_ability_choices();
    // repeat → Stop
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
// BRANCH 5: All four iterations → can only repeat the optional 4 additional times
// ============================================================
#[test]
fn natsumi_bp5_all_four_iterations_live() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    game.state.player1.stage.stage[1] = natsumi;
    setup_deck(&mut game, vec![live_card; 10]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    let deck_before = game.state.player1.main_deck.cards.len();

    // iter 0
    game.select_option(1);
    game.drain_auto_ability_choices();
    // repeat → Continue (1)
    game.select_option(1);
    game.drain_auto_ability_choices();
    // iter 1
    game.select_option(1);
    game.drain_auto_ability_choices();
    // repeat → Continue
    game.select_option(1);
    game.drain_auto_ability_choices();
    // iter 2
    game.select_option(1);
    game.drain_auto_ability_choices();
    // repeat → Continue
    game.select_option(1);
    game.drain_auto_ability_choices();
    // iter 3
    game.select_option(1);
    game.drain_auto_ability_choices();
    // repeat → Continue
    game.select_option(1);
    game.drain_auto_ability_choices();
    // iter 4 (max)
    eprintln!(
        "[TEST_DEBUG] before final select_option pending={:?}",
        game.state.get_pending_choice()
    );
    game.select_option(1);
    game.drain_auto_ability_choices();
    // after iter 4, no more repeat prompt → done

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 5,
        "5 cards milled (initial + 4 repeats)"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        5,
        "5 blades gained (one per mill)"
    );
}

// ============================================================
// BRANCH 6: change_state only targets self, not other fillers
// ============================================================
#[test]
fn natsumi_bp5_change_state_only_self_with_fillers() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    let filler_member = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [filler_member, -1, -1];
    let filler_2 = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[2] = filler_2;
    game.state.player1.stage.stage[1] = natsumi;
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
    // Only natsumi should be in wait state
    assert_eq!(
        game.state.mods.get_orientation_modifier(natsumi),
        Some(&"wait".to_string()),
        "Only Natsumi should be wait"
    );
    assert!(
        game.state
            .mods
            .get_orientation_modifier(filler_member)
            .is_none_or(|o| o != "wait"),
        "Filler should not be wait"
    );
}

// ============================================================
// BRANCH 7: Change state at left position
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
        "1 blade gained at left position"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(natsumi),
        Some(&"wait".to_string()),
        "Left position should be wait"
    );
}

// ============================================================
// BRANCH 8: Change state at right position
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
        "1 blade gained at right position"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(natsumi),
        Some(&"wait".to_string()),
        "Right position should be wait"
    );
}

// ============================================================
// BRANCH 9: All four iterations non-live → no wait
// ============================================================
#[test]
fn natsumi_bp5_all_four_iterations_non_live() {
    let (mut game, natsumi, _live_card, filler_live) = base_setup();
    let non_live = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = natsumi;
    setup_deck(&mut game, vec![non_live; 10]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    let deck_before = game.state.player1.main_deck.cards.len();

    // iter 0: mill → Yes, repeat → No
    game.select_option(1);
    game.drain_auto_ability_choices();
    game.select_option(0);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "Only 1 card milled when stopping early"
    );
    assert_eq!(game.state.mods.get_blade_modifier(natsumi), 1, "1 blade");
    let orientation = game.state.mods.get_orientation_modifier(natsumi);
    assert!(
        orientation.is_none_or(|o| o != "wait"),
        "No wait state when no live card milled"
    );
}

// ============================================================
// BRANCH 10: Stop after two iterations (mill live, then mill non-live, then stop)
// ============================================================
#[test]
fn natsumi_bp5_stop_after_two_iterations() {
    let (mut game, natsumi, live_card, filler_live) = base_setup();
    let non_live = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = natsumi;
    setup_deck(&mut game, vec![live_card, non_live, live_card]);
    game.give_energy(4);
    trigger_live_start(&mut game, filler_live);

    let deck_before = game.state.player1.main_deck.cards.len();

    // iter 0: mill → Yes, repeat → Continue
    game.select_option(1);
    game.drain_auto_ability_choices();
    game.select_option(1);
    game.drain_auto_ability_choices();
    // iter 1: mill → Yes, repeat → Stop
    game.select_option(1);
    game.drain_auto_ability_choices();
    game.select_option(0);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 2,
        "2 cards milled"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(natsumi),
        2,
        "2 blades gained"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(natsumi),
        Some(&"wait".to_string()),
        "Natsumi still in wait from iter 0 live mill"
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

// ============================================================
// Q264: PL!SP-pb2-020-R (鬼塚夏美) — on_yell: discard Liella! live → 2 extra yells
//
// Ability: Auto, 1/turn. When you yell, you may put 1 Liella! live
// card from hand to waitroom. If you do, perform 2 additional yells.
//
// Q264: All members wait + 0 cards revealed → ability NOT trigger.
// ============================================================

#[test]
fn natsumi_pb2_020_q264_no_trigger_when_zero_cards_revealed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-pb2-020-R");
    let liella_live = game.id("PL!SP-sd1-023-SD");
    let fill = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = natsumi;
    game.state.player1.hand.cards.push(liella_live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    game.give_energy(5);

    // Set natsumi to wait so total_blade=0 → yell reveals 0 cards
    game.state.mods.add_orientation_modifier(natsumi, "wait");

    // Advance through LiveCardSet to performance phase
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.set_live_card(liella_live);
    game.pass(); // LiveCardSetP2
    game.pass(); // FirstAttackerPerformance (LiveStart + yell + auto triggers)
    game.pass(); // SecondAttackerPerformance → LiveVictoryDetermination

    // Q264: 0 cards revealed → condition not met → ability should NOT trigger
    // The live card should still be in hand (not discarded by the ability)
    assert!(
        game.state.player1.hand.cards.contains(&liella_live)
            || game
                .state
                .player1
                .live_card_zone
                .cards
                .contains(&liella_live),
        "Q264: Liella! live card should NOT have been discarded"
    );
}
