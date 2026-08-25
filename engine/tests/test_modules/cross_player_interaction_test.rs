//! Cross-player ability interaction tests (mission §both-players).
//!
//! Scenario grounded in rules.txt 9.6.2.3.2 (baton/event ownership) and the
//! engine's opponent-cause watcher hook: an effect that repositions the
//! OPPONENT's member must also arm THAT member's own 「…したとき」 watchers
//! (「対戦相手のカードの効果でも発動する。」 class), even though the cause is
//! the other player's action.
//!
//! Pairing:
//! - PL!HS-pb1-014-R 安養寺姫芽 ab#0 (登場): if ALL your staged members are
//!   『みらくらぱーく！』, position-change 1 opponent member to her FRONT area.
//!   (姫芽 herself is MiraKura, unit=みらくらぱーく！.)
//! - PL!SP-sd2-002-P 唐可可 ab#1 (自動): when THIS member changes position,
//!   gain heart06 until live end — carrying the opponent-effect marker.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

const HIMEKO: &str = "PL!HS-pb1-014-R";
const MIRAKURA_RURINO: &str = "PL!HS-PR-029-PR"; // 大沢瑠璃乃, cost 5, MiraKura
const NON_MIRAKURA_MEMBER: &str = "PL!-sd1-010-SD"; // μ's — breaks the gate
const KOKO: &str = "PL!SP-sd2-002-P";

fn heart06(g: &TestGame, cid: i16) -> i32 {
    g.state.mods.get_heart_modifier(cid, HeartColor::Heart06)
}

/// Both players' abilities chain in one flow: P1 debuts 姫芽 (MiraKura-only
/// gate passes) → she force-repositions P2's 可可 out of P2-right into her
/// front (P2-center) → 可可's OWN area-move watcher fires from the opponent-
/// caused move and grants her heart06 until live end.
#[test]
fn himeko_debut_repositions_opponent_member_and_koko_responds() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let rurino = g.id(MIRAKURA_RURINO);
    let himeko = g.id(HIMEKO);
    let koko = g.id(KOKO);
    let filler = g.id("PL!-sd1-010-SD");

    // P1: one MiraKura member staged; 姫芽 will debut at CENTER (her front is
    // P2's center under both mirror and crossed conventions).
    g.state.player1.stage.stage = [rurino, -1, -1];
    // P2: exactly one member, parked at RIGHT so the forced position change
    // actually displaces her.
    g.state.player2.stage.stage = [-1, -1, koko];
    g.state.player1.hand.cards.push(himeko);
    fill_decks(&mut g, filler);
    g.give_energy(15);

    g.play_to_stage(himeko, MemberArea::Center);

    // 姫芽's debut asks WHICH opponent member to reposition
    // (SelectTarget position|destination, options=["right"]). Non-skippable.
    assert!(
        g.has_pending_choice(),
        "expected the opponent-member selection prompt"
    );
    g.assert_pending_choice_type("SelectTarget", "reposition target choice");
    g.select_indices(&[0]); // options[0] = "right" = 可可's slot

    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    // 1) 姫芽's effect moved 可可 off P2-right…
    assert_ne!(
        g.state.player2.stage.stage[2], koko,
        "可可 must be repositioned away from P2-right"
    );
    // …into her front area (P2-center)…
    assert_eq!(
        g.state.player2.stage.stage[1], koko,
        "可可 should land on P2-center (姫芽's front)"
    );
    // …still on P2's stage: a position change never leaves the stage.
    assert!(
        g.state.player2.stage.stage.contains(&koko),
        "position change keeps the member on the stage"
    );

    // 2) The interaction payoff: 可可's own auto fired on the opponent-caused
    //    position change → heart06 until live end.
    assert_eq!(
        heart06(&g, koko),
        1,
        "可可's area-move watcher must fire even though P1's effect moved her"
    );
}

/// Negative: a non-MiraKura member on P1's stage breaks 姫芽's gate → no
/// reposition, no response from 可可.
#[test]
fn himeko_gate_blocked_no_reposition_no_koko_response() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let rurino = g.id(MIRAKURA_RURINO);
    let himeko = g.id(HIMEKO);
    let koko = g.id(KOKO);
    let outsider = g.id(NON_MIRAKURA_MEMBER);
    let filler = g.id("PL!-sd1-010-SD");

    // A μ's member is already staged → 「みらくらぱーく！のみ」 fails.
    g.state.player1.stage.stage = [outsider, -1, -1];
    g.state.player2.stage.stage = [-1, -1, koko];
    g.state.player1.hand.cards.push(himeko);
    fill_decks(&mut g, filler);
    g.give_energy(15);

    g.play_to_stage(himeko, MemberArea::Center);
    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    assert_eq!(
        g.state.player2.stage.stage[2], koko,
        "gate failed: 可可 must stay at P2-right"
    );
    assert_eq!(
        heart06(&g, koko),
        0,
        "no movement happened, so 可可's watcher must be silent"
    );
}
