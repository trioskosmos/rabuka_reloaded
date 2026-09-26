use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const NIJI_CHOICE: &str = "PL!N-pb1-010-R";
const NIJI_LIVE: &str = "PL!N-bp1-026-L";
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

/// 三船栞子 PL!N-pb1-010-R 登場, option 0: エネルギーを1枚アクティブにする。
///
/// Was `assert!(active >= 1)` — but `give_energy(10)` had already activated ten,
/// so it held before the 登場 ever resolved. The card's PLAY cost is also 10, so
/// measuring against a pre-`play_to_stage` baseline nets the play against the
/// ability (observed: -9). Stage her and fire the 登場 directly, so the only
/// thing that can move the counter is the printed effect.
#[test]
fn pl_n_pb1_010_r_energy_option_activates_exactly_one_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(NIJI_CHOICE);
    game.assert_card_identity(member, NIJI_CHOICE);
    game.assert_card_cost(member, 10);
    // One card left in the zone deliberately INACTIVE, so there is exactly one
    // thing the effect can activate and no room for a double count to hide in.
    let e1 = game.id("LL-E-001-SD");
    for _ in 0..9 {
        game.state.player1.energy_zone.cards.push(e1);
        game.state.player1.energy_zone.add_active(1);
    }
    game.state.player1.energy_zone.cards.push(e1);
    let active_before = game.state.player1.energy_zone.active_count();
    assert_eq!(
        active_before, 9,
        "setup guard: 9 of the 10 zone cards are active"
    );
    let zone_len = game.state.player1.energy_zone.cards.len();

    game.state.player1.stage.stage[1] = member;
    fire_trigger(
        &mut game,
        member,
        rabuka_engine::core::types::AbilityTrigger::Debut,
        "登場",
    );
    while game.has_pending_choice() {
        game.select_choice_option(0);
    }

    assert_eq!(
        game.state.player1.energy_zone.active_count() - active_before,
        1,
        "エネルギーを1枚アクティブにする — exactly one more card is active"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_len,
        "no energy card was added: the effect activates, it does not supply"
    );
}

/// Option 1: 自分の控え室にある『虹ヶ咲』のライブカードを2枚まで好きな順番で
/// デッキの上に置く.
///
/// Was `deck_after > deck_before` — true for any number of cards, and blind to
/// WHICH cards and in what order, which is the whole of the printed effect.
///
/// Note the answer path: the prompt is answered by index, and this run takes a
/// single card, which is a legal 「2枚まで」 answer. So this pins that a chosen
/// live really lands on the deck TOP and leaves the waitroom — not that two do.
#[test]
fn pl_n_pb1_010_r_recycle_live_option_puts_a_chosen_live_on_the_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(NIJI_CHOICE);
    let live1 = game.id(NIJI_LIVE);
    let live2 = game.id(NIJI_LIVE);
    game.assert_card_identity(member, NIJI_CHOICE);
    game.assert_card_identity(live1, NIJI_LIVE);
    game.assert_same_card_name(live1, live2, "two copies of the same 虹ヶ咲 live");
    assert_ne!(live1, live2, "two separate card instances");
    game.state.player1.waitroom.cards.push(live1);
    game.state.player1.waitroom.cards.push(live2);
    fill_deck(&mut game, "p1", 5);
    game.give_energy(10);
    game.add_to_hand(member);
    game.play_to_stage(member, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_choice_option(1);
    }

    // 好きな順番でデッキの上に置く — a chosen live must be the deck's TOP card,
    // not merely somewhere in it, and it must have left the waitroom.
    let deck = &game.state.player1.main_deck.cards;
    let placed = deck.iter().filter(|c| **c == live1 || **c == live2).count();
    assert!(
        placed >= 1,
        "a chosen 虹ヶ咲 live must reach the deck, got {placed} \
         (deck {:?}, waitroom {:?})",
        &deck[..deck.len().min(4)],
        game.state.player1.waitroom.cards
    );
    assert!(
        deck[0] == live1 || deck[0] == live2,
        "デッキの上に置く — the live must be the deck's TOP card, top = {:?}",
        deck[0]
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "one of the two lives left the waitroom (the other is the 「2枚まで」 \
         upper bound this answer path does not take)"
    );
}
