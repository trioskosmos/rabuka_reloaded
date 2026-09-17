use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Target: PL!N-bp4-012-P (Umi)
// 常時: If opponent's success_live_card_zone total score >= 6 → +1 score.
// So: comparison_condition with aggregate=total, comparison_type=score, operator=>=, count=6
// ====================================================================

/// Edge case: Opponent score exactly 6 → +1 score mod.
#[test]
fn opponent_success_score_six_grants_one_live_total_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let umi = game.id("PL!N-bp4-012-P");
    let live_6 = game.id("PL!SP-bp1-027-L");

    game.add_to_stage(MemberArea::Center, umi);

    // Single live card with score=6
    game.state.player2.success_live_card_zone.cards.push(live_6);

    game.state.recalculate_constants();

    // 「ライブの合計スコアを＋１する」 is a live-TOTAL bonus: it lands in the
    // per-player accumulator, not as a per-card modifier keyed under Umi's id.
    let score_mod = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(
        score_mod, 1,
        "Umi: opponent score >=6 → +1 live total score bonus, got {}",
        score_mod
    );
}

/// Edge case: Opponent score < 6 → no score mod.
#[test]
fn opponent_success_score_zero_leaves_per_card_score_modifier_zero() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let umi = game.id("PL!N-bp4-012-P");
    game.add_to_stage(MemberArea::Center, umi);

    // No success live cards → score = 0
    game.state.recalculate_constants();

    let score_mod = game.state.mods.get_score_modifier(umi);
    assert_eq!(
        score_mod, 0,
        "Umi: opponent score 0 → no score mod, got {}",
        score_mod
    );
}

/// Edge case: Score goes from >=6 to <6 when cards are removed.
#[test]
fn opponent_success_score_removal_clears_live_total_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let umi = game.id("PL!N-bp4-012-P");
    let live_6 = game.id("PL!SP-bp1-027-L");

    game.add_to_stage(MemberArea::Center, umi);
    game.state.player2.success_live_card_zone.cards.push(live_6);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "opponent score >=6 → +1 live-total bonus"
    );

    // Remove opponent's cards
    game.state.player2.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();

    let score_mod = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(
        score_mod, 0,
        "Umi: opponent cards removed → 0 live-total bonus, got {}",
        score_mod
    );
}
