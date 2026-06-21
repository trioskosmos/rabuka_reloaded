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

fn setup_ceras_appearance(game: &mut TestGame, p2_active_count: usize) -> i16 {
    let ceras = game.new_id("PL!HS-bp6-007-R");
    game.state.player1.stage.stage[0] = ceras;
    game.state.recently_moved_cards = Some(vec![ceras]);

    for i in 0..p2_active_count {
        let p2_member = game.new_id("PL!-sd1-010-SD");
        game.state.player2.stage.stage[i] = p2_member;
    }

    let filler = game.new_id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.current_phase = rabuka_engine::game_state::Phase::Main;
    ceras
}

/// Q250 core: when the EdelNote member itself appears on stage,
/// its own auto ability triggers (opponent selects an active member to wait).
#[test]
fn q250_self_appearance_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_ceras_appearance(&mut game, 1);

    trigger_ceras_auto(&mut game);

    assert!(
        game.has_pending_choice(),
        "Auto ability triggers when Ceras herself appears — opponent picks a member to wait"
    );
}

/// No appearance (card already on stage, no recently_moved) → no trigger.
#[test]
fn q250_no_appearance_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ceras = game.new_id("PL!HS-bp6-007-R");
    game.state.player1.stage.stage[0] = ceras;
    game.state.current_phase = rabuka_engine::game_state::Phase::Main;

    trigger_ceras_auto(&mut game);

    assert!(
        !game.has_pending_choice(),
        "No trigger when no recent appearance"
    );
}

/// Opponent has 2 active members → selects which one to put into wait.
/// The selected member should become wait, the other stays active.
#[test]
fn q250_opponent_selects_which_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    setup_ceras_appearance(&mut game, 2);

    trigger_ceras_auto(&mut game);

    assert!(
        game.has_pending_choice(),
        "Opponent chooses which member to wait"
    );
    game.select_indices(&[1]); // pick the second active member (index 1)

    assert!(!game.has_pending_choice(), "Choice resolved");

    // Opponent's selected member should be in wait
    let p2_stage_1 = game.state.player2.stage.stage[1];
    assert!(
        game.state.player2.waitroom.cards.contains(&p2_stage_1),
        "Selected member moved to waitroom"
    );
    // The other member should remain active on stage
    assert_ne!(
        game.state.player2.stage.stage[0], -1,
        "Unselected member stays on stage"
    );
}

/// Use limit (1/turn): second trigger in the same turn is blocked.
#[test]
fn q250_use_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // First trigger
    setup_ceras_appearance(&mut game, 1);
    trigger_ceras_auto(&mut game);
    assert!(game.has_pending_choice(), "First trigger works");
    game.select_indices(&[0]); // pick the active member
    assert!(!game.has_pending_choice(), "First trigger resolved");

    // Second trigger — same turn, blocked by 1/turn
    let ceras2 = game.new_id("PL!HS-bp6-007-R");
    game.state.player1.stage.stage[1] = ceras2;
    game.state.recently_moved_cards = Some(vec![ceras2]);

    trigger_ceras_auto(&mut game);
    assert!(
        !game.has_pending_choice(),
        "Second auto trigger blocked by use_limit (1/turn)"
    );
}
