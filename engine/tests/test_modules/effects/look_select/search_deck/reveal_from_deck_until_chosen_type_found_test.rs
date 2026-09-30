/// Tests for PL!-pb1-001-R (高坂穂乃果) ab#0 — Q166, Q167
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

fn setup_honoka_deck(game: &mut TestGame, honoka: i16, deck_top: Vec<i16>) {
    game.state.player1.stage.stage[1] = honoka;
    let hand_filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(hand_filler);
    game.state.player1.hand.cards.push(hand_filler);
    game.state.player1.main_deck.cards = deck_top.into();
    game.give_energy(13);
}

fn activate_and_choose_type(game: &mut TestGame, honoka: i16, type_option: i16) {
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(honoka),
        None,
        None,
        None,
    )
    .expect("activate");
    assert!(
        game.has_pending_choice(),
        "Should have hand discard cost choice"
    );
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "Should have card type choice");
    let desc = game
        .state
        .get_pending_choice_json()
        .and_then(|j| {
            j.get("description")
                .and_then(|d| d.as_str().map(|s| s.to_string()))
        })
        .unwrap_or_default();
    assert!(
        desc.contains("Live card"),
        "Choice should mention Live card, got: {}",
        desc
    );
    assert!(
        desc.contains("Member card"),
        "Choice should mention Member card, got: {}",
        desc
    );
    game.select_option(type_option);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

#[test]
fn reveal_until_chosen_type_q166_member_skips_live_fillers() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let live_filler = game.id("PL!-sd1-019-SD");
    let target_member = game.id("PL!SP-bp2-006-P");
    let mut deck = Vec::new();
    for _ in 0..4 {
        deck.push(live_filler);
    }
    deck.push(target_member);
    setup_honoka_deck(&mut game, honoka, deck);
    activate_and_choose_type(&mut game, honoka, 1);
    assert!(
        game.state.player1.hand.cards.contains(&target_member),
        "Target member should be in hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&live_filler),
        "Live fillers should NOT be in hand"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&target_member),
        "Target member should not remain in deck"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "Deck should be exhausted (all 5 cards revealed)"
    );
}

#[test]
fn reveal_until_chosen_type_q166_target_first_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let target = game.id("PL!SP-bp2-006-P");
    let filler = game.id("PL!-sd1-019-SD");
    setup_honoka_deck(
        &mut game,
        honoka,
        vec![target, filler, filler, filler, filler, filler],
    );
    activate_and_choose_type(&mut game, honoka, 1);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "Target should be in hand"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&target),
        "Target should not remain in deck"
    );
}

#[test]
fn reveal_until_chosen_type_q166_target_last_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let target = game.id("PL!SP-bp2-006-P");
    let filler = game.id("PL!-sd1-019-SD");
    let mut deck = Vec::new();
    for _ in 0..9 {
        deck.push(filler);
    }
    deck.push(target);
    setup_honoka_deck(&mut game, honoka, deck);
    activate_and_choose_type(&mut game, honoka, 1);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "Target should be in hand"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "All deck cards should have been revealed"
    );
}

#[test]
fn reveal_until_chosen_type_q166_live_card_skips_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let member = game.id("PL!SP-bp2-006-P");
    let target_live = game.id("PL!-sd1-019-SD");
    setup_honoka_deck(&mut game, honoka, vec![member, member, member, target_live]);
    activate_and_choose_type(&mut game, honoka, 0);
    assert!(
        game.state.player1.hand.cards.contains(&target_live),
        "Live card should be in hand"
    );
    let hand_member = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&id| id == member)
        .count();
    assert_eq!(hand_member, 0, "Member cards should NOT be in hand");
}

#[test]
fn reveal_until_chosen_type_q166_two_matches_only_one_added() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let m1 = game.id("PL!SP-bp2-006-P");
    let m2 = game.id("PL!SP-bp2-006-P");
    let filler = game.id("PL!-sd1-019-SD");
    setup_honoka_deck(&mut game, honoka, vec![m1, filler, filler, m2]);
    activate_and_choose_type(&mut game, honoka, 1);
    let in_hand = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&id| id == m1 || id == m2)
        .count();
    assert_eq!(
        in_hand, 1,
        "Only 1 of 2 matching cards added (reveal stops at first)"
    );
}

#[test]
fn reveal_until_chosen_type_q166_no_member_in_deck_refresh() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let filler = game.id("PL!-sd1-019-SD");
    setup_honoka_deck(&mut game, honoka, vec![filler; 10]);
    activate_and_choose_type(&mut game, honoka, 1);
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "Deck exhausted"
    );
    // Revealed cards are in gs.revealed_cards, not moved to discard
    // since no matching card was found (all cards are live, not member).
    // The only card in waitroom is the hand_filler discarded as cost.
}

#[test]
fn reveal_until_chosen_type_q167_deck_exhausted_during_reveal() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let target = game.id("PL!SP-bp2-006-P");
    let filler = game.id("PL!-sd1-019-SD");
    let mut deck = Vec::new();
    for _ in 0..5 {
        deck.push(filler);
    }
    deck.push(target);
    setup_honoka_deck(&mut game, honoka, deck);
    activate_and_choose_type(&mut game, honoka, 1);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "Target should be in hand after refresh"
    );
}

#[test]
fn reveal_until_chosen_type_center_requirement_left_side_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    game.state.player1.stage.stage[0] = honoka;
    let hf = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(hf);
    game.state.player1.hand.cards.push(hf);
    game.give_energy(13);
    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(honoka),
        None,
        None,
        None,
    );
    assert!(
        result.is_err(),
        "Ability should fail from left side (center required)"
    );
    // The refusal must be the センター requirement, and it must have cost
    // nothing: the 13 energy is unspent, no prompt opened, and 紗羽司 is still
    // standing in the left side rather than waited.
    game.assert_card_identity(honoka, "PL!-pb1-001-R");
    game.assert_energy_untouched_after_refusal(13, "紗羽司 起動 from the left side");
    assert_eq!(
        game.state.player1.stage.stage,
        [honoka, -1, -1],
        "a refused 起動 leaves the stage as it was"
    );
    assert_ne!(
        game.state.mods.get_orientation_modifier(honoka),
        Some("wait"),
        "a refused 起動 must not wait her as its cost"
    );
    assert!(
        !game.has_pending_choice(),
        "a refused activation must not open the type-choice prompt"
    );
}

/// 「**コスト10以上の**メンバーカード」 is a real constraint, and nothing in this file
/// tests it.
///
/// Every member test above uses `PL!SP-bp2-006-P`, which costs **exactly 10** — the bound —
/// so it satisfies `>= 10` and can never contradict the filter. The live tests use
/// `PL!-sd1-019-SD` (START:DASH!!), a live printing **score 1**; the cost clause does not
/// apply to the live branch at all, so those tests never exercised the filter either.
/// Between them the fixtures sit on the safe side of every line, which is why a dropped
/// `コスト10以上` went unnoticed.
///
/// This stages a deck of sub-10 members and nothing else, so no arrangement of the deck can
/// accidentally produce a qualifying card: the chosen card cannot be found, the deck is
/// exhausted, and hand gains no member.
#[test]
fn reveal_until_chosen_type_a_member_below_cost_ten_does_not_satisfy_the_cost_clause() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");

    // 高坂 穂乃果 — a メンバー of cost 4, two below the printed コスト10以上.
    let cheap = game.id("PL!-sd1-010-SD");
    game.assert_card_identity(cheap, "PL!-sd1-010-SD");
    game.assert_card_cost(cheap, 4);

    setup_honoka_deck(&mut game, honoka, vec![cheap; 6]);
    activate_and_choose_type(&mut game, honoka, 1);

    assert!(
        !game.state.player1.hand.cards.contains(&cheap),
        "コスト10以上のメンバーカード — a メンバー of cost 4 is below the bound, so the \
         reveal must not stop on it and it must not reach hand (hand={:?})",
        game.state.player1.hand.cards
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        0,
        "every card was revealed looking for a qualifying card, so the deck empties without \
         a match — this proves the deck really was searched rather than the effect being \
         skipped"
    );
}

/// The live branch carries no cost clause at all: 「ライブカード**か** コスト10以上の
/// メンバーカード」 makes the 10 constrain only the member branch.
///
/// START:DASH!! prints score 1, so it is exactly the live that a `cost_limit: 10` applied
/// to both branches would wrongly reject. This pins that the live branch is unconstrained.
#[test]
fn reveal_until_chosen_type_the_cost_clause_does_not_constrain_the_live_branch() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let low_score_live = game.id("PL!-sd1-019-SD");

    // Pin the premise: score 1 is well under 10, so a filter spanning both branches would
    // reject this card and the test would be meaningless.
    game.assert_card_identity(low_score_live, "PL!-sd1-019-SD");
    game.assert_card_score(low_score_live, 1);

    let member = game.id("PL!SP-bp2-006-P");
    setup_honoka_deck(&mut game, honoka, vec![member, member, low_score_live]);
    activate_and_choose_type(&mut game, honoka, 0);

    assert!(
        game.state.player1.hand.cards.contains(&low_score_live),
        "ライブカード — the live branch has no コスト10以上 clause, so a live printing \
         score 1 still qualifies and the reveal must stop on it (hand={:?})",
        game.state.player1.hand.cards
    );
}

#[test]
fn reveal_until_chosen_type_use_limit_blocks_second_activation() {    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let honoka = game.id("PL!-pb1-001-R");
    let filler = game.id("PL!-sd1-019-SD");
    let target = game.id("PL!SP-bp2-006-P");
    setup_honoka_deck(&mut game, honoka, vec![target, filler, filler, filler]);
    activate_and_choose_type(&mut game, honoka, 1);
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "Target in hand after first activation"
    );
    let result = TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(honoka),
        None,
        None,
        None,
    );
    assert!(result.is_err(), "Second activation should fail (use_limit)");
}
