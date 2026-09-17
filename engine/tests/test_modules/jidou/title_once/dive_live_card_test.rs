use crate::helpers::*;

fn blade_mod(g: &TestGame, cid: i16) -> i32 {
    g.state
        .mods
        .blade_modifiers
        .get(&cid)
        .map_or(0, |e| e.total())
}

/// DIVE! in live zone + Nijigasaki on stage → ab#1 grants blade+2.
/// (ab#0 requires movement to trigger, not tested here.)
#[test]
fn dive_live_zone_only_ab1_triggers() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let niji = g.id("PL!N-PR-003-PR");

    g.state.player1.live_card_zone.cards.push(dive);
    g.state.set_recently_moved_cards(vec![dive]);
    g.state.player1.stage.stage = [-1, niji, -1];

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert_eq!(blade_mod(&g, niji), 2, "ab#1: blade+2 from live zone");
}

/// DIVE! only in hand (scannable zones empty) → no abilities fire.
#[test]
fn dive_not_in_live_zone_no_trigger() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let niji = g.id("PL!N-PR-003-PR");

    g.state.player1.hand.cards.push(dive);
    g.state.player1.stage.stage = [-1, niji, -1];

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert_eq!(
        blade_mod(&g, niji),
        0,
        "no trigger: DIVE! must be in live zone for ab#1"
    );
}

/// DIVE! in live zone but no Nijigasaki on stage.
/// ab#1 fires (movement gate passes via recently_moved_cards) but has no
/// target → no-op. ab#0 is NOT fired here — DIVE! was never moved from
/// discard to hand, only placed directly in the live zone for this test.
#[test]
fn dive_no_niji_no_target() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");

    g.state.player1.live_card_zone.cards.push(dive);
    g.state.set_recently_moved_cards(vec![dive]);

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    // ab#1 fires but has no Nijigasaki target → no-op.
    assert_eq!(
        g.state.player1.live_card_zone.cards.len(),
        1,
        "no extra placement from ab#0"
    );
}
