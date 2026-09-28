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

/// A higher member on her OWN side disqualifies her, exactly as a higher member
/// on the opponent's side does (`kasumi_lower_than_opponent_no_gain`).
///
/// `scope: both`, `all: true`, `exclude_self: true` — she must be strictly higher
/// than every other member across **both** stages, so the comparison member must
/// NOT be another 未来.
///
/// This test previously staged a second `PL!N-bp5-002-R` as the higher own-side
/// member and asserted `bonus == 0 || bonus == 1` "to document as permissive".
/// That was wrong twice over: the disjunction is over the same variable (true
/// for any outcome — the `or-equal-self` shape
/// `python cards/jidou_test_audit.py --sweep c` reports), and the fixture made
/// the "higher" member a 未来 herself, who is then the highest on the board and
/// legitimately grants her own +1. The engine was right throughout; the +1
/// belonged to the second 未来, not to the one under test.
///
/// The comparison member here is a non-未来 card, so nobody else can grant and
/// the total is attributable to her alone.
#[test]
fn kasumi_with_own_side_higher_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kasumi = kasumi_id(&game);
    let high = filler_id(&game); // a non-未来 member, so only kasumi can grant
    game.state
        .mods
        .add_heart_modifier(high, rabuka_engine::card::HeartColor::Heart03, 6);
    game.state.player1.stage.stage = [kasumi, high, -1];
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();
    let bonus = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(
        bonus, 0,
        "a member on her OWN side with more hearts disqualifies her: the condition \
         is 'strictly more hearts than every other member across BOTH stages', not \
         'more than every opponent'. `high` is a non-未来 card, so nothing else can \
         grant and this total is hers alone. The old fixture used a second 未来 as \
         the higher member, who then legitimately granted the +1 itself."
    );
}
