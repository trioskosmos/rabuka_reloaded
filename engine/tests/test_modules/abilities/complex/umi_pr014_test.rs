/// Tests for PL!-PR-014-PR (園田海未 / Umi Sonoda)
///
/// Ab#0 (登場): 自分の手札から、相手は見ないで3枚選び公開する。
///   公開した3枚にライブカードが1枚もない場合、カードを1枚引く。
///
/// Parsed:
///   trigger: appear
///   primary_effect: reveal(hand, count=3, target=opponent, blind=true, picker=self)
///   result_condition: no live_card in revealed 3
///   followup: draw_card(count=1)
///
/// Q176 contrast: This card lets YOU pick from OPPONENT's hand blind.
///               Q176's card (pb1-013) lets opponent pick from YOUR hand blind.
use crate::helpers::*;

// ====================================================================
//  Basic appear trigger: you pick from opponent's hand
// ====================================================================

#[test]
fn umi_pr014_appear_creates_blind_reveal_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-PR-014-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(3);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(umi);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Opponent has 3+ cards (need choice when >3, but trigger still fires at 3)
    game.state.player2.hand.cards.clear();
    for _ in 0..5 {
        game.state.player2.hand.cards.push(filler);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(umi, rabuka_engine::zones::MemberArea::LeftSide);

    assert!(
        game.has_pending_choice(),
        "Appear should create a reveal choice"
    );

    let choice = game.state.get_pending_choice().unwrap();
    match choice {
        rabuka_engine::ability::types::Choice::SelectCard {
            count,
            blind,
            is_reveal,
            zone,
            target_player_id,
            ..
        } => {
            assert_eq!(*count, 3, "Should pick 3 cards");
            assert!(
                *blind,
                "Should be blind — you cannot see opponent's card identities"
            );
            assert!(*is_reveal, "Should be a reveal action");
            assert_eq!(zone, "hand", "Zone should be opponent's hand");
            assert_eq!(
                target_player_id.as_deref(),
                Some("opponent"),
                "target_player_id should be 'opponent' — revealing OPPONENT's hand"
            );
        }
        _ => panic!("Expected SelectCard, got {:?}", choice),
    }
}

// ====================================================================
//  No live card among 3 revealed → draw 1
// ====================================================================

#[test]
fn umi_pr014_no_live_card_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-PR-014-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(3);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(umi);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // 3 non-live cards in opponent hand → force auto-select
    game.state.player2.hand.cards.clear();
    for _ in 0..3 {
        game.state.player2.hand.cards.push(filler);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(umi, rabuka_engine::zones::MemberArea::LeftSide);

    assert_eq!(game.state.player1.hand.len(), 1, "Should draw 1 card");
    assert_eq!(
        game.state.revealed_cards.len(),
        3,
        "3 cards should be revealed"
    );
    assert!(!game.has_pending_choice(), "Should have no pending choice");
}

// ====================================================================
//  Live card present among 3 revealed → no draw
// ====================================================================

#[test]
fn umi_pr014_live_card_present_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-PR-014-PR");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD"); // live card

    game.give_energy(3);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(umi);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // 3 cards including a live card
    game.state.player2.hand.cards.clear();
    game.state.player2.hand.cards.push(filler);
    game.state.player2.hand.cards.push(live_card);
    game.state.player2.hand.cards.push(filler);

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(umi, rabuka_engine::zones::MemberArea::LeftSide);

    assert_eq!(
        game.state.player1.hand.len(),
        0,
        "Should NOT draw when live card present"
    );
    assert!(
        game.state.revealed_cards.contains(&live_card),
        "Live card should be revealed"
    );
}

// ====================================================================
//  Sequential selection preserves blind + target
// ====================================================================

#[test]
fn umi_pr014_sequential_reprompt_preserves_blind_and_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-PR-014-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(3);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(umi);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // 5 cards → forces choice (count=3, available=5)
    game.state.player2.hand.cards.clear();
    for _ in 0..5 {
        game.state.player2.hand.cards.push(filler);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(umi, rabuka_engine::zones::MemberArea::LeftSide);

    // First pick
    game.select_indices(&[0]);
    assert!(
        game.has_pending_choice(),
        "Should re-prompt after first pick"
    );

    let rp1 = game.state.get_pending_choice().unwrap();
    match rp1 {
        rabuka_engine::ability::types::Choice::SelectCard {
            count,
            blind,
            is_reveal,
            target_player_id,
            ..
        } => {
            assert_eq!(*count, 2, "Re-prompt should ask for 2 more");
            assert!(*blind, "Re-prompt should preserve blind=true");
            assert!(*is_reveal, "Re-prompt should preserve is_reveal=true");
            assert_eq!(
                target_player_id.as_deref(),
                Some("opponent"),
                "Re-prompt should preserve target=opponent"
            );
        }
        _ => panic!("Expected SelectCard re-prompt"),
    }

    // Second pick
    game.select_indices(&[1]);
    assert!(game.has_pending_choice(), "Should re-prompt for last card");

    // Third pick
    game.select_indices(&[2]);
    assert!(!game.has_pending_choice(), "Should resolve after 3 picks");
}

// ====================================================================
//  Opponent's hand is not consumed by reveal
// ====================================================================

#[test]
fn umi_pr014_opponent_hand_not_consumed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-PR-014-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(3);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(umi);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player2.hand.cards.clear();
    for _ in 0..3 {
        game.state.player2.hand.cards.push(filler);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(umi, rabuka_engine::zones::MemberArea::LeftSide);

    assert_eq!(
        game.state.player2.hand.len(),
        3,
        "Opponent hand should not be consumed by reveal"
    );
}

// ====================================================================
//  You control the pick (picker=self): assert action ownership
// ====================================================================

#[test]
fn umi_pr014_you_control_the_pick() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id("PL!-PR-014-PR");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(3);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(umi);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player2.hand.cards.clear();
    for _ in 0..5 {
        game.state.player2.hand.cards.push(filler);
    }

    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(umi, rabuka_engine::zones::MemberArea::LeftSide);

    // P1 (you) selects from P2 (opponent) hand — YOU have control
    game.select_indices(&[0]);
    game.select_indices(&[1]);
    game.select_indices(&[2]);

    // We chose indices 0, 1, 2 from opponent's hand — those cards should be revealed
    assert_eq!(
        game.state.revealed_cards.len(),
        3,
        "3 cards revealed by P1's choice"
    );
    assert!(
        !game.has_pending_choice(),
        "Selection should be complete — YOU chose all 3"
    );
}
