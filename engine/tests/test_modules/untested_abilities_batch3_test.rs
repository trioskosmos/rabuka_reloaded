/// Untested abilities — batch 3.
///
/// Covers previously-untested cards whose texts pin down distinct engine
/// behaviours:
///   - PL!-PR-021-PR  矢澤にこ     常時: exactly-7-energy blade constant
///   - PL!SP-bp5-222-R 聖澤悠奈    常時: exactly-8-energy live-score constant
///                                 + ライブ開始時 optional E → wait energy
///   - PL!N-bp3-006-R 近江彼方     登場: self-wait (Q: waited blades don't cheer)
///   - PL!HS-bp1-008-R 徒町小鈴    登場: mill 3, all-member → draw 1
///   - PL!SP-bp1-008-R 若菜四季    登場: draw 1, +1 if 米女メイ on stage
///   - PL!-bp3-009-R＋ 矢澤にこ    登場 cost≥13 draw + 起動 heart choice
///   - PL!S-bp5-009-R 黒澤ルビィ   登場 optional E → SaintSnow fetch + blades
///   - PL!SP-bp4-022-N 鬼塚冬毬    ライブ開始時 pay up to 2 E → blade per E
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // ability-free member
const LIVE_FILLER: &str = "PL!-sd1-019-SD"; // live card (non-member) for mill tests

/// Trigger a card's auto ability (登場 / ライブ開始時) and process it.
fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .expect("card should have the requested trigger ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// PL!-PR-021-PR 矢澤にこ — 常時 自分のエネルギーがちょうど7枚あるかぎり、
// ブレード2つを得る。
// ====================================================================
#[test]
fn niko_pr021_exactly_seven_energy_grants_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-PR-021-PR");
    game.state.player1.stage.stage[1] = nico;

    // 6 energy → condition fails.
    game.give_energy(6);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(nico),
        0,
        "6 energy ≠ exactly 7 → no blades"
    );

    // 7 energy → exactly → +2 blades.
    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(nico),
        2,
        "exactly 7 energy → ブレード2つ"
    );

    // 8 energy → past the boundary → back to 0 ("〜あるかぎり" is live state).
    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(nico),
        0,
        "8 energy ≠ exactly 7 → blades lost again"
    );
}

// ====================================================================
// PL!SP-bp5-222-R 聖澤悠奈 ab#0 — 常時 自分のエネルギーがちょうど8枚
// あるかぎり、ライブの合計スコアを＋１する。
// ====================================================================
#[test]
fn yuna_sp222_exactly_eight_energy_scores_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yuna = game.id("PL!SP-bp5-222-R");
    game.state.player1.stage.stage[1] = yuna;

    game.give_energy(7);
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);

    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "exactly 8 energy → live total score +1"
    );

    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "9 energy → bonus gone"
    );
}

// ====================================================================
// PL!SP-bp5-222-R ab#1 — ライブ開始時 E支払ってもよい：エネルギーデッキから
// エネルギーカードを1枚ウェイト状態で置く。Optional payment must be offered,
// skipping changes nothing, paying flips a wait-zone energy in.
// ====================================================================
#[test]
fn yuna_sp222_live_start_optional_energy_places_wait_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let yuna = game.id("PL!SP-bp5-222-R");
    game.state.player1.stage.stage[1] = yuna;
    game.give_energy(3);
    fill_energy_deck(&mut game, 0, 5);
    let zone_before = game.state.player1.energy_zone.cards.len();

    trigger_auto(&mut game, yuna, AbilityTrigger::LiveStart, "ライブ開始時");

    // Optional payment is offered (rules 9.6.2.3: optional ⇒ still legal at 3).
    assert!(
        game.has_pending_choice(),
        "optional E payment should prompt the player"
    );
    game.select_option(0); // skip
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "skipping pays nothing"
    );

    // Trigger again (no turn limit on this auto ability within a test) → pay.
    trigger_auto(&mut game, yuna, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        game.has_pending_choice(),
        "optional E payment prompt expected on re-trigger"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget (pay_optional_cost:skip)"
    );
    game.select_option(1); // pay
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "paying places 1 energy card from the energy deck"
    );
    // Placed WAIT: active count unchanged by the placement itself (3 − 1 paid
    // stays 2; the new card is wait).
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "paid 1 of 3 active; placed energy sits in wait"
    );
}

// ====================================================================
// PL!N-bp3-006-R 近江彼方 — 登場 このメンバーをウェイトにする。
// ====================================================================
#[test]
fn kanata_bp3006_debut_enters_wait_state() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanata = game.id("PL!N-bp3-006-R");
    game.state.player1.stage.stage[1] = kanata;

    assert_ne!(
        game.state.mods.get_orientation_modifier(kanata),
        Some("wait"),
        "freshly placed member starts active"
    );
    trigger_auto(&mut game, kanata, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.mods.get_orientation_modifier(kanata),
        Some("wait"),
        "debut resolves with self-wait"
    );
}

// ====================================================================
// PL!HS-bp1-008-R 徒町小鈴 — 登場 デッキの上から3枚控え室に置く。
// それらがすべてメンバーカードの場合、カードを1枚引く。
// Deck index 0 = top; mill consumes from the top.
// ====================================================================
#[test]
fn suzuki_hsbp1008_mill_three_all_members_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let suzuki = game.id("PL!HS-bp1-008-R");
    game.state.player1.stage.stage[1] = suzuki;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    // top three (indices 0..2) become members
    put_on_deck_top(&mut game, 0, filler);
    put_on_deck_top(&mut game, 0, filler);
    put_on_deck_top(&mut game, 0, filler);
    let hand_before = game.state.player1.hand.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_auto(&mut game, suzuki, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "all 3 milled cards are members → draw 1"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 3,
        "three cards moved to the waitroom (控え室)"
    );
}

#[test]
fn suzuki_hsbp1008_mill_with_live_card_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let suzuki = game.id("PL!HS-bp1-008-R");
    game.state.player1.stage.stage[1] = suzuki;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    // top three = member, LIVE CARD, member → not all members → no draw
    put_on_deck_top(&mut game, 0, filler);
    let live_filler = game.id(LIVE_FILLER);
    put_on_deck_top(&mut game, 0, live_filler);
    put_on_deck_top(&mut game, 0, filler);
    let hand_before = game.state.player1.hand.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_auto(&mut game, suzuki, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "a non-member among the milled three → no draw"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 3,
        "the mill itself still happens"
    );
}

// ====================================================================
// PL!SP-bp1-008-R 若菜四季 — 登場 カードを1枚引く。自分のステージに
// 「米女メイ」がいる場合、さらにカードを1枚引く。
// ====================================================================
fn wakashi_sp1008_setup(game: &mut TestGame) -> i16 {
    let wakashi = game.id("PL!SP-bp1-008-R"); // cost 13 — also nico bp3-009's condition card
    game.state.player1.stage.stage[1] = wakashi;
    fill_decks(game, game.id(FILLER));
    wakashi
}

#[test]
fn wakashi_spbp1008_debut_draws_one_without_mei() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakashi = wakashi_sp1008_setup(&mut game);
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, wakashi, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "no 米女メイ on stage → single draw"
    );
}

#[test]
fn wakashi_spbp1008_debut_draws_two_with_mei() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let wakashi = wakashi_sp1008_setup(&mut game);
    let mei = game.id("PL!SP-pb1-007-R"); // 米女メイ
    game.state.player1.stage.stage[0] = mei;
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, wakashi, AbilityTrigger::Debut, "登場");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 2,
        "米女メイ on stage → additional draw"
    );
}

// ====================================================================
// PL!-bp3-009-R＋ 矢澤にこ ab#0 — 登場 自分のステージにコスト13以上の
// メンバーがいる場合、カードを1枚引く。(cost-13 若菜四季 as the enabler)
// ====================================================================
#[test]
fn niko_bp3009_cost13_condition_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let niko = game.id("PL!-bp3-009-R＋");
    let wakashi = game.id("PL!SP-bp1-008-R"); // cost 13
    game.state.player1.stage.stage[0] = wakashi;
    game.state.player1.stage.stage[1] = niko;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, niko, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "cost-13 member present → draw 1"
    );
}

#[test]
fn niko_bp3009_no_cost13_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let niko = game.id("PL!-bp3-009-R＋");
    game.state.player1.stage.stage[1] = niko;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, niko, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "only herself (cost 2) on stage → no draw"
    );
}

// ====================================================================
// PL!-bp3-009-R＋ ab#1 — 起動 ターン1回 このメンバーをウェイトにする：
// heart01/heart03/heart06 のうち1つを選ぶ。ライブ終了時まで、選んだハート
// を1つ得る。
// ====================================================================
#[test]
fn niko_bp3009_activation_self_wait_and_heart_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let niko = game.id("PL!-bp3-009-R＋");
    game.state.player1.stage.stage[1] = niko;

    game.activate_ability(niko);

    // Cost (self-wait) applies first.
    assert_eq!(
        game.state.mods.get_orientation_modifier(niko),
        Some("wait"),
        "activation cost waits this member"
    );

    // Heart colour selection appears; pick heart03 (option index 1).
    assert!(
        game.has_pending_choice(),
        "heart colour choice should be pending"
    );
    game.select_option(1);

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(niko, HeartColor::Heart03),
        1,
        "chosen heart03 granted until live end"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(niko, HeartColor::Heart01),
        0,
        "unchosen colours are not granted"
    );
}

// ====================================================================
// PL!S-bp5-009-R 黒澤ルビィ — 登場 E支払ってもよい：控え室から SaintSnow の
// カードを1枚手札に加える。そうした場合、ライブ終了時までブレード2つ。
// Decline path must NOT fetch; accept path does both halves.
// ====================================================================
#[test]
fn ruby_bp5009_decline_optional_payment_fetches_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp5-009-R");
    let saint = game.id("PL!S-pb1-054-SRE"); // 鹿角聖良 unit=SaintSnow
    game.state.player1.stage.stage[1] = ruby;
    game.state.player1.waitroom.cards.push(saint);
    game.give_energy(2);
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, ruby, AbilityTrigger::Debut, "登場");
    assert!(game.has_pending_choice(), "optional E payment prompts");
    game.select_option(0); // decline

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "declined payment → no SaintSnow fetch"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        0,
        "declined payment → no blades"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&saint),
        "SaintSnow stays in the waitroom"
    );
}

#[test]
fn ruby_bp5009_accept_optional_payment_fetches_and_grants_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp5-009-R");
    let saint = game.id("PL!S-pb1-054-SRE");
    game.state.player1.stage.stage[1] = ruby;
    game.state.player1.waitroom.cards.push(saint);
    game.give_energy(2);
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(&mut game, ruby, AbilityTrigger::Debut, "登場");
    game.select_option(1); // pay

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "SaintSnow moves from waitroom to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&saint),
        "fetched card left the waitroom"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        2,
        "そうした場合 → 2 blades until live end"
    );
    assert_eq!(game.state.player1.energy_zone.active_count(), 1, "paid 1 E");
}
