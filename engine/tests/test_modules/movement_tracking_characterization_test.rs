//! R1 characterization tests (audit Part 3, Wave-1 prerequisite).
//!
//! These pin the CURRENT semantics of the movement-tracking surface that the
//! planned unification (event log as single source; deleting the shadow
//! fields `recently_moved_cards` / `recently_moved_from_zone` /
//! `cards_moved_this_turn`) must preserve. If a refactor changes any assertion
//! here, either the refactor broke trigger fidelity or these were the
//! load-bearing quirks to replicate deliberately.

use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member

// ====================================================================
// Raw sync semantics of push_movement_event across the five views.
// ====================================================================

#[test]
fn push_movement_event_syncs_all_views() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.new_id(FILLER);

    assert!(game.state.recently_moved_cards.is_none());
    game.state.push_movement_event(card, "hand", "waitroom", None, "p1", false);

    // Batch view
    let batch = &game.state.batch_movements;
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].moved_card_id, card);
    assert_eq!(batch[0].source_zone, rabuka_engine::core::types::ZoneId::Hand);
    assert_eq!(
        batch[0].dest_zone,
        rabuka_engine::core::types::ZoneId::Waitroom
    );
    // Turn view
    assert_eq!(game.state.turn_movements.len(), 1);
    // Recently-batch scratch channel
    let recent = game.state.recently_moved_cards.as_ref().expect("Some after move");
    assert_eq!(recent.as_slice(), &[card]);
    assert_eq!(
        game.state.recently_moved_from_zone.as_deref(),
        Some("hand")
    );
    // Turn-scoped fast lookup
    assert!(game.state.has_card_moved_this_turn(card));
}

#[test]
fn stage_to_stage_move_also_records_area_movement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.new_id(FILLER);

    game.state.push_movement_event(card, "stage", "stage", None, "p1", false);

    assert_eq!(game.state.turn_area_movements.len(), 1);
    assert!(game.state.position_change_occurred_this_turn);
    assert_eq!(game.state.recently_moved_from_zone.as_deref(), Some("stage"));
}

#[test]
fn non_area_moves_do_not_record_area_movement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.new_id(FILLER);

    game.state.push_movement_event(card, "hand", "waitroom", None, "p1", false);

    assert_eq!(game.state.turn_area_movements.len(), 0);
    // The flag is only set by area moves.
    // (Not asserted here: prior state of position_change_occurred_this_turn.)
}

#[test]
fn second_move_appends_and_updates_recent_zone_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let a = game.new_id(FILLER);
    let b = game.new_id(FILLER);

    game.state.push_movement_event(a, "deck_top", "hand", None, "p1", false);
    game.state.push_movement_event(b, "hand", "waitroom", None, "p1", false);

    // recently_* reflects the LATEST batch write: both cards, latest zone wins.
    let recent = game.state.recently_moved_cards.as_ref().expect("Some");
    assert_eq!(recent.as_slice(), &[a, b]);
    assert_eq!(game.state.recently_moved_from_zone.as_deref(), Some("hand"));
    // Turn scope accumulates both.
    assert!(game.state.has_card_moved_this_turn(a));
    assert!(game.state.has_card_moved_this_turn(b));
}

#[test]
fn clear_card_movement_tracking_resets_turn_scope() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let a = game.new_id(FILLER);

    game.state.push_movement_event(a, "deck_top", "hand", None, "p1", false);
    assert!(game.state.has_card_moved_this_turn(a));

    game.state.clear_card_movement_tracking();

    assert!(!game.state.has_card_moved_this_turn(a));
    assert!(game.state.turn_movements.is_empty());
    assert!(game.state.turn_area_movements.is_empty());
}

// ====================================================================
// Integration: real activation flow feeds the turn-scoped views.
// Proteinbar: cost discards 2 hand cards → both must be tracked this turn,
// and each discard must emit a hand->waitroom movement event.
// ====================================================================

#[test]
fn activation_discard_cost_feeds_movement_views() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-sd1-005-PRproteinbar");
    game.state.player1.stage.stage[0] = me;

    let niji = game.id("PL!N-bp3-004-R");
    game.state.player1.waitroom.cards.push(niji);

    let f1 = game.new_id(FILLER);
    let f2 = game.new_id(FILLER);
    game.state.player1.hand.cards.push(f1);
    game.state.player1.hand.cards.push(f2);

    let events_before = game.state.turn_movements.len();

    game.activate_ability(me);
    if game.has_pending_choice() {
        game.select_indices(&[0, 1]);
    }

    // KNOWN GAP (R1 target): cost discards paid through the choice path do
    // NOT feed cards_moved_this_turn nor emit hand->waitroom events. Only the
    // effect-side retrieval is tracked. These assertions pin that divergence
    // so the R1 unification flips them deliberately.
    //
    // Desired post-R1 semantics (uncomment when the choke point lands):
    // assert!(game.state.has_card_moved_this_turn(f1), "fodder 1 tracked");
    // assert!(game.state.has_card_moved_this_turn(f2), "fodder 2 tracked");

    assert!(
        !game.state.has_card_moved_this_turn(f1) && !game.state.has_card_moved_this_turn(f2),
        "current behavior: choice-path cost discards are invisible to turn tracking"
    );

    // The effect-side retrieval DOES emit an event (discard->hand).
    let retrievals = game.state.turn_movements[events_before..]
        .iter()
        .filter(|e| {
            e.source_zone == rabuka_engine::core::types::ZoneId::Discard
                && e.dest_zone == rabuka_engine::core::types::ZoneId::Hand
        })
        .count();
    if retrievals == 0 {
        let dump: Vec<String> = game.state.turn_movements[events_before..]
            .iter()
            .map(|e| {
                format!(
                    "{:?} vs {:?} / {:?} vs {:?}",
                    e.source_zone,
                    rabuka_engine::core::types::ZoneId::Discard,
                    e.dest_zone,
                    rabuka_engine::core::types::ZoneId::Hand
                )
            })
            .collect();
        panic!("PROBE retrievals=0; comparisons: {dump:#?}");
    }
    assert!(
        retrievals >= 1,
        "the waitroom->hand retrieval emits a movement event"
    );
}
