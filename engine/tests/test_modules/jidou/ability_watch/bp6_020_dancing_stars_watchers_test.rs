//! Dancing stars on me! (PL!-bp6-020-L) — 自動 ability-watcher pair (Q255).
//!
//! ab#0: 自動 ターン1回 — when a 『μ's』 member in OWN center resolves a
//!   ライブ開始時 ability → position-change THAT member.
//! ab#1: 自動 ターン1回 — when a 『μ's』 member in OWN center resolves a
//!   ライブ成功時 ability → if that member moved this turn → +1 score to
//!   THIS live card.
//!
//! Q255 (2026.06.02): the watcher STILL triggers when the member has already
//! moved away from center by resolution time — "in center" cannot be checked
//! as a hard position gate at resolution time; the triggering member's GROUP
//! identity is what matters.
//!
//! Driving idiom: `fire_trigger` pushes the member's real LS/LSS ability
//! through the real ability queue, so the post-resolution each_time hook
//! (trigger_each_time_for_member) arms the watcher exactly like a live phase.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::types::PositionChangeEvent;

const LS: &str = "ライブ開始時";
const LSS: &str = "ライブ成功時";

fn setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let ds = game.id("PL!-bp6-020-L");
    game.state.player1.live_card_zone.cards.push(ds);
    ds
}

fn score_of(game: &TestGame, ds: i16) -> i32 {
    game.state
        .mods
        .score_modifiers
        .get(&ds)
        .map(|m| m.total())
        .unwrap_or(0)
}

/// Manually move `cid` from one stage slot to another WITHOUT going through
/// an ability — simulates an earlier effect's reposition this turn
/// (toubatsu_q263 idiom).
fn manually_move(game: &mut TestGame, cid: i16, from: usize, to: usize) {
    game.state.player1.stage.stage[from] = -1;
    game.state.player1.stage.stage[to] = cid;
    game.state.position_change_events.push(PositionChangeEvent {
        moved_card_id: cid,
        old_position: from as u8,
        new_position: to as u8,
        cause_card_id: None,
        cause_player_id: "p1".to_string(),
        effect_only: false,
    });
    game.state.record_card_movement(cid);
    game.state
        .push_movement_event(cid, "stage", "stage", None, "p1", false);
    game.state.position_change_occurred_this_turn = true;
}

/// Answer the watcher's mandatory position|destination prompt with `area`.
fn choose_destination(game: &mut TestGame, area: &str) {
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "watcher must ask for a position|destination"
    );
    let actions = game.generated_actions();
    let idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                == Some(area)
        })
        .unwrap_or_else(|| {
            panic!(
                "no '{area}' destination option; available: {:?}",
                actions.iter().filter_map(|a| {
                    a.parameters.as_ref().and_then(|p| p.stage_area.clone())
                }).collect::<Vec<_>>()
            )
        });
    game.select_generated(idx);
}

// ====================================================================
// ab#0 — ライブ開始時 watcher
// ====================================================================

/// Positive: center μ's member resolves LS → watcher repositions THE
/// RESOLVING member (not another staged μ's member).
#[test]
fn bp6020_ab0_center_mus_live_start_repositions_resolving_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ds = setup(&mut game);
    let honoka = game.id("PL!-bp6-001-R＋"); // μ's, has LS+LSS
    game.state.player1.stage.stage[1] = honoka;

    fire_trigger(&mut game, honoka, AbilityTrigger::LiveStart, LS);

    choose_destination(&mut game, "left");
    assert_eq!(
        game.state.player1.stage.stage[0], honoka,
        "the RESOLVING member (Honoka) is the one repositioned"
    );
    assert_eq!(game.state.player1.stage.stage[1], -1, "center now empty");
    assert!(
        game.state.has_card_moved_this_turn(honoka),
        "reposition counts as a move this turn"
    );
    assert_eq!(score_of(&game, ds), 0, "ab#0 alone grants no score");
    assert!(
        !game.has_pending_choice(),
        "no residual prompts after the reposition"
    );
}

/// Q255: member ALREADY off center when her LS resolves → watcher still fires.
#[test]
fn bp6020_ab0_q255_member_already_off_center_still_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let _ds = setup(&mut game);
    let honoka = game.id("PL!-bp6-001-R＋");
    game.state.player1.stage.stage[1] = honoka;
    manually_move(&mut game, honoka, 1, 0); // center -> left

    fire_trigger(&mut game, honoka, AbilityTrigger::LiveStart, LS);

    choose_destination(&mut game, "center");
    assert_eq!(
        game.state.player1.stage.stage[1], honoka,
        "Q255: watcher fired despite resolver being off center at resolution time"
    );
}

/// Negative: NON-μ's member resolves LS (and pays her optional cost so the
/// ability FULLY executes) while a μ's member stands elsewhere on stage —
/// the naive board-count reading of the group condition would fire the
/// watcher here; the printed text (μ's member's LS resolving) forbids it.
#[test]
fn bp6020_ab0_non_mus_resolver_does_not_reposition_anything() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let _ds = setup(&mut game);
    let chika = game.id("PL!S-PR-013-PR"); // Aqours, LS with optional {E}{E}
    let honoka = game.id("PL!-bp6-001-R＋");
    game.state.player1.stage.stage[1] = chika;
    game.state.player1.stage.stage[0] = honoka; // μ's bait for a naive count
    game.give_energy(4);

    fire_trigger(&mut game, chika, AbilityTrigger::LiveStart, LS);

    // Chika's own LS: pay the optional {E}{E} so it fully resolves
    // (options[1] = pay per WRITING_TESTS §H).
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "Chika's optional cost prompt expected"
    );
    game.select_option(1);

    assert!(
        !game.has_pending_choice(),
        "NON-μ's resolver must NOT arm the watcher — no destination prompt allowed"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], chika,
        "Chika stays in center"
    );
    assert_eq!(
        game.state.player1.stage.stage[0], honoka,
        "Honoka was never moved"
    );
}

/// Negative: OPPONENT's center μ's member resolving LS must not arm OUR
/// watcher (text says 自分のステージ; the hook scans only the resolving
/// player's live zone).
#[test]
fn bp6020_ab0_opponent_member_resolution_does_not_arm_own_watcher() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let _ds = setup(&mut game);
    let p2_honoka = game.id("PL!-bp6-001-R＋");
    game.state.player2.stage.stage[1] = p2_honoka;

    // Fire P2's Honoka LS through P2's own queue (mirror of helpers::fire_trigger).
    let ability_id = {
        let card = game.db.get_card(p2_honoka).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some(LS))
            .expect("P2 Honoka lacks LS");
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(p2_honoka).unwrap().card_no.to_string();
    let pid = game.state.player2.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(p2_honoka),
        None,
        None,
    );
    game.state.activating_card = Some(p2_honoka);
    game.state.process_pending_auto_abilities(&pid);

    assert!(
        !game.has_pending_choice(),
        "opponent-stage resolution must not trigger our watcher"
    );
    assert_eq!(
        game.state.player2.stage.stage[1], p2_honoka,
        "P2's member not repositioned"
    );
    assert!(
        game.state.player1.stage.stage.iter().all(|&c| c == -1),
        "our stage untouched"
    );
}

/// ターン1回: a second LS resolution in the SAME turn must not re-trigger.
#[test]
fn bp6020_ab0_once_per_turn_blocks_second_resolution() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let _ds = setup(&mut game);
    let honoka = game.id("PL!-bp6-001-R＋");
    game.state.player1.stage.stage[1] = honoka;

    fire_trigger(&mut game, honoka, AbilityTrigger::LiveStart, LS);
    choose_destination(&mut game, "left");

    // Deliberate re-fire in the same turn (helpers::fire_trigger contract).
    fire_trigger(&mut game, honoka, AbilityTrigger::LiveStart, LS);
    assert!(
        !game.has_pending_choice(),
        "ターン1回: second LS resolution same turn must not re-open the destination prompt"
    );
    assert_eq!(
        game.state.player1.stage.stage[0], honoka,
        "Honoka stayed at left — no second reposition"
    );
}

// ====================================================================
// ab#1 — ライブ成功時 watcher (conditional score+1)
// ====================================================================

/// Negative: μ's member resolves LSS but did NOT move this turn → no score.
#[test]
fn bp6020_ab1_no_move_this_turn_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ds = setup(&mut game);
    let honoka = game.id("PL!-bp6-001-R＋");
    game.state.player1.stage.stage[1] = honoka;

    fire_trigger(&mut game, honoka, AbilityTrigger::LiveSuccess, LSS);

    assert!(
        !game.has_pending_choice(),
        "nothing to prompt: her LSS condition misses (empty reveal) and the watcher's has_moved gate fails"
    );
    assert_eq!(
        score_of(&game, ds),
        0,
        "has_moved gate: unmoved resolver → no score"
    );
}

/// Q255 positive: member moved OFF center earlier this turn, then her LSS
/// resolves → +1 score to Dancing stars on me! itself.
#[test]
fn bp6020_ab1_q255_moved_before_resolution_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ds = setup(&mut game);
    let honoka = game.id("PL!-bp6-001-R＋");
    game.state.player1.stage.stage[1] = honoka;
    manually_move(&mut game, honoka, 1, 0); // center -> left

    fire_trigger(&mut game, honoka, AbilityTrigger::LiveSuccess, LSS);

    assert!(!game.has_pending_choice());
    assert_eq!(
        score_of(&game, ds),
        1,
        "Q255: moved-this-turn μ's resolver + LSS resolved → exactly +1 on the live card"
    );
}

/// Negative: NON-μ's member resolves LSS while a μ's member stands elsewhere
/// on stage AND the resolver HAS moved — group identity of the RESOLVER is
/// what gates, not mere presence of μ's on the board.
#[test]
fn bp6020_ab1_non_mus_resolver_no_score_even_with_mus_staged_and_moved() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ds = setup(&mut game);
    let karin = game.id("PL!N-bp5-016-N"); // 虹ヶ咲, LSS: draw 1, discard 1
    let honoka = game.id("PL!-bp6-001-R＋");
    game.state.player1.stage.stage[1] = karin;
    game.state.player1.stage.stage[0] = honoka; // μ's bait for a naive count
    manually_move(&mut game, karin, 1, 0);

    fire_trigger(&mut game, karin, AbilityTrigger::LiveSuccess, LSS);

    // Karin's own LSS runs (draw + discard prompt) — answer hers, strictly.
    game.drain_choices_strict(&["SelectCard"], &[0]);

    assert!(!game.has_pending_choice(), "no watcher prompts expected");
    assert_eq!(
        score_of(&game, ds),
        0,
        "resolver is 虹ヶ咲, not μ's → watcher must not score even though she moved"
    );
}

/// ターン1回 on ab#1: TWO μ's members each resolve LSS (both moved) → exactly
/// ONE +1 total.
#[test]
fn bp6020_ab1_once_per_turn_single_score_for_two_resolvers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ds = setup(&mut game);
    let honoka = game.id("PL!-bp6-001-R＋");
    let kotori = game.id("PL!-bp6-003-R＋"); // μ's, LS+LSS
    game.state.player1.stage.stage[1] = honoka;
    game.state.player1.stage.stage[2] = kotori;
    manually_move(&mut game, honoka, 1, 0); // honoka: center -> left

    fire_trigger(&mut game, honoka, AbilityTrigger::LiveSuccess, LSS);
    assert_eq!(score_of(&game, ds), 1, "first resolution scores");

    // Kotori moves right -> center, then HER LSS resolves same turn.
    manually_move(&mut game, kotori, 2, 1);
    fire_trigger(&mut game, kotori, AbilityTrigger::LiveSuccess, LSS);
    // Kotori's LSS is an optional debut from under-cards; nothing is under
    // her, so it should skip without prompts. Fail loudly if it does prompt.
    assert!(
        !game.has_pending_choice(),
        "Kotori's optional LSS should auto-skip with nothing under her"
    );

    assert_eq!(
        score_of(&game, ds),
        1,
        "ターン1回: second μ's LSS resolution same turn adds nothing"
    );
}
