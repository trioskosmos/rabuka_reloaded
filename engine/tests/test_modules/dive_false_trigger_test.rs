//! DIVE! false-trigger regression tests (docs/TEST_HARDENING_PLAN_2026-08-26.md §1).
//!
//! DIVE! (PL!N-bp4-026-L) ab#0 arms only when the card is added from DISCARD to
//! HAND during the owner's own main phase. These tests drive real game flows
//! through actual retrieval effects — no synthetic recently_moved_cards
//! injection — and pin down which sibling abilities may or may not arm it.
//!
//! Key sibling: PL!N-bp4-007-R+ 優木せつ菜 ab#0 — the ONLY effect in the DB
//! performing cross-line discard→hand retrieval
//! (「自分と相手はそれぞれ、自身の控え室からライブカードを1枚手札に加える。」,
//! move_cards source=discard destination=hand target=both).
//! Her ab#0 previously had NO coverage at all.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const DIVE: &str = "PL!N-bp4-026-L";
/// Full-width plus: PL!N-bp4-007-R＋ (same ID shape as l0_gap_constant_test).
const SETSUNA_BOTH: &str = "PL!N-bp4-007-R\u{ff0b}";
/// Single retrieval: 「自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える」
const SETSUNA_SINGLE: &str = "PL!N-bp5-019-N";
/// Another Nijigasaki live card (distinct name from DIVE!) for wrong-target picks.
const OTHER_NIJI_LIVE: &str = "PL!N-bp4-025-L";

fn blade_mod(g: &TestGame, cid: i16) -> i32 {
    g.state
        .mods
        .blade_modifiers
        .get(&cid)
        .map_or(0, |e| e.total())
}

/// Game 1: P1 plays bp4-007 Setsuna during P1's main phase. Her ab#0 makes EACH
/// player retrieve 1 live card from their OWN waitroom. Both waitrooms hold only
/// DIVE! copies, so both retrievals are forced onto DIVE!.
///
/// Expected:
/// - P1's DIVE!: added to hand by P1's OWN effect during P1's main phase
///   → ab#0 ARMS → optional placement offered → accept → live card zone.
/// - P2's DIVE!: added to hand by the OPPONENT's effect during P1's main phase
///   → ab#0 must NOT arm (phase_target=self gate; not P2's main phase).
#[test]
fn both_retrieval_arms_own_dive_not_opponents() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive_p1 = g.id(DIVE);
    let dive_p2 = g.new_id(DIVE);
    let setsuna = g.id(SETSUNA_BOTH);
    let filler = g.id("PL!-sd1-010-SD");

    // Only DIVE! per waitroom → each retrieval is forced onto that copy.
    g.state.player1.hand.cards.push(setsuna);
    g.state.player1.waitroom.cards.push(dive_p1);
    g.state.player2.waitroom.cards.push(dive_p2);
    fill_decks(&mut g, filler);
    g.give_energy(15); // bp4-007 costs 13

    // Debut → ab#0 fires: both players retrieve 1 live card each.
    g.play_to_stage(setsuna, MemberArea::Center);

    // Drain the resolution chain strictly. bp4-007 Setsuna is herself 虹ヶ咲,
    // so after placing DIVE! her... no — DIVE!'s own ab#1 may ask which Niji
    // member gets blade+2 (SelectTarget). Everything else must be SelectCard.
    let mut guard = 0;
    while g.has_pending_choice() {
        guard += 1;
        assert!(guard <= 20, "runaway prompt loop:\n{}", g.pending_choice_summary());
        let ty = g.pending_choice_type().unwrap_or_default();
        match ty.as_str() {
            "SelectCard" => g.select_indices(&[0]),
            "SelectTarget" => g.select_indices(&[0]),
            other => panic!(
                "unexpected prompt {} while resolving both-retrieval:\n{}",
                other,
                g.pending_choice_summary()
            ),
        }
    }

    assert!(
        g.state.player1.live_card_zone.cards.contains(&dive_p1),
        "P1's DIVE! should be placed by own retrieval (ab#0 armed); live={:?}",
        g.state.player1.live_card_zone.cards
    );
    assert!(
        g.state.player2.hand.cards.contains(&dive_p2),
        "P2's DIVE! should have been retrieved into P2's hand by the both-retrieval"
    );
    assert!(
        !g.state.player2.live_card_zone.cards.contains(&dive_p2),
        "P2's DIVE! must NOT auto-place: moved by OPPONENT's effect during \
         P1's main phase — ab#0 must not arm"
    );
}

/// Game 2: single retrieval with a choice of targets. bp5-019 Setsuna offers
/// BOTH Niji live cards in waitroom; the player picks the OTHER live card.
/// DIVE! never moves → ab#0 must never arm and no placement prompt appears.
#[test]
fn wrong_target_retrieval_does_not_arm_dive() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id(DIVE);
    let other_live = g.new_id(OTHER_NIJI_LIVE);
    let setsuna = g.id(SETSUNA_SINGLE);
    let filler = g.id("PL!-sd1-010-SD");
    g.state.player1.hand.cards.push(setsuna);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.waitroom.cards.push(dive);
    g.state.player1.waitroom.cards.push(other_live);
    g.give_energy(10);

    g.play_to_stage(setsuna, MemberArea::Center);

    // Optional discard cost (1 from hand) — pay it.
    assert!(g.has_pending_choice(), "optional discard cost expected");
    g.select_indices(&[0]);

    // Retrieval: two candidates [DIVE!, VIVID WORLD]. Pick the OTHER card (index 1).
    assert!(g.has_pending_choice(), "retrieval selection expected");
    g.assert_selection_contains(DIVE, "DIVE!");
    g.select_indices(&[1]);

    // Resolution finished: nothing else may be pending.
    assert!(
        !g.has_pending_choice(),
        "no further prompts expected after retrieving the non-DIVE card:\n{}",
        g.pending_choice_summary()
    );
    assert!(
        !g.state.player1.live_card_zone.cards.contains(&dive),
        "DIVE! stayed in waitroom — no placement without its own retrieval"
    );
    assert!(
        g.state.player1.waitroom.cards.contains(&dive),
        "DIVE! must still be in the waitroom"
    );
}

/// Game 3 (F2): DIVE! drawn from the MAIN DECK during the natural draw phase
/// must NOT arm ab#0. The printed trigger is 「控え室から手札に加えられたとき」
/// — deck→hand is a different zone change even though it IS P2's own main
/// phase when the draw completes.
#[test]
fn natural_draw_from_deck_does_not_arm_dive() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive_p2 = g.new_id(DIVE);
    let filler = g.id("PL!-sd1-010-SD");

    fill_decks(&mut g, filler);
    put_on_deck_top(&mut g, 2, dive_p2);

    // Advance P1(Main) -> P2 Active -> Energy -> Draw -> Main.
    // The Draw->Main transition performs the real deck draw.
    for _ in 0..4 {
        g.pass();
        // Any prompt appearing right after a pass means an ability armed.
        assert!(
            !g.has_pending_choice(),
            "a prompt appeared during phase progression — DIVE! must not arm \
             off a deck draw:\n{}",
            g.pending_choice_summary()
        );
    }

    assert!(
        g.state.player2.hand.cards.contains(&dive_p2),
        "P2 should have drawn DIVE! from the deck"
    );
    assert!(
        !g.state.player2.live_card_zone.cards.contains(&dive_p2),
        "deck-drawn DIVE! must NOT auto-place"
    );
}

/// Game 4 (F6): DIVE! statically in the live zone with NOTHING moved this turn
/// → ab#1 must not grant blade. The location condition carries
/// movement:"moved"; a static presence is not a placement event.
#[test]
fn ab1_no_blade_when_statically_in_live_zone() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id(DIVE);
    let niji = g.id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.live_card_zone.cards.push(dive);
    g.state.player1.stage.stage = [-1, niji, -1];
    g.state.player1.hand.cards.push(filler);
    fill_decks(&mut g, filler);
    g.state.clear_recently_moved_batch();

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);
    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    assert_eq!(
        blade_mod(&g, niji),
        0,
        "static live-zone presence without movement must not grant blade"
    );
}

/// Game 5 (F7): ab#1 grants exactly +2 once per placement — clearing the
/// movement flags and rescanning must not stack a second +2.
#[test]
fn ab1_rescan_after_flags_cleared_does_not_double_grant() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive = g.id(DIVE);
    let niji = g.new_id("PL!N-PR-003-PR");
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.live_card_zone.cards.push(dive);
    g.state.set_recently_moved_cards(vec![dive]);
    g.state.recently_moved_from_zone = Some("hand".to_string());
    g.state.player1.stage.stage = [-1, niji, -1];
    g.state.player1.hand.cards.push(filler);
    fill_decks(&mut g, filler);

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    assert_eq!(blade_mod(&g, niji), 2, "first grant gives exactly blade+2");

    // Movement flags consumed/cleared — rescan must be silent.
    g.state.clear_recently_moved_batch();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    assert_eq!(
        blade_mod(&g, niji),
        2,
        "rescan after flags cleared must NOT double the blade grant"
    );
}

/// Game 6 (F8): two DIVE! copies in hand, only ONE moved discard→hand.
/// Exactly one placement flow may occur; the untouched copy stays in hand.
#[test]
fn two_dive_copies_only_the_moved_one_places() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let dive_moved = g.id(DIVE);
    let dive_static = g.new_id(DIVE);
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.hand.cards.push(dive_moved);
    g.state.player1.hand.cards.push(dive_static);
    g.state.player1.waitroom.cards.push(filler);
    fill_decks(&mut g, filler);

    // Real movement event: ONLY dive_moved comes discard→hand this batch.
    g.state
        .push_movement_event(dive_moved, "discard", "hand", None, "p1", true);

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    // Accept the (single) placement offer.
    let mut placements = 0;
    let mut guard = 0;
    while g.has_pending_choice() {
        guard += 1;
        assert!(guard <= 10, "runaway prompts:\n{}", g.pending_choice_summary());
        let ty = g.pending_choice_type().unwrap_or_default();
        if ty == "SelectCard" {
            placements += 1;
            g.select_indices(&[0]);
        } else if ty == "SelectTarget" {
            g.select_indices(&[0]);
        } else {
            panic!("unexpected prompt {}:\n{}", ty, g.pending_choice_summary());
        }
    }

    assert_eq!(
        placements, 1,
        "exactly one placement selection expected (the moved copy)"
    );

    let placed = g.state.player1.live_card_zone.cards.contains(&dive_moved);
    let static_placed = g
        .state
        .player1
        .live_card_zone
        .cards
        .contains(&dive_static);
    assert!(
        placed ^ static_placed,
        "exactly one copy may end up in the live zone: moved_placed={} static_placed={}",
        placed,
        static_placed
    );
    assert!(
        !static_placed,
        "the copy that never moved must stay in hand"
    );
}
