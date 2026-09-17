/// Tests for PL!-bp5-003-R＋ 南 ことり (Minami Kotori):
///
/// Ability #0 (常時):
///   As long as 3+ members with distinct names on stage → gain heart03.
///
/// Ability #1 (起動, ターン1回):
///   Pay {E}{E} + discard 1 card from hand:
///   - If discarded card is μ's → look at top 4 deck, pick 2 to hand, rest to discard
///   - If discarded card is NOT μ's → add 1 live card from discard to hand
use crate::helpers::*;

const KOTORI: &str = "PL!-bp5-003-R\u{ff0b}";
const FILLER: &str = "PL!-sd1-010-SD";

fn fill_deck(game: &mut TestGame, player: &str, count: usize) {
    let ids: Vec<i16> = (0..count).map(|_| game.id(FILLER)).collect();
    let deck = if player == "p1" {
        &mut game.state.player1.main_deck.cards
    } else {
        &mut game.state.player2.main_deck.cards
    };
    for f in ids {
        deck.push(f);
    }
}

// ============================================================
// Ability #0: Constant heart03 with 3+ distinct names
// ============================================================

/// ab#0: 3 distinct names on stage → heart03 gained
#[test]
fn kotori_ab0_three_distinct_names() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    let filler_a = game.id(FILLER);
    let filler_b = game.id("PL!-sd1-014-SD"); // different card

    game.state.player1.stage.stage = [kotori, filler_a, filler_b];
    game.state.recalculate_constants();

    let heart03 = game.state.mods.get_heart_modifier(
        kotori,
        rabuka_engine::core::card::HeartColor::Heart03,
    );
    assert!(
        heart03 > 0,
        "ab#0 should grant heart03 with 3 distinct names on stage"
    );
}

/// ab#0: only 2 distinct names → no heart03
#[test]
fn kotori_ab0_two_names_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    let filler = game.id(FILLER);

    game.state.player1.stage.stage = [kotori, filler, -1];
    game.state.recalculate_constants();

    let heart03 = game.state.mods.get_heart_modifier(
        kotori,
        rabuka_engine::core::card::HeartColor::Heart03,
    );
    assert_eq!(
        heart03, 0,
        "ab#0 should NOT grant heart03 with only 2 distinct names"
    );
}

// ============================================================
// Ability #1: conditional_alternative based on discarded card
// ============================================================

/// ab#1: discard μ's card → look 4, pick 2, rest to discard
#[test]
fn kotori_ab1_discard_mus_look_and_select() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    let mus_card = game.id("PL!-bp6-001-R\u{ff0b}"); // 高坂穂乃果, μ's card

    game.state.player1.stage.stage = [kotori, -1, -1];
    game.state.player1.hand.cards.push(mus_card);

    fill_deck(&mut game, "p1", 10);
    game.give_energy(10);

    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();
    let discard_before = game.state.player1.waitroom.cards.len();

    game.activate_ability(kotori);

    // Resolve all choices
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        let choice = game.get_pending_choice();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectCard { count, zone, .. } if *count > 1 && zone == "revealed_cards" => {
                let indices: Vec<usize> = (0..*count as usize).collect();
                game.select_indices(&indices);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    let hand_after = game.state.player1.hand.cards.len();
    let deck_after = game.state.player1.main_deck.cards.len();
    let discard_after = game.state.player1.waitroom.cards.len();
    eprintln!("[KOTORI] hand: {}->{}, deck: {}->{}, discard: {}->{}",
        hand_before, hand_after, deck_before, deck_after, discard_before, discard_after);
    assert!(
        hand_after > hand_before,
        "Should have gained cards from look_and_select: before={}, after={}",
        hand_before,
        hand_after
    );
}

/// ab#1: discard non-μ's card → add 1 live card from discard to hand
#[test]
fn kotori_ab1_discard_non_mus_recover_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    // Use a 虹ヶ咲 card as the non-μ's discard — 虹ヶ咲 is NOT μ's
    let non_mus = game.id("PL!N-bp1-012-R\u{ff0b}"); // 鐘嵐珠, 虹ヶ咲 (not μ's)
    let live_in_discard = game.id("PL!-sd1-019-SD"); // a live card

    game.state.player1.stage.stage = [kotori, -1, -1];
    game.state.player1.hand.cards.push(non_mus);
    game.state.player1.waitroom.cards.push(live_in_discard);

    fill_deck(&mut game, "p1", 10);
    game.give_energy(10);

    game.activate_ability(kotori);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        let choice = game.get_pending_choice();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectCard { count, zone, .. } if *count > 1 && zone == "revealed_cards" => {
                let indices: Vec<usize> = (0..*count as usize).collect();
                game.select_indices(&indices);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    assert!(
        game.state.player1.hand.cards.contains(&live_in_discard),
        "Should have recovered live card from discard to hand"
    );
}
