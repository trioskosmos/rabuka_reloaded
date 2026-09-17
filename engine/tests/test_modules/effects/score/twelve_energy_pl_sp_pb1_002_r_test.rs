use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_sp_pb1_002_r_twelve_energy_grants_total_score_bonus_but_eleven_does_not() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = game.id("PL!SP-pb1-002-R");
    game.add_to_stage(MemberArea::Center, keke);
    game.give_energy(11);
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);
    game.give_energy(1);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "エネルギーが12枚以上 → live total +1"
    );
}
