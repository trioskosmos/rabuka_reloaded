use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";

fn pl_s_bp7_025_l_two_opponents_board(game: &mut TestGame) -> i16 {
    let live = game.id("PL!S-bp7-025-L");
    game.state.player1.live_card_zone.cards.push(live);
    let o1 = game.new_id(FILLER);
    let o2 = game.new_id(FILLER);
    game.state.player2.stage.stage[0] = o1;
    game.state.player2.stage.stage[1] = o2;
    live
}

#[test]
fn pl_s_bp7_025_l_choose_wait_sets_delayed_activation_blocks() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = pl_s_bp7_025_l_two_opponents_board(&mut game);
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice(), "option choice expected");
    game.select_option(0);
    let mut picked = false;
    for _ in 0..3 {
        if !game.has_pending_choice() {
            break;
        }
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                game.select_indices(&[0, 1]);
                picked = true;
            }
            _ => game.select_indices(&[]),
        }
    }
    assert!(picked, "the wait-target selection must be offered");
    for i in 0..2usize {
        let opp = game.state.player2.stage.stage[i];
        assert_eq!(game.state.mods.get_orientation_modifier(opp), Some("wait"), "opponent member {i} waited");
        assert!(game.state.mods.is_delayed_cannot_active(opp), "…and will NOT activate next turn");
    }
}

#[test]
fn pl_s_bp7_025_l_choose_draw_leaves_opponent_unmodified() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    let live = pl_s_bp7_025_l_two_opponents_board(&mut game);
    let o1 = game.state.player2.stage.stage[0];
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(game.has_pending_choice());
    game.select_option(1);
    assert_eq!(game.state.player1.hand.cards.len(), 1, "option B → draw 1");
    assert_eq!(game.state.mods.get_orientation_modifier(o1), None, "opponent untouched by option B");
}
