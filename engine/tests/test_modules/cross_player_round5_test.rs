//! Cross-player round 5 — seat-relative LiveSuccess condition evaluation.
//!
//! Card under test: PL!S-bp6-022-L 近未来ハッピーエンド (ライブ成功時):
//! 「相手のエネルギーが自分より多い場合、このカードのスコアを＋１する。」
//!
//! Rules grounding:
//! - 4.7.4: 「単に'エネルギー'を参照する場合、エネルギー置き場のカードを参照」
//!   → orientation-independent, so WAITED energy counts for the comparison.
//! - 8.4.4→8.4.5→8.4.6: the ライブ成功 event fires per player during victory
//!   determination and its effects resolve BEFORE the score comparison — both
//!   players' copies evaluate in the same phase, each against its own opponent.
//!
//! Prior coverage gap: batch12 only ever armed this from P1's seat
//! (`fire_trigger` hardcodes pid=p1). The engine resolves "self"/"opponent"
//! through `ability_master_id()` + the activating card's zone location; a
//! swapped-operand or seat-hardcoding bug there is invisible from P1-only
//! tests because "P1's opponent" and "the other seat" coincide.

use crate::helpers::*;

const LIVE: &str = "PL!S-bp6-022-L";

fn score_mod(game: &TestGame, cid: i16) -> i32 {
    game.state.mods.get_score_modifier(cid)
}

/// Fire the ライブ成功時 ability of `cid` as `seat` ("p1"/"p2"). Mirrors
/// helpers::fire_trigger but takes the activating player explicitly — the
/// whole point of this file is that the seat matters.
fn fire_live_success_as(game: &mut TestGame, cid: i16, seat: &str) {
    use rabuka_engine::core::types::AbilityTrigger;
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .expect("card lacks a ライブ成功時 ability");
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveSuccess,
        seat.to_string(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&seat.to_string());
}

fn give_total_energy(game: &mut TestGame, seat: usize, active: usize, waited: usize) {
    let n = active + waited;
    let mut cards = Vec::with_capacity(n);
    for _ in 0..n {
        cards.push(game.id("LL-E-001-SD"));
    }
    let player = match seat {
        0 => &mut game.state.player1,
        _ => &mut game.state.player2,
    };
    for e in cards {
        player.energy_zone.cards.push(e);
    }
    player.energy_zone.set_active_count(active as u8);
}

/// A: P2-owned copy fired AS P2. P1 has more energy → P2's copy scores +1;
/// flipped energies → it does not. Every prior test in the corpus arms this
/// card from P1 only.
#[test]
fn p2_owned_copy_compares_against_p1_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p2_live = game.id(LIVE);
    game.state.player2.live_card_zone.cards.push(p2_live);

    // P1=3 total, P2=1 total → P2's opponent (P1) is strictly greater → +1.
    give_total_energy(&mut game, 0, 3, 0);
    give_total_energy(&mut game, 1, 1, 0);

    fire_live_success_as(&mut game, p2_live, "p2");

    assert_eq!(
        score_mod(&game, p2_live),
        1,
        "P2-owned copy: opponent(P1)=3 > self(P2)=1 must yield +1"
    );
}

#[test]
fn p2_owned_copy_no_bonus_when_p1_not_strictly_ahead() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p2_live = game.id(LIVE);
    game.state.player2.live_card_zone.cards.push(p2_live);

    // P1=1, P2=3 → P2's opponent is NOT greater → no bonus.
    give_total_energy(&mut game, 0, 1, 0);
    give_total_energy(&mut game, 1, 3, 0);

    fire_live_success_as(&mut game, p2_live, "p2");

    assert_eq!(
        score_mod(&game, p2_live),
        0,
        "P2-owned copy with P1 behind must stay at +0"
    );
}

/// B: mirror boards — BOTH seats own a copy, asymmetric totals. Each copy is
/// evaluated against its own opponent in the same phase (8.4.4/8.4.5):
/// P1's copy sees opp(P2)=3 > self(P1)=2 → +1; P2's copy sees opp(P1)=2 not> 3 → +0.
/// One-sided arming could never distinguish "me vs my opponent" from a
/// hardcoded "seat1 vs seat2" comparison.
#[test]
fn mirror_copies_each_evaluate_against_own_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p1_live = game.id(LIVE);
    let p2_live = game.new_id(LIVE);
    game.state.player1.live_card_zone.cards.push(p1_live);
    game.state.player2.live_card_zone.cards.push(p2_live);

    give_total_energy(&mut game, 0, 2, 0); // P1 total 2
    give_total_energy(&mut game, 1, 3, 0); // P2 total 3

    // Same phase, both successes resolve (rules 8.4.4/8.4.5) — order P1 then P2.
    fire_live_success_as(&mut game, p1_live, "p1");
    fire_live_success_as(&mut game, p2_live, "p2");

    assert_eq!(
        score_mod(&game, p1_live),
        1,
        "P1's copy: opp(P2)=3 > self(P1)=2 → +1"
    );
    assert_eq!(
        score_mod(&game, p2_live),
        0,
        "P2's copy: opp(P1)=2 is NOT > self(P2)=3 → +0"
    );
}

/// C: rule 4.7.4 — 'エネルギー' means cards IN THE ENERGY ZONE regardless of
/// orientation (4.7.3 makes orientation mere placement state). Waited energy
/// therefore counts for the comparison. Guards against someone switching this
/// path to `active_count()` like the separate energy_relative branch uses.
#[test]
fn waited_energy_still_counts_for_comparison() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p1_live = game.id(LIVE);
    game.state.player1.live_card_zone.cards.push(p1_live);

    // Active counts would say 2 vs 1 (no bonus); zone totals say 2 vs 3 (+1).
    give_total_energy(&mut game, 0, 2, 0);
    give_total_energy(&mut game, 1, 1, 2);

    fire_live_success_as(&mut game, p1_live, "p1");

    assert_eq!(
        score_mod(&game, p1_live),
        1,
        "rule 4.7.4: P2's waited energy sits in her energy zone, so opp total=3 > self=2 → +1"
    );
}
