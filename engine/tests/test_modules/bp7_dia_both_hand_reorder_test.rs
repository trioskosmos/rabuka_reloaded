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

/// Gate check: the ability only fires when baton-touching FROM an 『Aqours』 member.
/// Baton-touching over a NON-Aqours member (μ's 高坂穂乃果) must NOT trigger the
/// keep-3-shuffle-rest / draw-3.
#[test]
fn c6_non_aqours_baton_touch_does_not_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    seed_decks(&mut game);
    fill_hands(&mut game, 4);
    let p1_hand_before = game.state.player1.hand.cards.len();
    let p2_hand_before = game.state.player2.hand.cards.len();
    let p1_deck_before = game.state.player1.main_deck.cards.len();

    // 高坂穂乃果 (μ's, NOT Aqours) occupies center; Dia baton-touches over her.
    game.give_energy(10);
    let honoka = game.id("PL!-sd1-010-SD");
    game.add_to_stage(MemberArea::Center, honoka);
    let dia = game.id("PL!S-bp7-004-P");
    game.state.player1.hand.cards.push(dia);
    game.play_to_stage(dia, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // No draw, no hand reorder: hands and deck are unchanged.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        p1_hand_before,
        "P1 hand must be unchanged when baton-touch source is not Aqours"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before,
        "P2 hand must be unchanged when baton-touch source is not Aqours"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        p1_deck_before,
        "P1 deck must be unchanged when baton-touch source is not Aqours"
    );
}

/// Drive both players through the keep-N-shuffle-rest: each player keeps the
/// given hand indices on their FIRST hand choice; every subsequent hand choice
/// (a "select more" re-prompt) is skipped so the player keeps exactly those.
fn both_players_keep(game: &mut TestGame, keep_indices: &[usize]) {
    let mut self_done = false;
    let mut opp_done = false;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectCard {
                target_player_id, ..
            } => {
                let tp = target_player_id.as_deref().unwrap_or("self");
                if tp == "self" && !self_done {
                    game.select_indices(keep_indices);
                    self_done = true;
                } else if tp == "opponent" && !opp_done {
                    game.select_indices(keep_indices);
                    opp_done = true;
                } else {
                    game.select_indices(&[]); // skip re-prompt
                }
            }
            _ => game.select_indices(&[]),
        }
    }
}
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

// ====================================================================
// More edge cases
// ====================================================================

/// Keep fewer than the max (2 of 5) → 3 go under each deck; both draw 3.
#[test]
fn c6_select_2_shuffle_3_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    seed_decks(&mut game);
    fill_hands(&mut game, 5);
    let p1_hand_before = game.state.player1.hand.cards.len();
    let p2_hand_before = game.state.player2.hand.cards.len();
    let p1_deck_before = game.state.player1.main_deck.cards.len();
    let p2_deck_before = game.state.player2.main_deck.cards.len();

    baton_touch_dia(&mut game);
    both_players_keep(&mut game, &[0, 1]); // each keeps 2 of 5

    // Both: kept 2, sent 3 under, drew 3 → 2+3 = 5 (== before).
    assert_eq!(
        game.state.player1.hand.cards.len(),
        p1_hand_before,
        "P1: kept 2, sent 3 under, drew 3"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before,
        "P2: kept 2, sent 3 under, drew 3"
    );
    // 3 moved under, then 3 drawn from the top → deck length unchanged.
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        p1_deck_before,
        "P1 deck net unchanged (3 under, 3 drawn)"
    );
    assert_eq!(
        game.state.player2.main_deck.cards.len(),
        p2_deck_before,
        "P2 deck net unchanged (3 under, 3 drawn)"
    );
}

/// Skip the selection entirely (up to N, can keep 0) → the whole hand goes
/// under each deck, then both draw 3.
#[test]
fn c6_skip_keeps_none_shuffles_all_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    seed_decks(&mut game);
    fill_hands(&mut game, 3);
    let p1_hand_before = game.state.player1.hand.cards.len();
    let p2_hand_before = game.state.player2.hand.cards.len();
    let p1_deck_before = game.state.player1.main_deck.cards.len();
    let p2_deck_before = game.state.player2.main_deck.cards.len();

    baton_touch_dia(&mut game);
    both_players_keep(&mut game, &[]); // each keeps 0

    // 0 kept + 3 drawn = 3 (same as before, since all 3 went under).
    assert_eq!(
        game.state.player1.hand.cards.len(),
        p1_hand_before,
        "P1: kept 0, all 3 under, drew 3"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before,
        "P2: kept 0, all 3 under, drew 3"
    );
    // 3 moved under, 3 drawn → net unchanged.
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        p1_deck_before,
        "P1 deck net unchanged (3 under, 3 drawn)"
    );
    assert_eq!(
        game.state.player2.main_deck.cards.len(),
        p2_deck_before,
        "P2 deck net unchanged (3 under, 3 drawn)"
    );
}

/// Hand of exactly N (the max) → all kept, nothing shuffled under.
#[test]
fn c6_hand_exactly_max_all_kept() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    seed_decks(&mut game);
    fill_hands(&mut game, 3);
    let p1_hand_before = game.state.player1.hand.cards.len();
    let p2_hand_before = game.state.player2.hand.cards.len();
    let p1_deck_before = game.state.player1.main_deck.cards.len();
    let p2_deck_before = game.state.player2.main_deck.cards.len();

    baton_touch_dia(&mut game);
    both_players_keep(&mut game, &[0, 1, 2]); // each keeps all 3 (== max)

    // Both kept all 3, nothing under, drew 3 → +3.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        p1_hand_before + 3,
        "P1: kept all 3, nothing under, drew 3"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before + 3,
        "P2: kept all 3, nothing under, drew 3"
    );
    // Nothing moved under, 3 drawn → 3 fewer in each deck.
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        p1_deck_before - 3,
        "P1 deck: 3 drawn, nothing moved under"
    );
    assert_eq!(
        game.state.player2.main_deck.cards.len(),
        p2_deck_before - 3,
        "P2 deck: 3 drawn, nothing moved under"
    );
}
