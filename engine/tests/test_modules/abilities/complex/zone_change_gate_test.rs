//! Zone-change trigger-gate coverage (docs/TEST_HARDENING_PLAN_2026-08-26.md §2).
//!
//! Abilities whose printed text fires only on a SPECIFIC zone transition
//! 「…から…に置かれたとき」. The gate inventory found three families with no
//! coverage at all; this file pins positive controls plus wrong-source,
//! wrong-group and wrong-player negatives for each.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::card::HeartColor;

/// 桜内梨子 ab#0: 『Aqours』のライブカードが自分のライブカード置き場から
/// 控え室に置かれたとき、そのライブカードをデッキの一番上か一番下に置いてもよい。
const RIKO: &str = "PL!S-bp6-002-R\u{ff0b}";
const AQOURS_LIVE: &str = "PL!S-bp2-019-L"; // WATER BLUE NEW WORLD
const MUSE_LIVE: &str = "PL!-sd1-019-SD"; // μ's live — wrong-group negative

/// 天王寺璃奈 ab#0 (ライブ開始時): このターン、ブレードハートを持たない
/// メンバーが自分のライブカード置き場から控え室に置かれている場合 →
/// draw 1 + heart03/05/06 until live end.
const REINA: &str = "PL!N-pb1-009-R";
/// 穂乃果: blade_heart=None (no b_heart icons printed).
const NO_BLADE_MEMBER: &str = "PL!-sd1-001-SD";
/// 花陽: blade_heart={'b_heart03': 1}.
const BLADE_MEMBER: &str = "PL!-sd1-008-SD";

/// 宮下愛 ab#0 (自動): このメンバーがステージから控え室に置かれたとき、
/// このメンバーがコスト10以上のブレードハートを持たない『虹ヶ咲』のメンバーと
/// バトンタッチしてしていた場合、エネルギーを2枚アクティブにする。
/// コスト15以上…の場合、さらにカードを1枚引く。
///
/// Rules 9.6.2.3.2/.1: baton touch = the OCCUPANT goes to the waitroom during
/// the arrival's cost payment, generating the 「バトンタッチした」 event for
/// the ARRIVING member's play; this ability's conditions therefore describe
/// the NEWCOMER.
const MIYAMIYA: &str = "PL!N-bp5-005-R\u{ff0b}";
/// 桜坂しずく cost=15, blade_heart=None → full payoff (+2 energy AND draw 1).
const NEWCOMER_COST15: &str = "PL!N-bp7-003-R\u{ff0b}";
/// 優木せつ菜 cost=13, blade_heart=None → energy-only branch.
const NEWCOMER_COST13: &str = "PL!N-bp4-007-R\u{ff0b}";
/// 天王寺璃奈 cost=13, blade_heart=YES → negated property blocks everything.
const NEWCOMER_BLADE_HEART: &str = "PL!N-bp4-009-R";

/// Move `card` live_card_zone→waitroom as a real effect-caused zone change
/// and arm the movement batch exactly like the engine does.
fn move_live_to_waitroom(g: &mut TestGame, card: i16, owner_side: u8, causer: &str) {
    let (zone, pid) = if owner_side == 1 {
        (&g.state.player1.live_card_zone, g.state.player1.id.clone())
    } else {
        (&g.state.player2.live_card_zone, g.state.player2.id.clone())
    };
    assert!(
        zone.cards.contains(&card),
        "test setup bug: card must start in the live card zone"
    );
    if owner_side == 1 {
        g.state
            .player1
            .live_card_zone
            .cards
            .retain(|c| *c != card);
        g.state.player1.waitroom.cards.push(card);
    } else {
        g.state
            .player2
            .live_card_zone
            .cards
            .retain(|c| *c != card);
        g.state.player2.waitroom.cards.push(card);
    }
    g.state
        .push_movement_event(card, "live_card_zone", "discard", None, causer, true);
    let _ = pid;
}

/// Riko responds when an AQOURS live card leaves HER live card zone:
/// optional deck-top/bottom placement is offered and resolves.
#[test]
fn riko_aqours_live_leaving_live_zone_offers_deck_placement() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let riko = g.id(RIKO);
    let live = g.id(AQOURS_LIVE);
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage = [-1, riko, -1];
    g.state.player1.live_card_zone.cards.push(live);
    fill_decks(&mut g, filler);
    let deck_before = g.state.player1.main_deck.cards.len();

    move_live_to_waitroom(&mut g, live, 1, "p1");

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    // Optional deck-top/bottom offer must appear.
    assert!(
        g.has_pending_choice(),
        "Riko should be offered the deck-top/bottom placement:\n{}",
        g.pending_choice_summary()
    );
    crate::helpers::answer_choice(&mut g, 0);
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    // The live card went to the deck (top or bottom) — either way out of the waitroom.
    assert!(
        g.state.player1.main_deck.cards.contains(&live),
        "Aqours live should be placed into the deck"
    );
    assert_eq!(
        g.state.player1.main_deck.cards.len(),
        deck_before + 1,
        "deck grew by exactly the placed live card"
    );
    assert!(
        !g.state.player1.waitroom.cards.contains(&live),
        "live card must not stay in the waitroom after the placement"
    );
}

/// Wrong group: a μ's live leaving the same zone must NOT arm Riko.
#[test]
fn riko_ignores_non_aqours_live_leaving_live_zone() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let riko = g.id(RIKO);
    let muse_live = g.id(MUSE_LIVE);
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage = [-1, riko, -1];
    g.state.player1.live_card_zone.cards.push(muse_live);
    fill_decks(&mut g, filler);

    move_live_to_waitroom(&mut g, muse_live, 1, "p1");

    let pid = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid);
    g.state.process_pending_auto_abilities(&pid);

    assert!(
        !g.has_pending_choice(),
        "μ's live leaving the zone must not offer Riko's placement:\n{}",
        g.pending_choice_summary()
    );
    assert!(
        g.state.player1.waitroom.cards.contains(&muse_live),
        "the μ's live stays in the waitroom"
    );
}

/// Ownership: P2's Aqours live leaving P2's zone arms only P2's Riko
/// (target=self), never P1's copy.
#[test]
fn riko_responds_only_to_own_side_live_zone() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let riko_p1 = g.id(RIKO);
    let riko_p2 = g.new_id(RIKO);
    let live_p2 = g.new_id(AQOURS_LIVE);
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage = [-1, riko_p1, -1];
    g.state.player2.stage.stage = [-1, riko_p2, -1];
    g.state.player2.live_card_zone.cards.push(live_p2);
    fill_decks(&mut g, filler);

    move_live_to_waitroom(&mut g, live_p2, 2, "p2");

    // Scan ONLY P1: her Riko must stay silent about P2's zone.
    let pid1 = g.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid1);
    g.state.process_pending_auto_abilities(&pid1);
    assert!(
        !g.has_pending_choice(),
        "P1's Riko must not react to P2's live-zone change:\n{}",
        g.pending_choice_summary()
    );

    // Scanning P2 arms HIS copy.
    let pid2 = g.state.player2.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut g.state, &pid2);
    g.state.process_pending_auto_abilities(&pid2);
    assert!(
        g.has_pending_choice(),
        "P2's own Riko should react to his live leaving:\n{}",
        g.pending_choice_summary()
    );
    crate::helpers::answer_choice(&mut g, 0);
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    assert!(g.state.player2.main_deck.cards.contains(&live_p2));
}

/// Reina: a NON-blade-heart member moved live zone→waitroom this turn →
/// LiveStart draws 1 and grants heart03/05/06 until live end.
#[test]
fn reina_fires_after_non_blade_member_left_live_zone() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let reina = g.id(REINA);
    let hanayo = g.id(NO_BLADE_MEMBER);
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage = [-1, reina, -1];
    g.state.player1.live_card_zone.cards.push(hanayo);
    fill_decks(&mut g, filler);

    move_live_to_waitroom(&mut g, hanayo, 1, "p1");

    let hand_before = g.state.player1.hand.cards.len();
    fire_trigger(
        &mut g,
        reina,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert_eq!(
        g.state.player1.hand.cards.len(),
        hand_before + 1,
        "draw 1 from the LiveStart ability"
    );
    // One of EACH listed color (count=3 spread over heart03/05/06).
    assert_eq!(
        g.state
            .mods
            .get_heart_modifier(reina, HeartColor::Heart03),
        1,
        "heart03 granted until live end"
    );
    assert_eq!(
        g.state
            .mods
            .get_heart_modifier(reina, HeartColor::Heart05),
        1,
        "heart05 granted until live end"
    );
    assert_eq!(
        g.state
            .mods
            .get_heart_modifier(reina, HeartColor::Heart06),
        1,
        "heart06 granted until live end"
    );
}

/// Negative twin: the member that left had a blade heart → negated
/// has_blade_heart gate fails → no draw, no hearts.
#[test]
fn reina_silent_when_blade_heart_member_left_live_zone() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let reina = g.id(REINA);
    let honoka = g.id(BLADE_MEMBER); // blade=3
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage = [-1, reina, -1];
    g.state.player1.live_card_zone.cards.push(honoka);
    fill_decks(&mut g, filler);

    move_live_to_waitroom(&mut g, honoka, 1, "p1");

    let hand_before = g.state.player1.hand.cards.len();
    fire_trigger(
        &mut g,
        reina,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert_eq!(
        g.state.player1.hand.cards.len(),
        hand_before,
        "blade-heart departure must NOT draw"
    );
    assert_eq!(
        g.state
            .mods
            .get_heart_modifier(reina, HeartColor::Heart03),
        0,
        "blade-heart departure must NOT grant hearts"
    );
}

/// Real baton touch: a cost-15 non-blade-heart Niji member replaces 宮下愛 →
/// she leaves stage→waitroom, the baton_touch condition passes fully →
/// activate 2 energy AND draw 1.
#[test]
fn miyamiya_baton_touch_cost15_newcomer_full_payoff() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let ai = g.id(MIYAMIYA);
    let newcomer = g.id(NEWCOMER_COST15);
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage[1] = ai;
    fill_decks(&mut g, filler);
    g.state.player1.hand.cards.push(newcomer);
    g.give_energy(25);
    // Leave 5 in wait state so the ability's "activate 2" has cards to flip.
    let wait_pool = 5u8;
    g.state.player1.energy_zone.set_active_count(25 - wait_pool);

    let active_before = g.state.player1.energy_zone.active_count();
    let hand_before = g.state.player1.hand.cards.len();

    // Playing onto the occupied center slot performs the baton touch
    // (rules 9.6.2.3.2): Ai -> waitroom, newcomer arrives; net payment =
    // newcomer cost - Ai's printed cost.
    g.play_to_stage(newcomer, rabuka_engine::zones::MemberArea::Center);
    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    let ai_cost = i32::from(
        g.state.card_database.get_card(ai).unwrap().cost.unwrap(),
    );
    let nc_cost = i32::from(
        g.state
            .card_database
            .get_card(newcomer)
            .unwrap()
            .cost
            .unwrap(),
    );
    assert_eq!(
        i32::from(g.state.player1.energy_zone.active_count()),
        i32::from(active_before) - (nc_cost - ai_cost) + 2,
        "paid {paid} net after baton discount, then activated 2 wait energy",
        paid = nc_cost - ai_cost
    );
    assert_eq!(
        g.state.player1.hand.cards.len(),
        hand_before - 1 + 1,
        "newcomer left hand (-1) and the ability drew 1 (+1)"
    );
}

/// Boundary: cost-13 (>=10 but <15) non-blade-heart Niji newcomer →
/// energy only, no draw.
#[test]
fn miyamiya_baton_touch_cost13_newcomer_energy_only() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let ai = g.id(MIYAMIYA);
    let newcomer = g.id(NEWCOMER_COST13);
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage[1] = ai;
    fill_decks(&mut g, filler);
    g.state.player1.hand.cards.push(newcomer);
    g.give_energy(25);
    let wait_pool = 5u8;
    g.state.player1.energy_zone.set_active_count(25 - wait_pool);

    let active_before = g.state.player1.energy_zone.active_count();
    let hand_before = g.state.player1.hand.cards.len();

    g.play_to_stage(newcomer, rabuka_engine::zones::MemberArea::Center);
    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    let ai_cost = i32::from(
        g.state.card_database.get_card(ai).unwrap().cost.unwrap(),
    );
    let nc_cost = i32::from(
        g.state
            .card_database
            .get_card(newcomer)
            .unwrap()
            .cost
            .unwrap(),
    );
    assert_eq!(
        i32::from(g.state.player1.energy_zone.active_count()),
        i32::from(active_before) - (nc_cost - ai_cost) + 2,
        "cost-13 newcomer still clears the >=10 gate: activate 2 energy"
    );
    assert_eq!(
        g.state.player1.hand.cards.len(),
        hand_before - 1,
        "below cost 15: NO draw (newcomer just left hand)"
    );
}

/// Negative: the replacing member HAS a blade heart → negated property fails
/// → no energy activation at all.
#[test]
fn miyamiya_baton_touch_blade_heart_newcomer_nothing() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let ai = g.id(MIYAMIYA);
    let newcomer = g.id(NEWCOMER_BLADE_HEART);
    let filler = g.id("PL!-sd1-010-SD");

    g.state.player1.stage.stage[1] = ai;
    fill_decks(&mut g, filler);
    g.state.player1.hand.cards.push(newcomer);
    g.give_energy(25);
    let wait_pool = 5u8;
    g.state.player1.energy_zone.set_active_count(25 - wait_pool);

    let active_before = g.state.player1.energy_zone.active_count();
    let hand_before = g.state.player1.hand.cards.len();

    g.play_to_stage(newcomer, rabuka_engine::zones::MemberArea::Center);
    while g.has_pending_choice() {
        g.select_indices(&[]);
    }

    let ai_cost = i32::from(
        g.state.card_database.get_card(ai).unwrap().cost.unwrap(),
    );
    let nc_cost = i32::from(
        g.state
            .card_database
            .get_card(newcomer)
            .unwrap()
            .cost
            .unwrap(),
    );
    assert_eq!(
        i32::from(g.state.player1.energy_zone.active_count()),
        i32::from(active_before) - (nc_cost - ai_cost),
        "blade-heart newcomer must NOT activate energy — only the net cost was paid"
    );
    assert_eq!(
        g.state.player1.hand.cards.len(),
        hand_before - 1,
        "no draw either — only the normal play happened"
    );
}
