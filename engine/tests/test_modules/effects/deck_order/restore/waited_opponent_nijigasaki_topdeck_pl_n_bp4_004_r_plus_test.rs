use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| {
            a.triggers
                .as_deref()
                .is_some_and(|t| t.contains(trigger_str))
        })
        .expect("card should have the requested trigger ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn pl_n_bp4_004_r_plus_live_start_waited_opponents_recover_nijigasaki_member_to_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-bp4-004-R＋");

    let opp_a = game.new_id(FILLER);
    let opp_b = game.new_id(FILLER);
    let opp_active = game.new_id(FILLER);
    game.state.player2.stage.stage = [opp_a, opp_b, opp_active];
    game.state.mods.add_orientation_modifier(opp_a, "wait");
    game.state.mods.add_orientation_modifier(opp_b, "wait");
    game.state.player1.stage.stage[1] = karin;

    let niji_a = game.id("PL!N-bp3-002-R");
    let niji_b = game.new_id("PL!N-bp3-002-R");
    let niji_c = game.new_id("PL!N-bp3-002-R");
    game.state
        .player1
        .waitroom
        .cards
        .extend_from_slice(&[niji_a, niji_b, niji_c]);

    trigger_auto(&mut game, karin, AbilityTrigger::LiveStart, "ライブ開始時");

    assert!(
        game.has_pending_choice(),
        "selection from the waitroom should be asked"
    );
    game.select_indices(&[0, 1]);

    let deck_top: Vec<_> = game
        .state
        .player1
        .main_deck
        .cards
        .iter()
        .take(2)
        .copied()
        .collect();
    assert!(
        deck_top.contains(&niji_a) || deck_top.contains(&niji_b),
        "selected members are placed on top of the deck"
    );
}
