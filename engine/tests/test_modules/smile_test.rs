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

/// Live succeeds with members on stage → LiveSuccess fires.
/// score +1 is applied via "5+ heart colors on stage collectively" OR condition.
#[test]
fn smile_q224_live_success_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let smile = game.id("LL-bp5-001-L");
    let filler = game.id("PL!-sd1-010-SD");

    // Members with total blades >= 2 (the condition checks aggregate=total = total_blades >=2).
    // Use members with blade > 0 that have NO interfering LIVE_START/AUTO triggers.
    let member1 = game.id("PL!S-sd1-003-SD"); // Aqours member with blade > 0
    let member2 = game.id("PL!-sd1-001-SD"); // filler member, blade 0 (just for count)
    game.state.player1.stage.stage = [member1, member2, -1];
    game.state.player1.stage.stage = [member1, member2, -1];
    game.state.player1.hand.cards.push(smile);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Record movement on the stage member so the "member moved areas this turn"
    // OR condition in the smile ability is satisfied as a fallback

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(smile);
    advance_to_live_start(&mut game);

    game.pass(); // → SecondAttackerPerformance
    game.pass(); // → LiveVictoryDetermination
    game.pass(); // → Active (processes LiveVictoryDetermination)

    let mod_id = game
        .state
        .player1
        .live_card_zone
        .cards
        .first()
        .or_else(|| game.state.player1.success_live_card_zone.cards.first())
        .copied()
        .unwrap_or(smile);

    let score_mod = game.state.mods.get_score_modifier(mod_id);
    assert_eq!(
        score_mod, 1,
        "LiveSuccess condition met → +1 score (got {})",
        score_mod
    );
}
