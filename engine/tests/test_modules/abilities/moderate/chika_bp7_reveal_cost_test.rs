/// 嵐 千砂都 (PL!SP-bp7-003-R＋) ab#2 — 起動:
///   手札のコストが10か20のメンバーカードを1枚公開する：これにより公開したカードを
///   このメンバーの下に置く。その後、カードを2枚引く。
///
/// (Activation) Reveal 1 member card from hand whose cost is 10 or 20: place the
/// revealed card under this member, then draw 2 cards.
///
/// The reveal choice must be FILTERED to members with cost 10 or 20 only — a
/// member with any other cost must NOT be selectable.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const CHIKA: &str = "PL!SP-bp7-003-R＋"; // 嵐 千砂都, cost 10
const COST20: &str = "PL!SP-pb2-005-R"; // 葉月 恋, cost 20
const COST10: &str = "PL!N-bp1-003-R＋"; // 桜坂しずく, cost 10
const LOW_COST: &str = "PL!-sd1-010-SD"; // 高坂穂乃果, low cost (must NOT be selectable)

fn setup(game: &mut TestGame) -> i16 {
    let chika = game.id(CHIKA);
    game.state.player1.stage.stage[1] = chika;
    game.give_energy(20);
    chika
}

fn under_center(game: &TestGame) -> Vec<i16> {
    game.state.player1.stage.get_under_cards(MemberArea::Center).to_vec()
}

/// Fire 嵐 千砂都's 起動 ab#2 and select the hand card at `hand_index` to reveal.
fn activate_and_reveal(game: &mut TestGame, chika: i16, hand_index: usize) {
    game.activate_ability(chika);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. } => {
                if zone == "hand" {
                    game.select_indices(&[hand_index]);
                } else {
                    game.select_indices(&[0]);
                }
            }
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1); // accept
            }
            _ => game.select_indices(&[0]),
        }
    }
    game.drain_auto_ability_choices();
}

/// The reveal cost filters hand cards to cost 10 or 20. A cost-20 member in hand
/// is selectable; after revealing it is placed under 嵐 千砂都 and 2 cards drawn.
#[test]
fn chika_reveal_cost20_places_under_and_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chika = setup(&mut game);
    let cost20 = game.id(COST20);
    let cost20b = game.id(COST20);
    let low = game.id(LOW_COST);
    let draw_fodder = game.id(LOW_COST);
    // Two valid cost-20 cards + one low-cost card → a choice must be offered,
    // filtered to the two cost-20 members only.
    game.state.player1.hand.cards.push(cost20);
    game.state.player1.hand.cards.push(cost20b);
    game.state.player1.hand.cards.push(low);
    // 2 cards in deck to draw
    game.state.player1.main_deck.cards.push(draw_fodder);
    game.state.player1.main_deck.cards.push(draw_fodder);
    let hand_before = game.state.player1.hand.cards.len();

    game.activate_ability(chika);
    let mut saw_hand_choice = false;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. } => {
                if zone == "hand" {
                    saw_hand_choice = true;
                    game.select_indices(&[0]); // select the first cost-20 card
                } else {
                    game.select_indices(&[0]);
                }
            }
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1);
            }
            _ => game.select_indices(&[0]),
        }
    }
    game.drain_auto_ability_choices();

    assert!(saw_hand_choice, "a hand reveal choice must be offered");
    let under = under_center(&game);
    assert_eq!(under.len(), 1, "one member placed under 嵐 千砂都");
    assert_eq!(under[0], cost20, "the revealed cost-20 member is placed under");
    // The low-cost card stays in hand; we drew 2, so net hand = before +2 -1(revealed)
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 2 - 1,
        "reveal 1 from hand, draw 2"
    );
}

/// A low-cost member in hand must NOT be selectable for the cost-10-or-20 reveal.
/// With only a low-cost member in hand, no valid choice is offered and the
/// ability cannot proceed (nothing placed, no draw).
#[test]
fn chika_low_cost_hand_not_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chika = setup(&mut game);
    let low = game.id(LOW_COST);
    game.state.player1.hand.cards.push(low);
    let draw_fodder = game.id(LOW_COST);
    game.state.player1.main_deck.cards.push(draw_fodder);
    game.state.player1.main_deck.cards.push(draw_fodder);
    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();

    game.activate_ability(chika);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        game.select_indices(&[0]);
    }
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "no reveal → no draw"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no reveal → hand unchanged"
    );
    assert_eq!(under_center(&game).len(), 0, "nothing placed under 嵐 千砂都");
}

/// A cost-10 member in hand is selectable for the reveal and placed under.
#[test]
fn chika_reveal_cost10_places_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chika = setup(&mut game);
    let cost10 = game.id(COST10);
    let draw_fodder = game.id(LOW_COST);
    game.state.player1.hand.cards.push(cost10);
    game.state.player1.main_deck.cards.push(draw_fodder);
    game.state.player1.main_deck.cards.push(draw_fodder);

    activate_and_reveal(&mut game, chika, 0);

    let under = under_center(&game);
    assert_eq!(under.len(), 1, "one member placed under 嵐 千砂都");
    assert_eq!(under[0], cost10, "the revealed cost-10 member is placed under");
}
