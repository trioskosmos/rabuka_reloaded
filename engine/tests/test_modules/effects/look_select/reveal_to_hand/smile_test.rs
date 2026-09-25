/// Tests for Live with a smile! (LL-bp5-001-L) — LiveSuccess condition:
///
/// ライブ成功時: 2+ live cards yelled, OR 5+ heart colors on stage collectively,
/// OR a member moved areas this turn → score +1.
///
/// Q224: Hearts checked collectively across ALL stage members, not per-member.
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

#[test]
fn smile_q224_live_success_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let smile = game.id("LL-bp5-001-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member1 = game.id("PL!S-sd1-003-SD");
    let member2 = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [member1, member2, -1];
    game.state.player1.hand.cards.push(smile);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(smile);
    advance_to_live_start(&mut game);

    game.pass();
    game.pass();
    game.pass();

    let l = &game.state.performance_snapshots[0].lives[0];
    assert_eq!(l.score - l.base_score, 1, "five distinct heart types add one score");
}

#[test]
fn smile_q224_five_icons_with_three_types_does_not_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let smile = game.id("LL-bp5-001-L");
    let filler = game.id("PL!-sd1-010-SD");
    let member1 = game.id("PL!S-sd1-003-SD");
    let member2 = game.id("PL!S-sd1-003-SD");

    game.state.player1.stage.stage = [member1, member2, -1];
    game.state.player1.hand.cards.push(smile);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(smile);
    advance_to_live_start(&mut game);

    game.pass();
    game.pass();
    game.pass();

    let l = &game.state.performance_snapshots[0].lives[0];
    assert_eq!(
        l.score - l.base_score,
        0,
        "five heart icons of only three types do not add score"
    );
}
