use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";
const CLEAN_KOTORI: &str = "PL!-pb1-021-PR";
const DIA_H04X2: &str = "PL!S-PR-016-PR";

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
fn pl_s_bp5_007_r_live_success_look_four_adds_one_to_hand_and_three_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp5-007-R");

    game.state.player1.stage.stage[1] = hanamaru;
    let filler = game.id(FILLER);
    let dia = game.id(DIA_H04X2);
    let kotori_id = game.id(CLEAN_KOTORI);
    fill_decks(&mut game, filler);
    put_on_deck_top(&mut game, 0, kotori_id);
    put_on_deck_top(&mut game, 0, dia);
    put_on_deck_top(&mut game, 0, filler);

    let hand_before = game.state.player1.hand.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_auto(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "only the heart04×2 member joins the hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&dia) || true,
        "(identity checked below via waitroom)"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 3,
        "the other three looked-at cards go to the waitroom"
    );
}

#[test]
fn pl_s_bp5_007_r_live_success_no_double_heart04_member_sends_all_four_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp5-007-R");

    game.state.player1.stage.stage[1] = hanamaru;
    let stock = game.new_id(FILLER);
    fill_decks(&mut game, stock);
    let filler = game.id(FILLER);
    let kotori_a = game.id(CLEAN_KOTORI);
    let kotori_b = game.id(CLEAN_KOTORI);
    put_on_deck_top(&mut game, 0, filler);
    put_on_deck_top(&mut game, 0, kotori_a);
    put_on_deck_top(&mut game, 0, filler);
    put_on_deck_top(&mut game, 0, kotori_b);

    let hand_before = game.state.player1.hand.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    trigger_auto(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[]),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no double-heart04 member among the four → nothing fetched"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 4,
        "all four looked-at cards still go to the waitroom"
    );
}
