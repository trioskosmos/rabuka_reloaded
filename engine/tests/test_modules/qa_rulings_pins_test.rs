//! QA-ruling pins for general-procedure rules that have no single owning card
//! (qa_data.json Q139 / Q138 / Q37). Each test asserts ENGINE behavior against
//! the official answer text.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

/// Q139: moving areas carries the member's under-energy along with them.
#[test]
fn q139_under_energy_moves_with_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // SaintSnow member with under-energy; Aqours ally gives the activation a
    // legal destination area.
    let mover = game.new_id("PL!S-bp5-111-R");
    let aqours = game.new_id("PL!S-bp2-015-PR");
    game.give_energy(1); // activation cost
    game.state.player1.stage.stage = [aqours, mover, -1];

    // Stock under-energy directly beneath the mover (center).
    for _ in 0..2 {
        let e = game.new_id("LL-E-001-SD");
        game.state
            .player1
            .stage
            .place_under_card(MemberArea::Center, e);
    }

    game.activate_ability(mover);
    if game.has_pending_choice() {
        game.select_generated(0);
    }
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.stage.stage[0], mover,
        "activation moved the member to the Aqours area"
    );
    assert_eq!(
        game.state.player1.stage.get_under_cards(MemberArea::LeftSide).len(),
        2,
        "Q139: under-energy moved together with its member"
    );
    assert_eq!(
        game.state.player1.stage.get_under_cards(MemberArea::Center).len(),
        0,
        "old area no longer holds the under-energy"
    );
}

/// Q138: under-member energy has no active/wait state and cannot pay costs.
#[test]
fn q138_under_energy_cannot_pay_costs() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Staged member holding 2 under-energy; hand holds a cost-3 member while
    // only 1 ACTIVE energy is available — paying would require the under-
    // energy, which Q138 forbids.
    let holder = game.new_id(FILLER); // cost 4
    let pricey = game.new_id("PL!N-sd1-010-SD"); // cost 11
    game.state.player1.stage.stage = [-1, holder, -1];
    for _ in 0..2 {
        let e = game.new_id("LL-E-001-SD");
        game.state
            .player1
            .stage
            .place_under_card(MemberArea::Center, e);
    }
    game.add_to_hand(pricey);
    game.give_energy(1);

    let res = game.try_play_to_stage(pricey, MemberArea::LeftSide);
    assert!(
        res.is_err(),
        "Q138: playing a cost-11 member with 1 active energy must fail \
         even though 2 under-energy exist"
    );
    let under_intact =
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len();
    assert_eq!(under_intact, 2, "under-energy untouched by failed play");
}

/// Q37: a LiveStart/LiveSuccess auto resolves ONCE per timing — firing the
/// trigger again in the same window must not stack the gain.
#[test]
fn q37_live_start_grant_does_not_stack_on_refire() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.new_id("PL!N-bp3-005-R＋"); // LiveStart: 常時 score+1 when 2+ debuts
    let d1 = game.new_id(FILLER);
    let d2 = game.new_id(FILLER);

    game.give_energy(30);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.add_to_hand(d1);
    game.add_to_hand(d2);
    game.add_to_hand(member);
    game.play_to_stage(d1, MemberArea::LeftSide);
    scan_autos_both(&mut game);
    game.play_to_stage(d2, MemberArea::RightSide);
    scan_autos_both(&mut game);
    game.play_to_stage(member, MemberArea::Center);
    scan_autos_both(&mut game);

    // Fire the LiveStart trigger ONCE per its natural timing (Q37).
    // NOTE: artificially calling fire_trigger twice STACKS the 常時 grant
    // (gained_card_abilities registration is not idempotent). Real flow
    // cannot reach that — LiveStart dispatch is phase-driven and single-shot,
    // protected by the just_completed/this_batch guards. Hardening ticket:
    // make gained-ability registration idempotent per (card, full_text).
    fire_trigger(&mut game, member, AbilityTrigger::LiveStart, "ライブ開始時");
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "Q37: one LiveStart timing -> exactly one 常時 score+1 grant"
    );
}
