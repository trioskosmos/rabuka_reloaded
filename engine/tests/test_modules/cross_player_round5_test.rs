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

// ====================================================================
// ② PL!SP-pb2-029-N 米女メイ (登場/ライブ開始時):
//    「相手のステージにいるコスト2以下のメンバー1人をウェイトにする。」
//
// Rules grounding: 5.2.1 (ウェイトにする sets orientation unconditionally),
// 9.6.3.1 (exact count MUST be chosen when possible — cannot decline),
// 9.6.3.1.3 (zero eligible targets → selection ignored entirely).
//
// Prior coverage gap: batch11 arms P1→P2 only. The mirror flow — BOTH
// players owning メイ and each debut resolving against its own opponent —
// was never exercised, nor was the cost gate protecting the opposing メイ
// herself (cost 9 > 2) in a real both-seats flow.
// ====================================================================

use rabuka_engine::core::game_modifiers::CardOrientation;

const MEI: &str = "PL!SP-pb2-029-N"; // cost 9
const CHEAP: &str = "PL!SP-PR-007-PR"; // cost 2
const RINA: &str = "PL!N-bp7-009-R"; // cost 4, 登場 mutual mill 7

fn orientation(game: &TestGame, cid: i16) -> Option<CardOrientation> {
    game.state.mods.orientation_modifiers.get(&cid).copied()
}

fn set_active_seat(game: &mut TestGame, p1_active: bool) {
    game.state.player1.is_first_attacker = p1_active;
    game.state.player2.is_first_attacker = !p1_active;
}

fn give_p2_energy(game: &mut TestGame, n: usize) {
    let mut cards = Vec::with_capacity(n);
    for _ in 0..n {
        cards.push(game.new_id("LL-E-001-SD"));
    }
    let p2 = &mut game.state.player2;
    for e in cards {
        p2.energy_zone.cards.push(e);
    }
    p2.energy_zone.add_active(n as u8);
}

/// Mirror standoff: pre-stage a cheap member per side, then BOTH players
/// debut their own メイ in sequence. Each debut must look across the table
/// at its OWN opponent: P1's copy rests P2's cheap member, P2's copy rests
/// P1's cheap member. The opposing メイ herself (cost 9) must never be an
/// eligible target, so both copies end the flow still ACTIVE.
#[test]
fn mei_mirror_standoff_each_side_rests_own_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let cheap_p1 = game.id(CHEAP);
    let cheap_p2 = game.new_id(CHEAP);
    let mei_p1 = game.id(MEI);
    let mei_p2 = game.new_id(MEI);
    let filler = game.id("PL!-sd1-010-SD");

    // Pre-existing stage members placed directly (they are NOT debuting here;
    // direct placement skips their own triggers, which these members don't
    // carry in this scenario).
    game.state.player1.stage.stage = [cheap_p1, -1, -1];
    game.state.player2.stage.stage = [-1, cheap_p2, -1];

    fill_decks(&mut game, filler);

    // P1's turn: debut her メイ into center. P2's board has exactly one
    // eligible target (cheap_p2, cost 2) → single candidate auto-resolves.
    game.give_energy(20);
    game.state.player1.hand.cards.push(mei_p1);
    game.play_to_stage(mei_p1, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();

    assert_eq!(
        orientation(&game, cheap_p2),
        Some(CardOrientation::Wait),
        "P1's メイ must rest P2's cost-2 member"
    );
    assert_ne!(
        orientation(&game, cheap_p1),
        Some(CardOrientation::Wait),
        "P1's own side is untouched by her own debut"
    );

    // P2's turn: flip the active seat and debut HER メイ. P1's board offers
    // exactly one eligible target again (cheap_p1; メイ herself is cost 9).
    set_active_seat(&mut game, false);
    give_p2_energy(&mut game, 20);
    game.state.player2.hand.cards.push(mei_p2);
    game.play_to_stage(mei_p2, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();

    assert_eq!(
        orientation(&game, cheap_p1),
        Some(CardOrientation::Wait),
        "P2's メイ must rest P1's cost-2 member"
    );
    assert_eq!(
        orientation(&game, cheap_p2),
        Some(CardOrientation::Wait),
        "P1's earlier rest on P2's member persists"
    );
    assert_ne!(
        orientation(&game, mei_p1),
        Some(CardOrientation::Wait),
        "cost gate: P2's メイ can never rest P1's メイ (cost 9 > 2)"
    );
    assert_ne!(
        orientation(&game, mei_p2),
        Some(CardOrientation::Wait),
        "P1's メイ never rested P2's メイ either"
    );
}

/// Cost-gate negative + already-waited no-op:
/// - A cost-4 member alone on the opponent's stage is NOT eligible
///   (9.6.3.1.3: zero eligible targets → selection ignored, no prompt stuck).
/// - An ALREADY-waited cost-2 member stays a legal target per 5.2.1 (no
///   active-state precondition): re-resting it is a no-op that leaves it Wait.
#[test]
fn mei_cost_gate_and_already_waited_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mei = game.id(MEI);
    let expensive = game.id("PL!-sd1-010-SD"); // μ's filler, cost 4
    let cheap = game.new_id(CHEAP); // cost 2

    // Opponent board: one over-cost member and one already-waited cheap member.
    game.state.player2.stage.stage = [expensive, cheap, -1];
    game.state
        .mods
        .add_orientation_modifier(cheap, "wait");

    fill_decks(&mut game, expensive);
    game.give_energy(20);
    game.state.player1.hand.cards.push(mei);
    game.play_to_stage(mei, rabuka_engine::zones::MemberArea::Center);

    // The only eligible candidate is the already-waited cheap member; single
    // candidate auto-resolves — no prompt may remain dangling either way.
    game.drain_auto_ability_choices();
    assert!(
        !game.has_pending_choice(),
        "debut resolution must not leave a stuck prompt"
    );

    assert_eq!(
        orientation(&game, cheap),
        Some(CardOrientation::Wait),
        "already-waited eligible member is re-selected (9.6.3.1) and stays Wait"
    );
    assert_ne!(
        orientation(&game, expensive),
        Some(CardOrientation::Wait),
        "cost gate: cost-4 member is not eligible"
    );
}

// ====================================================================
// ③ PL!N-bp7-009-R 天王寺璃奈 (登場):
//    「自分と相手はそれぞれ、自身のデッキの上からカードを7枚控え室に置く。」
//
// The refresh-boundary semantics (Q267) are thoroughly pinned in
// bp7_q267_rinna_mill_refresh_test.rs — NOT duplicated here. What that file
// never covers:
//   1) a P2-OWNED 璃奈 fired from P2's seat ("自分と相手" must follow the
//      ability master's seat, not hardcode p1),
//   2) card IDENTITY across both mills in one flow — no cross-pollination
//      between the two waitrooms, exact top-7 membership per side.
// ====================================================================

/// Fire RINA's 登場 ability as an arbitrary seat ("p1"/"p2").
fn fire_rina_debut_as(game: &mut TestGame, rina: i16, seat: &str) {
    use rabuka_engine::core::types::AbilityTrigger;
    let card = game.db.get_card(rina).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("RINA lacks her 登場 ability");
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::Debut,
        seat.to_string(),
        Some(card.card_no.to_string()),
        Some(rina),
        None,
        None,
    );
    game.state.activating_card = Some(rina);
    game.state.process_pending_auto_abilities(&seat.to_string());
    game.drain_auto_ability_choices();
}

#[test]
fn p2_owned_rina_mills_both_decks_from_p2_seat() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");
    let rina_p2 = game.id(RINA);

    // RINA sits on P2's board (direct placement: only HER debut is under test).
    game.state.player2.stage.stage[1] = rina_p2;

    fill_decks(&mut game, filler); // 20 cards per deck

    fire_rina_debut_as(&mut game, rina_p2, "p2");

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        23,
        "P1's deck must lose exactly its top 7 even when RINA belongs to P2"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        7,
        "P1's mill lands in P1's OWN waitroom"
    );
    assert_eq!(
        game.state.player2.main_deck.cards.len(),
        23,
        "'自分と相手' follows the firing seat: P2 mills her own deck too"
    );
    assert_eq!(game.state.player2.waitroom.cards.len(), 7, "P2's waitroom");
}

/// One flow, both mills, exact identity: each player's waitroom receives
/// EXACTLY its own former top-7 cards (as a set) — the opponent's mill never
/// leaks across the table, and the deck keeps every non-milled card.
#[test]
fn mutual_mill_identity_no_cross_pollination() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler_template = game.id("PL!-sd1-010-SD");

    let rina_p1 = game.id(RINA);
    game.state.player1.stage.stage[1] = rina_p1;
    fill_decks(&mut game, filler_template);

    // Distinct copies so every card id is unique and traceable. insert(0, …)
    // puts each new copy at the deck TOP (index 0), so the mill takes exactly
    // these seven.
    let mut p1_top7 = Vec::new();
    for _ in 0..7 {
        let c = game.new_id("PL!-sd1-010-SD");
        p1_top7.push(c);
        game.state.player1.main_deck.cards.insert(0, c);
    }
    let mut p2_top7 = Vec::new();
    for _ in 0..7 {
        let c = game.new_id("PL!-sd1-010-SD");
        p2_top7.push(c);
        game.state.player2.main_deck.cards.insert(0, c);
    }

    fire_rina_debut_as(&mut game, rina_p1, "p1");

    for &c in &p1_top7 {
        assert!(
            game.state.player1.waitroom.cards.contains(&c)
                && !game.state.player2.waitroom.cards.contains(&c),
            "P1 milled card {} must sit in P1's waitroom only",
            c
        );
    }
    for &c in &p2_top7 {
        assert!(
            game.state.player2.waitroom.cards.contains(&c)
                && !game.state.player1.waitroom.cards.contains(&c),
            "P2 milled card {} must sit in P2's waitroom only",
            c
        );
    }
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        7,
        "waitroom holds exactly the 7 milled cards"
    );
    assert_eq!(game.state.player2.waitroom.cards.len(), 7, "mirror");
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        30,
        "30 fillers + 7 distinct tops = 37, minus 7 milled"
    );
}

// ====================================================================
// ④ PL!HS-PR-035-PR 百生吟子 (登場):
//    「相手の控え室にあるメンバーカードを3枚選び、相手のデッキの下に好きな順番で
//     置いてもよい。そうした場合、相手のステージにいる元々持つブレードの数が
//     3つ以下のメンバー1人をウェイトにする。」
//
// bp7_ginko_select_discard_deck_bottom_test.rs pins ONLY the accept-path
// placement (set membership + count). Never covered until now:
//   - the 「そうした場合」 consequence chain (conditional_on_optional gated on
//     last_move_moved_any): decline ⇒ no wait,
//   - the ORIGINAL-blade<=3 gate on the follow-up rest,
//   - a P2-owned 吟子 rearranging P1's board from P2's seat.
// ====================================================================

const GINKO: &str = "PL!HS-PR-035-PR";
const HIGH_BLADE: &str = "PL!SP-sd2-001-SD2"; // original blade 7

fn fire_ginko_debut_as(game: &mut TestGame, gin: i16, seat: &str) {
    use rabuka_engine::core::types::AbilityTrigger;
    let card = game.db.get_card(gin).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("吟子 lacks her 登場 ability");
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::Debut,
        seat.to_string(),
        Some(card.card_no.to_string()),
        Some(gin),
        None,
        None,
    );
    game.state.activating_card = Some(gin);
    game.state.process_pending_auto_abilities(&seat.to_string());
}

/// Drive 吟子's prompt chain. The 「置いてもよい」 gate IS the SelectCard's
/// allow_skip (no separate optional prompt): accept answers with all 3
/// discard slots, decline answers empty — which must skip BOTH the placement
/// and the conditional rest.
fn drain_ginko(game: &mut TestGame, accept: bool) {
    use rabuka_engine::ability::types::Choice;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectCard { count, .. } => {
                if accept {
                    let n = (*count).max(1) as usize;
                    let idxs: Vec<usize> = (0..n).collect();
                    game.select_indices(&idxs);
                } else {
                    game.select_indices(&[]);
                }
            }
            _ => break,
        }
    }
}

#[test]
fn ginko_accept_places_then_rests_blade_gate_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD"); // original blade 1 -> eligible
    let tank = game.id(HIGH_BLADE); // original blade 7 -> gate blocks

    // Opponent (P2) discard: exactly the 3 selectable members.
    let a = game.id("PL!-sd1-001-SD");
    let b = game.id("PL!-sd1-003-SD");
    let c = game.id("PL!-sd1-004-SD");
    game.state.player2.waitroom.cards.push(a);
    game.state.player2.waitroom.cards.push(b);
    game.state.player2.waitroom.cards.push(c);

    // Opponent stage: one gate-passing member, one gate-blocking member.
    game.state.player2.stage.stage = [filler, tank, -1];
    fill_decks(&mut game, filler);
    let p2_deck_before = game.state.player2.main_deck.cards.len();

    let gin = game.id(GINKO);
    game.state.player1.stage.stage[1] = gin;
    fire_ginko_debut_as(&mut game, gin, "p1");
    drain_ginko(&mut game, true);

    for id in [a, b, c] {
        assert!(
            !game.state.player2.waitroom.cards.contains(&id),
            "chosen member {id} left the opponent's discard"
        );
        assert!(
            game.state.player2.main_deck.cards.contains(&id),
            "chosen member {id} went under the opponent's deck"
        );
    }
    assert_eq!(game.state.player2.main_deck.cards.len(), p2_deck_before + 3);

    assert_eq!(
        orientation(&game, filler),
        Some(CardOrientation::Wait),
        "「そうした場合」: accepting the placement rests the opponent's blade<=3 member"
    );
    assert_ne!(
        orientation(&game, tank),
        Some(CardOrientation::Wait),
        "original blade 7 > 3: the gate must protect this member"
    );
}

#[test]
fn ginko_decline_moves_and_rests_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");

    let a = game.id("PL!-sd1-001-SD");
    let b = game.id("PL!-sd1-003-SD");
    let c = game.id("PL!-sd1-004-SD");
    game.state.player2.waitroom.cards.push(a);
    game.state.player2.waitroom.cards.push(b);
    game.state.player2.waitroom.cards.push(c);
    game.state.player2.stage.stage = [filler, -1, -1];
    fill_decks(&mut game, filler);
    let p2_deck_before = game.state.player2.main_deck.cards.len();

    let gin = game.id(GINKO);
    game.state.player1.stage.stage[1] = gin;
    fire_ginko_debut_as(&mut game, gin, "p1");
    drain_ginko(&mut game, false);

    assert!(
        !game.has_pending_choice(),
        "declining must not leave the conditional wait dangling"
    );
    for id in [a, b, c] {
        assert!(
            game.state.player2.waitroom.cards.contains(&id),
            "declined: discard untouched ({id})"
        );
    }
    assert_eq!(
        game.state.player2.main_deck.cards.len(),
        p2_deck_before,
        "declined: opponent deck unchanged"
    );
    assert_ne!(
        orientation(&game, filler),
        Some(CardOrientation::Wait),
        "「そうした場合」 gates the REST on the placement too: declined ⇒ no rest"
    );
}

/// Mirror seat: P2-owned 吟子 rearranges P1's discard into P1's deck bottom
/// and rests P1's blade<=3 member — fired from P2's seat.
#[test]
fn p2_owned_ginko_targets_p1_board_from_p2_seat() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");

    let a = game.id("PL!-sd1-001-SD");
    let b = game.id("PL!-sd1-003-SD");
    let c = game.id("PL!-sd1-004-SD");
    game.state.player1.waitroom.cards.push(a);
    game.state.player1.waitroom.cards.push(b);
    game.state.player1.waitroom.cards.push(c);
    game.state.player1.stage.stage = [filler, -1, -1];
    fill_decks(&mut game, filler);
    let p1_deck_before = game.state.player1.main_deck.cards.len();

    let gin = game.id(GINKO);
    game.state.player2.stage.stage[1] = gin;
    fire_ginko_debut_as(&mut game, gin, "p2");
    drain_ginko(&mut game, true);

    for id in [a, b, c] {
        assert!(
            !game.state.player1.waitroom.cards.contains(&id)
                && game.state.player1.main_deck.cards.contains(&id),
            "member {id} moved from P1's discard to P1's deck bottom"
        );
    }
    assert_eq!(game.state.player1.main_deck.cards.len(), p1_deck_before + 3);
    assert_eq!(
        orientation(&game, filler),
        Some(CardOrientation::Wait),
        "P2-owned copy rests P1's eligible member"
    );
}

// ====================================================================
// ⑤ PL!SP-bp5-027-L HOT PASSION!! (ライブ成功時):
//    「自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で
//     置いてもよい。そうした場合、相手はカードを1枚引く。」
//
// batch32 pins the basic accept/decline flow. Missing until now:
//   - EMPTY energy deck: nothing placeable ⇒ no prompt at all and NO
//     opponent draw (Q102 / 9.6.3.1.3: unresolvable parts do nothing),
//   - the cross-ability combo: the WAITED energy it places still counts as
//     エネルギー (rules 4.7.4), flipping 近未来ハッピーエンド's strict
//     energy comparison on the OPPONENT's side — two abilities interacting
//     across seats in one flow.
// ====================================================================

#[test]
fn hot_passion_empty_energy_deck_no_prompt_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");

    let hot = game.id("PL!SP-bp5-027-L");
    game.state.player1.live_card_zone.cards.push(hot);
    fill_decks(&mut game, filler);
    // NOTE: no fill_energy_deck — the deck is empty.

    let p2_hand_before = game.state.player2.hand.cards.len();
    fire_live_success_as(&mut game, hot, "p1");
    game.drain_auto_ability_choices();

    assert!(
        !game.has_pending_choice(),
        "empty energy deck -> nothing placeable -> no pay/skip prompt"
    );
    assert_eq!(game.state.player1.energy_zone.cards.len(), 0);
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before,
        "no placement -> opponent draws nothing"
    );
}

/// Two abilities, both seats, one flow: P1's HOT PASSION places a WAITED
/// energy (accept), raising P1's energy-zone TOTAL from 2 to 3 while active
/// stays 2 — which flips P2-owned 近未来ハッピーエンド's strict comparison
/// (opponent 3 > self 2) to +1. Pins rule 4.7.4 through a REAL effect chain
/// instead of manual zone injection, plus the seat-relative evaluation of
/// both abilities across the table.
#[test]
fn hot_passion_waited_energy_flips_opponent_score_comparison() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");

    let hot_p1 = game.id("PL!SP-bp5-027-L");
    let happy_p2 = game.id(LIVE); // PL!S-bp6-022-L 近未来ハッピーエンド
    game.state.player1.live_card_zone.cards.push(hot_p1);
    game.state.player2.live_card_zone.cards.push(happy_p2);

    fill_decks(&mut game, filler);
    fill_energy_deck(&mut game, 0, 1);
    give_total_energy(&mut game, 0, 2, 0); // P1: 2 total
    give_total_energy(&mut game, 1, 2, 0); // P2: 2 total

    // Baseline: strict > fails both ways at 2 vs 2.
    fire_live_success_as(&mut game, happy_p2, "p2");
    assert_eq!(
        score_mod(&game, happy_p2),
        0,
        "baseline 2 vs 2: P2's copy scores nothing"
    );

    // P1's HOT PASSION resolves AFTER P2's success in this flow ordering —
    // same turn, later check timing (both are ライブ成功時 abilities of the
    // same live round; each fires once).
    fire_live_success_as(&mut game, hot_p1, "p1");
    assert!(
        game.has_pending_choice(),
        "energy deck has a card -> placement prompted"
    );
    game.select_option(1); // accept

    assert_eq!(game.state.player1.energy_zone.cards.len(), 3);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "the placed energy arrives WAITED (active count unchanged)"
    );

    // Re-evaluate P2's copy against the NEW totals: opp(P1)=3 (incl. the
    // waited card) > self(P2)=2 -> +1. A second live-success resolution of
    // the same ability within one live cannot occur naturally; firing again
    // here probes the CONDITION under post-placement state, pinning that
    // waited energy counts (4.7.4) end-to-end.
    fire_live_success_as(&mut game, happy_p2, "p2");
    assert_eq!(
        score_mod(&game, happy_p2),
        1,
        "HOT PASSION's waited energy counts as エネルギー (4.7.4): opp 3 > self 2 -> +1"
    );
}
