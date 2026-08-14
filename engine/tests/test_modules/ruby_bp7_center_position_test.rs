/// 黒澤ルビィ (PL!S-bp7-018-N) ab#0 — 登場:
///   自分のステージにいるメンバー1人をセンターエリアにポジションチェンジさせる。
///
/// The parser produces `position_change` with `target_member: select` and a fixed
/// destination of `center`. The player picks ANY member on their own stage
/// (including 黒澤ルビィ herself / the activating card) and that member moves to
/// the center area (swapping with whatever is there).
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn filler_id(game: &TestGame) -> i16 {
    game.id("PL!-sd1-010-SD")
}

fn ruby_id(game: &TestGame) -> i16 {
    game.id("PL!S-bp7-018-N")
}

/// Fires 黒澤ルビィ's 登場 (debut) ability, answers the member-selection choice by
/// picking the stage position given, and drains any follow-up auto choices.
fn fire_ruby_debut_and_move(game: &mut TestGame, ruby: i16, pick_area: MemberArea) {
    game.play_to_stage(ruby, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectTarget { options, .. } => {
                let area_str = match pick_area {
                    MemberArea::LeftSide => "left",
                    MemberArea::Center => "center",
                    MemberArea::RightSide => "right",
                };
                if let Some(opts) = options {
                    let idx = opts
                        .iter()
                        .position(|o| o.contains(area_str) || o.ends_with(area_str))
                        .expect("member choice should offer the requested area");
                    game.select_option(idx as i16);
                } else {
                    game.select_option(0);
                }
            }
            rabuka_engine::ability::types::Choice::SelectPosition { .. } => {
                let area_str = match pick_area {
                    MemberArea::LeftSide => "left",
                    MemberArea::Center => "center",
                    MemberArea::RightSide => "right",
                };
                let actions = game.generated_actions();
                let idx = actions
                    .iter()
                    .position(|a| {
                        a.parameters
                            .as_ref()
                            .and_then(|p| p.stage_area.as_deref())
                            .is_some_and(|area| area == area_str)
                    })
                    .expect("position choice should offer requested area");
                game.select_generated(idx);
            }
            _ => game.select_indices(&[0]),
        }
    }
    game.drain_auto_ability_choices();
}

#[test]
fn ruby_choice_offers_self_and_other_stage_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = ruby_id(&game);
    let member_a = filler_id(&game);
    let member_b = filler_id(&game);
    game.state.player1.stage.stage = [member_a, member_b, -1];
    game.add_to_hand(ruby);
    game.give_energy(10);

    game.play_to_stage(ruby, MemberArea::Center);
    // Ruby's debut must present a member-selection choice.
    let choice = game.get_pending_choice();
    match choice {
        rabuka_engine::ability::types::Choice::SelectTarget { options, .. } => {
            let opts = options.as_ref().expect("options expected");
            assert!(
                opts.len() >= 2,
                "should be able to choose between the two stage members, got {:?}",
                opts
            );
        }
        rabuka_engine::ability::types::Choice::SelectPosition { .. } => {
            // position-based selection is also acceptable
        }
        other => panic!("expected member-selection choice, got {:?}", other),
    }
}

#[test]
fn ruby_move_left_member_to_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = ruby_id(&game);
    let left = filler_id(&game);
    let right = filler_id(&game);
    // Stage: [left, center=ruby's slot, right]. Ruby will debut to center.
    game.state.player1.stage.stage = [left, -1, right];
    game.add_to_hand(ruby);
    game.give_energy(10);

    fire_ruby_debut_and_move(&mut game, ruby, MemberArea::LeftSide);

    let stage = &game.state.player1.stage.stage;
    assert_eq!(stage[1], left, "left member moved to center");
    // The member that was at center (ruby) ends up where left was.
    assert_eq!(stage[0], ruby, "ruby swapped to left");
    assert_eq!(stage[2], right, "right member untouched");
}

#[test]
fn ruby_move_right_member_to_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = ruby_id(&game);
    let left = filler_id(&game);
    let right = filler_id(&game);
    game.state.player1.stage.stage = [left, -1, right];
    game.add_to_hand(ruby);
    game.give_energy(10);

    fire_ruby_debut_and_move(&mut game, ruby, MemberArea::RightSide);

    let stage = &game.state.player1.stage.stage;
    assert_eq!(stage[1], right, "right member moved to center");
    assert_eq!(stage[0], left, "left member untouched");
    assert_eq!(stage[2], ruby, "ruby swapped to right");
}

/// The activating card (黒澤ルビィ herself) may be chosen to move to center.
/// If she is already at center the move is a no-op.
#[test]
fn ruby_can_choose_self_when_not_already_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = ruby_id(&game);
    let left = filler_id(&game);
    let right = filler_id(&game);
    // Ruby debuts to LEFT (not center). The choice should still let you pick her
    // and move her to center.
    game.state.player1.stage.stage = [-1, left, right];
    game.add_to_hand(ruby);
    game.give_energy(10);

    game.play_to_stage(ruby, MemberArea::LeftSide);
    // Choose ruby herself (the left member) to move to center.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectTarget { options, .. } => {
                let idx = options
                    .as_ref()
                    .and_then(|o| o.iter().position(|x| x.contains("left") || x.ends_with("left")))
                    .expect("should be able to pick the left (self) member");
                game.select_option(idx as i16);
            }
            rabuka_engine::ability::types::Choice::SelectPosition { .. } => {
                let actions = game.generated_actions();
                let idx = actions
                    .iter()
                    .position(|a| {
                        a.parameters
                            .as_ref()
                            .and_then(|p| p.stage_area.as_deref())
                            .is_some_and(|area| area == "left")
                    })
                    .expect("should offer left");
                game.select_generated(idx);
            }
            _ => game.select_indices(&[0]),
        }
    }
    game.drain_auto_ability_choices();

    let stage = &game.state.player1.stage.stage;
    assert_eq!(stage[1], ruby, "ruby moved to center");
    assert_eq!(stage[0], left, "left filler swapped out to where ruby was");
}

/// Choosing a member already at center is a no-op move.
#[test]
fn ruby_center_member_noop() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = ruby_id(&game);
    let left = filler_id(&game);
    let right = filler_id(&game);
    game.state.player1.stage.stage = [left, -1, right];
    game.add_to_hand(ruby);
    game.give_energy(10);

    // Ruby debuts to center; choose the CENTER member (ruby) -> no-op.
    game.play_to_stage(ruby, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectTarget { options, .. } => {
                let idx = options
                    .as_ref()
                    .and_then(|o| {
                        o.iter().position(|x| x.contains("center") || x.ends_with("center"))
                    })
                    .expect("should be able to pick center");
                game.select_option(idx as i16);
            }
            rabuka_engine::ability::types::Choice::SelectPosition { .. } => {
                let actions = game.generated_actions();
                let idx = actions
                    .iter()
                    .position(|a| {
                        a.parameters
                            .as_ref()
                            .and_then(|p| p.stage_area.as_deref())
                            .is_some_and(|area| area == "center")
                    })
                    .expect("should offer center");
                game.select_generated(idx);
            }
            _ => game.select_indices(&[0]),
        }
    }
    game.drain_auto_ability_choices();

    let stage = &game.state.player1.stage.stage;
    assert_eq!(stage[1], ruby, "ruby stays at center");
    assert_eq!(stage[0], left, "left untouched");
    assert_eq!(stage[2], right, "right untouched");
}
