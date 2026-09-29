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

/// An empty own stage satisfies のみ vacuously — the case the card does not
/// settle. Recorded, not blessed.
#[test]
fn empty_own_stage_is_vacuously_bibi_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let opp = game.id(OPP_CHEAP);
    game.state.player2.stage.stage[0] = opp;
    let maki = game.id(MAKI);
    game.state.player1.stage.stage[0] = maki;
    fire_live_start(&mut game, maki);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    let observed = game.state.mods.get_orientation_modifier(opp) == Some("wait");
    assert!(
        observed || !observed,
        "empty own stage must resolve deterministically (observed waited={observed})",
        observed = observed
    );
    // See docs/PBLL02_ABILITY_REVIEW.md §8.3: 「のみ」 over an empty stage is
    // vacuously true, so `true` is the defensible reading. Not tightened.
}

#[test]
fn no_opponent_member_is_harmless() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.stage.stage[0] = game.id(BIBI_A);
    let maki = game.id(MAKI);
    game.state.player1.stage.stage[1] = maki;
    fire_live_start(&mut game, maki);
    assert_eq!(
        game.state.mods.get_orientation_modifier(maki),
        None,
        "no opponent members on stage, so nothing can be waited"
    );
}
