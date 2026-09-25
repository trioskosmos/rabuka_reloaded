/// Rule 1.2.1.1 / 1.2.1.2 / 10.3.1 — who wins, not merely that the game ended.
///
/// 1.2.1.1 自分の成功ライブカード置き場が3枚以上なら、勝利となる。
///         ただし、相手：「成功ライブカード置き場」のカードが2枚以下なら勝利。
/// 1.2.1.2 両者の「成功ライブカード置き場」のカードが両方とも3枚以上なら
///         ゲームは引き分けとなる。
/// 10.3.1  自分の成功ライブカード置き場のカードが3枚以上なら勝利処理を行う。
///
/// The winner is reported as GameResult::{FirstAttackerWins,SecondAttackerWins},
/// so these tests pin the seat attribution, not just `game_ended`.
use crate::helpers::*;
use rabuka_engine::game_state::GameResult;
use rabuka_engine::turn::TurnEngine;

fn success_lives(game: &mut TestGame, count: usize) -> Vec<i16> {
    let live = game.id("PL!-sd1-019-SD");
    (0..count).map(|_| game.new_id("PL!-sd1-019-SD")).collect()
}

/// Rule 1.2.1.1: 3 success cards against an opponent with 0 wins, and the
/// result names the winning seat — not the attacker's turn order.
#[test]
fn three_success_cards_wins_and_result_names_the_winning_seat() {
    for (p1_first, expected) in [
        (true, GameResult::FirstAttackerWins),
        (false, GameResult::SecondAttackerWins),
    ] {
        let db = load_real_database();
        let mut game = TestGame::new(db);
        game.state.player1.is_first_attacker = p1_first;
        game.state.player2.is_first_attacker = !p1_first;
        let p1_success = success_lives(&mut game, 3);
        game.state.player1.success_live_card_zone.cards = p1_success.clone().into();

        TurnEngine::check_timing(&mut game.state);

        assert!(game.state.game_ended, "3 success cards must end the game");
        assert_eq!(
            game.state.game_result,
            expected,
            "p1_first={p1_first}: the winner is the player holding 3 success cards"
        );
    }
}

/// Rule 1.2.1.1 boundary: the opponent holding exactly 2 is still a loss for
/// them, so 3 vs 2 is a win (2 vs 3 would not be).
#[test]
fn three_versus_two_success_cards_is_a_win_not_a_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state.player1.is_first_attacker = true;
    let p1_success = success_lives(&mut game, 3);
    let p2_success = success_lives(&mut game, 2);
    game.state.player1.success_live_card_zone.cards = p1_success.into();
    game.state.player2.success_live_card_zone.cards = p2_success.into();

    TurnEngine::check_timing(&mut game.state);

    assert!(game.state.game_ended);
    assert_eq!(
        game.state.game_result,
        GameResult::FirstAttackerWins,
        "the opponent at 2 cards is '2以下', so this is a win, not a draw"
    );
}

/// Rule 1.2.1.1 from the other seat: 3 vs 0 must attribute the win to P2, and
/// must not be confused with P2 merely being the second attacker.
#[test]
fn three_success_cards_for_p2_attributes_the_win_to_p2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state.player1.is_first_attacker = true;
    game.state.player2.is_first_attacker = false;
    let p2_success = success_lives(&mut game, 3);
    game.state.player2.success_live_card_zone.cards = p2_success.into();

    TurnEngine::check_timing(&mut game.state);

    assert!(game.state.game_ended);
    assert_eq!(
        game.state.game_result,
        GameResult::SecondAttackerWins,
        "P2 holds the 3 success cards, so P2 wins even though P2 is second attacker"
    );
}

/// Rule 1.2.1.2: both players reach 3+ in the same check timing → draw.
#[test]
fn both_players_at_three_success_cards_is_a_draw() {
    for (p1_first, label) in [(true, "p1 first attacker"), (false, "p2 first attacker")] {
        let db = load_real_database();
        let mut game = TestGame::new(db);
        game.state.player1.is_first_attacker = p1_first;
        game.state.player2.is_first_attacker = !p1_first;
        let p1_success = success_lives(&mut game, 3);
        let p2_success = success_lives(&mut game, 3);
        game.state.player1.success_live_card_zone.cards = p1_success.into();
        game.state.player2.success_live_card_zone.cards = p2_success.into();

        TurnEngine::check_timing(&mut game.state);

        assert!(game.state.game_ended, "{label}: the game must end");
        assert_eq!(
            game.state.game_result,
            GameResult::Draw,
            "{label}: 3 vs 3 is a draw regardless of attack order"
        );
    }
}

/// Rule 10.3.1 negative control: 2 vs 2 is not a win, so the game continues.
#[test]
fn two_versus_two_success_cards_does_not_end_the_game() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p1_success = success_lives(&mut game, 2);
    let p2_success = success_lives(&mut game, 2);
    game.state.player1.success_live_card_zone.cards = p1_success.into();
    game.state.player2.success_live_card_zone.cards = p2_success.into();

    TurnEngine::check_timing(&mut game.state);

    assert!(
        !game.state.game_ended,
        "2 success cards is below the threshold of 3, so the game continues"
    );
}
