//! Regression scope for S1 energy-placed watchers (Ren bp7-005 ab#1 /
//! bp7-016 ab#0) vs the 〜か compound (Sumire bp5-004 ab#0).
//!
//! Printed rule for Ren: 「自分のカードの効果によって、
//! 自分のエネルギー置き場にエネルギーが置かれたとき」→ blade ×1.
//! It must fire ONLY on an own-effect arrival into the energy zone.
//!
//! Every test below drives the watcher with a REAL card ability resolved
//! through the engine — no hand-pushed movement events:
//! - Shiki PL!SP-bp2-008-R 起動 swaps Ren/Sumire across areas (own effect,
//!   zero energy moved).
//! - Setsuna-tutor PL!N-bp3-007-R 起動 parks a zone energy under the debut
//!   member via its parking prompt (not an effect-driven zone arrival).
//! - Kahori PL!SP-pb1-005-R 登場 places a waited energy deck → zone.
//!
//! (Mixed-cause batches need no pin: a single effect resolution has a single
//! causer and the batch is cleared per resolution, so they are unreachable
//! through real abilities. Opponent-cause rejection stays pinned by
//! `trigger_scope_test::s1_bp7_005_opponent_caused_placement_does_not_fire`.)

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const REN: &str = "PL!SP-bp7-005-R＋";
const SHIKI: &str = "PL!SP-bp2-008-R";
const TUTOR: &str = "PL!N-bp3-007-R";
const SETSUNA: &str = "PL!N-PR-009-PR";
const ENERGY: &str = "LL-E-001-SD";

fn blade(game: &TestGame, cid: i16) -> i32 {
    game.state.mods.get_blade_modifier(cid)
}

/// Activate Shiki's swap kidou and pick the area holding `target_area_holder`.
/// Returns after the move resolved.
fn shiki_swap_to(game: &mut TestGame, shiki: i16, want_area: &str) {
    game.activate_ability(shiki);
    game.drain_auto_ability_choices();
    assert!(
        game.has_pending_choice(),
        "Shiki swap should offer target areas"
    );
    let actions = game.generated_actions();
    let idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .is_some_and(|area| area == want_area || area == &format!("{want_area}_side"))
        })
        .expect("swap target area not offered");
    game.select_generated(idx);
    game.drain_auto_ability_choices();
}

/// Shiki swaps Ren across areas (own effect, no energy moved) → NO blade.
///
/// Pre-fix this fired: the moves-branch `area_ok || energy_ok` disjunction
/// armed the pure-energy watcher on the area move alone.
#[test]
fn own_effect_area_swap_does_not_trigger_energy_placement_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.new_id(REN);
    let shiki = game.new_id(SHIKI);
    // Ren staged directly (no debut → her ab#0 cascade stays out of this).
    game.state.player1.stage.stage = [ren, -1, -1];
    game.give_energy(10); // 9 to stage Shiki + E for his kidou
    game.add_to_hand(shiki);
    game.try_play_to_stage(shiki, MemberArea::Center)
        .expect("shiki debut");
    scan_autos_both(&mut game);
    assert_eq!(blade(&game, ren), 0, "no legit trigger before the swap");

    // Shiki Center → Left: Ren (at Left) is swapped to Center by our effect.
    shiki_swap_to(&mut game, shiki, "left");
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.stage.stage,
        [shiki, ren, -1],
        "swap must have moved Ren (Center) via our own effect"
    );
    assert_eq!(
        blade(&game, ren),
        0,
        "pure-energy watcher: own area move alone must not grant blade"
    );
}

/// Setsuna-tutor parks a zone energy under the debut member (choice-attributed
/// parking step) → NO blade for Ren.
///
/// Pins the effect_only requirement end to end: parking energy under a member
/// is not an effect-driven zone arrival, so the watcher must stay silent
/// (this held pre-fix too — the choice path records effect_only=false).
/// The dest-aware match additionally guards the helper-level conflation
/// where under_member arrivals counted as zone placements.
#[test]
fn energy_parked_under_member_does_not_trigger_energy_zone_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.new_id(REN);
    let tutor = game.new_id(TUTOR);
    let target = game.new_id(SETSUNA);
    game.state.player1.stage.stage = [ren, -1, tutor];
    game.state.player1.hand.cards.push(target);
    game.give_energy(2); // EE cost
    game.state
        .player1
        .energy_zone
        .cards
        .push(game.new_id(ENERGY)); // the park candidate

    game.activate_ability(tutor);
    // Single-candidate debut auto-resolves, then the parking prompt:
    assert!(
        game.has_pending_choice(),
        "energy-under-member SelectCard prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the tutor's parking step"
    );
    game.select_indices(&[0]);
    scan_autos_both(&mut game);

    assert_eq!(
        game.state.player1.stage.stage[2], target,
        "Setsuna must have debuted to the vacated RightSide"
    );
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::RightSide)
            .len(),
        1,
        "tutor effect really parked 1 zone energy under the member"
    );
    assert_eq!(
        blade(&game, ren),
        0,
        "under_member parking is not a zone arrival: no blade"
    );
}


