use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";
const CLEAN_KOTORI: &str = "PL!-pb1-021-PR";

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref().is_some_and(|t| t.contains(trigger_str)))
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
fn pl_bp3_007_r_live_start_paid_look_three_splits_hand_topdeck_and_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id("PL!-bp3-007-R");
    let filler = game.id(FILLER);

    game.state.player1.stage.stage[1] = nozomi;
    let spare_a = game.id(FILLER);
    let spare_b = game.id(FILLER);
    game.add_to_hand(spare_a);
    game.add_to_hand(spare_b);
    let stock = game.new_id(FILLER);
    fill_decks(&mut game, stock);
    put_on_deck_top(&mut game, 0, filler);
    let kotori_id2 = game.id(CLEAN_KOTORI);
    put_on_deck_top(&mut game, 0, kotori_id2);

    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_auto(&mut game, nozomi, AbilityTrigger::LiveStart, "ライブ開始時");

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectTarget") => game.select_option(1),
            Some("SelectCard") => {
                let n = game.pending_choice_count();
                let take: Vec<usize> = (0..n.min(1)).collect();
                game.select_indices(&take);
            }
            _ => break,
        }
        game.drain_auto_ability_choices();
    }

    let hand_now = game.state.player1.hand.cards.len();
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 2,
        "deck: three looked off, one placed back on top"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 3,
        "two paid cards + one looked leftover land in the waitroom"
    );
    assert!(
        hand_now <= 3,
        "paid two, gained one — hand must not exceed start +1"
    );
}
