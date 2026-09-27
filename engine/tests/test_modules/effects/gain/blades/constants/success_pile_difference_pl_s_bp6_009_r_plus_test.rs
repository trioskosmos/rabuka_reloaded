use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;

const LIVE_FILLER: &str = "PL!-sd1-019-SD";
const CONTROL_LIVE: &str = "PL!S-bp2-026-L";
const AQOURS_SCORE_ICON_LIVE: &str = "PL!S-bp2-024-L";
const AQOURS_NO_SCORE_ICON_LIVE: &str = "PL!S-bp2-026-L";
const NON_AQOURS_SCORE_ICON_LIVE: &str = "PL!SP-sd1-023-SD";

#[test]
fn pl_s_bp6_009_r_plus_constant_blades_follow_opponent_success_pile_lead() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp6-009-R＋");
    let opponent_lives: [i16; 3] = std::array::from_fn(|_| game.new_id(LIVE_FILLER));

    game.state.player1.stage.stage[1] = ruby;

    game.state
        .player2
        .success_live_card_zone
        .cards
        .extend(opponent_lives);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        3,
        "diff 3 → ブレード3"
    );

    let own_lives: [i16; 2] = std::array::from_fn(|_| game.new_id(LIVE_FILLER));
    game.state
        .player1
        .success_live_card_zone
        .cards
        .extend(own_lives);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        1,
        "diff 1 → ブレード1"
    );
}

#[test]
fn pl_s_bp6_009_r_plus_constant_blades_apply_to_each_instance() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby_left = game.id("PL!S-bp6-009-R＋");
    let ruby_center = game.new_id("PL!S-bp6-009-R＋");
    let opponent_lives: [i16; 3] = std::array::from_fn(|_| game.new_id(LIVE_FILLER));
    game.state.player1.stage.stage = [ruby_left, ruby_center, -1];
    game.state
        .player2
        .success_live_card_zone
        .cards
        .extend(opponent_lives);

    game.state.recalculate_constants();

    assert_eq!(game.state.mods.get_blade_modifier(ruby_left), 3);
    assert_eq!(game.state.mods.get_blade_modifier(ruby_center), 3);
}

fn fire_ruby_live_success(game: &mut TestGame, live_card: i16) {
    let mut heart_map = HeartMap::new();
    heart_map.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts: heart_map });
    game.state.player1.live_card_zone.cards.push(live_card);
    game.state.current_phase = Phase::LiveVictoryDetermination;
    // The wildcard hearts and the phase above OPEN the ライブ成功時 window this
    // dispatches through. Asserted so a failure here is reported as "the window is
    // shut" rather than as whatever the success-pile difference happens to read.
    assert!(
        game.state.should_trigger_live_success(&game.state.player1),
        "precondition: the ライブ成功時 window must be OPEN, or the dispatch below \
         does nothing at all"
    );
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");
}

#[test]
fn ruby_center_live_success_scores_revealed_aqours_score_icon_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp6-009-R＋");
    let live_card = game.id(CONTROL_LIVE);
    let revealed = game.id(AQOURS_SCORE_ICON_LIVE);
    assert_eq!(game.db.get_card(ruby).unwrap().card_no, "PL!S-bp6-009-R＋");
    game.state.player1.stage.stage[1] = ruby;
    game.state.revealed_cards.push(revealed);

    fire_ruby_live_success(&mut game, live_card);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    assert_eq!(game.state.mods.get_score_modifier(live_card), 0);
}

#[test]
fn ruby_off_center_live_success_does_not_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp6-009-R＋");
    let live_card = game.id(CONTROL_LIVE);
    let revealed = game.id(AQOURS_SCORE_ICON_LIVE);
    game.state.player1.stage.stage[0] = ruby;
    game.state.revealed_cards.push(revealed);

    fire_ruby_live_success(&mut game, live_card);

    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);
}

#[test]
fn ruby_live_success_requires_one_revealed_card_to_match_every_filter() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp6-009-R＋");
    let live_card = game.id(CONTROL_LIVE);
    let aqours_without_score_icon = game.id(AQOURS_NO_SCORE_ICON_LIVE);
    let non_aqours_with_score_icon = game.id(NON_AQOURS_SCORE_ICON_LIVE);
    game.state.player1.stage.stage[1] = ruby;
    game.state
        .revealed_cards
        .extend([aqours_without_score_icon, non_aqours_with_score_icon]);

    fire_ruby_live_success(&mut game, live_card);

    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);
}
