//! 西木野真姫 `PL!-pb2-024-N` (ライブ開始時)
//!
//! 自分のステージにいるメンバーが『BiBi』のみの場合、相手のステージにいるコスト2以下の
//! メンバー1人をウェイトにする。
//!
//! Two independent gates: stage exclusivity over BiBi, and a cost ceiling on the
//! target. Each negative below breaks exactly one of them.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const MAKI: &str = "PL!-pb2-024-N";
/// BiBi members with no ability, so nothing else contributes.
const BIBI_A: &str = "PL!-bp3-015-N";
const BIBI_B: &str = "PL!-bp5-018-N";
/// Not BiBi.
const NON_BIBI: &str = "PL!S-bp5-012-N";
/// Opponent members. The cost is the whole point of this card, so both are pinned:
/// `PL!SP-pb1-014-N` (cost 2) and `PL!SP-bp1-014-N` (cost 4) are different cards
/// with the same number and the same name, and picking the wrong one makes the
/// positive case pass or fail for the wrong reason.
const OPP_CHEAP: &str = "PL!SP-pb1-014-N"; // 嵐 千砂都, cost 2
const OPP_DEAR: &str = "PL!SP-bp1-014-N"; // 嵐 千砂都, cost 4

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

/// Stage `own` for player 1, `theirs` for player 2, fire ライブ開始時, and return
/// the opponent ids plus the game.
fn run(own: &[&str], theirs: &[&str]) -> (TestGame, Vec<i16>) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    for (i, c) in own.iter().enumerate() {
        game.state.player1.stage.stage[i] = game.id(c);
    }
    let ids: Vec<i16> = theirs.iter().map(|c| game.id(c)).collect();
    for (i, &id) in ids.iter().enumerate() {
        game.state.player2.stage.stage[i] = id;
    }
    let maki = game.id(MAKI);
    game.assert_card_identity(maki, MAKI);
    // The target gate is a cost test, so the target needs a printed cost. The group
    // pin is NOT applied here: one negative deliberately stages a non-BiBi member.
    for c in theirs {
        let id = game.id(c);
        assert!(
            game.db.get_card(id).and_then(|c| c.cost).is_some(),
            "{c} must be a member with a printed cost",
        );
    }
    game.state.player1.stage.stage[2] = maki;
    fire_live_start(&mut game, maki);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    (game, ids)
}

fn waited(game: &TestGame, ids: &[i16]) -> Vec<bool> {
    ids.iter()
        .map(|&id| game.state.mods.get_orientation_modifier(id) == Some("wait"))
        .collect()
}

#[test]
fn all_bibi_stage_waits_a_cost_two_opponent_member() {
    let db = load_real_database();
    let probe = TestGame::new(db.clone());
    // Pin the ally group: stage exclusivity is the first gate, so a fixture that is
    // not actually BiBi would make this pass for the wrong reason.
    for c in [BIBI_A, BIBI_B] {
        probe.assert_card_in_group(probe.id(c), "BiBi", "own-stage fixture");
    }

    let (game, ids) = run(&[BIBI_A, BIBI_B], &[OPP_CHEAP]);
    assert_eq!(
        game.db.get_card(ids[0]).and_then(|c| c.cost),
        Some(2),
        "precondition: the target is exactly on the コスト2以下 ceiling"
    );
    assert_eq!(
        waited(&game, &ids),
        vec![true],
        "『BiBi』のみ and コスト2以下 — both gates hold, so the target is waited"
    );
}

#[test]
fn all_bibi_stage_leaves_a_cost_four_opponent_member_alone() {
    let (game, ids) = run(&[BIBI_A, BIBI_B], &[OPP_DEAR]);
    assert_eq!(
        game.db.get_card(ids[0]).and_then(|c| c.cost),
        Some(4),
        "precondition: the target is just over the ceiling"
    );
    assert_eq!(
        waited(&game, &ids),
        vec![false],
        "コスト4 — over the コスト2以下 ceiling, so not an eligible target"
    );
}

/// Break the exclusivity half only: the target is cheap and eligible, but our own
/// stage is not purely BiBi.
#[test]
fn a_non_bibi_ally_blocks_the_effect() {
    let (game, ids) = run(&[BIBI_A, NON_BIBI], &[OPP_CHEAP]);
    assert_eq!(
        waited(&game, &ids),
        vec![false],
        "メンバーが『BiBi』のみ — a μ's and an AZALEA member on our stage break \
         exclusivity even though the target is eligible"
    );
}

/// のみ over a genuinely EMPTY own stage: vacuously true, so the wait happens.
///
/// This test previously asserted `observed || !observed`, which is `assert!(true)`
/// and could not fail under any engine — and it did not even build the scenario its
/// name claimed, because it staged MAKI in slot 0, leaving a one-BiBi-member stage.
/// The stage really is empty here: MAKI lives in the live card zone for a
/// ライブ開始時, and the ability is triggered directly.
///
/// The reading is pinned as `true` because のみ is a universal and the repository
/// already works this way: see `pb2_debut_wait_opponent_blade_two_or_less_pl_pb2_029_n_test.rs::an_empty_ally_slot_still_satisfies_exclusivity`,
/// which pins the same vacuous reading for 「『μ's』のみ」.
///
/// **If the rules are ever ruled the other way this test goes red, and that is the
/// point** — it is a report on an open question, not a shrug. See
/// docs/PBLL02_ABILITY_REVIEW.md §8.3 item 1.
#[test]
fn an_empty_own_stage_satisfies_bibi_only_vacuously() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let opp = game.id(OPP_CHEAP);
    game.state.player2.stage.stage[0] = opp;
    let maki = game.id(MAKI);
    assert_eq!(
        game.state.player1.stage.stage,
        [-1, -1, -1],
        "precondition: our own stage is genuinely empty, which is the case this test is about"
    );
    fire_live_start(&mut game, maki);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(opp),
        Some("wait"),
        "『のみ』 is a universal, so an empty stage satisfies it vacuously and the cost-2 \
         opponent member is waited"
    );
}

/// The cost ceiling is a real gate: an opponent member ABOVE コスト2 is not waited.
///
/// This previously asserted that MAKI — our own card — had no orientation
/// modifier. The ability targets the OPPONENT, so it could never have touched her
/// and the assertion said nothing about the engine. The negative now breaks the
/// cost gate on a card the ability actually selects.
#[test]
fn an_opponent_member_above_the_cost_bound_is_not_waited() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let bibi_a = game.id(BIBI_A);
    let opp = game.id(OPP_DEAR);
    game.assert_card_cost(opp, 4);
    game.state.player1.stage.stage[0] = bibi_a;
    game.state.player2.stage.stage[0] = opp;
    let maki = game.id(MAKI);
    fire_live_start(&mut game, maki);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(opp),
        None,
        "コスト2以下 — 嵐 千砂都 (cost 4) is above the bound, so she is not waited even \
         though our stage is exclusively BiBi"
    );
}
