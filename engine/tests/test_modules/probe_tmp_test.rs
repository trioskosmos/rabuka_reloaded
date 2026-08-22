use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn probe_sumire_actions() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp4-004-R＋");
    let big_a = game.id("PL!S-bp5-009-R");
    let big_b = game.id("PL!HS-bp6-006-R＋");

    game.state.player1.stage.stage = [big_a, big_b, -1];
    game.add_to_hand(sumire);
    game.give_energy(3);

    let acts = rabuka_engine::game_setup::generate_possible_actions(&game.state);
    for a in &acts {
        if a.action_type == rabuka_engine::game_setup::ActionType::PlayMemberToStage {
            eprintln!(
                "[PROBE] desc='{}' card_id={:?} base_cost={:?}",
                a.description,
                a.parameters.as_ref().and_then(|p| p.card_id),
                a.parameters.as_ref().and_then(|p| p.base_cost)
            );
        }
    }
    eprintln!(
        "[PROBE] total actions={} hand={:?}",
        acts.len(),
        game.state.player1.hand.cards
    );
}
