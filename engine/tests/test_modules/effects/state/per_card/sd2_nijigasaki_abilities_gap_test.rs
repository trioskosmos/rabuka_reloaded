/// Gameplay coverage for the PL!N-sd2 cards that previously had parser-verdict
/// rows but NO gameplay test (see `_bp07_ability_gaps_hand_analysis.md`).
///
/// Covered:
///   - PL!N-sd2-006-SD2 近江彼方  (ライブ開始時): wait 1 own 虹 member (optional) -> blade+2 till live end
///   - PL!N-sd2-010-SD2 三船栞子  (登場): draw 2 ; (自動 1回): 虹 member waited -> discard 手札, activate it, blade+2
///   - PL!N-sd2-013-SD2 上原歩夢  (登場/ライブ開始時): if stage is 虹 only, wait opp ORIGINAL blade <= 2
///   - PL!N-sd2-015-SD2 桜坂しずく(起動 ターン1): wait self + discard 手札 1 -> draw 1
///   - PL!N-sd2-017-SD2 宮下 愛    (ライブ開始時): pay 1E optional -> active 1 stage member
///   - PL!N-sd2-019-SD2 優木せつ菜 (登場): heart05 till live end
///   - PL!N-sd2-021-SD2 天王寺璃奈 (登場): wait 1 opp member cost <= 4
use crate::helpers::*;
use crate::test_modules::support::bp7_wait_immunity_helpers::is_waited;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

// SD2 players under test (all 虹ヶ咲).
const KANATA: &str = "PL!N-sd2-006-SD2"; // 近江彼方
const SHIORIKO: &str = "PL!N-sd2-010-SD2"; // 三船栞子
const AYUMU: &str = "PL!N-sd2-013-SD2"; // 上原歩夢
const SHIZUKU: &str = "PL!N-sd2-015-SD2"; // 桜坂しずく
const AI: &str = "PL!N-sd2-017-SD2"; // 宮下 愛
const SETSUNA: &str = "PL!N-sd2-019-SD2"; // 優木せつ菜
const RINNE: &str = "PL!N-sd2-021-SD2"; // 天王寺璃奈

// Another 虹ヶ咲 member used as an ally (wait target / activation target).
const ALLY: &str = "PL!N-sd2-013-SD2"; // same-family 虹 card is fine

// Opponent (μ's) members with known COST / ORIGINAL blade.
const OPP_COST4_B1: &str = "PL!-sd1-010-SD"; // cost 4, ORIGINAL blade 1
const OPP_B3: &str = "PL!-sd1-001-SD"; // ORIGINAL blade 3
const OPP_COST5: &str = "PL!-pb1-021-PR"; // cost 5 (over the cost-4 limit)
const OUTSIDER: &str = "PL!-sd1-010-SD"; // μ's ally -> stage not 虹-only
const LIVE: &str = "PL!-sd1-019-SD"; // a live card
const FILLER: &str = "PL!-sd1-010-SD";

fn seed_decks(g: &mut TestGame) {
    let f = g.id(FILLER);
    g.state.player1.main_deck.cards.clear();
    g.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        g.state.player1.main_deck.cards.push(f);
        g.state.player2.main_deck.cards.push(f);
    }
}

/// Resolve every pending choice with a default, `pay`-aware behavior for the
/// pay-or-skip (SelectTarget "pay_optional_cost") prompt.
fn drain(g: &mut TestGame, pay: bool) {
    while g.has_pending_choice() {
        match g.pending_choice_type().as_deref() {
            Some("SelectTarget") => g.select_option(if pay { 1 } else { 0 }),
            Some("SelectCard") | Some("SelectPosition") => g.select_indices(&[0]),
            Some("SelectAutoAbility") => g.select_indices(&[]),
            Some("SelectHeartColor") | Some("SelectHeartType") => g.select_indices(&[0]),
            _ => g.select_indices(&[]),
        }
    }
}

/// Fire an ability matching `trigger_str` and resolve all its choices.
fn fire(g: &mut TestGame, card_id: i16, trigger_str: &str) {
    let card = g.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .unwrap();
    let pid = g.state.player1.id.clone();
    let trigger = if trigger_str == "登場" {
        AbilityTrigger::Debut
    } else if trigger_str == "ライブ開始時" {
        AbilityTrigger::LiveStart
    } else if trigger_str == "起動" {
        AbilityTrigger::Activation
    } else {
        AbilityTrigger::Auto
    };
    g.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    g.state.activating_card = Some(card_id);
    g.state.process_pending_auto_abilities(&pid);
    drain(g, true);
}

// ====================================================================
// 006 近江彼方 (ライブ開始時): wait 1 own 虹 member (optional) -> blade+2
// ====================================================================

fn drive_to_live_start(g: &mut TestGame) {
    let live = g.id(LIVE);
    // Push the cached live id; TestGame::id() allocates a fresh instance id on
    // every call, so we must use ONE cached value for both hand and set_live_card.
    g.state.player1.hand.cards.push(live);
    for _ in 0..5 {
        g.pass();
    }
    g.set_live_card(live);
    g.pass();
    g.pass();
}

#[test]
fn kanata_pays_wait_and_gains_two_blade() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let kanata = g.id(KANATA);
    let ally = g.id(ALLY);
    g.state.player1.stage.stage = [ally, kanata, -1];
    g.give_energy(8);
    seed_decks(&mut g);
    drive_to_live_start(&mut g);
    drain(&mut g, true); // pay: wait the own 虹 ally
    assert!(
        is_waited(&g, ally),
        "006: paying the optional wait leaves the 虹 ally waited"
    );
    assert_eq!(
        g.state.mods.get_blade_modifier(kanata),
        2,
        "006: waiting an own 虹 member grants 近江彼方 +2 blade till live end"
    );
}

#[test]
fn kanata_skip_optional_wait_gains_nothing() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let kanata = g.id(KANATA);
    let ally = g.id(ALLY);
    g.state.player1.stage.stage = [ally, kanata, -1];
    g.give_energy(8);
    seed_decks(&mut g);
    drive_to_live_start(&mut g);
    drain(&mut g, false); // skip the optional wait
    assert!(
        !is_waited(&g, ally),
        "006: skipping the optional wait leaves the ally NOT waited"
    );
    assert_eq!(
        g.state.mods.get_blade_modifier(kanata),
        0,
        "006: skipping the optional wait grants no blade"
    );
}

// ====================================================================
// 010 三船栞子: (登場) draw 2 ; (自動 1回) 虹 member waited ->
// discard 手札, activate it, blade+2
// ====================================================================

#[test]
fn shioriko_debut_draws_two() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let shi = g.id(SHIORIKO);
    seed_decks(&mut g);
    let hand_before = g.state.player1.hand.cards.len();
    g.add_to_hand(shi);
    g.give_energy(20);
    g.play_to_stage(shi, MemberArea::Center);
    drain(&mut g, true);
    assert_eq!(
        g.state.player1.hand.cards.len(),
        hand_before + 2,
        "010: 三船栞子 debut draws 2 cards"
    );
}

#[test]
fn shioriko_auto_activates_waited_ally() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let shi = g.id(SHIORIKO);
    let ally = g.id(ALLY);
    g.state.player1.stage.stage = [ally, shi, -1];
    g.state.mods.add_orientation_modifier(ally, "wait"); // a 虹 member is waited
    g.state.player1.hand.cards.push(g.id(FILLER)); // a card to discard for the cost

    fire(&mut g, shi, "自動");

    assert!(!is_waited(&g, ally), "010: auto leg activates the waited 虹 member");
    assert_eq!(
        g.state.mods.get_blade_modifier(ally),
        2,
        "010: the member gains +2 blade after being activated"
    );
}

// ====================================================================
// 013 上原歩夢 (登場 / ライブ開始時): stage 虹のみ -> wait opp ORIGINAL blade <= 2
// ====================================================================

#[test]
fn ayumu_waits_opp_low_blade_when_stage_only_nijigasaki() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let ayumu = g.id(AYUMU);
    let opp_low = g.id(OPP_COST4_B1); // ORIGINAL blade 1 -> legal
    let opp_high = g.id(OPP_B3); // ORIGINAL blade 3 -> NOT legal
    g.state.player1.stage.stage = [ayumu, -1, -1]; // 虹 only
    g.state.player2.stage.stage = [opp_low, opp_high, -1];
    g.add_to_hand(ayumu);
    g.give_energy(20);
    seed_decks(&mut g);
    g.play_to_stage(ayumu, MemberArea::Center);
    drain(&mut g, true);
    assert!(
        is_waited(&g, opp_low),
        "013: blade-1 opp member is waited when stage is 虹-only"
    );
    assert!(
        !is_waited(&g, opp_high),
        "013: blade-3 opp member is NOT waitable (ORIGINAL blade > 2)"
    );
}

#[test]
fn ayumu_no_wait_with_non_nijigasaki_ally() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let ayumu = g.id(AYUMU);
    let outsider = g.id(OUTSIDER);
    let opp = g.id(OPP_COST4_B1);
g.state.player1.stage.stage = [outsider, -1, -1]; // non-虹 ally at show window
    g.state.player2.stage.stage = [opp, -1, -1];
    g.add_to_hand(ayumu);
    g.give_energy(20);
    seed_decks(&mut g);
    g.try_play_to_stage(ayumu, MemberArea::Center)
        .expect("ayumu play to center should succeed (cost 4, 20 energy, center empty)");
    drain(&mut g, true);
    assert!(
        !is_waited(&g, opp),
        "013: with a non-Aq ally on stage the wait must not happen"
    );
}

// ====================================================================
// 015 桜坂しずく (起動 ターン1): wait self + discard 手札 1 -> draw 1
// ====================================================================

#[test]
fn shizuku_kidou_draws_after_waiting_self() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let shizuku = g.id(SHIZUKU);
    g.state.player1.stage.stage = [-1, shizuku, -1];
    seed_decks(&mut g);
    g.state.player1.hand.cards.push(g.id(FILLER));
    let hand_before = g.state.player1.hand.cards.len();
    g.give_energy(4);

    g.activate_ability(shizuku);
    drain(&mut g, true);

    assert!(
        is_waited(&g, shizuku),
        "015: しずく is waited as the cost of the 起動"
    );
    assert_eq!(
        g.state.player1.hand.cards.len(),
        hand_before,
        "015: discard 手札 1 then draw 1 leaves hand size unchanged"
    );
}

// ====================================================================
// 017 宮下 愛 (ライブ開始時): pay 1E optional -> active 1 stage member
// ====================================================================

#[test]
fn ai_pay_activates_a_waited_member() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let ai = g.id(AI);
    let waited = g.id(ALLY);
    g.state.player1.stage.stage = [waited, ai, -1];
    g.state.mods.add_orientation_modifier(waited, "wait");
    g.give_energy(8);
    seed_decks(&mut g);
    drive_to_live_start(&mut g);
    drain(&mut g, true); // pay 1E, then active the waited member
    assert!(
        !is_waited(&g, waited),
        "017: paying the energy activates the waited stage member"
    );
}

#[test]
fn ai_skip_pay_keeps_waited() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let ai = g.id(AI);
    let waited = g.id(ALLY);
    g.state.player1.stage.stage = [waited, ai, -1];
    g.state.mods.add_orientation_modifier(waited, "wait");
    g.give_energy(8);
    seed_decks(&mut g);
    drive_to_live_start(&mut g);
    drain(&mut g, false); // skip the optional energy pay
    assert!(
        is_waited(&g, waited),
        "017: skipping the pay leaves the member waited"
    );
}

// ====================================================================
// 019 優木せつ菜 (登場): heart05 till live end
// ====================================================================

#[test]
fn setsuna_debut_gains_heart05() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let setsu = g.id(SETSUNA);
    seed_decks(&mut g);
    g.add_to_hand(setsu);
    g.give_energy(8);
    g.play_to_stage(setsu, MemberArea::Center);
    drain(&mut g, true);
    let h5 = g.state.mods.get_heart_modifier(setsu, HeartColor::Heart05);
    assert!(h5 >= 1, "019: she gains heart05 on debut, got {}", h5);
}

#[test]
fn setsuna_live_start_waits_only_cost2_or_less() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let setsu = g.id(SETSUNA);
    let opp4 = g.id(OPP_COST4_B1); // cost 4 -> over 019's cost<=2 cap
    let opp5 = g.id(OPP_COST5); // cost 5 -> over the cap
    g.state.player1.stage.stage = [-1, setsu, -1];
    g.state.player2.stage.stage = [opp4, opp5, -1];
    g.give_energy(8);
    seed_decks(&mut g);
    drive_to_live_start(&mut g);
    drain(&mut g, true);
    assert!(
        !is_waited(&g, opp4) && !is_waited(&g, opp5),
        "019: live_start wait targets cost<=2, so cost-4/5 members are NOT waited"
    );
}

// ====================================================================
// 021 天王寺璃奈 (登場): wait 1 opp member cost <= 4
// ====================================================================

#[test]
fn rinne_waits_opp_cost_four_member() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let rinne = g.id(RINNE);
    let opp4 = g.id(OPP_COST4_B1); // cost 4 -> legal
    let opp5 = g.id(OPP_COST5); // cost 5 -> NOT legal
    g.state.player2.stage.stage = [opp4, opp5, -1];
    g.add_to_hand(rinne);
    g.give_energy(20);
    seed_decks(&mut g);
    g.play_to_stage(rinne, MemberArea::Center);
    drain(&mut g, true);
    assert!(is_waited(&g, opp4), "021: cost-4 opp member is waited");
    assert!(
        !is_waited(&g, opp5),
        "021: cost-5 opp member is NOT waited (over the cost-4 limit)"
    );
}
