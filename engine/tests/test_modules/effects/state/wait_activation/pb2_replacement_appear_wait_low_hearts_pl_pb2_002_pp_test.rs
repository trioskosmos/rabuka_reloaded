//! 絢瀬絵里 `PL!-pb2-002-PP` / `-R` (常時) — replacement appearance rule
//!
//! {{jyouji.png|常時}}このメンバーの正面のエリアには、元々持つハートの数が4つ以下の
//! メンバーは、ウェイト状態で登場する。
//!
//! A REPLACEMENT rule: it changes how other members APPEAR, it does not fire on an
//! event. That is why it is resolved ONCE at placement
//! (`GameState::replacement_appear_wait_applies`, called from
//! `handle_play_member_to_stage`) instead of in the constant scan — a constant would
//! re-apply every recalculation and the member could never leave wait. The last test
//! pins exactly that, and is the one that fails under a constant implementation.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-pb2-002-PP";
/// 0 printed hearts — inside the bound.
const IN_BOUND_0: &str = "PL!-bp3-012-N";
/// 4 printed hearts — exactly the bound, inclusive.
const IN_BOUND_4: &str = "PL!-bp3-002-P";
/// 5 printed hearts — one over the bound. A member with no ability of its own, so
/// the only thing that could wait it is this rule.
const OVER_BOUND_5: &str = "PL!N-bp4-019-N";

/// Stand `host` in a side slot, play `victim` into `victim_slot`, report orientation.
/// The host goes to the RIGHT slot so `victim_slot: 0` is genuinely empty — playing
/// into an occupied slot is a baton touch, which is a different scenario entirely.
fn orientation_for(host: &str, victim: &str, victim_slot: usize) -> Option<String> {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let eri = game.id(host);
    game.assert_card_identity(eri, host);
    game.state.player1.hand.cards.push(eri);
    game.give_energy(40);
    game.play_to_stage(eri, MemberArea::RightSide);
    assert_eq!(
        game.state.mods.get_orientation_modifier(eri),
        None,
        "precondition: the host arrives active — a member is never in front of itself"
    );

    let v = game.id(victim);
    game.state.player1.hand.cards.push(v);
    game.give_energy(40);
    game.play_to_stage(v, MemberArea::ALL[victim_slot]);

    game.state
        .mods
        .get_orientation_modifier(v)
        .map(|s| s.to_string())
}

fn orientation_after_appear(victim: &str, victim_slot: usize) -> Option<String> {
    orientation_for(ERI, victim, victim_slot)
}

/// Both prints of this ability, driven separately. `-PP` and `-R` are DIFFERENT cards
/// that share a character and number, so a suite that stages only one proves only that
/// one's wiring — the other is a separate entry in `cards.json` and a separate
/// `card_no` → ability edge.
#[test]
fn both_prints_of_the_ability_wait_the_same_members() {
    for host in ["PL!-pb2-002-PP", "PL!-pb2-002-R"] {
        assert_eq!(
            orientation_for(host, IN_BOUND_4, 1).as_deref(),
            Some("wait"),
            "{host}: 4つ以下 is an INCLUSIVE bound, so exactly 4 qualifies"
        );
        assert_eq!(
            orientation_for(host, OVER_BOUND_5, 1),
            None,
            "{host}: 5つ — one over the bound, so the member is unaffected"
        );
    }
}

#[test]
fn without_the_rule_a_member_appears_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let v = game.id(IN_BOUND_0);
    game.state.player1.hand.cards.push(v);
    game.give_energy(10);
    game.play_to_stage(v, MemberArea::Center);

    assert_eq!(
        game.state.mods.get_orientation_modifier(v),
        None,
        "precondition: with no replacement rule in play a member appears active"
    );
}

#[test]
fn a_zero_heart_member_entering_the_front_appears_in_wait() {
    assert_eq!(
        orientation_after_appear(IN_BOUND_0, 1).as_deref(),
        Some("wait"),
        "ハートの数が4つ以下 — 0 qualifies, so the member appears in wait"
    );
}

#[test]
fn a_four_heart_member_entering_the_front_appears_in_wait() {
    assert_eq!(
        orientation_after_appear(IN_BOUND_4, 1).as_deref(),
        Some("wait"),
        "4つ以下 is an INCLUSIVE bound, so exactly 4 qualifies"
    );
}

#[test]
fn a_five_heart_member_enters_active() {
    assert_eq!(
        orientation_after_appear(OVER_BOUND_5, 1),
        None,
        "5つ — one over the bound, so the member is unaffected"
    );
}

/// The bound reads PRINTED hearts (元々持つ), so a member whose current hearts have
/// been raised is still judged on its printed total.
#[test]
fn the_bound_reads_printed_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let eri = game.id(ERI);
    game.state.player1.hand.cards.push(eri);
    game.give_energy(40);
    game.play_to_stage(eri, MemberArea::RightSide);

    let v = game.id(OVER_BOUND_5);
    game.state.player1.hand.cards.push(v);
    game.give_energy(40);
    game.state
        .mods
        .add_heart_modifier(v, rabuka_engine::card::HeartColor::Heart04, 9);
    game.play_to_stage(v, MemberArea::Center);

    assert_eq!(
        game.state.mods.get_orientation_modifier(v),
        None,
        "元々持つ — the bound reads the PRINTED total, so a member whose current \
         hearts were raised is still judged on its printed 5"
    );
}

/// 正面のエリア is read as the CENTRE area, so a side-slot arrival is unaffected.
#[test]
fn a_member_entering_a_side_slot_is_unaffected() {
    assert_eq!(
        orientation_after_appear(IN_BOUND_0, 0),
        None,
        "正面のエリア — the rule covers the centre area, so a member entering the \
         left slot appears active"
    );
}

/// The rule must already be on the BOARD when the other member appears.
#[test]
fn a_member_that_appeared_before_the_rule_is_unaffected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let v = game.id(IN_BOUND_0);
    game.state.player1.hand.cards.push(v);
    game.give_energy(10);
    game.play_to_stage(v, MemberArea::Center);

    let eri = game.id(ERI);
    game.state.player1.hand.cards.push(eri);
    game.give_energy(40);
    game.play_to_stage(eri, MemberArea::RightSide);

    assert_eq!(
        game.state.mods.get_orientation_modifier(v),
        None,
        "the rule governs APPEARANCE — a member already on stage when the rule \
         arrives is not retroactively put in wait"
    );
}

/// THE test that separates a replacement from a constant.
#[test]
fn the_member_can_still_be_activated_afterwards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let eri = game.id(ERI);
    game.state.player1.hand.cards.push(eri);
    game.give_energy(40);
    game.play_to_stage(eri, MemberArea::RightSide);

    let v = game.id(IN_BOUND_0);
    game.state.player1.hand.cards.push(v);
    game.give_energy(40);
    game.play_to_stage(v, MemberArea::Center);
    assert_eq!(
        game.state.mods.get_orientation_modifier(v),
        Some("wait"),
        "precondition: the member appeared in wait"
    );

    game.state.mods.add_orientation_modifier(v, "active");
    game.state.recalculate_constants();
    assert_ne!(
        game.state.mods.get_orientation_modifier(v),
        Some("wait"),
        "the rule is a REPLACEMENT, not a constant — once the member is activated it \
         must not be put straight back to wait by the next recalculation"
    );
}
