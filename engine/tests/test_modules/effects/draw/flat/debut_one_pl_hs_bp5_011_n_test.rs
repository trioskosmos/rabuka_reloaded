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
fn pl_hs_bp5_011_n_debut_draws_one_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let rurino = game.id("PL!HS-bp5-011-N");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.add_to_stage(MemberArea::Center, rurino);
    let hand_before = game.state.player1.hand.cards.len();
    fire_trigger(&mut game, rurino, AbilityTrigger::Debut, "登場");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "登場 → draw 1"
    );
}
