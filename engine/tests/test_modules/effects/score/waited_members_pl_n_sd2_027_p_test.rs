use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

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
fn pl_n_sd2_027_p_declined_wait_grants_no_score_and_three_waited_members_grant_three() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!N-sd2-027-P");
    game.state.player1.live_card_zone.cards.push(live);
    let niji_a = game.id("PL!N-PR-019-PR");
    let niji_b = game.id("PL!N-PR-012-PR");
    let niji_c = game.id("PL!N-PR-014-PR");
    game.state.player1.stage.stage[0] = niji_a;
    game.state.player1.stage.stage[1] = niji_b;
    game.state.player1.stage.stage[2] = niji_c;
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        game.has_pending_choice(),
        "optional wait-cost must be offered"
    );
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(0),
        Some("SelectCard") => game.select_indices(&[]),
        other => panic!("unexpected choice type {other:?}"),
    }
    assert!(
        !game.has_pending_choice(),
        "declining the optional cost ends the ability"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "waited nobody → +0"
    );
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(game.has_pending_choice());
    match game.pending_choice_type().as_deref() {
        Some("SelectTarget") => game.select_option(1),
        Some("SelectCard") => game.select_indices(&[0, 1, 2]),
        other => panic!("unexpected choice type {other:?}"),
    }
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(niji_a),
        Some("wait"),
        "member 1 waited by the cost"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(niji_c),
        Some("wait"),
        "member 3 waited by the cost"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(live),
        3,
        "ウェイトにした1人につきスコア+1 → three waits = +3"
    );
}
