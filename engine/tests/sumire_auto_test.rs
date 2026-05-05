/// Tests for PL!SP-bp2-015-N 平安名すみれ — Auto ability (ab#0):
///
/// {{jidou.png|自動}}{{turn1.png|ターン1回}}エールにより公開された自分のカードの中に
/// ブレードハートを持つカードがないとき、ライブ終了まで、heart06を得る。
///
/// Q112: Does ALL blade count as blade heart? A: Yes.
/// Q113: If no cheer occurs (0 blades), does the ability trigger? A: No.

mod helpers;
use helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_success(game: &mut TestGame) {
    game.pass(); game.pass(); game.pass(); game.pass(); game.pass();
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

/// Q113: No cheer occurs (member on stage has 0 blades, so no yell happens).
/// The auto ability should NOT trigger.
#[test]
fn sumire_q113_no_cheer_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let sumire = game.id("PL!SP-bp2-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    // A member with 0 blades (so no yell/cheer happens)
    let zero_blade_member = game.id("PL!-sd1-001-SD");

    // Stage: 平安名すみれ + zero-blade member (only for presence)
    game.state.player1.stage.stage = [zero_blade_member, sumire, -1];
    game.state.player1.hand.cards.push(filler);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    // Set a live card so the performance starts
    game.state.player1.hand.cards.push(filler);
    game.set_live_card(filler);
    advance_to_live_success(&mut game);

    let heart_mod = game.state.get_heart_modifier(sumire, HeartColor::Heart06);
    assert_eq!(heart_mod, 0,
        "No cheer → ability should not trigger → no heart06 modifier");
}

/// Q112: If revealed cards have blade heart, condition fails (no heart06 gain).
/// Test with a stage member that has blades → cheer happens → some revealed
/// cards will have blade hearts → condition fails.
#[test]
fn sumire_q112_cheer_with_blade_heart_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let sumire = game.id("PL!SP-bp2-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    // A member with blades to trigger cheer
    let bladed_member = game.id("PL!S-sd1-003-SD");

    game.state.player1.stage.stage = [bladed_member, sumire, -1];
    game.state.player1.hand.cards.push(filler);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(filler);
    game.set_live_card(filler);
    advance_to_live_success(&mut game);

    let heart_mod = game.state.get_heart_modifier(sumire, HeartColor::Heart06);
    assert_eq!(heart_mod, 0,
        "Blade heart cards exist → condition fails → no heart06 gain");
}
