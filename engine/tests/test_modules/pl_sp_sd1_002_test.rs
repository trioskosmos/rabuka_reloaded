use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn drain_auto(v: &mut TestGame) {
    while v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => v.select_indices(&[0]),
            s => eprintln!("  draining: {:?}", s),
        }
    }
    // After draining, if there's still a choice (like yes/no or select card), don't loop
}

/// Keke's debut: place a Liella! cost≤4 from hand into an EMPTY slot
#[test]
fn keke_place_in_empty_slot() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let keke = v.id("PL!SP-sd1-002-SD");
    let liella = v.id("PL!SP-sd1-013-SD"); // cost 4, Liella!, no ability

    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(liella);
    v.state.player1.hand.cards.push(keke);
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    v.give_energy(20);

    // Play Keke from hand to Center (index of keke in hand is 1)
    v.play_to_stage(keke, MemberArea::Center);

    // Process debut auto ability (Keke's debut fires)
    if v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                v.select_indices(&[0]);
            }
            _ => {}
        }
    }

    // The optional effect should now prompt: select a card from hand
    if v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                let idx = v
                    .state
                    .player1
                    .hand
                    .cards
                    .iter()
                    .position(|&c| c == liella)
                    .unwrap();
                v.select_indices(&[idx]);
            }
            _ => v.select_indices(&[]),
        }
    }

    // If position choice (multiple empty slots): pick Left
    if v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectPosition") => {
                v.select_option(0); // left
            }
            _ => v.select_indices(&[]),
        }
    }

    // Keke should be on Center (played), Liella! should be on stage somewhere
    assert_eq!(v.state.player1.stage.stage[1], keke, "Keke at Center");
    assert!(
        v.state.player1.stage.stage.contains(&liella),
        "Liella! card should be on stage"
    );
}

/// Keke's debut: place a Liella! card in an OCCUPIED slot (replaces occupant)
#[test]
fn keke_place_in_occupied_slot() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let keke = v.id("PL!SP-sd1-002-SD");
    let liella = v.id("PL!SP-sd1-013-SD"); // cost 4, Liella!, no ability
    let filler = v.id("PL!-sd1-010-SD"); // cost 4, no ability, NOT Liella!

    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(liella);
    v.state.player1.hand.cards.push(keke);
    // Put filler on Left from a previous turn (simulate by manually placing)
    v.state.player1.stage.stage = [filler, -1, -1];
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    v.give_energy(20);

    v.play_to_stage(keke, MemberArea::Center);

    // Process debut auto ability
    if v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                v.select_indices(&[0]);
            }
            _ => {}
        }
    }

    // Select Liella! from hand
    if v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                let idx = v
                    .state
                    .player1
                    .hand
                    .cards
                    .iter()
                    .position(|&c| c == liella)
                    .unwrap();
                v.select_indices(&[idx]);
            }
            _ => v.select_indices(&[]),
        }
    }

    // Position choice: pick Left (occupied by filler)
    if v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectPosition") => {
                v.select_option(0); // left
            }
            _ => v.select_indices(&[]),
        }
    }

    // Verify: Liella! replaced filler at Left
    assert_eq!(
        v.state.player1.stage.stage[0], liella,
        "Liella! at Left (replaced)"
    );
    assert_eq!(v.state.player1.stage.stage[1], keke, "Keke at Center");
    assert!(
        v.state.player1.waitroom.cards.contains(&filler),
        "Filler moved to waitroom"
    );
}

/// Keke's debut: BLOCKED from placing in an area locked by a debut this turn
#[test]
fn keke_blocked_from_locked_area() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let keke = v.id("PL!SP-sd1-002-SD");
    let liella = v.id("PL!SP-sd1-013-SD"); // cost 4, Liella!
    let starter = v.id("PL!-sd1-010-SD"); // cost 4, no ability

    v.state.player1.hand.cards.clear();
    v.state.player1.hand.cards.push(liella);
    v.state.player1.hand.cards.push(starter);
    v.state.player1.hand.cards.push(keke);

    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }
    v.give_energy(20);

    // Play starter to Center (now Center is locked for this turn)
    v.play_to_stage(starter, MemberArea::Center);

    // Now play Keke to Left (index of keke in hand... we need to track carefully)
    // After playing starter, hand should have [liella, keke]
    // Let's find keke's position
    let _keke_pos = v
        .state
        .player1
        .hand
        .cards
        .iter()
        .position(|&c| c == keke)
        .unwrap();
    TurnEngine::execute_main_phase_action(
        &mut v.state,
        &ActionType::PlayMemberToStage,
        Some(keke),
        None,
        Some(MemberArea::LeftSide),
        Some(false),
    )
    .expect("play Keke failed");

    // Process Keke's debut ability
    if v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                v.select_indices(&[0]);
            }
            _ => {}
        }
    }

    // Select Liella! from hand
    if v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                let idx = v
                    .state
                    .player1
                    .hand
                    .cards
                    .iter()
                    .position(|&c| c == liella)
                    .unwrap();
                v.select_indices(&[idx]);
            }
            _ => v.select_indices(&[]),
        }
    }

    // Center is locked (starter debuted there this turn)
    // Left is locked (Keke debuted there this turn)
    // Only Right should be available
    if v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectPosition") => {
                // Only Right (2) should be available
                v.select_indices(&[0]);
            }
            _ => v.select_indices(&[]),
        }
    }

    // Liella! should be at Right (the only unlocked slot)
    assert_eq!(
        v.state.player1.stage.stage[2], liella,
        "Liella! at Right (only unlocked)"
    );
    assert_eq!(v.state.player1.stage.stage[0], keke, "Keke at Left");
    assert_eq!(v.state.player1.stage.stage[1], starter, "Starter at Center");
}
