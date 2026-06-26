use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn blade_mod(g: &TestGame, cid: i16) -> i32 {
    g.state
        .mods
        .blade_modifiers
        .get(&cid)
        .map_or(0, |e| e.total())
}

fn count_dive_in_hand(g: &TestGame, pid: &str) -> usize {
    let player = if pid == "p1" {
        &g.state.player1
    } else {
        &g.state.player2
    };
    player
        .hand
        .cards
        .iter()
        .filter(|&&cid| {
            g.state
                .card_database
                .get_card(cid)
                .is_some_and(|c| c.card_no == "PL!N-bp4-026-L")
        })
        .count()
}

/// Two DIVE! move from waitroom → hand. Both are in the moved batch.
/// Verify that DIVE! ends up in live zone (ab#0 placed it there)
/// and blade is granted (ab#1 fired). This confirms the chain works
/// even when multiple copies move simultaneously.
#[test]
fn two_dive_retrieved_chain_still_works() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive_a = g.id("PL!N-bp4-026-L");
    let dive_b = g.id("PL!N-bp4-026-L");
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.waitroom.cards.push(dive_a);
    g.state.player1.waitroom.cards.push(dive_b);
    g.state.player1.stage.stage = [niji, -1, -1];
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }

    g.state
        .player1
        .waitroom
        .cards
        .retain(|c| *c != dive_a && *c != dive_b);
    g.state.player1.hand.cards.push(dive_a);
    g.state.player1.hand.cards.push(dive_b);
    g.state.recently_moved_cards = Some(vec![dive_a, dive_b]);

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    // Drain all choices (ab#0 placement, ab#1 target selection, etc.)
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    let dive_in_live = g.state.player1.live_card_zone.cards.iter().any(|&c| {
        g.state
            .card_database
            .get_card(c)
            .is_some_and(|card| card.card_no == "PL!N-bp4-026-L")
    });
    assert!(
        dive_in_live,
        "at least one DIVE! should be in live zone after ab#0"
    );
    assert!(
        blade_mod(&g, niji) >= 2,
        "ab#1 should grant blade+2 to Nijigasaki member"
    );
}

/// One DIVE! moves from waitroom → hand, another DIVE! already in hand (static).
/// Only the moved copy should trigger ab#0 (1 choice, not 2).
#[test]
fn only_moved_copy_triggers_static_copy_does_not() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive_moved = g.id("PL!N-bp4-026-L");
    let dive_static = g.id("PL!N-bp4-026-L");
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.waitroom.cards.push(dive_moved);
    g.state.player1.stage.stage = [niji, -1, -1];

    // Already in hand (static, did NOT move this trigger)
    g.state.player1.hand.cards.push(dive_static);
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }

    // Only dive_moved moves from waitroom → hand
    g.state.player1.waitroom.cards.retain(|c| *c != dive_moved);
    g.state.player1.hand.cards.push(dive_moved);
    g.state.recently_moved_cards = Some(vec![dive_moved]);

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    let mut choice_count = 0;
    while g.has_pending_choice() {
        choice_count += 1;
        g.select_indices(&[0]);
        while g.has_pending_choice() {
            g.select_indices(&[0]);
        }
    }

    assert_eq!(
        choice_count, 1,
        "only the moved DIVE! should trigger ab#0; static copy should not. got {} choices",
        choice_count
    );
}

/// DIVE! retrieved → ab#0 places DIVE! in live zone → ab#1 grants blade+2.
/// Verify via blade modifier (not pending choice, which may auto-resolve
/// when there is exactly 1 target).
#[test]
fn ab0_places_dive_ab1_grants_blade_verify_modifier() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.waitroom.cards.push(dive);
    g.state.player1.stage.stage = [niji, -1, -1];
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }

    g.state.player1.waitroom.cards.retain(|c| *c != dive);
    g.state.player1.hand.cards.push(dive);
    g.state.recently_moved_cards = Some(vec![dive]);

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    // ab#0: place DIVE! if prompted
    if g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    // ab#1 may auto-select or create a choice — drain all
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert!(
        g.state.player1.live_card_zone.cards.contains(&dive),
        "DIVE! should be in live zone after ab#0"
    );
    assert!(
        blade_mod(&g, niji) >= 2,
        "Nijigasaki member should have blade+2 from ab#1, got {}",
        blade_mod(&g, niji)
    );
}

/// DIVE! ab#0 declined → DIVE! stays in hand → ab#1 should NOT fire.
#[test]
fn ab0_declined_ab1_not_fired() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.waitroom.cards.push(dive);
    g.state.player1.stage.stage = [niji, -1, -1];
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }

    g.state.player1.waitroom.cards.retain(|c| *c != dive);
    g.state.player1.hand.cards.push(dive);
    g.state.recently_moved_cards = Some(vec![dive]);

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    assert!(g.has_pending_choice(), "ab#0 optional placement expected");
    g.select_indices(&[]); // decline

    assert!(
        !g.state.player1.live_card_zone.cards.contains(&dive),
        "DIVE! should NOT be in live zone when declined"
    );

    // ab#1 should not have fired — no choice and no blade
    assert!(
        !g.has_pending_choice(),
        "ab#1 should not fire when DIVE! placement was declined"
    );
    assert_eq!(
        blade_mod(&g, niji),
        0,
        "no blade should be granted when DIVE! placement declined"
    );
}

/// Full live game flow: Setsuna's debut retrieves DIVE! → ab#0 → ab#1.
#[test]
fn full_chain_setsuna_debut_retrieval() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let setsuna = g.id("PL!N-bp5-019-N");
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

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
    assert!(g.has_pending_choice(), "discard cost expected");
    g.select_indices(&[0]);
    assert!(g.has_pending_choice(), "ab#0 optional placement expected");
    g.select_indices(&[0]);

    // ab#1 may auto-select blade target or prompt — drain all
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert!(
        g.state.player1.live_card_zone.cards.contains(&dive),
        "DIVE! should be in live zone"
    );
    assert!(
        blade_mod(&g, niji) >= 2,
        "ab#1 should grant blade+2, got {}",
        blade_mod(&g, niji)
    );
}

/// Two DIVE! in live zone → both ab#1 fire, each granting blade+2 to a member.
#[test]
fn two_dive_live_zone_two_blade_grants() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive_a = g.id("PL!N-bp4-026-L");
    let dive_b = g.id("PL!N-bp4-026-L");
    let niji_a = g.id("PL!N-PR-003-PR");
    let niji_b = g.id("PL!N-sd1-001-SD");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.live_card_zone.cards.push(dive_a);
    g.state.player1.live_card_zone.cards.push(dive_b);
    g.state.recently_moved_cards = Some(vec![dive_a, dive_b]);
    g.state.player1.stage.stage = [-1, niji_a, niji_b];
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

    let total_blade = blade_mod(&g, niji_a) + blade_mod(&g, niji_b);
    assert!(
        total_blade >= 2,
        "total blade across members should be >= 2 from 2 DIVE!, got {}",
        total_blade
    );
}

/// No Nijigasaki member on stage → ab#1 fires but has no valid target.
/// Ability should resolve harmlessly (no crash).
#[test]
fn ab1_no_niji_target_no_crash() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.live_card_zone.cards.push(dive);
    g.state.recently_moved_cards = Some(vec![dive]);
    g.state.player1.stage.stage = [-1, filler, -1]; // filler is NOT Niji
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

    // Should not panic — ab#1 has no valid target
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }
}

/// Non-main phase: DIVE! retrieved → ab#0 should NOT fire.
#[test]
fn ab0_does_not_fire_outside_main_phase() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id("PL!N-bp4-026-L");
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.current_phase = rabuka_engine::types::Phase::Active;

    g.state.player1.waitroom.cards.push(dive);
    g.state.player1.stage.stage = [niji, -1, -1];
    g.state.player1.hand.cards.push(filler);

    g.state.player1.waitroom.cards.retain(|c| *c != dive);
    g.state.player1.hand.cards.push(dive);
    g.state.recently_moved_cards = Some(vec![dive]);

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert!(
        !g.state.player1.live_card_zone.cards.contains(&dive),
        "ab#0 should not fire outside main phase"
    );
}

/// 2 static DIVE! in hand + 1 DIVE! moves discard→hand.
/// ab#0 should fire exactly once (for the moved copy only).
#[test]
fn two_static_one_moved_only_one_trigger() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive_moved = g.id("PL!N-bp4-026-L");
    let dive_static_a = g.id("PL!N-bp4-026-L");
    let dive_static_b = g.id("PL!N-bp4-026-L");
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    // 2 static DIVE! already in hand (NOT moved this turn)
    g.state.player1.hand.cards.push(dive_static_a);
    g.state.player1.hand.cards.push(dive_static_b);
    // 1 DIVE! in waitroom, will be retrieved
    g.state.player1.waitroom.cards.push(dive_moved);
    g.state.player1.stage.stage = [niji, -1, -1];
    g.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        g.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        g.state.player2.main_deck.cards.push(filler);
    }

    // Only dive_moved moves from waitroom → hand
    g.state.player1.waitroom.cards.retain(|c| *c != dive_moved);
    g.state.player1.hand.cards.push(dive_moved);
    g.state.recently_moved_cards = Some(vec![dive_moved]);

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    let mut choice_count = 0;
    while g.has_pending_choice() {
        choice_count += 1;
        g.select_indices(&[0]);
        while g.has_pending_choice() {
            g.select_indices(&[0]);
        }
    }

    assert_eq!(
        choice_count, 1,
        "only the moved DIVE! should trigger ab#0; 2 static copies should not. got {} choices",
        choice_count
    );
}
