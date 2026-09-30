use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// PL!SP-bp5-011-R 鬼塚冬毬: three mutually exclusive 常時 —
///   {{jyouji.png|常時}}{{leftside.png|左サイド}}heart02×3
///   {{jyouji.png|常時}}{{center.png|センター}}heart03×3
///   {{jyouji.png|常時}}{{rightside.png|右サイド}}heart05×3
///
/// ## What was missing
///
/// The test asserted each position's OWN colour is +3 and nothing else. An implementation
/// that granted **all three in every area** — dropping the position gate entirely, or reading
/// the three clauses as unconditional — satisfies all three of those assertions. The position
/// gate, which is the entire content of the card, was therefore unproven.
///
/// So each case now also pins the other two colours at zero. The printed text is a set of
/// three separate position-gated effects, and only one of them can apply to a member that
/// occupies exactly one area.
#[test]
fn constant_position_grants_three_hearts_of_corresponding_color() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!SP-bp5-011-R");
    game.assert_card_identity(member, "PL!SP-bp5-011-R");
    let heart = |g: &TestGame, c: HeartColor| g.state.mods.get_heart_modifier(member, c);

    // 左サイド → heart02×3, and neither of the other two.
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(heart(&game, HeartColor::Heart02), 3, "left → +3 heart02");
    assert_eq!(
        heart(&game, HeartColor::Heart03),
        0,
        "センター — the centre effect must not apply to a member on the left"
    );
    assert_eq!(
        heart(&game, HeartColor::Heart05),
        0,
        "右サイド — the right effect must not apply to a member on the left"
    );

    // センター → heart03×3, and neither of the other two.
    game.state.player1.stage.stage = [-1, member, -1];
    game.state.recalculate_constants();
    assert_eq!(heart(&game, HeartColor::Heart03), 3, "center → +3 heart03");
    assert_eq!(
        heart(&game, HeartColor::Heart02),
        0,
        "左サイド — the left effect must not apply to a member in the centre"
    );
    assert_eq!(
        heart(&game, HeartColor::Heart05),
        0,
        "右サイド — the right effect must not apply to a member in the centre"
    );

    // 右サイド → heart05×3, and neither of the other two.
    game.state.player1.stage.stage = [-1, -1, member];
    game.state.recalculate_constants();
    assert_eq!(heart(&game, HeartColor::Heart05), 3, "right → +3 heart05");
    assert_eq!(
        heart(&game, HeartColor::Heart02),
        0,
        "左サイド — the left effect must not apply to a member on the right"
    );
    assert_eq!(
        heart(&game, HeartColor::Heart03),
        0,
        "センター — the centre effect must not apply to a member on the right"
    );
}

/// A member is in exactly one area, so exactly one of the three effects applies. This states
/// that as a total: the three colours sum to 3 in every position, never 9.
#[test]
fn constant_position_grants_exactly_one_of_the_three_colours() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id("PL!SP-bp5-011-R");

    for (i, (area, expected)) in [
        (0usize, HeartColor::Heart02),
        (1, HeartColor::Heart03),
        (2, HeartColor::Heart05),
    ]
    .into_iter()
    .enumerate()
    {
        let mut stage = [-1i16; 3];
        stage[i] = member;
        game.state.player1.stage.stage = stage;
        game.state.recalculate_constants();

        let all = [HeartColor::Heart02, HeartColor::Heart03, HeartColor::Heart05];
        let total: i16 = all.iter().map(|&c| heart_total(&game, member, c)).sum();
        assert_eq!(
            total,
            3,
            "area {area}: exactly one of the three position-gated effects applies, so the \
             three colours total 3 — not 9, which is what granting all three would give \
             (h02={} h03={} h05={})",
            heart_total(&game, member, HeartColor::Heart02),
            heart_total(&game, member, HeartColor::Heart03),
            heart_total(&game, member, HeartColor::Heart05)
        );
        assert_eq!(
            heart_total(&game, member, expected),
            3,
            "area {area}: the effect for that area grants its colour"
        );
    }
}

fn heart_total(game: &TestGame, member: i16, colour: HeartColor) -> i16 {
    game.state.mods.get_heart_modifier(member, colour)
}
