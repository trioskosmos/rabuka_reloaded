use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    fire_trigger_nth(game, cid, trigger, trig, 0);
}

fn fire_trigger_nth(
    game: &mut TestGame,
    cid: i16,
    trigger: AbilityTrigger,
    trig: &str,
    nth: usize,
) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .filter(|a| a.triggers.as_deref() == Some(trig))
            .nth(nth)
            .unwrap_or_else(|| panic!("card {} lacks '{trig}' ability #{nth}", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

#[test]
fn pl_n_bp4_023_n_optional_member_wait_gates_draw_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = game.id("PL!N-bp4-023-N");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.add_to_stage(MemberArea::Center, mia);
    let niji = game.id("PL!N-PR-019-PR");
    game.add_to_stage(MemberArea::LeftSide, niji);
    let keep = game.id("PL!N-PR-012-PR");
    game.add_to_hand(keep);
    let hand_before = game.state.player1.hand.cards.len();
    fire_trigger(&mut game, mia, AbilityTrigger::Debut, "登場");
    assert!(
        game.has_pending_choice(),
        "optional wait-cost offer expected on debut"
    );
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(0),
        Some("SelectCard") => game.select_indices(&[]),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "declined cost → no draw/discard"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(niji),
        None,
        "member untouched when declined"
    );
    let sacrifice = game.new_id(FILLER);
    game.add_to_hand(sacrifice);
    fire_trigger(&mut game, mia, AbilityTrigger::Debut, "登場");
    assert!(game.has_pending_choice(), "cost offer expected");
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(1),
        Some("SelectCard") => game.select_indices(&[0]),
        other => panic!("unexpected {other:?}"),
    }
    for _ in 0..3 {
        if !game.has_pending_choice() {
            break;
        }
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. } if zone == "stage" => {
                let idx = game
                    .state
                    .player1
                    .stage
                    .stage
                    .iter()
                    .position(|&c| c == niji)
                    .expect("Niji member still on stage");
                game.select_indices(&[idx]);
            }
            _ => {
                let idx = game
                    .state
                    .player1
                    .hand
                    .cards
                    .iter()
                    .position(|&c| c == sacrifice)
                    .unwrap_or(0);
                game.select_indices(&[idx]);
            }
        }
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(niji),
        Some("wait"),
        "accepting waited the Niji member"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&sacrifice),
        "the discard step put the chosen card in the waitroom"
    );
    assert!(game.state.player1.hand.cards.contains(&keep));
}
