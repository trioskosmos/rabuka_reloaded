//! 園田海未 `PL!-pb2-004-PP` / `-R` (常時)
//!
//! 自分の成功ライブカード置き場にある{{icon_score.png|スコア}}を持つ『μ's』のカード1枚につき、
//! {{icon_blade.png|ブレード}}を得る。
//!
//! Two filters stack: the card must be a μ's card AND carry the スコア icon. The
//! fixtures deliberately mix the four ways a μ's live can miss the icon, because the
//! icon filter is a *card property* (`Card::has_score_icon`) and not an ability read.
//!
//! Note on the fixture set: among μ's lives, sweet&sweet holiday and WAO-WAO Powerful
//! day! print the ドロー icon, not スコア, so they are negative cases even though they
//! are μ's. ダイヤモンドプリンセスの憂鬱 (BiBi) and 知らないLove (lily white) are the
//! μ's lives that actually print the スコア icon.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const UMI: &str = "PL!-pb2-004-PP";

/// μ's (BiBi) with the スコア icon.
const MUS_SCORE_BIBI: &str = "PL!-bp4-026-L";
/// μ's (lily white) with the スコア icon — a different unit, same group.
const MUS_SCORE_LW: &str = "PL!-pb1-029-SRL";
/// μ's (Printemps) with the ドロー icon — right group, wrong icon.
const MUS_DRAW: &str = "PL!-bp6-023-L";
/// μ's (Printemps) with no icon at all.
const MUS_PLAIN: &str = "PL!-bp4-024-L";
/// Not μ's (DOLLCHESTRA / 蓮ノ空) but carries the スコア icon — isolates the group
/// filter from the icon filter.
const NON_MUS_SCORE: &str = "PL!HS-bp1-019-L";

fn blade_with(success_zone: &[&str]) -> i16 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id(UMI);
    game.assert_card_identity(umi, UMI);
    game.assert_card_cost(umi, 15);
    game.add_to_stage(MemberArea::Center, umi);
    for card_no in success_zone {
        game.state
            .player1
            .success_live_card_zone
            .add_card(game.id(card_no));
    }
    game.state.recalculate_constants();
    game.state.mods.get_blade_modifier(umi)
}

#[test]
fn one_mus_score_icon_card_grants_one_blade() {
    assert_eq!(
        blade_with(&[MUS_SCORE_BIBI]),
        1,
        "{{icon_score}}を持つ『μ's』のカード1枚につき — one qualifying card is +1 blade"
    );
}

/// Two qualifying cards in DIFFERENT μ's units still count 1 each — 「μ's」 spans
/// Printemps, lily white and BiBi, so the group filter must not be read as a unit
/// string compare.
#[test]
fn two_mus_score_icon_cards_in_different_units_grant_two_blades() {
    assert_eq!(
        blade_with(&[MUS_SCORE_BIBI, MUS_SCORE_LW]),
        2,
        "1枚につき scales to 2 across two different μ's units"
    );
}

#[test]
fn mus_card_with_draw_icon_is_not_counted() {
    assert_eq!(
        blade_with(&[MUS_DRAW]),
        0,
        "sweet&sweet holiday is a μ's card but prints the ドロー icon, so \
         {{icon_score}}を持つ excludes it"
    );
}

#[test]
fn mus_card_with_no_icon_is_not_counted() {
    assert_eq!(
        blade_with(&[MUS_PLAIN]),
        0,
        "小夜啼鳥恋詩 is a μ's card with no score/draw icon, so {{icon_score}}を持つ \
         excludes it"
    );
}

#[test]
fn mixed_zone_counts_only_mus_score_icon_cards() {
    assert_eq!(
        blade_with(&[MUS_DRAW, MUS_SCORE_BIBI, MUS_PLAIN, MUS_SCORE_LW]),
        2,
        "of four μ's lives only the two printing the スコア icon count; the ドロー-icon \
         and icon-less μ's cards are excluded"
    );
}

#[test]
fn non_mus_card_with_score_icon_is_not_counted() {
    assert_eq!(
        blade_with(&[NON_MUS_SCORE]),
        0,
        "『μ's』のカード — a DOLLCHESTRA live carrying the スコア icon fails the group filter"
    );
}

#[test]
fn empty_zone_grants_nothing() {
    assert_eq!(
        blade_with(&[]),
        0,
        "0 qualifying cards is +0"
    );
}

#[test]
fn opponent_zone_does_not_contribute() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id(UMI);
    game.add_to_stage(MemberArea::Center, umi);
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id(MUS_SCORE_BIBI));
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(umi),
        0,
        "自分の成功ライブカード置き場 — the opponent's qualifying card must not count"
    );
}

#[test]
fn off_stage_grants_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let umi = game.id(UMI);
    game.state
        .player1
        .success_live_card_zone
        .add_card(game.id(MUS_SCORE_BIBI));
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(umi),
        0,
        "常時 — off stage she contributes no blade"
    );
}
