/// BP07 C8: PL!S-bp7-008-R 小原鞠莉 ab#0 (登場).
///
/// 登場：自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番で
/// デッキの上に置き、残りを好きな順番でデッキの下に置く。
///
/// (Debut) Look at the top 3 cards of your deck. Put any number of them (any order)
/// back on the TOP of the deck, and put the rest (any order) on the BOTTOM of the
/// deck — the rest must NOT go to the discard.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const MARI: &str = "PL!S-bp7-008-R";
const A: &str = "PL!-sd1-001-SD";
const B: &str = "PL!-sd1-003-SD";
const C: &str = "PL!-sd1-004-SD";

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

fn on_deck(game: &TestGame, id: i16) -> bool {
    game.state.player1.main_deck.cards.contains(&id)
}

fn in_waitroom(game: &TestGame, id: i16) -> bool {
    game.state.player1.waitroom.cards.contains(&id)
}

/// Deck is exactly [a, b, c] (top=a). Look at the top 3 → all. Keep the first
/// looked-at (top-most) card on the deck TOP; the other two go to the deck BOTTOM
/// (NOT the waitroom).
#[test]
fn mari_keeps_one_on_top_rest_to_deck_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let a = game.id(A);
    let b = game.id(B);
    let c = game.id(C);
    game.state.player1.main_deck.cards = vec![a, b, c].into(); // top=a

    let mari = game.id(MARI);
    trigger_debut(&mut game, mari);

    // Look-and-select is any_number: select the first looked-at card, then skip
    // the re-prompt to finalize.
    let mut guard = 0;
    let mut selected = false;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        if !selected {
            game.select_indices(&[0]);
            selected = true;
        } else {
            game.select_indices(&[]);
        }
    }

    let kept: Vec<i16> = [a, b, c].iter().copied().filter(|&id| on_deck(&game, id)).collect();
    let discarded: Vec<i16> = [a, b, c].iter().copied().filter(|&id| in_waitroom(&game, id)).collect();
    assert_eq!(kept.len(), 3, "all 3 looked-at cards stay on the deck");
    assert!(
        discarded.is_empty(),
        "remaining cards must go to the deck BOTTOM, NOT the waitroom"
    );
    // Deck has all 3 cards: the kept card on TOP, the rest on the BOTTOM.
    let deck = &game.state.player1.main_deck.cards;
    assert_eq!(deck.len(), 3, "deck keeps all 3 cards");
    let top = deck[0];
    assert!(
        [a, b, c].contains(&top),
        "the selected card is on the deck top"
    );
    let bottom: Vec<i16> = deck[1..].to_vec();
    for other in [a, b, c].iter().copied().filter(|&x| x != top) {
        assert!(bottom.contains(&other), "remaining card {} on deck bottom", other);
    }
}
