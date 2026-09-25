use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const LIVE_FILLER: &str = "PL!-sd1-019-SD";

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
fn pl_bp5_011_n_live_start_chosen_heart05_matches_two_success_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let eli = game.id("PL!-bp5-011-N");
    let live = game.id(LIVE_FILLER);

    game.state.player1.stage.stage[1] = eli;
    game.state.player1.success_live_card_zone.cards.push(live);
    game.state.player1.success_live_card_zone.cards.push(live);

    trigger_auto(&mut game, eli, AbilityTrigger::LiveStart, "ライブ開始時");

    assert!(game.has_pending_choice(), "colour choice must be asked");
    game.select_option(1);

    assert_eq!(
        game.state.mods.get_heart_modifier(eli, HeartColor::Heart05),
        2,
        "chosen heart05 × 2 success-pile cards"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(eli, HeartColor::Heart04),
        0,
        "unchosen colours are not granted"
    );
}
