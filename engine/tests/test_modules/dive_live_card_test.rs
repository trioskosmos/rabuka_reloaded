use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn drain_all_and_process(v: &mut TestGame) {
    loop {
        if !v.has_pending_choice() {
            break;
        }
        match v.get_pending_choice().clone() {
            Choice::SelectAutoAbility { .. } => v.select_option(0),
            Choice::SelectCard { .. } => v.select_indices(&[0]),
            _ => v.select_indices(&[]),
        }
    }
}

fn trigger_process_drain(v: &mut TestGame) {
    let pid = v.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut v.state, &pid);
    v.state.process_pending_auto_abilities(&pid);
    drain_all_and_process(v);
}

fn blade_mod(v: &TestGame, cid: i16) -> i32 {
    v.state
        .mods
        .blade_modifiers
        .get(&cid)
        .map_or(0, |e| e.total())
}

// ═══════════════════════════════════════════════════════════════
// DIVE! (PL!N-bp4-026-L)
// ab#0: on_discard_to_hand → condition location:discard
// ab#1: on_placed_in_live_zone → condition location:live_card_zone
// ═══════════════════════════════════════════════════════════════

/// DIVE! in live zone + discard copy + hand copy + Niji on stage.
/// Both abilities fire: ab#0 places hand→live_zone, ab#1 gives blade+2.
#[test]
fn dive_both_abilities_fire() {
    let mut v = TestGame::new(load_real_database());
    let niji = v.id("PL!N-PR-003-PR");

    v.state
        .player1
        .live_card_zone
        .cards
        .push(v.id("PL!N-bp4-026-L"));
    v.state.player1.waitroom.cards.push(v.id("PL!N-bp4-026-L"));
    v.state.player1.hand.cards.push(v.id("PL!N-bp4-026-L"));
    v.state.player1.stage.stage = [-1, niji, -1];

    trigger_process_drain(&mut v);

    assert_eq!(blade_mod(&v, niji), 2, "ab#1: blade+2 for Nijigasaki");
    assert!(
        v.state.player1.live_card_zone.cards.len() >= 2,
        "ab#0: DIVE! placed from hand into live zone"
    );
}

/// DIVE! only in hand (not scannable) → no trigger.
#[test]
fn dive_not_in_live_zone_no_trigger() {
    let mut v = TestGame::new(load_real_database());
    let niji = v.id("PL!N-PR-003-PR");

    v.state.player1.hand.cards.push(v.id("PL!N-bp4-026-L"));
    v.state.player1.stage.stage = [-1, niji, -1];

    trigger_process_drain(&mut v);

    assert_eq!(
        blade_mod(&v, niji),
        0,
        "no trigger: DIVE! must be in live zone"
    );
}

/// DIVE! in live zone only (no discard copy) → only ab#1 fires.
#[test]
fn dive_only_live_zone_only_ab1_triggers() {
    let mut v = TestGame::new(load_real_database());
    let niji = v.id("PL!N-PR-003-PR");

    v.state
        .player1
        .live_card_zone
        .cards
        .push(v.id("PL!N-bp4-026-L"));
    v.state.player1.stage.stage = [-1, niji, -1];

    trigger_process_drain(&mut v);

    assert_eq!(blade_mod(&v, niji), 2, "ab#1 fires: blade+2 from live zone");
}

/// DIVE! ab#0: no DIVE! card in hand → move_cards finds no target → ability
/// fires but the optional placement can't execute. No crash.
#[test]
fn dive_no_dive_card_in_hand_ab0_no_target() {
    let mut v = TestGame::new(load_real_database());
    let niji = v.id("PL!N-PR-003-PR");

    v.state
        .player1
        .live_card_zone
        .cards
        .push(v.id("PL!N-bp4-026-L"));
    v.state.player1.waitroom.cards.push(v.id("PL!N-bp4-026-L"));
    // NO DIVE! in hand
    v.state.player1.stage.stage = [-1, niji, -1];

    trigger_process_drain(&mut v);

    // ab#1 should still fire (blade+2) even though ab#0 had no target
    assert_eq!(
        blade_mod(&v, niji),
        2,
        "ab#1 fires regardless of ab#0 target availability"
    );
    // ab#0 tried to move but no matching card in hand → no-op
}

/// DIVE! live + discard but no Nijigasaki on stage → ab#0 places card, ab#1 no target.
#[test]
fn dive_no_niji_on_stage_ab1_no_target() {
    let mut v = TestGame::new(load_real_database());

    v.state
        .player1
        .live_card_zone
        .cards
        .push(v.id("PL!N-bp4-026-L"));
    v.state.player1.waitroom.cards.push(v.id("PL!N-bp4-026-L"));
    v.state.player1.hand.cards.push(v.id("PL!N-bp4-026-L"));

    trigger_process_drain(&mut v);

    assert!(
        v.state.player1.live_card_zone.cards.len() >= 2,
        "ab#0 places DIVE! even without Nijigasaki target"
    );
}
