/// Tests for PL!-pb1-001-R (高坂穂乃果) ab#0 — Q166, Q167
///
/// Ability (起動[Center][ターン1]):
///   このメンバーをウェイトにし、手札1枚を控え室に置く：
///   ライブカードかコスト10以上のメンバーカードのどちらか1つを選ぶ。
///   選んだカードが公開されるまで、デッキの上から1枚ずつ公開する。
///   そのカードを手札に加え、他をすべて控え室に置く。
///
/// The reveal_until_chosen_card action handles the type choice,
/// reveal loop, add-to-hand, and discard internally.

mod helpers;
use helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

fn activate_and_choose_member(game: &mut TestGame, honoka: i16) {
    TurnEngine::execute_main_phase_action(
        &mut game.state, &ActionType::UseAbility,
        Some(honoka), None, None, None,
    ).expect("activate");
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_option(1);
    }
}

fn activate_and_choose_live(game: &mut TestGame, honoka: i16) {
    TurnEngine::execute_main_phase_action(
        &mut game.state, &ActionType::UseAbility,
        Some(honoka), None, None, None,
    ).expect("activate");
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    if game.has_pending_choice() {
        game.select_option(0);
    }
}

/// Live card ID (not a member, member_card filter skips it)
const LIVE_FILLER: &str = "PL!-sd1-019-SD";
/// Member card ID (not a live, live_card filter skips it)
const MEMBER_FILLER: &str = "PL!-sd1-010-SD";

/// Choose member_card type. Fillers are live cards (non-members).
/// Target (cost-10 member) found after 4 fillers, added to hand.
#[test]
fn honoka_q166_member_found_after_4_fillers() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-pb1-001-R");
    let filler = game.id("PL!-sd1-019-SD");
    let target = game.id("PL!SP-bp2-006-P");

    game.state.player1.stage.stage[1] = honoka;
    let member = game.id(MEMBER_FILLER);
    game.state.player1.hand.cards.push(member);
    game.state.player1.hand.cards.push(member);
    for _ in 0..4 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.main_deck.cards.push(target);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }

    game.give_energy(13);
    activate_and_choose_member(&mut game, honoka);

    assert!(game.state.player1.hand.cards.contains(&target),
        "Target should be in hand");
    assert!(!game.state.player1.main_deck.cards.contains(&target),
        "Target should not remain in deck");
}

/// Target is first card in deck. Reveal stops immediately.
#[test]
fn honoka_q166_target_first_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-pb1-001-R");
    let filler = game.id("PL!-sd1-019-SD");
    let target = game.id("PL!SP-bp2-006-P");

    game.state.player1.stage.stage[1] = honoka;
    let member = game.id(MEMBER_FILLER);
    game.state.player1.hand.cards.push(member);
    game.state.player1.hand.cards.push(member);
    game.state.player1.main_deck.cards.push(target);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }

    game.give_energy(13);
    activate_and_choose_member(&mut game, honoka);

    assert!(game.state.player1.hand.cards.contains(&target),
        "Target (first card) should be in hand");
    assert_eq!(game.state.player1.main_deck.cards.len(), 5,
        "Only 1 card revealed, 5 remain");
}

/// Target is LAST card in deck.
#[test]
fn honoka_q166_target_last_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-pb1-001-R");
    let filler = game.id("PL!-sd1-019-SD");
    let target = game.id("PL!SP-bp2-006-P");

    game.state.player1.stage.stage[1] = honoka;
    let member = game.id(MEMBER_FILLER);
    game.state.player1.hand.cards.push(member);
    game.state.player1.hand.cards.push(member);
    for _ in 0..9 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.main_deck.cards.push(target);

    game.give_energy(13);
    activate_and_choose_member(&mut game, honoka);

    assert!(game.state.player1.hand.cards.contains(&target),
        "Target (last card) should be in hand");
    assert!(game.state.player1.main_deck.cards.is_empty(),
        "All cards revealed, deck empty");
}

/// Choose LIVE card type. Fillers are member cards (non-live).
#[test]
fn honoka_q166_live_card_chosen() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-pb1-001-R");
    let filler = game.id("PL!-sd1-010-SD");
    let target = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[1] = honoka;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..3 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.main_deck.cards.push(target);
    for _ in 0..3 { game.state.player1.main_deck.cards.push(filler); }

    game.give_energy(13);
    activate_and_choose_live(&mut game, honoka);

    assert!(game.state.player1.hand.cards.contains(&target),
        "Live card should be found and added to hand");
}

/// Two matching member cards at front of deck. Choose member_card type.
/// Only 1 should be added (reveal stops at first match).
#[test]
fn honoka_q166_two_matches_only_one_added() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-pb1-001-R");
    let filler = game.id("PL!-sd1-019-SD");
    let target1 = game.id("PL!SP-bp2-006-P");
    let target2 = game.id("PL!HS-bp2-005-P");

    game.state.player1.stage.stage[1] = honoka;
    let member = game.id(MEMBER_FILLER);
    game.state.player1.hand.cards.push(member);
    game.state.player1.hand.cards.push(member);
    game.state.player1.main_deck.cards.push(target1);
    game.state.player1.main_deck.cards.push(target2);
    for _ in 0..3 { game.state.player1.main_deck.cards.push(filler); }

    game.give_energy(13);
    activate_and_choose_member(&mut game, honoka);

    let count = game.state.player1.hand.cards.iter()
        .filter(|&&id| id == target1 || id == target2).count();
    assert_eq!(count, 1,
        "Only 1 of 2 matching cards added (reveal stops at first match)");
    assert!(game.state.player1.main_deck.cards.contains(&target2)
        || game.state.player1.waitroom.cards.contains(&target2),
        "Second matching card should be in deck or discard (not 'lost')");
}

/// Deck has only live cards (no cost>=10 member). Choose member_card.
/// Cost discards a non-member card so the refresh doesn't find a match.
#[test]
fn honoka_q166_no_member_in_deck_refresh() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-pb1-001-R");
    let filler = game.id("PL!-sd1-019-SD");
    let live_in_hand = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[1] = honoka;
    game.state.player1.hand.cards.push(live_in_hand);
    game.state.player1.hand.cards.push(live_in_hand);
    for _ in 0..8 { game.state.player1.main_deck.cards.push(filler); }

    game.give_energy(13);
    activate_and_choose_member(&mut game, honoka);

    assert_eq!(game.state.player1.hand.cards.len(), 1,
        "Only the non-discarded filler remains");
}

/// Deck exhausted mid-reveal.
#[test]
fn honoka_q167_deck_exhausted_during_reveal() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let honoka = game.id("PL!-pb1-001-R");
    let filler = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[1] = honoka;
    let member = game.id(MEMBER_FILLER);
    game.state.player1.hand.cards.push(member);
    game.state.player1.hand.cards.push(member);
    for _ in 0..2 { game.state.player1.main_deck.cards.push(filler); }

    game.give_energy(13);
    activate_and_choose_member(&mut game, honoka);

    assert!(game.state.player1.hand.cards.len() <= 2,
        "Hand should not have extra cards added after refresh with no match");
}


