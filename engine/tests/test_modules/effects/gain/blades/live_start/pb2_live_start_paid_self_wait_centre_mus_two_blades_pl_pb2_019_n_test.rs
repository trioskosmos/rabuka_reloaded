//! 高坂穂乃果 `PL!-pb2-019-N` (ライブ開始時)
//!
//! このメンバーをウェイトにし、手札を1枚控え室に置いてもよい：
//! ライブ終了時まで、自分のセンターエリアにいる『μ's』のメンバーは、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。
//! （ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。）
//!
//! The whole effect is behind an optional cost. The existing
//! `live_start_paid_self_wait_center_member_blades_test.rs` covers the paid path for
//! the identically-worded `PL!-bp4-011-N`; this file covers the rest of the range —
//! declining, the area filter, the group filter, and the cost's own resource check.
//!
//! The trailing parenthetical is a rules clarification about the yell, not an
//! effect, and is not asserted as one. See docs/PBLL02_ABILITY_REVIEW.md.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const HONOKA: &str = "PL!-pb2-019-N";
/// μ's members for the centre slot.
const MUS_CENTER: &str = "PL!-sd1-001-SD";
const MUS_SIDE: &str = "PL!-sd1-007-SD";
/// Not μ's.
const AZALEA: &str = "PL!S-bp5-012-N";

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("card {} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

/// Stage the card, fire ライブ開始時, and answer the optional-cost gate with `choice`.
#[test]
fn paying_grants_two_blades_to_the_centre_member() {
    let mut game = TestGame::new(load_real_database());
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id(HONOKA);
    let centre = game.id(MUS_CENTER);
    let side = game.id(MUS_SIDE);
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = me;
    game.state.player1.stage.stage[MemberArea::Center as usize] = centre;
    game.state.player1.stage.stage[MemberArea::RightSide as usize] = side;
    game.state.player1.hand.cards.push(filler);

    fire_live_start(&mut game, me);
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "the second cost step is a hand discard, so the prompt is a card selection"
    );
    assert!(
        game.get_pending_choice().allow_skip(),
        "「置いてもよい」 — the cost is optional, so the discard prompt must be \
         skippable rather than forced"
    );
    game.select_indices(&[0]);

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        Some("wait"),
        "this member をウェイト is part of the cost and must actually be paid"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(centre),
        2,
        "センターエリア — the centre μ's member gains exactly +2"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(side),
        0,
        "not the centre — a μ's member in another area gains nothing"
    );
}

#[test]
fn declining_grants_nothing_and_keeps_the_member_active() {
    let mut game = TestGame::new(load_real_database());
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id(HONOKA);
    let centre = game.id(MUS_CENTER);
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = me;
    game.state.player1.stage.stage[MemberArea::Center as usize] = centre;
    game.state.player1.hand.cards.push(filler);

    let hand_before = game.state.player1.hand.cards.len();
    fire_live_start(&mut game, me);
    // An empty selection is the decline on a skippable card prompt.
    game.select_indices(&[]);

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        None,
        "declining the optional cost must not wait her — rule 9.4.2.2 skips the \
         WHOLE cost, including the self-wait deferred ahead of the choice"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(centre),
        0,
        "置いてもよい — the effect is optional, so declining grants nothing"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "declining must not discard a card"
    );
}

/// The group filter: a non-μ's member in the centre does not benefit even when the
/// cost is paid.
#[test]
fn a_non_mus_centre_member_gains_nothing() {
    let mut game = TestGame::new(load_real_database());
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id(HONOKA);
    let centre = game.id(AZALEA);
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = me;
    game.state.player1.stage.stage[MemberArea::Center as usize] = centre;
    game.state.player1.hand.cards.push(filler);

    fire_live_start(&mut game, me);
    game.select_option(1);

    assert_eq!(
        game.state.mods.get_blade_modifier(centre),
        0,
        "『μ's』のメンバー — an AZALEA member in the centre is not eligible"
    );
}

/// With no μ's member in the centre the effect still resolves, it just has no target.
#[test]
fn no_centre_member_is_harmless() {
    let mut game = TestGame::new(load_real_database());
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id(HONOKA);
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = me;
    game.state.player1.hand.cards.push(filler);

    fire_live_start(&mut game, me);
    assert!(game.has_pending_choice(), "the cost is still offered");
    game.select_indices(&[0]);
    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "her own left-side slot is not the centre, so she gains nothing"
    );
}
