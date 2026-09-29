//! 星空 凛 `PL!-pb2-005-PP` / `-R` (登場, gain_ability)
//!
//! {{toujyou.png|登場}}自分の成功ライブカード置き場に{{icon_score.png|スコア}}を持つ『μ's』の
//! カードがある場合、ライブ終了時まで、
//! 「{{jyouji.png|常時}}自分のステージにいる『μ's』のメンバーは、{{icon_blade.png|ブレード}}を得る。」
//! を得る。
//!
//! Two independent filters gate the gain — a score-icon card AND a μ's card in the
//! success zone — so the fixtures vary each one separately. The gained ability then
//! applies to EVERY μ's member on stage, including her, and to nobody else.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const RIN: &str = "PL!-pb2-005-PP";
/// μ's member with no ability of her own, so nothing else contributes a blade.
const MUS_ALLY: &str = "PL!-bp6-017-N"; // 小泉花陽, Printemps
/// Not μ's, no ability.
const NON_MUS: &str = "PL!S-bp5-012-N"; // 松浦果南, AZALEA

/// Success-zone fixtures: μ's + score icon, μ's + no icon, non-μ's + score icon.
const MUS_SCORE: &str = "PL!-bp4-026-L"; // BiBi, スコア icon
const MUS_NO_ICON: &str = "PL!-bp4-024-L"; // Printemps, no icon
const NON_MUS_SCORE: &str = "PL!HS-bp1-019-L"; // DOLLCHESTRA, スコア icon

/// Play 凛 to stage with `success_zone` pre-loaded; return (rin, ally, non_mus).
fn debut_with(success_zone: &[&str]) -> (TestGame, i16, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let ally = game.id(MUS_ALLY);
    let non_mus = game.id(NON_MUS);
    game.state.player1.stage.stage[MemberArea::RightSide as usize] = ally;
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = non_mus;
    for card_no in success_zone {
        game.state
            .player1
            .success_live_card_zone
            .add_card(game.id(card_no));
    }

    let rin = game.id(RIN);
    game.assert_card_identity(rin, RIN);
    game.assert_card_cost(rin, 5);
    game.state.player1.hand.cards.push(rin);
    game.give_energy(10);
    game.play_to_stage(rin, MemberArea::Center);
    game.drain_auto_ability_choices();
    game.state.recalculate_constants();
    (game, rin, ally, non_mus)
}

#[test]
fn mus_score_card_grants_the_constant_to_every_mus_member() {
    let (mut game, rin, ally, non_mus) = debut_with(&[MUS_SCORE]);

    assert_eq!(
        game.state.mods.get_blade_modifier(ally),
        1,
        "自分のステージにいる『μ's』のメンバー — the other μ's member gains +1 blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(rin),
        1,
        "she is a μ's member too, so the gained constant applies to her"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(non_mus),
        0,
        "『μ's』のメンバー — an AZALEA member gains nothing"
    );
}

/// Break the icon half of the condition only: right group, no スコア icon.
#[test]
fn mus_card_without_score_icon_grants_nothing() {
    let (mut game, _rin, ally, _non_mus) = debut_with(&[MUS_NO_ICON]);

    assert_eq!(
        game.state.mods.get_blade_modifier(ally),
        0,
        "自分の成功ライブカード置き場に{{icon_score}}を持つ — a μ's live with no \
         スコア icon does not qualify"
    );
}

/// Break the group half only: right icon, wrong group.
#[test]
fn non_mus_score_card_grants_nothing() {
    let (mut game, _rin, ally, _non_mus) = debut_with(&[NON_MUS_SCORE]);

    assert_eq!(
        game.state.mods.get_blade_modifier(ally),
        0,
        "『μ's』のカード — a DOLLCHESTRA live carrying the スコア icon fails the group filter"
    );
}

#[test]
fn empty_success_zone_grants_nothing() {
    let (mut game, _rin, ally, _non_mus) = debut_with(&[]);

    assert_eq!(
        game.state.mods.get_blade_modifier(ally),
        0,
        "no cards in the success zone, so the ability is not gained"
    );
}

/// The condition is a card in the zone, not one arriving there afterwards. The
/// gained ability is ライブ終了時まで — the trigger is her debut, and the gate is
/// evaluated at that moment.
#[test]
fn a_score_card_arriving_after_debut_does_not_backfill() {
    let (mut game, _rin, ally, _non_mus) = debut_with(&[]);
    assert_eq!(
        game.state.mods.get_blade_modifier(ally),
        0,
        "precondition: debuted with an empty success zone"
    );

    game.state
        .player1
        .success_live_card_zone
        .add_card(game.id(MUS_SCORE));
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(ally),
        0,
        "登場 evaluated the condition once at debut; a card arriving afterwards does \
         not retroactively grant the ability"
    );
}

/// Two qualifying cards still grant the constant once — it is a flat +1 per μ's
/// member, not scaled by how many score-icon lives are in the zone.
#[test]
fn two_mus_score_cards_do_not_double_the_constant() {
    let (mut game, _rin, ally, _non_mus) = debut_with(&[MUS_SCORE, "PL!-pb1-029-SRL"]);

    assert_eq!(
        game.state.mods.get_blade_modifier(ally),
        1,
        ">gaining the ability is not per qualifying card — two score-icon lives are \
         still one 常時 worth of +1"
    );
}
