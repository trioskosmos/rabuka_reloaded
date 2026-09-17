use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const SHION: &str = "PL!S-bp6-001-R";
const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn pl_s_bp6_001_r_hand_debut_leaves_low_cost_opponent_unwaited() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(SHION);
    let opp_member = game.id(FILLER);
    game.state.player2.stage.stage[0] = opp_member;
    game.state.player1.hand.cards.push(member);
    game.give_energy(4);
    game.play_to_stage(member, MemberArea::Center);
    scan_autos_both(&mut game);
    assert!(
        game.state.player1.stage.stage.contains(&member),
        "Shion debuted from hand"
    );
    let opp_orientation = game.state.mods.get_orientation_modifier(opp_member);
    assert!(
        opp_orientation.is_none(),
        "appearance from hand (not graveyard) must not trigger the wait"
    );
}
