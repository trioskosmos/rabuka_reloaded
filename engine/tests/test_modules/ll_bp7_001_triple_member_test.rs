/// Tests for `LL-bp7-001-R＋` 国木田花丸&優木せつ菜&嵐千砂都 — all three abilities.
///
/// ab#0 (常時): このカードのプレイに際し、自分の手札から「国木田花丸」と「優木せつ菜」と
///   「嵐千砂都」のメンバーカードをそれぞれ1枚ずつ控え室に置いてもよい。そうしたとき、
///   このカードのコストは10になる。(base cost 15 → set to 10)
/// ab#1 (登場): 自分の控え室からライブカードを1枚手札に加える。
/// ab#2 (ライブ成功時): 自分の控え室からメンバーカードを1枚手札に加える。
use crate::helpers::*;

/// Card numbers (cheap member cards per character).
const HANAMARU: &str = "PL!S-bp2-016-N"; // 国木田花丸
const SETSUNA: &str = "PL!N-PR-009-PR"; // 優木せつ菜
const CHISATO: &str = "PL!SP-pb1-014-PR"; // 嵐 千砂都
const TRIPLE: &str = "LL-bp7-001-R＋"; // 国木田花丸&優木せつ菜&嵐千砂都 (cost 15)
const LIVE_CARD: &str = "PL!-sd1-020-SD"; // live card for live start / live success

// ====================================================================
// ab#0 (常時): cost becomes 10 when 1 of each named character is in discard
// ====================================================================

/// No named cards in discard → play cost stays base 15.
#[test]
fn triple_cost_unchanged_without_named_discards() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    game.state.player1.hand.cards.push(triple);
    // Empty discard → condition (1 of each in discard) false → no cost set.
    game.state.recalculate_constants();

    // Base cost 15; no modifier.
    assert_eq!(
        game.state.mods.get_cost_modifier(triple),
        0,
        "no cost modifier without named members in discard"
    );
}

/// All three named characters (1 each) in discard → cost modifier set to 10.
#[test]
fn triple_cost_set_to_10_when_all_three_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.waitroom.cards.push(hanamaru);
    game.state.player1.waitroom.cards.push(setsuna);
    game.state.player1.waitroom.cards.push(chisato);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(triple),
        10,
        "cost should be set to 10 when all 3 named members are in discard"
    );
}

/// Only TWO of the three named characters in discard → condition false → no set.
#[test]
fn triple_cost_unchanged_with_two_of_three() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.waitroom.cards.push(hanamaru);
    game.state.player1.waitroom.cards.push(setsuna);
    // No 嵐千砂都 → one of the three conditions fails.

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(triple),
        0,
        "cost modifier must NOT apply with only 2 of the 3 named members"
    );
}

/// ONE of the three in discard → no set.
#[test]
fn triple_cost_unchanged_with_one_of_three() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.waitroom.cards.push(hanamaru);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(triple),
        0,
        "cost modifier must NOT apply with only 1 named member"
    );
}

/// Two cards of the SAME character don't satisfy the "それぞれ1枚ずつ" (one of
/// each) requirement — need one 花丸 AND one せつ菜 AND one 千砂都.
#[test]
fn triple_cost_unchanged_with_duplicate_character() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    let hanamaru1 = game.id(HANAMARU);
    let hanamaru2 = game.id("PL!S-bp2-007-R＋"); // second 国木田花丸
    let setsuna = game.id(SETSUNA);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.waitroom.cards.push(hanamaru1);
    game.state.player1.waitroom.cards.push(hanamaru2);
    game.state.player1.waitroom.cards.push(setsuna);
    // No 嵐千砂都 → condition fails even though 2 花丸 + 1 せつ菜 are there.

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(triple),
        0,
        "duplicate character must not satisfy the one-of-each requirement"
    );
}

/// Cards in HAND (not discard) don't count — the condition requires them IN
/// the discard (placed from hand to waiting room).
#[test]
fn triple_cost_unchanged_with_named_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.hand.cards.push(hanamaru);
    game.state.player1.hand.cards.push(setsuna);
    game.state.player1.hand.cards.push(chisato);
    // All three named members are in HAND, not discard.

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(triple),
        0,
        "named members in hand must not count (must be in discard)"
    );
}

/// GAMEPLAY: with 1 of each named member in discard, playing the card costs 10
/// (base 15 → set to 10), verified via energy actually spent.
#[test]
fn triple_gameplay_play_costs_10_when_all_three_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    let hanamaru = game.id(HANAMARU);
    let setsuna = game.id(SETSUNA);
    let chisato = game.id(CHISATO);
    game.state.player1.hand.cards.push(triple);
    game.state.player1.waitroom.cards.push(hanamaru);
    game.state.player1.waitroom.cards.push(setsuna);
    game.state.player1.waitroom.cards.push(chisato);
    game.give_energy(15);

    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();

    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(
        remaining, 5,
        "cost set to 10 (15 given − 10 spent = 5 left)"
    );
}

/// GAMEPLAY: without the named members in discard, playing costs 15 (base).
#[test]
fn triple_gameplay_play_costs_15_without_named_discards() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    game.state.player1.hand.cards.push(triple);
    // No named members in discard.
    game.give_energy(15);

    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();

    let remaining = game.state.player1.energy_zone.active_count();
    assert_eq!(
        remaining, 0,
        "cost stays base 15 (15 given − 15 spent = 0 left)"
    );
}

// ====================================================================
// ab#1 (登場): add 1 live card from waitroom to hand
// ====================================================================

/// On debut, a live card in the waitroom is added to hand.
#[test]
fn triple_debut_adds_live_card_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    let live = game.id(LIVE_CARD);
    game.state.player1.waitroom.cards.push(live);
    game.state.player1.hand.cards.push(triple);
    game.give_energy(16); // cost 15

    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);

    // The live card moved from waitroom to hand.
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "debut should add a live card from waitroom to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "the live card should leave the waitroom"
    );
}

/// ab#1 with no live card in waitroom → nothing to add (no crash).
#[test]
fn triple_debut_no_live_card_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    game.state.player1.hand.cards.push(triple);
    // Waitroom has only a member card, no live card.
    let member = game.id("PL!-sd1-010-SD");
    game.state.player1.waitroom.cards.push(member);
    game.give_energy(16);

    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);

    // The member card stays (ab#1 only grabs live cards).
    assert!(
        game.state.player1.waitroom.cards.contains(&member),
        "non-live cards in waitroom are not touched by ab#1"
    );
}

// ====================================================================
// ab#2 (ライブ成功時): add 1 member card from waitroom to hand
// ====================================================================

/// Fire ab#2 (ライブ成功時) directly, following the convention in
/// chisato_live_success_test.rs / jimo_ai_dash_test.rs.
fn trigger_live_success(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
}

/// Live success: a member card in the waitroom is added to hand.
#[test]
fn triple_live_success_adds_member_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    game.state.player1.stage.stage = [-1, triple, -1];
    let member = game.id("PL!-sd1-010-SD");
    game.state.player1.waitroom.cards.push(member);

    trigger_live_success(&mut game, triple);

    // ab#2 fires at live success → member card to hand.
    assert!(
        game.state.player1.hand.cards.contains(&member),
        "live success should add a member card from waitroom to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&member),
        "the member card should leave the waitroom"
    );
}

/// Live success with a LIVE card in waitroom (not a member) → untouched.
#[test]
fn triple_live_success_ignores_live_cards_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let triple = game.id(TRIPLE);
    game.state.player1.stage.stage = [-1, triple, -1];
    let live_in_waitroom = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(live_in_waitroom);

    trigger_live_success(&mut game, triple);

    // ab#2 grabs MEMBER cards only — the live card stays.
    assert!(
        game.state
            .player1
            .waitroom
            .cards
            .contains(&live_in_waitroom),
        "live cards in waitroom are not touched by ab#2"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&live_in_waitroom),
        "live card must not be added to hand by ab#2"
    );
}
