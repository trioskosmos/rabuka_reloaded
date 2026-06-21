use crate::helpers::*;
use rabuka_engine::game_state::AbilityTrigger;

const CERAS_AUTO: &str = "{{jidou.png|自動}}{{turn1.png|ターン1回}}自分のステージに『EdelNote』のメンバーが登場したとき、相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。";

fn trigger_ceras_auto(game: &mut TestGame) {
    let ability_id = format!("PL!HS-bp6-007-R_{}", CERAS_AUTO);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Auto,
        "player1".to_string(),
        Some("PL!HS-bp6-007-R".to_string()),
        None,
        None,
        None,
    );
    game.state.process_pending_auto_abilities("player1");
}

fn setup_ceras_appearance(game: &mut TestGame, p2_active_count: usize) -> (i16, Vec<i16>) {
    let ceras = game.new_id("PL!HS-bp6-007-R");
    game.state.player1.stage.stage[0] = ceras;
    game.state.recently_moved_cards = Some(vec![ceras]);

    let mut p2_members = Vec::new();
    for i in 0..p2_active_count {
        let p2_member = game.new_id("PL!-sd1-010-SD");
        game.state.player2.stage.stage[i] = p2_member;
        p2_members.push(p2_member);
    }

    let filler = game.new_id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.current_phase = rabuka_engine::game_state::Phase::Main;
    (ceras, p2_members)
}

fn is_wait(game: &TestGame, id: i16) -> bool {
    game.state
        .mods
        .get_orientation_modifier(id)
        .is_some_and(|o| o == "wait")
}

/// Q250 core: when the EdelNote member appears on stage,
/// its auto ability triggers and puts 1 opponent active member into wait.
#[test]
fn q250_self_appearance_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (_, p2_members) = setup_ceras_appearance(&mut game, 1);
    let p2_member = p2_members[0];

    assert!(!is_wait(&game, p2_member), "Member starts active");
    trigger_ceras_auto(&mut game);
    assert!(!game.has_pending_choice(), "1 target → auto-resolved");
    assert!(is_wait(&game, p2_member), "Opponent member put into wait");
}

/// No appearance (card on stage, no recently_moved) → no trigger.
#[test]
fn q250_no_appearance_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ceras = game.new_id("PL!HS-bp6-007-R");
    game.state.player1.stage.stage[0] = ceras;
    game.state.current_phase = rabuka_engine::game_state::Phase::Main;

    trigger_ceras_auto(&mut game);

    assert!(!game.has_pending_choice());
}

/// 2 active members → opponent selects which one to put into wait.
#[test]
fn q250_opponent_selects_which_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (_, p2_members) = setup_ceras_appearance(&mut game, 2);
    let m0 = p2_members[0];
    let m1 = p2_members[1];

    assert!(!is_wait(&game, m0), "m0 active");
    assert!(!is_wait(&game, m1), "m1 active");

    trigger_ceras_auto(&mut game);

    assert!(game.has_pending_choice(), "Opponent picks which to wait");
    game.select_indices(&[1]);
    assert!(!game.has_pending_choice());

    assert!(!is_wait(&game, m0), "m0 stays active");
    assert!(is_wait(&game, m1), "m1 put into wait");
}

/// Use limit (1/turn): same copy cannot trigger twice in one turn.
#[test]
fn q250_use_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (ceras, first) = setup_ceras_appearance(&mut game, 2);
    trigger_ceras_auto(&mut game);
    assert!(game.has_pending_choice(), "First trigger works");
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());

    // Re-trigger with SAME copy ID — use_limit (1/turn) blocks it
    game.state.recently_moved_cards = Some(vec![ceras]);
    trigger_ceras_auto(&mut game);
    assert!(
        !game.has_pending_choice(),
        "Second trigger blocked — same copy used 1/turn"
    );
}
