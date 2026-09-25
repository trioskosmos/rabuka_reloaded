use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn blade_mod(g: &TestGame, cid: i16) -> i32 {
    g.state
        .mods
        .blade_modifiers
        .get(&cid)
        .map_or(0, |e| e.total())
}

/// Real-game simulation: DIVE! in discard, Gets retrieved to hand by Setsuna's
/// debut, then ab#0 fires and places it in the live zone, then ab#1 grants
/// blade+2 to a Nijigasaki member.
///
/// No cheating: DIVE! starts in exactly ONE zone (waitroom), not two.
#[test]
fn ab0_places_dive_ab1_grants_blade() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let setsuna = g.id("PL!N-bp5-019-N");
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    // DIVE! only in waitroom — NOT also in hand (real-game state)
    g.state.player1.waitroom.cards.push(dive);
    g.state.player1.hand.cards.push(setsuna);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.stage.stage = [niji, -1, -1];
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }

    g.give_energy(10);
    g.play_to_stage(setsuna, MemberArea::Center);

    // Setsuna: optional discard cost (discard filler)
    assert!(g.has_pending_choice(), "discard cost expected");
    g.select_indices(&[0]);

    // Setsuna retrieves DIVE! (auto-resolve: only 1 Niji live card in waitroom).
    // Then ab#0 fires and asks: place 1 DIVE! from hand to live zone?
    assert!(g.has_pending_choice(), "ab#0 optional placement expected");
    g.select_indices(&[0]); // select the DIVE! in hand

    // After placement, ab#1 asks which Nijigasaki member gets blade+2.
    assert!(g.has_pending_choice(), "ab#1 target selection expected");
    g.select_indices(&[0]); // select the Nijigasaki member

    // ab#0 should have placed DIVE! in live zone
    assert!(
        g.state.player1.live_card_zone.cards.contains(&dive),
        "DIVE! should be in live card zone (ab#0 placed it)"
    );

    // ab#1 should have granted blade+2 to the Nijigasaki member
    assert!(
        blade_mod(&g, niji) >= 2,
        "ab#1 should grant blade+2 to Nijigasaki member, got {}",
        blade_mod(&g, niji)
    );
}

/// ab#0 triggers for P2 during P2's own main phase (phase gate allows).
#[test]
fn ab0_triggers_for_p2_during_p2_main_phase() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    // P2's main phase
    g.state.current_turn_phase = rabuka_engine::types::TurnPhase::SecondAttackerNormal;

    // DIVE! in P2's waitroom, retrieve to P2's hand
    g.state.player2.waitroom.cards.push(dive);
    g.state.player2.hand.cards.push(filler);
    g.state.player2.waitroom.cards.retain(|c| *c != dive);
    g.state.player2.hand.cards.push(dive);
    g.state.set_recently_moved_cards(vec![dive]); // simulate movement tracking

    // P2 has a Nijigasaki member on stage
    g.state.player2.stage.stage = [-1, niji, -1];

    // Trigger auto abilities for P2 (must fire because P2 owns DIVE! and it's P2's main phase)
    let pid = g.state.player2.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    // ab#0 presents optional placement choice; ab#1 then asks for stage target
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert!(
        g.state.player2.live_card_zone.cards.contains(&dive),
        "DIVE! should be in P2's live zone during P2's main phase"
    );
}

/// ab#0 does NOT trigger for P2 during P1's main phase (phase_target: self).
#[test]
fn ab0_no_trigger_for_p2_during_p1_main_phase() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    // P1's main phase (default: FirstAttackerNormal)

    // DIVE! in P2's waitroom, retrieve to P2's hand
    g.state.player2.waitroom.cards.push(dive);
    g.state.player2.hand.cards.push(filler);
    g.state.player2.waitroom.cards.retain(|c| *c != dive);
    g.state.player2.hand.cards.push(dive);
    g.state.set_recently_moved_cards(vec![dive]); // simulate movement tracking

    // P2 has a Nijigasaki member on stage
    g.state.player2.stage.stage = [-1, niji, -1];

    // Trigger auto abilities for P2 (should NOT fire: P2's main phase hasn't started)
    let pid = g.state.player2.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);
    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    assert!(
        !g.state.player2.live_card_zone.cards.contains(&dive),
        "DIVE! should not fire for P2 during P1's main phase"
    );
}

/// ab#0 does NOT trigger outside main phase.
#[test]
fn ab0_no_trigger_outside_main_phase() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.current_phase = rabuka_engine::types::Phase::Active;
    g.state.player1.waitroom.cards.push(dive);
    g.state.player1.hand.cards.push(filler);

    // Manual retrieval
    g.state.player1.waitroom.cards.retain(|c| *c != dive);
    g.state.player1.hand.cards.push(dive);

    // Process auto abilities
    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);
    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    assert!(
        !g.state.player1.live_card_zone.cards.contains(&dive),
        "ab#0 should not fire outside main phase"
    );
}

/// ab#0 does NOT trigger from static hand state (no movement).
#[test]
fn ab0_no_trigger_from_static_hand() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.hand.cards.push(dive);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    g.state.player1.stage.stage = [-1, -1, -1];

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);
    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    assert!(
        !g.state.player1.live_card_zone.cards.contains(&dive),
        "no trigger from static hand state"
    );
}

/// ab#1: two Nijigasaki members — only one gets blade+2.
#[test]
fn ab1_two_niji_members_only_one_gets_blade() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let niji_a = g.id("PL!N-PR-003-PR");
    let niji_b = g.id("PL!N-sd1-001-SD");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.live_card_zone.cards.push(dive);
    g.state.set_recently_moved_cards(vec![dive]);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.stage.stage = [-1, niji_a, niji_b];
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    let mod_a = blade_mod(&g, niji_a);
    let mod_b = blade_mod(&g, niji_b);
    assert!(
        mod_a >= 2 || mod_b >= 2,
        "at least one Nijigasaki member should have blade+2, got a={} b={}",
        mod_a,
        mod_b
    );
    assert!(
        !(mod_a > 0 && mod_b > 0),
        "only one Nijigasaki member should have blade+2, got a={} b={}",
        mod_a,
        mod_b
    );
}

/// ab#1 alone: DIVE! in live zone → blade+2 granted.
#[test]
fn ab1_fires_on_direct_placement() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.live_card_zone.cards.push(dive);
    g.state.set_recently_moved_cards(vec![dive]);
    g.state.player1.stage.stage = [-1, niji, -1];
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert!(
        blade_mod(&g, niji) >= 2,
        "Nijigasaki member should have blade+2 from ab#1, got {}",
        blade_mod(&g, niji)
    );
}
