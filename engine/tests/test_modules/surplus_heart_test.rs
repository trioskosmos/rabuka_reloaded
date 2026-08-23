/// Tests for 余剰ハート (surplus heart) mechanics — Q173 La Bella Patria.
///
/// ライブ成功時: this turn, IF you had ≥1 surplus heart04 AND a 『虹ヶ咲』
/// member on your stage → place 1 energy card from the energy deck onto the
/// energy zone **in WAIT state** (ウェイト状態で置く — NOT active).
///
/// Assertions are made AT THE ROUND BOUNDARY: driving further passes rolls
/// into the next round whose natural Active phase refreshes waited energy
/// and would pollute both the "waited" check and the placement count.
use crate::helpers::*;


/// Bounded, dispatched drain: auto-selection prompts are skipped; anything
/// unexpected stops the drain so mis-shaped prompts surface as failures
/// downstream instead of being blanket-consumed.
fn drain_auto_prompts(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
}

fn setup_bellas(game: &mut TestGame, keep_second: bool) -> i16 {
    let bella1 = game.new_id("PL!N-bp3-027-L");
    let bella2 = game.new_id("PL!N-bp3-027-L");
    let emma = game.id("PL!N-pb1-008-R"); // 『虹ヶ咲』 member — condition requirement
    let ayumu = game.id("PL!N-PR-003-PR");
    let hasu = game.id("PL!HS-pb1-023-N");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Enough energies for every potential trigger.
    for _ in 0..3 {
        let e = game.new_id("LL-E-001-SD");
        game.state.player1.energy_deck.cards.push(e);
    }

    game.state.player1.stage.stage = [emma, ayumu, hasu];
    game.state.player1.hand.cards.push(bella1);
    if keep_second {
        game.state.player1.hand.cards.push(bella2);
    }
    bella1
}

/// Pass+drain through the live phase into the first normal-phase passes
/// after the rollover — that is where the placement(s) land. Stops once
/// `want` energies are observed (or the window closes).
fn drive_past_placement(game: &mut TestGame, want: usize) {
    for _ in 0..14 {
        if !game.has_pending_choice() {
            game.pass();
        } else {
            drain_auto_prompts(game);
            continue;
        }
        drain_auto_prompts(game);
        // Placements land in the first post-rollover window.
        if game.state.player1.energy_zone.cards.len() >= want
            && game.state.current_turn_phase
                == rabuka_engine::game_state::TurnPhase::FirstAttackerNormal
        {
            break;
        }
    }
}

#[test]
fn bella_q173_two_lives_succeed_both_trigger_waited_placement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_bellas(&mut game, true);

    for _ in 0..5 {
        game.pass();
    }
    // BOTH lives are set back-to-back in P1's own LiveCardSet window
    // (both Bellas sit in P1's hand).
    let h0 = game.state.player1.hand.cards[0];
    let h1 = game.state.player1.hand.cards[1];
    assert_ne!(h0, h1, "two distinct Bella copies in hand");
    game.set_live_card(h0);
    game.set_live_card(h1);
    drive_past_placement(&mut game, 2);

    // Both triggers fired: BOTH energies moved deck→zone…
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        2,
        "both successful Bella lives place one energy each"
    );
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        1,
        "3 seeded − 2 placed = exactly 1 left in the deck"
    );
    // …and the text's ウェイト状態で means they arrive WAITED, not active.
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "Q173: energy must be placed in WAIT state"
    );
}

#[test]
fn bella_q173_single_life_places_single_waited_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_bellas(&mut game, false); // ONE life → ONE trigger

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(game.state.player1.hand.cards[0]);
    drive_past_placement(&mut game, 1);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        1,
        "one successful life → exactly one energy placed (no global fire-once)"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "single placement is also WAITED"
    );
}
