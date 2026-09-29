//! 西木野真姫 `PL!-pb2-006-PP` / `-R` (ライブ開始時, and the identical-effect 起動)
//!
//! {{live_start.png|ライブ開始時}}このメンバーをウェイトにし、手札を1枚控え室に置いてもよい：
//! 相手のステージにいる元々持つハートの数が1つ以下のメンバー1人をウェイトにする。
//!
//! Three things to pin, each on its own axis:
//!   * 「元々持つ」 — the printed heart total, not the current one.
//!   * 「1つ以下」 — an inclusive ceiling, so 0 and 1 qualify and 2 does not.
//!   * 「メンバー1人」 — exactly one, and only on the OPPONENT's stage.
//!
//! The cost is the optional sequential from rule 9.4.2.2: declining must skip the
//! whole cost, including the self-wait.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const MAKI: &str = "PL!-pb2-006-PP";
/// 0 printed hearts — qualifies.
const OPP_0: &str = "PL!-bp3-012-N";
/// 1 printed heart — the inclusive boundary, qualifies.
const OPP_1: &str = "PL!-PR-014-PR";
/// 2 printed hearts — one over the ceiling, must not qualify.
const OPP_2: &str = "PL!-bp6-017-N";

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

/// Build: our Maki centre, opponent stages given by (card_no, slot), hand seeded.
fn setup(opponent: &[&str]) -> (TestGame, i16, Vec<i16>) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id(MAKI);
    game.assert_card_identity(me, MAKI);
    game.state.player1.stage.stage[MemberArea::Center as usize] = me;
    let ids: Vec<i16> = opponent.iter().map(|c| game.id(c)).collect();
    for (i, &id) in ids.iter().enumerate() {
        game.state.player2.stage.stage[i] = id;
    }
    // Enough hand to pay: the ability card is not a hand card, so seed spares.
    for _ in 0..3 {
        game.state.player1.hand.cards.push(filler);
    }
    (game, me, ids)
}

/// Pay the optional cost, leaving whatever target prompt follows on the table.
fn fire_and_pay(game: &mut TestGame, me: i16) {
    fire_live_start(game, me);
    // First prompt is the cost's hand discard; the hand has only identical filler
    // so index 0 is unambiguous.
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "the cost step is a hand discard"
    );
    game.select_indices(&[0]);
}

#[test]
fn a_zero_printed_heart_opponent_member_is_waited() {
    let (mut game, me, ids) = setup(&[OPP_0]);
    fire_and_pay(&mut game, me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(ids[0]),
        Some("wait"),
        "元々持つハートの数が0 — 0つ以下 qualifies, so the opponent member is waited"
    );
}

#[test]
fn a_one_printed_heart_opponent_member_is_waited() {
    let (mut game, me, ids) = setup(&[OPP_1]);
    fire_and_pay(&mut game, me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(ids[0]),
        Some("wait"),
        "1つ以下 is an INCLUSIVE ceiling, so exactly 1 printed heart qualifies"
    );
}

#[test]
fn a_two_printed_heart_opponent_member_is_never_offered() {
    let (mut game, me, ids) = setup(&[OPP_2]);
    fire_and_pay(&mut game, me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(ids[0]),
        None,
        "2つ — one over the ceiling, so this member is not an eligible target and \
         must be left active"
    );
}

/// The heart filter and the 1人 count together: with a 0-heart and a 2-heart member
/// on board, only the 0-heart one is selectable, and only it is waited.
#[test]
fn only_the_qualifying_member_is_affected() {
    let (mut game, me, ids) = setup(&[OPP_0, OPP_2]);
    fire_and_pay(&mut game, me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(ids[0]),
        Some("wait"),
        "the 0-heart member is eligible and is waited"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(ids[1]),
        None,
        "the 2-heart member is over the ceiling and stays active"
    );
}

/// 「メンバー1人」 — one only. With several eligible members the player must choose.
#[test]
fn exactly_one_is_chosen_from_several_eligible() {
    let (mut game, me, ids) = setup(&[OPP_0, OPP_1]);
    fire_and_pay(&mut game, me);

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "two eligible members and メンバー1人 — the player picks one"
    );
    assert_eq!(
        game.pending_choice_count(),
        1,
        "メンバー1人 — exactly one selection, not both"
    );
    game.select_indices(&[0]);

    let waited = ids
        .iter()
        .filter(|&&id| game.state.mods.get_orientation_modifier(id) == Some("wait"))
        .count();
    assert_eq!(waited, 1, "precisely one opponent member ends up waited");
}

/// 「相手のステージ」 — our own members are never targets, even low-heart ones.
#[test]
fn our_own_low_heart_member_is_not_a_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id(MAKI);
    let ally = game.id(OPP_0); // 0 printed hearts, on OUR side
    game.state.player1.stage.stage[MemberArea::Center as usize] = me;
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = ally;
    for _ in 0..3 {
        game.state.player1.hand.cards.push(filler);
    }

    fire_and_pay(&mut game, me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(ally),
        None,
        "相手のステージ — our own 0-heart member is not an eligible target"
    );
}

/// No eligible opponent member: the effect resolves without a target prompt.
#[test]
fn no_eligible_opponent_member_is_harmless() {
    let (mut game, me, ids) = setup(&[OPP_2]);
    fire_and_pay(&mut game, me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(ids[0]),
        None,
        "only a 2-heart member is on the opponent's stage, so nothing is waited"
    );
}

/// Declining the optional cost skips the WHOLE cost, self-wait included.
#[test]
fn declining_skips_the_self_wait_and_the_target_prompt() {
    let (mut game, me, ids) = setup(&[OPP_0]);
    let hand_before = game.state.player1.hand.cards.len();

    fire_live_start(&mut game, me);
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "the optional cost offers a hand discard"
    );
    assert!(
        game.get_pending_choice().allow_skip(),
        "置いてもよい — the discard must be skippable"
    );
    game.select_indices(&[]);

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        None,
        "rule 9.4.2.2 — declining skips the whole cost, so she is not waited"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(ids[0]),
        None,
        "declining means no opponent member is waited either"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "declining must not discard a card"
    );
}
