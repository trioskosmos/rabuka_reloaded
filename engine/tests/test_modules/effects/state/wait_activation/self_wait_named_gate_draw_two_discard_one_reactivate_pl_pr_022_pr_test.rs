//! 西木野真姫 `PL!-PR-022-PR` (起動・ターン1回)
//!
//! このメンバーをウェイトにする：自分のステージに「星空 凛」か「小泉花陽」がいる場合、
//! カードを2枚引き、手札を1枚控え室に置く。2人ともいる場合、さらにこのメンバーを
//! アクティブにする。
//!
//! Three gates, all of them on the printed names:
//!  * the draw needs 星空 凛 OR 小泉花陽 on stage;
//!  * 「2人とも」needs BOTH, and only both;
//!  * the wait cost is paid either way — it is a cost, not a conditional.
//!
//! The reactivation condition parses as a bare `stage count >= 2`, so a stage of
//! three members who are neither named character is the shape that would light
//! it up wrongly.

use crate::helpers::*;

const MAKI: &str = "PL!-PR-022-PR";
/// 星空 凛 — abilityless print.
const RIN: &str = "PL!-pb1-023-N";
/// 小泉花陽 — abilityless print.
const HANAYO: &str = "PL!-pb1-026-N";
/// Abilityless members whose names are neither of the two.
const FILLER_A: &str = "PL!-pb1-022-N";
const FILLER_B: &str = "PL!-bp5-018-N";
const DECK_FILLER: &str = "PL!-sd1-010-SD";

fn setup(with: &[(&str, i32)]) -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state.player1.hand.cards.clear();
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        let f = game.id(DECK_FILLER);
        game.state.player1.main_deck.cards.push(f);
    }
    let mut stage = [-1i16; 3];
    stage[0] = game.id(MAKI);
    for (card_no, slot) in with {
        stage[*slot as usize] = game.id(card_no);
    }
    game.state.player1.stage.stage = stage;
    game
}

fn wait_state(game: &TestGame, cid: i16) -> Option<String> {
    game.state
        .mods
        .get_orientation_modifier(cid)
        .map(|s| s.to_string())
}

/// The draw raises a 「手札を1枚控え室に置く」 selection before the rest of the
/// ability resolves, so every assertion below runs after the board has settled.
/// Loop on the TYPE, not on pendency (EDGE_CASE_FINDINGS C9): the 「手札を1枚控え室に
/// 置く」 selection is the only prompt this fixture raises, and anything else ends the
/// loop so the trailing assert can name it.
fn drain(game: &mut TestGame) {
    while game.pending_choice_type().as_deref() == Some("SelectCard") {
        game.select_indices(&[0]);
    }
    assert!(
        !game.has_pending_choice(),
        "maki's ability left {:?} queued",
        game.pending_choice_type()
    );
}

fn maki_of(game: &TestGame) -> i16 {
    game.state.player1.stage.stage[0]
}

/// Neither named character is present: the cost still waits maki, and nothing draws.
#[test]
fn no_named_member_on_stage_pays_the_wait_cost_and_draws_nothing() {
    let mut game = setup(&[(FILLER_A, 1), (FILLER_B, 2)]);
    let maki = maki_of(&game);

    game.activate_ability(maki);
    drain(&mut game);

    assert_eq!(
        wait_state(&game, maki),
        Some("wait".to_string()),
        "このメンバーをウェイトにする is a COST — it is paid even when the gate fails"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "ステージに「星空 凛」か「小泉花陽」がいない — no draw"
    );
}

/// Exactly one of the two names: the draw fires, and maki stays waited.
#[test]
fn one_named_member_draws_two_and_discards_one_then_stays_waited() {
    let mut game = setup(&[(RIN, 1)]);
    let maki = maki_of(&game);

    game.activate_ability(maki);
    drain(&mut game);

    assert_eq!(
        wait_state(&game, maki),
        Some("wait".to_string()),
        "片方しかいない — the reactivation branch 「2人とも」does not run"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "2枚引き、手札を1枚控え室に置く — net +1 from an empty hand"
    );
}

/// A stage of three members, none of them the named two — the bare
/// `stage count >= 2` reading of 「2人とも」must not reactivate her.
#[test]
fn three_unnamed_members_do_not_satisfy_2人とも() {
    let mut game = setup(&[(FILLER_A, 1), (FILLER_B, 2)]);
    let maki = maki_of(&game);
    assert_eq!(
        game.state.player1.stage.stage.iter().filter(|&&s| s != -1).count(),
        3,
        "precondition: the stage is full, which is what a count-only gate would read"
    );

    game.activate_ability(maki);
    drain(&mut game);

    assert_eq!(
        wait_state(&game, maki),
        Some("wait".to_string()),
        "「2人とも」is 「星空 凛」と「小泉花陽」の2人 — three unnamed members are not them"
    );
}

/// Both names present: draw 2, discard 1, then reactivate.
#[test]
fn both_named_members_reactivate_maki() {
    let mut game = setup(&[(RIN, 1), (HANAYO, 2)]);
    let maki = maki_of(&game);

    game.activate_ability(maki);
    drain(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "the draw still happens on the both-present branch"
    );
    assert_ne!(
        wait_state(&game, maki),
        Some("wait".to_string()),
        "2人ともいる場合、さらにこのメンバーをアクティブにする"
    );
}

/// ターン1回 — a second activation in the same turn is refused.
#[test]
fn only_one_activation_per_turn() {
    let mut game = setup(&[(RIN, 1)]);
    let maki = maki_of(&game);

    game.activate_ability(maki);
    drain(&mut game);
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "precondition: the first activation drew"
    );

    let second = game.try_activate_ability(maki);
    assert!(
        second.is_err(),
        "{{turn1}} — the second activation of the same turn must be refused, got {second:?}"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "the refused activation must not draw"
    );
}
