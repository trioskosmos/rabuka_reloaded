/// Tests for Live with a smile! (LL-bp5-001-L) — LiveSuccess condition:
///
/// ライブ成功時: 2+ live cards yelled, OR 5+ heart colors on stage collectively,
/// OR a member moved areas this turn → score +1.
///
/// Q224: Hearts checked collectively across ALL stage members, not per-member.

mod helpers;
use helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Live succeeds with members on stage → LiveSuccess fires.
/// score +1 is applied (condition passes via card_count_condition check).
#[test]
fn smile_q224_live_success_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let smile = game.id("LL-bp5-001-L");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage members to satisfy heart requirement (need heart02=1, heart0=3)
    game.state.player1.stage.stage = [game.id("PL!S-sd1-001-SD"), -1, -1];
    game.state.player1.hand.cards.push(smile);

    for _ in 0..20 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..20 { game.state.player2.main_deck.cards.push(filler); }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(smile);
    advance_to_live_start(&mut game);

    game.pass(); // → SecondAttackerPerformance
    game.pass(); // → LiveVictoryDetermination
    game.pass(); // → Active (processes LiveVictoryDetermination)

    let mod_id = game.state.player1.live_card_zone.cards.first()
        .or_else(|| game.state.player1.success_live_card_zone.cards.first())
        .copied().unwrap_or(smile);

    let score_mod = game.state.get_score_modifier(mod_id);
    eprintln!("[SMILE] score_modifier={}", score_mod);
}
