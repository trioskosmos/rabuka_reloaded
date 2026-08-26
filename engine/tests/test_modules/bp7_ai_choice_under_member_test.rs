/// BP07 C2: PL!N-bp7-005-R 宮下 愛 ab#0.
///
/// 登場：自分のステージに名前の異なる『DiverDiva』のメンバーが2人いる場合、以下から1つを選ぶ。
/// ・エネルギーを2枚アクティブにする。
/// ・自分のエネルギーデッキから、エネルギーカード1枚を自分のステージにいる『虹ヶ咲』のメンバーの下に置く。
///
/// (On appearance) If there are 2 members with distinct card names belonging to
/// "DiverDiva" on your stage, choose 1:
///   - Activate 2 energy cards.
///   - Place 1 energy card from your energy deck under a "Nijigasaki" member on your stage.
///
/// The parser defect (documented in _bp07_ability_gaps_hand_analysis.md C2): option 2
/// used `source:"stage"` + `destination:null` instead of
/// `place_energy_under_member{source:energy_deck, destination:under_member}`.
/// These tests pin the correct behavior.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const AI: &str = "PL!N-bp7-005-R"; // 宮下 愛 — DiverDiva + 虹ヶ咲
const KARIN: &str = "PL!N-bp7-004-R"; // 朝香果林 — DiverDiva + 虹ヶ咲 (distinct name from Ai)
const ENERGY: &str = "LL-E-001-SD";

/// Fire the 登場 ability on a card directly (borrowed from pl_s_bp5_010_test).
fn trigger_debut(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("card should have a 登場 ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::Debut,
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

/// Put `n` WAIT energy cards in player1's energy zone.
fn give_wait_energy(game: &mut TestGame, n: usize) {
    for _ in 0..n {
        let e = game.id(ENERGY);
        game.state.player1.energy_zone.cards.push(e);
    }
}

/// Put `n` energy cards in player1's energy deck.
fn give_energy_deck(game: &mut TestGame, n: usize) {
    for _ in 0..n {
        let e = game.id(ENERGY);
        game.state.player1.energy_deck.cards.push(e);
    }
}

fn active_energy_count(game: &TestGame) -> usize {
    game.state.player1.energy_zone.active_count() as usize
}

/// Two distinct DiverDiva members on stage → the choice is offered.
#[test]
fn ai_two_distinct_diverdiva_offers_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ai = game.id(AI);
    let karin = game.id(KARIN);
    game.state.player1.stage.stage = [ai, karin, -1];

    trigger_debut(&mut game, ai);

    assert!(
        game.has_pending_choice(),
        "2 distinct DiverDiva members should offer the choice"
    );
}

/// Option 0 (エネルギーを2枚アクティブにする): activates 2 WAIT energy cards.
#[test]
fn ai_option_0_activates_two_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ai = game.id(AI);
    let karin = game.id(KARIN);
    game.state.player1.stage.stage = [ai, karin, -1];
    give_wait_energy(&mut game, 2);
    let before = active_energy_count(&game);

    trigger_debut(&mut game, ai);
    assert!(game.has_pending_choice(), "choice should be offered");
    game.select_choice_option(0);

    assert_eq!(
        active_energy_count(&game),
        before + 2,
        "option 0 should activate 2 energy"
    );
}

/// Option 1 (エネルギーデッキから…メンバーの下に置く): moves 1 energy from the
/// energy deck under a 虹ヶ咲 member on stage.
#[test]
fn ai_option_1_places_energy_under_nijigasaki_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ai = game.id(AI);
    let karin = game.id(KARIN);
    game.state.player1.stage.stage = [ai, karin, -1];
    give_energy_deck(&mut game, 3);
    let energy_deck_before = game.state.player1.energy_deck.cards.len();

    trigger_debut(&mut game, ai);
    assert!(game.has_pending_choice(), "choice should be offered");
    game.select_choice_option(1);

    // place_energy_under_member auto-resolves: no member-under prompt follows.
    assert!(
        !game.has_pending_choice(),
        "place_energy_under_member must not prompt for the under-member"
    );

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        energy_deck_before - 1,
        "energy deck should lose exactly 1 card"
    );
    // Exactly one stage member now has an energy card under it.
    let under_total: usize = (0..3)
        .map(|i| game.state.player1.stage.under_cards[i].len())
        .sum();
    assert_eq!(
        under_total, 1,
        "exactly 1 energy card should be under a stage member"
    );
    let _ = MemberArea::Center;
}

/// Condition gate: only 1 DiverDiva member on stage → no choice.
#[test]
fn ai_condition_fails_with_one_diverdiva() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ai = game.id(AI);
    game.state.player1.stage.stage = [ai, -1, -1];

    trigger_debut(&mut game, ai);

    assert!(
        !game.has_pending_choice(),
        "1 DiverDiva member must not offer the choice"
    );
}

/// Condition gate: 2 DiverDiva members of the SAME name → distinct-name fails.
#[test]
fn ai_condition_fails_with_duplicate_name() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ai1 = game.id(AI);
    let ai2 = game.id(AI);
    game.state.player1.stage.stage = [ai1, ai2, -1];

    trigger_debut(&mut game, ai1);

    assert!(
        !game.has_pending_choice(),
        "2 DiverDiva members with the same name must not offer the choice"
    );
}
