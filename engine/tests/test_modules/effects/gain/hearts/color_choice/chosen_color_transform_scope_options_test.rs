use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

fn pl_sp_pb2_030_n_setup_color_transform(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.id("PL!SP-pb2-030-N");
    game.state.player1.stage.stage[1] = me;
    let bystander = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[0] = bystander;
    me
}

#[test]
fn pl_sp_pb2_030_n_first_option_transforms_original_hearts_to_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = pl_sp_pb2_030_n_setup_color_transform(&mut game);
    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.select_option(0);
    assert_eq!(
        game.state.mods.heart_color_multiplier.get(&me).copied(),
        Some(HeartColor::Heart02),
        "chose heart02 -> original hearts become heart02"
    );
}

#[test]
fn pl_sp_pb2_030_n_third_option_transforms_only_self_to_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = pl_sp_pb2_030_n_setup_color_transform(&mut game);
    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.select_option(2);
    assert_eq!(
        game.state.mods.heart_color_multiplier.get(&me).copied(),
        Some(HeartColor::Heart06),
        "chose heart06 -> original hearts become heart06"
    );
    assert_eq!(
        game.state
            .mods
            .heart_color_multiplier
            .get(&game.id_ref("PL!-sd1-010-SD"))
            .copied(),
        None,
        "bystander untouched"
    );
}
