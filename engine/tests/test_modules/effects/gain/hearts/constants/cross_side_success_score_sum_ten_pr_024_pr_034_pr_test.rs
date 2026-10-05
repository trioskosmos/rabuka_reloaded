//! 絢瀬絵里 `PL!-PR-024-PR` / 三船栞子 `PL!N-PR-034-PR` (常時)
//!
//! 自分と相手の成功ライブカード置き場にあるすべてのライブカードのスコアの合計が10以上
//! であるかぎり、{{heart_01.png|heart01}}を得る。
//!
//! The printed text names BOTH success zones (自分と相手), which is the entire
//! difference from `PL!-pb2-020-N` — same shape, but its zone is 自分's alone and
//! its threshold is 9. A shared "sum the success zone" helper that always reads
//! player1 would pass every pb2-020 test and still fail this card's own text, so
//! the opponent's zone is asserted here on its own.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-PR-024-PR";
const SHIKO: &str = "PL!N-PR-034-PR";
/// Score 9 — the whole of my zone, one short of the threshold.
const LIVE9: &str = "PL!S-pb1-023-L";
/// Score 1 — the card that carries a 9-only zone over the line.
const LIVE1: &str = "LL-bp5-001-L";
/// Score 5.
const LIVE5: &str = "PL!-bp3-022-L";

fn heart01(game: &TestGame, id: i16) -> i16 {
    game.state.mods.get_heart_modifier(id, HeartColor::Heart01)
}

#[test]
fn my_zone_of_nine_alone_is_short_of_the_threshold() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    game.add_to_stage(MemberArea::Center, eri);

    let live9 = game.id(LIVE9);
    game.assert_card_score(live9, 9);
    game.state.player1.success_live_card_zone.add_card(live9);
    game.state.recalculate_constants();

    assert_eq!(
        heart01(&game, eri),
        0,
        "自分の置き場だけで9 — the threshold is 10, not pb2-020's 9"
    );
}

#[test]
fn my_own_zone_reaching_exactly_ten_grants_the_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.add_to_stage(MemberArea::Center, eri);

    game.state.player1.success_live_card_zone.add_card(game.id(LIVE9));
    game.state.player1.success_live_card_zone.add_card(game.id(LIVE1));
    game.state.recalculate_constants();

    assert_eq!(
        heart01(&game, eri),
        1,
        "自分の置き場で合計ちょうど10 — the self half of the sum must still cross"
    );
}

#[test]
fn opponent_success_zone_counts_toward_the_sum() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.add_to_stage(MemberArea::Center, eri);

    let live9 = game.id(LIVE9);
    game.state.player1.success_live_card_zone.add_card(live9);
    game.state.recalculate_constants();
    assert_eq!(heart01(&game, eri), 0, "precondition: 9 is not enough");

    let live1 = game.id(LIVE1);
    game.assert_card_score(live1, 1);
    game.state
        .player2
        .success_live_card_zone
        .add_card(live1);
    game.state.recalculate_constants();

    assert_eq!(
        heart01(&game, eri),
        1,
        "自分と相手の成功ライブカード置き場 — the opponent's zone completes 9 + 1"
    );
}

#[test]
fn opponent_zone_alone_crosses_the_threshold() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.add_to_stage(MemberArea::Center, eri);

    let a = game.id(LIVE5);
    let b = game.new_id(LIVE5);
    game.state.player2.success_live_card_zone.add_card(a);
    game.state.player2.success_live_card_zone.add_card(b);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.player2.success_live_card_zone.len(),
        2,
        "precondition: 5 + 5 sits in the opponent's zone"
    );
    assert_eq!(
        heart01(&game, eri),
        1,
        "相手の置き場のみでも合計10 — my own zone may be empty"
    );
}

#[test]
fn off_stage_grants_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.state.player1.success_live_card_zone.add_card(game.id(LIVE9));
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id(LIVE1));
    game.state.recalculate_constants();

    assert_eq!(
        heart01(&game, eri),
        0,
        "常時 — an off-stage member contributes nothing even with a satisfied sum"
    );
}

/// The card prints heart01 only — the sum must not leak into other colours.
#[test]
fn the_granted_heart_is_heart01_and_nothing_else() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.add_to_stage(MemberArea::Center, eri);
    game.state.player1.success_live_card_zone.add_card(game.id(LIVE9));
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id(LIVE1));
    game.state.recalculate_constants();

    assert_eq!(heart01(&game, eri), 1, "合計10 — heart01 appears once");
    for colour in [HeartColor::Heart03, HeartColor::Heart06] {
        assert_eq!(
            game.state.mods.get_heart_modifier(eri, colour),
            0,
            "the printed heart01 must not collateral-grant {colour:?}"
        );
    }
}

/// The same printed text on a different card — a second member must satisfy it
/// independently, not inherit eri's modifier.
#[test]
fn shiko_satisfies_the_same_text_on_her_own() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shiko = game.id(SHIKO);
    game.assert_card_identity(shiko, SHIKO);
    game.add_to_stage(MemberArea::Center, shiko);

    game.state.player1.success_live_card_zone.add_card(game.id(LIVE9));
    game.state.recalculate_constants();
    assert_eq!(
        heart01(&game, shiko),
        0,
        "PL!N-PR-034-PR — 9 alone is short"
    );

    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id(LIVE1));
    game.state.recalculate_constants();
    assert_eq!(
        heart01(&game, shiko),
        1,
        "PL!N-PR-034-PR — the opponent's zone completes the sum"
    );
}
