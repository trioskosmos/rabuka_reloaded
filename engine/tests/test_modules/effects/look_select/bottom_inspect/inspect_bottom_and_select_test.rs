/// BP07 C7: PL!S-bp7-004-R 黒澤ダイヤ ab#1 (ライブ開始時).
///
/// ライブ開始時：自分のデッキの下からカードを3枚見る。その中から好きな枚数を
/// 好きな順番でデッキの下に置き、残りを控え室に置く。
///
/// (Live start) Look at the bottom 3 cards of your deck. Put any number of them
/// (in any order) back on the BOTTOM of the deck, and put the rest in the waitroom.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const DIA: &str = "PL!S-bp7-004-R";
const A: &str = "PL!-sd1-001-SD"; // distinct cards to identify the bottom 3
const B: &str = "PL!-sd1-003-SD";
const C: &str = "PL!-sd1-004-SD";

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
    game.drain_auto_ability_choices();
}

fn in_waitroom(game: &TestGame, id: i16) -> bool {
    game.state.player1.waitroom.cards.contains(&id)
}

fn on_deck(game: &TestGame, id: i16) -> bool {
    game.state.player1.main_deck.cards.contains(&id)
}

/// Deck is exactly [a, b, c] (top=a, bottom=c). Look at the bottom 3 → all three.
/// Keep the FIRST looked-at (bottom-most) card on the deck bottom; the other two
/// go to the waitroom.
#[test]
fn dia_keeps_one_on_bottom_rest_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let a = game.id(A);
    let b = game.id(B);
    let c = game.id(C);
    game.state.player1.main_deck.cards = vec![a, b, c].into(); // top=a, bottom=c

    let dia = game.id(DIA);
    trigger_live_start(&mut game, dia);

    // Look-and-select is any_number: select the first looked-at card, then skip
    // the re-prompt to finalize (selected → deck bottom, rest → waitroom).
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

    // Exactly one of a/b/c stays on the deck (the kept card); the other two are
    // in the waitroom.
    let kept: Vec<i16> = [a, b, c].iter().copied().filter(|&id| on_deck(&game, id)).collect();
    let discarded: Vec<i16> = [a, b, c].iter().copied().filter(|&id| in_waitroom(&game, id)).collect();
    assert_eq!(kept.len(), 1, "exactly 1 looked-at card should be kept on deck");
    assert_eq!(discarded.len(), 2, "the other 2 looked-at cards go to the waitroom");
    // The kept card is on the deck BOTTOM.
    let deck = &game.state.player1.main_deck.cards;
    assert_eq!(deck[deck.len() - 1], kept[0], "kept card is on the deck bottom");
}
