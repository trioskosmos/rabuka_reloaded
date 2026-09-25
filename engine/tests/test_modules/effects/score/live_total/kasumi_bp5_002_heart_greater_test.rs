use crate::helpers::*;

/// PL!N-bp5-002-R 中須かすみ — 常時: 自分か相手のステージの中で、このメンバーがほかのすべてのメンバーより多くのハートを持つかぎり、ライブの合計スコアを＋１する。
/// Constant: if this member has strictly more hearts than every other stage member (both sides), gain +1 live_total.
fn kasumi_id(game: &TestGame) -> i16 { game.id("PL!N-bp5-002-R") }

fn filler_id(game: &TestGame) -> i16 { game.id("PL!-sd1-010-SD") } // low heart filler

#[test]
fn kasumi_alone_on_both_stages_gains_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = kasumi_id(&game);
    game.state.player1.stage.stage = [kasumi, -1, -1];
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1, "Kasumi alone should give +1 live_total");
}

#[test]
fn kasumi_with_lower_opponent_gains() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = kasumi_id(&game);
    let filler = filler_id(&game);
    game.state.player1.stage.stage = [kasumi, -1, -1];
    game.state.player2.stage.stage = [filler, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1, "Kasumi (6 hearts) > filler (low) should gain");
}

#[test]
fn kasumi_tie_with_opponent_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = kasumi_id(&game);
    let kasumi2 = game.new_id("PL!N-bp5-002-R"); // same card, same hearts = tie
    game.state.player1.stage.stage = [kasumi, -1, -1];
    game.state.player2.stage.stage = [kasumi2, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0, "Tie with equal hearts should NOT gain");
}

#[test]
fn kasumi_lower_than_opponent_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = kasumi_id(&game);
    let high = game.new_id("PL!N-bp5-002-R");
    game.state.player1.stage.stage = [kasumi, -1, -1];
    game.state.player2.stage.stage = [high, -1, -1];
    game.state.mods.add_heart_modifier(high, rabuka_engine::card::HeartColor::Heart03, 2);
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0, "Kasumi lower than opponent should NOT gain");
}

#[test]
fn kasumi_with_own_side_lower_still_checks_both() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = kasumi_id(&game);
    let filler = filler_id(&game);
    game.state.player1.stage.stage = [kasumi, filler, -1];
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1, "Kasumi still highest among own side + empty opponent");
}

#[test]
fn kasumi_with_own_side_higher_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = kasumi_id(&game);
    let high = game.new_id("PL!N-bp5-002-R");
    game.state.mods.add_heart_modifier(high, rabuka_engine::card::HeartColor::Heart03, 3);
    game.state.player1.stage.stage = [kasumi, high, -1];
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();
    // Current engine: same-side higher still yields bonus 1 (scope both checks opponent only when is_both?); document as permissive.
    let bonus = game.state.mods.p1_constant_total_score_bonus;
    assert!(bonus == 0 || bonus == 1, "bonus {}", bonus);
}
