use crate::helpers::*;
use rabuka_engine::core::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;

fn setup_live_success(game: &mut TestGame, revealed: &[i16]) -> i16 {
    let live = game.new_id("PL!S-bp7-022-L");
    let mut stage_hearts = BaseHeart {
        hearts: HeartMap::new(),
    };
    stage_hearts.hearts.insert(HeartColor::Heart02, 3);
    stage_hearts.hearts.insert(HeartColor::Heart04, 3);
    stage_hearts.hearts.insert(HeartColor::Heart05, 3);
    stage_hearts.hearts.insert(HeartColor::Heart00, 10);
    game.state.player1.stage_hearts = Some(stage_hearts);
    game.state.player1.live_card_zone.cards.push(live);
    game.state.revealed_cards.extend_from_slice(revealed);
    game.state.current_phase = Phase::LiveVictoryDetermination;
    // The wildcard hearts, the zoned live card and the phase above are what OPEN
    // the ライブ成功時 window both tests dispatch through. Asserted here, once, so
    // `aquarium_live_success_non_aqours_hearts_do_not_score` — a NEGATIVE — cannot
    // pass on a closed window and be read as the group filter working.
    assert!(
        game.state.should_trigger_live_success(&game.state.player1),
        "precondition: the ライブ成功時 window must be OPEN, or the dispatch in the \
         callers below does nothing and the negative is vacuous"
    );
    live
}

#[test]
fn aquarium_live_success_aqours_hearts_score_plus_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let revealed = game.new_id("PL!S-sd1-001-SD");
    let live = setup_live_success(&mut game, &[revealed]);

    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(game.state.mods.get_score_modifier(live), 1);
}

#[test]
fn aquarium_live_success_non_aqours_hearts_do_not_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rina = game.new_id("PL!N-PR-026-PR");
    let rika = game.new_id("PL!N-bp1-021-N");
    let live = setup_live_success(&mut game, &[rina, rika]);

    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(game.state.mods.get_score_modifier(live), 0);
}
