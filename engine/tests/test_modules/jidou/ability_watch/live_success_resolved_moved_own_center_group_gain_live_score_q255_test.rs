use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::types::PositionChangeEvent;

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

#[test]
fn live_success_resolution_score_unmoved_resolver_scores_nothing() {
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

#[test]
fn live_success_resolution_score_q255_moved_before_resolution_scores() {
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

#[test]
fn live_success_resolution_score_non_mus_resolver_no_score_even_with_mus_staged_and_moved() {
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

#[test]
fn live_success_resolution_score_once_per_turn_single_score_for_two_resolvers() {
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
