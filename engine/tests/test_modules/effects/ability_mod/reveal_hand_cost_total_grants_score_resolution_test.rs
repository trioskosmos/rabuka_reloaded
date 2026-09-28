/// 嵐 千砂都 PL!SP-bp1-003-P ab#0 (起動):
///
/// {{kidou.png/起動}}{{turn1.png/ターン1回}}手札にあるメンバーカードを好きな枚数公開する：
/// 公開したカードのコストの合計が、10、20、30、40、50のいずれかの場合、
/// ライブ終了時まで、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。
///
/// (Activation, once per turn: reveal any number of member cards from hand; if
/// the total cost of the revealed cards is exactly 10, 20, 30, 40 or 50, gain
/// +1 to the live total score until live end.)
///
/// The three tests this replaces were `assert!(true)` placeholders. Worse, both
/// of them revealed two cost-4 cards — a total of 8, which is NOT one of the
/// five printed thresholds — so the 10/20/30/40/50 rule had no real test at all.
/// Each test below uses a total that is deliberately on or off that ladder.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::game_setup::{self, ActionType};

const CHISATO: &str = "PL!SP-bp1-003-P"; // 嵐 千砂都 — the 起動 source
const COST_4: &str = "PL!S-bp2-002-R"; // 桜内梨子
const COST_2: &str = "PL!-sd1-002-SD"; // 絢瀬 絵里

/// 嵐 千砂都 alone in hand's-selection space, with the hand cleared so the
/// revealed-card count is exact. The card prints cost 10 itself; PL!SP-bp1-003
/// and the other bp1-003 prints are one bp number apart.
fn setup() -> (TestGame, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id(CHISATO);
    game.assert_card_identity(chisato, CHISATO);
    game.state.player1.stage.stage[0] = chisato;
    game.state.player1.hand.cards.clear();
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    (game, chisato)
}

/// Put `n` copies of a cost-4 member and `m` of a cost-2 member in hand, so the
/// caller states the cost total directly. Returns the ids in hand order.
fn fill_hand(game: &mut TestGame, fours: usize, twos: usize) -> Vec<i16> {
    let mut ids = Vec::new();
    for _ in 0..fours {
        let c = game.id(COST_4);
        game.assert_card_identity(c, COST_4);
        game.state.player1.hand.cards.push(c);
        ids.push(c);
    }
    for _ in 0..twos {
        let c = game.id(COST_2);
        game.assert_card_identity(c, COST_2);
        game.state.player1.hand.cards.push(c);
        ids.push(c);
    }
    ids
}

fn use_offers(game: &TestGame, cid: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
        })
        .count()
}

/// Activate, reveal `pick` of the hand, then drain whatever follows. Returns
/// whether a reveal prompt was actually raised: with an empty hand there is
/// nothing to 公開, so the effect never asks (get_pending_choice panics on
/// "No pending choice", so this must not be assumed).
fn reveal_and_resolve(game: &mut TestGame, chisato: i16, pick: usize) -> bool {
    assert_eq!(use_offers(game, chisato), 1, "the 起動 must be offered");
    assert!(
        game.try_activate_ability(chisato).is_ok(),
        "activation must succeed"
    );
    if !game.has_pending_choice() {
        return false;
    }
    // 手札にあるメンバーカードを好きな枚数公開する — the choice is out of the HAND.
    match game.get_pending_choice() {
        Choice::SelectCard { zone, .. } => assert_eq!(
            zone, "hand",
            "公開する cards come from the hand, not the deck or stage"
        ),
        other => panic!("expected a hand SelectCard for the reveal, got {other:?}"),
    }
    let picks: Vec<usize> = (0..pick).collect();
    game.select_indices(&picks);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
    true
}

/// 合計が10 (4+4+2) → the printed +1 to the live total score.
#[test]
fn reveal_hand_cost_total_10_grants_live_total_score() {
    let (mut game, chisato) = setup();
    let hand = fill_hand(&mut game, 2, 1);
    game.give_energy(15);

    reveal_and_resolve(&mut game, chisato, 3);

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "合計が10 → ライブ終了時まで、「ライブの合計スコアを＋１する。」"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand.len(),
        "公開する is a reveal, not a move — the cards stay in hand"
    );
    for c in &hand {
        assert!(
            game.state.player1.hand.cards.contains(c),
            "revealed card {c} stayed in hand"
        );
    }
}

/// 合計が20 (4+4+4+4+4) → same reward, a different rung of the ladder.
#[test]
fn reveal_hand_cost_total_20_grants_live_total_score() {
    let (mut game, chisato) = setup();
    let hand = fill_hand(&mut game, 5, 0);
    game.give_energy(15);

    reveal_and_resolve(&mut game, chisato, 5);

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "合計が20 is on the 10/20/30/40/50 ladder"
    );
    assert_eq!(game.state.player1.hand.cards.len(), hand.len());
}

/// 合計が8 (4+4) is NOT on the ladder — the exact-threshold rule must exclude a
/// total that is merely close. This is the case the old placeholders exercised
/// while asserting nothing.
#[test]
fn reveal_hand_cost_total_8_off_ladder_grants_nothing() {
    let (mut game, chisato) = setup();
    fill_hand(&mut game, 2, 0);
    game.give_energy(15);

    reveal_and_resolve(&mut game, chisato, 2);

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        0,
        "合計が8 is not 10/20/30/40/50 → no live total score bonus"
    );
}

/// 合計が12 (4+4+4) is also off-ladder, and 12 < 10 is false while 12 > 10 is
/// true — this pins that the comparison is EXACT equality, not a threshold.
#[test]
fn reveal_hand_cost_total_12_is_exact_not_a_minimum() {
    let (mut game, chisato) = setup();
    fill_hand(&mut game, 3, 0);
    game.give_energy(15);

    reveal_and_resolve(&mut game, chisato, 3);

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        0,
        "合計が12 must not qualify: the rule is equality with 10/20/30/40/50, \
         not '10 or more'"
    );
}

/// An empty hand means no cards to reveal: the prompt is never raised, so there
/// is no cost total and no reward.
#[test]
fn reveal_hand_cost_total_empty_hand_no_cost_total() {
    let (mut game, chisato) = setup();
    game.give_energy(15);

    assert!(
        !reveal_and_resolve(&mut game, chisato, 0),
        "手札にあるメンバーカードを…公開する — with an empty hand there is nothing \
         to reveal, so no choice may be offered"
    );
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        0,
        "an empty hand has no cost total to match against the ladder"
    );
    assert!(!game.has_pending_choice());
}

/// ターン1回 — the second activation in the same turn is neither offered nor
/// accepted, and no second +1 is granted.
#[test]
fn reveal_hand_cost_total_turn1_blocks_second() {
    let (mut game, chisato) = setup();
    fill_hand(&mut game, 2, 1);
    game.give_energy(20);

    reveal_and_resolve(&mut game, chisato, 3);
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "the first activation granted the +1"
    );

    assert_eq!(
        use_offers(&game, chisato),
        0,
        "ターン1回 — the 起動 must not be offered again this turn"
    );
    assert!(
        game.try_activate_ability(chisato).is_err(),
        "the second activation must be refused, got Ok"
    );
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "a refused second activation must not grant a second +1"
    );
}
