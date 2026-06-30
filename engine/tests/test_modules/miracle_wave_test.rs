/// Tests for MIRACLE WAVE (PL!S-bp3-019-L) — LiveSuccess set_score:
///
/// ライブ成功時 このターン、エールにより公開された自分のカードの中に
/// ブレードハートを持たないカードが0枚の場合か、または自分が余剰ハートを
/// 2つ以上持っている場合、このカードのスコアは４になる。
///
/// OR condition: A (no blade heart = 0) OR B (excess >= 2) → score = 4.
/// Q182: Either condition met → score = 4.
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// Q182: Excess heart >= 2 (condition B) → score set to 4.
/// Stage: 2x PL!S-sd1-001-SD (千歌: h02=3, h04=2, h05=2 × 2 = 6,4,4)
/// Need: h02=4, h04=4, h05=4. Total given: 14. Excess: 2. ✓
#[test]
fn miracle_wave_q182_excess_heart_score_4() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let wave = game.id("PL!S-bp3-019-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [game.id("PL!S-sd1-001-SD"), game.id("PL!S-sd1-001-SD"), -1];
    game.state.player1.hand.cards.push(wave);

    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(wave);
    advance_to_live_start(&mut game);

    game.pass(); // → SecondAttackerPerformance
    game.drain_auto_ability_choices();
    game.pass(); // → LiveVictoryDetermination (set)
    game.drain_auto_ability_choices();
    game.pass(); // → Active (processes LiveVictoryDetermination)
    game.drain_auto_ability_choices();

    let mod_val = game.state.mods.get_score_modifier(wave);
    // Card base score is 7, set_score adjusts modifier: 4 - 7 = -3 → final = 7 + (-3) = 4
    assert_eq!(
        mod_val, -3,
        "modifier should be -3 (base=7 + modifier=-3 = final=4), got {}",
        mod_val
    );

    // Verify the actual live score in the performance snapshot
    let live_score = game
        .state
        .performance_snapshots
        .iter()
        .flat_map(|s| s.lives.iter())
        .find(|l| l.card_id == wave)
        .map(|l| l.score)
        .unwrap_or(0);
    assert_eq!(live_score, 4, "Live score should be 4, got {}", live_score);
}
