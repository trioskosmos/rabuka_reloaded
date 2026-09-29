//! 東條 希 `PL!-pb2-025-N` (常時)
//!
//! 自分の成功ライブカード置き場にある『lily white』のカード1枚につき、
//! {{icon_blade.png|ブレード}}を得る。
//!
//! A per-card multiplier gated on a group, so it must scale with the number of
//! matching cards and ignore everything else in the zone.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const NOZOMI: &str = "PL!-pb2-025-N";
/// `lily white` lives, cheapest first.
const LW_CHEAP: &str = "PL!-pb1-029-SRL";
const LW_MID: &str = "PL!-bp5-023-SRL";
const LW_THIRD: &str = "PL!-bp4-025-SRL";
/// Not `lily white`.
const MUS_LIVE: &str = "PL!-sd1-021-SD";

/// Put the member on stage, load the success zone, return the blade modifier.
fn blade_with(success_zone: &[&str]) -> i16 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id(NOZOMI);
    game.add_to_stage(MemberArea::Center, nozomi);
    for card_no in success_zone {
        game.state
            .player1
            .success_live_card_zone
            .add_card(game.id(card_no));
    }
    game.state.recalculate_constants();
    game.state.mods.get_blade_modifier(nozomi)
}

#[test]
fn one_lily_white_card_grants_one_blade() {
    assert_eq!(
        blade_with(&[LW_CHEAP]),
        1,
        "『lily white』のカード1枚につき — one matching card is +1 blade"
    );
}

#[test]
fn two_lily_white_cards_grant_two_blades() {
    assert_eq!(
        blade_with(&[LW_CHEAP, LW_MID]),
        2,
        "1枚につき scales — two matching cards are +2, not +1"
    );
}

#[test]
fn three_lily_white_cards_grant_three_blades() {
    assert_eq!(
        blade_with(&[LW_CHEAP, LW_MID, LW_THIRD]),
        3,
        "1枚につき keeps scaling to 3"
    );
}

#[test]
fn non_lily_white_cards_are_not_counted() {
    assert_eq!(
        blade_with(&[MUS_LIVE, MUS_LIVE]),
        0,
        "『lily white』のカード — μ's lives are not lily white and must not count"
    );
}

#[test]
fn mixed_zone_counts_only_the_lily_white_cards() {
    assert_eq!(
        blade_with(&[MUS_LIVE, LW_CHEAP, MUS_LIVE, LW_MID]),
        2,
        "only the two lily white cards count; the μ's cards are ignored"
    );
}

#[test]
fn empty_zone_grants_nothing() {
    assert_eq!(
        blade_with(&[]),
        0,
        "no cards in 自分の成功ライブカード置き場 — 0 matching cards is +0"
    );
}

/// The zone is 自分の — the opponent's lily white lives must not add blades.
#[test]
fn opponent_zone_does_not_contribute() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id(NOZOMI);
    game.add_to_stage(MemberArea::Center, nozomi);
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id(LW_CHEAP));
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(nozomi),
        0,
        "自分の成功ライブカード置き場 — the opponent's lily white card must not count"
    );
}

#[test]
fn off_stage_grants_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id(NOZOMI);
    game.state
        .player1
        .success_live_card_zone
        .add_card(game.id(LW_CHEAP));
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(nozomi),
        0,
        "常時 — off stage she contributes no blade"
    );
}
