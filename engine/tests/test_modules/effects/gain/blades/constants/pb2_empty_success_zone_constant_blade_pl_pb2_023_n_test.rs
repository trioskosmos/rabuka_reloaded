//! 南ことり `PL!-pb2-023-N` (常時)
//!
//! 自分の成功ライブカード置き場にカードがないかぎり、{{icon_blade.png|ブレード}}を得る。
//!
//! A NEGATED location condition: the bonus is present while the zone is empty and
//! lapses the moment anything is in it.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const KOTORI: &str = "PL!-pb2-023-N";

#[test]
fn empty_success_zone_grants_one_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kotori = game.id(KOTORI);
    game.assert_card_identity(kotori, KOTORI);
    game.assert_card_cost(kotori, 7);
    game.add_to_stage(MemberArea::Center, kotori);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(kotori),
        1,
        "成功ライブカード置き場にカードがない — the negated condition holds and +1 blade appears"
    );
}

#[test]
fn any_card_in_the_zone_lapses_the_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kotori = game.id(KOTORI);
    game.add_to_stage(MemberArea::Center, kotori);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(kotori),
        1,
        "precondition: empty zone grants the blade"
    );

    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.success_live_card_zone.add_card(live);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(kotori),
        0,
        "かぎり — one card in the zone breaks カードがない and the blade must lapse"
    );

    game.state.player1.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(kotori),
        1,
        "emptying the zone restores the bonus"
    );
}

/// The opponent's success zone is not 自分の — an empty own zone still grants.
#[test]
fn opponent_zone_does_not_lapse_the_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kotori = game.id(KOTORI);
    game.add_to_stage(MemberArea::Center, kotori);
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id("PL!-sd1-021-SD"));
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(kotori),
        1,
        "自分の成功ライブカード置き場 — the opponent's cards do not break カードがない"
    );
}

#[test]
fn off_stage_grants_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kotori = game.id(KOTORI);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(kotori),
        0,
        "常時 — off stage she contributes no blade"
    );
}
