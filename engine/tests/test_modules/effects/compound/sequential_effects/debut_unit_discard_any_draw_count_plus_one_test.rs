/// 大沢瑠璃乃 PL!HS-pb1-003-R ab#0 (登場):
///
/// 手札の『みらくらぱーく！』のメンバーカードを好きな枚数控え室に置き、
/// その後、その枚数に1を足した枚数のカードを引く。
///
/// (Debut: put any number of your 『みらくらぱーく！』 member cards from hand into
/// the waitroom, then draw that number plus one.)
///
/// Both tests this replaces stepped through the prompts by ORDINAL — `if step
/// == 3 { select [0,1] } else if step == 4 { skip }` — so adding or removing one
/// prompt anywhere in the chain silently made them answer the wrong thing. The
/// drain below answers each prompt by its own identity instead, and each test now
/// checks every clause of the printed sentence rather than only the final hand
/// count (which a wrong discard/draw pair could still reproduce).
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

const RURINO: &str = "PL!HS-pb1-003-R"; // 大沢瑠璃乃 — the 登場 source
const MIRAKU: &str = "PL!HS-sd1-011-SD"; // a 『みらくらぱーく！』 member (cost 4)

/// Rurino in hand, `miraku_copies` 『みらくらぱーく！』 members in hand, filled
/// decks, and enough energy to play her. Returns the hand ids in order.
fn setup(miraku_copies: usize) -> (TestGame, i16, Vec<i16>) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rurino = game.id(RURINO);
    game.assert_card_identity(rurino, RURINO);
    game.assert_card_cost(rurino, 15);
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(rurino);
    let mut miraku = Vec::new();
    for _ in 0..miraku_copies {
        let m = game.id(MIRAKU);
        game.assert_card_identity(m, MIRAKU);
        miraku.push(m);
        game.add_to_hand(m);
    }
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(20);
    (game, rurino, miraku)
}

/// Answer the debut's prompts by identity. The engine asks ONE hand
/// SelectCard with the card count to take, then may ask "more?" — so this
/// answers the first with `discard` indices and declines anything after it.
/// Driven by the prompt's own shape, not by how many prompts came before.
fn resolve_debut(game: &mut TestGame, discard: usize) {
    let mut hand_prompts = 0usize;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectCard { zone, count, .. } if zone == "hand" => {
                hand_prompts += 1;
                if hand_prompts == 1 {
                    assert!(
                        *count >= discard || *count == 0,
                        "the discard prompt offers {count} card(s), cannot take {discard}"
                    );
                    let picks: Vec<usize> = (0..discard).collect();
                    game.select_indices(&picks);
                } else {
                    // 好きな枚数 — zero further cards, which is a legal answer.
                    game.select_indices(&[]);
                }
            }
            other => {
                if hand_prompts == 0 {
                    panic!(
                        "expected a hand SelectCard for the discard, got {other:?} \
                         (step {guard})"
                    );
                }
                game.select_indices(&[]);
            }
        }
    }
    assert!(
        !game.has_pending_choice(),
        "the 登場 and its prompts must all resolve"
    );
    assert!(
        hand_prompts >= 1,
        "the 登場 must have offered its 手札の…枚数 select"
    );
}

/// 2 in the waitroom → draw 3. Every clause is checked by card identity, not by
/// a hand total that a compensating error could still satisfy.
#[test]
fn debut_unit_discard_two_draw_three_moves_the_exact_cards() {
    let (mut game, rurino, miraku) = setup(3);
    let deck_before = game.state.player1.main_deck.cards.len();
    assert_eq!(miraku.len(), 3, "setup: three candidates to discard from");

    game.play_to_stage(rurino, MemberArea::Center);
    resolve_debut(&mut game, 2);

    // 手札の…メンバーカードを好きな枚数控え室に置く — exactly 2, and they must be
    // two of the three candidates, gone from hand.
    let waited: Vec<i16> = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .copied()
        .filter(|c| miraku.contains(c))
        .collect();
    assert_eq!(
        waited.len(),
        2,
        "好きな枚数 = 2 chosen: exactly two of the candidates reach the waitroom"
    );
    for c in &waited {
        assert!(
            !game.state.player1.hand.cards.contains(c),
            "a discarded card must not still be in hand"
        );
    }

    // その後、その枚数に1を足した枚数のカードを引く — 2 + 1 = 3 drawn.
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "3 cards must have left the deck (2 + 1)"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        4,
        "started with 4 (Rurino + 3 candidates): play -1, discard -2, draw +3 = 4"
    );
    let hand = game.state.player1.hand.cards.clone();
    let survivor = miraku
        .iter()
        .copied()
        .find(|c| hand.contains(c))
        .expect("the one un-discarded candidate is still in hand");
    assert_eq!(
        hand.iter()
            .filter(|c| miraku.contains(c))
            .count(),
        1,
        "exactly one candidate is left in hand"
    );
    let drawn = hand.iter().filter(|c| !miraku.contains(c)).count();
    assert_eq!(drawn, 3, "the 3 drawn cards are in hand alongside the survivor");
    let _ = survivor;
}

/// 0 in the waitroom → draw 1. Zero is a legal 好きな枚数, and the printed
/// "+1" still applies.
#[test]
fn debut_unit_discard_zero_draw_one() {
    let (mut game, rurino, miraku) = setup(1);
    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    game.play_to_stage(rurino, MemberArea::Center);
    resolve_debut(&mut game, 0);

    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before,
        "0 chosen → nothing reaches the waitroom"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&miraku[0]),
        "the candidate must stay out of the waitroom when 0 is chosen"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "0 + 1 = exactly one card drawn"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "started with 2 (Rurino + 1 candidate): play -1, discard 0, draw +1 = 2"
    );
    assert!(
        game.state.player1.hand.cards.contains(&miraku[0]),
        "the candidate is still in hand"
    );
}
