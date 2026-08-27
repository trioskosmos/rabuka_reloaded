/// BP07 B5: PL!N-bp7-006-R＋ 近江彼方 ab#1.
///
/// 起動 (ターン2回)：デッキの上からカードを3枚控え室に置く：これにより
/// 控え室に置いたカードの中に『虹ヶ咲』のライブカードか、ブレードハートを
/// 持たない『虹ヶ咲』のメンバーカードがある場合、以下から1つを選ぶ。
/// ・エネルギーを2枚アクティブにする。
/// ・ライブ終了時まで、ブレード×2を得る。
///
/// (Activation, twice per turn) Put the top 3 cards of your deck into the
/// discard: if among the cards placed this way there is a 『虹ヶ咲』 LIVE card
/// OR a 『虹ヶ咲』 member card without a blade heart, choose 1 of:
///   • activate 2 energy
///   • gain blade ×2 until the end of the live
///
/// The parser defect (documented in _bp07_ability_gaps_hand_analysis.md B5):
/// the condition used `location:"stage"` (should scope to the 3 cards just
/// placed into discard), and the live-card OR branch was dropped — only
/// `card_type:member_card` survived. These tests pin the correct behavior.
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

const KANATA: &str = "PL!N-bp7-006-R＋";
const NIJI_LIVE: &str = "PL!N-bp1-026-L"; // 虹ヶ咲 live card (Poppin' Up!)
const NIJI_MEMBER_NO_BLADE: &str = "PL!N-bp1-001-R"; // 虹ヶ咲 member, no blade heart
const NIJI_MEMBER_HAS_BLADE: &str = "PL!N-bp7-007-R＋"; // 虹ヶ咲 member, has blade heart
const NON_NIJI: &str = "PL!SP-sd1-001-SD"; // 澁谷かのん (Liella!, not 虹ヶ咲)

/// Put 彼方 on stage and make the top 3 deck cards exactly `top3` (index 0 = top).
fn setup(game: &mut TestGame, top3: [i16; 3]) -> i16 {
    let kanata = game.id(KANATA);
    game.state.player1.stage.stage = [-1, kanata, -1];
    game.state.player1.main_deck.cards.clear();
    for c in top3 {
        game.state.player1.main_deck.cards.push(c);
    }
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    kanata
}

/// Activate 彼方's ab#1 (the second 起動 ability, index 1). Returns the result.
fn try_activate_ab1(game: &mut TestGame, kanata: i16) -> Result<(), String> {
    TurnEngine::execute_main_phase_action_with_ability_index(
        &mut game.state,
        &ActionType::UseAbility,
        Some(kanata),
        None,
        None,
        None,
        Some(1),
    )
}

/// Activate ab#1 and panic on failure.
fn activate_ab1(game: &mut TestGame, kanata: i16) {
    try_activate_ab1(game, kanata).expect("activate ab#1 failed");
}

/// The cost is paid (top 3 → discard) regardless of the condition outcome.
#[test]
fn kanata_cost_mills_top_three() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let a = game.id(NIJI_LIVE);
    let b = game.id(NON_NIJI);
    let c = game.id(NON_NIJI);
    let kanata = setup(&mut game, [a, b, c]);

    let deck_before = game.state.player1.main_deck.cards.len();
    let wait_before = game.state.player1.waitroom.cards.len();

    activate_ab1(&mut game, kanata);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "cost should mill exactly the top 3 deck cards"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        wait_before + 3,
        "the 3 milled cards should be in the discard"
    );
    let waitroom: Vec<i16> = game.state.player1.waitroom.cards.iter().copied().collect();
    for id in [a, b, c] {
        assert!(waitroom.contains(&id), "card {} should have been discarded", id);
    }
}

/// Top 3 contains a 虹ヶ咲 LIVE card → the choice is offered.
#[test]
fn kanata_live_card_in_discard_offers_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let live = game.id(NIJI_LIVE);
    let fill = game.id(NON_NIJI);
    let kanata = setup(&mut game, [live, fill, fill]);

    activate_ab1(&mut game, kanata);

    assert!(
        game.has_pending_choice(),
        "a 虹ヶ咲 live card in the discarded 3 should offer the choice"
    );
}

/// Top 3 contains a 虹ヶ咲 member card WITHOUT a blade heart → choice offered.
#[test]
fn kanata_member_without_blade_heart_offers_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let member = game.id(NIJI_MEMBER_NO_BLADE);
    let fill = game.id(NON_NIJI);
    let kanata = setup(&mut game, [member, fill, fill]);

    activate_ab1(&mut game, kanata);

    assert!(
        game.has_pending_choice(),
        "a 虹ヶ咲 member without blade heart should offer the choice"
    );
}

/// Top 3 is only a 虹ヶ咲 member WITH a blade heart (no live card) → NO choice.
#[test]
fn kanata_member_with_blade_heart_does_not_offer_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let member = game.id(NIJI_MEMBER_HAS_BLADE);
    let fill = game.id(NON_NIJI);
    let kanata = setup(&mut game, [member, fill, fill]);

    activate_ab1(&mut game, kanata);

    assert!(
        !game.has_pending_choice(),
        "a 虹ヶ咲 member WITH a blade heart must NOT satisfy the no-blade-heart branch"
    );
}

/// Top 3 is entirely non-虹ヶ咲 cards → NO choice.
#[test]
fn kanata_no_matching_card_does_not_offer_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let f = game.id(NON_NIJI);
    let kanata = setup(&mut game, [f, f, f]);

    activate_ab1(&mut game, kanata);

    assert!(
        !game.has_pending_choice(),
        "no 虹ヶ咲 live/member-without-blade card in the discarded 3 → no choice"
    );
}

/// Option 0 (エネルギーを2枚アクティブにする): activates 2 WAIT energy.
#[test]
fn kanata_option_activate_two_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let live = game.id(NIJI_LIVE);
    let fill = game.id(NON_NIJI);
    let kanata = setup(&mut game, [live, fill, fill]);

    // Put 2 energy cards in the zone in WAIT state.
    let energy = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(energy);
    game.state.player1.energy_zone.cards.push(energy);
    game.state.player1.energy_zone.active_count(); // (both start WAIT; active stays 0)

    let active_before = game.state.player1.energy_zone.active_count();

    activate_ab1(&mut game, kanata);
    assert!(game.has_pending_choice(), "choice should be offered");
    game.select_choice_option(0);

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before + 2,
        "option 0 should activate 2 energy"
    );
}

/// Option 1 (ライブ終了時までブレード×2を得る): grants blade ×2 to 彼方.
#[test]
fn kanata_option_gain_blade_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let member = game.id(NIJI_MEMBER_NO_BLADE);
    let fill = game.id(NON_NIJI);
    let kanata = setup(&mut game, [member, fill, fill]);

    let blade_before = game.state.mods.get_blade_modifier(kanata);

    activate_ab1(&mut game, kanata);
    assert!(game.has_pending_choice(), "choice should be offered");
    game.select_choice_option(1);

    assert_eq!(
        game.state.mods.get_blade_modifier(kanata),
        blade_before + 2,
        "option 1 should grant blade ×2"
    );
}

/// The condition scopes to the 3 cards JUST placed by this cost — a 虹ヶ咲 live
/// card already sitting in the discard (NOT among the milled 3) must not
/// satisfy it.
#[test]
fn kanata_preexisting_live_in_discard_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // A 虹ヶ咲 live card already in the discard before activating.
    game.state.player1.waitroom.cards.push(game.id(NIJI_LIVE));

    let f = game.id(NON_NIJI);
    let kanata = setup(&mut game, [f, f, f]);

    activate_ab1(&mut game, kanata);

    assert!(
        !game.has_pending_choice(),
        "the condition must only look at the 3 cards placed by THIS cost"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// ターン2回 — twice per turn
// ═════════════════════════════════════════════════════════════════════════

/// The ability can be used twice per turn (ターン2回).
#[test]
fn kanata_use_limit_twice_per_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let live = game.id(NIJI_LIVE);
    let live2 = game.id(NIJI_LIVE);
    let fill = game.id(NON_NIJI);
    let kanata = setup(&mut game, [live, live2, fill]);

    // First use → cost mills 3 (incl. a live card) → choice offered.
    activate_ab1(&mut game, kanata);
    assert!(game.has_pending_choice(), "first use should offer a choice");
    game.select_choice_option(1);

    // Second use: deck is now fillers (top 3 after the first mill). Use is
    // still allowed (ターン2回), even though the condition fails → no choice.
    activate_ab1(&mut game, kanata);

    // Third use → blocked by the twice-per-turn limit.
    assert!(
        try_activate_ab1(&mut game, kanata).is_err(),
        "third activation in the same turn must be blocked by ターン2回"
    );
}
