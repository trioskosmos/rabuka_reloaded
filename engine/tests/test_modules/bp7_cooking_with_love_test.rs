/// BP07 B6: PL!N-bp7-028-L Cooking with Love ab#0.
///
/// ライブ開始時：自分の控え室に『虹ヶ咲』のライブカードと、ブレードハートを
/// 持たない『虹ヶ咲』のメンバーカードがある場合、自分の控え室にあるすべての
/// カードをシャッフルし、デッキの下に置いてもよい。そうしたとき、ライブ終了時
/// まで、自分のステージにいるすべての『虹ヶ咲』のメンバーはheart01を得る。
///
/// (Live start) If in your discard there is a 『虹ヶ咲』 LIVE card AND a
/// 『虹ヶ咲』 member card without a blade heart, you MAY shuffle all cards in
/// your discard and put them on the bottom of the deck. When you do, until the
/// end of the live, all 『虹ヶ咲』 members on your stage gain heart01.
///
/// The parser defects (documented in _bp07_ability_gaps_hand_analysis.md B6):
///   - the condition used `location:"stage"` instead of discard, and
///   - the AND-branch (LIVE card AND member-without-blade-heart) was collapsed
///     to `card_type:member_card` only — the LIVE-card requirement was lost.
/// These tests pin the correct behavior.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const COOKING: &str = "PL!N-bp7-028-L"; // Cooking with Love (trigger card)
const NIJI_LIVE: &str = "PL!N-bp1-026-L"; // 虹ヶ咲 live card (Poppin' Up!)
const NIJI_MEMBER_NO_BLADE: &str = "PL!N-bp1-001-R"; // 虹ヶ咲 member, no blade heart
const NIJI_MEMBER_HAS_BLADE: &str = "PL!N-bp7-007-R＋"; // 虹ヶ咲 member, has blade heart
const NON_NIJI: &str = "PL!SP-sd1-001-SD"; // 澁谷かのん (Liella!, not 虹ヶ咲)

/// Fire the ライブ開始時 ability on a card directly (borrowed from the BP6 audit
/// tests) so we don't need to set up a whole live phase to reach it.
fn trigger_live_start(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("card should have a ライブ開始時 ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
    // Drain any SelectAutoAbility confirmation before inspecting the result.
    game.drain_auto_ability_choices();
}

/// Fire the ability using a fresh copy of the Cooking with Love card. Bound to a
/// temp so the mutable borrow in trigger_live_start doesn't alias the immutable
/// `game.id()` borrow.
fn trigger_cooking(game: &mut TestGame) {
    let c = game.id(COOKING);
    trigger_live_start(game, c);
}

/// Put a card in player1's waitroom (discard). Returns its id.
fn discard(game: &mut TestGame, card_no: &str) -> i16 {
    let id = game.id(card_no);
    game.state.player1.waitroom.cards.push(id);
    id
}

/// Put 虹ヶ咲 members + a non-虹ヶ咲 member on stage so heart01 gain can be
/// observed. Returns (虹ヶ咲A, 虹ヶ咲B, non-虹ヶ咲) stage ids.
fn stage_members(game: &mut TestGame) -> (i16, i16, i16) {
    let a = game.id(NIJI_MEMBER_NO_BLADE);
    let b = game.id(NIJI_MEMBER_HAS_BLADE);
    let non = game.id(NON_NIJI);
    game.state.player1.stage.stage = [a, b, non];
    (a, b, non)
}

// ═════════════════════════════════════════════════════════════════════════
// Condition: AND of (LIVE card) + (member WITHOUT blade heart) in DISCARD
// ═════════════════════════════════════════════════════════════════════════

/// Both a 虹ヶ咲 LIVE card AND a 虹ヶ咲 member without blade heart in discard →
/// the "してもよい" (may shuffle) option is offered.
#[test]
fn cooking_both_in_discard_offers_optional() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    discard(&mut game, NIJI_LIVE);
    discard(&mut game, NIJI_MEMBER_NO_BLADE);
    stage_members(&mut game);

    trigger_cooking(&mut game);

    assert!(
        game.has_pending_choice(),
        "LIVE + member-without-blade in discard should offer the may-shuffle option"
    );
    game.assert_conditional_optional(&["Skip", "Pay"]);
}

/// Only a 虹ヶ咲 LIVE card (no member card) in discard → NO offer (AND).
#[test]
fn cooking_only_live_card_no_offer() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    discard(&mut game, NIJI_LIVE);
    stage_members(&mut game);

    trigger_cooking(&mut game);

    assert!(
        !game.has_pending_choice(),
        "LIVE card alone must not satisfy the AND condition"
    );
}

/// Only a 虹ヶ咲 member WITHOUT blade heart (no LIVE card) in discard → NO offer.
#[test]
fn cooking_only_member_without_blade_no_offer() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    discard(&mut game, NIJI_MEMBER_NO_BLADE);
    stage_members(&mut game);

    trigger_cooking(&mut game);

    assert!(
        !game.has_pending_choice(),
        "member-without-blade alone must not satisfy the AND condition"
    );
}

/// A 虹ヶ咲 LIVE card + a 虹ヶ咲 member WITH a blade heart (no member without)
/// → NO offer: the blade-heartless member is missing.
#[test]
fn cooking_live_plus_member_with_blade_no_offer() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    discard(&mut game, NIJI_LIVE);
    discard(&mut game, NIJI_MEMBER_HAS_BLADE);
    stage_members(&mut game);

    trigger_cooking(&mut game);

    assert!(
        !game.has_pending_choice(),
        "a member WITH a blade heart must not satisfy the no-blade-heart prong"
    );
}

/// Only a 虹ヶ咲 member WITH a blade heart in discard → NO offer.
#[test]
fn cooking_only_member_with_blade_no_offer() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    discard(&mut game, NIJI_MEMBER_HAS_BLADE);
    stage_members(&mut game);

    trigger_cooking(&mut game);

    assert!(
        !game.has_pending_choice(),
        "member WITH blade heart alone must not satisfy the condition"
    );
}

/// Only non-虹ヶ咲 cards in discard → NO offer.
#[test]
fn cooking_non_niji_in_discard_no_offer() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    discard(&mut game, NON_NIJI);
    discard(&mut game, NON_NIJI);
    stage_members(&mut game);

    trigger_cooking(&mut game);

    assert!(
        !game.has_pending_choice(),
        "non-虹ヶ咲 cards must not satisfy the condition"
    );
}

/// The condition must scope to DISCARD, not stage: with the qualifying cards on
/// stage (and discard empty) there must be NO offer. This pins the parser's
/// wrong `location:"stage"` bug.
#[test]
fn cooking_condition_checks_discard_not_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Qualifying cards sit on STAGE, not in discard.
    let member = game.id(NIJI_MEMBER_NO_BLADE);
    let filler = game.id(NON_NIJI);
    let _live = game.id(NIJI_LIVE);
    game.state.player1.stage.stage = [member, filler, filler];

    trigger_cooking(&mut game);

    assert!(
        !game.has_pending_choice(),
        "cards on stage (not in discard) must not satisfy the discard condition"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Optional acceptance / decline
// ═════════════════════════════════════════════════════════════════════════

/// Accepting (Pay): ALL discard cards are shuffled and moved to the deck bottom,
/// so the discard becomes empty.
#[test]
fn cooking_accept_shuffles_discard_to_deck_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let live = discard(&mut game, NIJI_LIVE);
    let member = discard(&mut game, NIJI_MEMBER_NO_BLADE);
    let extra = discard(&mut game, NON_NIJI);
    stage_members(&mut game);
    let deck_before = game.state.player1.main_deck.cards.len();

    trigger_cooking(&mut game);
    assert!(game.has_pending_choice(), "option should be offered");
    game.select_option(1); // Pay (accept)

    assert!(
        game.state.player1.waitroom.cards.is_empty(),
        "all discard cards should be shuffled to the deck bottom"
    );
    let deck = &game.state.player1.main_deck.cards;
    assert_eq!(deck.len(), deck_before + 3, "3 discard cards go back to deck");
    // All 3 landed on the bottom (last 3 positions).
    for id in [live, member, extra] {
        assert!(
            deck[deck.len() - 3..].contains(&id),
            "discarded card should land on the deck bottom"
        );
    }
}

/// Declining (Skip): nothing happens — discard untouched, no heart01 gained.
#[test]
fn cooking_decline_does_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    discard(&mut game, NIJI_LIVE);
    let member = discard(&mut game, NIJI_MEMBER_NO_BLADE);
    let (stage_a, _, _) = stage_members(&mut game);
    let discard_before = game.state.player1.waitroom.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();

    trigger_cooking(&mut game);
    assert!(game.has_pending_choice(), "option should be offered");
    game.select_option(0); // Skip (decline)

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before,
        "declining must leave the discard untouched"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declining must not touch the deck"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(stage_a, HeartColor::Heart01),
        0,
        "declining must not grant heart01"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&member),
        "discarded member should remain in discard after declining"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Consequence: そうしたとき → all 虹ヶ咲 members gain heart01 until live end
// ═════════════════════════════════════════════════════════════════════════

/// After accepting, all 虹ヶ咲 members on stage gain heart01; a non-虹ヶ咲
/// member on stage gains nothing.
#[test]
fn cooking_accept_gives_heart01_to_all_niji_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    discard(&mut game, NIJI_LIVE);
    discard(&mut game, NIJI_MEMBER_NO_BLADE);
    let (member_a, member_b, non_niji) = stage_members(&mut game);

    trigger_cooking(&mut game);
    assert!(game.has_pending_choice(), "option should be offered");
    game.select_option(1); // Pay (accept)

    assert_eq!(
        game.state.mods.get_heart_modifier(member_a, HeartColor::Heart01),
        1,
        "虹ヶ咲 member should gain heart01"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(member_b, HeartColor::Heart01),
        1,
        "the other 虹ヶ咲 member should gain heart01"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(non_niji, HeartColor::Heart01),
        0,
        "a non-虹ヶ咲 member must NOT gain heart01"
    );
}
