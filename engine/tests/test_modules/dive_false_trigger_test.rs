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
