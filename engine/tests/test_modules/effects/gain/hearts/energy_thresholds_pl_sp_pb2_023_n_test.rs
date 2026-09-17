use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_sp_pb2_023_n_energy_thresholds_stack_heart02_at_six_and_eight() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = game.id("PL!SP-pb2-023-N");
    game.add_to_stage(MemberArea::Center, kanon);
    game.give_energy(5);
    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kanon, HeartColor::Heart02),
        0
    );
    game.give_energy(2);
    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kanon, HeartColor::Heart02),
        1,
        "energy 7 ≥ 6 → one heart02"
    );
    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(kanon, HeartColor::Heart02),
        2,
        "energy 8 ≥ 8 → さらに one more heart02 (total 2)"
    );
}
