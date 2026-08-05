/// BP07 parser/engine fix C6: `PL!S-bp7-004-R` / `PL!S-bp7-004-P` 黒澤ダイヤ ab#0 (登場).
///
/// 登場：『Aqours』のメンバーからバトンタッチして登場した場合、自分と相手はそれぞれ、
/// 自身の手札のカードを3枚まで選び、選んだカード以外のカードをシャッフルし、自身の
/// デッキの下に置く。その後、自分と相手はそれぞれカードを3枚引く。
///
/// "Debut: when you appear via baton touch from an Aqours member, you and your
/// opponent each select up to 3 cards from your own hands, shuffle the cards
/// OTHER than the selected ones and put them under your own deck. Then you and
/// your opponent each draw 3 cards."
///
/// The defect (C6): the shuffle step targeted energy_deck instead of both
/// players, and the "選んだカード以外" (cards other than the selected N) —
/// keep up to 3, shuffle the rest under your own deck — was lost.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

fn filler(game: &mut TestGame) -> i16 {
    game.id(FILLER)
}

/// Give each player a hand of `n` filler cards.
fn fill_hands(game: &mut TestGame, n: usize) {
    let f = filler(game);
    for _ in 0..n {
        game.state.player1.hand.cards.push(f);
        game.state.player2.hand.cards.push(f);
    }
}

/// Seed each player's deck with filler cards so the draws have something.
fn seed_decks(game: &mut TestGame) {
    let f = filler(game);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// Place an Aqours member on P1's center, then baton-touch 黒澤ダイヤ onto it.
/// Returns Dia's id.
fn baton_touch_dia(game: &mut TestGame) -> i16 {
    game.give_energy(10);
    let chika = game.id("PL!S-bp2-001-R"); // 高海千歌 (Aqours)
    game.add_to_stage(MemberArea::Center, chika);
    let dia = game.id("PL!S-bp7-004-P");
    game.state.player1.hand.cards.push(dia);
    game.play_to_stage(dia, MemberArea::Center);
    dia
}

/// Count cards under the deck bottom region (the last N cards of main_deck).
fn deck_bottom_region(game: &mut TestGame, player: usize) -> Vec<i16> {
    let deck = if player == 1 {
        &game.state.player1.main_deck.cards
    } else {
        &game.state.player2.main_deck.cards
    };
    // The cards moved to "deck bottom" are the ones placed at the end.
    deck.to_vec()
}

/// Basic flow: 4 cards in each hand → each selects 3 to keep → 1 shuffled under
/// own deck → both draw 3.
#[test]
fn c6_keep_3_shuffle_rest_under_then_draw_3() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    seed_decks(&mut game);
    fill_hands(&mut game, 4);
    let p1_hand_before = game.state.player1.hand.cards.len();
    let p2_hand_before = game.state.player2.hand.cards.len();

    baton_touch_dia(&mut game);

    // Drain: P1 selects up to 3 (keep 3), P2 selects up to 3 (keep 3).
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        if game.pending_choice_type().as_deref() == Some("SelectCard") {
            game.select_indices(&[0, 1, 2]);
        } else {
            game.select_indices(&[]);
        }
    }

    // Both players kept 3 (of 4) and then drew 3 → hand grew by (3 drawn - 1 sent under) = +2.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        p1_hand_before + 2,
        "P1: 1 card sent under deck, 3 drawn"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before + 2,
        "P2: 1 card sent under deck, 3 drawn"
    );
}

/// Hand below the max (2 cards): all kept, nothing shuffled under, both draw 3.
#[test]
fn c6_hand_below_max_nothing_shuffled_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    seed_decks(&mut game);
    fill_hands(&mut game, 2);
    let p1_hand_before = game.state.player1.hand.cards.len();
    let p2_hand_before = game.state.player2.hand.cards.len();

    baton_touch_dia(&mut game);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        if game.pending_choice_type().as_deref() == Some("SelectCard") {
            game.select_indices(&[0, 1]);
        } else {
            game.select_indices(&[]);
        }
    }

    // 2 kept + 3 drawn = 5 (nothing sent under).
    assert_eq!(
        game.state.player1.hand.cards.len(),
        p1_hand_before + 3,
        "P1: nothing sent under, drew 3"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before + 3,
        "P2: nothing sent under, drew 3"
    );
}

/// Both players are processed independently: P1 has 5 hand cards (keep 3 → 2
/// under), P2 has 2 (keep 2 → 0 under).
#[test]
fn c6_hands_processed_independently() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    seed_decks(&mut game);
    let f = filler(&mut game);
    for _ in 0..5 {
        game.state.player1.hand.cards.push(f);
    }
    for _ in 0..2 {
        game.state.player2.hand.cards.push(f);
    }
    let p1_hand_before = game.state.player1.hand.cards.len();
    let p2_hand_before = game.state.player2.hand.cards.len();

    baton_touch_dia(&mut game);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        if game.pending_choice_type().as_deref() == Some("SelectCard") {
            game.select_indices(&[0, 1, 2]);
        } else {
            game.select_indices(&[]);
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        p1_hand_before + 1,
        "P1: 2 sent under, 3 drawn"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before + 3,
        "P2: none sent under, 3 drawn"
    );
}
